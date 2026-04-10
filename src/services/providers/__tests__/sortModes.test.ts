import { describe, expect, it, vi } from "vitest";
import { invokeTauriOrNull } from "../../tauriInvoke";
import {
  sortModeActiveList,
  sortModeActiveSet,
  sortModeCreate,
  sortModeDelete,
  sortModeProvidersList,
  sortModeProviderSetEnabled,
  sortModeProvidersSetOrder,
  sortModeRename,
  sortModesList,
} from "../sortModes";

vi.mock("../../tauriInvoke", async () => {
  const actual = await vi.importActual<typeof import("../../tauriInvoke")>("../../tauriInvoke");
  return {
    ...actual,
    invokeTauriOrNull: vi.fn(),
  };
});

describe("services/providers/sortModes", () => {
  it("invokes sort mode commands with expected parameters", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue({} as any);

    await sortModesList();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("sort_modes_list");

    await sortModeCreate({ name: "M1" });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("sort_mode_create", { name: "M1" });

    await sortModeRename({ mode_id: 1, name: "M2" });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("sort_mode_rename", { modeId: 1, name: "M2" });

    await sortModeDelete({ mode_id: 2 });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("sort_mode_delete", { modeId: 2 });

    await sortModeActiveList();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("sort_mode_active_list");

    await sortModeActiveSet({ cli_key: "claude" as any, mode_id: null });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("sort_mode_active_set", {
      cliKey: "claude",
      modeId: null,
    });

    await sortModeProvidersList({ mode_id: 3, cli_key: "codex" as any });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("sort_mode_providers_list", {
      modeId: 3,
      cliKey: "codex",
    });

    await sortModeProvidersSetOrder({
      mode_id: 4,
      cli_key: "gemini" as any,
      ordered_provider_ids: [9, 8, 7],
    });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("sort_mode_providers_set_order", {
      modeId: 4,
      cliKey: "gemini",
      orderedProviderIds: [9, 8, 7],
    });

    await sortModeProviderSetEnabled({
      mode_id: 5,
      cli_key: "claude" as any,
      provider_id: 9,
      enabled: false,
    });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("sort_mode_provider_set_enabled", {
      modeId: 5,
      cliKey: "claude",
      providerId: 9,
      enabled: false,
    });
  });
});
