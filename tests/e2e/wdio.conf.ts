import { spawn } from "node:child_process";
import type { ChildProcess } from "node:child_process";

let tauriDriver: ChildProcess | undefined;

// APP_BINARY is injected by CI (see .github/workflows/ci.yml e2e job).
// Locally, set it before running: APP_BINARY=../../src-tauri/target/release/kafka-console npm test
const appBinary = process.env.APP_BINARY;
if (!appBinary) {
  throw new Error("APP_BINARY environment variable must be set to the path of the built kafka-console binary");
}

export const config: WebdriverIO.Config = {
  specs: ["./specs/**/*.e2e.ts"],
  maxInstances: 1,
  capabilities: [
    {
      // 不要设 browserName（如 "wry"）—— 它不是底层 WebKitWebDriver 认识的浏览器名，
      // 会在创建会话时被判为 “Failed to match capabilities”。"wry" 只是 driver 回报的
      // 引擎名，不应由客户端请求。对齐 Tauri 官方 wdio 9 示例：仅用 tauri:options。
      // @ts-expect-error — tauri:options is a Tauri-specific WebDriver capability not in the standard types
      "tauri:options": {
        application: appBinary,
      },
    },
  ],
  reporters: ["spec"],
  framework: "mocha",
  mochaOpts: {
    timeout: 120_000,
  },
  hostname: "127.0.0.1",
  port: 4444,

  beforeSession: () => {
    // 告知应用处于 E2E 模式：后端就绪后保留 splash 窗口（不关闭），作为 WebDriver
    // 稳定观测窗口。tauri-driver 由本进程派生、再派生应用，故 env 会逐级继承下去。
    process.env.KAFKA_CONSOLE_E2E = "1";
    // Spawn tauri-driver; it listens on port 4444 and proxies WebDriver commands to the app.
    tauriDriver = spawn("tauri-driver", [], {
      stdio: [null, process.stdout, process.stderr],
    });
  },

  afterSession: () => {
    tauriDriver?.kill();
  },
};
