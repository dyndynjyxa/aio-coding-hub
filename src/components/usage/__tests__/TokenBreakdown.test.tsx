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

    expect(screen.getByText("100K · 15K · 15%")).toBeInTheDocument();
  });
});
