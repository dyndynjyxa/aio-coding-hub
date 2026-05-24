// Usage: Helpers for displaying and validating provider base URLs.

import type { ProviderSummary } from "../../services/providers/providers";
import type { BaseUrlRow } from "./types";

export const MAX_PROVIDER_BASE_URLS = 32;
export const MAX_PROVIDER_BASE_URL_CHARS = 2048;

export function providerPrimaryBaseUrl(provider: ProviderSummary | null | undefined) {
  return provider?.base_urls?.[0] ?? "—";
}

export function providerBaseUrlSummary(provider: ProviderSummary | null | undefined) {
  if (!provider) return "—";
  const primary = providerPrimaryBaseUrl(provider);
  if (primary === "—" && provider.auth_mode === "oauth") return "OAuth (自动)";

  const urls = provider.base_urls ?? [];
  if (urls.length <= 1) return primary;

  const visibleUrls = urls.slice(0, 2);
  const extraCount = Math.max(0, urls.length - visibleUrls.length);
  const summary = visibleUrls.join(" · ");
  return extraCount > 0 ? `${summary} (+${extraCount})` : summary;
}

export function resolveProviderLabel(
  name: string | null | undefined,
  id: number | null | undefined
): string | null {
  const trimmed = name?.trim();
  if (trimmed && trimmed !== "Unknown") return trimmed;
  if (typeof id === "number" && id > 0) return `#${id}`;
  return null;
}

export function normalizeBaseUrlRows(rows: BaseUrlRow[]) {
  const baseUrls: string[] = [];
  const seen = new Set<string>();

  for (const row of rows) {
    const url = row.url.trim();
    if (!url) continue;

    if ([...url].length > MAX_PROVIDER_BASE_URL_CHARS) {
      return {
        ok: false as const,
        message: `Base URL 不能超过 ${MAX_PROVIDER_BASE_URL_CHARS} 字符：${url.slice(0, 80)}`,
      };
    }

    try {
      const parsed = new URL(url);
      if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
        return { ok: false as const, message: `Base URL 协议必须是 http/https：${url}` };
      }
    } catch {
      return { ok: false as const, message: `Base URL 格式不合法：${url}` };
    }

    if (seen.has(url)) {
      return { ok: false as const, message: `Base URL 重复：${url}` };
    }
    seen.add(url);
    baseUrls.push(url);

    if (baseUrls.length > MAX_PROVIDER_BASE_URLS) {
      return {
        ok: false as const,
        message: `Base URL 最多支持 ${MAX_PROVIDER_BASE_URLS} 个`,
      };
    }
  }

  if (baseUrls.length === 0) {
    return { ok: false as const, message: "至少需要 1 个 Base URL" };
  }

  return { ok: true as const, baseUrls };
}
