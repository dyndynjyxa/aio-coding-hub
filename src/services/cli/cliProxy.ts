import {
  commands,
  type CliProxyResult as GeneratedCliProxyResult,
  type CliProxyStatus as GeneratedCliProxyStatus,
} from "../../generated/bindings";
import type { CliKey } from "../providers/providers";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";
import { narrowGeneratedStringUnion, type Override } from "../generatedTypeUtils";

const CLI_KEY_VALUES = ["claude", "codex", "gemini"] as const satisfies readonly CliKey[];

export type CliProxyStatus = Override<
  GeneratedCliProxyStatus,
  {
    cli_key: CliKey;
    current_gateway_origin?: string | null;
  }
>;

export type CliProxyResult = Override<
  GeneratedCliProxyResult,
  {
    cli_key: CliKey;
  }
>;

export function validateCliProxyCliKey(cliKey: string): CliKey {
  const normalizedCliKey = cliKey.trim();
  if ((CLI_KEY_VALUES as readonly string[]).includes(normalizedCliKey)) {
    return normalizedCliKey as CliKey;
  }
  throw new Error(`SEC_INVALID_INPUT: invalid cliKey=${cliKey}`);
}

export function normalizeCliProxyBaseOrigin(baseOrigin: string): string {
  const normalized = baseOrigin.trim();
  if (!normalized) {
    throw new Error("SEC_INVALID_INPUT: baseOrigin is required");
  }

  let parsed: URL;
  try {
    parsed = new URL(normalized);
  } catch {
    throw new Error("SEC_INVALID_INPUT: baseOrigin must be a valid URL origin");
  }

  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    throw new Error("SEC_INVALID_INPUT: baseOrigin must use http or https");
  }
  if (!parsed.hostname) {
    throw new Error("SEC_INVALID_INPUT: baseOrigin host is required");
  }
  if (parsed.username || parsed.password) {
    throw new Error("SEC_INVALID_INPUT: baseOrigin credentials are not allowed");
  }
  if (parsed.pathname !== "/" || parsed.search || parsed.hash) {
    throw new Error("SEC_INVALID_INPUT: baseOrigin must not include path, query, or hash");
  }

  return parsed.origin;
}

function toCliProxyStatus(value: GeneratedCliProxyStatus): CliProxyStatus {
  return {
    ...value,
    cli_key: narrowGeneratedStringUnion(value.cli_key, CLI_KEY_VALUES, "cli_proxy_status.cli_key"),
  };
}

function toCliProxyResult(value: GeneratedCliProxyResult): CliProxyResult {
  return {
    ...value,
    cli_key: narrowGeneratedStringUnion(value.cli_key, CLI_KEY_VALUES, "cli_proxy_result.cli_key"),
  };
}

export async function cliProxyStatusAll() {
  return invokeGeneratedIpc<CliProxyStatus[]>({
    title: "读取 CLI 代理状态失败",
    cmd: "cli_proxy_status_all",
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.cliProxyStatusAll(), (rows) =>
        rows.map(toCliProxyStatus)
      ),
  });
}

export async function cliProxySetEnabled(input: { cli_key: CliKey; enabled: boolean }) {
  const cliKey = validateCliProxyCliKey(input.cli_key);

  return invokeGeneratedIpc<CliProxyResult>({
    title: "设置 CLI 代理开关失败",
    cmd: "cli_proxy_set_enabled",
    args: { cliKey, enabled: input.enabled },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.cliProxySetEnabled(cliKey, input.enabled),
        toCliProxyResult
      ),
  });
}

export async function cliProxySyncEnabled(base_origin: string, options?: { apply_live?: boolean }) {
  const baseOrigin = normalizeCliProxyBaseOrigin(base_origin);
  const applyLive = options?.apply_live ?? null;

  return invokeGeneratedIpc<CliProxyResult[]>({
    title: "同步 CLI 代理状态失败",
    cmd: "cli_proxy_sync_enabled",
    args: {
      baseOrigin,
      applyLive,
    },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.cliProxySyncEnabled(baseOrigin, applyLive),
        (rows) => rows.map(toCliProxyResult)
      ),
  });
}

export async function cliProxyRebindCodexHome() {
  return invokeGeneratedIpc<CliProxyResult>({
    title: "重绑 Codex 目录失败",
    cmd: "cli_proxy_rebind_codex_home",
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.cliProxyRebindCodexHome(), toCliProxyResult),
  });
}
