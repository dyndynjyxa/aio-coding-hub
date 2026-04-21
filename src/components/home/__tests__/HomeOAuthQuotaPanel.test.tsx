import { fireEvent, render, screen } from "@testing-library/react";
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
    expect(screen.getByText("5h: 61%·2h34m / 7d: 92%·3d2h29m")).toBeInTheDocument();
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
    expect(screen.queryByText("fetch boom")).not.toBeInTheDocument();
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
});
