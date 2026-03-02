import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { ProviderEditorDialog } from "../ProviderEditorDialog";
import { providerUpsert, type ProviderSummary } from "../../../services/providers";

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));

vi.mock("../../../services/providers", async () => {
  const actual = await vi.importActual<typeof import("../../../services/providers")>(
    "../../../services/providers"
  );
  return { ...actual, providerUpsert: vi.fn(), baseUrlPingMs: vi.fn() };
});

function makeProvider(partial: Partial<ProviderSummary> = {}): ProviderSummary {
  return {
    id: 1,
    cli_key: "claude",
    name: "Existing",
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
    auth_type: "api_key",
    oauth_provider_type: null,
    oauth_expires_at: null,
    oauth_last_error: null,
    oauth_quota_exceeded: false,
    oauth_quota_recover_at: null,
    created_at: 0,
    updated_at: 0,
    ...partial,
  };
}

describe("pages/providers/ProviderEditorDialog", () => {
  it("validates create form and saves provider", async () => {
    vi.mocked(providerUpsert).mockResolvedValue({
      id: 1,
      cli_key: "claude",
      name: "My Provider",
      base_urls: ["https://example.com/v1"],
      base_url_mode: "order",
      enabled: true,
      cost_multiplier: 1.0,
      claude_models: {},
    } as any);

    const onSaved = vi.fn();
    const onOpenChange = vi.fn();

    render(
      <ProviderEditorDialog
        mode="create"
        open={true}
        cliKey="claude"
        onSaved={onSaved}
        onOpenChange={onOpenChange}
      />
    );

    const dialog = within(screen.getByRole("dialog"));

    fireEvent.click(dialog.getByRole("button", { name: "保存" }));
    expect(vi.mocked(toast)).toHaveBeenCalledWith("名称不能为空");

    fireEvent.change(dialog.getByPlaceholderText("default"), { target: { value: "My Provider" } });
    fireEvent.click(dialog.getByRole("button", { name: "保存" }));
    expect(vi.mocked(toast)).toHaveBeenCalledWith("API Key 不能为空（新增 Provider 必填）");

    fireEvent.change(dialog.getByPlaceholderText("sk-…"), { target: { value: "sk-test" } });
    fireEvent.change(dialog.getByPlaceholderText("1.0"), { target: { value: "0" } });
    fireEvent.click(dialog.getByRole("button", { name: "保存" }));
    expect(vi.mocked(toast)).toHaveBeenCalledWith("价格倍率必须大于 0");

    fireEvent.change(dialog.getByPlaceholderText("1.0"), { target: { value: "1.0" } });
    fireEvent.change(dialog.getByPlaceholderText(/中转 endpoint/), {
      target: { value: "ftp://x" },
    });
    fireEvent.click(dialog.getByRole("button", { name: "保存" }));
    expect(vi.mocked(toast)).toHaveBeenCalledWith(
      expect.stringContaining("Base URL 协议必须是 http/https")
    );

    fireEvent.change(dialog.getByPlaceholderText(/中转 endpoint/), {
      target: { value: "https://example.com/v1" },
    });

    fireEvent.click(dialog.getByText("Claude 模型映射"));
    fireEvent.change(dialog.getByPlaceholderText(/minimax-text-01/), {
      target: { value: "x".repeat(201) },
    });
    fireEvent.click(dialog.getByRole("button", { name: "保存" }));
    expect(vi.mocked(toast)).toHaveBeenCalledWith(expect.stringContaining("主模型 过长"));

    fireEvent.change(dialog.getByPlaceholderText(/minimax-text-01/), { target: { value: "ok" } });
    fireEvent.click(dialog.getByRole("button", { name: "保存" }));

    await waitFor(() =>
      expect(vi.mocked(providerUpsert)).toHaveBeenCalledWith(
        expect.objectContaining({
          cli_key: "claude",
          name: "My Provider",
          base_urls: ["https://example.com/v1"],
          base_url_mode: "order",
          api_key: "sk-test",
          enabled: true,
          cost_multiplier: 1.0,
        })
      )
    );

    await waitFor(() => expect(onSaved).toHaveBeenCalledWith("claude"));
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false));
  });

  it("toasts when provider upsert is unavailable (returns null)", async () => {
    vi.mocked(providerUpsert).mockResolvedValue(null as any);

    const onSaved = vi.fn();
    const onOpenChange = vi.fn();

    render(
      <ProviderEditorDialog
        mode="create"
        open={true}
        cliKey="codex"
        onSaved={onSaved}
        onOpenChange={onOpenChange}
      />
    );

    const dialog = within(screen.getByRole("dialog"));

    fireEvent.change(dialog.getByPlaceholderText("default"), { target: { value: "My Provider" } });
    fireEvent.change(dialog.getByPlaceholderText("sk-…"), { target: { value: "sk-test" } });
    fireEvent.change(dialog.getByPlaceholderText(/中转 endpoint/), {
      target: { value: "https://example.com/v1" },
    });

    fireEvent.click(dialog.getByRole("button", { name: "保存" }));

    await waitFor(() =>
      expect(vi.mocked(toast)).toHaveBeenCalledWith("仅在 Tauri Desktop 环境可用")
    );
    expect(onSaved).not.toHaveBeenCalled();
    expect(onOpenChange).not.toHaveBeenCalled();
  });

  it("supports edit mode, drives UI handlers, and blocks close while saving", async () => {
    let resolveUpsert: (value: any) => void;
    const upsertPromise = new Promise((resolve) => {
      resolveUpsert = resolve as (value: any) => void;
    });
    vi.mocked(providerUpsert).mockReturnValue(upsertPromise as any);

    const onSaved = vi.fn();
    const onOpenChange = vi.fn();
    const provider = makeProvider();

    render(
      <ProviderEditorDialog
        mode="edit"
        open={true}
        provider={provider}
        onSaved={onSaved}
        onOpenChange={onOpenChange}
      />
    );

    const dialogEl = screen.getByRole("dialog");
    const dialog = within(dialogEl);

    // Toggle base url mode (covers BaseUrlModeRadioGroup button handlers)
    fireEvent.click(dialog.getByRole("radio", { name: "Ping" }));
    fireEvent.click(dialog.getByRole("radio", { name: "顺序" }));

    // Open limits details and toggle daily reset modes (covers DailyResetModeRadioGroup handlers)
    fireEvent.click(dialog.getByText("限流配置"));
    fireEvent.click(dialog.getByRole("radio", { name: "滚动窗口 (24h)" }));

    const timeInput = dialogEl.querySelector('input[type="time"]') as HTMLInputElement | null;
    expect(timeInput).not.toBeNull();
    expect(timeInput!).toBeDisabled();

    fireEvent.click(dialog.getByRole("radio", { name: "固定时间" }));
    expect(timeInput!).toBeEnabled();

    // Drive limit card onChange handlers
    fireEvent.change(dialog.getByPlaceholderText("例如: 10"), { target: { value: "1" } });
    fireEvent.change(dialog.getByPlaceholderText("例如: 100"), { target: { value: "2" } });
    fireEvent.change(dialog.getByPlaceholderText("例如: 500"), { target: { value: "3" } });
    fireEvent.change(dialog.getByPlaceholderText("例如: 2000"), { target: { value: "4" } });
    fireEvent.change(dialog.getByPlaceholderText("例如: 1000"), { target: { value: "2" } });

    // Toggle enabled switch (covers Switch onCheckedChange handler)
    fireEvent.click(dialog.getByRole("switch"));

    // Drive Claude models onChange handlers
    fireEvent.click(dialog.getByText("Claude 模型映射"));
    fireEvent.change(dialog.getByPlaceholderText(/minimax-text-01/), { target: { value: "m" } });
    fireEvent.change(dialog.getByPlaceholderText(/kimi-k2-thinking/), { target: { value: "r" } });
    fireEvent.change(dialog.getByPlaceholderText(/glm-4-plus-haiku/), { target: { value: "h" } });
    fireEvent.change(dialog.getByPlaceholderText(/glm-4-plus-sonnet/), { target: { value: "s" } });
    fireEvent.change(dialog.getByPlaceholderText(/glm-4-plus-opus/), { target: { value: "o" } });

    // Start saving and block close while saving
    fireEvent.click(dialog.getByRole("button", { name: "保存" }));
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onOpenChange).not.toHaveBeenCalled();

    resolveUpsert!(provider);

    await waitFor(() =>
      expect(vi.mocked(providerUpsert)).toHaveBeenCalledWith(
        expect.objectContaining({
          provider_id: 1,
          cli_key: "claude",
          base_url_mode: "order",
        })
      )
    );

    await waitFor(() => expect(onSaved).toHaveBeenCalledWith("claude"));
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false));
  });
});
