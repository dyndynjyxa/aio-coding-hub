import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TokenBreakdown } from "../TokenBreakdown";

describe("components/usage/TokenBreakdown", () => {
  it("renders compact cache breakdown as total/cache/ratio", () => {
    render(
      <TokenBreakdown
        totalTokens={85_000}
        inputTokens={60_000}
        outputTokens={25_000}
        totalTokensWithCache={100_000}
        displayMode="compactRatio"
        useCompactUnits={true}
      />
    );

    expect(screen.getByText("100K / 15K / 15%")).toBeInTheDocument();
  });

  it("renders full mode with input/output breakdown", () => {
    render(
      <TokenBreakdown
        totalTokens={18_000}
        inputTokens={12_000}
        outputTokens={6_000}
      />
    );

    expect(screen.getByText("18,000")).toBeInTheDocument();
    expect(screen.getByText("12,000")).toBeInTheDocument();
    expect(screen.getByText("6,000")).toBeInTheDocument();
  });

  it("renders full mode with cache line when totalTokensWithCache is provided", () => {
    render(
      <TokenBreakdown
        totalTokens={18_000}
        inputTokens={12_000}
        outputTokens={6_000}
        totalTokensWithCache={22_500}
      />
    );

    expect(screen.getByText("18,000")).toBeInTheDocument();
    expect(screen.getByText("22,500")).toBeInTheDocument();
  });

  it("hides cache line when totalTokensWithCache is undefined", () => {
    const { container } = render(
      <TokenBreakdown
        totalTokens={5_000}
        inputTokens={3_000}
        outputTokens={2_000}
      />
    );

    expect(screen.getByText("5,000")).toBeInTheDocument();
    expect(container.textContent).not.toContain("含缓存");
  });

  it("shows dash for cache in compactRatio when no cache data", () => {
    render(
      <TokenBreakdown
        totalTokens={10_000}
        inputTokens={7_000}
        outputTokens={3_000}
        displayMode="compactRatio"
        useCompactUnits={true}
      />
    );

    expect(screen.getByText("10K / — / —")).toBeInTheDocument();
  });

  it("handles zero tokens in compactRatio mode", () => {
    render(
      <TokenBreakdown
        totalTokens={0}
        inputTokens={0}
        outputTokens={0}
        totalTokensWithCache={0}
        displayMode="compactRatio"
        useCompactUnits={true}
      />
    );

    expect(screen.getByText("0 / — / —")).toBeInTheDocument();
  });
});
