// 多窗口与 WebDriver 的关系（重要）：
// 应用先开一个 "splash" 窗口（本地 index.html，受信、具备 Tauri 能力），后端就绪后
// 再开一个 "main" 窗口指向内嵌 kafbat-ui。main 出于安全设计【不授予任何 Tauri 能力】
// （capabilities/default.json 只覆盖 splash/settings）。
//
// tauri-driver 在 Linux(WebKitWebDriver)/Windows 上只能可靠驱动会话初始绑定的窗口，
// 应用运行时自行 new 出来的 main 窗口不会注册进 automation session，getWindowHandles/
// switchToWindow 无法可靠切过去。因此本测试【不切窗口】，全程停留在稳定的 splash 上：
// 后端就绪时 Rust 向 splash 发 "startup-ready"，splash 置 body[data-ready=1]；E2E 模式下
// splash 不关闭（见 lib.rs 的 KAFKA_CONSOLE_E2E 分支），故该标记可被稳定断言。
// 这把断言粒度定在“启动 happy-path 完成（splash 渲染 + 后端健康 + main 建成）”，
// 这是在 tauri-driver 无法驱动 capability-less 远程窗口的前提下唯一可靠的烟囱粒度。

describe("Application launch", () => {
  it("shows splash heading then reaches backend-ready state", async () => {
    // splash 标题含产品名。
    const heading = await $("h1");
    await expect(heading).toHaveText(
      expect.stringContaining("Kafka Console"),
    );

    // 最多等 90s：后端就绪后 splash 会被打上 data-ready=1。
    await browser.waitUntil(
      async () =>
        browser.execute(
          () => document.body.getAttribute("data-ready") === "1",
        ),
      {
        timeout: 90_000,
        interval: 1_000,
        timeoutMsg:
          "应用未在 90s 内进入 backend-ready 状态：splash 未收到 startup-ready。" +
          "可能是 sidecar 启动失败或健康探测未通过——查看应用日志。",
      },
    );
  });
});
