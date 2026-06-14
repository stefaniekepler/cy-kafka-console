use crate::clock::Sleeper;
use crate::config::{build_env, LaunchConfig};
use crate::error::SidecarError;
use crate::health::HealthProbe;
use crate::process::{ManagedProcess, ProcessSpawner};
use crate::resources::Resources;
use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

pub struct StartParams<'a> {
    pub resources: &'a Resources,
    pub config: &'a LaunchConfig,
    pub max_attempts: u32,
    pub poll_interval: Duration,
}

pub struct RunningSidecar {
    pub port: u16,
    pub process: Box<dyn ManagedProcess>,
}

pub struct SidecarManager<'a> {
    pub spawner: &'a dyn ProcessSpawner,
    pub probe: &'a dyn HealthProbe,
    pub sleeper: &'a dyn Sleeper,
}

impl SidecarManager<'_> {
    /// 启动 JVM 并轮询直到就绪；失败时终止进程并返回错误。
    /// 每轮先探健康端点，再判存活——避免"已就绪但随即退出"被误判为早退。
    pub fn start(&self, p: &StartParams) -> Result<RunningSidecar, SidecarError> {
        let env = build_env(p.config);
        let stdio_log = p.config.console_log();
        let mut process =
            self.spawner
                .spawn(&p.resources.java_bin, &p.resources.jar, &env, &stdio_log)?;

        let attempts = p.max_attempts.max(1);
        for _ in 0..attempts {
            if self.probe.is_ready(p.config.port) {
                return Ok(RunningSidecar {
                    port: p.config.port,
                    process,
                });
            }
            if !process.is_running() {
                let code = process.exit_code();
                let tail = diagnostic_tail(p.config);
                return Err(SidecarError::JvmExitedEarly {
                    code,
                    log_tail: tail,
                });
            }
            self.sleeper.sleep(p.poll_interval);
        }
        let _ = process.terminate();
        let waited = p
            .poll_interval
            .checked_mul(attempts)
            .unwrap_or(Duration::MAX);
        Err(SidecarError::StartupTimeout(waited))
    }
}

/// 选取最有用的诊断日志：优先 Spring 日志（kafka-console.log），
/// 为空时回退到 stdio 重定向日志（jvm.out.log，含 Spring 启动前的早期失败）。
fn diagnostic_tail(config: &LaunchConfig) -> String {
    let spring = read_log_tail(&config.log_file(), 40);
    if spring.trim().is_empty() {
        read_log_tail(&config.console_log(), 40)
    } else {
        spring
    }
}

