import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { configExport, configImport } from "../../services/app/configMigrate";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import {
  cliProxyKeys,
  gatewayKeys,
  mcpKeys,
  promptsKeys,
  providersKeys,
  settingsKeys,
  skillsKeys,
  sortModesKeys,
  workspacesKeys,
  wslKeys,
} from "../keys";
import { useConfigExportMutation, useConfigImportMutation } from "../configMigrate";

vi.mock("../../services/app/configMigrate", async () => {
  const actual = await vi.importActual<typeof import("../../services/app/configMigrate")>(
    "../../services/app/configMigrate"
  );
  return {
    ...actual,
    configExport: vi.fn(),
    configImport: vi.fn(),
  };
});

describe("query/configMigrate", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("useConfigExportMutation delegates file path to configExport", async () => {
    vi.mocked(configExport).mockResolvedValue(true);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(() => useConfigExportMutation(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync({ filePath: "/tmp/export.json" });
    });

    expect(configExport).toHaveBeenCalledWith("/tmp/export.json");
  });

  it("useConfigImportMutation invalidates imported config queries after success", async () => {
    vi.mocked(configImport).mockResolvedValue({
      providers_imported: 1,
      sort_modes_imported: 1,
      workspaces_imported: 1,
      prompts_imported: 1,
      mcp_servers_imported: 1,
      skill_repos_imported: 1,
      installed_skills_imported: 1,
      local_skills_imported: 1,
    });

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(() => useConfigImportMutation(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync({ filePath: "/tmp/import.json" });
    });

    expect(configImport).toHaveBeenCalledWith("/tmp/import.json");
    expect(invalidateSpy).toHaveBeenCalledTimes(10);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: settingsKeys.all });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: gatewayKeys.all });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: providersKeys.all });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: sortModesKeys.all });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: workspacesKeys.all });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: promptsKeys.all });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: mcpKeys.all });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: skillsKeys.all });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: wslKeys.all });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: cliProxyKeys.all });
  });

  it("useConfigImportMutation skips invalidation when import returns null", async () => {
    vi.mocked(configImport).mockResolvedValue(null as never);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(() => useConfigImportMutation(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync({ filePath: "/tmp/import.json" });
    });

    expect(configImport).toHaveBeenCalledWith("/tmp/import.json");
    expect(invalidateSpy).not.toHaveBeenCalled();
  });
});
