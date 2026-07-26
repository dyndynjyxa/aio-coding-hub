import { describe, expect, it } from "vitest";
import { hasFailoverFromSegments } from "../traceRoute";

describe("services/gateway/traceRoute hasFailoverFromSegments", () => {
  it("multi-provider switch counts as failover", () => {
    expect(
      hasFailoverFromSegments([
        { provider: "A", status: "failed" },
        { provider: "B", status: "success" },
      ])
    ).toBe(true);
  });

  it("single provider with failed segment (retry then success) is not failover", () => {
    // 回归锚点：旧内联判定因 some(failed) 返回 true，导致落库后徽章跳变。
    expect(hasFailoverFromSegments([{ provider: "A", status: "failed" }])).toBe(false);
    expect(
      hasFailoverFromSegments([
        { provider: "A", status: "failed" },
        { provider: "A", status: "success" },
      ])
    ).toBe(false);
  });

  it("counts hops by provider sequence regardless of status (skipped included, mirrors Rust)", () => {
    // 与 Rust route_from_attempts 一致：provider_id>0 的 skipped attempt 计入 hop
    //（其测试 route_includes_skipped_attempts）。真实输入（RealtimeTraceCards
    // segments）从不产出 "skipped"（已映射为 "failed"），此处锚定镜像语义本身。
    expect(
      hasFailoverFromSegments([
        { provider: "A", status: "skipped" },
        { provider: "B", status: "success" },
      ])
    ).toBe(true);
    // 单 provider 的 skipped 仍只是一个 hop，不算 failover。
    expect(hasFailoverFromSegments([{ provider: "A", status: "skipped" }])).toBe(false);
  });

  it("empty segments are not failover and do not throw", () => {
    expect(hasFailoverFromSegments([])).toBe(false);
  });
});
