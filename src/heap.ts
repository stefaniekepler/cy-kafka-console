export type HeapResult = { ok: true; value: number } | { ok: false; error: string };

/** 校验用户填写的 JVM 最大堆（MB）。 */
export function validateHeapMb(mb: number): HeapResult {
  if (!Number.isInteger(mb)) return { ok: false, error: "必须为整数 MB" };
  if (mb < 128) return { ok: false, error: "至少 128MB" };
  if (mb > 8192) return { ok: false, error: "最多 8192MB" };
  return { ok: true, value: mb };
}
