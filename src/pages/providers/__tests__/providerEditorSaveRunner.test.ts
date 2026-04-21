import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import type { ProviderSummary } from "../../../services/providers/providers";
import { DEFAULT_FORM_VALUES } from "../providerEditorUtils";
import { runProviderEditorSave } from "../providerEditorSaveRunner";
import type { SaveActionContext } from "../providerEditorActionContext";

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));

function makeSavedProvider(partial: Partial<ProviderSummary> = {}): ProviderSummary {
  return {
    id: partial.id ?? 1,
    cli_key: partial.cli_key ?? "claude",
    name: partial.name ?? "Saved Provider",
    base_urls: partial.base_urls ?? ["https://example.com/v1"],
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
    api_key_configured: partial.api_key_configured ?? true,
  };
}

function makeContext(overrides: Partial<SaveActionContext> = {}): SaveActionContext {
  const getValues = vi.fn().mockReturnValue({
    ...DEFAULT_FORM_VALUES,
    name: "Provider A",
    api_key: "sk-test",
  });

  return {
    mode: "create",
    cliKey: "claude",
    editingProviderId: null,
    editProvider: null,
    open: true,
    onOpenChange: vi.fn(),
    onSaved: vi.fn(),
    authMode: "api_key",
    baseUrlMode: "order",
    baseUrlRows: [{ id: "1", url: "https://example.com/v1", ping: { status: "idle" } }],
    tags: [],
    claudeModels: {},
    streamIdleTimeoutSeconds: "",
    apiKeyConfigured: false,
    isCodexGatewaySource: false,
    sourceProviderId: null,
    selectedCx2ccSourceProvider: null,
    formValues: getValues(),
    saving: false,
    setSaving: vi.fn(),
    form: {
      getValues,
      setValue: vi.fn(),
    },
    oauthStatus: null,
    setOauthStatus: vi.fn(),
    refreshOauthStatus: vi.fn().mockResolvedValue(null),
    persistProvider: vi.fn().mockResolvedValue(makeSavedProvider()),
    ...overrides,
  };
}

describe("pages/providers/providerEditorSaveRunner", () => {
  it("stops before persist when payload validation fails", async () => {
    const ctx = makeContext({
      form: {
        getValues: vi.fn().mockReturnValue({
          ...DEFAULT_FORM_VALUES,
          name: "",
          api_key: "",
        }),
        setValue: vi.fn(),
      },
    });

    await runProviderEditorSave(ctx);

    expect(vi.mocked(toast)).toHaveBeenCalled();
    expect(ctx.persistProvider).not.toHaveBeenCalled();
  });

  it("blocks oauth save when the provider is still disconnected", async () => {
    const ctx = makeContext({
      mode: "edit",
      editingProviderId: 7,
      authMode: "oauth",
      apiKeyConfigured: true,
      form: {
        getValues: vi.fn().mockReturnValue({
          ...DEFAULT_FORM_VALUES,
          name: "OAuth Provider",
          api_key: "",
          auth_mode: "oauth",
        }),
        setValue: vi.fn(),
      },
      refreshOauthStatus: vi.fn().mockResolvedValue({
        connected: false,
        provider_type: null,
        email: null,
        expires_at: null,
        has_refresh_token: null,
      }),
    });

    await runProviderEditorSave(ctx);

    expect(ctx.refreshOauthStatus).toHaveBeenCalledWith(7);
    expect(vi.mocked(toast)).toHaveBeenCalledWith("请先完成 OAuth 登录");
    expect(ctx.persistProvider).not.toHaveBeenCalled();
  });

  it("persists the provider and clears the draft api key on success", async () => {
    const ctx = makeContext();

    await runProviderEditorSave(ctx);

    expect(ctx.setSaving).toHaveBeenNthCalledWith(1, true);
    expect(ctx.persistProvider).toHaveBeenCalledTimes(1);
    expect(ctx.form.setValue).toHaveBeenCalledWith("api_key", "", {
      shouldDirty: false,
      shouldValidate: false,
    });
    expect(ctx.onOpenChange).toHaveBeenCalledWith(false);
    expect(ctx.setSaving).toHaveBeenLastCalledWith(false);
  });
});
