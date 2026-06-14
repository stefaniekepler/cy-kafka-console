# Kafka Console (cy-kafka-console) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 kafbat-ui 包装成跨平台桌面客户端 Kafka Console：双击即用，内部静默拉起内置 JRE 运行 kafbat-ui jar，WebView 指向 localhost 显示原生界面。

**Architecture:** Tauri 2.x 外壳（Rust core + 系统 WebView）将 kafbat-ui 的 `api-<ver>.jar` 作为本地 sidecar 进程整体包裹；内置 jlink 裁剪的最小 JRE；只绑 127.0.0.1。Rust 侧用一个可注入依赖的确定性状态机管理"选端口 → 拉 JVM → 健康探测 → 加载界面 → 优雅退出"。

**Tech Stack:** Rust + Tauri 2、ureq（健康探测）、thiserror、directories；Vite + TypeScript + Vitest（自建 splash/error/settings 窗口）；jlink（最小 JRE）；GitHub Actions（多平台矩阵）；Testcontainers（集成测试真 Kafka）；tauri-driver + WebdriverIO（E2E）；tauri-plugin-updater（自动更新）。

**前置约定**
- 所有路径相对仓库根 `cy-kafka-console/`。
- kafbat-ui 锁定版本变量：`KAFBAT_VERSION=1.5.0`，jar 名 `api-1.5.0.jar`，下载地址 `https://github.com/kafbat/kafka-ui/releases/download/v1.5.0/api-1.5.0.jar`。
- Bundle ID：`com.cy.kafkaconsole`；产品名 `Kafka Console`；二进制 `kafka-console`。
- 健康端点默认 `/actuator/health`（Task 2.1 会在需要时显式开启 management 端点）。
- 每个任务末尾提交一次（frequent commits）。

---

## Milestone M0 — 脚手架与工具链脚本

### Task 0.1: 初始化 Tauri 2 项目与目录骨架

