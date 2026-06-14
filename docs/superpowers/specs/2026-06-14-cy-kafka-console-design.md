# Kafka Console（cy-kafka-console）设计文档

- 文档版本：v1.0
- 日期：2026-06-14
- 状态：已通过头脑风暴评审，待用户复核 → 进入实施计划
- 作者：Claude Code + 用户协作（Spec-Driven 流程）

---

## 1. 背景与目标

将开源项目 **kafbat-ui**（https://github.com/kafbat/kafka-ui）包装成一个**跨平台桌面客户端**，让用户像打开普通 App 一样双击即用，无需自行部署 Docker、配置 Java 或访问网页。

**产品名 / 命名约定**

| 用途 | 名称 |
|---|---|
| 产品显示名（窗口标题 / 关于页） | Kafka Console |
| 仓库 / 文件夹名 | `cy-kafka-console` |
| 应用二进制 / 标识 | `kafka-console` |
| Bundle ID | `com.cy.kafkaconsole` |

**支持平台**：macOS（Apple Silicon arm64 + Intel x64）、Windows（x64）、Linux（x64）。

**核心约束**：界面风格与原版 kafbat-ui 完全一致（直接复用其原生界面，不重写）；界面简洁高效；提供完整打包部署方案；包含完整单元测试与端到端测试；遵循 AI 开发全流程与 Harness Engineering 工程实践。

---

## 2. 关键事实（已核实，基于 kafbat-ui v1.5.0）

- 后端：Java + Spring Boot，Gradle 构建，发布**可运行 jar**（`api-<version>.jar`，如 `api-1.5.0.jar`，作为 GitHub Release 资产直接提供）。
- 前端：TypeScript + React，已编译进 jar，无需单独处理。
- **内置「Web UI 集群配置向导」**：开启 `DYNAMIC_CONFIG_ENABLED=true` 后，用户可直接在原生界面里增删 Kafka 集群 → 桌面壳**无需自建集群管理界面**，天然保持界面一致。
- Java 版本：**已核实为 Java 25**（v1.5.0 标签 `.java-version=25`，官方运行时镜像 `azul/zulu-openjdk:25.0.2-jre-headless`）；jlink 与 CI 均用 JDK 25，构建时由 CI 断言运行的 JDK 与之匹配。
- 最新版本：v1.5.0（2026-04）。

---

## 3. 已确认的关键决策

| 项 | 决策 | 理由 |
|---|---|---|
| 桌面外壳 | **Tauri 2.x**（Rust + 系统 WebView） | 体积小、内存低、安全模型好、原生 sidecar 机制适合拉起 Java 进程、自带多平台打包与自动更新 |
| Java 运行时 | **jlink 裁剪的最小运行时**，按平台内置 | 零依赖开箱即用、体积最小（~50–70MB） |
| kafbat-ui 获取 | 锁定版本，CI 下载官方 `api-<ver>.jar` 并校验 SHA-256 | 无需自建构建、防供应链篡改 |
| 分发范围 | 安装包（dmg/exe/AppImage） + GitHub Actions 多平台矩阵 + 应用内自动更新 | 用户选择；代码签名/公证预留为可选开关 |
| AI 开发流程 | Spec-Driven + TDD + 子代理并行 | 与 Harness Engineering 配套，工程严谨可复现 |
| 测试 | 完整单测 + 集成（真 Kafka + 真 JVM） + E2E（真窗口） | 用户要求；集成/E2E 兜底 jlink 模块完整性 |

**明确不做（YAGNI）**：不 fork kafbat-ui 源码、不自建后端、不重写其前端、不内置 Kafka、不依赖用户本机 Docker/Java。

---

## 4. 总体架构

不修改、不重写 kafbat-ui，将其作为**本地 sidecar 进程**整体包裹。Tauri 壳负责"拉起 JVM → 等就绪 → 把 WebView 指向 localhost"，用户看到的就是原生 kafbat-ui 界面。

