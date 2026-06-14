import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";

export const APP_NAME = "Kafka Console";

function showError(detail: string): void {
  document.getElementById("loading")?.setAttribute("hidden", "");
  const box = document.getElementById("error");
  box?.removeAttribute("hidden");
  const pre = document.getElementById("error-detail");
  if (pre) pre.textContent = detail;
}

function showLoading(): void {
  document.getElementById("error")?.setAttribute("hidden", "");
  document.getElementById("loading")?.removeAttribute("hidden");
}

async function init(): Promise<void> {
  await listen<string>("startup-error", (e) => showError(e.payload));

  // 兜底竞态：若错误在监听器注册前已发生，主动查询一次
  const existing = await invoke<string | null>("get_startup_error");
  if (existing) showError(existing);

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
}

void init();
