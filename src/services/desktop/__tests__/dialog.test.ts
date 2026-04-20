import { describe, expect, it, vi } from "vitest";
import { tauriDialogOpen, tauriDialogSave } from "../../../test/mocks/tauri";
import {
  openDesktopDialog,
  openDesktopSinglePath,
  pickDesktopSinglePath,
  saveDesktopDialog,
  saveDesktopFilePath,
} from "../dialog";

describe("services/desktop/dialog", () => {
  it("openDesktopDialog delegates to tauri dialog open", async () => {
    vi.mocked(tauriDialogOpen).mockResolvedValue("/tmp/import.json");

    await expect(
      openDesktopDialog({
        directory: false,
        multiple: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      })
    ).resolves.toBe("/tmp/import.json");

    expect(tauriDialogOpen).toHaveBeenCalledWith({
      title: null,
      defaultPath: null,
      directory: false,
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
      recursive: null,
      canCreateDirectories: null,
      pickerMode: null,
      fileAccessMode: null,
    });
  });

  it("saveDesktopDialog delegates to tauri dialog save", async () => {
    vi.mocked(tauriDialogSave).mockResolvedValue("/tmp/export.json");

    await expect(
      saveDesktopDialog({
        defaultPath: "/tmp/export.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      })
    ).resolves.toBe("/tmp/export.json");

    expect(tauriDialogSave).toHaveBeenCalledWith({
      title: null,
      defaultPath: "/tmp/export.json",
      filters: [{ name: "JSON", extensions: ["json"] }],
      canCreateDirectories: null,
    });
  });

  it("pickDesktopSinglePath normalizes string arrays and null", () => {
    expect(pickDesktopSinglePath("/tmp/a.json")).toBe("/tmp/a.json");
    expect(pickDesktopSinglePath(["/tmp/a.json", "/tmp/b.json"])).toBe("/tmp/a.json");
    expect(pickDesktopSinglePath([])).toBeNull();
    expect(pickDesktopSinglePath(null)).toBeNull();
  });

  it("openDesktopSinglePath returns a normalized path", async () => {
    vi.mocked(tauriDialogOpen).mockResolvedValue(["/tmp/import.json"]);

    await expect(
      openDesktopSinglePath({
        directory: false,
        multiple: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      })
    ).resolves.toBe("/tmp/import.json");

    expect(tauriDialogOpen).toHaveBeenCalledWith({
      title: null,
      defaultPath: null,
      directory: false,
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
      recursive: null,
      canCreateDirectories: null,
      pickerMode: null,
      fileAccessMode: null,
    });
  });

  it("saveDesktopFilePath returns a normalized path", async () => {
    vi.mocked(tauriDialogSave).mockResolvedValue("/tmp/export.json");

    await expect(
      saveDesktopFilePath({
        defaultPath: "/tmp/export.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      })
    ).resolves.toBe("/tmp/export.json");

    expect(tauriDialogSave).toHaveBeenCalledWith({
      title: null,
      defaultPath: "/tmp/export.json",
      filters: [{ name: "JSON", extensions: ["json"] }],
      canCreateDirectories: null,
    });
  });
});
