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
