import { describe, expect, it, vi } from "vitest";
import { tauriInvoke } from "../../../test/mocks/tauri";
import { setDesktopWindowTheme } from "../window";

describe("services/desktop/window", () => {
  it("normalizes desktop theme values before invoking the backend", async () => {
    vi.mocked(tauriInvoke).mockResolvedValue(true as any);

    await expect(setDesktopWindowTheme(" dark " as any)).resolves.toBe(true);

    expect(tauriInvoke).toHaveBeenCalledWith("desktop_window_set_theme", {
      theme: "dark",
    });
  });

  it("rejects invalid desktop themes before invoking the backend", async () => {
    await expect(setDesktopWindowTheme("sepia" as any)).rejects.toThrow(
      "invalid desktop theme=sepia"
    );

    expect(tauriInvoke).not.toHaveBeenCalled();
  });
});
