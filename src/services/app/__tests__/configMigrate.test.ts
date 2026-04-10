import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { invokeTauriOrNull } from "../../tauriInvoke";
import { configExport, configImport } from "../configMigrate";

vi.mock("../../tauriInvoke", async () => {
  const actual = await vi.importActual<typeof import("../../tauriInvoke")>("../../tauriInvoke");
  return {
    ...actual,
    invokeTauriOrNull: vi.fn(),
  };
});

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return {
    ...actual,
    logToConsole: vi.fn(),
  };
});

describe("services/app/configMigrate", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("export boom"));

    await expect(configExport()).rejects.toThrow("export boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "导出配置失败",
      expect.objectContaining({
        cmd: "config_export",
        error: expect.stringContaining("export boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(configExport()).rejects.toThrow("IPC_NULL_RESULT: config_export");
  });

  it("invokes config migrate commands with expected parameters", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue({} as any);

    await configExport();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("config_export");

    const bundle = { schema_version: 1 };
    await configImport(bundle);
    expect(invokeTauriOrNull).toHaveBeenCalledWith("config_import", { bundle });
  });
});
