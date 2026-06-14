// NOTE on multi-window handling:
// The app starts with a "splash" window (index.html) then opens a second "main" window
// pointing at the embedded kafbat-ui server before closing splash.
// tauri-driver currently exposes whichever window is active; switching between window
// handles (browser.getWindowHandles / browser.switchToWindow) may be needed once the
// main window opens.  This is expected to need iteration on the first real CI run —
// the test below is intentionally conservative and polls page source so it works even
// if we are still on the splash window or have already switched to the main one.

describe("Application launch", () => {
  it("shows splash heading then loads kafbat-ui interface", async () => {
    // Assert the splash screen heading contains the product name.
    // If tauri-driver has already switched to the main window this assertion may need
    // to be adapted to use browser.getWindowHandles() and switch back to splash first.
    const heading = await $("h1");
    await expect(heading).toHaveText(
      expect.stringContaining("Kafka Console"),
    );

    // Wait up to 90 s for the kafbat-ui UI to appear in the current window's page source.
    // Recognised markers: "kafbat" (brand name), "Dashboard", "Clusters", "Brokers".
    await browser.waitUntil(
      async () => {
        const html = await browser.getPageSource();
        return (
          html.includes("kafbat") ||
          html.includes("Dashboard") ||
          html.includes("Clusters") ||
          html.includes("Brokers")
        );
      },
      {
        timeout: 90_000,
        interval: 1_000,
        timeoutMsg:
          "kafbat-ui interface did not appear within 90 s. " +
          "If splash closed and a new window opened, window-handle switching may be required.",
      },
    );
  });
});
