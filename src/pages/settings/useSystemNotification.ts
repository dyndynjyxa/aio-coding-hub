import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { logToConsole } from "../../services/consoleLog";
import { noticeSend } from "../../services/notification/notice";

export type NoticePermissionStatus = "checking" | "granted" | "not_granted" | "denied" | "unknown";

/**
 * Check notification permission via Tauri IPC (native), bypassing the Web API
 * wrapper in `@tauri-apps/plugin-notification` which uses `window.Notification`
 * — a permission state that does NOT persist across WKWebView sessions.
 *
 * The Rust desktop plugin always returns `Some(true)` (Granted).
 */
async function isPermissionGrantedNative(): Promise<boolean> {
  const result = await invoke<boolean | null>("plugin:notification|is_permission_granted");
  return result === true;
}

/**
 * Request notification permission via Tauri IPC (native).
 * On desktop, this always returns "granted".
 */
async function requestPermissionNative(): Promise<string> {
  return await invoke<string>("plugin:notification|request_permission");
}

export function useSystemNotification() {
  const [noticePermissionStatus, setNoticePermissionStatus] =
    useState<NoticePermissionStatus>("checking");
  const [requestingNoticePermission, setRequestingNoticePermission] = useState(false);
  const [sendingNoticeTest, setSendingNoticeTest] = useState(false);

  useEffect(() => {
    let cancelled = false;
    isPermissionGrantedNative()
      .then((granted) => {
        if (cancelled) return;
        setNoticePermissionStatus(granted ? "granted" : "not_granted");
      })
      .catch((err) => {
        if (cancelled) return;
        logToConsole("error", "检查系统通知权限失败", { error: String(err) });
        setNoticePermissionStatus("unknown");
      });

    return () => {
      cancelled = true;
    };
  }, []);

  async function requestSystemNotificationPermission() {
    if (requestingNoticePermission) return;
    setRequestingNoticePermission(true);

    try {
      const permission = await requestPermissionNative();
      const granted = permission === "granted";
      setNoticePermissionStatus(granted ? "granted" : "denied");
      toast(granted ? "系统通知权限已授权" : "系统通知权限已拒绝");
    } catch (err) {
      logToConsole("error", "请求系统通知权限失败", { error: String(err) });
      toast("请求系统通知权限失败：请查看控制台日志");
      setNoticePermissionStatus("unknown");
    } finally {
      setRequestingNoticePermission(false);
    }
  }

  async function sendSystemNotificationTest() {
    if (sendingNoticeTest) return;
    setSendingNoticeTest(true);

    try {
      const granted = await isPermissionGrantedNative();
      if (!granted) {
        setNoticePermissionStatus("not_granted");
        toast("请先在「系统通知」中授权通知权限");
        return;
      }

      const ok = await noticeSend({
        level: "info",
        title: "测试通知",
        body: "这是一条来自 AIO Coding Hub 的系统通知",
      });
      if (!ok) {
        return;
      }

      toast("已发送测试通知");
    } catch (err) {
      logToConsole("error", "发送测试通知失败", { error: String(err) });
      toast("发送测试通知失败：请查看控制台日志");
    } finally {
      setSendingNoticeTest(false);
    }
  }

  return {
    noticePermissionStatus,
    requestingNoticePermission,
    sendingNoticeTest,
    requestSystemNotificationPermission,
    sendSystemNotificationTest,
  };
}
