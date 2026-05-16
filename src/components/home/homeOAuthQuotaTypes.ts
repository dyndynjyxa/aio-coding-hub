import type { CliKey, OAuthLimitsResult } from "../../services/providers/providers";

export type HomeOAuthQuotaRowState = "idle" | "loading" | "success" | "error";

export type HomeOAuthQuotaRow = {
  providerId: number;
  cliKey: CliKey;
  providerName: string;
  enabled: boolean;
  state: HomeOAuthQuotaRowState;
  limits: OAuthLimitsResult | null;
  error: string | null;
};

export function hasHomeOAuthQuotaText(limits: OAuthLimitsResult | null): boolean {
  return Boolean(limits?.limit_5h_text || limits?.limit_weekly_text);
}

function isExhaustedQuotaText(value: string | null | undefined): boolean {
  const text = value?.trim();
  if (!text) return false;
  if (/^0(?:\.0+)?\s*%$/.test(text)) return true;
  return /^0(?:\.0+)?$/.test(text);
}

export function hasInsufficientHomeOAuthQuota(limits: OAuthLimitsResult | null): boolean {
  return (
    isExhaustedQuotaText(limits?.limit_5h_text) || isExhaustedQuotaText(limits?.limit_weekly_text)
  );
}