**Files:**
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/build.rs`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Create: `package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`, `src/splash.ts`
- Create: `.gitignore`, `README.md`
- Create: `resources/kafbat/.gitkeep`, `resources/.gitkeep`

- [ ] **Step 1: 创建 `.gitignore`**

```gitignore
# Rust
/src-tauri/target
# Node
node_modules
dist
# 构建期拉取的产物（不入库）
/resources/kafbat/*.jar
/resources/kafbat/*.sha256.verified
/resources/jre
# OS
.DS_Store
```

- [ ] **Step 2: 创建前端 `package.json`**

```json
{
  "name": "cy-kafka-console",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "test:coverage": "vitest run --coverage",
    "lint": "eslint . --ext .ts",
    "tauri": "tauri"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2",
    "@vitest/coverage-v8": "^2",
    "eslint": "^9",
    "typescript": "^5.6",
    "vite": "^5",
    "vitest": "^2"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-updater": "^2"
  }
}
```

- [ ] **Step 3: 创建 `tsconfig.json`、`vite.config.ts`、`index.html`、最小 `src/splash.ts`**

`tsconfig.json`:
```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "skipLibCheck": true,
    "types": ["vite/client"]
  },
  "include": ["src"]
}
```

`vite.config.ts`:
```ts
import { defineConfig } from "vite";

export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: "es2020", outDir: "dist" },
  test: { environment: "jsdom", coverage: { provider: "v8", lines: 80 } },
});
```

`index.html`:
```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <title>Kafka Console</title>
  </head>
  <body>
    <main id="app">正在启动 Kafka Console…</main>
    <script type="module" src="/src/splash.ts"></script>
  </body>
</html>
```

`src/splash.ts`:
```ts
// 占位入口；真实 splash/error 逻辑在 Task 1.11 实现
export const APP_NAME = "Kafka Console";
```

- [ ] **Step 4: 创建 `src-tauri/Cargo.toml`**

```toml
[package]
name = "kafka-console"
version = "0.1.0"
edition = "2021"
rust-version = "1.77"

[lib]
name = "kafka_console_lib"
crate-type = ["lib", "cdylib", "staticlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-single-instance = "2"
tauri-plugin-updater = "2"
thiserror = "1"
ureq = { version = "2", default-features = false, features = ["tls"] }
directories = "5"

[dev-dependencies]
tempfile = "3"

[features]
# 集成测试开关：需要本机已存在 resources/jre 与 resources/kafbat/*.jar
integration = []
```

- [ ] **Step 5: 创建 `src-tauri/build.rs` 与 `src-tauri/src/main.rs`**

`build.rs`:
```rust
fn main() {
    tauri_build::build();
}
```

`src/main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    kafka_console_lib::run();
}
```

- [ ] **Step 6: 创建最小 `src-tauri/src/lib.rs`（先能编译，glue 在 Task 1.10 完善）**

```rust
pub mod clock;
pub mod config;
pub mod error;
pub mod health;
pub mod paths;
pub mod port;
pub mod process;
pub mod resources;
pub mod sidecar;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

> 说明：`lib.rs` 声明的模块文件将在 M1 各任务中创建。本步骤先创建空模块文件占位以便编译：为每个模块建一个空 `.rs` 文件（内容 `// placeholder`），随后任务逐个填充。

- [ ] **Step 7: 为 9 个模块创建占位空文件**

为 `clock.rs config.rs error.rs health.rs paths.rs port.rs process.rs resources.rs sidecar.rs` 各创建内容为 `// placeholder` 的文件于 `src-tauri/src/`。

- [ ] **Step 8: 创建最小 `tauri.conf.json`（打包配置在 Task 2.1 完善）**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Kafka Console",
  "version": "0.1.0",
  "identifier": "com.cy.kafkaconsole",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "resources": []
  }
}
```

- [ ] **Step 9: 创建 `README.md` 占位**

```markdown
# Kafka Console (cy-kafka-console)

将 kafbat-ui 包装成跨平台桌面客户端。详见 `docs/superpowers/specs/`。
```

- [ ] **Step 10: 验证能编译**

Run: `cd src-tauri && cargo build`
Expected: 编译成功（可能有 unused 警告，可接受）。

- [ ] **Step 11: 提交**

```bash
git add -A
git commit -m "chore: 初始化 Tauri 2 项目脚手架与目录骨架"
```

---

### Task 0.2: kafbat-ui jar 下载与校验脚本

**Files:**
- Create: `scripts/fetch-kafbat-jar.sh`（macOS/Linux）
- Create: `scripts/fetch-kafbat-jar.ps1`（Windows）
- Create: `scripts/kafbat.env`（版本与期望校验和）

- [ ] **Step 1: 创建 `scripts/kafbat.env`**

```bash
KAFBAT_VERSION=1.5.0
KAFBAT_JAR=api-1.5.0.jar
KAFBAT_URL=https://github.com/kafbat/kafka-ui/releases/download/v1.5.0/api-1.5.0.jar
# 期望 SHA-256：首次运行脚本时会打印实际值；核实无误后填入此处，使 CI 强制校验。
KAFBAT_SHA256=REPLACE_WITH_VERIFIED_SHA256
```

> 实施备注：首次运行下载后，脚本打印实际 SHA-256；人工核对后回填 `KAFBAT_SHA256` 并提交。`REPLACE_WITH_VERIFIED_SHA256` 时脚本仅警告不阻断（方便首次引导），一旦填入真实值则严格校验。

- [ ] **Step 2: 创建 `scripts/fetch-kafbat-jar.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/kafbat.env"
DEST_DIR="$SCRIPT_DIR/../resources/kafbat"
DEST="$DEST_DIR/$KAFBAT_JAR"
mkdir -p "$DEST_DIR"

echo "下载 $KAFBAT_URL"
curl -fSL "$KAFBAT_URL" -o "$DEST"

ACTUAL=$(shasum -a 256 "$DEST" | awk '{print $1}')
echo "实际 SHA-256: $ACTUAL"
if [ "$KAFBAT_SHA256" = "REPLACE_WITH_VERIFIED_SHA256" ]; then
  echo "::warning:: 尚未固化校验和，请将 $ACTUAL 填入 scripts/kafbat.env"
elif [ "$ACTUAL" != "$KAFBAT_SHA256" ]; then
  echo "::error:: SHA-256 不匹配！期望 $KAFBAT_SHA256 实际 $ACTUAL" >&2
  exit 1
else
  echo "SHA-256 校验通过"
fi
```

- [ ] **Step 3: 创建 `scripts/fetch-kafbat-jar.ps1`**

```powershell
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Get-Content "$root/kafbat.env" | ForEach-Object {
  if ($_ -match '^\s*([^#=]+)=(.*)$') { Set-Variable -Name $Matches[1].Trim() -Value $Matches[2].Trim() }
}
$destDir = Join-Path $root "../resources/kafbat"
New-Item -ItemType Directory -Force -Path $destDir | Out-Null
$dest = Join-Path $destDir $KAFBAT_JAR
Write-Host "下载 $KAFBAT_URL"
Invoke-WebRequest -Uri $KAFBAT_URL -OutFile $dest
$actual = (Get-FileHash -Algorithm SHA256 $dest).Hash.ToLower()
Write-Host "实际 SHA-256: $actual"
if ($KAFBAT_SHA256 -eq "REPLACE_WITH_VERIFIED_SHA256") {
  Write-Host "::warning:: 尚未固化校验和，请将 $actual 填入 scripts/kafbat.env"
} elseif ($actual -ne $KAFBAT_SHA256.ToLower()) {
  Write-Error "SHA-256 不匹配！期望 $KAFBAT_SHA256 实际 $actual"; exit 1
} else { Write-Host "SHA-256 校验通过" }
```

- [ ] **Step 4: 赋可执行权限并运行（验证脚本可工作）**

Run: `chmod +x scripts/fetch-kafbat-jar.sh && ./scripts/fetch-kafbat-jar.sh`
Expected: 下载成功，打印实际 SHA-256，并提示回填 env。

- [ ] **Step 5: 回填真实 SHA-256 到 `scripts/kafbat.env`**

将上一步打印的值替换 `REPLACE_WITH_VERIFIED_SHA256`。
Run: `./scripts/fetch-kafbat-jar.sh`
Expected: 输出 "SHA-256 校验通过"。

- [ ] **Step 6: 提交**

```bash
git add scripts/
git commit -m "build: 添加 kafbat-ui jar 下载与 SHA-256 校验脚本"
```

---

### Task 0.3: jlink 最小 JRE 构建脚本

**Files:**
- Create: `scripts/build-jre.sh`（macOS/Linux）
- Create: `scripts/build-jre.ps1`（Windows）
- Create: `scripts/jre-modules.txt`（模块集，单一事实源）

- [ ] **Step 1: 创建 `scripts/jre-modules.txt`（Spring Boot 安全起步集）**

```
java.base,java.management,java.naming,java.net.http,java.sql,java.security.sasl,java.security.jgss,java.desktop,java.instrument,java.scripting,java.xml,jdk.crypto.ec,jdk.crypto.cryptoki,jdk.unsupported,jdk.zipfs,jdk.management,jdk.httpserver,java.management.rmi
```

> 备注：此集合是基线，由 Task 3.2 的集成测试兜底；若运行报 `ClassNotFound`/`Provider not found`，把缺失模块追加到本文件即可（单一事实源）。

- [ ] **Step 2: 创建 `scripts/build-jre.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT="$SCRIPT_DIR/../resources/jre"
MODULES=$(cat "$SCRIPT_DIR/jre-modules.txt")

command -v jlink >/dev/null || { echo "::error:: 未找到 jlink，请安装匹配 kafbat-ui .java-version 的 Temurin JDK" >&2; exit 1; }
echo "使用的 JDK：$(java -version 2>&1 | head -1)"

rm -rf "$OUT"
jlink --add-modules "$MODULES" \
  --strip-debug --no-header-files --no-man-pages --compress=zip-9 \
  --output "$OUT"

echo "jlink 运行时大小：$(du -sh "$OUT" | awk '{print $1}')"
"$OUT/bin/java" -version
```

- [ ] **Step 3: 创建 `scripts/build-jre.ps1`**

```powershell
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$out = Join-Path $root "../resources/jre"
$modules = (Get-Content (Join-Path $root "jre-modules.txt")).Trim()
if (-not (Get-Command jlink -ErrorAction SilentlyContinue)) { Write-Error "未找到 jlink"; exit 1 }
java -version
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
jlink --add-modules $modules --strip-debug --no-header-files --no-man-pages --compress=zip-9 --output $out
& "$out/bin/java.exe" -version
```

- [ ] **Step 4: 安装匹配的 Temurin JDK 并运行脚本（验证）**

> 实施备注：先核实 kafbat-ui v1.5.0 标签的 `.java-version`，安装对应大版本的 Temurin JDK 并确保 `jlink`/`java` 在 PATH。

Run: `chmod +x scripts/build-jre.sh && ./scripts/build-jre.sh`
Expected: 生成 `resources/jre/`，打印大小（约 50–70MB）与 `java -version`。

- [ ] **Step 5: 提交**

```bash
git add scripts/build-jre.sh scripts/build-jre.ps1 scripts/jre-modules.txt
git commit -m "build: 添加 jlink 最小 JRE 构建脚本与模块清单"
```

---

## Milestone M1 — Sidecar 核心（Rust，TDD 重点）

> 以下每个模块替换 Task 0.1 创建的占位文件。统一用 `cargo test`（在 `src-tauri/` 下执行）。

### Task 1.1: 错误类型

**Files:**
- Modify: `src-tauri/src/error.rs`

- [ ] **Step 1: 写失败测试**

`src-tauri/src/error.rs`:
```rust
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    #[error("资源缺失或损坏: {0}")]
    ResourceNotFound(String),
    #[error("端口分配失败: {0}")]
    PortAllocation(String),
    #[error("用户数据目录不可用")]
    DataDirUnavailable,
    #[error("JVM 进程提前退出 (code={code:?})\n日志末尾:\n{log_tail}")]
    JvmExitedEarly { code: Option<i32>, log_tail: String },
    #[error("启动超时（{0:?}）")]
    StartupTimeout(Duration),
    #[error("启动 JVM 失败: {0}")]
    Spawn(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn display_includes_context() {
        let e = SidecarError::ResourceNotFound("jre/bin/java".into());
        assert!(format!("{e}").contains("jre/bin/java"));
        let e2 = SidecarError::JvmExitedEarly { code: Some(1), log_tail: "boom".into() };
        assert!(format!("{e2}").contains("boom"));
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test error::tests`
Expected: PASS（thiserror 派生生成 Display）。

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/error.rs
git commit -m "feat(core): 定义 SidecarError 统一错误类型"
```

---

### Task 1.2: 端口分配

**Files:**
- Modify: `src-tauri/src/port.rs`

- [ ] **Step 1: 写失败测试**

`src-tauri/src/port.rs`:
```rust
use crate::error::SidecarError;
use std::net::TcpListener;

/// 在 loopback 上申请一个空闲端口（绑定到 0 让 OS 分配后立即释放）。
pub fn allocate_free_port() -> Result<u16, SidecarError> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| SidecarError::PortAllocation(e.to_string()))?;
    let port = listener
        .local_addr()
        .map_err(|e| SidecarError::PortAllocation(e.to_string()))?
        .port();
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn returns_nonzero_bindable_port() {
        let p = allocate_free_port().unwrap();
        assert!(p > 0);
        // 释放后应可再次绑定该端口
        TcpListener::bind(("127.0.0.1", p)).unwrap();
    }
    #[test]
    fn successive_calls_succeed() {
        assert!(allocate_free_port().is_ok());
        assert!(allocate_free_port().is_ok());
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test port::tests`
Expected: PASS。

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/port.rs
git commit -m "feat(core): 实现 loopback 空闲端口分配"
```

---

### Task 1.3: 资源路径解析

**Files:**
- Modify: `src-tauri/src/resources.rs`

- [ ] **Step 1: 写失败测试**

`src-tauri/src/resources.rs`:
```rust
use crate::error::SidecarError;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resources {
    pub java_bin: PathBuf,
    pub jar: PathBuf,
}

fn java_exe_name() -> &'static str {
    if cfg!(windows) { "java.exe" } else { "java" }
}

/// 在 `resource_dir` 下解析内置 JRE 的 java 可执行文件与 kafbat jar。
pub fn resolve(resource_dir: &Path) -> Result<Resources, SidecarError> {
    let java_bin = resource_dir.join("jre").join("bin").join(java_exe_name());
    if !java_bin.exists() {
        return Err(SidecarError::ResourceNotFound(java_bin.display().to_string()));
    }
    let kafbat_dir = resource_dir.join("kafbat");
    let jar = std::fs::read_dir(&kafbat_dir)
        .map_err(|_| SidecarError::ResourceNotFound(kafbat_dir.display().to_string()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.extension().map(|x| x == "jar").unwrap_or(false))
        .ok_or_else(|| SidecarError::ResourceNotFound(format!("{}/*.jar", kafbat_dir.display())))?;
    Ok(Resources { java_bin, jar })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn layout(dir: &Path, with_java: bool, with_jar: bool) {
        if with_java {
            let bin = dir.join("jre").join("bin");
            fs::create_dir_all(&bin).unwrap();
            fs::write(bin.join(java_exe_name()), b"x").unwrap();
        }
        if with_jar {
            let k = dir.join("kafbat");
            fs::create_dir_all(&k).unwrap();
            fs::write(k.join("api-1.5.0.jar"), b"x").unwrap();
        }
    }

    #[test]
    fn resolves_full_layout() {
        let tmp = tempfile::tempdir().unwrap();
        layout(tmp.path(), true, true);
        let r = resolve(tmp.path()).unwrap();
        assert!(r.java_bin.ends_with(java_exe_name()));
        assert!(r.jar.to_string_lossy().ends_with(".jar"));
    }
    #[test]
    fn errors_when_java_missing() {
        let tmp = tempfile::tempdir().unwrap();
        layout(tmp.path(), false, true);
        assert!(matches!(resolve(tmp.path()), Err(SidecarError::ResourceNotFound(_))));
    }
    #[test]
    fn errors_when_jar_missing() {
        let tmp = tempfile::tempdir().unwrap();
        layout(tmp.path(), true, false);
        assert!(matches!(resolve(tmp.path()), Err(SidecarError::ResourceNotFound(_))));
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test resources::tests`
Expected: PASS。

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/resources.rs
git commit -m "feat(core): 实现内置 JRE 与 jar 的资源路径解析"
```

---

### Task 1.4: 用户数据目录

**Files:**
- Modify: `src-tauri/src/paths.rs`

- [ ] **Step 1: 写失败测试**

`src-tauri/src/paths.rs`:
```rust
use crate::error::SidecarError;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub config_file: PathBuf,
    pub log_dir: PathBuf,
}

/// 基于给定数据根目录推导各子路径（纯函数，便于测试）。
pub fn app_paths_from(data_root: &Path) -> AppPaths {
    AppPaths {
        config_file: data_root.join("dynamic_config.yaml"),
        log_dir: data_root.join("logs"),
    }
}

/// 解析 OS 标准数据目录并确保其存在。
pub fn app_paths() -> Result<AppPaths, SidecarError> {
    let dirs = directories::ProjectDirs::from("com", "cy", "kafkaconsole")
        .ok_or(SidecarError::DataDirUnavailable)?;
    let root = dirs.data_dir().to_path_buf();
    let paths = app_paths_from(&root);
    std::fs::create_dir_all(&paths.log_dir).map_err(|_| SidecarError::DataDirUnavailable)?;
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn derives_subpaths() {
        let p = app_paths_from(Path::new("/data/app"));
        assert!(p.config_file.ends_with("dynamic_config.yaml"));
        assert!(p.log_dir.ends_with("logs"));
        assert!(p.config_file.starts_with("/data/app"));
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test paths::tests`
Expected: PASS。

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/paths.rs
git commit -m "feat(core): 推导跨平台用户数据目录与子路径"
```

---

### Task 1.5: 启动配置与环境变量组装

**Files:**
- Modify: `src-tauri/src/config.rs`

- [ ] **Step 1: 写失败测试**

`src-tauri/src/config.rs`:
```rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct LaunchConfig {
    pub port: u16,
    pub config_file: PathBuf,
    pub log_dir: PathBuf,
    pub max_heap_mb: u32,
}

impl LaunchConfig {
    pub fn log_file(&self) -> PathBuf {
        self.log_dir.join("kafka-console.log")
    }
}

/// 组装传给 JVM 的环境变量。强制 loopback 绑定与动态配置。
pub fn build_env(cfg: &LaunchConfig) -> Vec<(String, String)> {
    vec![
        ("SERVER_PORT".into(), cfg.port.to_string()),
        ("SERVER_ADDRESS".into(), "127.0.0.1".into()),
        ("DYNAMIC_CONFIG_ENABLED".into(), "true".into()),
        ("DYNAMIC_CONFIG_PATH".into(), cfg.config_file.to_string_lossy().into()),
        ("LOGGING_FILE_NAME".into(), cfg.log_file().to_string_lossy().into()),
        ("MANAGEMENT_ENDPOINT_HEALTH_ENABLED".into(), "true".into()),
        ("JAVA_TOOL_OPTIONS".into(), format!("-Xmx{}m", cfg.max_heap_mb)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> LaunchConfig {
        LaunchConfig {
            port: 39123,
            config_file: PathBuf::from("/data/dynamic_config.yaml"),
            log_dir: PathBuf::from("/data/logs"),
            max_heap_mb: 512,
        }
    }
    #[test]
    fn env_forces_loopback_and_dynamic_config() {
        let env = build_env(&sample());
        let get = |k: &str| env.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
        assert_eq!(get("SERVER_ADDRESS").as_deref(), Some("127.0.0.1"));
        assert_eq!(get("SERVER_PORT").as_deref(), Some("39123"));
        assert_eq!(get("DYNAMIC_CONFIG_ENABLED").as_deref(), Some("true"));
        assert_eq!(get("JAVA_TOOL_OPTIONS").as_deref(), Some("-Xmx512m"));
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test config::tests`
Expected: PASS。

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/config.rs
git commit -m "feat(core): 组装 JVM 启动环境变量（强制 loopback）"
```

---

### Task 1.6: 进程抽象（trait + OS 实现）

**Files:**
- Modify: `src-tauri/src/process.rs`

- [ ] **Step 1: 写失败测试（用假实现验证 trait 行为契约）**

`src-tauri/src/process.rs`:
```rust
use crate::error::SidecarError;
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// 已启动的被管子进程。
pub trait ManagedProcess: Send {
    /// 是否仍在运行（非阻塞）。
    fn is_running(&mut self) -> bool;
    /// 优雅终止（并尽量清理进程树）。
    fn terminate(&mut self) -> Result<(), SidecarError>;
}

/// 负责拉起 JVM 子进程。
pub trait ProcessSpawner: Send + Sync {
    fn spawn(
        &self,
        java_bin: &Path,
        jar: &Path,
        env: &[(String, String)],
    ) -> Result<Box<dyn ManagedProcess>, SidecarError>;
}

pub struct OsProcessSpawner;

struct OsProcess(Child);

impl ManagedProcess for OsProcess {
    fn is_running(&mut self) -> bool {
        matches!(self.0.try_wait(), Ok(None))
    }
    fn terminate(&mut self) -> Result<(), SidecarError> {
        // 简化：先 kill 再 wait；Windows 进程树清理在 Task 1.10 用 taskkill 增强。
        let _ = self.0.kill();
        let _ = self.0.wait();
        Ok(())
    }
}

impl ProcessSpawner for OsProcessSpawner {
    fn spawn(
        &self,
        java_bin: &Path,
        jar: &Path,
        env: &[(String, String)],
    ) -> Result<Box<dyn ManagedProcess>, SidecarError> {
        let mut cmd = Command::new(java_bin);
        cmd.arg("-jar").arg(jar).stdout(Stdio::null()).stderr(Stdio::null());
        for (k, v) in env {
            cmd.env(k, v);
        }
        let child = cmd.spawn().map_err(|e| SidecarError::Spawn(e.to_string()))?;
        Ok(Box::new(OsProcess(child)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// 测试用假进程：可编程"是否运行"。
    pub struct FakeProcess {
        pub running: Arc<AtomicBool>,
        pub terminated: Arc<AtomicBool>,
    }
    impl ManagedProcess for FakeProcess {
        fn is_running(&mut self) -> bool { self.running.load(Ordering::SeqCst) }
        fn terminate(&mut self) -> Result<(), SidecarError> {
            self.terminated.store(true, Ordering::SeqCst);
            self.running.store(false, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn fake_process_contract() {
        let running = Arc::new(AtomicBool::new(true));
        let terminated = Arc::new(AtomicBool::new(false));
        let mut p = FakeProcess { running: running.clone(), terminated: terminated.clone() };
        assert!(p.is_running());
        p.terminate().unwrap();
        assert!(terminated.load(Ordering::SeqCst));
        assert!(!p.is_running());
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test process::tests`
Expected: PASS。

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/process.rs
git commit -m "feat(core): 定义进程 spawner/managed trait 与 OS 实现"
```

---

### Task 1.7: 健康探测抽象

**Files:**
- Modify: `src-tauri/src/health.rs`

- [ ] **Step 1: 写失败测试**

`src-tauri/src/health.rs`:
```rust
/// 探测 kafbat-ui 后端是否就绪。
pub trait HealthProbe: Send + Sync {
    fn is_ready(&self, port: u16) -> bool;
}

/// 真实实现：GET http://127.0.0.1:<port>/actuator/health，2xx 视为就绪。
pub struct HttpHealthProbe;

impl HealthProbe for HttpHealthProbe {
    fn is_ready(&self, port: u16) -> bool {
        let url = format!("http://127.0.0.1:{port}/actuator/health");
        match ureq::get(&url).timeout(std::time::Duration::from_secs(2)).call() {
            Ok(resp) => resp.status() == 200,
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub struct FakeProbe { pub ready_after: u32, pub calls: std::cell::Cell<u32> }
    impl HealthProbe for FakeProbe {
        fn is_ready(&self, _port: u16) -> bool {
            let n = self.calls.get() + 1;
            self.calls.set(n);
            n >= self.ready_after
        }
    }
    #[test]
    fn fake_probe_becomes_ready() {
        let p = FakeProbe { ready_after: 3, calls: std::cell::Cell::new(0) };
        assert!(!p.is_ready(1));
        assert!(!p.is_ready(1));
        assert!(p.is_ready(1));
    }
}
```

> 注：`FakeProbe` 用 `Cell` 非线程安全；状态机测试在单线程内调用，满足 `Send + Sync`？`Cell` 非 `Sync`。为满足 trait 约束，测试里改用 `std::sync::atomic::AtomicU32`。下一步修正。

- [ ] **Step 2: 用 AtomicU32 修正 FakeProbe 以满足 Send+Sync**

将测试中的 `FakeProbe` 改为：
```rust
use std::sync::atomic::{AtomicU32, Ordering};
pub struct FakeProbe { pub ready_after: u32, pub calls: AtomicU32 }
impl HealthProbe for FakeProbe {
    fn is_ready(&self, _port: u16) -> bool {
        let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        n >= self.ready_after
    }
}
#[test]
fn fake_probe_becomes_ready() {
    let p = FakeProbe { ready_after: 3, calls: AtomicU32::new(0) };
    assert!(!p.is_ready(1));
    assert!(!p.is_ready(1));
    assert!(p.is_ready(1));
}
```

- [ ] **Step 3: 运行测试**

Run: `cargo test health::tests`
Expected: PASS。

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/health.rs
git commit -m "feat(core): 定义健康探测 trait 与 HTTP 实现"
```

---

### Task 1.8: 可注入睡眠

**Files:**
- Modify: `src-tauri/src/clock.rs`

- [ ] **Step 1: 写实现 + 测试**

`src-tauri/src/clock.rs`:
```rust
use std::time::Duration;

/// 抽象睡眠，便于在测试中零延迟。
pub trait Sleeper: Send + Sync {
    fn sleep(&self, dur: Duration);
}

pub struct RealSleeper;
impl Sleeper for RealSleeper {
    fn sleep(&self, dur: Duration) { std::thread::sleep(dur); }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    /// 记录睡眠次数、零延迟。
    pub struct FakeSleeper { pub count: AtomicU32 }
    impl Sleeper for FakeSleeper {
        fn sleep(&self, _dur: Duration) { self.count.fetch_add(1, Ordering::SeqCst); }
    }
    #[test]
    fn fake_sleeper_counts() {
        let s = FakeSleeper { count: AtomicU32::new(0) };
        s.sleep(Duration::from_secs(99));
        assert_eq!(s.count.load(Ordering::SeqCst), 1);
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test clock::tests`
Expected: PASS。

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/clock.rs
git commit -m "feat(core): 添加可注入 Sleeper 抽象"
```

---

### Task 1.9: 状态机 SidecarManager（核心，TDD 全分支）

**Files:**
- Modify: `src-tauri/src/sidecar.rs`

- [ ] **Step 1: 写实现**

`src-tauri/src/sidecar.rs`:
```rust
use crate::clock::Sleeper;
use crate::config::{build_env, LaunchConfig};
use crate::error::SidecarError;
use crate::health::HealthProbe;
use crate::process::{ManagedProcess, ProcessSpawner};
use crate::resources::Resources;
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

impl<'a> SidecarManager<'a> {
    /// 启动 JVM 并轮询直到就绪；失败时终止进程并返回错误。
    pub fn start(&self, p: &StartParams) -> Result<RunningSidecar, SidecarError> {
        let env = build_env(p.config);
        let mut process = self.spawner.spawn(&p.resources.java_bin, &p.resources.jar, &env)?;

        for _ in 0..p.max_attempts {
            if !process.is_running() {
                let tail = read_log_tail(&p.config.log_file(), 40);
                return Err(SidecarError::JvmExitedEarly { code: None, log_tail: tail });
            }
            if self.probe.is_ready(p.config.port) {
                return Ok(RunningSidecar { port: p.config.port, process });
            }
            self.sleeper.sleep(p.poll_interval);
        }
        let _ = process.terminate();
        Err(SidecarError::StartupTimeout(p.poll_interval * p.max_attempts))
    }
}

/// 读取日志文件最后 n 行（用于错误展示）。
pub fn read_log_tail(path: &std::path::Path, n: usize) -> String {
    match std::fs::read_to_string(path) {
        Ok(s) => s.lines().rev().take(n).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n"),
        Err(_) => String::new(),
    }
}
```

- [ ] **Step 2: 写失败测试（覆盖成功/早退/超时/超时后终止）**

在 `sidecar.rs` 末尾追加：
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::tests::FakeSleeper;
    use crate::health::tests::FakeProbe;
    use crate::process::ManagedProcess;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;

    struct FakeProcess { running: Arc<AtomicBool>, terminated: Arc<AtomicBool> }
    impl ManagedProcess for FakeProcess {
        fn is_running(&mut self) -> bool { self.running.load(Ordering::SeqCst) }
        fn terminate(&mut self) -> Result<(), SidecarError> {
            self.terminated.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    struct FakeSpawner { running: Arc<AtomicBool>, terminated: Arc<AtomicBool> }
    impl ProcessSpawner for FakeSpawner {
        fn spawn(&self, _j: &std::path::Path, _r: &std::path::Path, _e: &[(String, String)])
            -> Result<Box<dyn ManagedProcess>, SidecarError> {
            Ok(Box::new(FakeProcess { running: self.running.clone(), terminated: self.terminated.clone() }))
        }
    }

    fn cfg() -> LaunchConfig {
        LaunchConfig { port: 40000, config_file: PathBuf::from("/x/c.yaml"),
            log_dir: PathBuf::from("/x/logs"), max_heap_mb: 512 }
    }
    fn res() -> Resources {
        Resources { java_bin: PathBuf::from("/x/java"), jar: PathBuf::from("/x/a.jar") }
    }

    #[test]
    fn succeeds_when_probe_becomes_ready() {
        let running = Arc::new(AtomicBool::new(true));
        let terminated = Arc::new(AtomicBool::new(false));
        let spawner = FakeSpawner { running, terminated };
        let probe = FakeProbe { ready_after: 3, calls: AtomicU32::new(0) };
        let sleeper = FakeSleeper { count: AtomicU32::new(0) };
        let mgr = SidecarManager { spawner: &spawner, probe: &probe, sleeper: &sleeper };
        let c = cfg(); let r = res();
        let params = StartParams { resources: &r, config: &c, max_attempts: 10, poll_interval: Duration::from_millis(1) };
        let run = mgr.start(&params).unwrap();
        assert_eq!(run.port, 40000);
        assert_eq!(sleeper.count.load(Ordering::SeqCst), 2); // 第3次探测成功，前2次睡眠
    }

    #[test]
    fn errors_when_jvm_exits_early() {
        let running = Arc::new(AtomicBool::new(false)); // 一开始就不在跑
        let terminated = Arc::new(AtomicBool::new(false));
        let spawner = FakeSpawner { running, terminated };
        let probe = FakeProbe { ready_after: 99, calls: AtomicU32::new(0) };
        let sleeper = FakeSleeper { count: AtomicU32::new(0) };
        let mgr = SidecarManager { spawner: &spawner, probe: &probe, sleeper: &sleeper };
        let c = cfg(); let r = res();
        let params = StartParams { resources: &r, config: &c, max_attempts: 5, poll_interval: Duration::from_millis(1) };
        assert!(matches!(mgr.start(&params), Err(SidecarError::JvmExitedEarly { .. })));
    }

    #[test]
    fn times_out_and_terminates() {
        let running = Arc::new(AtomicBool::new(true));
        let terminated = Arc::new(AtomicBool::new(false));
        let spawner = FakeSpawner { running, terminated: terminated.clone() };
        let probe = FakeProbe { ready_after: 999, calls: AtomicU32::new(0) }; // 永不就绪
        let sleeper = FakeSleeper { count: AtomicU32::new(0) };
        let mgr = SidecarManager { spawner: &spawner, probe: &probe, sleeper: &sleeper };
        let c = cfg(); let r = res();
        let params = StartParams { resources: &r, config: &c, max_attempts: 4, poll_interval: Duration::from_millis(1) };
        assert!(matches!(mgr.start(&params), Err(SidecarError::StartupTimeout(_))));
        assert!(terminated.load(Ordering::SeqCst), "超时后必须终止进程");
    }
}
```

- [ ] **Step 3: 运行测试验证全绿**

Run: `cargo test sidecar::tests`
Expected: 3 个测试 PASS（成功 / 早退 / 超时并终止）。

- [ ] **Step 4: 运行全部单测**

Run: `cargo test`
Expected: 所有模块测试 PASS。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/sidecar.rs
git commit -m "feat(core): 实现 Sidecar 启动状态机（TDD 覆盖成功/早退/超时）"
```

---

## Milestone M1b — Tauri 接线与自建窗口

### Task 1.10: lib.rs 接线（单实例 / 启动编排 / 窗口 / 优雅退出）

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`（加 `tauri-plugin-single-instance` 已在 0.1，确认）

- [ ] **Step 1: 实现启动编排与窗口切换**

`src-tauri/src/lib.rs`:
```rust
pub mod clock;
pub mod config;
pub mod error;
pub mod health;
pub mod paths;
pub mod port;
pub mod process;
pub mod resources;
pub mod sidecar;

use std::sync::Mutex;
use std::time::Duration;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

use crate::clock::RealSleeper;
use crate::config::LaunchConfig;
use crate::health::HttpHealthProbe;
use crate::process::{ManagedProcess, OsProcessSpawner};
use crate::sidecar::{SidecarManager, StartParams};

/// 应用持有运行中的子进程句柄，用于退出时清理。
struct AppState {
    process: Mutex<Option<Box<dyn ManagedProcess>>>,
}

fn start_backend(app: &tauri::AppHandle) -> Result<u16, error::SidecarError> {
    let resource_dir = app.path().resource_dir().map_err(|e| error::SidecarError::ResourceNotFound(e.to_string()))?;
    let resources = resources::resolve(&resource_dir)?;
    let app_paths = paths::app_paths()?;
    let port = port::allocate_free_port()?;
    let cfg = LaunchConfig {
        port,
        config_file: app_paths.config_file,
        log_dir: app_paths.log_dir,
        max_heap_mb: 512,
    };
    let spawner = OsProcessSpawner;
    let probe = HttpHealthProbe;
    let sleeper = RealSleeper;
    let mgr = SidecarManager { spawner: &spawner, probe: &probe, sleeper: &sleeper };
    let params = StartParams {
        resources: &resources,
        config: &cfg,
        max_attempts: 120,                 // 120 * 500ms = 60s
        poll_interval: Duration::from_millis(500),
    };
    let run = mgr.start(&params)?;
    app.state::<AppState>().process.lock().unwrap().replace(run.process);
    Ok(run.port)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("splash").or_else(|| app.get_webview_window("main")) {
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState { process: Mutex::new(None) })
        .setup(|app| {
            // 显示 splash 窗口
            WebviewWindowBuilder::new(app, "splash", WebviewUrl::App("index.html".into()))
                .title("Kafka Console")
                .inner_size(420.0, 260.0)
                .resizable(false)
                .center()
                .build()?;

            let handle = app.handle().clone();
            // 后台线程做阻塞式启动编排，完成后切换窗口
            std::thread::spawn(move || match start_backend(&handle) {
                Ok(port) => {
                    let _ = WebviewWindowBuilder::new(&handle, "main",
                        WebviewUrl::External(format!("http://127.0.0.1:{port}").parse().unwrap()))
                        .title("Kafka Console")
                        .inner_size(1280.0, 820.0)
                        .center()
                        .build();
                    if let Some(s) = handle.get_webview_window("splash") { let _ = s.close(); }
                }
                Err(e) => {
                    // 把错误信息发给 splash 窗口（它会切换到错误视图）
                    if let Some(s) = handle.get_webview_window("splash") {
                        let _ = s.emit("startup-error", e.to_string());
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if window.label() == "main" {
                    // 主窗口关闭即退出并清理 JVM
                    let state = window.state::<AppState>();
                    if let Some(mut p) = state.process.lock().unwrap().take() {
                        let _ = p.terminate();
                    }
                    window.app_handle().exit(0);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: 编译验证**

Run: `cd src-tauri && cargo build`
Expected: 编译成功（Tauri 插件需要在 `tauri.conf.json` 注册权限，Task 2.1 处理；编译期不报错）。

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat(shell): 接线启动编排、单实例、窗口切换与优雅退出"
```

---

### Task 1.11: splash / error 前端窗口

**Files:**
- Modify: `index.html`
- Modify: `src/splash.ts`
- Create: `src/heap.ts`（设置项校验逻辑，供 Vitest 测试）
- Create: `src/heap.test.ts`

- [ ] **Step 1: 写前端逻辑单测（heap 校验）**

`src/heap.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { validateHeapMb } from "./heap";

describe("validateHeapMb", () => {
  it("接受合理范围", () => {
    expect(validateHeapMb(512)).toEqual({ ok: true, value: 512 });
  });
  it("拒绝过小", () => {
    expect(validateHeapMb(64).ok).toBe(false);
  });
  it("拒绝非整数/NaN", () => {
    expect(validateHeapMb(NaN).ok).toBe(false);
  });
});
```

`src/heap.ts`:
```ts
export type HeapResult = { ok: true; value: number } | { ok: false; error: string };

export function validateHeapMb(mb: number): HeapResult {
  if (!Number.isInteger(mb)) return { ok: false, error: "必须为整数 MB" };
  if (mb < 128) return { ok: false, error: "至少 128MB" };
  if (mb > 8192) return { ok: false, error: "最多 8192MB" };
  return { ok: true, value: mb };
}
```

- [ ] **Step 2: 运行前端测试**

Run: `npm install && npm run test`
Expected: heap.test.ts PASS。

- [ ] **Step 3: 实现 splash + 错误视图切换**

`index.html`（替换 body）:
```html
<body>
  <main id="app">
    <section id="loading">
      <h1>Kafka Console</h1>
      <p>正在启动后端服务…</p>
      <div class="spinner"></div>
    </section>
    <section id="error" hidden>
      <h1>启动失败</h1>
      <pre id="error-detail"></pre>
      <p>日志目录见上方路径。</p>
      <button id="retry">重试</button>
      <button id="quit">退出</button>
    </section>
  </main>
  <script type="module" src="/src/splash.ts"></script>
</body>
```

`src/splash.ts`:
```ts
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

const loading = document.getElementById("loading")!;
const errBox = document.getElementById("error")!;
const detail = document.getElementById("error-detail")!;

await listen<string>("startup-error", (e) => {
  loading.setAttribute("hidden", "");
  errBox.removeAttribute("hidden");
  detail.textContent = e.payload;
});

document.getElementById("retry")?.addEventListener("click", () => location.reload());
document.getElementById("quit")?.addEventListener("click", () => getCurrentWindow().close());
```

> 说明：重试目前用 `location.reload()` 触发 splash 重载；真正"重新执行后端编排"由 Task 1.10 的 setup 在窗口重建时再次运行（M4 E2E 阶段若发现需要专用 retry command，可补一个 `#[tauri::command]`，此处保持最小）。

- [ ] **Step 4: 构建前端验证**

Run: `npm run build`
Expected: `dist/` 产出成功。

- [ ] **Step 5: 提交**

```bash
git add index.html src/ package.json package-lock.json
git commit -m "feat(ui): splash 加载页与启动错误视图 + heap 校验单测"
```

---

## Milestone M2 — 本地打包

### Task 2.1: 完善 tauri.conf.json（资源/打包目标/权限）

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`

- [ ] **Step 1: 配置 bundle.resources 与目标**

`src-tauri/tauri.conf.json`:
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Kafka Console",
  "version": "0.1.0",
  "identifier": "com.cy.kafkaconsole",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": ["dmg", "nsis", "appimage", "deb"],
    "resources": ["../resources/jre/**/*", "../resources/kafbat/*.jar"],
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.icns", "icons/icon.ico"]
  },
  "plugins": {
    "updater": {
      "endpoints": ["https://github.com/<owner>/cy-kafka-console/releases/latest/download/latest.json"],
      "pubkey": "REPLACE_WITH_TAURI_UPDATER_PUBKEY"
    }
  }
}
```

> 备注：`<owner>` 与 `pubkey` 在 Task 5.1 生成密钥后回填；图标用 `npm run tauri icon <png>` 生成或先放占位。

- [ ] **Step 2: 创建能力（权限）文件**

`src-tauri/capabilities/default.json`:
```json
{
  "$schema": "https://schema.tauri.app/capability/2",
  "identifier": "default",
  "description": "默认能力",
  "windows": ["splash", "main"],
  "permissions": [
    "core:default",
    "core:window:allow-set-focus",
    "core:window:allow-close",
    "core:event:default",
    "updater:default"
  ]
}
```

- [ ] **Step 3: 准备图标（占位即可）**

Run: `npm run tauri icon path/to/logo.png`（无 logo 时先用任意 1024x1024 png）
Expected: 生成 `src-tauri/icons/`。

- [ ] **Step 4: 提交**

```bash
git add src-tauri/tauri.conf.json src-tauri/capabilities/ src-tauri/icons/
git commit -m "build: 配置 Tauri 资源打包、打包目标与窗口权限"
```

---

### Task 2.2: 本地端到端构建串联

**Files:**
- Create: `scripts/build-app.sh`、`scripts/build-app.ps1`

- [ ] **Step 1: 创建一键构建脚本（fetch jar → jlink → tauri build）**

`scripts/build-app.sh`:
```bash
#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."
./scripts/fetch-kafbat-jar.sh
./scripts/build-jre.sh
npm install
npm run tauri build
echo "产物位于 src-tauri/target/release/bundle/"
```

`scripts/build-app.ps1`:
```powershell
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $root
& "$root/scripts/fetch-kafbat-jar.ps1"
& "$root/scripts/build-jre.ps1"
npm install
npm run tauri build
Write-Host "产物位于 src-tauri/target/release/bundle/"
```

- [ ] **Step 2: 本地运行构建（验证产出安装包）**

Run: `chmod +x scripts/build-app.sh && ./scripts/build-app.sh`
Expected: `src-tauri/target/release/bundle/` 下生成当前平台安装包（macOS: dmg）。

- [ ] **Step 3: 手动冒烟（验证应用真能跑起来）**

打开生成的安装包安装并启动；预期：出现 splash → 切换到 kafbat-ui 界面 → 用配置向导可添加集群。

- [ ] **Step 4: 提交**

```bash
git add scripts/build-app.sh scripts/build-app.ps1
git commit -m "build: 添加本地一键构建脚本（fetch+jlink+tauri build）"
```

---

## Milestone M3 — 测试基础设施与 CI

### Task 3.1: CI 工作流（lint + 单测 + 覆盖率）

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `src-tauri/.config/nextest.toml`（可选）

- [ ] **Step 1: 创建 ci.yml**

`.github/workflows/ci.yml`:
```yaml
name: CI
on:
  pull_request:
  push:
    branches: [main]

jobs:
  rust:
    strategy:
      matrix:
        os: [ubuntu-22.04, windows-latest, macos-14]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: clippy, rustfmt }
      - name: Linux 系统依赖
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
      - name: 安装 cargo-llvm-cov
        uses: taiki-e/install-action@cargo-llvm-cov
      - name: fmt
        run: cd src-tauri && cargo fmt --check
      - name: clippy
        run: cd src-tauri && cargo clippy -- -D warnings
      - name: 单测 + 覆盖率
        run: cd src-tauri && cargo llvm-cov --fail-under-lines 80

  frontend:
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20, cache: npm }
      - run: npm ci
      - run: npm run lint
      - run: npm run test:coverage
```

> 备注：`cargo clippy -D warnings` 要求清理 Task 1.10 引入的告警。本步骤需把现有告警修干净。

- [ ] **Step 2: 本地预跑确保通过**

Run: `cd src-tauri && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
Expected: 全绿。

- [ ] **Step 3: 提交并推送，确认 CI 绿**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: 三平台 lint + 单测 + 覆盖率门禁"
```

---

### Task 3.2: 集成测试（真 Kafka + 真 JVM，验证 jlink 模块集）

**Files:**
- Create: `src-tauri/tests/integration_sidecar.rs`
- Modify: `src-tauri/Cargo.toml`（dev-deps 加 testcontainers）
- Modify: `.github/workflows/ci.yml`（新增 integration job）

- [ ] **Step 1: 加测试依赖**

`src-tauri/Cargo.toml` 的 `[dev-dependencies]` 追加：
```toml
testcontainers = "0.20"
testcontainers-modules = { version = "0.8", features = ["kafka"] }
```

- [ ] **Step 2: 写集成测试（标记 ignore，仅集成 job 跑）**

`src-tauri/tests/integration_sidecar.rs`:
```rust
//! 集成测试：用真实 jlink JRE 跑真实 kafbat-ui jar，连真实 Kafka。
//! 前置：仓库根 resources/jre 与 resources/kafbat/*.jar 已就绪（CI 先 fetch+jlink）。
//! 运行：cargo test --test integration_sidecar -- --ignored

use kafka_console_lib::{config::LaunchConfig, health::{HealthProbe, HttpHealthProbe},
    port, process::OsProcessSpawner, resources, sidecar::{SidecarManager, StartParams}};
use std::time::Duration;

#[test]
#[ignore]
fn backend_boots_against_real_kafka() {
    // 1) 起一个真实 Kafka
    use testcontainers_modules::kafka::Kafka;
    use testcontainers::runners::SyncRunner;
    let _kafka = Kafka::default().start().expect("启动 Kafka 容器");

    // 2) 解析本机 resources（CI 已 fetch jar + jlink）
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let res = resources::resolve(&repo_root.join("resources")).expect("resolve resources");

    // 3) 用真实 spawner/probe 启动后端
    let tmp = tempfile::tempdir().unwrap();
    let cfg = LaunchConfig {
        port: port::allocate_free_port().unwrap(),
        config_file: tmp.path().join("dynamic_config.yaml"),
        log_dir: tmp.path().to_path_buf(),
        max_heap_mb: 512,
    };
    std::fs::create_dir_all(&cfg.log_dir).unwrap();
    let spawner = OsProcessSpawner;
    let probe = HttpHealthProbe;
    let sleeper = kafka_console_lib::clock::RealSleeper;
    let mgr = SidecarManager { spawner: &spawner, probe: &probe, sleeper: &sleeper };
    let params = StartParams { resources: &res, config: &cfg,
        max_attempts: 120, poll_interval: Duration::from_millis(500) };

    let mut run = mgr.start(&params).expect("后端应在超时内就绪（若失败常因 jlink 模块缺失）");

    // 4) 断言健康端点 UP
    assert!(HttpHealthProbe.is_ready(run.port));
    let _ = run.process.terminate();
}
```

> 关键：若该测试因 `ClassNotFound`/`Provider not found` 失败 → 把缺失模块加入 `scripts/jre-modules.txt` 重跑。这就是"集成测试兜底 jlink 模块集"的闭环。

- [ ] **Step 3: 新增 integration job**

`.github/workflows/ci.yml` 追加：
```yaml
  integration:
    strategy:
      matrix:
        os: [ubuntu-22.04, windows-latest, macos-14]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/setup-java@v4
        with: { distribution: temurin, java-version: '25' }   # kafbat-ui v1.5.0 运行时为 Java 25（已核实）
      - name: Linux 系统依赖
        if: runner.os == 'Linux'
        run: sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev
      - name: 拉取 kafbat jar
        shell: bash
        run: ./scripts/fetch-kafbat-jar.sh
      - name: 构建 jlink JRE
        shell: bash
        run: ./scripts/build-jre.sh
      - name: 集成测试
        run: cd src-tauri && cargo test --test integration_sidecar -- --ignored --nocapture
```

- [ ] **Step 4: 本地预跑（已 fetch+jlink 的环境）**

Run: `cd src-tauri && cargo test --test integration_sidecar -- --ignored --nocapture`
Expected: PASS；失败则按备注补模块。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/Cargo.toml src-tauri/tests/integration_sidecar.rs .github/workflows/ci.yml
git commit -m "test: 集成测试（真 Kafka+真 JVM）验证 jlink 模块集"
```

---

### Task 3.3: E2E（tauri-driver + WebdriverIO，Linux/Windows）

**Files:**
- Create: `tests/e2e/package.json`、`tests/e2e/wdio.conf.ts`、`tests/e2e/specs/launch.e2e.ts`
- Modify: `.github/workflows/ci.yml`（新增 e2e job）

- [ ] **Step 1: 创建 E2E 工程**

`tests/e2e/package.json`:
```json
{
  "name": "cy-kafka-console-e2e",
  "private": true,
  "type": "module",
  "scripts": { "test": "wdio run wdio.conf.ts" },
  "devDependencies": {
    "@wdio/cli": "^9",
    "@wdio/local-runner": "^9",
    "@wdio/mocha-framework": "^9",
    "@wdio/spec-reporter": "^9",
    "typescript": "^5.6"
  }
}
```

`tests/e2e/wdio.conf.ts`:
```ts
import { spawn, spawnSync, ChildProcess } from "node:child_process";
import path from "node:path";

let tauriDriver: ChildProcess;
const appBinary = process.env.APP_BINARY!; // CI 注入已构建的二进制路径

export const config: WebdriverIO.Config = {
  specs: ["./specs/**/*.e2e.ts"],
  maxInstances: 1,
  capabilities: [{
    // @ts-expect-error tauri 自定义 capability
    "tauri:options": { application: appBinary },
    browserName: "wry",
  }],
  reporters: ["spec"],
  framework: "mocha",
  mochaOpts: { timeout: 120000 },
  beforeSession: () => {
    tauriDriver = spawn("tauri-driver", [], { stdio: [null, process.stdout, process.stderr] });
  },
  afterSession: () => { tauriDriver?.kill(); },
  hostname: "127.0.0.1",
  port: 4444,
};
```

`tests/e2e/specs/launch.e2e.ts`:
```ts
describe("应用启动", () => {
  it("出现 splash 并最终加载出 kafbat-ui 界面", async () => {
    // splash 文案
    const heading = await $("h1");
    await expect(heading).toHaveText(expect.stringContaining("Kafka Console"));
    // 等待主界面（kafbat-ui）出现其特征元素；超时 90s
    await browser.waitUntil(async () => {
      const html = await browser.getPageSource();
      return html.includes("kafbat") || html.includes("Dashboard") || html.includes("Clusters");
    }, { timeout: 90000, interval: 1000, timeoutMsg: "kafbat-ui 界面未在 90s 内加载" });
  });
});
```

- [ ] **Step 2: 新增 e2e job（Linux + Windows）**

`.github/workflows/ci.yml` 追加：
```yaml
  e2e:
    strategy:
      matrix:
        os: [ubuntu-22.04, windows-latest]   # macOS 无 WKWebView WebDriver，见 spec 7.3
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - uses: actions/setup-java@v4
        with: { distribution: temurin, java-version: '25' }
      - name: Linux 系统依赖 + tauri-driver
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev webkit2gtk-driver xvfb
      - name: 安装 tauri-driver
        run: cargo install tauri-driver --locked
      - name: 拉 jar + jlink + 构建应用
        shell: bash
        run: |
          ./scripts/fetch-kafbat-jar.sh
          ./scripts/build-jre.sh
          npm ci
          npm run tauri build -- --no-bundle
      - name: 安装 E2E 依赖
        run: cd tests/e2e && npm ci
      - name: 运行 E2E
        shell: bash
        env:
          APP_BINARY: ${{ runner.os == 'Windows' && 'src-tauri/target/release/kafka-console.exe' || 'src-tauri/target/release/kafka-console' }}
        run: |
          if [ "${{ runner.os }}" = "Linux" ]; then
            xvfb-run -a npm --prefix tests/e2e test
          else
            npm --prefix tests/e2e test
          fi
