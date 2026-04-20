import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

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
  return invokeGeneratedIpc<WslDistroConfigStatus[]>({
    title: "读取 WSL 配置状态失败",
    cmd: "wsl_config_status_get",
    args: distros !== undefined ? { distros } : undefined,
    invoke: () => commands.wslConfigStatusGet(distros ?? null),
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
