import { commands, type AppAboutInfo } from "../../generated/bindings";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";

export type { AppAboutInfo };

const APP_RUN_MODE_VALUES = ["installer", "portable", "unknown"] as const;
const APP_ABOUT_MAX_TEXT_CHARS = 256;
const APP_ABOUT_MAX_VERSION_CHARS = 128;

function normalizeRequiredAboutText(
  value: string,
  label: string,
  maxChars = APP_ABOUT_MAX_TEXT_CHARS
) {
  const normalized = value.trim();
  if (!normalized) {
    throw new Error(`IPC_INVALID_RESULT: ${label} is required`);
  }
  if ([...normalized].length > maxChars) {
    throw new Error(`IPC_INVALID_RESULT: ${label} is too long (max ${maxChars} chars)`);
  }
  return normalized;
}

function normalizeOptionalAboutText(value: string | null | undefined, label: string) {
  if (value == null) return null;
  return normalizeRequiredAboutText(value, label);
}

function normalizeRunMode(value: string) {
  const normalized = normalizeRequiredAboutText(value, "app_about.run_mode");
  if ((APP_RUN_MODE_VALUES as readonly string[]).includes(normalized)) {
    return normalized;
  }
  throw new Error(`IPC_INVALID_RESULT: invalid app_about.run_mode=${value}`);
}

function toAppAboutInfo(value: AppAboutInfo): AppAboutInfo {
  return {
    os: normalizeRequiredAboutText(value.os, "app_about.os"),
    arch: normalizeRequiredAboutText(value.arch, "app_about.arch"),
    profile: normalizeRequiredAboutText(value.profile, "app_about.profile"),
    app_version: normalizeRequiredAboutText(
      value.app_version,
      "app_about.app_version",
      APP_ABOUT_MAX_VERSION_CHARS
    ),
    bundle_type: normalizeOptionalAboutText(value.bundle_type, "app_about.bundle_type"),
    run_mode: normalizeRunMode(value.run_mode),
  };
}

export async function appAboutGet() {
  return invokeGeneratedIpc<AppAboutInfo>({
    title: "读取应用信息失败",
    cmd: "app_about_get",
    invoke: async () => mapGeneratedCommandResponse(await commands.appAboutGet(), toAppAboutInfo),
  });
}