```

- [ ] **Step 3: 本地预跑（Linux）验证**

Run: `xvfb-run -a npm --prefix tests/e2e test`（需先构建好二进制并设置 APP_BINARY）
Expected: launch.e2e.ts PASS。

- [ ] **Step 4: 提交**

```bash
git add tests/e2e/ .github/workflows/ci.yml
git commit -m "test(e2e): tauri-driver + WebdriverIO 启动黄金路径（Linux/Windows）"
```

---

## Milestone M4 — 发布流水线

### Task 4.1: release.yml（多平台矩阵构建 + 发布）

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: 创建发布工作流**

`.github/workflows/release.yml`:
```yaml
name: Release
on:
  push:
    tags: ["v*"]

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: macos-14
            target: aarch64-apple-darwin
          - os: macos-13
            target: x86_64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
          - os: ubuntu-22.04
            target: x86_64-unknown-linux-gnu
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: ${{ matrix.target }} }
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - uses: actions/setup-java@v4
        with: { distribution: temurin, java-version: '25' }   # kafbat-ui v1.5.0 运行时为 Java 25（已核实）
      - name: Linux 系统依赖
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
      - name: 拉 jar + 校验
        shell: bash
        run: ./scripts/fetch-kafbat-jar.sh
      - name: jlink JRE
        shell: bash
        run: ./scripts/build-jre.sh
      - name: 安装前端依赖
        run: npm ci
      - name: 构建并发布（tauri-action）
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
          # 可选签名：仅当 secrets 存在时生效
          APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
        with:
          args: --target ${{ matrix.target }}
          tagName: ${{ github.ref_name }}
          releaseName: "Kafka Console ${{ github.ref_name }}"
          releaseDraft: true
          includeUpdaterJson: true
