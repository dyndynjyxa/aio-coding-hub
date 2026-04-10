import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { SortableProviderCard, type SortableProviderCardProps } from "../SortableProviderCard";
import {
  providerOAuthFetchLimits,
  type ProviderSummary,
} from "../../../services/providers/providers";

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));
vi.mock("../../../services/providers/providers", async () => {
  const actual = await vi.importActual<typeof import("../../../services/providers/providers")>(
    "../../../services/providers/providers"
  );
  return { ...actual, providerOAuthFetchLimits: vi.fn() };
});

vi.mock("@dnd-kit/sortable", () => ({
  useSortable: () => ({
    attributes: {},
    listeners: {},
    setNodeRef: () => {},
    transform: null,
    transition: undefined,
    isDragging: false,
  }),
}));

vi.mock("@dnd-kit/utilities", () => ({
  CSS: { Transform: { toString: () => null } },
}));

function makeProvider(partial: Partial<ProviderSummary> = {}): ProviderSummary {
  return {
    id: 1,
    cli_key: "claude",
    name: "Test Provider",
    base_urls: ["https://example.com/v1"],
    base_url_mode: "order",
    claude_models: {},
    enabled: true,
    priority: 0,
    cost_multiplier: 1.0,
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
    auth_mode: "api_key",
    oauth_provider_type: null,
    oauth_email: null,
    oauth_expires_at: null,
    oauth_last_error: null,
    source_provider_id: null,
    bridge_type: null,
    ...partial,
    stream_idle_timeout_seconds: partial.stream_idle_timeout_seconds ?? null,
  };
}

function renderCard(
  partialProvider: Partial<ProviderSummary> = {},
  extraProps: Partial<SortableProviderCardProps> = {}
) {
  const provider = makeProvider(partialProvider);
  const defaultProps: SortableProviderCardProps = {
    provider,
    circuit: null,
    circuitResetting: false,
    onToggleEnabled: vi.fn(),
    onResetCircuit: vi.fn(),
    onEdit: vi.fn(),
    onDelete: vi.fn(),
    ...extraProps,
  };
  return render(<SortableProviderCard {...defaultProps} />);
}