```
┌─────────────────────────── Kafka Console (单一应用进程树) ───────────────────────────┐
│                                                                                      │
│   ┌──────────────── Tauri 外壳 (Rust Core) ────────────────┐                          │
│   │  · 启动编排器 (Sidecar Manager)                          │                          │
│   │    1. 选空闲端口 (bind 127.0.0.1:0 取端口)               │   spawn 子进程            │
│   │    2. 拉起内置 JRE 跑 jar                                ├──────────────┐           │
│   │    3. 轮询 /actuator/health 直到 UP                      │              ▼           │
│   │    4. 就绪后把主 WebView 指向 127.0.0.1:<port>           │   ┌────────────────────┐  │
│   │  · 单实例锁 / 系统托盘 / 优雅退出 (杀 JVM 进程树)         │   │ 内置 JRE (jlink)    │  │
│   │  · 自动更新                                              │   │  └ java -jar        │  │
│   └───────────────┬──────────────────────────────────────┘   │     api-<ver>.jar   │  │
│                   │ 加载页(splash) ←启动中                       │  (kafbat-ui 后端     │  │
│                   ▼                                            │   + 内置前端)        │  │
│   ┌──────────── 主 WebView ────────────┐                       └─────────┬──────────┘  │
│   │  http://127.0.0.1:<port>           │◀──────HTTP (仅 loopback)────────┘             │
│   │  = 原生 kafbat-ui 界面 + 配置向导    │                                               │
│   └────────────────────────────────────┘                                              │
│                                                                                       │
│   用户数据目录 (按 OS):  dynamic_config.yaml(集群)  ·  logs/  ·  jvm 配置覆盖             │
└───────────────────────────────────────────────────────────────────────────────────────┘
                                       │ 仅 127.0.0.1，不监听 LAN
                                       ▼
                              用户自己的 Kafka 集群
```

**架构层关键设计点**

1. **进程模型**：一个应用 = Tauri 父进程 + 一个 JVM 子进程。父进程持有子进程句柄，退出时优雅终止（SIGTERM → 超时 kill；Windows 用进程树 kill 防孤儿进程）。
2. **端口**：每次启动动态选空闲端口，避免冲突；**只绑定 127.0.0.1**，绝不暴露到局域网（kafbat-ui 默认无鉴权，绑 loopback 是关键安全措施）。
3. **就绪探测**：壳启动后显示加载页，轮询健康端点；超时（60s）显示可读错误页（含日志路径、重试按钮），不白屏。
4. **配置与数据**：`DYNAMIC_CONFIG_ENABLED=true`，配置文件落 OS 标准用户目录，升级/重装不丢集群配置：
   - macOS：`~/Library/Application Support/com.cy.kafkaconsole/`
   - Windows：`%APPDATA%\com.cy.kafkaconsole\`
   - Linux：`~/.config/cy-kafka-console/`
5. **单实例**：第二次启动只聚焦已有窗口，不再拉起第二个 JVM（tauri-plugin-single-instance）。
6. **JVM 资源**：默认 `-Xmx512m`，允许用户在设置里调整。

---

## 5. 运行时生命周期与错误处理（状态机）

壳核心是一个**确定性状态机**，每个状态有明确 UI 表现与失败兜底，也是 Rust 单测的主要对象。

```
            ┌─────────┐
            │ Startup │  应用启动
            └────┬────┘
                 ▼
        ┌─────────────────┐  另一个实例已在跑?
        │ SingleInstance  │──── 是 ──▶ 聚焦已有窗口 + 退出本次
        │   Check         │
        └────────┬────────┘ 否
                 ▼
        ┌─────────────────┐  失败: 找不到 jar/JRE 资源
        │ ResolveResources│──────────▶ FatalError(安装损坏)
        └────────┬────────┘
                 ▼
        ┌─────────────────┐  bind 127.0.0.1:0 → 取端口 → 释放
        │ AllocatePort    │
        └────────┬────────┘
                 ▼
        ┌─────────────────┐  显示 Splash(加载页)
        │ SpawnJvm        │  java -jar，注入端口/配置/日志路径 env
        └────────┬────────┘
                 │  子进程秒退(非0)? ──▶ FatalError(读最后N行日志展示)
                 ▼
        ┌─────────────────┐  轮询 /actuator/health 每500ms，最长60s
        │ WaitForReady    │
        └───┬─────────┬───┘
         UP │         │ 超时 / 进程死亡
            ▼         ▼
   ┌──────────────────┐  ┌────────────────────────────┐
   │ Running           │  │ StartupError                │
   │ WebView→localhost │  │ 显示错误页: 原因 + 日志路径   │
   │ (kafbat-ui)       │  │ + [重试] [打开日志] [退出]    │
   └──────┬───────────┘  └────────────────────────────┘
          │ 用户退出 / 窗口关闭
          ▼
   ┌──────────────────┐  SIGTERM → 等5s → 强杀进程树
   │ Shutdown          │  确保无孤儿 JVM；落盘日志
   └──────────────────┘
