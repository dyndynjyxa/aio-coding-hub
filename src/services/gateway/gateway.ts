import {
  commands,
  type GatewayActiveSessionSummary,
  type GatewayUpstreamProxyInput,
  type GatewayProviderCircuitStatus,
  type GatewayStatus,
} from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";
import type { CliKey } from "../providers/providers";

export type { GatewayProviderCircuitStatus, GatewayStatus };
export type GatewayActiveSession = GatewayActiveSessionSummary;

const CLI_KEY_VALUES = ["claude", "codex", "gemini"] as const satisfies readonly CliKey[];

export const GATEWAY_SESSIONS_DEFAULT_LIMIT = 50;
export const GATEWAY_SESSIONS_MIN_LIMIT = 1;
export const GATEWAY_SESSIONS_MAX_LIMIT = 200;
export const GATEWAY_PORT_MIN = 1024;
export const GATEWAY_PORT_MAX = 65_535;

export function normalizeGatewaySessionsLimit(limit?: number | null): number | null {
  if (limit == null) return null;
  if (!Number.isSafeInteger(limit)) {
    throw new Error(`SEC_INVALID_INPUT: invalid gateway sessions limit=${limit}`);
  }
  return Math.min(Math.max(limit, GATEWAY_SESSIONS_MIN_LIMIT), GATEWAY_SESSIONS_MAX_LIMIT);
}

function assertSafeInteger(label: string, value: number) {
  if (!Number.isSafeInteger(value)) {
    throw new Error(`SEC_INVALID_INPUT: invalid ${label}=${value}`);
  }
}

export function normalizeGatewayPreferredPort(preferredPort?: number | null): number | null {
  if (preferredPort == null) return null;
  assertSafeInteger("gateway preferredPort", preferredPort);
  if (preferredPort < GATEWAY_PORT_MIN || preferredPort > GATEWAY_PORT_MAX) {
    throw new Error(
      `SEC_INVALID_INPUT: gateway preferredPort must be between ${GATEWAY_PORT_MIN} and ${GATEWAY_PORT_MAX}`
    );
  }
  return preferredPort;
}

export function normalizeGatewayPortAvailabilityCheck(port: number): number | null {
  assertSafeInteger("gateway port", port);
  if (port < GATEWAY_PORT_MIN || port > GATEWAY_PORT_MAX) return null;
  return port;
}

