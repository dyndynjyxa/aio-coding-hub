import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  promptDelete,
  promptSetEnabled,
  promptUpsert,
  promptsList,
  type PromptSummary,
  validatePromptId,
  validatePromptWorkspaceId,
} from "../prompts";

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
  beforeEach(() => {
    vi.clearAllMocks();
  });

  function createPromptSummary(overrides: Partial<PromptSummary> = {}): PromptSummary {
    return {
      id: 1,
      workspace_id: 1,
      cli_key: "claude",
      name: "Prompt A",
      content: "hello",
      enabled: true,
      created_at: 0,
      updated_at: 0,
      ...overrides,
    };
  }

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
    vi.mocked(commands.promptsList).mockResolvedValueOnce(null as never);

    await expect(promptsList(1)).rejects.toThrow("IPC_NULL_RESULT: prompts_list");
  });

  it("invokes generated commands with normalized args", async () => {
    vi.mocked(commands.promptUpsert).mockResolvedValue({
      status: "ok",
      data: createPromptSummary(),
    });
    vi.mocked(commands.promptSetEnabled).mockResolvedValue({
      status: "ok",
      data: createPromptSummary({ enabled: false }),
    });
    vi.mocked(commands.promptDelete).mockResolvedValue({ status: "ok", data: true });

    await promptUpsert({
      promptId: null,
      workspaceId: 1,
      name: " P1 ",
      content: " hello ",
      enabled: true,
    });
    expect(commands.promptUpsert).toHaveBeenCalledWith(null, 1, "P1", "hello", true);

    await promptSetEnabled(10, true);
    expect(commands.promptSetEnabled).toHaveBeenCalledWith(10, true);

    await promptDelete(10);
    expect(commands.promptDelete).toHaveBeenCalledWith(10);
  });

  it("rejects invalid ids and blank names before generated commands", async () => {
    expect(validatePromptWorkspaceId(1)).toBe(1);
    expect(validatePromptId(2)).toBe(2);
    expect(() => validatePromptWorkspaceId(0)).toThrow("SEC_INVALID_INPUT");
    expect(() => validatePromptId(Number.NaN)).toThrow("SEC_INVALID_INPUT");

    await expect(promptsList(0)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      promptUpsert({
        promptId: 0,
        workspaceId: 1,
        name: "P1",
        content: "hello",
        enabled: true,
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      promptUpsert({
        promptId: null,
        workspaceId: Number.NaN,
        name: "P1",
        content: "hello",
        enabled: true,
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      promptUpsert({
        promptId: null,
        workspaceId: 1,
        name: "   ",
        content: "hello",
        enabled: true,
      })
    ).rejects.toThrow("PROMPT_NAME_REQUIRED");
    await expect(promptSetEnabled(-1, true)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(promptDelete(Number.NaN)).rejects.toThrow("SEC_INVALID_INPUT");

    expect(commands.promptsList).not.toHaveBeenCalled();
    expect(commands.promptUpsert).not.toHaveBeenCalled();
    expect(commands.promptSetEnabled).not.toHaveBeenCalled();
    expect(commands.promptDelete).not.toHaveBeenCalled();
  });
});
