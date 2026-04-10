import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { RouteTooltipContent } from "../RouteTooltipContent";
import type { RequestLogRouteHop } from "../../../services/gateway/requestLogs";

function makeHop(overrides: Partial<RequestLogRouteHop> = {}): RequestLogRouteHop {
  return {
    provider_id: 1,
    provider_name: "TestProvider",
    ok: true,
    ...overrides,
  };
}

describe("RouteTooltipContent", () => {
  it("returns null for empty hops", () => {
    const { container } = render(<RouteTooltipContent hops={[]} finalStatus={200} />);
    expect(container.innerHTML).toBe("");
  });

  it("renders a successful hop", () => {
    render(<RouteTooltipContent hops={[makeHop()]} finalStatus={200} />);
    expect(screen.getAllByText("TestProvider").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("成功")).toBeInTheDocument();
  });

  it("renders a failed hop with error_code", () => {
    render(
      <RouteTooltipContent
        hops={[makeHop({ ok: false, error_code: "TIMEOUT" })]}
        finalStatus={500}
      />
    );
    expect(screen.getByText("失败")).toBeInTheDocument();
  });

  it("renders a skipped hop", () => {
    render(
      <RouteTooltipContent hops={[makeHop({ ok: false, skipped: true })]} finalStatus={null} />
    );
    expect(screen.getByText("已跳过")).toBeInTheDocument();
    expect(screen.getByText("本次未实际发出请求")).toBeInTheDocument();
  });

  it("renders summary and skippedCount", () => {
    render(
      <RouteTooltipContent
        hops={[makeHop()]}
        finalStatus={200}
        summary="路由概览"
        skippedCount={2}
      />
    );
    expect(screen.getByText("路由概览")).toBeInTheDocument();
    expect(screen.getByText(/跳过 2 个候选/)).toBeInTheDocument();
  });

  it("renders hop with attempts > 1", () => {
    render(<RouteTooltipContent hops={[makeHop({ ok: true, attempts: 3 })]} finalStatus={200} />);
    expect(screen.getByText("成功（重试 3 次）")).toBeInTheDocument();
  });

  it("renders skipped hop with attempts > 1", () => {
    render(
      <RouteTooltipContent
        hops={[makeHop({ ok: false, skipped: true, attempts: 2 })]}
        finalStatus={null}
      />
    );
    expect(screen.getByText("已跳过 2 次")).toBeInTheDocument();
  });

  it("renders provider name as 未知 for empty name", () => {
    render(<RouteTooltipContent hops={[makeHop({ provider_name: "" })]} finalStatus={200} />);
    // Two instances: one in chain view and one in hop row
    expect(screen.getAllByText("未知").length).toBeGreaterThanOrEqual(1);
  });
});
