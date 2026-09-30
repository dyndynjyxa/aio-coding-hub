import type { ProviderCustomHeader } from "../../services/providers/providers";

// Mirrors shared/provider_headers.rs. The backend remains authoritative.
const PROTECTED_HEADER_NAMES = new Set([
  "authorization",
  "x-api-key",
  "x-goog-api-key",
  "x-goog-api-client",
  "chatgpt-account-id",
  "proxy-authorization",
  "proxy-authenticate",
  "host",
  "content-length",
  "content-encoding",
  "transfer-encoding",
  "connection",
  "keep-alive",
  "te",
  "trailer",
  "upgrade",
  "x-trace-id",
  "session-id",
  "session_id",
  "x-session-id",
  "x-codex-turn-state",
  "x-codex-turn-metadata",
]);
const HEADER_NAME_PATTERN = /^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/;
const trimHttpWhitespace = (value: string) => value.replace(/^[ \t]+|[ \t]+$/g, "");

export function isProtectedCustomHeaderName(name: string): boolean {
  const normalized = trimHttpWhitespace(name).toLowerCase();
  return (
    normalized.startsWith("x-aio-") ||
    normalized.startsWith("sec-websocket-") ||
    PROTECTED_HEADER_NAMES.has(normalized)
  );
}

export function isValidCustomHeaderName(name: string): boolean {
  return HEADER_NAME_PATTERN.test(trimHttpWhitespace(name));
}

/** Call after validation; omit only empty editor placeholders. */
export function normalizeCustomHeaders(headers: ProviderCustomHeader[]): ProviderCustomHeader[] {
  const byName = new Map<string, ProviderCustomHeader>();
  for (const header of headers) {
    const name = trimHttpWhitespace(header.name).toLowerCase();
    if (!name && !trimHttpWhitespace(header.value)) continue;
    byName.set(name, { name, value: trimHttpWhitespace(header.value) });
  }
  return Array.from(byName.values()).sort((a, b) =>
    a.name < b.name ? -1 : a.name > b.name ? 1 : 0
  );
}

export function validateCustomHeaders(headers: ProviderCustomHeader[]): string | null {
  for (const header of headers) {
    const name = trimHttpWhitespace(header.name);
    const value = trimHttpWhitespace(header.value);
    if (!name && !value) continue;
    if (!name) return "请求头名称不能为空";
    if (!isValidCustomHeaderName(header.name)) return "请求头名称无效";
    if (isProtectedCustomHeaderName(name)) return "该请求头由网关管理，无法自定义";
    // HeaderValue accepts HTAB and visible UTF-8 bytes, but not other controls.
    if (
      Array.from(header.value).some((char) => {
        const code = char.charCodeAt(0);
        return (code < 32 && code !== 9) || code === 127;
      })
    )
      return "请求头值包含非法控制字符";
    if (!value) return "请求头值不能为空";
  }
  return null;
}
