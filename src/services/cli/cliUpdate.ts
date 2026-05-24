import { commands, type CliUpdateResult, type CliVersionCheck } from "../../generated/bindings";
import { isCliKey } from "../../constants/clis";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";
import type { CliKey } from "../providers/providers";

export type { CliVersionCheck, CliUpdateResult } from "../../generated/bindings";

const CLI_UPDATE_SHORT_TEXT_MAX_CHARS = 256;
const CLI_UPDATE_ERROR_MAX_CHARS = 4096;
const CLI_UPDATE_OUTPUT_MAX_CHARS = 65_536;

function truncateChars(value: string, maxChars: number): string {
  const chars = [...value];
  if (chars.length <= maxChars) return value;
  return chars.slice(0, maxChars).join("");
}

export function normalizeCliUpdateKey(cliKey: string): CliKey {
  const normalized = cliKey.trim().toLowerCase();
  if (isCliKey(normalized)) return normalized;
  throw new Error(`SEC_INVALID_INPUT: invalid cliKey=${cliKey}`);
}

function normalizeResultBoolean(value: unknown, label: string): boolean {
  if (typeof value === "boolean") return value;
  throw new Error(`IPC_INVALID_RESULT: ${label} must be a boolean`);
}

function normalizeRequiredResultText(value: unknown, label: string, maxChars: number): string {
  if (typeof value !== "string") {
    throw new Error(`IPC_INVALID_RESULT: ${label} must be a string`);
  }
  const normalized = value.trim();
  if (!normalized) {
    throw new Error(`IPC_INVALID_RESULT: ${label} is required`);
  }
  return truncateChars(normalized, maxChars);
}

function normalizeOptionalResultText(
  value: unknown,
  label: string,
  maxChars: number
): string | null {
  if (value == null) return null;
  if (typeof value !== "string") {
    throw new Error(`IPC_INVALID_RESULT: ${label} must be a string`);
  }
  const normalized = value.trim();
  if (!normalized) return null;
  return truncateChars(normalized, maxChars);
}

function toCliVersionCheck(value: CliVersionCheck): CliVersionCheck {
  return {
    cliKey: normalizeCliUpdateKey(value.cliKey),
    npmPackage: normalizeRequiredResultText(
      value.npmPackage,
      "cli_version_check.npmPackage",
      CLI_UPDATE_SHORT_TEXT_MAX_CHARS
    ),
    installedVersion: normalizeOptionalResultText(
      value.installedVersion,
      "cli_version_check.installedVersion",
      CLI_UPDATE_SHORT_TEXT_MAX_CHARS
    ),
    latestVersion: normalizeOptionalResultText(
      value.latestVersion,
      "cli_version_check.latestVersion",
      CLI_UPDATE_SHORT_TEXT_MAX_CHARS
    ),
    updateAvailable: normalizeResultBoolean(
      value.updateAvailable,
      "cli_version_check.updateAvailable"
    ),
    error: normalizeOptionalResultText(
      value.error,
      "cli_version_check.error",
      CLI_UPDATE_ERROR_MAX_CHARS
    ),
  };
}

function toCliUpdateResult(value: CliUpdateResult): CliUpdateResult {
  return {
    cliKey: normalizeCliUpdateKey(value.cliKey),
    success: normalizeResultBoolean(value.success, "cli_update.success"),
    output:
      normalizeOptionalResultText(value.output, "cli_update.output", CLI_UPDATE_OUTPUT_MAX_CHARS) ??
      "",
    error: normalizeOptionalResultText(value.error, "cli_update.error", CLI_UPDATE_ERROR_MAX_CHARS),
  };
}

export async function cliCheckLatestVersion(cliKey: string) {
  const normalizedCliKey = normalizeCliUpdateKey(cliKey);

  return invokeGeneratedIpc<CliVersionCheck>({
    title: "检查版本失败",
    cmd: "cli_check_latest_version",
    args: { cliKey: normalizedCliKey },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.cliCheckLatestVersion(normalizedCliKey),
        toCliVersionCheck
      ),
  });
}

export async function cliUpdateCli(cliKey: string) {
  const normalizedCliKey = normalizeCliUpdateKey(cliKey);

  return invokeGeneratedIpc<CliUpdateResult>({
    title: "更新失败",
    cmd: "cli_update",
    args: { cliKey: normalizedCliKey },
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.cliUpdate(normalizedCliKey), toCliUpdateResult),
  });
}
