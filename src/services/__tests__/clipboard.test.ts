import { describe, expect, it, vi, beforeEach } from "vitest";
import { copyText } from "../clipboard";
import { writeDesktopClipboardText } from "../desktop/clipboard";

vi.mock("../desktop/clipboard", () => ({
  writeDesktopClipboardText: vi.fn(),
}));

describe("services/clipboard", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("uses tauri clipboard when runtime is available", async () => {
    vi.mocked(writeDesktopClipboardText).mockResolvedValue(true as any);

    await copyText("hello");

    expect(writeDesktopClipboardText).toHaveBeenCalledWith("hello");
  });

  it("falls back to navigator clipboard when tauri write fails", async () => {
    vi.mocked(writeDesktopClipboardText).mockRejectedValue(new Error("denied"));

    const navWrite = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText: navWrite },
      configurable: true,
    });

    await copyText("hello2");

    expect(navWrite).toHaveBeenCalledWith("hello2");
  });

  it("falls back to execCommand when tauri and navigator clipboard unavailable", async () => {
    vi.mocked(writeDesktopClipboardText).mockRejectedValue(new Error("tauri denied"));
    Object.defineProperty(navigator, "clipboard", {
      value: undefined,
      configurable: true,
    });

    const execSpy = vi.fn().mockReturnValue(true);
    Object.defineProperty(document, "execCommand", {
      value: execSpy,
      configurable: true,
    });

    await copyText("hello3");

    expect(execSpy).toHaveBeenCalledWith("copy");
  });
});
