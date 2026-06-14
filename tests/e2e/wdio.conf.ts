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
      browserName: "wry",
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
    // Spawn tauri-driver; it listens on port 4444 and proxies WebDriver commands to the app.
    tauriDriver = spawn("tauri-driver", [], {
      stdio: [null, process.stdout, process.stderr],
    });
  },

  afterSession: () => {
    tauriDriver?.kill();
  },
};
