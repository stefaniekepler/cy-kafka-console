pub mod clock;
pub mod config;
pub mod error;
pub mod health;
pub mod paths;
pub mod port;
pub mod process;
pub mod resources;
pub mod settings;
pub mod sidecar;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::clock::RealSleeper;
use crate::config::LaunchConfig;
use crate::health::HttpHealthProbe;
use crate::process::{ManagedProcess, OsProcessSpawner};
use crate::sidecar::{SidecarManager, StartParams};

#[derive(Default)]
struct AppState {
    process: Mutex<Option<Box<dyn ManagedProcess>>>,
    last_error: Mutex<Option<String>>,
    restarted_once: Mutex<bool>,
    shutting_down: AtomicBool,
}

fn start_backend(app: &tauri::AppHandle) -> Result<u16, error::SidecarError> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| error::SidecarError::ResourceNotFound(e.to_string()))?;
    let resources = resources::resolve(&resource_dir)?;
    let app_paths = paths::app_paths()?;
    let port = port::allocate_free_port()?;
    let cfg = LaunchConfig {
        port,
        config_file: app_paths.config_file,
        log_dir: app_paths.log_dir,
        max_heap_mb: settings::read_max_heap(&app_paths.settings_file),
    };
    let spawner = OsProcessSpawner;
    let probe = HttpHealthProbe;
    let sleeper = RealSleeper;
    let mgr = SidecarManager {
        spawner: &spawner,
        probe: &probe,
        sleeper: &sleeper,
    };
    let params = StartParams {
        resources: &resources,
        config: &cfg,
        max_attempts: 120,
        poll_interval: Duration::from_millis(500),
    };
    let run = mgr.start(&params)?;
    let st = app.state::<AppState>();
    st.process.lock().unwrap().replace(run.process);
    Ok(run.port)
}

/// 记录错误并通知 splash 切到错误视图（在主线程调用）。
fn record_and_show_error(handle: &tauri::AppHandle, msg: String) {
    if let Ok(mut e) = handle.state::<AppState>().last_error.lock() {
        *e = Some(msg.clone());
    }
    if let Some(s) = handle.get_webview_window("splash") {
        let _ = s.emit("startup-error", msg);
    }
}

/// 注入到 kafbat-ui 主窗口的界面微调脚本（每次导航前在页面上下文执行）。
/// 仅作用于顶栏 `[aria-label="Page Header"]`，不触碰页面其它内容：
///   1) 隐藏版本块（黄色"版本过期"感叹号 + commit 短哈希 + 构建时间）；
///   2) 隐藏主题切换之后的外部跳转入口（GitHub / Discord / ProductHunt），保留主题切换（非 <a>）。
///
/// CSS（`:has` 整块命中、响应式无闪烁）为主，JS 设 inline-style 作 CSP 兜底，
/// MutationObserver 兜住版本信息异步加载与 SPA 路由重渲染。
const UI_TWEAKS_JS: &str = r#"(function () {
  var STYLE_ID = "kc-ui-tweaks";
  var CSS = [
    '[aria-label="Page Header"] :has(> div > a[title="Current commit"]){display:none !important;}',
    '[aria-label="Page Header"] [title^="Your app version is outdated"]{display:none !important;}',
    '[aria-label="Page Header"] a[title="Current commit"]{display:none !important;}',
    '[aria-label="Page Header"] a[target="_blank"]{display:none !important;}'
  ].join("");
  function injectStyle() {
    if (!document.head || document.getElementById(STYLE_ID)) return;
    var s = document.createElement("style");
    s.id = STYLE_ID;
    s.textContent = CSS;
    document.head.appendChild(s);
  }
  function hardHide() {
    var nav = document.querySelector('[aria-label="Page Header"]');
    if (!nav) return;
    var commit = nav.querySelector('a[title="Current commit"]');
    if (commit && commit.parentElement && commit.parentElement.parentElement) {
      commit.parentElement.parentElement.style.display = "none";
    }
    nav.querySelectorAll('[title^="Your app version is outdated"]').forEach(function (el) { el.style.display = "none"; });
    nav.querySelectorAll('a[target="_blank"]').forEach(function (el) { el.style.display = "none"; });
  }
  function run() { injectStyle(); hardHide(); }
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", run);
  } else {
    run();
  }
  var scheduled = false;
  var obs = new MutationObserver(function () {
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(function () { scheduled = false; hardHide(); });
  });
  function startObserver() {
    if (document.body) { injectStyle(); obs.observe(document.body, { childList: true, subtree: true }); }
    else { setTimeout(startObserver, 50); }
  }
  startObserver();
})();"#;

