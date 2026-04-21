import type { ClaudeModelValidationResult } from "./claudeModelValidation";

type RequestSnapshot = {
  url: string;
  headers: Record<string, string>;
  body: unknown;
};

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function normalizeNonEmptyString(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function sortRecordKeys(record: Record<string, string>) {
  const out: Record<string, string> = {};
  for (const k of Object.keys(record).sort((a, b) => a.localeCompare(b))) {
    out[k] = record[k] ?? "";
  }
  return out;
}

function normalizeHeaderNameToLowercase(name: string) {
  return name.trim().toLowerCase();
}

function buildAuthHeaderValue(input: {
  apiKeyPlaintext: string | null;
  wantsAuthorization: boolean;
}): string {
  const key = input.apiKeyPlaintext?.trim() || "***";
  return input.wantsAuthorization ? `Bearer ${key}` : key;
}

function buildRequestHeadersFromUnknown(
  headersValue: unknown,
  apiKeyPlaintext: string | null
): Record<string, string> {
  const out: Record<string, string> = {};

  const headersObj = isPlainObject(headersValue) ? (headersValue as Record<string, unknown>) : null;
  const headerNames = headersObj ? Object.keys(headersObj) : [];
  const wantsAuthorization = headerNames.some(
    (k) => normalizeHeaderNameToLowercase(k) === "authorization"
  );

  if (headersObj) {
    for (const [rawName, rawValue] of Object.entries(headersObj)) {
      if (typeof rawValue !== "string") continue;
      const name = normalizeHeaderNameToLowercase(rawName);
      if (!name) continue;
      if (name === "x-api-key" || name === "authorization" || name === "host") continue;
      out[name] = rawValue;
    }
  }

  if (wantsAuthorization) {
    out["authorization"] = buildAuthHeaderValue({ apiKeyPlaintext, wantsAuthorization: true });
  } else {
    out["x-api-key"] = buildAuthHeaderValue({ apiKeyPlaintext, wantsAuthorization: false });
  }

  return sortRecordKeys(out);
}

function parseForwardedPathAndQuery(pathValue: unknown): {
  path: string;
  queryFromPath: string | null;
} {
  const raw = normalizeNonEmptyString(pathValue);
  if (!raw) return { path: "/v1/messages", queryFromPath: null };

  const [pathPart, queryPart] = (() => {
    const idx = raw.indexOf("?");
    if (idx < 0) return [raw, null] as const;
    return [raw.slice(0, idx), raw.slice(idx + 1)] as const;
  })();

  const trimmedPathPart = pathPart.trim();
  if (!trimmedPathPart) return { path: "/v1/messages", queryFromPath: null };

  const path = trimmedPathPart.startsWith("/") ? trimmedPathPart : `/${trimmedPathPart}`;

  const queryFromPath =
    normalizeNonEmptyString(queryPart)
      ?.replace(/^\?+/, "")
      ?.trim()
      ?.trimStart()
      ?.trim()
      ?.replace(/^\?+/, "") ?? null;

  return { path, queryFromPath };
}

function normalizeQuery(value: unknown): string | null {
  const raw = normalizeNonEmptyString(value);
  if (!raw) return null;
  return raw.replace(/^\?+/, "").trim() || null;
}

function buildTargetUrlFromWrapper(input: {
  baseUrl: string;
  forwardedPath: string;
  forwardedQuery: string | null;
}): string | null {
  const baseUrl = input.baseUrl.trim();
  if (!baseUrl) return null;

  let url: URL;
  try {
    url = new URL(baseUrl);
  } catch {
    return null;
  }

  const basePath = url.pathname.replace(/\/+$/, "");

  let forwardedPath = input.forwardedPath.trim();
  if (!forwardedPath) forwardedPath = "/v1/messages";
  if (!forwardedPath.startsWith("/")) forwardedPath = `/${forwardedPath}`;

  if (basePath.endsWith("/v1") && (forwardedPath === "/v1" || forwardedPath.startsWith("/v1/"))) {
    forwardedPath = forwardedPath.slice("/v1".length) || "";
  }

  let combinedPath = `${basePath}${forwardedPath}`;
  if (!combinedPath) combinedPath = "/";
  if (!combinedPath.startsWith("/")) combinedPath = `/${combinedPath}`;

  url.pathname = combinedPath;

  const forwardedQuery = (input.forwardedQuery ?? "").trim().replace(/^\?+/, "");
  url.search = forwardedQuery ? `?${forwardedQuery}` : "";

  return url.toString();
}

function extractBodyAndHeadersFromRequestValue(requestValue: unknown): {
  headersValue: unknown;
  body: unknown;
} {
  if (!isPlainObject(requestValue)) {
    return { headersValue: null, body: requestValue };
  }

  if ("body" in requestValue) {
    const headersValue = requestValue.headers;
    const body = requestValue.body;
    return { headersValue, body };
  }

  return { headersValue: null, body: requestValue };
}

export function buildClaudeModelValidationRequestSnapshotTextFromResult(
  result: ClaudeModelValidationResult | null,
  apiKeyPlaintext?: string | null
): string {
  const targetUrl = normalizeNonEmptyString(result?.target_url);
  if (!targetUrl) return "";

  const { headersValue, body } = extractBodyAndHeadersFromRequestValue(result?.request);
  const headers = buildRequestHeadersFromUnknown(headersValue, apiKeyPlaintext ?? null);
  const snapshot: RequestSnapshot = { url: targetUrl, headers, body };

  try {
    return JSON.stringify(snapshot, null, 2);
  } catch {
    return "";
  }
}

export function buildClaudeModelValidationRequestSnapshotTextFromWrapper(input: {
  baseUrl: string;
  wrapperJsonText: string;
  apiKeyPlaintext?: string | null;
}): string {
  const baseUrl = input.baseUrl.trim();
  const wrapperText = input.wrapperJsonText.trim();
  if (!baseUrl || !wrapperText) return "";

  let value: unknown;
  try {
    value = JSON.parse(wrapperText);
  } catch {
    return "";
  }

  const { headersValue, body } = extractBodyAndHeadersFromRequestValue(value);
  const headers = buildRequestHeadersFromUnknown(headersValue, input.apiKeyPlaintext ?? null);

  const forwarded = parseForwardedPathAndQuery(isPlainObject(value) ? value.path : null);
  const query =
    normalizeQuery(isPlainObject(value) ? value.query : null) ?? forwarded.queryFromPath;

  const url =
    buildTargetUrlFromWrapper({
      baseUrl,
      forwardedPath: forwarded.path,
      forwardedQuery: query,
    }) ?? "";
  if (!url) return "";

  const snapshot: RequestSnapshot = { url, headers, body };
  try {
    return JSON.stringify(snapshot, null, 2);
  } catch {
    return "";
  }
}
