/// <reference types="vitest/config" />
import { defineConfig } from "vite";

export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: "es2020", outDir: "dist", rollupOptions: { input: { main: "index.html", settings: "settings.html" } } },
  // all:false —— 仅对“写了测试的模块”计覆盖率（当前 heap.ts / splash-progress.ts 均 100%）。
  // 否则 vitest 默认 all:true 会把未测的胶水文件（settings.ts/splash.ts）与构建脚本一并算入分母。
  test: { environment: "jsdom", coverage: { provider: "v8", all: false, lines: 80 } },
});
