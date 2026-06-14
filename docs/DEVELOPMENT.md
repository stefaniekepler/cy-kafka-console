# 开发者文档

## 工程总览

### 仓库布局

```
cy-kafka-console/
├── src-tauri/              # Rust 核心（Tauri 2）
│   ├── src/
│   │   ├── main.rs         # Tauri 应用入口 + 窗口管理 + RunEvent 兜底
│   │   ├── lib.rs          # 库 crate 公开导出
│   │   ├── sidecar.rs      # SidecarManager：启动/健康检查状态机
│   │   ├── process.rs      # OsProcessSpawner（可注入 trait）
│   │   ├── health.rs       # HttpHealthProbe（可注入 trait）
│   │   ├── config.rs       # LaunchConfig（端口、堆大小、路径）
│   │   ├── resources.rs    # 解析 bundle 内 jre/kafbat 路径
│   │   ├── paths.rs        # 用户数据目录（directories crate）
│   │   ├── port.rs         # 随机空闲端口分配
│   │   ├── clock.rs        # RealSleeper（测试可替换）
│   │   └── error.rs        # 统一错误类型
│   ├── tests/
│   │   └── integration_sidecar.rs   # 集成测试（真 JRE + 真 jar）
│   ├── capabilities/       # Tauri 权限声明
│   └── tauri.conf.json     # bundle 配置（资源映射、更新器等）
├── src/                    # 前端（启动画面 / 错误视图，Vite + TypeScript）
├── resources/              # 构建产物（gitignored）
│   ├── jre/                # jlink 裁剪后的 Java 25 运行时
│   └── kafbat/             # api-v1.5.0.jar
├── scripts/
│   ├── kafbat.env          # KAFBAT_VERSION / KAFBAT_URL / KAFBAT_SHA256
│   ├── jre-modules.txt     # jlink 固化模块列表（18 个）
│   ├── fetch-kafbat-jar.sh / .ps1   # 下载并 SHA-256 校验 jar
│   ├── build-jre.sh / .ps1          # jlink 生成最小运行时
│   └── build-app.sh / .ps1          # 一键全量构建
├── tests/
│   └── e2e/                # tauri-driver WebDriver E2E 测试
├── docs/
│   ├── superpowers/
│   │   ├── specs/          # 设计规格文档
│   │   └── plans/          # 实施计划文档
│   └── DEVELOPMENT.md      # 本文件
└── .github/workflows/
    ├── ci.yml              # 持续集成（lint/单测/覆盖率/集成/E2E）
    └── release.yml         # 多平台发布（矩阵构建 + 更新器 latest.json）
```

---

## AI 开发全流程

本项目采用 **Spec-Driven + TDD + 子代理并行** 的 AI 辅助开发流程，分为以下阶段：

1. **头脑风暴（brainstorm）** — 明确产品目标、核心约束与架构方向。
2. **规格撰写（spec）** — 在 `docs/superpowers/specs/` 下产出设计文档，涵盖架构图、状态机、接口契约、测试策略。
3. **计划拆解（plan）** — 在 `docs/superpowers/plans/` 下将规格分解为可并行执行的任务单元（Milestone × Task）。
4. **TDD 先行** — 每个模块先写单测 / 集成测试，再实现，保证可执行规格先于实现存在。
5. **子代理并行实现（subagent impl）** — 独立任务由子代理并发执行，互不阻塞。
6. **两阶段评审（two-stage review）** — 先自查（clippy/fmt/覆盖率），再人工/AI 复审接口契约与安全边界。
7. **真实环境验证（verify）** — 集成测试与 E2E 测试在真实运行时中跑通，本地端到端验证后方可合并。

规格文档入口：`docs/superpowers/specs/2026-06-14-cy-kafka-console-design.md`

---

## Harness Engineering 五层

### 1. 确定性构建

| 层面 | 实现 |
|------|------|
| kafbat 版本锁定 | `scripts/kafbat.env`：固定 `KAFBAT_VERSION=1.5.0`、下载 URL 与 SHA-256（`8bebff7b…`），CI 下载后强制校验，不匹配则中止 |
| JDK 版本锁定 | `scripts/build-jre.sh` 启动时断言 `java -version` 输出为 25，版本不符立即退出 |
| jlink 模块固化 | `scripts/jre-modules.txt` 枚举 18 个模块，脚本逐字传入 `jlink --add-modules`，保证运行时体积与行为可复现 |

### 2. 可执行规格

- **单元测试**：`src-tauri/src/*.rs` 中 `#[cfg(test)]` 块，覆盖端口分配、健康探针、配置解析等纯逻辑。
- **集成测试**：`src-tauri/tests/integration_sidecar.rs`——用真实 jlink 运行时启动真实 kafbat-ui jar，断言 `/actuator/health` 返回 UP，随后干净终止进程。

### 3. 真实环境验证

集成测试分两个场景（均标记 `#[ignore]`，需显式运行）：

| 测试 | 场景 | 运行条件 |
|------|------|---------|
| `backend_boots_on_bundled_runtime` | 仅验证 JVM + jar 启动与健康检查 | 无需 Docker（macOS/Windows CI 跑此版） |
| `backend_boots_with_real_kafka` | 同上 + 真实 Kafka 容器（Testcontainers） | 需 Docker（Linux CI 跑全部） |

