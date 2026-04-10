/**
 * Notice（系统通知）模块 - 全局事件监听
 *
 * 用法：
 * - 在 `src/App.tsx` 启动时调用 `listenNoticeEvents()`（只需要注册一次）
 * - 权限请求由 Settings 页面负责；此监听器仅在已授权时发送通知
 *
 * 注意：直接使用 Tauri IPC 调用原生通知，绕过 `@tauri-apps/plugin-notification`
 * JS 包装层中基于 `window.Notification`（Web API）的权限检查和通知发送——
 * WebView 的 Web Notification 权限不会跨会话持久化，导致每次重启都需要重新授权。
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { appEventNames } from "../../constants/appEvents";
import { logToConsole } from "../consoleLog";
import type { NoticeLevel } from "./notice";
import { getNotificationSoundEnabled, playNotificationSound } from "./notificationSound";

export type NoticeEventPayload = {
  level: NoticeLevel;
  title: string;
  body: string;
};

async function isPermissionGrantedNative(): Promise<boolean> {
  const result = await invoke<boolean | null>("plugin:notification|is_permission_granted");
  return result === true;
}

async function sendNotificationNative(options: {
  title: string;
  body: string;
  sound?: string;
}): Promise<void> {
  await invoke("plugin:notification|notify", {
    options: {
      title: options.title,
      body: options.body,
      ...(options.sound ? { sound: options.sound } : {}),
    },
  });
}

export async function listenNoticeEvents(): Promise<() => void> {
  const unlisten = await listen<NoticeEventPayload>(appEventNames.notice, async (event) => {
    const payload = event.payload;
    if (!payload) return;

    try {
      const permissionGranted = await isPermissionGrantedNative();
      if (!permissionGranted) return;

      if (getNotificationSoundEnabled()) {
        // Custom sound enabled: play ding.mp3 and send silent notification (no sound param)
        playNotificationSound();
        await sendNotificationNative({ title: payload.title, body: payload.body });
      } else {
        // Custom sound disabled: normal system notification with default sound
        await sendNotificationNative({
          title: payload.title,
          body: payload.body,
          sound: "default",
        });
      }
    } catch (err) {
      logToConsole("error", "发送系统通知失败", {
        error: String(err),
        level: payload.level,
        title: payload.title,
      });
    }
  });

  return () => {
    unlisten();
  };
}
