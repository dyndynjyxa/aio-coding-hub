import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import { promptDelete, promptSetEnabled, promptUpsert, promptsList } from "../prompts";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      promptsList: vi.fn(),
      promptUpsert: vi.fn(),
      promptSetEnabled: vi.fn(),
      promptDelete: vi.fn(),
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

describe("services/workspace/prompts", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.promptsList).mockRejectedValueOnce(new Error("prompts boom"));

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
    vi.mocked(commands.promptsList).mockResolvedValueOnce(null as any);

    await expect(promptsList(1)).rejects.toThrow("IPC_NULL_RESULT: prompts_list");
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(commands.promptUpsert).mockResolvedValue({ status: "ok", data: { id: 1 } as any });
    vi.mocked(commands.promptSetEnabled).mockResolvedValue({
      status: "ok",
      data: { id: 1 } as any,
    });
    vi.mocked(commands.promptDelete).mockResolvedValue({ status: "ok", data: true });

    await promptUpsert({
      prompt_id: null,
      workspace_id: 1,
      name: "P1",
      content: "hello",
      enabled: true,
    });
    expect(commands.promptUpsert).toHaveBeenCalledWith(null, 1, "P1", "hello", true);

    await promptSetEnabled(10, true);
    expect(commands.promptSetEnabled).toHaveBeenCalledWith(10, true);

    await promptDelete(10);
    expect(commands.promptDelete).toHaveBeenCalledWith(10);
  });
});
