import { commands, type FrontendErrorReportInput } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type { FrontendErrorReportInput };

const FRONTEND_ERROR_SOURCE_VALUES = ["error", "unhandledrejection", "render"] as const;
export const FRONTEND_ERROR_MESSAGE_MAX_CHARS = 4096;
export const FRONTEND_ERROR_STACK_MAX_CHARS = 16_384;
export const FRONTEND_ERROR_DETAILS_MAX_CHARS = 16_384;
export const FRONTEND_ERROR_HREF_MAX_CHARS = 2048;
export const FRONTEND_ERROR_USER_AGENT_MAX_CHARS = 1024;

function truncateChars(value: string, maxChars: number): string {
  const chars = [...value];
  if (chars.length <= maxChars) return value;
  return chars.slice(0, maxChars).join("");
}

function normalizeFrontendErrorSource(source: string): string {
  const normalized = source.trim();
  if ((FRONTEND_ERROR_SOURCE_VALUES as readonly string[]).includes(normalized)) {
    return normalized;
  }
  throw new Error(`SEC_INVALID_INPUT: invalid frontend error source=${source}`);
}

function normalizeRequiredReportText(value: string, label: string, maxChars: number): string {
  const normalized = value.trim();
  if (!normalized) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is required`);
  }
  return truncateChars(normalized, maxChars);
}

function normalizeOptionalReportText(
  value: string | null | undefined,
  maxChars: number
): string | null {
  if (value == null) return null;
  const normalized = value.trim();
  if (!normalized) return null;
  return truncateChars(normalized, maxChars);
}

export function normalizeFrontendErrorReportInput(
  input: FrontendErrorReportInput
): FrontendErrorReportInput {
  return {
    source: normalizeFrontendErrorSource(input.source),
    message: normalizeRequiredReportText(
      input.message,
      "message",
      FRONTEND_ERROR_MESSAGE_MAX_CHARS
    ),
    stack: normalizeOptionalReportText(input.stack, FRONTEND_ERROR_STACK_MAX_CHARS),
    detailsJson: normalizeOptionalReportText(input.detailsJson, FRONTEND_ERROR_DETAILS_MAX_CHARS),
    href: normalizeOptionalReportText(input.href, FRONTEND_ERROR_HREF_MAX_CHARS),
    userAgent: normalizeOptionalReportText(input.userAgent, FRONTEND_ERROR_USER_AGENT_MAX_CHARS),
  };
}

export async function appFrontendErrorReport(input: FrontendErrorReportInput) {
  const normalizedInput = normalizeFrontendErrorReportInput(input);

  return invokeGeneratedIpc<boolean>({
    title: "上报前端异常失败",
    cmd: "app_frontend_error_report",
    args: { input: normalizedInput },
    invoke: () =>
      commands.appFrontendErrorReport(normalizedInput) as Promise<GeneratedCommandResult<boolean>>,
  });
}
