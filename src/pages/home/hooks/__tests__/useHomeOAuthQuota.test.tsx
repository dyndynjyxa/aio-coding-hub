import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { RequestLogSummary } from "../../../../services/gateway/requestLogs";
import type { ProviderSummary } from "../../../../services/providers/providers";
import { providerOAuthFetchLimits, providersList } from "../../../../services/providers/providers";
import { oauthLimitsKeys } from "../../../../query/keys";
import { createQueryWrapper, createTestQueryClient } from "../../../../test/utils/reactQuery";
import { useHomeOAuthQuota } from "../useHomeOAuthQuota";

vi.mock("../../../../services/providers/providers", async () => {
  const actual = await vi.importActual<typeof import("../../../../services/providers/providers")>(
    "../../../../services/providers/providers"
  );
  return {
    ...actual,
    providersList: vi.fn(),
    providerOAuthFetchLimits: vi.fn(),
  };
});

function makeProvider(
  partial: Partial<ProviderSummary> & Pick<ProviderSummary, "id" | "cli_key" | "name">
): ProviderSummary {
  return {
    id: partial.id,
    cli_key: partial.cli_key,
    name: partial.name,
    base_urls: partial.base_urls ?? [],
    base_url_mode: partial.base_url_mode ?? "order",
    claude_models: partial.claude_models ?? {},
    enabled: partial.enabled ?? true,
    priority: partial.priority ?? 0,
    cost_multiplier: partial.cost_multiplier ?? 1,
    limit_5h_usd: partial.limit_5h_usd ?? null,
    limit_daily_usd: partial.limit_daily_usd ?? null,
    daily_reset_mode: partial.daily_reset_mode ?? "fixed",
    daily_reset_time: partial.daily_reset_time ?? "00:00:00",
    limit_weekly_usd: partial.limit_weekly_usd ?? null,
    limit_monthly_usd: partial.limit_monthly_usd ?? null,
    limit_total_usd: partial.limit_total_usd ?? null,
    tags: partial.tags ?? [],
    note: partial.note ?? "",
    created_at: partial.created_at ?? 0,
    updated_at: partial.updated_at ?? 0,
    auth_mode: partial.auth_mode ?? "api_key",
    oauth_provider_type: partial.oauth_provider_type ?? null,
    oauth_email: partial.oauth_email ?? null,
    oauth_expires_at: partial.oauth_expires_at ?? null,
    oauth_last_error: partial.oauth_last_error ?? null,
    source_provider_id: partial.source_provider_id ?? null,
    bridge_type: partial.bridge_type ?? null,
    stream_idle_timeout_seconds: partial.stream_idle_timeout_seconds ?? null,
    api_key_configured: partial.api_key_configured ?? false,
  };
}

function makeRequestLog(
  partial: Partial<RequestLogSummary> & Pick<RequestLogSummary, "id">
): RequestLogSummary {
  return {
    id: partial.id,
    trace_id: partial.trace_id ?? `trace-${partial.id}`,
    cli_key: partial.cli_key ?? "codex",
    session_id: partial.session_id ?? null,
    method: partial.method ?? "POST",
    path: partial.path ?? "/v1/messages",
    excluded_from_stats: partial.excluded_from_stats ?? false,
    special_settings_json: partial.special_settings_json ?? null,
    requested_model: partial.requested_model ?? null,
    status: partial.status ?? 200,
    error_code: partial.error_code ?? null,
    duration_ms: partial.duration_ms ?? 1000,
    ttfb_ms: partial.ttfb_ms ?? null,
    attempt_count: partial.attempt_count ?? 1,
    has_failover: partial.has_failover ?? false,
    start_provider_id: partial.start_provider_id ?? 0,
    start_provider_name: partial.start_provider_name ?? "start",
    final_provider_id: partial.final_provider_id ?? 0,
    final_provider_name: partial.final_provider_name ?? "final",
    final_provider_source_id: partial.final_provider_source_id ?? null,
    final_provider_source_name: partial.final_provider_source_name ?? null,
    route: partial.route ?? [],
    session_reuse: partial.session_reuse ?? false,
    input_tokens: partial.input_tokens ?? null,
    output_tokens: partial.output_tokens ?? null,
    total_tokens: partial.total_tokens ?? null,
    cache_read_input_tokens: partial.cache_read_input_tokens ?? null,
    cache_creation_input_tokens: partial.cache_creation_input_tokens ?? null,
    cache_creation_5m_input_tokens: partial.cache_creation_5m_input_tokens ?? null,
    cache_creation_1h_input_tokens: partial.cache_creation_1h_input_tokens ?? null,
    cost_usd: partial.cost_usd ?? null,
    provider_chain_json: partial.provider_chain_json ?? null,
    error_details_json: partial.error_details_json ?? null,
    cost_multiplier: partial.cost_multiplier ?? 1,
    created_at_ms: partial.created_at_ms ?? (partial.created_at ?? 0) * 1000,
    created_at: partial.created_at ?? 0,
  };
}

