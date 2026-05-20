import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import { configExport, configImport, normalizeConfigMigrateFilePath } from "../configMigrate";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      configExport: vi.fn(),
      configImport: vi.fn(),
    },
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
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.configExport).mockRejectedValueOnce(new Error("export boom"));

    await expect(configExport("/tmp/aio-export.json")).rejects.toThrow("export boom");
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
    vi.mocked(commands.configExport).mockResolvedValueOnce({ status: "ok", data: null as any });

    await expect(configExport("/tmp/aio-export.json")).rejects.toThrow(
      "IPC_NULL_RESULT: config_export"
    );
  });

  it("invokes config migrate commands with expected parameters", async () => {
    vi.mocked(commands.configExport).mockResolvedValueOnce({ status: "ok", data: true });
    vi.mocked(commands.configImport).mockResolvedValueOnce({
      status: "ok",
      data: {
        providers_imported: 1,
        sort_modes_imported: 0,
        workspaces_imported: 0,
        prompts_imported: 0,
        mcp_servers_imported: 0,
        skill_repos_imported: 0,
        installed_skills_imported: 0,
        local_skills_imported: 0,
      },
    });

    await configExport(" /tmp/aio-export.json ");
    expect(commands.configExport).toHaveBeenCalledWith("/tmp/aio-export.json");

    await configImport(" /tmp/aio-import.json ");
    expect(commands.configImport).toHaveBeenCalledWith(
      "/tmp/aio-import.json",
      expect.objectContaining({
        confirm: expect.objectContaining({
          action: "config_import",
          resource: "/tmp/aio-import.json",
          nonce: expect.any(String),
        }),
      })
    );
  });

  it("rejects blank file paths before generated commands", async () => {
    expect(normalizeConfigMigrateFilePath(" /tmp/aio.json ")).toBe("/tmp/aio.json");
    expect(() => normalizeConfigMigrateFilePath("   ")).toThrow("SEC_INVALID_INPUT");

    await expect(configExport("   ")).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(configImport("\t")).rejects.toThrow("SEC_INVALID_INPUT");

    expect(commands.configExport).not.toHaveBeenCalled();
    expect(commands.configImport).not.toHaveBeenCalled();
  });
});