```

> 说明：`tauri-action` 会自动生成并上传 `latest.json`（updater 用）。Apple 签名相关 env 缺失时自动跳过、产出未签名包。Windows 代码签名同理可后续在该 job 加 `signtool` 步骤并用 secrets 条件开关。

- [ ] **Step 2: 提交**

```bash
git add .github/workflows/release.yml
git commit -m "ci: 多平台发布流水线（矩阵构建 + Release + updater json）"
```

---

## Milestone M5 — 自动更新密钥与回填

### Task 5.1: 生成 updater 密钥并接线

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- 文档: `docs/superpowers/specs/`（无需改）

- [ ] **Step 1: 生成 Tauri updater 密钥对**

Run: `npm run tauri signer generate -- -w ~/.tauri/cy-kafka-console.key`
Expected: 输出公钥与私钥；私钥与口令保存为 GitHub secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。

- [ ] **Step 2: 回填公钥与发布端点**

将 `tauri.conf.json` 的 `plugins.updater.pubkey` 填为生成的公钥；`endpoints` 的 `<owner>` 改为实际 GitHub 仓库 owner。

- [ ] **Step 3: 编译验证**

Run: `cd src-tauri && cargo build`
Expected: 成功。

- [ ] **Step 4: 提交**

```bash
git add src-tauri/tauri.conf.json
git commit -m "feat(update): 接线 updater 公钥与发布端点"
```

- [ ] **Step 5: 端到端验证（手动，发布两个版本）**

打 `v0.1.0` tag → CI 出 draft release → 发布；提升版本到 `0.1.1` 再打 `v0.1.1`；安装 0.1.0 后启动应预期收到更新提示。记录结果。

---

## Milestone M6 — 文档完善

### Task 6.1: README 与工程实践文档

**Files:**
- Modify: `README.md`
- Create: `docs/DEVELOPMENT.md`

- [ ] **Step 1: 完善 README（用户向 + 开发者向）**

`README.md` 包含：产品简介、支持平台、下载安装（含 macOS 未签名首次放行说明）、首次使用（用配置向导加集群）、数据/日志目录位置、从源码构建步骤（`scripts/build-app.sh`）。

- [ ] **Step 2: 创建 DEVELOPMENT.md（落地 AI 流程与 Harness）**

`docs/DEVELOPMENT.md` 摘要并链接设计文档第 8、9 节：Spec-Driven + TDD + 子代理并行的工作约定；五层 Harness（确定性构建 / 可执行规格 / 真实环境验证 / 质量门禁 / 可观测可恢复）在本仓库的具体落点（脚本、CI job、测试）。

- [ ] **Step 3: 提交**

```bash
git add README.md docs/DEVELOPMENT.md
git commit -m "docs: 完善 README 与开发者文档（AI 流程 + Harness 实践）"
```

---

## Self-Review（计划对照设计自查）

**1. Spec coverage**
- 跨平台支持 → Task 0.3/4.1 矩阵覆盖 mac arm/x64、win、linux ✅
- 界面与原版一致 → 直接加载 kafbat-ui（Task 1.10 WebviewUrl::External）✅
- jlink 最小 JRE → Task 0.3 + 集成测试兜底 Task 3.2 ✅
- jar 下载+SHA256 → Task 0.2 ✅
- 生命周期状态机 + 错误兜底 → Task 1.9（成功/早退/超时）+ Task 1.11（错误视图）✅
- 安全 loopback → Task 1.5 build_env 强制 127.0.0.1 ✅
- 单实例/优雅退出 → Task 1.10 ✅
- 完整测试（单测/集成/E2E）→ M1 各任务 + Task 3.2 + 3.3 ✅
- CI 多平台矩阵 → Task 3.1/3.2/3.3/4.1 ✅
- 应用内自动更新 → Task 1.10 插件 + Task 4.1 updater json + Task 5.1 密钥 ✅
- 代码签名可选预留 → Task 4.1 env 条件开关 ✅
- AI 流程 + Harness 文档 → Task 6.1 ✅
- 覆盖率门禁 → Task 3.1（Rust 80% / 前端 80%）✅
- macOS E2E 限制说明 → Task 3.3 矩阵注释 ✅

**2. Placeholder scan**
- 代码步骤均含完整代码；`REPLACE_WITH_*` 为需人工提供的真实凭据/校验和（已在对应步骤说明何时回填），非代码占位 ✅

**3. Type consistency**
- `Resources{java_bin,jar}`、`LaunchConfig{port,config_file,log_dir,max_heap_mb}`、`SidecarManager{spawner,probe,sleeper}`、`StartParams{resources,config,max_attempts,poll_interval}`、`RunningSidecar{port,process}`、`HealthProbe::is_ready`、`ProcessSpawner::spawn`、`Sleeper::sleep`、`build_env`、`app_paths_from` 在各任务间命名/签名一致 ✅
- `FakeProbe`/`FakeSleeper` 在 health/clock 的 `tests` 模块定义，sidecar 测试 `use crate::health::tests::FakeProbe` 复用 ✅

**开放项**（继承自 spec 第 12 节，实施时核实）：~~JDK 精确版本~~（已核实：kafbat-ui v1.5.0 运行时为 **Java 25**，官方镜像 `azul/zulu-openjdk:25.0.2-jre-headless`，CI 与 jlink 均用 JDK 25）、jlink 模块集补全（由集成测试兜底）、健康端点路径、签名证书。
