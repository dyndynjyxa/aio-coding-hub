import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
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

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      sortModesList: vi.fn(),
      sortModeCreate: vi.fn(),
      sortModeRename: vi.fn(),
      sortModeDelete: vi.fn(),
      sortModeActiveList: vi.fn(),
      sortModeActiveSet: vi.fn(),
      sortModeProvidersList: vi.fn(),
      sortModeProvidersSetOrder: vi.fn(),
      sortModeProviderSetEnabled: vi.fn(),
    },
  };
});

describe("services/providers/sortModes", () => {
  it("invokes sort mode commands with expected parameters", async () => {
    vi.mocked(commands.sortModesList).mockResolvedValue({ status: "ok", data: [] as any });
    vi.mocked(commands.sortModeCreate).mockResolvedValue({ status: "ok", data: {} as any });
    vi.mocked(commands.sortModeRename).mockResolvedValue({ status: "ok", data: {} as any });
    vi.mocked(commands.sortModeDelete).mockResolvedValue({ status: "ok", data: true });
    vi.mocked(commands.sortModeActiveList).mockResolvedValue({ status: "ok", data: [] as any });
    vi.mocked(commands.sortModeActiveSet).mockResolvedValue({ status: "ok", data: {} as any });
    vi.mocked(commands.sortModeProvidersList).mockResolvedValue({ status: "ok", data: [] as any });
    vi.mocked(commands.sortModeProvidersSetOrder).mockResolvedValue({
      status: "ok",
      data: [] as any,
    });
    vi.mocked(commands.sortModeProviderSetEnabled).mockResolvedValue({
      status: "ok",
      data: {} as any,
    });

    await sortModesList();
    expect(commands.sortModesList).toHaveBeenCalledWith();

    await sortModeCreate({ name: "M1" });
    expect(commands.sortModeCreate).toHaveBeenCalledWith("M1");

    await sortModeRename({ mode_id: 1, name: "M2" });
    expect(commands.sortModeRename).toHaveBeenCalledWith(1, "M2");

    await sortModeDelete({ mode_id: 2 });
    expect(commands.sortModeDelete).toHaveBeenCalledWith(2);

    await sortModeActiveList();
    expect(commands.sortModeActiveList).toHaveBeenCalledWith();

    await sortModeActiveSet({ cli_key: "claude" as any, mode_id: null });
    expect(commands.sortModeActiveSet).toHaveBeenCalledWith("claude", null);

    await sortModeProvidersList({ mode_id: 3, cli_key: "codex" as any });
    expect(commands.sortModeProvidersList).toHaveBeenCalledWith(3, "codex");

    await sortModeProvidersSetOrder({
      mode_id: 4,
      cli_key: "gemini" as any,
      ordered_provider_ids: [9, 8, 7],
    });
    expect(commands.sortModeProvidersSetOrder).toHaveBeenCalledWith(4, "gemini", [9, 8, 7]);

    await sortModeProviderSetEnabled({
      mode_id: 5,
      cli_key: "claude" as any,
      provider_id: 9,
      enabled: false,
    });
    expect(commands.sortModeProviderSetEnabled).toHaveBeenCalledWith(5, "claude", 9, false);
  });
});
