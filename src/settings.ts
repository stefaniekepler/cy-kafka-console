import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { validateHeapMb } from "./heap";

const input = document.getElementById("heap") as HTMLInputElement;
const hint = document.getElementById("hint")!;

async function load(): Promise<void> {
  const mb = await invoke<number>("get_max_heap");
  input.value = String(mb);
}

document.getElementById("save")?.addEventListener("click", async () => {
  const res = validateHeapMb(Number(input.value));
  if (!res.ok) {
    hint.textContent = res.error;
    hint.classList.add("err");
    return;
  }
  try {
    await invoke("set_max_heap", { mb: res.value });
    hint.classList.remove("err");
    hint.textContent = "已保存，将在下次启动应用时生效。";
  } catch (e) {
    hint.classList.add("err");
    hint.textContent = String(e);
  }
});

document.getElementById("cancel")?.addEventListener("click", () => void getCurrentWindow().close());

void load();