```

**SpawnJvm 注入的环境变量**
- `SERVER_PORT=<分配端口>`、`SERVER_ADDRESS=127.0.0.1`（强制 loopback）
- `DYNAMIC_CONFIG_ENABLED=true`、`DYNAMIC_CONFIG_PATH=<用户目录>/dynamic_config.yaml`
- 日志重定向到 `<用户目录>/logs/`
- `JAVA_TOOL_OPTIONS` 注入 `-Xmx`（默认 512m，可被用户设置覆盖）

**错误处理原则（"绝不白屏"）**

| 失败点 | 用户看到 | 可恢复性 |
|---|---|---|
| 资源缺失/损坏 | "安装似乎已损坏，请重装" + 路径 | 不可恢复，引导重装 |
| JVM 秒退（端口被占/jar 损坏/JRE 模块缺失） | 错误页 + 日志最后 N 行 + 重试 | 重试（重新选端口） |
| 60s 未就绪 | "启动超时" + 打开日志按钮 | 重试 / 退出 |
| 运行中 JVM 崩溃 | 横幅提示 + 自动尝试重启一次 | 自动重启 1 次，再失败转错误页 |

**可测试性抽象**：状态机与"真正 spawn 进程 / 发 HTTP"解耦——通过 trait 抽象 `ProcessSpawner` 与 `HealthProbe`，单测注入假实现，可在不启动真 JVM 的情况下覆盖**所有状态转移与失败分支**。

---

## 6. 打包与分发

### 6.1 构建产物拼装

```
最终安装包 = Tauri 壳(原生二进制) + resources/jre/(jlink 运行时) + resources/kafbat/api-<ver>.jar
```

**① 获取 kafbat-ui jar**（`scripts/fetch-kafbat-jar`）
- 仓库固化 `KAFBAT_VERSION` + 期望 `api-<ver>.jar.sha256`。
- CI 从官方 Release 下载 → 校验 SHA-256（不匹配则失败）→ 放入 `resources/kafbat/`。
- jar 不入库（.gitignore），由 CI 拉取并缓存。

**② jlink 最小运行时**（`scripts/build-jre`，各平台 runner 各自构建）
- 用与 kafbat-ui `.java-version` 一致的 Temurin **JDK**；CI 断言 `java -version` 匹配。
- 起步模块集（Spring Boot 安全集）：`java.base, java.management, java.naming, java.net.http, java.sql, java.security.sasl, java.security.jgss, jdk.crypto.ec, jdk.crypto.cryptoki, jdk.unsupported, jdk.zipfs, java.desktop, java.instrument, jdk.management` 等。
- 参数：`--strip-debug --no-header-files --no-man-pages --compress=zip-9`，产出 ~50–70MB。
- **模块集正确性由集成/E2E 兜底**：跑真 JVM 连真 Kafka，缺模块会报 `ClassNotFound/Provider not found` → 补模块。不靠人肉静态分析。

**③ Tauri 打包**
- `jre/` 与 jar 作为 `bundle.resources` 打包；Rust 运行时按 `resource_dir()` 解析路径再 spawn `jre/bin/java`。
- 每平台产物：
  - macOS：`.dmg`（arm64 + x64 两份，可选合成 universal）
  - Windows：`.exe`（NSIS，体积友好、安装灵活）
  - Linux：`.AppImage`（首选，免依赖）+ 可选 `.deb`

### 6.2 CI/CD（GitHub Actions）

```
.github/workflows/
├─ ci.yml       # PR/push: 三平台并行 → lint(clippy/eslint/fmt) + 单测 + 集成 + E2E
└─ release.yml  # 打 tag v* → 构建矩阵 → 签名(可选) → 上传 GitHub Release + latest.json
```

发布矩阵（4 目标）：

| OS Runner | 目标 | 产物 |
|---|---|---|
| macos-14 (arm) | aarch64-apple-darwin | dmg |
| macos-13 (intel) | x86_64-apple-darwin | dmg |
| windows-latest | x86_64-pc-windows | exe (NSIS) |
| ubuntu-22.04 | x86_64-unknown-linux | AppImage (+deb) |

每 job 顺序：拉 jar + 校验 → jlink → `tauri build` →（可选签名）→ 产物上传。

### 6.3 代码签名 / 公证（预留，可选开关）
- 流水线写好步骤，用 **secrets 是否存在**做条件开关：有证书就签，无则跳过并产出未签名包（首次安装有系统告警，README 说明手动放行）。
- macOS：`codesign` + `notarytool`（需 Apple Developer，$99/年）。
- Windows：`signtool`（需代码签名证书）。
- 拿到证书后只需把私钥/口令放进仓库 secrets，无需改流水线。

### 6.4 应用内自动更新
- `tauri-plugin-updater`：用 Tauri 自有签名密钥对产物签名，发布 `latest.json` 到 GitHub Release；启动时静默检查 → 提示"立即更新/稍后"。
- **诚实说明**：更新器签名独立于 Apple 公证，机制可跑；但 macOS 未公证时下载的更新仍会被 Gatekeeper 隔离、可能弹提示。要彻底顺滑建议后续补公证（6.3 已预留）。

---

## 7. 测试策略

测试金字塔 + "真实依赖优先"。kafbat-ui 自身前后端测试归上游，**本项目只测自己写的壳与集成边界**。

### 7.1 单元测试（快、多、覆盖分支）
**Rust 壳**（`cargo nextest`，主战场，对应第 5 节状态机）：
- 端口分配、资源路径解析、环境变量组装。
- **状态机全分支**：注入假 `ProcessSpawner`（秒退/正常/崩溃）+ 假 `HealthProbe`（UP/超时），覆盖 SpawnJvm → WaitForReady → Running / StartupError / 自动重启 / Shutdown 全路径。
- 进程树终止逻辑（mock，验证信号顺序与超时升级）。

**前端**（`Vitest` + Testing Library，范围小）：
- 仅自建窗口（splash 加载页、错误页、设置面板 Xmx 校验逻辑）。原生 kafbat-ui 界面不在此测。

### 7.2 集成测试（无 GUI，验证"真 JVM + 真 Kafka"）
- **Testcontainers** 起真实 Kafka 容器。
- 用打好的 jlink 运行时真实 `java -jar api.jar` 拉起后端（headless）。
- 断言：①`/actuator/health` 超时内 UP（证明模块集够用、jar 能跑）；②经 REST API 写集群配置 → 列 topic → 创建/读取一条消息成功。
- 跨平台都在 CI 矩阵跑 → 任一平台模块缺失立即暴露。

### 7.3 端到端测试（GUI，真用户路径）
- 技术：**tauri-driver + WebDriverIO**（Tauri 官方 E2E 方案）。
- 黄金路径：启动 → splash → 主窗口加载 kafbat-ui → 向导填入指向 Testcontainers Kafka 的地址 → 看到 broker/topic 列表 → 关窗 → 断言无残留 JVM 进程。
- 异常路径：端口被占可重试；坏 jar 路径触发错误页。
- **平台限制（明示）**：tauri-driver 的 GUI E2E 在 **Linux + Windows** 可靠运行；**macOS** 因 Apple 不提供 WKWebView 的 WebDriver，GUI E2E 无法在 CI 跑 → macOS 用集成测试全覆盖后端链路 + 发布前人工冒烟补足。

### 7.4 覆盖率门禁与 CI 集成
- Rust 用 `cargo-llvm-cov`、前端用 Vitest coverage，设阈值（壳逻辑行覆盖 ≥ 80%）作为 CI 硬门禁。
- `ci.yml` 顺序：lint（clippy + eslint + fmt）→ 单测 + 覆盖率 → 集成（含 Testcontainers）→ E2E（Linux/Win）。任一红则阻断合并。

---

## 8. AI 开发全流程（Spec-Driven + TDD + 子代理并行）

完整链路，每步有产物、有门禁：

```
① 头脑风暴(brainstorming)  → 确认需求与决策
② 规格 spec               → 本设计文档
③ 实施计划 plan           → writing-plans 生成可执行任务清单（带验收标准）
④ TDD 实现(红-绿-重构)     → 每任务先写失败测试 → 实现到通过 → 重构
   (独立任务用子代理并行: 状态机 / jlink脚本 / CI / E2E 分头推进)
