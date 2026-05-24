import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import {
  MAX_SORT_MODE_NAME_CHARS,
  MAX_SORT_MODE_PROVIDER_IDS,
  type SortModeActiveRow,
  type SortModeProviderRow,
  type SortModeSummary,
  sortModeActiveList,
  sortModeActiveSet,
  sortModeCreate,
  sortModeDelete,
  sortModeProvidersList,
  sortModeProviderSetEnabled,
  sortModeProvidersSetOrder,
  sortModeRename,
  sortModesList,
  validateSortModeId,
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

function makeSortModeSummary(overrides: Partial<SortModeSummary> = {}): SortModeSummary {
  return {
    id: 1,
    name: "Work",
    created_at: 0,
    updated_at: 0,
    ...overrides,
  };
}

function makeSortModeActiveRow(overrides: Partial<SortModeActiveRow> = {}): SortModeActiveRow {
  return {
    cli_key: "claude",
    mode_id: 1,
    updated_at: 0,
    ...overrides,
  };
}

function makeSortModeProviderRow(
  overrides: Partial<SortModeProviderRow> = {}
): SortModeProviderRow {
  return {
    provider_id: 101,
    enabled: true,
    ...overrides,
  };
}

describe("services/providers/sortModes", () => {
  it("invokes sort mode commands with expected parameters", async () => {
    vi.mocked(commands.sortModesList).mockResolvedValue({ status: "ok", data: [] });
    vi.mocked(commands.sortModeCreate).mockResolvedValue({
      status: "ok",
      data: makeSortModeSummary({ id: 2, name: "M1" }),
    });
    vi.mocked(commands.sortModeRename).mockResolvedValue({
      status: "ok",
      data: makeSortModeSummary({ id: 1, name: "M2" }),
    });
    vi.mocked(commands.sortModeDelete).mockResolvedValue({ status: "ok", data: true });
    vi.mocked(commands.sortModeActiveList).mockResolvedValue({
      status: "ok",
      data: [makeSortModeActiveRow()],
    });
    vi.mocked(commands.sortModeActiveSet).mockResolvedValue({
      status: "ok",
      data: makeSortModeActiveRow({ mode_id: null }),
    });
    vi.mocked(commands.sortModeProvidersList).mockResolvedValue({
      status: "ok",
      data: [makeSortModeProviderRow()],
    });
    vi.mocked(commands.sortModeProvidersSetOrder).mockResolvedValue({
      status: "ok",
      data: [makeSortModeProviderRow({ provider_id: 9 })],
    });
    vi.mocked(commands.sortModeProviderSetEnabled).mockResolvedValue({
      status: "ok",
      data: makeSortModeProviderRow({ provider_id: 9, enabled: false }),
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

    await sortModeActiveSet({ cli_key: "claude", mode_id: null });
    expect(commands.sortModeActiveSet).toHaveBeenCalledWith("claude", null);

    await sortModeProvidersList({ mode_id: 3, cli_key: "codex" });
    expect(commands.sortModeProvidersList).toHaveBeenCalledWith(3, "codex");

    await sortModeProvidersSetOrder({
      mode_id: 4,
      cli_key: "gemini",
      ordered_provider_ids: [9, 8, 7],
    });
    expect(commands.sortModeProvidersSetOrder).toHaveBeenCalledWith(4, "gemini", [9, 8, 7]);

    await sortModeProviderSetEnabled({
      mode_id: 5,
      cli_key: "claude",
      provider_id: 9,
      enabled: false,
    });
    expect(commands.sortModeProviderSetEnabled).toHaveBeenCalledWith(5, "claude", 9, false);
  });

  it("normalizes and validates sort mode command inputs before IPC", async () => {
    vi.mocked(commands.sortModeCreate).mockClear();
    vi.mocked(commands.sortModeRename).mockClear();
    vi.mocked(commands.sortModeActiveSet).mockClear();
    vi.mocked(commands.sortModeProvidersList).mockClear();
    vi.mocked(commands.sortModeProvidersSetOrder).mockClear();
    vi.mocked(commands.sortModeProviderSetEnabled).mockClear();

    vi.mocked(commands.sortModeCreate).mockResolvedValue({
      status: "ok",
      data: makeSortModeSummary({ id: 10, name: "Trimmed" }),
    });
    vi.mocked(commands.sortModeActiveSet).mockResolvedValue({
      status: "ok",
      data: makeSortModeActiveRow({ cli_key: "claude", mode_id: 2 }),
    });
    vi.mocked(commands.sortModeProvidersList).mockResolvedValue({
      status: "ok",
      data: [makeSortModeProviderRow()],
    });
    vi.mocked(commands.sortModeProvidersSetOrder).mockResolvedValue({
      status: "ok",
      data: [makeSortModeProviderRow()],
    });
    vi.mocked(commands.sortModeProviderSetEnabled).mockResolvedValue({
      status: "ok",
      data: makeSortModeProviderRow({ enabled: false }),
    });

    await sortModeCreate({ name: "  Trimmed  " });
    expect(commands.sortModeCreate).toHaveBeenCalledWith("Trimmed");
    await sortModeActiveSet({ cli_key: " claude " as never, mode_id: 2 });
    await sortModeProvidersList({ mode_id: 3, cli_key: " codex " as never });
    await sortModeProvidersSetOrder({
      mode_id: 4,
      cli_key: " gemini " as never,
      ordered_provider_ids: [9, 8, 7],
    });
    await sortModeProviderSetEnabled({
      mode_id: 5,
      cli_key: " claude " as never,
      provider_id: 9,
      enabled: false,
    });

    expect(commands.sortModeActiveSet).toHaveBeenCalledWith("claude", 2);
    expect(commands.sortModeProvidersList).toHaveBeenCalledWith(3, "codex");
    expect(commands.sortModeProvidersSetOrder).toHaveBeenCalledWith(4, "gemini", [9, 8, 7]);
    expect(commands.sortModeProviderSetEnabled).toHaveBeenCalledWith(5, "claude", 9, false);

    vi.mocked(commands.sortModeRename).mockClear();
    vi.mocked(commands.sortModeProvidersSetOrder).mockClear();
    vi.mocked(commands.sortModeProviderSetEnabled).mockClear();

    await expect(sortModeCreate({ name: "" })).rejects.toThrow("mode name is required");
    await expect(sortModeCreate({ name: "default" })).rejects.toThrow("mode name is reserved");
    await expect(
      sortModeCreate({ name: "x".repeat(MAX_SORT_MODE_NAME_CHARS + 1) })
    ).rejects.toThrow("mode name is too long");

    await expect(sortModeRename({ mode_id: 0, name: "Next" })).rejects.toThrow("invalid modeId=0");
    expect(() => validateSortModeId(0)).toThrow("SEC_INVALID_INPUT");
    await expect(
      sortModeProvidersSetOrder({ mode_id: 1, cli_key: "claude", ordered_provider_ids: [1, 0] })
    ).rejects.toThrow("invalid providerId=0");
    await expect(
      sortModeProvidersSetOrder({
        mode_id: 1,
        cli_key: "opencode" as never,
        ordered_provider_ids: [1],
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      sortModeProvidersSetOrder({ mode_id: 1, cli_key: "claude", ordered_provider_ids: [1, 1] })
    ).rejects.toThrow("duplicate providerId=1");
    await expect(
      sortModeProvidersSetOrder({
        mode_id: 1,
        cli_key: "claude",
        ordered_provider_ids: Array.from(
          { length: MAX_SORT_MODE_PROVIDER_IDS + 1 },
          (_, index) => index + 1
        ),
      })
    ).rejects.toThrow("orderedProviderIds must contain at most");
    await expect(
      sortModeProviderSetEnabled({ mode_id: 1, cli_key: "claude", provider_id: -1, enabled: true })
    ).rejects.toThrow("invalid providerId=-1");

    expect(commands.sortModeRename).not.toHaveBeenCalled();
    expect(commands.sortModeProvidersSetOrder).not.toHaveBeenCalled();
    expect(commands.sortModeProviderSetEnabled).not.toHaveBeenCalled();
  });
});
