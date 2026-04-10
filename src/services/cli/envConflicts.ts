import { invokeService } from "../invokeServiceCommand";
import type { CliKey } from "../providers/providers";

export type EnvConflict = {
  var_name: string;
  source_type: "system" | "file";
  source_path: string;
};

export async function envConflictsCheck(cliKey: CliKey): Promise<EnvConflict[] | null> {
  return invokeService<EnvConflict[]>("检查环境变量冲突失败", "env_conflicts_check", {
    cliKey,
  });
}