`backend_boots_on_bundled_runtime` 已在本地端到端验证通过（健康端点 UP，WebView 加载 kafbat-ui 原生 UI，Cmd+Q 无孤儿 JVM）。

### 4. 质量门禁（`.github/workflows/ci.yml`）

| 门禁 | 内容 |
|------|------|
| `rust` job | `cargo fmt --check` + `cargo clippy -- -D warnings` + 覆盖率 ≥ 80%（`cargo llvm-cov --fail-under-lines 80`） |
| `frontend` job | ESLint + Vitest 覆盖率 |
| `integration` job | 三平台（ubuntu/windows/macos）跑集成测试 |
| `e2e` job | Linux + Windows 跑 tauri-driver WebDriver E2E（macOS 因 WKWebView 无 WebDriver 实现而排除） |

### 5. 可观测与可恢复

- **状态机日志落盘**：`SidecarManager` 每次状态转换写入 `logs/kafka-console.log`（Spring 应用日志）与 `logs/jvm.out.log`（JVM stdout/stderr）。
- **Splash 错误视图**：启动超时或异常时，前端显示错误画面并提供 **Retry** 按钮。
- **RunEvent 兜底清理**：`main.rs` 监听 `RunEvent::Exit`，确保任何退出路径（含 Cmd+Q、Dock 退出）均终止 JVM，避免孤儿进程。
- **崩溃看门狗自动重启**：运行中后端意外退出时，自动重启一次并重新加载界面；再次崩溃则弹原生错误对话框并退出。
- **系统托盘**：应用图标常驻系统托盘，菜单提供"显示主窗口 / 设置… / 退出"；关闭主窗口 = 最小化到托盘，仅托盘"退出"或 Cmd+Q 真正退出。
- **设置窗口**：用户可在设置窗口调整最大 JVM 堆（128–8192 MB），持久化到用户数据目录 `settings.json`，下次启动生效（默认 512 MB）。

---

## 常用命令

```bash
# 仅运行单元测试
cd src-tauri && cargo test --lib

# 运行集成测试（需先执行 scripts/fetch-kafbat-jar + scripts/build-jre）
cd src-tauri && cargo test --test integration_sidecar -- --ignored --nocapture

# 运行带 Kafka 容器的集成测试（需 Docker）
cd src-tauri && cargo test --test integration_sidecar backend_boots_with_real_kafka -- --ignored --nocapture

# 前端测试
npm test

# 前端测试 + 覆盖率
npm run test:coverage

# 全量构建（产物在 src-tauri/target/release/bundle/）
npm run tauri build
```

---

## 实现说明（与计划的关键偏差 / 经验总结）

### bundle.resources 使用映射形式

`tauri.conf.json` 中资源配置为：

```json
"resources": { "../resources/jre": "jre", "../resources/kafbat": "kafbat" }
```

若改用数组形式（`["../resources/jre", "../resources/kafbat"]`），Tauri 会在目标路径中插入 `_up_` 前缀（反映相对路径的 `../`），导致 `resources::resolve()` 在运行时找不到 jre / jar。映射形式直接指定目标名称，规避该问题。

### RunEvent::Exit 兜底终止 JVM

Tauri 的窗口 `Destroyed` 事件不覆盖所有退出路径（如 macOS 的 Cmd+Q 或 Dock 菜单退出）。在 `main.rs` 中额外监听 `RunEvent::Exit`，保证 JVM 进程在任何情况下都被终止。

### JAVA_TOOL_OPTIONS 增加 `--add-opens`

JVM 启动参数中加入：

```
-Xmx512m
--add-opens=java.rmi/javax.rmi.ssl=ALL-UNNAMED
```

`--add-opens` 解决 kafbat-ui 通过 JMX-over-SSL 采集指标时的模块访问限制，该问题在真实运行测试中发现。

### macOS headless 下 DMG 打包失败

在无 GUI 会话的 macOS 环境（如本地 CI 或 SSH 会话）中，`hdiutil` 创建 DMG 时可能失败，但 `.app` 仍正常生成。GitHub Actions 的 macOS runner 拥有完整 GUI 会话，可完整产出 DMG——这是正式发布时使用 CI 构建产物的原因之一。

### JDK 版本确认为 25

kafbat-ui v1.5.0 运行时要求 Java 25。`scripts/build-jre.sh` 中已有版本断言；CI 通过 `actions/setup-java@v4`（`java-version: '25'`）固定版本。

---

## 待办（需 GitHub 仓库）

自动更新与 CI/CD 发布的 Rust 端代码已就绪，但以下配置尚未完成，需在建好 GitHub 仓库后操作：

1. **替换 `OWNER` 占位符**：在 `src-tauri/tauri.conf.json` 的 `plugins.updater.endpoints` 中将 `OWNER` 替换为真实 GitHub 仓库 owner，以使更新检查指向正确的 `latest.json`。
2. **配置更新器签名 secrets**：在仓库 Settings → Secrets 中添加 `TAURI_SIGNING_PRIVATE_KEY` 与 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，`release.yml` 读取这两个 secret 对产物签名后才能通过 Tauri 更新器的完整性校验。

完成以上两步后，推送 `v*` tag 即可触发 `release.yml` 矩阵构建并自动发布。
