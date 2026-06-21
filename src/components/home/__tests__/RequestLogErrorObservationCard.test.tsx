import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { GatewayErrorDescriptions } from "../../../constants/gatewayErrorCodes";
import { RequestLogErrorObservationCard } from "../RequestLogErrorObservationCard";
import type { RequestLogErrorObservation } from "../requestLogErrorDetails";

const baseObservation: RequestLogErrorObservation = {
  attemptDurationMs: null,
  circuitFailureCount: null,
  circuitFailureThreshold: null,
  circuitStateAfter: null,
  circuitStateBefore: null,
  decision: null,
  displayErrorCode: null,
  errorCategory: null,
  gatewayErrorCode: null,
  gwDescription: null,
  matchedRule: null,
  outcome: null,
  providerId: null,
  providerIndex: null,
  providerName: null,
  rawDetailsText: null,
  reason: null,
  reasonCode: null,
  retryIndex: null,
  selectionMethod: null,
  source: "summary",
  upstreamBodyPreview: null,
  upstreamStatus: null,
};

describe("components/home/RequestLogErrorObservationCard", () => {
  it("renders nothing without an observation", () => {
    const { container } = render(<RequestLogErrorObservationCard observation={null} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("renders description, suggestion, detail fields, and truncated upstream preview", () => {
    render(
      <RequestLogErrorObservationCard
        observation={{
          ...baseObservation,
          circuitFailureCount: 3,
          circuitFailureThreshold: 5,
          circuitStateAfter: "OPEN",
          circuitStateBefore: "CLOSED",
          decision: "skip",
          displayErrorCode: "GW_PROVIDER_CIRCUIT_OPEN",
          errorCategory: "upstream",
          gatewayErrorCode: "GW_PROVIDER_CIRCUIT_OPEN",
          gwDescription: GatewayErrorDescriptions.GW_PROVIDER_CIRCUIT_OPEN,
          matchedRule: "provider_circuit",
          providerId: 9,
          providerName: "Provider B",
          reason: "熔断器打开",
          reasonCode: "circuit_open",
          selectionMethod: "sort_mode",
          upstreamBodyPreview: "x".repeat(501),
          upstreamStatus: 503,
        }}
      />
    );

    expect(screen.getByText("供应商熔断 (GW_PROVIDER_CIRCUIT_OPEN)")).toBeInTheDocument();
    expect(screen.getByText("Provider 已熔断")).toBeInTheDocument();
    expect(screen.getByText(/该 Provider 因连续失败已被熔断/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "详细信息" }));

    expect(screen.getByText("错误分类:")).toBeInTheDocument();
    expect(screen.getByText("上游状态码:")).toBeInTheDocument();
    expect(screen.getByText("决策:")).toBeInTheDocument();
    expect(screen.getByText("选择方式:")).toBeInTheDocument();
    expect(screen.getByText("原因码:")).toBeInTheDocument();
    expect(screen.getByText("匹配规则:")).toBeInTheDocument();
    expect(screen.getByText("原因:")).toBeInTheDocument();
    expect(screen.getByText("熔断器:")).toBeInTheDocument();
    expect(screen.getByText("供应商:")).toBeInTheDocument();
    expect(screen.getByText(/CLOSED → OPEN \(3\/5\)/)).toBeInTheDocument();
    expect(screen.getByText("Provider B (id=9)")).toBeInTheDocument();
    expect(screen.getByText(/x{500}…/)).toBeInTheDocument();
  });

  it("renders reason as primary text and raw details when no gateway description exists", () => {
    render(
      <RequestLogErrorObservationCard
        observation={{
          ...baseObservation,
          circuitStateAfter: "OPEN",
          displayErrorCode: "CUSTOM_ERROR",
          providerId: 12,
          rawDetailsText: "raw error details",
          reason: "custom reason",
        }}
      />
    );

    expect(screen.getByText("CUSTOM_ERROR")).toBeInTheDocument();
    expect(screen.getByText("custom reason")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "详细信息" }));

    expect(screen.getByText("→ OPEN")).toBeInTheDocument();
    expect(screen.getByText("id=12")).toBeInTheDocument();
    expect(screen.getByText("原始错误信息")).toBeInTheDocument();
    expect(screen.getByText("raw error details")).toBeInTheDocument();
  });

  it("renders a fallback title when only an upstream status is available", () => {
    render(
      <RequestLogErrorObservationCard
        observation={{
          ...baseObservation,
          upstreamStatus: 502,
        }}
      />
    );

    expect(screen.getByText("HTTP 502 响应异常")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "详细信息" }));

    expect(screen.getByText("上游状态码:")).toBeInTheDocument();
    expect(screen.getByText("502")).toBeInTheDocument();
  });
});
