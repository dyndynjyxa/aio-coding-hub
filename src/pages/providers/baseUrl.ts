// Usage: Helpers for displaying and validating provider base URLs.

import type { ProviderSummary } from "../../services/providers/providers";
import type { BaseUrlRow } from "./types";

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
  if (trimmed) return trimmed;
  if (typeof id === "number") return `#${id}`;
  return null;
}

export function normalizeBaseUrlRows(rows: BaseUrlRow[]) {
  const baseUrls: string[] = [];
  const seen = new Set<string>();

  for (const row of rows) {
    const url = row.url.trim();
    if (!url) continue;

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
  }

  if (baseUrls.length === 0) {
    return { ok: false as const, message: "至少需要 1 个 Base URL" };
  }

  return { ok: true as const, baseUrls };
}
