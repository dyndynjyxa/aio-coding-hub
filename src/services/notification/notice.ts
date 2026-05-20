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
const NOTICE_LEVEL_VALUES = [
  "info",
  "success",
  "warning",
  "error",
] as const satisfies readonly NoticeLevel[];
export const NOTICE_TITLE_MAX_CHARS = 128;
export const NOTICE_BODY_MAX_CHARS = 4096;

export type NoticeSendParams = {
  level: NoticeLevel;
  title?: string;
  body: string;
};

function normalizeNoticeLevel(value: string): NoticeLevel {
  const normalized = value.trim();
  if ((NOTICE_LEVEL_VALUES as readonly string[]).includes(normalized)) {
    return normalized as NoticeLevel;
  }
  throw new Error(`SEC_INVALID_INPUT: invalid notice level=${value}`);
}

function normalizeNoticeText(value: string, label: string, maxChars: number): string {
  const normalized = value.trim();
  if (!normalized) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is required`);
  }
  if ([...normalized].length > maxChars) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is too long (max ${maxChars} chars)`);
  }
  return normalized;
}

function normalizeOptionalNoticeText(
  value: string | null | undefined,
  label: string
): string | null {
  if (value == null) return null;
  const normalized = value.trim();
  if (!normalized) return null;
  if ([...normalized].length > NOTICE_TITLE_MAX_CHARS) {
    throw new Error(
      `SEC_INVALID_INPUT: ${label} is too long (max ${NOTICE_TITLE_MAX_CHARS} chars)`
    );
  }
  return normalized;
}

export function normalizeNoticeSendInput(params: NoticeSendParams) {
  return {
    level: normalizeNoticeLevel(params.level),
    title: normalizeOptionalNoticeText(params.title, "title"),
    body: normalizeNoticeText(params.body, "body", NOTICE_BODY_MAX_CHARS),
  };
}

export async function noticeSend(params: NoticeSendParams): Promise<boolean> {
  const input = normalizeNoticeSendInput(params);

  return invokeGeneratedIpc<boolean>({
    title: "发送系统通知失败",
    cmd: "notice_send",
    args: { input },
    invoke: () => commands.noticeSend(input) as Promise<GeneratedCommandResult<boolean>>,
  });
}
