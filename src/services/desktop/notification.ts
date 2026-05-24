import {
  commands,
  type DesktopNotificationPayload,
  type DesktopNotificationPermissionState,
} from "../../generated/bindings";
import {
  invokeGeneratedIpc,
  mapGeneratedCommandResponse,
  type GeneratedCommandResult,
} from "../generatedIpc";

export type DesktopNotificationPermission = DesktopNotificationPermissionState;
export type DesktopNotificationNotifyOptions = {
  title: string;
  body: string;
  sound?: string | null;
};

const DESKTOP_NOTIFICATION_PERMISSION_VALUES = [
  "granted",
  "denied",
  "prompt",
  "prompt-with-rationale",
] as const satisfies readonly DesktopNotificationPermission[];
const DESKTOP_NOTIFICATION_TITLE_MAX_CHARS = 256;
const DESKTOP_NOTIFICATION_BODY_MAX_CHARS = 4096;
const DESKTOP_NOTIFICATION_SOUND_MAX_CHARS = 128;

function charLength(value: string) {
  return [...value].length;
}

function normalizeRequiredNotificationText(value: unknown, label: string, maxChars: number) {
  if (typeof value !== "string") {
    throw new Error(`SEC_INVALID_INPUT: ${label} must be a string`);
  }
  const normalized = value.trim();
  if (!normalized) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is required`);
  }
  if (charLength(normalized) > maxChars) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is too long (max ${maxChars} chars)`);
  }
  return normalized;
}

function normalizeOptionalNotificationSound(value: unknown) {
  if (value == null) return null;
  if (typeof value !== "string") {
    throw new Error("SEC_INVALID_INPUT: sound must be a string");
  }
  const normalized = value.trim();
  if (!normalized) return null;
  if (charLength(normalized) > DESKTOP_NOTIFICATION_SOUND_MAX_CHARS) {
    throw new Error(
      `SEC_INVALID_INPUT: sound is too long (max ${DESKTOP_NOTIFICATION_SOUND_MAX_CHARS} chars)`
    );
  }
  return normalized;
}

function normalizeDesktopNotificationPermission(value: unknown): DesktopNotificationPermission {
  if (
    typeof value === "string" &&
    (DESKTOP_NOTIFICATION_PERMISSION_VALUES as readonly string[]).includes(value)
  ) {
    return value as DesktopNotificationPermission;
  }
  throw new Error(`IPC_INVALID_RESULT: invalid desktop notification permission=${String(value)}`);
}

function normalizeDesktopNotificationPayload(
  options: DesktopNotificationNotifyOptions
): DesktopNotificationPayload {
  return {
    title: normalizeRequiredNotificationText(
      options.title,
      "title",
      DESKTOP_NOTIFICATION_TITLE_MAX_CHARS
    ),
    body: normalizeRequiredNotificationText(
      options.body,
      "body",
      DESKTOP_NOTIFICATION_BODY_MAX_CHARS
    ),
    sound: normalizeOptionalNotificationSound(options.sound),
  };
}

export async function desktopNotificationIsPermissionGranted(): Promise<boolean> {
  const result = await invokeGeneratedIpc<boolean, boolean>({
    title: "检查系统通知权限失败",
    cmd: "desktop_notification_is_permission_granted",
    invoke: () =>
      commands.desktopNotificationIsPermissionGranted() as Promise<GeneratedCommandResult<boolean>>,
    nullResultBehavior: "return_fallback",
    fallback: false,
  });
  return result === true;
}

export async function desktopNotificationRequestPermission(): Promise<DesktopNotificationPermission> {
  const result = await invokeGeneratedIpc<
    DesktopNotificationPermission,
    DesktopNotificationPermission
  >({
    title: "请求系统通知权限失败",
    cmd: "desktop_notification_request_permission",
    invoke: async () => {
      const response = await commands.desktopNotificationRequestPermission();
      return mapGeneratedCommandResponse(response, normalizeDesktopNotificationPermission);
    },
    nullResultBehavior: "return_fallback",
    fallback: "denied",
  });
  return result;
}

export async function desktopNotificationNotify(
  options: DesktopNotificationNotifyOptions
): Promise<void> {
  const payload = normalizeDesktopNotificationPayload(options);

  await invokeGeneratedIpc<boolean>({
    title: "发送系统通知失败",
    cmd: "desktop_notification_notify",
    args: { options: payload },
    invoke: () =>
      commands.desktopNotificationNotify(payload) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function desktopNotificationPlaySound(): Promise<void> {
  await invokeGeneratedIpc<boolean>({
    title: "播放通知音效失败",
    cmd: "desktop_notification_play_sound",
    invoke: () =>
      commands.desktopNotificationPlaySound() as Promise<GeneratedCommandResult<boolean>>,
  });
}