/// 在后台线程执行阻塞式启动编排；成功建主窗口并关 splash，失败展示错误。可被重试复用。
fn launch_startup(handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        if let Ok(mut e) = handle.state::<AppState>().last_error.lock() {
            *e = None;
        }
        match start_backend(&handle) {
            Ok(port) => {
                let h = handle.clone();
                let _ = handle.run_on_main_thread(move || {
                    let url: tauri::Url = format!("http://127.0.0.1:{port}")
                        .parse()
                        .expect("BUG: u16 端口必产生合法 URL");
                    match WebviewWindowBuilder::new(&h, "main", WebviewUrl::External(url))
                        .title("Kafka Console")
                        .inner_size(1280.0, 820.0)
                        .center()
                        .initialization_script(UI_TWEAKS_JS)
                        .build()
                    {
                        Ok(_) => {
                            if let Some(s) = h.get_webview_window("splash") {
                                // 通知 splash：后端已就绪、主窗口已建成（E2E 在此窗口上断言）。
                                let _ = s.emit("startup-ready", ());
                                // 生产：关闭 splash。E2E（KAFKA_CONSOLE_E2E 置位）：保留 splash 作为
                                // WebDriver 稳定观测窗口——main 加载远程 kafbat-ui 且无 Tauri 能力，
                                // tauri-driver 无法可靠驱动它，故断言必须落在受信的 splash 上。
                                if std::env::var_os("KAFKA_CONSOLE_E2E").is_none() {
                                    let _ = s.close();
                                }
                            }
                            #[cfg(desktop)]
                            spawn_update_check(h.clone());
                            #[cfg(desktop)]
                            spawn_watchdog(h.clone());
                        }
                        Err(e) => record_and_show_error(&h, format!("无法创建主窗口: {e}")),
                    }
                });
            }
            Err(e) => {
                let h = handle.clone();
                let msg = e.to_string();
                let _ = handle.run_on_main_thread(move || record_and_show_error(&h, msg));
            }
        }
    });
}

#[tauri::command]
fn get_max_heap() -> u32 {
    paths::app_paths()
        .map(|p| settings::read_max_heap(&p.settings_file))
        .unwrap_or(settings::DEFAULT_HEAP_MB)
}

#[tauri::command]
fn set_max_heap(mb: u32) -> Result<(), String> {
    let p = paths::app_paths().map_err(|e| e.to_string())?;
    settings::write_max_heap(&p.settings_file, mb)
}

#[tauri::command]
fn open_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.set_focus();
        return Ok(());
    }
    WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Kafka Console 设置")
        .inner_size(420.0, 380.0)
        .resizable(false)
        .center()
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_startup_error(state: tauri::State<'_, AppState>) -> Option<String> {
    state.last_error.lock().ok().and_then(|g| g.clone())
}

#[tauri::command]
fn retry_startup(app: tauri::AppHandle) {
    launch_startup(app);
}

/// 用系统文件管理器打开目录（macOS Finder / Windows 资源管理器 / Linux xdg-open）。
fn reveal_dir(dir: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "windows")]
    let program = "explorer";
    #[cfg(target_os = "linux")]
    let program = "xdg-open";
    std::process::Command::new(program)
        .arg(dir)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn open_logs() -> Result<(), String> {
    let paths = paths::app_paths().map_err(|e| e.to_string())?;
    reveal_dir(&paths.log_dir)
}

