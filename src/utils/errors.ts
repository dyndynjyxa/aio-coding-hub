const ERROR_TEXT_MAX_CHARS = 4096;
const ERROR_FIELD_STRING_MAX_CHARS = 512;
const ERROR_OBJECT_MAX_DEPTH = 4;
const ERROR_OBJECT_MAX_ARRAY_ITEMS = 25;
const ERROR_OBJECT_MAX_KEYS = 25;

function truncateErrorText(value: string, maxChars = ERROR_TEXT_MAX_CHARS) {
  if (value.length <= maxChars) return value;
  return `${value.slice(0, maxChars)}[Truncated ${value.length - maxChars} chars]`;
}

function sanitizeErrorValue(value: unknown, seen: WeakSet<object>, depth: number): unknown {
  if (value == null) return value;
  if (typeof value === "string") return truncateErrorText(value, ERROR_FIELD_STRING_MAX_CHARS);
  if (typeof value === "number" || typeof value === "boolean") return value;
  if (typeof value === "bigint") return value.toString();
  if (typeof value === "function") return "[Function]";
  if (typeof value !== "object") return String(value);
  if (seen.has(value)) return "[Circular]";
  if (depth >= ERROR_OBJECT_MAX_DEPTH) return "[Truncated]";

  seen.add(value);

  if (Array.isArray(value)) {
    const items = value
      .slice(0, ERROR_OBJECT_MAX_ARRAY_ITEMS)
      .map((item) => sanitizeErrorValue(item, seen, depth + 1));
    if (value.length > ERROR_OBJECT_MAX_ARRAY_ITEMS) {
      items.push(`[Truncated ${value.length - ERROR_OBJECT_MAX_ARRAY_ITEMS} items]`);
    }
    return items;
  }

  const output: Record<string, unknown> = {};
  const keys = Object.keys(value).slice(0, ERROR_OBJECT_MAX_KEYS);
  for (const key of keys) {
    try {
      output[key] = sanitizeErrorValue((value as Record<string, unknown>)[key], seen, depth + 1);
    } catch {
      output[key] = "[Unreadable]";
    }
  }
  const omittedKeys = Object.keys(value).length - ERROR_OBJECT_MAX_KEYS;
  if (omittedKeys > 0) {
    output.__truncated__ = `${omittedKeys} keys truncated`;
  }
  return output;
}

function stringifyErrorObject(value: object) {
  try {
    const serialized = JSON.stringify(sanitizeErrorValue(value, new WeakSet(), 0));
    if (serialized) return truncateErrorText(serialized);
  } catch {
    // Fall through to String below.
  }
  return null;
}

export function formatUnknownError(err: unknown) {
  if (typeof err === "string") return truncateErrorText(err);
  if (err instanceof Error && err.message) return truncateErrorText(err.message);
  if (err && typeof err === "object") {
    const maybeMessage = (err as { message?: unknown }).message;
    if (typeof maybeMessage === "string" && maybeMessage.trim()) {
      return truncateErrorText(maybeMessage);
    }
    const serialized = stringifyErrorObject(err);
    if (serialized) return serialized;
  }
  try {
    return truncateErrorText(String(err));
  } catch {
    return "未知错误";
  }
}

export function parseErrorCodeMessage(raw: string): {
  error_code: string | null;
  message: string;
} {
  const trimmed = raw.trim();
  const msg = trimmed.replace(/^Error:\s*/i, "").trim();
  if (!msg) return { error_code: null, message: "未知错误" };

  const match = /^([A-Z][A-Z0-9_]*):\s*(.*)$/.exec(msg);
  if (!match) return { error_code: null, message: msg };
  const code = match[1] || null;
  const rest = (match[2] ?? "").trim();
  return { error_code: code, message: rest || msg };
}

export function compactWhitespace(text: string) {
  return text.replace(/\s+/g, " ").trim();
}

export function normalizeErrorWithCode(err: unknown): {
  raw: string;
  error_code: string | null;
  message: string;
} {
  const raw = formatUnknownError(err);
  const { error_code, message } = parseErrorCodeMessage(raw);
  return { raw, error_code, message: compactWhitespace(message) };
}

export function formatActionFailureToast(
  action: string,
  err: unknown
): {
  raw: string;
  error_code: string | null;
  message: string;
  toast: string;
} {
  const normalized = normalizeErrorWithCode(err);
  const codeLabel = normalized.error_code ? `（code ${normalized.error_code}）` : "";
  return {
    ...normalized,
    toast: `${action}失败${codeLabel}：${normalized.message}`,
  };
}
