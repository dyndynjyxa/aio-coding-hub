import { invokeTauriOrNull } from "./tauriInvoke";
import type { WslTargetCli } from "./settings";

export type WslDetection = {
  detected: boolean;
  distros: string[];
};

export type WslDistroConfigStatus = {
  distro: string;
  claude: boolean;
  codex: boolean;
  gemini: boolean;
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
  return invokeTauriOrNull<WslDetection>("wsl_detect");
}

export async function wslHostAddressGet() {
  return invokeTauriOrNull<string | null>("wsl_host_address_get");
}

export async function wslConfigStatusGet(distros?: string[]) {
  return invokeTauriOrNull<WslDistroConfigStatus[]>(
    "wsl_config_status_get",
    distros !== undefined ? { distros } : undefined
  );
}

export async function wslConfigureClients(input: { targets: WslTargetCli }) {
  return invokeTauriOrNull<WslConfigureReport>("wsl_configure_clients", {
    targets: input.targets,
  });
}
