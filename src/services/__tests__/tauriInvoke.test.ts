import { describe, expect, it, vi } from "vitest";
import { tauriInvoke } from "../../test/mocks/tauri";
import { invokeTauriOrNull } from "../tauriInvoke";

describe("services/tauriInvoke", () => {
  it("invokeTauriOrNull calls @tauri-apps/api/core.invoke with runtime", async () => {
    vi.mocked(tauriInvoke).mockResolvedValueOnce({ ok: true });

    await expect(invokeTauriOrNull("cmd", { a: 1 })).resolves.toEqual({ ok: true });
    expect(tauriInvoke).toHaveBeenCalledWith("cmd", { a: 1 });
  });

  it("invokeTauriOrNull rejects on default timeout", async () => {
    vi.useFakeTimers();
    try {
      vi.mocked(tauriInvoke).mockImplementationOnce(() => new Promise(() => {}));

      const pending = invokeTauriOrNull("cmd-timeout");
      const assertion = expect(pending).rejects.toThrow(
        "IPC_TIMEOUT: cmd-timeout timed out after 60000ms"
      );

      await vi.advanceTimersByTimeAsync(60_000);
      await assertion;
    } finally {
      vi.useRealTimers();
    }
  });

  it("invokeTauriOrNull supports custom timeoutMs", async () => {
    vi.useFakeTimers();
    try {
      vi.mocked(tauriInvoke).mockImplementationOnce(() => new Promise(() => {}));

      const pending = invokeTauriOrNull("cmd-custom-timeout", undefined, { timeoutMs: 25 });
      const assertion = expect(pending).rejects.toThrow(
        "IPC_TIMEOUT: cmd-custom-timeout timed out after 25ms"
      );

      await vi.advanceTimersByTimeAsync(25);
      await assertion;
    } finally {
      vi.useRealTimers();
    }
  });

  it("invokeTauriOrNull disables timeout when timeoutMs <= 0", async () => {
    vi.useFakeTimers();
    try {
      vi.mocked(tauriInvoke).mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            setTimeout(() => resolve({ ok: true }), 100);
          })
      );

      const pending = invokeTauriOrNull("cmd-no-timeout", undefined, { timeoutMs: 0 });
      await vi.advanceTimersByTimeAsync(100);

      await expect(pending).resolves.toEqual({ ok: true });
    } finally {
      vi.useRealTimers();
    }
  });
});
