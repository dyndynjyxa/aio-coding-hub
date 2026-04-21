import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import { settingsGatewayRectifierSet } from "../settingsGatewayRectifier";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      settingsGatewayRectifierSet: vi.fn(),
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

describe("services/settings/settingsGatewayRectifier", () => {
  const input = {
    verbose_provider_error: true,
    intercept_anthropic_warmup_requests: false,
    enable_thinking_signature_rectifier: true,
    enable_thinking_budget_rectifier: false,
    enable_billing_header_rectifier: true,
    enable_claude_metadata_user_id_injection: true,
    enable_response_fixer: true,
    response_fixer_fix_encoding: true,
    response_fixer_fix_sse_format: false,
    response_fixer_fix_truncated_json: true,
    response_fixer_max_json_depth: 8,
    response_fixer_max_fix_size: 4096,
  };

  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.settingsGatewayRectifierSet).mockRejectedValueOnce(
      new Error("rectifier boom")
    );

    await expect(settingsGatewayRectifierSet(input)).rejects.toThrow("rectifier boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "保存网关修复配置失败",
      expect.objectContaining({
        cmd: "settings_gateway_rectifier_set",
        error: expect.stringContaining("rectifier boom"),
      })
    );
  });

  it("maps generated args and treats null as runtime error", async () => {
    vi.mocked(commands.settingsGatewayRectifierSet).mockResolvedValueOnce(null as any);

    await expect(settingsGatewayRectifierSet(input)).rejects.toThrow(
      "IPC_NULL_RESULT: settings_gateway_rectifier_set"
    );

    vi.mocked(commands.settingsGatewayRectifierSet).mockResolvedValueOnce({
      status: "ok",
      data: { schema_version: 1 } as any,
    });

    await settingsGatewayRectifierSet(input);
    expect(commands.settingsGatewayRectifierSet).toHaveBeenCalledWith({
      verboseProviderError: true,
      interceptAnthropicWarmupRequests: false,
      enableThinkingSignatureRectifier: true,
      enableThinkingBudgetRectifier: false,
      enableBillingHeaderRectifier: true,
      enableClaudeMetadataUserIdInjection: true,
      enableResponseFixer: true,
      responseFixerFixEncoding: true,
      responseFixerFixSseFormat: false,
      responseFixerFixTruncatedJson: true,
      responseFixerMaxJsonDepth: 8,
      responseFixerMaxFixSize: 4096,
    });
  });
});