describe("pages/providers/SortableProviderCard", () => {
  it("renders OAuth badge with email", () => {
    renderCard({
      auth_mode: "oauth",
      oauth_email: "user@example.com",
    });

    // OAuth badge is a button; email is rendered in a separate span
    expect(screen.getByText("OAuth")).toBeInTheDocument();
    expect(screen.getByText("user@example.com")).toBeInTheDocument();
  });

  it("renders OAuth badge with error styling", () => {
    renderCard({
      auth_mode: "oauth",
      oauth_last_error: "Token expired",
    });

    const badge = screen.getByText("OAuth");
    expect(badge).toBeInTheDocument();
    expect(badge.getAttribute("title")).toContain("OAuth 错误: Token expired");
  });

  it("renders OAuth badge without email", () => {
    renderCard({
      auth_mode: "oauth",
      oauth_email: null,
    });

    const badge = screen.getByText("OAuth");
    expect(badge.getAttribute("title")).toContain("OAuth 已连接");
  });

  it("renders OAuth button that triggers limits fetch", () => {
    renderCard({
      auth_mode: "oauth",
    });

    // OAuth button renders with "OAuth" text and acts as the fetch trigger
    const oauthButton = screen.getByText("OAuth");
    expect(oauthButton).toBeInTheDocument();
    expect(oauthButton.tagName).toBe("BUTTON");
  });

  it("fetches OAuth limits on button click", async () => {
    vi.mocked(providerOAuthFetchLimits).mockResolvedValueOnce({
      limit_5h_text: "100 requests",
      limit_weekly_text: "1000 requests",
    });

    renderCard({
      auth_mode: "oauth",
    });

    fireEvent.click(screen.getByText("OAuth"));

    await waitFor(() => expect(vi.mocked(providerOAuthFetchLimits)).toHaveBeenCalledWith(1));
  });

  it("polls OAuth limits every 3 minutes", () => {
    const setIntervalSpy = vi.spyOn(globalThis, "setInterval");

    renderCard({
      auth_mode: "oauth",
    });

    expect(setIntervalSpy).toHaveBeenCalledWith(expect.any(Function), 180000);
  });

  it("renders provider-specific short-window labels for Gemini OAuth limits", async () => {
    vi.mocked(providerOAuthFetchLimits).mockResolvedValueOnce({
      limit_short_label: "短窗",
      limit_5h_text: "88",
      limit_weekly_text: "300",
    });

    renderCard({
      id: 77,
      cli_key: "gemini",
      auth_mode: "oauth",
    });

    fireEvent.click(screen.getByText("OAuth"));

    await waitFor(() => expect(screen.getByText("短窗: 88")).toBeInTheDocument());
  });

  it("forces Gemini OAuth limits to render with a generic short-window label", async () => {
    vi.mocked(providerOAuthFetchLimits).mockResolvedValueOnce({
      limit_short_label: "1h",
      limit_5h_text: "88",
      limit_weekly_text: "300",
    });

    renderCard({
      id: 78,
      cli_key: "gemini",
      auth_mode: "oauth",
    });

    fireEvent.click(screen.getByText("OAuth"));

    await waitFor(() => expect(screen.getByText("短窗: 88")).toBeInTheDocument());
  });

  it("handles null result from fetchLimits", async () => {
    vi.mocked(providerOAuthFetchLimits).mockResolvedValueOnce(null);

    renderCard({
      auth_mode: "oauth",
    });

    fireEvent.click(screen.getByText("OAuth"));

    await waitFor(() => expect(vi.mocked(toast)).toHaveBeenCalledWith("获取 OAuth 用量失败"));
  });

  it("handles fetchLimits error", async () => {
    vi.mocked(providerOAuthFetchLimits).mockRejectedValueOnce(new Error("fetch error"));

    renderCard({
      auth_mode: "oauth",
    });

    fireEvent.click(screen.getByText("OAuth"));

    await waitFor(() =>
      expect(vi.mocked(toast)).toHaveBeenCalledWith(expect.stringContaining("获取 OAuth 用量失败"))
    );
  });

  it("renders note when present", () => {
    renderCard({ note: "My custom note" });

    const note = screen.getByTitle("My custom note");
    expect(note).toBeInTheDocument();
    expect(note).toHaveTextContent("My custom note");
  });

  it("renders http links in note as clickable anchors", () => {
    renderCard({ note: "文档 https://example.com/docs?q=1, 备用说明" });

    const link = screen.getByRole("link", { name: "https://example.com/docs?q=1" });
    expect(link).toBeInTheDocument();
    expect(link).toHaveAttribute("href", "https://example.com/docs?q=1");
  });

  it("renders limit chips", () => {
    renderCard({
      limit_5h_usd: 10,
      limit_daily_usd: 100,
      daily_reset_mode: "rolling",
      limit_weekly_usd: 500,
      limit_monthly_usd: 2000,
      limit_total_usd: 10000,
    });

    expect(screen.getByText("限额")).toBeInTheDocument();
  });

  it("renders Claude models badge", () => {
    renderCard({
      cli_key: "claude",
      claude_models: {
        main_model: " claude-3 ",
        reasoning_model: "claude-thinking",
        haiku_model: null as any,
      },
    });

    const badge = screen.getByText("模型映射 2/5");
    expect(badge).toBeInTheDocument();
    expect(badge).toHaveAttribute(
      "title",
      "已配置 Claude 模型映射（2/5）\n主模型: claude-3\n推理模型(Thinking): claude-thinking"
    );
  });

  it("renders tags at the end of the second row", () => {
    renderCard({ tags: ["prod", "cn"] });

    expect(screen.getByText("prod")).toBeInTheDocument();
    expect(screen.getByText("cn")).toBeInTheDocument();
  });

  it("renders 免费 tag with emerald styling", () => {
    renderCard({ tags: ["免费"] });

    const freeTag = screen.getByText("免费");
    expect(freeTag.className).toContain("bg-emerald-100");
    expect(freeTag.className).toContain("text-emerald-700");
  });

  it("keeps cx2cc source label without the top translation badge", () => {
    renderCard(
      {
        source_provider_id: 7,
      },
      {
        sourceProviderName: "Lisa",
      }
    );

    expect(screen.getAllByText((_, el) => el?.textContent === "源: Lisa").length).toBeGreaterThan(
      0
    );
    expect(screen.queryByText("CX2CC 转译")).not.toBeInTheDocument();
    expect(screen.getByText("CX2CC")).toBeInTheDocument();
  });

  it("renders ping mode label", () => {
    renderCard({ base_url_mode: "ping" });

    expect(screen.getByText("Ping")).toBeInTheDocument();
  });

  it("shows api key base url summary only after clicking the api key badge", () => {
    renderCard({ auth_mode: "api_key" });

    expect(screen.queryByText("https://example.com/v1")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "API Key" }));

    expect(screen.getByText("https://example.com/v1")).toBeInTheDocument();
  });

  it("hides base url mode label for oauth providers", () => {
    renderCard({ auth_mode: "oauth", base_url_mode: "ping" });

    expect(screen.queryByText("Ping")).not.toBeInTheDocument();
    expect(screen.queryByText("顺序")).not.toBeInTheDocument();
  });

  it("does not render cost multiplier label when cost multiplier is zero", () => {
    renderCard({ cost_multiplier: 0 });

    expect(screen.queryByText("免费")).not.toBeInTheDocument();
  });

  it("renders circuit breaker state", () => {
    renderCard(
      {},
      {
        circuit: {
          provider_id: 1,
          state: "OPEN",
          open_until: null,
          cooldown_until: null,
        } as any,
      }
    );

    expect(screen.getByTitle("熔断")).toBeInTheDocument();
    expect(screen.getByText("解除熔断")).toBeInTheDocument();
  });

  it("does not render circuit breaker controls for HALF_OPEN probe state", () => {
    renderCard(
      {},
      {
        circuit: {
          provider_id: 1,
          state: "HALF_OPEN",
          open_until: null,
          cooldown_until: null,
        } as any,
      }
    );

    expect(screen.queryByTitle("熔断")).not.toBeInTheDocument();
    expect(screen.queryByText("解除熔断")).not.toBeInTheDocument();
  });

  it("computes unavailableUntil as max of open_until and cooldown_until", () => {
    const futureTs = Math.floor(Date.now() / 1000) + 600;
    renderCard(
      {},
      {
        circuit: {
          provider_id: 1,
          state: "OPEN",
          open_until: futureTs,
          cooldown_until: futureTs + 100,
        } as any,
      }
    );

    // The title should contain the formatted timestamp
    const badge = screen.getByTitle(/熔断至/);
    expect(badge).toBeInTheDocument();
  });

  it("computes unavailableUntil from cooldown_until when not OPEN", () => {
    const futureTs = Math.floor(Date.now() / 1000) + 600;
    renderCard(
      {},
      {
        circuit: {
          provider_id: 1,
          state: "CLOSED",
          open_until: null,
          cooldown_until: futureTs,
        } as any,
      }
    );

    const badge = screen.getByTitle(/熔断至/);
    expect(badge).toBeInTheDocument();
  });

  it("shows terminal launch button when callback provided", () => {
    renderCard(
      {},
      {
        onCopyTerminalLaunchCommand: vi.fn(),
      }
    );

    expect(screen.getByText("终端启动")).toBeInTheDocument();
  });

  it("shows validate model button when callback provided", () => {
    renderCard(
      {},
      {
        onValidateModel: vi.fn(),
      }
    );

    expect(screen.getByText("模型验证")).toBeInTheDocument();
  });

  it("renders limit chips with fixed daily reset", () => {
    renderCard({
      limit_daily_usd: 50,
      daily_reset_mode: "fixed",
      daily_reset_time: "08:00:00",
    });

    expect(screen.getByText("限额")).toBeInTheDocument();
  });
});
