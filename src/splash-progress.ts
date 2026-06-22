// 启动进度条的纯逻辑：后端就绪时刻无法预知（Rust 端就绪后直接切窗口，不通知
// splash），因此按“预测启动时长”模拟一条指数渐近、永不触顶的进度条——前快后慢，
// 真就绪时窗口被关闭即结束。把进度数学抽出为纯函数，便于单测、不计入 DOM 胶水。

/**
 * 按已耗时与预测总时长，返回模拟进度（0..0.99）。
 *
 * 采用指数渐近 `1 - e^(-t/τ)`：在 `expectedMs` 处约达 0.9，之后无限逼近却永不到
 * 1（封顶 0.99）。这样即便真实启动慢于预测，进度条也只会越走越慢而非卡死或骗到满。
 */
export function simulatedProgress(elapsedMs: number, expectedMs: number): number {
  if (elapsedMs <= 0) return 0;
  if (expectedMs <= 0) return 0.99;
  const tau = expectedMs / Math.LN10; // 使 elapsed===expected 时约为 0.9
  const p = 1 - Math.exp(-elapsedMs / tau);
  return Math.min(p, 0.99);
}
