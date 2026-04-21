import { commands } from "../../generated/bindings";
import type { CliKey } from "../providers/providers";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type CliProxyStatus = {
  cli_key: CliKey;
  enabled: boolean;
  base_origin: string | null;
  current_gateway_origin?: string | null;
  applied_to_current_gateway: boolean | null;
};

export type CliProxyResult = {
  trace_id: string;
  cli_key: CliKey;
  enabled: boolean;
  ok: boolean;
  error_code: string | null;
  message: string;
  base_origin: string | null;
};

export async function cliProxyStatusAll() {
  return invokeGeneratedIpc<CliProxyStatus[]>({
    title: "读取 CLI 代理状态失败",
    cmd: "cli_proxy_status_all",
    invoke: () =>
      commands.cliProxyStatusAll() as Promise<GeneratedCommandResult<CliProxyStatus[]>>,
  });
}

export async function cliProxySetEnabled(input: { cli_key: CliKey; enabled: boolean }) {
  return invokeGeneratedIpc<CliProxyResult>({
    title: "设置 CLI 代理开关失败",
    cmd: "cli_proxy_set_enabled",
    args: { cliKey: input.cli_key, enabled: input.enabled },
    invoke: () =>
      commands.cliProxySetEnabled(input.cli_key, input.enabled) as Promise<
        GeneratedCommandResult<CliProxyResult>
      >,
  });
}

export async function cliProxySyncEnabled(base_origin: string, options?: { apply_live?: boolean }) {
  return invokeGeneratedIpc<CliProxyResult[]>({
    title: "同步 CLI 代理状态失败",
    cmd: "cli_proxy_sync_enabled",
    args: {
      baseOrigin: base_origin,
      applyLive: options?.apply_live ?? null,
    },
    invoke: () =>
      commands.cliProxySyncEnabled(base_origin, options?.apply_live ?? null) as Promise<
        GeneratedCommandResult<CliProxyResult[]>
      >,
  });
}

export async function cliProxyRebindCodexHome() {
  return invokeGeneratedIpc<CliProxyResult>({
    title: "重绑 Codex 目录失败",
    cmd: "cli_proxy_rebind_codex_home",
    invoke: () =>
      commands.cliProxyRebindCodexHome() as Promise<GeneratedCommandResult<CliProxyResult>>,
  });
}
