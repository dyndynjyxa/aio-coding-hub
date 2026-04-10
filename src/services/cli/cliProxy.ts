import { invokeService } from "../invokeServiceCommand";
import type { CliKey } from "../providers/providers";

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
  return invokeService<CliProxyStatus[]>("读取 CLI 代理状态失败", "cli_proxy_status_all");
}

export async function cliProxySetEnabled(input: { cli_key: CliKey; enabled: boolean }) {
  return invokeService<CliProxyResult>("设置 CLI 代理开关失败", "cli_proxy_set_enabled", {
    cliKey: input.cli_key,
    enabled: input.enabled,
  });
}

export async function cliProxySyncEnabled(base_origin: string, options?: { apply_live?: boolean }) {
  return invokeService<CliProxyResult[]>("同步 CLI 代理状态失败", "cli_proxy_sync_enabled", {
    baseOrigin: base_origin,
    applyLive: options?.apply_live,
  });
}

export async function cliProxyRebindCodexHome() {
  return invokeService<CliProxyResult>("重绑 Codex 目录失败", "cli_proxy_rebind_codex_home");
}
