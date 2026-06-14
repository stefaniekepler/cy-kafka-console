# Kafka Console

## 简介

Kafka Console 是一款跨平台桌面应用，将 [kafbat-ui v1.5.0](https://github.com/kafbat/kafka-ui) 封装为原生桌面客户端。底层架构为 **Tauri 2**（Rust + 系统 WebView），内嵌一个经 jlink 裁剪的 Java 25 运行时（约 57 MB，18 个模块），在本地以 sidecar 方式启动 kafbat-ui 的 Spring Boot 服务，随机绑定到 `127.0.0.1`（仅回环，不暴露给局域网），WebView 直接打开该本地端口。**用户界面即 kafbat-ui 原生界面**，功能与上游完全一致。

---

## 支持平台

| 平台 | 架构 |
|------|------|
| macOS | Apple Silicon (arm64) / Intel (x64) |
| Windows | x64 |
| Linux | x64 |

---

## 下载与安装

从 [GitHub Releases](https://github.com/OWNER/cy-kafka-console/releases) 下载对应平台的安装包。

### 首次运行提示（未签名/未公证二进制）

**macOS（未公证）**，系统会阻止直接打开，任选其一：

- 在 Finder 中右键点击应用 → **打开**
- 在终端执行：`xattr -dr com.apple.quarantine "/Applications/Kafka Console.app"`
- 前往 **系统设置 → 隐私与安全性 → 仍要打开**

**Windows（未签名）**，SmartScreen 弹出拦截界面时：

- 点击 **更多信息** → **仍要运行**

> 以上提示仅在未配置代码签名/公证时出现。正式签名后将自动消失。

---

## 功能亮点

- **系统托盘**：应用图标常驻托盘，菜单提供"显示主窗口 / 设置… / 退出"。关闭主窗口只是最小化到托盘，不会退出应用；只有点击托盘菜单"退出"或按 Cmd+Q 才真正退出。
- **可调 JVM 堆内存**：在设置窗口可调整最大堆大小（128–8192 MB），下次启动生效（默认 512 MB）。
- **后端崩溃自动重启**：运行中若后端意外退出，应用会自动重启一次并重新加载界面；再次崩溃则弹出错误对话框并退出。
- **应用内自动更新**：启动后自动检查 GitHub Releases 是否有新版本，有则弹框确认后自动下载安装重启。需发布到 GitHub Releases 后方可生效（见下方待办说明）。

---

## 首次使用

1. 启动 **Kafka Console**，等待启动画面消失（后端 kafbat-ui 服务就绪后 WebView 自动跳转）。
2. 原生 kafbat-ui 界面加载完成后，点击界面内的集群配置向导（**Configure new cluster**），填写 Bootstrap Servers 等连接信息，完成后保存。
3. 集群配置自动持久化到用户数据目录（见下节），下次启动无需重新配置。

---

## 数据与日志位置

| 平台 | 路径 |
|------|------|
| macOS | `~/Library/Application Support/com.cy.kafkaconsole/` |
| Windows / Linux | 平台标准应用数据目录（由系统决定） |

该目录下包含：

- `dynamic_config.yaml` — kafbat-ui 动态配置（集群连接信息）
- `settings.json` — 应用设置（如 JVM 最大堆大小）
- `logs/kafka-console.log` — Spring Boot 应用日志
- `logs/jvm.out.log` — JVM 标准输出/错误

遇到问题时，优先查阅以上日志文件。

---

## 自动更新待办（需 GitHub 仓库）

自动更新代码已就绪，实际启用前需完成：

1. 将 `src-tauri/tauri.conf.json` 中 `plugins.updater.endpoints` 里的 `OWNER` 占位符替换为真实 GitHub 仓库 owner。
2. 在仓库 secrets 中配置 `TAURI_SIGNING_PRIVATE_KEY` 与 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，用于对更新产物签名。

完成以上两步并推送 tag 触发 CI 构建后，应用内自动更新即可正常分发。

---

## 从源码构建

### 前置依赖

- **Rust**（stable 工具链）
- **Node.js 20+**（含 npm）
- **JDK 25**（需含 `jlink`，并在 `PATH` 中可访问 `jlink` 与 `java`）

### 构建步骤

**macOS / Linux：**

```bash
./scripts/build-app.sh
```

**Windows（PowerShell）：**

```powershell
.\scripts\build-app.ps1
```

脚本依次执行：下载并校验 kafbat jar（SHA-256 固定）→ jlink 裁剪 JRE → `npm run tauri build`。

产物位于 `src-tauri/target/release/bundle/`。

> **注意（macOS headless 环境）：** 在无 GUI 会话的 macOS 机器上，DMG 打包步骤可能失败，但 `.app` 仍可正常生成。GitHub Actions 的 macOS runner 拥有 GUI 会话，可完整产出 DMG。

---

## 许可证 / 上游致谢

本项目遵循 [Apache-2.0](LICENSE) 许可证。

内嵌的 kafbat-ui（[https://github.com/kafbat/kafka-ui](https://github.com/kafbat/kafka-ui)）同样采用 Apache-2.0 协议，本项目以**原始未修改形式**捆绑其发布 jar（`api-v1.5.0.jar`），版权归 kafbat-ui 作者所有。
