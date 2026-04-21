/**
 * Notice（系统通知）模块 - 前端调用入口
 *
 * 用法：
 * - 在任意页面：`await noticeSend({ level: "info", body: "..." })`
 * - `title` 为空时，Rust 会按 level 生成默认标题并追加固定前缀
 */

import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type NoticeLevel = "info" | "success" | "warning" | "error";

export type NoticeSendParams = {
  level: NoticeLevel;
  title?: string;
  body: string;
};

export async function noticeSend(params: NoticeSendParams): Promise<boolean> {
  const input = {
    level: params.level,
    title: params.title ?? null,
    body: params.body,
  };

  return invokeGeneratedIpc<boolean>({
    title: "发送系统通知失败",
    cmd: "notice_send",
    args: { input },
    invoke: () =>
      commands.noticeSend(input) as Promise<GeneratedCommandResult<boolean>>,
  });
}