/// 读取日志文件最后 n 行（仅读末尾约 64KB，避免大文件占用内存）。
pub fn read_log_tail(path: &std::path::Path, n: usize) -> String {
    let Ok(mut f) = std::fs::File::open(path) else {
        return String::new();
    };
    let len = f.seek(SeekFrom::End(0)).unwrap_or(0);
    let window = len.min(65_536);
    if window > 0 && f.seek(SeekFrom::End(-(window as i64))).is_err() {
        return String::new();
    }
    let mut buf = vec![0u8; window as usize];
    let read = f.read(&mut buf).unwrap_or(0);
    let text = String::from_utf8_lossy(&buf[..read]);
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::tests::FakeSleeper;
    use crate::health::tests::FakeProbe;
    use crate::process::ManagedProcess;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;

    struct FakeProcess {
        running: Arc<AtomicBool>,
        terminated: Arc<AtomicBool>,
        code: Option<i32>,
    }
    impl ManagedProcess for FakeProcess {
        fn is_running(&mut self) -> bool {
            self.running.load(Ordering::SeqCst)
        }
        fn exit_code(&mut self) -> Option<i32> {
            self.code
        }
        fn terminate(&mut self) -> Result<(), SidecarError> {
            self.terminated.store(true, Ordering::SeqCst);
            self.running.store(false, Ordering::SeqCst);
            Ok(())
        }
    }

    struct FakeSpawner {
        running: Arc<AtomicBool>,
        terminated: Arc<AtomicBool>,
        code: Option<i32>,
    }
    impl ProcessSpawner for FakeSpawner {
        fn spawn(
            &self,
            _j: &std::path::Path,
            _r: &std::path::Path,
            _e: &[(String, String)],
            _stdio: &std::path::Path,
        ) -> Result<Box<dyn ManagedProcess>, SidecarError> {
            Ok(Box::new(FakeProcess {
                running: self.running.clone(),
                terminated: self.terminated.clone(),
                code: self.code,
            }))
        }
    }

    fn cfg() -> LaunchConfig {
        LaunchConfig {
            port: 40000,
            config_file: PathBuf::from("/x/c.yaml"),
            log_dir: PathBuf::from("/x/logs"),
            max_heap_mb: 512,
        }
    }
    fn res() -> Resources {
        Resources {
            java_bin: PathBuf::from("/x/java"),
            jar: PathBuf::from("/x/a.jar"),
        }
    }

    #[test]
    fn succeeds_when_probe_becomes_ready() {
        let running = Arc::new(AtomicBool::new(true));
        let terminated = Arc::new(AtomicBool::new(false));
        let spawner = FakeSpawner {
            running,
            terminated,
            code: None,
        };
        let probe = FakeProbe {
            ready_after: 3,
            calls: AtomicU32::new(0),
        };
        let sleeper = FakeSleeper {
            count: AtomicU32::new(0),
        };
        let mgr = SidecarManager {
            spawner: &spawner,
            probe: &probe,
            sleeper: &sleeper,
        };
        let c = cfg();
        let r = res();
        let params = StartParams {
            resources: &r,
            config: &c,
            max_attempts: 10,
            poll_interval: Duration::from_millis(1),
        };
        let run = mgr.start(&params).unwrap();
        assert_eq!(run.port, 40000);
        assert_eq!(sleeper.count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn errors_when_jvm_exits_early_with_code() {
        let running = Arc::new(AtomicBool::new(false));
        let terminated = Arc::new(AtomicBool::new(false));
        let spawner = FakeSpawner {
            running,
            terminated,
            code: Some(1),
        };
        let probe = FakeProbe {
            ready_after: 99,
            calls: AtomicU32::new(0),
        };
        let sleeper = FakeSleeper {
            count: AtomicU32::new(0),
        };
        let mgr = SidecarManager {
            spawner: &spawner,
            probe: &probe,
            sleeper: &sleeper,
        };
        let c = cfg();
        let r = res();
        let params = StartParams {
            resources: &r,
            config: &c,
            max_attempts: 5,
            poll_interval: Duration::from_millis(1),
        };
        match mgr.start(&params) {
            Err(SidecarError::JvmExitedEarly { code, .. }) => assert_eq!(code, Some(1)),
            Err(e) => panic!("期望 JvmExitedEarly，得到错误: {e}"),
            Ok(_) => panic!("期望错误，却成功了"),
        }
    }

    #[test]
    fn times_out_and_terminates() {
        let running = Arc::new(AtomicBool::new(true));
        let terminated = Arc::new(AtomicBool::new(false));
        let spawner = FakeSpawner {
            running,
            terminated: terminated.clone(),
            code: None,
        };
        let probe = FakeProbe {
            ready_after: 999,
            calls: AtomicU32::new(0),
        };
        let sleeper = FakeSleeper {
            count: AtomicU32::new(0),
        };
        let mgr = SidecarManager {
            spawner: &spawner,
            probe: &probe,
            sleeper: &sleeper,
        };
        let c = cfg();
        let r = res();
        let params = StartParams {
            resources: &r,
            config: &c,
            max_attempts: 4,
            poll_interval: Duration::from_millis(1),
        };
        assert!(matches!(
            mgr.start(&params),
            Err(SidecarError::StartupTimeout(_))
        ));
        assert!(terminated.load(Ordering::SeqCst), "超时后必须终止进程");
    }

    #[test]
    fn read_log_tail_returns_last_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("t.log");
        std::fs::write(&f, "l1\nl2\nl3\nl4\nl5\n").unwrap();
        assert_eq!(read_log_tail(&f, 2), "l4\nl5");
        assert_eq!(
            read_log_tail(tmp.path().join("missing.log").as_path(), 5),
            ""
        );
    }
}
