/**
 * Notice（系统通知）模块 - 前端调用入口
 *
 * 用法：
 * - 在任意页面：`await noticeSend({ level: "info", body: "..." })`
 * - `title` 为空时，Rust 会按 level 生成默认标题并追加固定前缀
 */

import { invokeServiceCommand } from "../invokeServiceCommand";

export type NoticeLevel = "info" | "success" | "warning" | "error";

export type NoticeSendParams = {
  level: NoticeLevel;
  title?: string;
  body: string;
};

export async function noticeSend(params: NoticeSendParams): Promise<boolean> {
  return invokeServiceCommand<boolean, false>({
    title: "发送系统通知失败",
    cmd: "notice_send",
    args: params,
    fallback: false,
  });
}
