import { describe, it, expect } from "vitest";
import { simulatedProgress } from "./splash-progress";

describe("simulatedProgress", () => {
  it("starts at 0 for non-positive elapsed", () => {
    expect(simulatedProgress(0, 14000)).toBe(0);
    expect(simulatedProgress(-100, 14000)).toBe(0);
  });

  it("reaches ~0.9 at the predicted duration", () => {
    expect(simulatedProgress(14000, 14000)).toBeCloseTo(0.9, 2);
  });

  it("is strictly increasing over time", () => {
    expect(simulatedProgress(2000, 14000)).toBeLessThan(simulatedProgress(7000, 14000));
    expect(simulatedProgress(7000, 14000)).toBeLessThan(simulatedProgress(13000, 14000));
  });

  it("caps at 0.99 and never reaches 1", () => {
    expect(simulatedProgress(10_000_000, 14000)).toBe(0.99);
    expect(simulatedProgress(60000, 14000)).toBeLessThan(1);
  });

  it("degrades gracefully when predicted duration is non-positive", () => {
    expect(simulatedProgress(5000, 0)).toBe(0.99);
    expect(simulatedProgress(5000, -1)).toBe(0.99);
  });
});
