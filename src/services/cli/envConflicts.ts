import { commands } from "../../generated/bindings";
import type { CliKey } from "../providers/providers";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type EnvConflict = {
  var_name: string;
  source_type: "system" | "file";
  source_path: string;
};

export async function envConflictsCheck(cliKey: CliKey): Promise<EnvConflict[] | null> {
  return invokeGeneratedIpc<EnvConflict[] | null, null>({
    title: "检查环境变量冲突失败",
    cmd: "env_conflicts_check",
    args: { cliKey },
    invoke: () =>
      commands.envConflictsCheck(cliKey) as Promise<GeneratedCommandResult<EnvConflict[] | null>>,
    nullResultBehavior: "return_fallback",
    fallback: null,
  });
}
