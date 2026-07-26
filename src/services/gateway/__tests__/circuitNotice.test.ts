import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../notification/notice", () => ({
  noticeSend: vi.fn().mockResolvedValue(true),
}));

vi.mock("../../consoleLog", () => ({
  logToConsole: vi.fn(),
}));

import { logToConsole } from "../../consoleLog";
import { noticeSend } from "../../notification/notice";
import {
  buildCircuitNoticeContent,
  getCircuitBreakerNoticeEnabled,
  maybeSendCircuitBreakerNotice,
  setCircuitBreakerNoticeEnabled,
} from "../circuitNotice";
import type { GatewayCircuitEvent } from "../gatewayEvents";

// 语义对齐已删除的 Rust 单测（src-tauri/src/gateway/events.rs build_circuit_notice tests）。
function circuitEvent(overrides: Partial<GatewayCircuitEvent> = {}): GatewayCircuitEvent {
  return {
    trace_id: "trace-1",
    cli_key: "claude",
    provider_id: 7,
    provider_name: "Provider A",
    base_url: "https://provider-a.example",
    prev_state: "CLOSED",
    next_state: "OPEN",
    failure_count: 5,
    failure_threshold: 5,
    open_until: 1_750_001_800,
    cooldown_until: null,
    reason: "FAILURE_THRESHOLD_REACHED",
    ts: 1_750_000_000,
    trigger_error_code: null,
    first_byte_timeout_secs: null,
    ...overrides,
  };
}

describe("services/gateway/circuitNotice buildCircuitNoticeContent", () => {
  it("→熔断且超时触发时追加触发失败行与首字节超时提示行", () => {
    const content = buildCircuitNoticeContent(
      circuitEvent({ trigger_error_code: "GW_UPSTREAM_TIMEOUT", first_byte_timeout_secs: 300 })
    );

    expect(content).not.toBeNull();
    expect(content?.level).toBe("warning");
    expect(content?.title).toBe("熔断触发：Provider A");
    const lines = content?.lines ?? [];
    const reasonIndex = lines.findIndex((line) => line.startsWith("原因："));
    expect(reasonIndex).toBeGreaterThanOrEqual(0);
    expect(lines[reasonIndex + 1]).toBe("触发失败：上游超时（GW_UPSTREAM_TIMEOUT）");
    expect(lines[reasonIndex + 2]).toBe(
      "首字节超时配置：300 秒；若上游响应慢属预期，可调大：设置 → 通用 → 首字节超时（0=禁用）"
    );
  });

  it("→熔断且 5XX 触发时不追加首字节超时提示", () => {
    const content = buildCircuitNoticeContent(
      circuitEvent({ trigger_error_code: "GW_UPSTREAM_5XX", first_byte_timeout_secs: 300 })
    );

    const lines = content?.lines ?? [];
    expect(lines).toContain("触发失败：上游5XX（GW_UPSTREAM_5XX）");
    expect(lines.some((line) => line.includes("首字节超时配置"))).toBe(false);
  });

  it("未映射的触发错误码回退为原始码", () => {
    const content = buildCircuitNoticeContent(
      circuitEvent({ trigger_error_code: "GW_SOMETHING_NEW" })
    );

    expect(content?.lines).toContain("触发失败：GW_SOMETHING_NEW（GW_SOMETHING_NEW）");
  });

  it("超时触发但缺少秒数时省略提示行", () => {
    const content = buildCircuitNoticeContent(
      circuitEvent({ trigger_error_code: "GW_UPSTREAM_TIMEOUT", first_byte_timeout_secs: null })
    );

    expect(content?.lines).toContain("触发失败：上游超时（GW_UPSTREAM_TIMEOUT）");
    expect((content?.lines ?? []).some((line) => line.includes("首字节超时配置"))).toBe(false);
  });

  it("旧后端事件（无触发字段）正文与旧版 Rust 文案一致", () => {
    // 生成类型里这两个字段必填；解构剔除后断言（旧后端 payload 运行时确实缺字段）。
    const {
      trigger_error_code: _triggerErrorCode,
      first_byte_timeout_secs: _firstByteTimeoutSecs,
      ...legacy
    } = circuitEvent();

    const content = buildCircuitNoticeContent(legacy as GatewayCircuitEvent);

    expect(content?.level).toBe("warning");
    expect(content?.title).toBe("熔断触发：Provider A");
    expect(content?.lines).toEqual([
      "CLI：claude",
      "Provider：Provider A (id=7)",
      "Base URL：https://provider-a.example",
      "状态：正常 → 熔断",
      "失败：5 / 5",
      "原因：失败次数达到阈值（FAILURE_THRESHOLD_REACHED）",
      "熔断至：1750001800（约 30 分钟后）",
      "Trace：trace-1",
    ]);
  });

  it("非 →熔断 跃迁忽略触发字段（半开/恢复标题与级别）", () => {
    const halfOpenBase = {
      prev_state: "OPEN",
      next_state: "HALF_OPEN",
      reason: "OPEN_EXPIRED",
    } as const;
    const closedBase = {
      prev_state: "HALF_OPEN",
      next_state: "CLOSED",
      reason: "HALF_OPEN_SUCCESS",
      open_until: null,
    } as const;

    const halfOpen = buildCircuitNoticeContent(
      circuitEvent({ ...halfOpenBase, trigger_error_code: "GW_UPSTREAM_TIMEOUT" })
    );
    expect(halfOpen?.level).toBe("info");
    expect(halfOpen?.title).toBe("熔断试探：Provider A");
    expect(halfOpen?.lines).toEqual(buildCircuitNoticeContent(circuitEvent(halfOpenBase))?.lines);
    expect(halfOpen?.lines.some((line) => line.startsWith("触发失败"))).toBe(false);
    expect(halfOpen?.lines).toContain("原因：熔断到期，进入半开试探（OPEN_EXPIRED）");

    const closed = buildCircuitNoticeContent(
      circuitEvent({ ...closedBase, trigger_error_code: "GW_UPSTREAM_TIMEOUT" })
    );
    expect(closed?.level).toBe("success");
    expect(closed?.title).toBe("熔断恢复：Provider A");
    expect(closed?.lines.some((line) => line.startsWith("触发失败"))).toBe(false);
    expect(closed?.lines).toContain("熔断至：—");
  });

  it("open_until 已到期与未知 reason 透传", () => {
    const content = buildCircuitNoticeContent(
      circuitEvent({ open_until: 1_750_000_000, reason: "SOMETHING_ELSE" })
    );

    expect(content?.lines).toContain("熔断至：1750000000（已到期）");
    expect(content?.lines).toContain("原因：SOMETHING_ELSE（SOMETHING_ELSE）");
  });

  it("非跃迁或未知状态返回 null", () => {
    expect(
      buildCircuitNoticeContent(circuitEvent({ prev_state: "OPEN", next_state: "OPEN" }))
    ).toBeNull();
    expect(buildCircuitNoticeContent(circuitEvent({ prev_state: "WEIRD" }))).toBeNull();
    expect(buildCircuitNoticeContent(circuitEvent({ next_state: "WEIRD" }))).toBeNull();
  });
});

