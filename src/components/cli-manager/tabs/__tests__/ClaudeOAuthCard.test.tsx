import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { ClaudeOAuthCard } from "../ClaudeOAuthCard";
import type { ProviderSummary } from "../../../../services/providers/providers";
import {
  providerOAuthDisconnect,
  providerOAuthRefresh,
  providerOAuthStartFlow,
  providerOAuthStatus,
} from "../../../../services/providers/providers";

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { success: vi.fn(), error: vi.fn() }),
}));

vi.mock("../../../../services/providers/providers", async () => {
  const actual = await vi.importActual<typeof import("../../../../services/providers/providers")>(
    "../../../../services/providers/providers"
  );
  return {
    ...actual,
    providerOAuthStartFlow: vi.fn(),
    providerOAuthStatus: vi.fn(),
    providerOAuthRefresh: vi.fn(),
    providerOAuthDisconnect: vi.fn(),
  };
});

function makeProvider(overrides: Partial<ProviderSummary> = {}): ProviderSummary {
  return {
    id: 1,
    cli_key: "claude",
    name: "Claude OAuth Provider",
    base_urls: [],
    base_url_mode: "order",
    claude_models: {},
    enabled: true,
    priority: 0,
    cost_multiplier: 1,
    limit_5h_usd: null,
    limit_daily_usd: null,
    daily_reset_mode: "fixed",
    daily_reset_time: "00:00:00",
    limit_weekly_usd: null,
    limit_monthly_usd: null,
    limit_total_usd: null,
    tags: [],
    note: "",
    created_at: 0,
    updated_at: 0,
    auth_mode: "oauth",
    oauth_provider_type: "claude",
    oauth_email: null,
    oauth_expires_at: null,
    oauth_last_error: null,
    source_provider_id: null,
    bridge_type: null,
    ...overrides,
    stream_idle_timeout_seconds: overrides.stream_idle_timeout_seconds ?? null,
  };
}

function renderCard(providers: ProviderSummary[] | null) {
  return render(
    <MemoryRouter>
      <ClaudeOAuthCard providers={providers} />
    </MemoryRouter>
  );
}

describe("components/cli-manager/tabs/ClaudeOAuthCard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows guidance when no Claude OAuth provider exists", () => {
    renderCard([makeProvider({ id: 2, cli_key: "codex" })]);

    expect(screen.getByText("请先在供应商页面创建一个 Claude OAuth 供应商")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "前往供应商页面" })).toHaveAttribute(
      "href",
      "/providers"
    );
    expect(providerOAuthStatus).not.toHaveBeenCalled();
  });

  it("starts OAuth login when provider exists but is not connected", async () => {
    vi.mocked(providerOAuthStatus)
      .mockResolvedValueOnce({
        connected: false,
        provider_type: "claude",
      } as any)
      .mockResolvedValueOnce({
        connected: true,
        provider_type: "claude",
        email: "user@example.com",
        expires_at: 1_700_000_000,
        has_refresh_token: true,
      } as any);
    vi.mocked(providerOAuthStartFlow).mockResolvedValue({
      success: true,
      provider_type: "claude",
      expires_at: 1_700_000_000,
    } as any);

    renderCard([makeProvider()]);

    const loginButton = await screen.findByRole("button", { name: "登录 Claude" });
    fireEvent.click(loginButton);

    await waitFor(() => expect(providerOAuthStartFlow).toHaveBeenCalledWith("claude", 1));
    await waitFor(() => expect(providerOAuthStatus).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(screen.getByText("user@example.com")).toBeInTheDocument());
    expect(toast.success).toHaveBeenCalled();
  });

  it("shows connected state and supports refresh and disconnect", async () => {
    vi.mocked(providerOAuthStatus)
      .mockResolvedValueOnce({
        connected: true,
        provider_type: "claude",
        email: "user@example.com",
        expires_at: 1_700_000_000,
        has_refresh_token: true,
      } as any)
      .mockResolvedValueOnce({
        connected: true,
        provider_type: "claude",
        email: "user@example.com",
        expires_at: 1_800_000_000,
        has_refresh_token: true,
      } as any);
    vi.mocked(providerOAuthRefresh).mockResolvedValue({
      success: true,
      expires_at: 1_800_000_000,
    } as any);
    vi.mocked(providerOAuthDisconnect).mockResolvedValue({ success: true } as any);

    renderCard([
      makeProvider({ oauth_email: "provider@example.com", oauth_expires_at: 1_700_000_000 }),
    ]);

    expect(await screen.findByText("user@example.com")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "刷新令牌" }));
    await waitFor(() => expect(providerOAuthRefresh).toHaveBeenCalledWith(1));
    expect(toast.success).toHaveBeenCalledWith("令牌已刷新");

    fireEvent.click(screen.getByRole("button", { name: "断开连接" }));
    await waitFor(() => expect(providerOAuthDisconnect).toHaveBeenCalledWith(1));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "登录 Claude" })).toBeInTheDocument()
    );
  });
});
