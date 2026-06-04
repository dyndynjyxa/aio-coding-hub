import { describe, expect, it, vi } from "vitest";
import { tauriInvoke } from "../../../test/mocks/tauri";
import { writeDesktopClipboardText } from "../clipboard";

describe("services/desktop/clipboard", () => {
  it("normalizes clipboard text before invoking the backend", async () => {
    vi.mocked(tauriInvoke).mockResolvedValue(true as any);

    await expect(writeDesktopClipboardText("  hello  ")).resolves.toBe(true);

    expect(tauriInvoke).toHaveBeenCalledWith("desktop_clipboard_write_text", {
      text: "hello",
    });
  });

  it("rejects invalid clipboard text before invoking the backend", async () => {
    await expect(writeDesktopClipboardText("   ")).rejects.toThrow("clipboard text is required");
    await expect(writeDesktopClipboardText("x".repeat(1_000_001))).rejects.toThrow(
      "clipboard text is too long"
    );

    expect(tauriInvoke).not.toHaveBeenCalled();
  });
});
