import {
  commands,
  type GatewayActiveSessionSummary,
  type GatewayUpstreamProxyInput,
  type GatewayProviderCircuitStatus,
  type GatewayStatus,
} from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type { GatewayProviderCircuitStatus, GatewayStatus };
export type GatewayActiveSession = GatewayActiveSessionSummary;

export async function gatewayStatus() {
  return invokeGeneratedIpc<GatewayStatus>({
    title: "获取网关状态失败",
    cmd: "gateway_status",
    invoke: () => commands.gatewayStatus(),
  });
}

export async function gatewayStart(preferredPort?: number) {
  return invokeGeneratedIpc<GatewayStatus>({
    title: "启动网关失败",
    cmd: "gateway_start",
    args: { preferredPort: preferredPort ?? null },
    invoke: () =>
      commands.gatewayStart(preferredPort ?? null) as Promise<GeneratedCommandResult<GatewayStatus>>,
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
  return invokeGeneratedIpc<boolean>({
    title: "检查端口可用性失败",
    cmd: "gateway_check_port_available",
    args: { port },
    invoke: () =>
      commands.gatewayCheckPortAvailable(port) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function gatewaySessionsList(limit?: number) {
  return invokeGeneratedIpc<GatewayActiveSession[]>({
    title: "获取会话列表失败",
    cmd: "gateway_sessions_list",
    args: { limit: limit ?? null },
    invoke: () =>
      commands.gatewaySessionsList(limit ?? null) as Promise<
        GeneratedCommandResult<GatewayActiveSession[]>
      >,
  });
}

export async function gatewayCircuitStatus(cliKey: string) {
  return invokeGeneratedIpc<GatewayProviderCircuitStatus[]>({
    title: "获取熔断器状态失败",
    cmd: "gateway_circuit_status",
    args: { cliKey },
    invoke: () =>
      commands.gatewayCircuitStatus(cliKey) as Promise<
        GeneratedCommandResult<GatewayProviderCircuitStatus[]>
      >,
  });
}

export async function gatewayCircuitResetProvider(providerId: number) {
  return invokeGeneratedIpc<boolean>({
    title: "重置 Provider 熔断器失败",
    cmd: "gateway_circuit_reset_provider",
    args: { providerId },
    invoke: () =>
      commands.gatewayCircuitResetProvider(providerId) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function gatewayCircuitResetCli(cliKey: string) {
  return invokeGeneratedIpc<number>({
    title: "重置 CLI 熔断器失败",
    cmd: "gateway_circuit_reset_cli",
    args: { cliKey },
    invoke: () =>
      commands.gatewayCircuitResetCli(cliKey) as Promise<GeneratedCommandResult<number>>,
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
    invoke: () =>
      commands.gatewayUpstreamProxyTest(input) as Promise<GeneratedCommandResult<null>>,
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
