import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HomeOAuthQuotaRow } from "../homeOAuthQuotaTypes";
import { HomeOAuthQuotaPanelContent } from "../HomeOAuthQuotaPanel";

function makeRow(partial: Partial<HomeOAuthQuotaRow>): HomeOAuthQuotaRow {
  const nowUnix = Math.floor(Date.now() / 1000);
  return {
    providerId: 1,
    cliKey: "codex",
    providerName: "TG 合租账号",
    enabled: true,
    state: "success",
    limits: {
      limit_short_label: "5h",
      limit_5h_text: "61%",
      limit_weekly_text: "92%",
      limit_5h_reset_at: nowUnix + 2 * 3600 + 34 * 60,
      limit_weekly_reset_at: nowUnix + 3 * 86400 + 2 * 3600 + 29 * 60,
      reset_credit_available_count: 4,
    },
    error: null,
    ...partial,
  };
}

describe("components/home/HomeOAuthQuotaPanel", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-04-21T12:00:00Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("lists provider cards and shows a refresh notice when nothing has been fetched", () => {
    render(
      <HomeOAuthQuotaPanelContent
        rows={[makeRow({ state: "idle", limits: null })]}
        hasProviders={true}
        hasRefreshed={false}
        refreshing={false}
        onRefresh={vi.fn()}
        onRefreshRow={vi.fn()}
      />
    );

    expect(screen.getByText("TG 合租账号")).toBeInTheDocument();
    expect(screen.getByText("点击右上角刷新获取 OAuth 配额")).toBeInTheDocument();
  });

  it("renders cached quota data with compact summary text", () => {
    render(
      <HomeOAuthQuotaPanelContent
        rows={[makeRow({ state: "success" })]}
        hasProviders={true}
        hasRefreshed={false}
        refreshing={false}
        onRefresh={vi.fn()}
        onRefreshRow={vi.fn()}
      />
    );

    expect(screen.getByText("TG 合租账号")).toBeInTheDocument();
    expect(screen.queryByText("OAuth")).not.toBeInTheDocument();
    expect(screen.getByText("5h: 61%(重置时间: 2h 34m)")).toBeInTheDocument();
    expect(screen.getByText("周: 92%(重置时间: 3d 2h 29m)")).toBeInTheDocument();
    expect(screen.getByText("可重置次数: 4")).toBeInTheDocument();
  });

  it("shows insufficient quota when either quota window is exhausted", () => {
    const { rerender } = render(
      <HomeOAuthQuotaPanelContent
        rows={[
          makeRow({
            limits: {
              limit_short_label: "5h",
              limit_5h_text: "61%",
              limit_weekly_text: "0%",
              limit_5h_reset_at: null,
              limit_weekly_reset_at: null,
              reset_credit_available_count: 1,
            },
          }),
        ]}
        hasProviders={true}
        hasRefreshed={true}
        refreshing={false}
        onRefresh={vi.fn()}
        onRefreshRow={vi.fn()}
      />
    );

    expect(screen.getByText("配额不足")).toBeInTheDocument();

    rerender(
      <HomeOAuthQuotaPanelContent
        rows={[
          makeRow({
            limits: {
              limit_short_label: "5h",
              limit_5h_text: "0%",
              limit_weekly_text: "92%",
              limit_5h_reset_at: null,
              limit_weekly_reset_at: null,
              reset_credit_available_count: 1,
            },
          }),
        ]}
        hasProviders={true}
        hasRefreshed={true}
        refreshing={false}
        onRefresh={vi.fn()}
        onRefreshRow={vi.fn()}
      />
    );

    expect(screen.getByText("配额不足")).toBeInTheDocument();
  });

  it("does not show insufficient quota for non-exhausted percentages", () => {
    render(
      <HomeOAuthQuotaPanelContent
        rows={[
          makeRow({
            limits: {
              limit_short_label: "5h",
              limit_5h_text: "1%",
              limit_weekly_text: "1%",
              limit_5h_reset_at: null,
              limit_weekly_reset_at: null,
              reset_credit_available_count: 1,
            },
          }),
        ]}
        hasProviders={true}
        hasRefreshed={true}
        refreshing={false}
        onRefresh={vi.fn()}
        onRefreshRow={vi.fn()}
      />
    );

    expect(screen.queryByText("配额不足")).not.toBeInTheDocument();
  });

  it("shows insufficient quota for exhausted numeric Gemini quota", () => {
    render(
      <HomeOAuthQuotaPanelContent
        rows={[
          makeRow({
            cliKey: "gemini",
            limits: {
              limit_short_label: "短窗",
              limit_5h_text: "0",
              limit_weekly_text: "3",
              limit_5h_reset_at: null,
              limit_weekly_reset_at: null,
              reset_credit_available_count: 1,
            },
          }),
        ]}
        hasProviders={true}
        hasRefreshed={true}
        refreshing={false}
        onRefresh={vi.fn()}
        onRefreshRow={vi.fn()}
      />
    );

    expect(screen.getByText("配额不足")).toBeInTheDocument();
  });

  it("renders loading, empty, and error card states", () => {
    const { rerender } = render(
      <HomeOAuthQuotaPanelContent
        rows={[makeRow({ state: "loading", limits: null })]}
        hasProviders={true}
        hasRefreshed={true}
        refreshing={true}
        onRefresh={vi.fn()}
        onRefreshRow={vi.fn()}
      />
    );

    expect(screen.getByText("刷新中...")).toBeInTheDocument();
    expect(screen.queryByText("配额不足")).not.toBeInTheDocument();

    rerender(
      <HomeOAuthQuotaPanelContent
        rows={[
          makeRow({
            state: "success",
            limits: {
              limit_short_label: "5h",
              limit_5h_text: null,
              limit_weekly_text: null,
              limit_5h_reset_at: null,
              limit_weekly_reset_at: null,
              reset_credit_available_count: null,
            },
          }),
        ]}
        hasProviders={true}
        hasRefreshed={true}
        refreshing={false}
        onRefresh={vi.fn()}
        onRefreshRow={vi.fn()}
      />
    );

    expect(screen.getByText("暂无 OAuth 配额信息")).toBeInTheDocument();
    expect(screen.queryByText("配额不足")).not.toBeInTheDocument();

    rerender(
      <HomeOAuthQuotaPanelContent
        rows={[makeRow({ state: "error", limits: null, error: "fetch boom" })]}
        hasProviders={true}
        hasRefreshed={true}
        refreshing={false}
        onRefresh={vi.fn()}
        onRefreshRow={vi.fn()}
      />
    );

    expect(screen.getByText("刷新失败")).toBeInTheDocument();
    expect(screen.getByText("刷新失败，请重试")).toBeInTheDocument();
    expect(
      within(screen.getByTestId("oauth-quota-status-1")).getByText("刷新失败")
    ).toBeInTheDocument();
    expect(screen.queryByText("fetch boom")).not.toBeInTheDocument();
    expect(screen.queryByText("配额不足")).not.toBeInTheDocument();
  });

  it("forwards bulk and row refresh actions", () => {
    const onRefresh = vi.fn();
    const onRefreshRow = vi.fn();

    render(
      <HomeOAuthQuotaPanelContent
        rows={[makeRow({})]}
        hasProviders={true}
        hasRefreshed={true}
        refreshing={false}
        onRefresh={onRefresh}
        onRefreshRow={onRefreshRow}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: "刷新 OAuth 配额" }));
    expect(onRefresh).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "刷新 TG 合租账号 OAuth 配额" }));
    expect(onRefreshRow).toHaveBeenCalledWith(1);
  });

  it("confirms before forwarding row reset actions", async () => {
    const onResetRow = vi.fn().mockResolvedValue(undefined);

    render(
      <HomeOAuthQuotaPanelContent
        rows={[makeRow({ providerId: 31, providerName: "Codex A" })]}
        hasProviders={true}
        hasRefreshed={true}
        refreshing={false}
        onRefresh={vi.fn()}
        onRefreshRow={vi.fn()}
        onResetRow={onResetRow}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: "可重置次数: 4(点击重置)" }));
    expect(screen.getByText("使用 1 次 Codex 重置次数刷新该账号额度？")).toBeInTheDocument();
    expect(onResetRow).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "确认重置" }));

    expect(onResetRow).toHaveBeenCalledWith(31);
  });

  it("does not render reset action for non-Codex rows or unknown counts", () => {
    render(
      <HomeOAuthQuotaPanelContent
        rows={[
          makeRow({
            cliKey: "gemini",
            limits: { ...makeRow({}).limits!, reset_credit_available_count: 4 },
          }),
          makeRow({
            providerId: 2,
            providerName: "Codex B",
            limits: { ...makeRow({}).limits!, reset_credit_available_count: null },
          }),
        ]}
        hasProviders={true}
        hasRefreshed={true}
        refreshing={false}
        onRefresh={vi.fn()}
        onRefreshRow={vi.fn()}
        onResetRow={vi.fn()}
      />
    );

    expect(screen.queryByText(/可重置次数/)).not.toBeInTheDocument();
  });
});
