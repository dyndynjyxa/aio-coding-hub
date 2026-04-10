import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { settingsGatewayRectifierSet } from "../settingsGatewayRectifier";
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

describe("services/settings/settingsGatewayRectifier", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("rectifier boom"));

    await expect(
      settingsGatewayRectifierSet({
        verbose_provider_error: true,
        intercept_anthropic_warmup_requests: false,
        enable_thinking_signature_rectifier: true,
        enable_thinking_budget_rectifier: true,
        enable_billing_header_rectifier: true,
        enable_claude_metadata_user_id_injection: true,
        enable_response_fixer: true,
        response_fixer_fix_encoding: true,
        response_fixer_fix_sse_format: true,
        response_fixer_fix_truncated_json: true,
        response_fixer_max_json_depth: 200,
        response_fixer_max_fix_size: 1024,
      })
    ).rejects.toThrow("rectifier boom");

    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "保存网关修复配置失败",
      expect.objectContaining({
        cmd: "settings_gateway_rectifier_set",
        error: expect.stringContaining("rectifier boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(
      settingsGatewayRectifierSet({
        verbose_provider_error: true,
        intercept_anthropic_warmup_requests: false,
        enable_thinking_signature_rectifier: true,
        enable_thinking_budget_rectifier: true,
        enable_billing_header_rectifier: true,
        enable_claude_metadata_user_id_injection: true,
        enable_response_fixer: true,
        response_fixer_fix_encoding: true,
        response_fixer_fix_sse_format: true,
        response_fixer_fix_truncated_json: true,
        response_fixer_max_json_depth: 200,
        response_fixer_max_fix_size: 1024,
      })
    ).rejects.toThrow("IPC_NULL_RESULT: settings_gateway_rectifier_set");
  });
});
