import { invoke } from "@tauri-apps/api/core";

const DEFAULT_TAURI_INVOKE_TIMEOUT_MS = 60_000;

export type InvokeTauriOptions = {
  timeoutMs?: number | null;
};

function normalizeTimeoutMs(value: number | null | undefined) {
  if (value == null) return DEFAULT_TAURI_INVOKE_TIMEOUT_MS;
  if (!Number.isFinite(value) || value <= 0) return null;
  return Math.floor(value);
}

export async function invokeTauriOrNull<T>(
  cmd: string,
  args?: Record<string, unknown>,
  options?: InvokeTauriOptions
): Promise<T | null> {
  const invokePromise = invoke<T>(cmd, args);
  const timeoutMs = normalizeTimeoutMs(options?.timeoutMs);
  if (timeoutMs == null) return invokePromise;

  let timer: ReturnType<typeof setTimeout> | null = null;
  const timeoutPromise = new Promise<never>((_resolve, reject) => {
    timer = setTimeout(() => {
      reject(new Error(`IPC_TIMEOUT: ${cmd} timed out after ${timeoutMs}ms`));
    }, timeoutMs);
  });

  try {
    return await Promise.race([invokePromise, timeoutPromise]);
  } finally {
    if (timer != null) clearTimeout(timer);
  }
}