⑤ 代码评审(code review)    → requesting-code-review，对照 spec 验收
⑥ 完成前验证(verification) → 跑真命令拿真输出，证据先行
⑦ 收尾(finishing-branch)   → 合并 / PR
```

**关键纪律**
- **规格驱动**：先有 spec/plan 再写码；需求变更先改 spec；文档是唯一事实源。
- **TDD 强制**：壳逻辑、脚本、集成先红后绿；测试即可执行的规格。
- **子代理并行**：互不依赖的工作块（Rust 状态机 / jlink 脚本 / CI / E2E）派发并行子代理，各自隔离工作区（worktree），收敛时统一评审。
- **证据先行**：任何"完成/修复/通过"结论必须附真实命令输出，禁止臆断。
- **协作产物入库**：spec、plan、评审记录进 `docs/`，流程可追溯可复现。

---

## 9. Harness Engineering 工程实践

核心理念：**不直接信任 AI 产出，而是投资"夹具"（harness）让 AI 能安全、可重复地迭代**——夹具越强，AI 自主度越高、结果越可信。本项目夹具由五层构成：

| 夹具层 | 本项目落地 |
|---|---|
| **确定性构建** | 锁定 kafbat-ui 版本 + SHA-256 校验；锁定 JDK 版本（CI 断言）；jlink 模块集固化。任何机器构建一致。 |
| **可执行规格** | spec → plan → 测试三级对齐；测试是 AI 自我验证标尺，红/绿给出明确反馈信号。 |
| **真实环境验证** | Testcontainers 起真 Kafka + 真 JVM 集成，E2E 驱动真窗口。E2E/集成是地面真相，兜底 jlink 模块缺失这类静态分析测不出的问题。 |
| **质量门禁** | CI 硬门禁：lint + 覆盖率阈值 + 集成 + E2E，全绿才可合并；红灯即停。AI 改动穿过同一道门。 |
| **可观测与可恢复** | 状态机每步日志落盘 + 用户可见错误页 + 日志路径；失败有明确信号供定位，而非白屏黑盒。 |

一句话：**spec/plan 是控制结构，TDD 是反馈回路，CI 门禁是安全网，真实环境测试是地面真相**——四者合起来即让 AI 放手干又不出格的 Harness。

---

## 10. 仓库结构

```
cy-kafka-console/
├─ src-tauri/        # Rust 壳: core / sidecar_manager(状态机) / commands / tray
│  ├─ src/
│  ├─ tauri.conf.json
│  └─ Cargo.toml
├─ src/             # 自建窗口前端(splash/error/settings), Vite + TS
├─ resources/
│  └─ kafbat/       # api-<ver>.jar (CI 拉取, gitignore) + .sha256
├─ scripts/
│  ├─ fetch-kafbat-jar.*   # 下载 + 校验 jar
│  └─ build-jre.*          # jlink 各平台
├─ tests/
│  ├─ integration/  # headless: 真 JVM + Testcontainers Kafka
│  └─ e2e/          # tauri-driver + WebDriverIO
├─ .github/workflows/
│  ├─ ci.yml
│  └─ release.yml
├─ docs/superpowers/specs/  # 设计文档
└─ README.md
```

---

## 11. 里程碑

| 里程碑 | 内容 |
|---|---|
| **M0** | 脚手架 + CI 骨架 |
| **M1** | sidecar 生命周期（本机跑通：拉 JVM → 就绪 → 加载界面） |
| **M2** | jlink + 打包，本地产出三平台安装包 |
| **M3** | CI 矩阵三平台自动构建 |
| **M4** | 测试三件套（单测/集成/E2E）+ 覆盖率门禁 |
| **M5** | 自动更新（+ 可选签名预留） |
| **M6** | 文档（Harness + AI 流程）完善 + 首个 Release |

---

## 12. 开放项（最终实现态）

- ✅ **JDK 版本**：已确认为 **Java 25**（v1.5.0 `.java-version=25` / 官方镜像 Zulu 25.0.2-jre）；CI 与 jlink 均用 JDK 25，构建脚本断言匹配。
- ✅ **jlink 模块集**：已通过本地集成测试（`backend_boots_on_bundled_runtime`）验证 18 个模块充分，后端在精简运行时正常启动。
- ✅ **健康端点**：已确认 `/actuator/health` 返回 `{"status":"UP"}`，无需额外配置。
- ✅ **用户可调堆内存**：设置窗口已实现（128–8192 MB），持久化到 `settings.json`，下次启动生效，默认 512 MB。
- ✅ **系统托盘**：已实现。托盘图标常驻，菜单含"显示主窗口 / 设置… / 退出"；关闭主窗口 = 最小化到托盘；仅托盘"退出"或 Cmd+Q 真正退出。
- ✅ **崩溃自动重启**：已实现。运行中后端意外退出自动重启一次并重新加载界面；再次崩溃弹原生错误对话框并退出。
- ⏳ **代码签名/公证**：仍未持有证书，CI 流水线已预留可选开关（secrets 存在则签，不存在则跳过），后续补 secrets 即生效。
- ⏳ **GitHub 仓库 + 自动更新分发**：自动更新的 Rust 端代码已就绪（启动后异步 `updater.check()` → 确认框 → 下载安装 → 重启）。实际启用需：(a) 将 `src-tauri/tauri.conf.json` 中 `plugins.updater.endpoints` 里的 `OWNER` 占位符替换为真实仓库 owner；(b) 在仓库 secrets 中配置 `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。完成后推送 `v*` tag 即可触发 `release.yml` 并分发更新。

---

_本文档遵循 Spec-Driven 流程产出；下一步经用户复核后由 writing-plans 生成实施计划。_
