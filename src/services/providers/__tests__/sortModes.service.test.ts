import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { sortModeCreate, sortModeDelete, sortModesList } from "../sortModes";
import { invokeTauriOrNull } from "../../tauriInvoke";

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

describe("services/sortModes (error semantics)", () => {
  it("rethrows and logs on invoke failure", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("sort modes boom"));

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
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(sortModesList()).rejects.toThrow("IPC_NULL_RESULT: sort_modes_list");
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue({ id: 1, name: "Mode" } as any);

    await sortModeCreate({ name: "Mode" });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("sort_mode_create", { name: "Mode" });

    await sortModeDelete({ mode_id: 2 });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("sort_mode_delete", { modeId: 2 });
  });
});
