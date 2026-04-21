import {
  commands,
  type DesktopNotificationPermissionState,
} from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type DesktopNotificationPermission = DesktopNotificationPermissionState;

export async function desktopNotificationIsPermissionGranted(): Promise<boolean> {
  const result = await invokeGeneratedIpc<boolean, boolean>({
    title: "检查系统通知权限失败",
    cmd: "desktop_notification_is_permission_granted",
    invoke: () =>
      commands.desktopNotificationIsPermissionGranted() as Promise<
        GeneratedCommandResult<boolean>
      >,
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
    invoke: () =>
      commands.desktopNotificationRequestPermission() as Promise<
        GeneratedCommandResult<DesktopNotificationPermission>
      >,
    nullResultBehavior: "return_fallback",
    fallback: "denied",
  });
  return result ?? "denied";
}

export async function desktopNotificationNotify(options: {
  title: string;
  body: string;
  sound?: string;
}): Promise<void> {
  const payload = {
    title: options.title,
    body: options.body,
    sound: options.sound ?? null,
  };

  await invokeGeneratedIpc<boolean>({
    title: "发送系统通知失败",
    cmd: "desktop_notification_notify",
    args: { options: payload },
    invoke: () =>
      commands.desktopNotificationNotify(payload) as Promise<GeneratedCommandResult<boolean>>,
  });
}
