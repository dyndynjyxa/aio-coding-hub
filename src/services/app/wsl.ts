import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export const WSL_DISTROS_MAX_COUNT = 64;
export const WSL_DISTRO_MAX_CHARS = 128;

export type WslDetection = {
  detected: boolean;
  distros: string[];
};

export type WslDistroConfigStatus = {
  distro: string;
  claude: boolean;
  codex: boolean;
  gemini: boolean;
  claude_mcp?: boolean;
  codex_mcp?: boolean;
  gemini_mcp?: boolean;
  claude_prompt?: boolean;
  codex_prompt?: boolean;
  gemini_prompt?: boolean;
};

export type WslConfigureCliReport = {
  cli_key: string;
  ok: boolean;
  message: string;
};

export type WslConfigureDistroReport = {
  distro: string;
  ok: boolean;
  results: WslConfigureCliReport[];
};

export type WslConfigureReport = {
  ok: boolean;
  message: string;
  distros: WslConfigureDistroReport[];
};

export function normalizeWslDistros(distros?: readonly string[] | null): string[] | null {
  if (distros == null) return null;
  if (distros.length > WSL_DISTROS_MAX_COUNT) {
    throw new Error(
      `SEC_INVALID_INPUT: WSL distro list must contain at most ${WSL_DISTROS_MAX_COUNT} entries`
    );
  }

  const seen = new Set<string>();
  const normalized: string[] = [];
  for (const raw of distros) {
    const distro = raw.trim();
    if (!distro || seen.has(distro)) continue;
    if (/[\u0000-\u001f\u007f]/u.test(distro)) {
      throw new Error("SEC_INVALID_INPUT: WSL distro name contains control characters");
    }
    if ([...distro].length > WSL_DISTRO_MAX_CHARS) {
      throw new Error(
        `SEC_INVALID_INPUT: WSL distro name is too long (max ${WSL_DISTRO_MAX_CHARS} chars)`
      );
    }
    seen.add(distro);
    normalized.push(distro);
  }

  return normalized;
}

export async function wslDetect() {
  return invokeGeneratedIpc<WslDetection>({
    title: "检测 WSL 失败",
    cmd: "wsl_detect",
    invoke: () => commands.wslDetect(),
  });
}

export async function wslHostAddressGet() {
  return invokeGeneratedIpc<string | null, null>({
    title: "读取 WSL 主机地址失败",
    cmd: "wsl_host_address_get",
    invoke: () => commands.wslHostAddressGet(),
    fallback: null,
    nullResultBehavior: "return_fallback",
  });
}

export async function wslConfigStatusGet(distros?: string[]) {
  const normalizedDistros = normalizeWslDistros(distros);
  if (normalizedDistros?.length === 0) return [];

  return invokeGeneratedIpc<WslDistroConfigStatus[]>({
    title: "读取 WSL 配置状态失败",
    cmd: "wsl_config_status_get",
    args: normalizedDistros !== null ? { distros: normalizedDistros } : undefined,
    invoke: () => commands.wslConfigStatusGet(normalizedDistros),
  });
}

export async function wslConfigureClients() {
  return invokeGeneratedIpc<WslConfigureReport>({
    title: "配置 WSL 客户端失败",
    cmd: "wsl_configure_clients",
    invoke: () =>
      commands.wslConfigureClients() as Promise<GeneratedCommandResult<WslConfigureReport>>,
  });
}
