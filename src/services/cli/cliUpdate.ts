import { commands, type CliUpdateResult, type CliVersionCheck } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type { CliVersionCheck, CliUpdateResult } from "../../generated/bindings";

export async function cliCheckLatestVersion(cliKey: string) {
  return invokeGeneratedIpc<CliVersionCheck>({
    title: "检查版本失败",
    cmd: "cli_check_latest_version",
    args: { cliKey },
    invoke: () =>
      commands.cliCheckLatestVersion(cliKey) as Promise<GeneratedCommandResult<CliVersionCheck>>,
  });
}

export async function cliUpdateCli(cliKey: string) {
  return invokeGeneratedIpc<CliUpdateResult>({
    title: "更新失败",
    cmd: "cli_update",
    args: { cliKey },
    invoke: () => commands.cliUpdate(cliKey) as Promise<GeneratedCommandResult<CliUpdateResult>>,
  });
}