describe("pages/home/hooks/useHomeOAuthQuota", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(providersList).mockImplementation(async (cliKey) => {
      if (cliKey === "codex") {
        return [
          makeProvider({
            id: 11,
            cli_key: "codex",
            name: "Codex OAuth",
            auth_mode: "oauth",
          }),
        ];
      }
      return [];
    });
  });

  it("does not fetch OAuth limits automatically", async () => {
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(
      () => useHomeOAuthQuota({ cliPriorityOrder: ["claude", "codex", "gemini"] }),
      { wrapper }
    );

    await waitFor(() => expect(result.current.oauthQuotaVisible).toBe(true));
    expect(providerOAuthFetchLimits).not.toHaveBeenCalled();
    expect(result.current.oauthQuotaRows[0]?.state).toBe("idle");
  });

  it("reuses existing OAuth limit cache without issuing a new request", async () => {
    const client = createTestQueryClient();
    client.setQueryData(oauthLimitsKeys.detail(11), {
      limit_short_label: "5h",
      limit_5h_text: "61%",
      limit_weekly_text: "92%",
      limit_5h_reset_at: null,
      limit_weekly_reset_at: null,
    });
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(
      () => useHomeOAuthQuota({ cliPriorityOrder: ["claude", "codex", "gemini"] }),
      { wrapper }
    );

    await waitFor(() => expect(result.current.oauthQuotaRows[0]?.state).toBe("success"));
    expect(providerOAuthFetchLimits).not.toHaveBeenCalled();
    expect(result.current.oauthQuotaRows[0]?.limits?.limit_5h_text).toBe("61%");
  });

  it("refreshes OAuth limits manually and writes the result into cache", async () => {
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    vi.mocked(providerOAuthFetchLimits).mockResolvedValue({
      limit_short_label: "5h",
      limit_5h_text: "44%",
      limit_weekly_text: "88%",
      limit_5h_reset_at: null,
      limit_weekly_reset_at: null,
    });

    const { result } = renderHook(
      () => useHomeOAuthQuota({ cliPriorityOrder: ["claude", "codex", "gemini"] }),
      { wrapper }
    );

    await waitFor(() => expect(result.current.oauthQuotaVisible).toBe(true));

    await act(async () => {
      await result.current.refreshOAuthQuota();
    });

    expect(providerOAuthFetchLimits).toHaveBeenCalledWith(11);
    expect(result.current.oauthQuotaHasRefreshed).toBe(true);
    expect(client.getQueryData(oauthLimitsKeys.detail(11))).toEqual({
      limit_short_label: "5h",
      limit_5h_text: "44%",
      limit_weekly_text: "88%",
      limit_5h_reset_at: null,
      limit_weekly_reset_at: null,
    });
  });

  it("refreshes only the selected OAuth provider when using row refresh", async () => {
    vi.mocked(providersList).mockImplementation(async (cliKey) => {
      if (cliKey === "codex") {
        return [
          makeProvider({
            id: 11,
            cli_key: "codex",
            name: "Codex OAuth",
            auth_mode: "oauth",
          }),
        ];
      }
      if (cliKey === "gemini") {
        return [
          makeProvider({
            id: 22,
            cli_key: "gemini",
            name: "Gemini OAuth",
            auth_mode: "oauth",
          }),
        ];
      }
      return [];
    });

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    vi.mocked(providerOAuthFetchLimits).mockImplementation(async (providerId) => ({
      limit_short_label: providerId === 11 ? "5h" : "短窗",
      limit_5h_text: providerId === 11 ? "44%" : "12%",
      limit_weekly_text: providerId === 11 ? "88%" : "63%",
      limit_5h_reset_at: null,
      limit_weekly_reset_at: null,
    }));

    const { result } = renderHook(
      () => useHomeOAuthQuota({ cliPriorityOrder: ["claude", "codex", "gemini"] }),
      { wrapper }
    );

    await waitFor(() => expect(result.current.oauthQuotaRows).toHaveLength(2));

    await act(async () => {
      await result.current.refreshOAuthQuotaRow(22);
    });

    expect(providerOAuthFetchLimits).toHaveBeenCalledTimes(1);
    expect(providerOAuthFetchLimits).toHaveBeenCalledWith(22);
    expect(client.getQueryData(oauthLimitsKeys.detail(11))).toBeUndefined();
    expect(client.getQueryData(oauthLimitsKeys.detail(22))).toEqual({
      limit_short_label: "短窗",
      limit_5h_text: "12%",
      limit_weekly_text: "63%",
      limit_5h_reset_at: null,
      limit_weekly_reset_at: null,
    });
  });

  it("sorts OAuth providers by recent usage from request logs", async () => {
    vi.mocked(providersList).mockImplementation(async (cliKey) => {
      if (cliKey === "codex") {
        return [
          makeProvider({
            id: 11,
            cli_key: "codex",
            name: "Codex OAuth",
            auth_mode: "oauth",
          }),
        ];
      }
      if (cliKey === "gemini") {
        return [
          makeProvider({
            id: 22,
            cli_key: "gemini",
            name: "Gemini OAuth",
            auth_mode: "oauth",
          }),
        ];
      }
      return [];
    });

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const requestLogs: RequestLogSummary[] = [
      makeRequestLog({
        id: 1,
        cli_key: "codex",
        final_provider_id: 11,
        final_provider_name: "Codex OAuth",
        route: [
          {
            provider_id: 11,
            provider_name: "Codex OAuth",
            ok: true,
            attempts: 1,
            skipped: false,
            status: 200,
          },
        ],
        created_at_ms: 1_000,
        created_at: 1,
      }),
      makeRequestLog({
        id: 2,
        cli_key: "gemini",
        final_provider_id: 22,
        final_provider_name: "Gemini OAuth",
        route: [
          {
            provider_id: 22,
            provider_name: "Gemini OAuth",
            ok: true,
            attempts: 1,
            skipped: false,
            status: 200,
          },
        ],
        created_at_ms: 2_000,
        created_at: 2,
      }),
    ];

    const { result } = renderHook(
      () =>
        useHomeOAuthQuota({
          cliPriorityOrder: ["claude", "codex", "gemini"],
          requestLogs,
        }),
      { wrapper }
    );

    await waitFor(() => expect(result.current.oauthQuotaRows).toHaveLength(2));
    expect(result.current.oauthQuotaRows.map((row) => row.providerId)).toEqual([22, 11]);
  });
});
