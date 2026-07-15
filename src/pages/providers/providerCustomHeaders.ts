import type { ProviderCustomHeader } from "../../services/providers/providers";

/**
 * Header names the gateway controls itself; users must not override them via
 * custom headers. Mirrors the backend's protected-header guard.
 */
const PROTECTED_HEADER_NAMES = new Set([
  "host",
  "content-length",
  "content-encoding",
  "transfer-encoding",
  "connection",
]);

/** RFC 7230 token chars allowed in an HTTP header field name. */
const HEADER_NAME_PATTERN = /^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/;

export function isProtectedCustomHeaderName(name: string): boolean {
  return PROTECTED_HEADER_NAMES.has(name.trim().toLowerCase());
}

export function isValidCustomHeaderName(name: string): boolean {
  return HEADER_NAME_PATTERN.test(name.trim());
}

/**
 * Trim entries, drop rows with an empty name, and dedupe by case-insensitive
 * name keeping the last occurrence. Keeps the same semantics as the backend
 * normalizer so the UI and persisted value agree.
 */
export function normalizeCustomHeaders(headers: ProviderCustomHeader[]): ProviderCustomHeader[] {
  const byName = new Map<string, ProviderCustomHeader>();
  for (const header of headers) {
    const name = header.name.trim();
    if (!name) continue;
    byName.set(name.toLowerCase(), { name, value: header.value.trim() });
  }
  return Array.from(byName.values());
}

/**
 * Validate the in-progress rows before save. Returns an error message when a
 * row has a value but no name, an invalid header name, or targets a protected
 * header; otherwise null.
 */
export function validateCustomHeaders(headers: ProviderCustomHeader[]): string | null {
  for (const header of headers) {
    const name = header.name.trim();
    if (!name) {
      if (header.value.trim()) return "请求头名称不能为空";
      continue;
    }
    if (!isValidCustomHeaderName(name)) {
      return `请求头名称无效：${name}`;
    }
    if (isProtectedCustomHeaderName(name)) {
      return `请求头 ${name} 由网关管理，无法自定义`;
    }
  }
  return null;
}
