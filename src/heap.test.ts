import { describe, it, expect } from "vitest";
import { validateHeapMb } from "./heap";

describe("validateHeapMb", () => {
  it("接受合理范围", () => {
    expect(validateHeapMb(512)).toEqual({ ok: true, value: 512 });
    expect(validateHeapMb(128)).toEqual({ ok: true, value: 128 });
    expect(validateHeapMb(8192)).toEqual({ ok: true, value: 8192 });
  });
  it("拒绝过小", () => {
    expect(validateHeapMb(64).ok).toBe(false);
  });
  it("拒绝过大", () => {
    expect(validateHeapMb(9000).ok).toBe(false);
  });
  it("拒绝非整数/NaN", () => {
    expect(validateHeapMb(1.5).ok).toBe(false);
    expect(validateHeapMb(NaN).ok).toBe(false);
  });
});
