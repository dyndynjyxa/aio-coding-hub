import { invokeService, invokeServiceCommand } from "../invokeServiceCommand";

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
  return invokeService<WslDetection>("检测 WSL 失败", "wsl_detect");
}

export async function wslHostAddressGet() {
  return invokeServiceCommand<string | null, null>({
    title: "读取 WSL 主机地址失败",
    cmd: "wsl_host_address_get",
    fallback: null,
    nullResultBehavior: "return_fallback",
  });
}

export async function wslConfigStatusGet(distros?: string[]) {
  return invokeService<WslDistroConfigStatus[]>(
    "读取 WSL 配置状态失败",
    "wsl_config_status_get",
    distros !== undefined ? { distros } : undefined
  );
}

export async function wslConfigureClients() {
  return invokeService<WslConfigureReport>("配置 WSL 客户端失败", "wsl_configure_clients");
}
