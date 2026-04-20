import { logToConsole } from "./consoleLog";
import { appFrontendErrorReport, type FrontendErrorReportInput } from "./app/frontendErrorReport";

type FrontendErrorSource = "error" | "unhandledrejection" | "render";

type FrontendErrorPayload = Omit<FrontendErrorReportInput, "source"> & {
  source: FrontendErrorSource;
  message: string;
  stack?: string | null;
  detailsJson?: string | null;
  href?: string | null;
  userAgent?: string | null;
};

const DEDUP_WINDOW_MS = 3_000;
const MAX_DEDUP_KEYS = 200;
const MAX_MESSAGE_LENGTH = 2_000;
const MAX_STACK_LENGTH = 10_000;
const MAX_JSON_LENGTH = 10_000;

const dedupMap = new Map<string, number>();

let installed = false;

export function __testResetFrontendErrorReporterState() {
  dedupMap.clear();
  installed = false;
}

function truncateText(value: string, max: number): string {
  if (value.length <= max) return value;
  return `${value.slice(0, max)}…`;
}

function safeToString(value: unknown): string {
  if (typeof value === "string") return value;
  if (value instanceof Error) return value.message || String(value);
  try {
    return String(value);
  } catch {
    return "[unstringifiable]";
  }
}

function safeJson(value: unknown, maxLength = MAX_JSON_LENGTH): string | null {
  try {
    const text = JSON.stringify(value);
    if (!text) return null;
    return truncateText(text, maxLength);
  } catch {
    return null;
  }
}

function normalizeStack(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  if (!trimmed) return null;
  return truncateText(trimmed, MAX_STACK_LENGTH);
}

function makeDedupKey(payload: FrontendErrorPayload): string {
  return [
    payload.source,
    payload.message,
    payload.stack ?? "",
    payload.href ?? "",
    payload.detailsJson ?? "",
  ].join("|");
}

function shouldSend(payload: FrontendErrorPayload): boolean {
  const key = makeDedupKey(payload);
  const now = Date.now();
  const last = dedupMap.get(key);
  if (last != null && now - last < DEDUP_WINDOW_MS) return false;

  dedupMap.set(key, now);
  if (dedupMap.size > MAX_DEDUP_KEYS) {
    dedupMap.clear();
    dedupMap.set(key, now);
  }
  return true;
}

async function send(payload: FrontendErrorPayload): Promise<void> {
  if (!shouldSend(payload)) return;

  logToConsole("error", `前端异常（${payload.source}）`, {
    message: payload.message,
    stack: payload.stack ?? null,
    details: payload.detailsJson ?? null,
    href: payload.href ?? null,
  });

  try {
    await appFrontendErrorReport(payload);
  } catch {
    // swallow: avoid recursive crash loops in reporter itself
  }
}

function normalizeMessage(value: unknown): string {
  const text = safeToString(value).trim();
  if (!text) return "Unknown frontend error";
  return truncateText(text, MAX_MESSAGE_LENGTH);
}

function buildSharedMeta() {
  return {
    href: typeof location !== "undefined" ? location.href : null,
    userAgent: typeof navigator !== "undefined" ? navigator.userAgent : null,
  };
}

function reportWindowError(event: ErrorEvent) {
  const message = normalizeMessage(event.message || event.error);
  const stack = normalizeStack(event.error instanceof Error ? event.error.stack : null);
  const details = safeJson({
    filename: event.filename || null,
    lineno: event.lineno || null,
    colno: event.colno || null,
  });

  void send({
    source: "error",
    message,
    stack,
    detailsJson: details,
    ...buildSharedMeta(),
  });
}

function reportUnhandledRejection(event: PromiseRejectionEvent) {
  const reason = event.reason;
  const message = normalizeMessage(reason instanceof Error ? reason.message : reason);
  const stack = normalizeStack(reason instanceof Error ? reason.stack : null);
  const details = safeJson({
    reason_type: typeof reason,
    reason: reason instanceof Error ? reason.message : safeToString(reason),
  });

  void send({
    source: "unhandledrejection",
    message,
    stack,
    detailsJson: details,
    ...buildSharedMeta(),
  });
}

export function reportRenderError(error: unknown, errorInfo?: { componentStack?: string }) {
  const message = normalizeMessage(error instanceof Error ? error.message : error);
  const stack = normalizeStack(error instanceof Error ? error.stack : null);
  const details = safeJson({
    component_stack: errorInfo?.componentStack ?? null,
  });

  void send({
    source: "render",
    message,
    stack,
    detailsJson: details,
    ...buildSharedMeta(),
  });
}

export function installGlobalErrorReporting() {
  if (installed) return;
  if (typeof window === "undefined") return;

  window.addEventListener("error", reportWindowError);
  window.addEventListener("unhandledrejection", reportUnhandledRejection);

  installed = true;
}