describe("services/gateway/circuitNotice maybeSendCircuitBreakerNotice", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("开关开启且状态跃迁时通过 noticeSend 发送通知", async () => {
    setCircuitBreakerNoticeEnabled(true);

    await maybeSendCircuitBreakerNotice(
      circuitEvent({ trigger_error_code: "GW_UPSTREAM_TIMEOUT", first_byte_timeout_secs: 300 })
    );

    expect(noticeSend).toHaveBeenCalledTimes(1);
    const arg = vi.mocked(noticeSend).mock.calls[0]?.[0];
    expect(arg?.level).toBe("warning");
    expect(arg?.title).toBe("熔断触发：Provider A");
    expect(arg?.body).toContain("触发失败：上游超时（GW_UPSTREAM_TIMEOUT）");
    expect(arg?.body).toContain("\nTrace：trace-1");
  });

  it("开关关闭时不发送（事件本身不受影响）", async () => {
    setCircuitBreakerNoticeEnabled(false);

    await maybeSendCircuitBreakerNotice(circuitEvent());

    expect(noticeSend).not.toHaveBeenCalled();
  });

  it("prev == next 的非跃迁事件不发送", async () => {
    setCircuitBreakerNoticeEnabled(true);

    await maybeSendCircuitBreakerNotice(
      circuitEvent({ prev_state: "OPEN", next_state: "OPEN", reason: "SKIP_OPEN" })
    );

    expect(noticeSend).not.toHaveBeenCalled();
  });

  it("noticeSend 失败时不抛错，仅记录日志", async () => {
    setCircuitBreakerNoticeEnabled(true);
    vi.mocked(noticeSend).mockRejectedValueOnce(new Error("ipc boom"));

    await expect(maybeSendCircuitBreakerNotice(circuitEvent())).resolves.toBeUndefined();

    expect(logToConsole).toHaveBeenCalledWith(
      "warn",
      "发送熔断通知失败",
      expect.objectContaining({ error: expect.stringContaining("ipc boom") })
    );
  });

  it("settings 未就绪时按 Rust 默认值兜底（默认禁用，不发送）", async () => {
    // 默认值来源：src-tauri/src/infra/settings/defaults.rs
    // DEFAULT_ENABLE_CIRCUIT_BREAKER_NOTICE = false
    vi.resetModules();
    const freshNotice = await import("../../notification/notice");
    const fresh = await import("../circuitNotice");

    expect(fresh.getCircuitBreakerNoticeEnabled()).toBe(false);
    await fresh.maybeSendCircuitBreakerNotice(circuitEvent());
    expect(vi.mocked(freshNotice.noticeSend)).not.toHaveBeenCalled();
  });

  it("setCircuitBreakerNoticeEnabled 读写一致", () => {
    setCircuitBreakerNoticeEnabled(true);
    expect(getCircuitBreakerNoticeEnabled()).toBe(true);
    setCircuitBreakerNoticeEnabled(false);
    expect(getCircuitBreakerNoticeEnabled()).toBe(false);
  });
});
