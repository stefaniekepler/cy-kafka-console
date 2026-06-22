import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { simulatedProgress } from "./splash-progress";

export const APP_NAME = "Kafka Console";

// 预测的后端启动时长（经验值）：进度条据此模拟前进。真实就绪时 Rust 端直接关闭
// 本窗口，因此这里只需平滑逼近、不必精确——快了就提前消失，慢了就继续渐近。
const EXPECTED_MS = 14000;

let timer: ReturnType<typeof setInterval> | undefined;
let startedAt = 0;

function render(progress: number): void {
  const bar = document.getElementById("bar");
  if (bar) bar.style.width = `${(progress * 100).toFixed(1)}%`;
  const pct = document.getElementById("pct");
  if (pct) pct.textContent = `${Math.round(progress * 100)}%`;
}

function startProgress(): void {
  startedAt = performance.now();
  render(0);
  if (timer) clearInterval(timer);
  timer = setInterval(() => {
    render(simulatedProgress(performance.now() - startedAt, EXPECTED_MS));
  }, 120);
}

function stopProgress(): void {
  if (timer) clearInterval(timer);
  timer = undefined;
}

function showError(detail: string): void {
  stopProgress();
  document.getElementById("loading")?.setAttribute("hidden", "");
  const box = document.getElementById("error");
  box?.removeAttribute("hidden");
  const pre = document.getElementById("error-detail");
  if (pre) pre.textContent = detail;
}

function showLoading(): void {
  document.getElementById("error")?.setAttribute("hidden", "");
  document.getElementById("loading")?.removeAttribute("hidden");
  startProgress();
}

async function init(): Promise<void> {
  await listen<string>("startup-error", (e) => showError(e.payload));

  document.getElementById("retry")?.addEventListener("click", () => {
    showLoading();
    void invoke("retry_startup");
  });
  document
    .getElementById("open-logs")
    ?.addEventListener("click", () => void invoke("open_logs"));
  document
    .getElementById("quit")
    ?.addEventListener("click", () => void getCurrentWindow().close());

  // 兜底竞态：若错误在监听器注册前已发生，主动查询一次
  const existing = await invoke<string | null>("get_startup_error");
  if (existing) {
    showError(existing);
    return;
  }
  startProgress();
}

void init();
