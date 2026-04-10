import { invokeServiceWithDetails } from "../invokeServiceCommand";

export type GatewayStatus = {
  running: boolean;
  port: number | null;
  base_url: string | null;
  listen_addr: string | null;
};

export type GatewayActiveSession = {
  cli_key: string;
  session_id: string;
  session_suffix: string;
  provider_id: number;
  provider_name: string;
  expires_at: number;
  request_count: number | null;
  total_input_tokens: number | null;
  total_output_tokens: number | null;
  total_cost_usd: number | null;
  total_duration_ms: number | null;
};

export type GatewayProviderCircuitStatus = {
  provider_id: number;
  state: string;
  failure_count: number;
  failure_threshold: number;
  open_until: number | null;
  cooldown_until: number | null;
};

export async function gatewayStatus() {
  return invokeServiceWithDetails<GatewayStatus>("获取网关状态失败", "gateway_status");
}

export async function gatewayStart(preferredPort?: number) {
  return invokeServiceWithDetails<GatewayStatus>(
    "启动网关失败",
    "gateway_start",
    {
      preferredPort: preferredPort ?? null,
    },
    { preferredPort }
  );
}

export async function gatewayStop() {
  return invokeServiceWithDetails<GatewayStatus>("停止网关失败", "gateway_stop");
}

export async function gatewayCheckPortAvailable(port: number) {
  return invokeServiceWithDetails<boolean>(
    "检查端口可用性失败",
    "gateway_check_port_available",
    { port },
    { port }
  );
}

export async function gatewaySessionsList(limit?: number) {
  return invokeServiceWithDetails<GatewayActiveSession[]>(
    "获取会话列表失败",
    "gateway_sessions_list",
    { limit: limit ?? null },
    { limit }
  );
}

export async function gatewayCircuitStatus(cliKey: string) {
  return invokeServiceWithDetails<GatewayProviderCircuitStatus[]>(
    "获取熔断器状态失败",
    "gateway_circuit_status",
    { cliKey },
    { cliKey }
  );
}

export async function gatewayCircuitResetProvider(providerId: number) {
  return invokeServiceWithDetails<boolean>(
    "重置 Provider 熔断器失败",
    "gateway_circuit_reset_provider",
    { providerId },
    { providerId }
  );
}

export async function gatewayCircuitResetCli(cliKey: string) {
  return invokeServiceWithDetails<number>(
    "重置 CLI 熔断器失败",
    "gateway_circuit_reset_cli",
    { cliKey },
    { cliKey }
  );
}
