import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import { sortModeCreate, sortModeDelete, sortModesList } from "../sortModes";

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
      sortModeDelete: vi.fn(),
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

describe("services/sortModes (error semantics)", () => {
  it("rethrows and logs on invoke failure", async () => {
    vi.mocked(commands.sortModesList).mockRejectedValueOnce(new Error("sort modes boom"));

    await expect(sortModesList()).rejects.toThrow("sort modes boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取排序模板失败",
      expect.objectContaining({
        cmd: "sort_modes_list",
        error: expect.stringContaining("sort modes boom"),
      })
    );
  });

  it("treats null result as IPC null error with runtime", async () => {
    vi.mocked(commands.sortModesList).mockResolvedValueOnce(null as any);

    await expect(sortModesList()).rejects.toThrow("IPC_NULL_RESULT: sort_modes_list");
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(commands.sortModeCreate).mockResolvedValue({
      status: "ok",
      data: { id: 1, name: "Mode" } as any,
    });
    vi.mocked(commands.sortModeDelete).mockResolvedValue({
      status: "ok",
      data: true,
    });

    await sortModeCreate({ name: "Mode" });
    expect(commands.sortModeCreate).toHaveBeenCalledWith("Mode");

    await sortModeDelete({ mode_id: 2 });
    expect(commands.sortModeDelete).toHaveBeenCalledWith(2);
  });
});
