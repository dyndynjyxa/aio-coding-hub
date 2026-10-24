import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { OAuthLimitsResult } from "../../../services/providers/providers";
import { OAuthQuotaUsageInline } from "../OAuthQuotaUsageInline";

const now = 1_800_000_000;
const estimate = {
  used_usd: 20,
  estimated_total_usd: 100,
  estimated_remaining_usd: 80,
  window_start: now - 3600,
  window_end: now + 4 * 3600,
  sample_count: 3,
  unpriced_count: 0,
};

function limits(overrides: Partial<OAuthLimitsResult> = {}): OAuthLimitsResult {
  return {
    limit_short_label: "5h",
    limit_5h_text: "80%",
    limit_weekly_text: null,
    limit_5h_reset_at: estimate.window_end,
    limit_weekly_reset_at: null,
    reset_credit_available_count: 1,
    credits: { has_credits: false, unlimited: false, balance: "0" },
    limit_5h_remaining_percent: 80,
    limit_weekly_remaining_percent: null,
    routing_limited: false,
    quota_estimates: { short: estimate, weekly: null },
    ...overrides,
  };
}

describe("OAuthQuotaUsageInline quota estimates", () => {
  it("shows total and remaining estimates beside the existing credits and reset action", () => {
    const onReset = vi.fn();
    render(
      <OAuthQuotaUsageInline
        cliKey="codex"
        limits={limits()}
        nowUnix={now}
        onResetCreditClick={onReset}
      />
    );
    expect(screen.getByText("预估额度(5h): ~$100.00(剩余 ~$80.00)")).toHaveAttribute(
      "title",
      expect.stringContaining("本地已用: $20.00，样本: 3 次")
    );
    expect(screen.getByText("可用点数: 0")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "可重置次数: 1(点击重置)" }));
    expect(onReset).toHaveBeenCalledOnce();
  });

  it("uses the actual window duration including a weekly primary window", () => {
    render(
      <OAuthQuotaUsageInline
        cliKey="codex"
        nowUnix={now}
        limits={limits({
          limit_weekly_text: "80%",
          limit_weekly_reset_at: now + 3600,
          limit_window_seconds: [604800, 7200],
          quota_estimates: {
            short: { ...estimate, window_end: estimate.window_start + 7 * 24 * 3600 },
            weekly: { ...estimate, window_end: estimate.window_start + 2 * 3600 },
          },
        })}
      />
    );
    expect(screen.getByText("预估额度(周): ~$100.00(剩余 ~$80.00)")).toBeInTheDocument();
    expect(screen.getByText("预估额度(2h): ~$100.00(剩余 ~$80.00)")).toBeInTheDocument();
    expect(screen.getByText("2h: 80%(重置时间: 1h)")).toBeInTheDocument();
    expect(screen.getByText(/^周: 80%/)).toBeInTheDocument();
  });

  it("keeps upstream window labels when cost estimates are unavailable", () => {
    render(
      <OAuthQuotaUsageInline
        cliKey="codex"
        nowUnix={now}
        limits={limits({
          limit_weekly_text: "80%",
          limit_weekly_reset_at: now + 3600,
          limit_window_seconds: [604800, 7200],
          quota_estimates: null,
        })}
      />
    );
    expect(screen.getByText(/^周: 80%/)).toBeInTheDocument();
    expect(screen.getByText("2h: 80%(重置时间: 1h)")).toBeInTheDocument();
    expect(screen.queryByText(/预估额度/)).not.toBeInTheDocument();
  });

  it("keeps a partial-cost estimate visible and explains missing costs on refresh", () => {
    const { rerender } = render(
      <OAuthQuotaUsageInline cliKey="codex" nowUnix={now} limits={limits()} />
    );
    const initial = screen.getByText("预估额度(5h): ~$100.00(剩余 ~$80.00)");
    expect(initial.getAttribute("title")).not.toContain("请求缺少费用");

    rerender(
      <OAuthQuotaUsageInline
        cliKey="codex"
        nowUnix={now}
        limits={limits({
          quota_estimates: {
            short: { ...estimate, sample_count: 4, unpriced_count: 1 },
            weekly: null,
          },
        })}
      />
    );
    expect(screen.getByText("预估额度(5h): ~$100.00(剩余 ~$80.00)")).toHaveAttribute(
      "title",
      expect.stringContaining("其中 1 次请求缺少费用，仅按已记录费用估算，结果可能偏低。")
    );
    expect(screen.queryByText(/费用数据不足/)).not.toBeInTheDocument();

    rerender(
      <OAuthQuotaUsageInline
        cliKey="codex"
        nowUnix={now}
        limits={limits({
          quota_estimates: {
            short: {
              ...estimate,
              unpriced_count: 1,
              window_start: now - 5 * 3600,
              window_end: now,
            },
            weekly: null,
          },
        })}
      />
    );
    expect(screen.getByText("预估额度(5h): 已重置，待刷新")).toBeInTheDocument();
    expect(screen.queryByText(/预估额度.*\$100/)).not.toBeInTheDocument();
  });

  it("renders Claude estimates and preserves compatibility with older quota results", () => {
    const { rerender } = render(
      <OAuthQuotaUsageInline cliKey="claude" nowUnix={now} limits={limits()} />
    );
    expect(screen.getByText(/预估额度\(5h\)/)).toBeInTheDocument();
    expect(screen.queryByText(/可用点数/)).not.toBeInTheDocument();
    rerender(
      <OAuthQuotaUsageInline
        cliKey="codex"
        nowUnix={now}
        limits={limits({ quota_estimates: undefined })}
      />
    );
    expect(screen.queryByText(/预估额度/)).not.toBeInTheDocument();
    expect(screen.getByText("可用点数: 0")).toBeInTheDocument();
    rerender(<OAuthQuotaUsageInline cliKey="gemini" nowUnix={now} limits={limits()} />);
    expect(screen.queryByText(/预估额度/)).not.toBeInTheDocument();
    rerender(<OAuthQuotaUsageInline cliKey="codex" nowUnix={now} limits={null} />);
    expect(screen.queryByText(/预估额度/)).not.toBeInTheDocument();
  });

  it.each([
    [
      {
        ...estimate,
        used_usd: null,
        estimated_total_usd: null,
        estimated_remaining_usd: null,
        sample_count: 0,
      },
      "待积累用量",
    ],
    [
      {
        ...estimate,
        used_usd: null,
        estimated_total_usd: null,
        estimated_remaining_usd: null,
        unpriced_count: 1,
      },
      "费用数据不足",
    ],
    [
      {
        ...estimate,
        used_usd: 0,
        estimated_total_usd: null,
        estimated_remaining_usd: null,
        unpriced_count: 1,
      },
      "待积累用量",
    ],
    [{ ...estimate, window_end: now }, "已重置，待刷新"],
    [{ ...estimate, estimated_total_usd: Number.NaN }, "待积累用量"],
    [{ ...estimate, estimated_total_usd: Number.POSITIVE_INFINITY }, "待积累用量"],
    [{ ...estimate, estimated_remaining_usd: -1 }, "待积累用量"],
  ])("keeps unavailable and expired estimates explicit", (sample, message) => {
    render(
      <OAuthQuotaUsageInline
        cliKey="codex"
        nowUnix={now}
        limits={limits({
          quota_estimates: { short: sample, weekly: null },
        })}
      />
    );
    expect(screen.getByText(new RegExp(`预估额度.*${message}`))).toBeInTheDocument();
    expect(screen.queryByText(/预估额度.*\$100/)).not.toBeInTheDocument();
  });

  it("distinguishes exhausted quota from a small positive dollar estimate", () => {
    const { rerender } = render(
      <OAuthQuotaUsageInline
        cliKey="codex"
        nowUnix={now}
        limits={limits({
          quota_estimates: { short: { ...estimate, estimated_remaining_usd: 0 }, weekly: null },
        })}
      />
    );
    expect(screen.getByText("预估额度(5h): ~$100.00(剩余 ~$0.00)")).toBeInTheDocument();
    rerender(
      <OAuthQuotaUsageInline
        cliKey="codex"
        nowUnix={now}
        limits={limits({
          quota_estimates: {
            short: { ...estimate, estimated_total_usd: 0.005, estimated_remaining_usd: 0.001 },
            weekly: null,
          },
        })}
      />
    );
    expect(screen.getByText("预估额度(5h): ~<$0.01(剩余 ~<$0.01)")).toBeInTheDocument();
  });
});
