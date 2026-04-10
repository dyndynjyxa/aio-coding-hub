import { invokeService } from "../invokeServiceCommand";

export type CliVersionCheck = {
  cliKey: string;
  npmPackage: string;
  installedVersion: string | null;
  latestVersion: string | null;
  updateAvailable: boolean;
  error: string | null;
};

export type CliUpdateResult = {
  cliKey: string;
  success: boolean;
  output: string;
  error: string | null;
};

export async function cliCheckLatestVersion(cliKey: string) {
  return invokeService<CliVersionCheck>("检查版本失败", "cli_check_latest_version", { cliKey });
}

export async function cliUpdateCli(cliKey: string) {
  return invokeService<CliUpdateResult>("更新失败", "cli_update", { cliKey });
}