export function normalizeGatewayProviderId(providerId: number): number {
  assertSafeInteger("providerId", providerId);
  if (providerId <= 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid providerId=${providerId}`);
  }
  return providerId;
}

export function validateGatewayCliKey(cliKey: string): CliKey {
  const normalizedCliKey = cliKey.trim();
  if ((CLI_KEY_VALUES as readonly string[]).includes(normalizedCliKey)) {
    return normalizedCliKey as CliKey;
  }
  throw new Error(`SEC_INVALID_INPUT: invalid cliKey=${cliKey}`);
}

export async function gatewayStatus() {
  return invokeGeneratedIpc<GatewayStatus>({
    title: "获取网关状态失败",
    cmd: "gateway_status",
    invoke: () => commands.gatewayStatus(),
  });
}

export async function gatewayStart(preferredPort?: number | null) {
  const normalizedPreferredPort = normalizeGatewayPreferredPort(preferredPort);

  return invokeGeneratedIpc<GatewayStatus>({
    title: "启动网关失败",
    cmd: "gateway_start",
    args: { preferredPort: normalizedPreferredPort },
    invoke: () =>
      commands.gatewayStart(normalizedPreferredPort) as Promise<
        GeneratedCommandResult<GatewayStatus>
      >,
  });
}

export async function gatewayStop() {
  return invokeGeneratedIpc<GatewayStatus>({
    title: "停止网关失败",
    cmd: "gateway_stop",
    invoke: () => commands.gatewayStop() as Promise<GeneratedCommandResult<GatewayStatus>>,
  });
}

export async function gatewayCheckPortAvailable(port: number) {
  const normalizedPort = normalizeGatewayPortAvailabilityCheck(port);
  if (normalizedPort == null) return false;

  return invokeGeneratedIpc<boolean>({
    title: "检查端口可用性失败",
    cmd: "gateway_check_port_available",
    args: { port: normalizedPort },
    invoke: () =>
      commands.gatewayCheckPortAvailable(normalizedPort) as Promise<
        GeneratedCommandResult<boolean>
      >,
  });
}

export async function gatewaySessionsList(limit?: number | null) {
  const normalizedLimit = normalizeGatewaySessionsLimit(limit);

  return invokeGeneratedIpc<GatewayActiveSession[]>({
    title: "获取会话列表失败",
    cmd: "gateway_sessions_list",
    args: { limit: normalizedLimit },
    invoke: () =>
      commands.gatewaySessionsList(normalizedLimit) as Promise<
        GeneratedCommandResult<GatewayActiveSession[]>
      >,
  });
}

export async function gatewayCircuitStatus(cliKey: string) {
  const normalizedCliKey = validateGatewayCliKey(cliKey);

  return invokeGeneratedIpc<GatewayProviderCircuitStatus[]>({
    title: "获取熔断器状态失败",
    cmd: "gateway_circuit_status",
    args: { cliKey: normalizedCliKey },
    invoke: () =>
      commands.gatewayCircuitStatus(normalizedCliKey) as Promise<
        GeneratedCommandResult<GatewayProviderCircuitStatus[]>
      >,
  });
}

export async function gatewayCircuitResetProvider(providerId: number) {
  const normalizedProviderId = normalizeGatewayProviderId(providerId);

  return invokeGeneratedIpc<boolean>({
    title: "重置 Provider 熔断器失败",
    cmd: "gateway_circuit_reset_provider",
    args: { providerId: normalizedProviderId },
    invoke: () =>
      commands.gatewayCircuitResetProvider(normalizedProviderId) as Promise<
        GeneratedCommandResult<boolean>
      >,
  });
}

export async function gatewayCircuitResetCli(cliKey: string) {
  const normalizedCliKey = validateGatewayCliKey(cliKey);

  return invokeGeneratedIpc<number>({
    title: "重置 CLI 熔断器失败",
    cmd: "gateway_circuit_reset_cli",
    args: { cliKey: normalizedCliKey },
    invoke: () =>
      commands.gatewayCircuitResetCli(normalizedCliKey) as Promise<GeneratedCommandResult<number>>,
  });
}

type GatewayUpstreamProxyAuthInput = {
  proxyUrl: string;
  proxyUsername?: string;
  proxyPassword?: string;
};

function buildGatewayUpstreamProxyInput({
  proxyUrl,
  proxyUsername,
  proxyPassword,
}: GatewayUpstreamProxyAuthInput): GatewayUpstreamProxyInput {
  return {
    proxyUrl,
    proxyUsername: proxyUsername ?? null,
    proxyPassword: proxyPassword ?? null,
  };
}

export async function gatewayUpstreamProxyValidate({
  proxyUrl,
  proxyUsername,
  proxyPassword,
}: GatewayUpstreamProxyAuthInput) {
  const input = buildGatewayUpstreamProxyInput({ proxyUrl, proxyUsername, proxyPassword });
  return invokeGeneratedIpc<null, null>({
    title: "代理配置验证失败",
    cmd: "gateway_upstream_proxy_validate",
    args: { input },
    invoke: () =>
      commands.gatewayUpstreamProxyValidate(input) as Promise<GeneratedCommandResult<null>>,
    nullResultBehavior: "return_fallback",
    fallback: null,
  });
}

export async function gatewayUpstreamProxyTest({
  proxyUrl,
  proxyUsername,
  proxyPassword,
}: GatewayUpstreamProxyAuthInput) {
  const input = buildGatewayUpstreamProxyInput({ proxyUrl, proxyUsername, proxyPassword });
  return invokeGeneratedIpc<null, null>({
    title: "代理连接测试失败",
    cmd: "gateway_upstream_proxy_test",
    args: { input },
    invoke: () => commands.gatewayUpstreamProxyTest(input) as Promise<GeneratedCommandResult<null>>,
    nullResultBehavior: "return_fallback",
    fallback: null,
  });
}

export async function gatewayUpstreamProxyDetectIp({
  proxyUrl,
  proxyUsername,
  proxyPassword,
}: GatewayUpstreamProxyAuthInput) {
  const input = buildGatewayUpstreamProxyInput({ proxyUrl, proxyUsername, proxyPassword });
  return invokeGeneratedIpc<string>({
    title: "代理出口 IP 检测失败",
    cmd: "gateway_upstream_proxy_detect_ip",
    args: { input },
    invoke: () =>
      commands.gatewayUpstreamProxyDetectIp(input) as Promise<GeneratedCommandResult<string>>,
  });
}