/// 打开集群配置文件 dynamic_config.yaml 所在目录，便于备份/复制。
#[tauri::command]
fn open_config_dir() -> Result<(), String> {
    let paths = paths::app_paths().map_err(|e| e.to_string())?;
    // config_file = <data_root>/dynamic_config.yaml，其父目录即配置根目录
    let dir = paths.config_file.parent().ok_or("无法定位配置目录")?;
    reveal_dir(dir)
}

#[cfg(desktop)]
fn spawn_update_check(handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        use tauri_plugin_updater::UpdaterExt;
        let updater = match handle.updater() {
            Ok(u) => u,
            Err(_) => return,
        };
        let update = match updater.check().await {
            Ok(Some(u)) => u,
            _ => return,
        };
        let version = update.version.clone();
        // Show native confirm dialog on main thread
        let (tx, rx) = std::sync::mpsc::channel();
        let h = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
            let ok = h
                .dialog()
                .message(format!("发现新版本 {version}，是否现在更新？"))
                .title("Kafka Console 更新")
                .kind(MessageDialogKind::Info)
                .buttons(MessageDialogButtons::OkCancelCustom(
                    "更新".to_string(),
                    "稍后".to_string(),
                ))
                .blocking_show();
            let _ = tx.send(ok);
        });
        if rx.recv().unwrap_or(false) && update.download_and_install(|_, _| {}, || {}).await.is_ok()
        {
            handle.restart();
        }
    });
}

#[cfg(desktop)]
fn spawn_watchdog(handle: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(3));
        let st = handle.state::<AppState>();
        if st.shutting_down.load(Ordering::SeqCst) {
            return;
        }
        let dead = {
            let mut g = st.process.lock().unwrap();
            match g.as_mut() {
                Some(p) => !p.is_running(),
                None => false,
            }
        };
        if !dead {
            continue;
        }
        let already = {
            let mut r = st.restarted_once.lock().unwrap();
            let was = *r;
            *r = true;
            was
        };
        if already {
            notify_fatal_and_exit(
                &handle,
                "后端进程多次异常退出，应用将关闭。请通过托盘菜单或日志排查。",
            );
            return;
        }
        // 二次崩溃保护后，尝试重启一次
        if st.shutting_down.load(Ordering::SeqCst) {
            return;
        }
        match start_backend(&handle) {
            Ok(port) => {
                // 退出竞态：start_backend 期间用户已退出 → 立即终止刚拉起的进程，避免孤儿
                if st.shutting_down.load(Ordering::SeqCst) {
                    if let Some(mut p) = st.process.lock().unwrap().take() {
                        let _ = p.terminate();
                    }
                    return;
                }
                let h = handle.clone();
                let _ = handle.run_on_main_thread(move || {
                    if let Some(w) = h.get_webview_window("main") {
                        if let Ok(url) = format!("http://127.0.0.1:{port}").parse::<tauri::Url>() {
                            let _ = w.navigate(url);
                        }
                        let _ = w.show();
                    }
                });
            }
            Err(_) => {
                notify_fatal_and_exit(&handle, "后端重启失败，应用将关闭。");
                return;
            }
        }
    });
}

fn notify_fatal_and_exit(handle: &tauri::AppHandle, msg: &str) {
    let st = handle.state::<AppState>();
    st.shutting_down.store(true, Ordering::SeqCst);
    if let Some(mut p) = st.process.lock().unwrap().take() {
        let _ = p.terminate();
    }
    let h = handle.clone();
    let m = msg.to_string();
    let _ = handle.run_on_main_thread(move || {
        use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
        let _ = h
            .dialog()
            .message(m)
            .title("Kafka Console")
            .kind(MessageDialogKind::Error)
            .blocking_show();
        h.exit(1);
    });
}

pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
                if let Some(w) = app
                    .get_webview_window("main")
                    .or_else(|| app.get_webview_window("splash"))
                {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }))
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_dialog::init());
    }

    // macOS：在系统左上角应用菜单（“Kafka Console”下拉）里追加“设置…”。
    // 基于 Menu::default 增量插入，保留默认的 Edit/Window 菜单，
    // 否则整体替换会丢失这些项，导致 WebView 内 Cmd+C/V/A 等快捷键失效。
    #[cfg(target_os = "macos")]
    {
        builder = builder.menu(|handle| {
            use tauri::menu::{Menu, MenuItem, MenuItemKind};
            let menu = Menu::default(handle)?;
            let settings_i =
                MenuItem::with_id(handle, "settings", "设置…", true, Some("CmdOrCtrl+,"))?;
            if let Some(MenuItemKind::Submenu(app_menu)) = menu.items()?.into_iter().next() {
                // 插在 About 之后（索引 1），符合 macOS 惯例
                let _ = app_menu.insert(&settings_i, 1);
            }
            Ok(menu)
        });
    }

    builder
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_startup_error,
            retry_startup,
            open_logs,
            open_config_dir,
            get_max_heap,
            set_max_heap,
            open_settings_window
        ])
        .on_menu_event(|app, event| {
            if event.id() == "settings" {
                let _ = open_settings_window(app.clone());
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();
            WebviewWindowBuilder::new(&handle, "splash", WebviewUrl::App("index.html".into()))
                .title("Kafka Console")
                .inner_size(440.0, 280.0)
                .resizable(false)
                .center()
                .build()?;

            #[cfg(desktop)]
            {
                use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
                use tauri::tray::TrayIconBuilder;
                let show_i = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
                let settings_i = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
                let sep = PredefinedMenuItem::separator(app)?;
                let quit_i =
                    MenuItem::with_id(app, "quit", "退出 Kafka Console", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_i, &settings_i, &sep, &quit_i])?;
                let mut tray = TrayIconBuilder::new().tooltip("Kafka Console").menu(&menu);
                if let Some(icon) = app.default_window_icon() {
                    tray = tray.icon(icon.clone());
                }
                tray.on_menu_event(|app, event| {
                    if event.id() == "show" {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    } else if event.id() == "settings" {
                        let _ = open_settings_window(app.clone());
                    } else if event.id() == "quit" {
                        let st = app.state::<AppState>();
                        st.shutting_down.store(true, Ordering::SeqCst);
                        if let Some(mut p) = st.process.lock().unwrap().take() {
                            let _ = p.terminate();
                        }
                        app.exit(0);
                    }
                })
                .build(app)?;
            }

            launch_startup(handle);
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                // 关闭主窗口 = 最小化到托盘，不退出
                api.prevent_close();
                let _ = window.hide();
            }
            // 启动过程中用户手动关闭 splash（主窗口尚未出现）= 取消启动
            tauri::WindowEvent::Destroyed
                if window.label() == "splash" && window.get_webview_window("main").is_none() =>
            {
                let st = window.state::<AppState>();
                st.shutting_down.store(true, Ordering::SeqCst);
                if let Some(mut p) = st.process.lock().unwrap().take() {
                    let _ = p.terminate();
                }
                window.app_handle().exit(0);
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // 兜底：无论经窗口关闭、Cmd+Q 还是 Dock 退出，都终止 JVM，杜绝孤儿进程
            if let tauri::RunEvent::Exit = event {
                app_handle
                    .state::<AppState>()
                    .shutting_down
                    .store(true, Ordering::SeqCst);
                if let Some(mut p) = app_handle
                    .state::<AppState>()
                    .process
                    .lock()
                    .unwrap()
                    .take()
                {
                    let _ = p.terminate();
                }
            }
        });
}
