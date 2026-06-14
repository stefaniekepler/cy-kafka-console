/// <reference types="vitest/config" />
import { defineConfig } from "vite";

export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: "es2020", outDir: "dist", rollupOptions: { input: { main: "index.html", settings: "settings.html" } } },
  test: { environment: "jsdom", coverage: { provider: "v8", lines: 80 } },
});
