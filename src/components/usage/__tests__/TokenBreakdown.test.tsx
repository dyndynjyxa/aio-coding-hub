import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TokenBreakdown } from "../TokenBreakdown";

describe("components/usage/TokenBreakdown", () => {
  it("renders full mode with input/output breakdown", () => {
    render(<TokenBreakdown totalTokens={18_000} inputTokens={12_000} outputTokens={6_000} />);

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
      <TokenBreakdown totalTokens={5_000} inputTokens={3_000} outputTokens={2_000} />
    );

    expect(screen.getByText("5,000")).toBeInTheDocument();
    expect(container.textContent).not.toContain("含缓存");
  });
});
