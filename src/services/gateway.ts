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

import { invokeTauriOrNull } from "./tauriInvoke";

export async function gatewayStatus() {
  try {
    return await invokeTauriOrNull<GatewayStatus>("gateway_status");
  } catch {
    return null;
  }
}

export async function gatewayStart(preferredPort?: number) {
  try {
    return await invokeTauriOrNull<GatewayStatus>("gateway_start", {
      preferredPort: preferredPort ?? null,
    });
  } catch {
    return null;
  }
}

export async function gatewayStop() {
  try {
    return await invokeTauriOrNull<GatewayStatus>("gateway_stop");
  } catch {
    return null;
  }
}

export async function gatewayCheckPortAvailable(port: number) {
  try {
    return await invokeTauriOrNull<boolean>("gateway_check_port_available", {
      port,
    });
  } catch {
    return null;
  }
}

export async function gatewaySessionsList(limit?: number) {
  try {
    return await invokeTauriOrNull<GatewayActiveSession[]>("gateway_sessions_list", {
      limit: limit ?? null,
    });
  } catch {
    return null;
  }
}

export async function gatewayCircuitStatus(cliKey: string) {
  try {
    return await invokeTauriOrNull<GatewayProviderCircuitStatus[]>("gateway_circuit_status", {
      cliKey,
    });
  } catch {
    return null;
  }
}

export async function gatewayCircuitResetProvider(providerId: number) {
  try {
    return await invokeTauriOrNull<boolean>("gateway_circuit_reset_provider", {
      providerId,
    });
  } catch {
    return null;
  }
}

export async function gatewayCircuitResetCli(cliKey: string) {
  try {
    return await invokeTauriOrNull<number>("gateway_circuit_reset_cli", {
      cliKey,
    });
  } catch {
    return null;
  }
}
