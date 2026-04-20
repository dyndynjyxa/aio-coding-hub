import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import { claudeProviderValidateModel } from "../claudeModelValidation";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      claudeProviderValidateModel: vi.fn(),
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

describe("services/claude/claudeModelValidation", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.claudeProviderValidateModel).mockRejectedValueOnce(
      new Error("claude validation boom")
    );

    await expect(
      claudeProviderValidateModel({ provider_id: 1, base_url: "https://x", request_json: "{}" })
    ).rejects.toThrow("claude validation boom");

    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "Claude 模型验证失败",
      expect.objectContaining({
        cmd: "claude_provider_validate_model",
        error: expect.stringContaining("claude validation boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(commands.claudeProviderValidateModel).mockResolvedValueOnce(null as any);

    await expect(
      claudeProviderValidateModel({ provider_id: 1, base_url: "https://x", request_json: "{}" })
    ).rejects.toThrow("IPC_NULL_RESULT: claude_provider_validate_model");
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(commands.claudeProviderValidateModel).mockResolvedValue({
      status: "ok",
      data: { ok: true } as any,
    });

    await claudeProviderValidateModel({
      provider_id: 9,
      base_url: "https://api",
      request_json: "{}",
    });
    expect(commands.claudeProviderValidateModel).toHaveBeenCalledWith(9, "https://api", "{}");
  });
});
