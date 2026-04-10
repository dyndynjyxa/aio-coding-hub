import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import {
  gatewayCheckPortAvailable,
  gatewayStart,
  gatewayStop,
  gatewayCircuitStatus,
  gatewaySessionsList,
  gatewayStatus,
  type GatewayActiveSession,
  type GatewayProviderCircuitStatus,
  type GatewayStatus,
} from "../gateway";
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

describe("services/gateway/gateway", () => {
  it("returns invoke result with tauri runtime", async () => {
    const status: GatewayStatus = {
      running: true,
      port: 37123,
      base_url: "http://127.0.0.1:37123",
      listen_addr: "127.0.0.1:37123",
    };
    const sessions: GatewayActiveSession[] = [
      {
        cli_key: "claude",
        session_id: "session-1",
        session_suffix: "1",
        provider_id: 1,
        provider_name: "Provider-1",
        expires_at: 1,
        request_count: 2,
        total_input_tokens: 3,
        total_output_tokens: 4,
        total_cost_usd: 0.01,
        total_duration_ms: 20,
      },
    ];
    const circuits: GatewayProviderCircuitStatus[] = [
      {
        provider_id: 1,
        state: "OPEN",
        failure_count: 3,
        failure_threshold: 5,
        open_until: 100,
        cooldown_until: null,
      },
    ];

    vi.mocked(invokeTauriOrNull)
      .mockResolvedValueOnce(status)
      .mockResolvedValueOnce(sessions)
      .mockResolvedValueOnce(circuits);

    await expect(gatewayStatus()).resolves.toEqual(status);
    await expect(gatewaySessionsList(20)).resolves.toEqual(sessions);
    await expect(gatewayCircuitStatus("claude")).resolves.toEqual(circuits);
  });

  it("passes gateway command args with stable contract fields", async () => {
    vi.mocked(invokeTauriOrNull)
      .mockResolvedValueOnce({ running: true } as any)
      .mockResolvedValueOnce({ running: false } as any)
      .mockResolvedValueOnce(true)
      .mockResolvedValueOnce([] as any)
      .mockResolvedValueOnce([] as any)
      .mockResolvedValueOnce(1 as any)
      .mockResolvedValueOnce(true as any);

    await gatewayStart(37123);
    await gatewayStop();
    await gatewayCheckPortAvailable(37123);
    await gatewaySessionsList(undefined);
    await gatewayCircuitStatus("claude");
    await gatewayCircuitStatus("codex");
    await gatewayCircuitStatus("gemini");

    expect(invokeTauriOrNull).toHaveBeenCalledWith("gateway_start", {
      preferredPort: 37123,
    });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("gateway_stop");
    expect(invokeTauriOrNull).toHaveBeenCalledWith("gateway_check_port_available", {
      port: 37123,
    });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("gateway_sessions_list", {
      limit: null,
    });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("gateway_circuit_status", {
      cliKey: "claude",
    });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("gateway_circuit_status", {
      cliKey: "codex",
    });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("gateway_circuit_status", {
      cliKey: "gemini",
    });
  });

  it("rethrows invoke errors and logs details", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("boom"));

    await expect(gatewayStatus()).rejects.toThrow("boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "获取网关状态失败",
      expect.objectContaining({
        cmd: "gateway_status",
        error: expect.stringContaining("boom"),
      })
    );
  });

  it("treats null invoke result as error and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(gatewayStatus()).rejects.toThrow("IPC_NULL_RESULT: gateway_status");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "获取网关状态失败",
      expect.objectContaining({
        cmd: "gateway_status",
        error: expect.stringContaining("IPC_NULL_RESULT: gateway_status"),
      })
    );
  });
});
