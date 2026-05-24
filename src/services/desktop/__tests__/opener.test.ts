import { describe, expect, it, vi } from "vitest";
import {
  tauriInvoke,
  tauriOpenPath,
  tauriOpenUrl,
  tauriRevealItemInDir,
} from "../../../test/mocks/tauri";
import { openDesktopPath, openDesktopUrl, revealDesktopItem } from "../opener";

describe("services/desktop/opener", () => {
  it("opens desktop urls through the backend-owned desktop command", async () => {
    vi.mocked(tauriOpenUrl).mockResolvedValue(undefined as any);

    await expect(openDesktopUrl("https://example.com/releases")).resolves.toBe(true);
    expect(tauriOpenUrl).toHaveBeenCalledWith("https://example.com/releases");
  });

  it("normalizes url payloads before invoking the backend", async () => {
    vi.mocked(tauriOpenUrl).mockResolvedValue(undefined as any);

    await expect(
      openDesktopUrl({
        url: "  https://example.com/releases  ",
        with: "  browser-app  ",
      })
    ).resolves.toBe(true);

    expect(tauriInvoke).toHaveBeenCalledWith("desktop_opener_open_url", {
      input: {
        url: "https://example.com/releases",
        with: "browser-app",
      },
    });
    expect(tauriOpenUrl).toHaveBeenCalledWith("https://example.com/releases");
  });

  it("rejects invalid urls before invoking the backend", async () => {
    await expect(openDesktopUrl("   ")).rejects.toThrow("url is required");
    await expect(openDesktopUrl("javascript:alert(1)")).rejects.toThrow(
      "unsupported url scheme=javascript"
    );

    expect(tauriInvoke).not.toHaveBeenCalled();
    expect(tauriOpenUrl).not.toHaveBeenCalled();
  });

  it("opens desktop paths through the backend-owned desktop command", async () => {
    vi.mocked(tauriOpenPath).mockResolvedValue(undefined as any);

    await expect(openDesktopPath("/tmp/aio")).resolves.toBe(true);
    expect(tauriOpenPath).toHaveBeenCalledWith("/tmp/aio");
  });

  it("normalizes path payloads before invoking the backend", async () => {
    vi.mocked(tauriOpenPath).mockResolvedValue(undefined as any);

    await expect(
      openDesktopPath({
        path: "  /tmp/aio  ",
        with: "   ",
      })
    ).resolves.toBe(true);

    expect(tauriInvoke).toHaveBeenCalledWith("desktop_opener_open_path", {
      input: {
        path: "/tmp/aio",
        with: null,
      },
    });
    expect(tauriOpenPath).toHaveBeenCalledWith("/tmp/aio");
  });

  it("rejects invalid paths before invoking the backend", async () => {
    await expect(openDesktopPath("   ")).rejects.toThrow("path is required");
    await expect(revealDesktopItem({ path: "x".repeat(4097) })).rejects.toThrow("path is too long");

    expect(tauriInvoke).not.toHaveBeenCalled();
    expect(tauriOpenPath).not.toHaveBeenCalled();
    expect(tauriRevealItemInDir).not.toHaveBeenCalled();
  });

  it("reveals desktop items through the backend-owned desktop command", async () => {
    vi.mocked(tauriRevealItemInDir).mockResolvedValue(undefined as any);

    await expect(revealDesktopItem("/tmp/aio/file.txt")).resolves.toBe(true);
    expect(tauriRevealItemInDir).toHaveBeenCalledWith("/tmp/aio/file.txt");
  });

  it("normalizes reveal payloads before invoking the backend", async () => {
    vi.mocked(tauriRevealItemInDir).mockResolvedValue(undefined as any);

    await expect(revealDesktopItem({ path: "  /tmp/aio/file.txt  " })).resolves.toBe(true);

    expect(tauriInvoke).toHaveBeenCalledWith("desktop_opener_reveal_item_in_dir", {
      input: { path: "/tmp/aio/file.txt" },
    });
    expect(tauriRevealItemInDir).toHaveBeenCalledWith("/tmp/aio/file.txt");
  });
});
