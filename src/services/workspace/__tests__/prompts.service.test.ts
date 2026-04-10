import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { invokeTauriOrNull } from "../../tauriInvoke";
import { promptDelete, promptSetEnabled, promptUpsert, promptsList } from "../prompts";

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

describe("services/workspace/prompts", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("prompts boom"));

    await expect(promptsList(1)).rejects.toThrow("prompts boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取提示词列表失败",
      expect.objectContaining({
        cmd: "prompts_list",
        error: expect.stringContaining("prompts boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(promptsList(1)).rejects.toThrow("IPC_NULL_RESULT: prompts_list");
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue({ id: 1 } as any);

    await promptUpsert({
      prompt_id: null,
      workspace_id: 1,
      name: "P1",
      content: "hello",
      enabled: true,
    });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("prompt_upsert", {
      promptId: null,
      workspaceId: 1,
      name: "P1",
      content: "hello",
      enabled: true,
    });

    await promptSetEnabled(10, true);
    expect(invokeTauriOrNull).toHaveBeenCalledWith("prompt_set_enabled", {
      promptId: 10,
      enabled: true,
    });

    await promptDelete(10);
    expect(invokeTauriOrNull).toHaveBeenCalledWith("prompt_delete", { promptId: 10 });
  });
});
