import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ProviderChainView } from "../ProviderChainView";

describe("components/ProviderChainView", () => {
  it("renders loading + empty + merged views", () => {
    const { rerender } = render(
      <ProviderChainView attemptLogs={[]} attemptLogsLoading attemptsJson={null} />
    );
    expect(screen.getByText("加载中…")).toBeInTheDocument();

    rerender(<ProviderChainView attemptLogs={[]} attemptLogsLoading={false} attemptsJson={null} />);
    expect(screen.getByText("无故障切换尝试。")).toBeInTheDocument();

    rerender(
      <ProviderChainView
        attemptLogs={[
          {
            attempt_index: 1,
            provider_id: 1,
            provider_name: "P1",
            base_url: "https://p1",
            outcome: "failed",
            status: 500,
          },
          {
            attempt_index: 2,
            provider_id: 2,
            provider_name: "P2",
            base_url: "https://p2",
            outcome: "success",
            status: 200,
          },
        ]}
        attemptLogsLoading={false}
        attemptsJson={"not-json"}
      />
    );
    expect(screen.getByText("尝试 JSON 解析失败")).toBeInTheDocument();
    expect(screen.getByText("起始供应商：")).toBeInTheDocument();
    expect(screen.getByText("最终供应商：")).toBeInTheDocument();

    rerender(
      <ProviderChainView
        attemptLogs={[]}
        attemptLogsLoading={false}
        attemptsJson={JSON.stringify([
          {
            provider_id: 1,
            provider_name: "P1",
            base_url: "https://p1",
            outcome: "success",
            status: 200,
            provider_index: 0,
            retry_index: 0,
          },
        ])}
      />
    );
    expect(screen.getByText("数据源：request_logs.attempts_json")).toBeInTheDocument();
    expect(screen.getByText("请求成功")).toBeInTheDocument();

    rerender(
      <ProviderChainView
        attemptLogs={[
          {
            attempt_index: 1,
            provider_id: 99,
            provider_name: "未知",
            base_url: "",
            outcome: "failed",
            status: null,
            attempt_started_ms: 10,
            attempt_duration_ms: 50,
          },
        ]}
        attemptLogsLoading={false}
        attemptsJson={JSON.stringify([
          {
            provider_id: 99,
            provider_name: "未知",
            base_url: "https://p99",
            outcome: "failed",
            status: 400,
            provider_index: 1,
            retry_index: 2,
            error_code: "E",
            decision: "skip",
            reason: "because",
          },
        ])}
      />
    );
    expect(screen.getByText("数据源：request_logs.attempts_json（结构化）")).toBeInTheDocument();
    expect(screen.getAllByText("未知（id=99）").length).toBeGreaterThan(0);
    expect(screen.getByText("请求失败")).toBeInTheDocument();
    // Error reason is displayed in the error block
    expect(screen.getByText("because")).toBeInTheDocument();
    // Provider ID shown in detail body
    expect(screen.getByText("99")).toBeInTheDocument();
    // Endpoint shown in detail body
    expect(screen.getByText("https://p99")).toBeInTheDocument();
  });
});
