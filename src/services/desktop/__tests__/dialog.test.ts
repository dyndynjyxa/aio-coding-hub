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

  it("normalizes open dialog options before invoking the backend", async () => {
    vi.mocked(tauriDialogOpen).mockResolvedValue(["/tmp/import.json"]);

    await expect(
      openDesktopDialog({
        title: "  Import config  ",
        defaultPath: "  /tmp/import.json  ",
        directory: false,
        multiple: true,
        recursive: true,
        canCreateDirectories: true,
        pickerMode: "document",
        fileAccessMode: "scoped",
        filters: [{ name: "  JSON files  ", extensions: [" .json ", " .jsonl ", "   "] }],
      })
    ).resolves.toEqual(["/tmp/import.json"]);

    expect(tauriDialogOpen).toHaveBeenCalledWith({
      title: "Import config",
      defaultPath: "/tmp/import.json",
      directory: false,
      multiple: true,
      recursive: true,
      canCreateDirectories: true,
      pickerMode: "document",
      fileAccessMode: "scoped",
      filters: [{ name: "JSON files", extensions: ["json", "jsonl"] }],
    });
  });

  it("rejects invalid open dialog options before invoking the backend", async () => {
    await expect(
      openDesktopDialog({ filters: [{ name: "   ", extensions: ["json"] }] })
    ).rejects.toThrow("filter name is required");
    await expect(
      openDesktopDialog({ filters: [{ name: "JSON", extensions: ["x".repeat(65)] }] })
    ).rejects.toThrow("filter extension is too long");
    await expect(openDesktopDialog({ pickerMode: "archive" as any })).rejects.toThrow(
      "invalid pickerMode=archive"
    );
    await expect(openDesktopDialog({ multiple: "yes" as any })).rejects.toThrow(
      "multiple must be a boolean"
    );

    expect(tauriDialogOpen).not.toHaveBeenCalled();
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

  it("normalizes save dialog options before invoking the backend", async () => {
    vi.mocked(tauriDialogSave).mockResolvedValue("/tmp/export.json");

    await expect(
      saveDesktopDialog({
        title: "  Export config  ",
        defaultPath: "   ",
        canCreateDirectories: false,
        filters: [{ name: "  JSON  ", extensions: [".json"] }],
      })
    ).resolves.toBe("/tmp/export.json");

    expect(tauriDialogSave).toHaveBeenCalledWith({
      title: "Export config",
      defaultPath: null,
      filters: [{ name: "JSON", extensions: ["json"] }],
      canCreateDirectories: false,
    });
  });

  it("rejects invalid save dialog options before invoking the backend", async () => {
    await expect(saveDesktopDialog({ title: "x".repeat(257) })).rejects.toThrow(
      "title is too long"
    );
    await expect(
      saveDesktopDialog({ filters: [{ name: "JSON", extensions: ["   "] }] })
    ).rejects.toThrow("filter extensions are required");

    expect(tauriDialogSave).not.toHaveBeenCalled();
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
