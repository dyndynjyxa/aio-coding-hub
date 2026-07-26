import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { HomeActiveSessionsCard } from "../HomeActiveSessionsCard";

function session(idx: number, overrides: Partial<any> = {}) {
  return {
    cli_key: idx % 2 === 0 ? "claude" : "codex",
    session_id: `s-${idx}`,
    session_suffix: String(idx).padStart(4, "0"),
    provider_id: 1,
    provider_name: idx === 9 ? "Unknown" : `P${idx}`,
    expires_at: 10_000 + idx * 100,
    request_count: idx + 1,
    total_input_tokens: 100 + idx,
    total_output_tokens: 200 + idx,
    total_cost_usd: 0.000001 * idx,
    total_duration_ms: 1000 + idx,
    ...overrides,
  };
}

describe("components/home/HomeActiveSessionsCard", () => {
  it("renders loading/unavailable/empty states", () => {
    render(
      <HomeActiveSessionsCard
        activeSessions={[]}
        activeSessionsLoading={true}
        activeSessionsAvailable={null}
      />
    );
    expect(screen.getByText("加载中…")).toBeInTheDocument();

    render(
      <HomeActiveSessionsCard
        activeSessions={[]}
        activeSessionsLoading={false}
        activeSessionsAvailable={false}
      />
    );
    expect(screen.getByText("数据不可用")).toBeInTheDocument();

    render(
      <HomeActiveSessionsCard
        activeSessions={[]}
        activeSessionsLoading={false}
        activeSessionsAvailable={true}
      />
    );
    expect(screen.getByText("暂无活跃 Session。")).toBeInTheDocument();
  });

  it("renders full list and shows provider fallback", () => {
    const sessions = Array.from({ length: 10 }, (_, idx) => session(idx));
    render(
      <HomeActiveSessionsCard
        activeSessions={sessions as any}
        activeSessionsLoading={false}
        activeSessionsAvailable={true}
      />
    );

    expect(screen.getByText("活跃 Session")).toBeInTheDocument();
    expect(screen.queryByText("+2 个")).not.toBeInTheDocument();
    expect(screen.getByText("0000")).toBeInTheDocument();
    const cost = screen.getByText("$0.000000");
    expect(cost).toBeInTheDocument();
    expect(cost.previousElementSibling).toBeNull();
    expect(screen.getAllByText("未知").length).toBeGreaterThan(0);
  });
});
