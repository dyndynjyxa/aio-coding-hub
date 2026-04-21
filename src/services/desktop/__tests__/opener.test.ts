import { describe, expect, it, vi } from "vitest";
import { tauriOpenPath, tauriOpenUrl, tauriRevealItemInDir } from "../../../test/mocks/tauri";
import { openDesktopPath, openDesktopUrl, revealDesktopItem } from "../opener";

describe("services/desktop/opener", () => {
  it("opens desktop urls through the backend-owned desktop command", async () => {
    vi.mocked(tauriOpenUrl).mockResolvedValue(undefined as any);

    await expect(openDesktopUrl("https://example.com/releases")).resolves.toBe(true);
    expect(tauriOpenUrl).toHaveBeenCalledWith("https://example.com/releases");
  });

  it("opens desktop paths through the backend-owned desktop command", async () => {
    vi.mocked(tauriOpenPath).mockResolvedValue(undefined as any);

    await expect(openDesktopPath("/tmp/aio")).resolves.toBe(true);
    expect(tauriOpenPath).toHaveBeenCalledWith("/tmp/aio");
  });

  it("reveals desktop items through the backend-owned desktop command", async () => {
    vi.mocked(tauriRevealItemInDir).mockResolvedValue(undefined as any);

    await expect(revealDesktopItem("/tmp/aio/file.txt")).resolves.toBe(true);
    expect(tauriRevealItemInDir).toHaveBeenCalledWith("/tmp/aio/file.txt");
  });
});
