import { commands, type EnvConflict as GeneratedEnvConflict } from "../../generated/bindings";
import { isCliKey } from "../../constants/clis";
import type { CliKey } from "../providers/providers";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";

export type EnvConflict = {
  var_name: string;
  source_type: "system" | "file";
  source_path: string;
};

const ENV_CONFLICT_SOURCE_TYPES = ["system", "file"] as const;
const ENV_CONFLICT_VAR_NAME_MAX_CHARS = 128;
const ENV_CONFLICT_SOURCE_PATH_MAX_CHARS = 4096;

function charLength(value: string) {
  return [...value].length;
}

export function normalizeEnvConflictCliKey(cliKey: string): CliKey {
  const normalized = cliKey.trim().toLowerCase();
  if (isCliKey(normalized)) return normalized;
  throw new Error(`SEC_INVALID_INPUT: invalid cliKey=${cliKey}`);
}

function normalizeRequiredEnvConflictText(value: unknown, label: string, maxChars: number) {
  if (typeof value !== "string") {
    throw new Error(`IPC_INVALID_RESULT: ${label} must be a string`);
  }
  const normalized = value.trim();
  if (!normalized) {
    throw new Error(`IPC_INVALID_RESULT: ${label} is required`);
  }
  if (charLength(normalized) > maxChars) {
    throw new Error(`IPC_INVALID_RESULT: ${label} is too long (max ${maxChars} chars)`);
  }
  return normalized;
}

function normalizeEnvConflictSourceType(value: unknown): EnvConflict["source_type"] {
  if (
    typeof value === "string" &&
    (ENV_CONFLICT_SOURCE_TYPES as readonly string[]).includes(value)
  ) {
    return value as EnvConflict["source_type"];
  }
  throw new Error(`IPC_INVALID_RESULT: invalid env conflict source_type=${String(value)}`);
}

function toEnvConflict(value: GeneratedEnvConflict): EnvConflict {
  return {
    var_name: normalizeRequiredEnvConflictText(
      value.var_name,
      "env_conflict.var_name",
      ENV_CONFLICT_VAR_NAME_MAX_CHARS
    ),
    source_type: normalizeEnvConflictSourceType(value.source_type),
    source_path: normalizeRequiredEnvConflictText(
      value.source_path,
      "env_conflict.source_path",
      ENV_CONFLICT_SOURCE_PATH_MAX_CHARS
    ),
  };
}

function toEnvConflicts(value: GeneratedEnvConflict[]): EnvConflict[] {
  if (!Array.isArray(value)) {
    throw new Error("IPC_INVALID_RESULT: env conflicts must be an array");
  }
  return value.map(toEnvConflict);
}

export async function envConflictsCheck(cliKey: CliKey): Promise<EnvConflict[] | null> {
  const normalizedCliKey = normalizeEnvConflictCliKey(cliKey);

  return invokeGeneratedIpc<EnvConflict[] | null, null>({
    title: "检查环境变量冲突失败",
    cmd: "env_conflicts_check",
    args: { cliKey: normalizedCliKey },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.envConflictsCheck(normalizedCliKey),
        toEnvConflicts
      ),
    nullResultBehavior: "return_fallback",
    fallback: null,
  });
}
