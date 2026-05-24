import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { gatewayEventNames } from "../../constants/gatewayEvents";
import type { ConsoleLogEntry } from "../consoleLog";

async function importFresh() {
  vi.resetModules();
  return await import("../consoleLog");
}

describe("services/consoleLog (formatting)", () => {
  // ---- formatConsoleLogDetails ----
  describe("formatConsoleLogDetails", () => {
    it("returns undefined for undefined", async () => {
      const { formatConsoleLogDetails } = await importFresh();
      expect(formatConsoleLogDetails(undefined)).toBeUndefined();
    });

    it("returns 'null' for null", async () => {
      const { formatConsoleLogDetails } = await importFresh();
      expect(formatConsoleLogDetails(null)).toBe("null");
    });

    it("returns string as-is", async () => {
      const { formatConsoleLogDetails } = await importFresh();
      expect(formatConsoleLogDetails("hello")).toBe("hello");
    });

    it("returns number as string", async () => {
      const { formatConsoleLogDetails } = await importFresh();
      expect(formatConsoleLogDetails(42)).toBe("42");
    });

    it("returns boolean as string", async () => {
      const { formatConsoleLogDetails } = await importFresh();
      expect(formatConsoleLogDetails(true)).toBe("true");
    });

    it("returns bigint as string", async () => {
      const { formatConsoleLogDetails } = await importFresh();
      expect(formatConsoleLogDetails(BigInt(123))).toBe("123");
    });

    it("returns JSON for objects", async () => {
      const { formatConsoleLogDetails } = await importFresh();
      const result = formatConsoleLogDetails({ a: 1 });
      expect(result).toContain('"a": 1');
    });
  });

  // ---- formatConsoleLogDetailsSmart ----
  describe("formatConsoleLogDetailsSmart", () => {
    it("returns undefined for undefined details", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: undefined,
      };
      expect(formatConsoleLogDetailsSmart(entry)).toBeUndefined();
    });

    it("falls back to formatConsoleLogDetails for non-object details", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: "plain string",
      };
      expect(formatConsoleLogDetailsSmart(entry)).toBe("plain string");
    });

    it("formats gateway:request event", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: {
          trace_id: "t-1",
          cli: "claude",
          status: 200,
          duration_ms: 500,
          ttfb_ms: 100,
          input_tokens: 10,
          output_tokens: 20,
          total_tokens: 30,
          output_tokens_per_second: 40.5,
          cache_read_input_tokens: 5,
          cache_creation_input_tokens: 3,
          attempts: [
            { provider_name: "P1", status: 200, outcome: "success", attempt_duration_ms: 100 },
            { provider_name: "P2", status: 500, outcome: "failure", attempt_duration_ms: 200 },
          ],
          error_code: "GW_UPSTREAM_5XX",
          error_category: "upstream",
        },
        eventType: gatewayEventNames.request,
      };
      const result = formatConsoleLogDetailsSmart(entry)!;
      expect(result).toContain("请求摘要");
      expect(result).toContain("t-1");
      expect(result).toContain("claude");
      expect(result).toContain("200 (成功)");
      expect(result).toContain("500ms");
      expect(result).toContain("TTFB: 100ms");
      expect(result).toContain("输入 10");
      expect(result).toContain("输出 20");
      expect(result).toContain("40.5 tokens/sec");
      expect(result).toContain("缓存");
      expect(result).toContain("故障切换路径");
      expect(result).toContain("P1");
      expect(result).toContain("P2");
      expect(result).toContain("错误信息");
    });

    it("formats gateway:request edge branches with sparse attempts and unknown errors", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: {
          trace_id: "",
          cli: "",
          status: 302,
          duration_ms: 10,
          attempts: [null, { provider_name: "", outcome: "", status: null }],
          error_code: "GW_UNKNOWN",
        },
        eventType: gatewayEventNames.request,
      };

      const result = formatConsoleLogDetailsSmart(entry)!;

      expect(result).toContain("状态码:    302");
      expect(result).toContain("Trace ID:  —");
      expect(result).toContain("#1  —  ✗ —");
      expect(result).toContain("GW_UNKNOWN");
      expect(result).not.toContain("说明:");
    });

    it("formats gateway:attempt event", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: {
          attempt_index: 0,
          provider_name: "P1",
          provider_id: "id1",
          outcome: "success",
          status: 200,
          attempt_duration_ms: 150,
          circuit_state_before: "CLOSED",
          circuit_state_after: "CLOSED",
          circuit_failure_count: 1,
          circuit_failure_threshold: 5,
        },
        eventType: gatewayEventNames.attempt,
      };
      const result = formatConsoleLogDetailsSmart(entry)!;
      expect(result).toContain("故障切换尝试");
      expect(result).toContain("P1");
      expect(result).toContain("成功");
      expect(result).toContain("150ms");
      expect(result).toContain("熔断器状态");
    });

    it("formats gateway:attempt without status or circuit details", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: {
          attempt_index: 1,
          provider_name: "P1",
          provider_id: 1,
          outcome: "failure",
        },
        eventType: gatewayEventNames.attempt,
      };

      const result = formatConsoleLogDetailsSmart(entry)!;

      expect(result).toContain("失败");
      expect(result).not.toContain("status:");
      expect(result).not.toContain("熔断器状态");
    });

    it("formats gateway:attempt with near-threshold warning", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: {
          attempt_index: 0,
          provider_name: "P1",
          provider_id: "id1",
          outcome: "failure",
          status: 500,
          circuit_state_before: "CLOSED",
          circuit_state_after: "CLOSED",
          circuit_failure_count: 4,
          circuit_failure_threshold: 5,
        },
        eventType: gatewayEventNames.attempt,
      };
      const result = formatConsoleLogDetailsSmart(entry)!;
      expect(result).toContain("距离熔断阈值还差 1 次失败");
    });

    it("formats gateway:circuit OPEN event", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const nowSec = Math.floor(Date.now() / 1000);
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: {
          provider_name: "P1",
          prev_state: "CLOSED",
          next_state: "熔断",
          reason: "too many failures",
          failure_count: 5,
          failure_threshold: 5,
          open_until: nowSec + 300,
          trace_id: "t-1",
        },
        eventType: gatewayEventNames.circuit,
      };
      const result = formatConsoleLogDetailsSmart(entry)!;
      expect(result).toContain("熔断器触发");
      expect(result).toContain("P1");
      expect(result).toContain("too many failures");
      expect(result).toContain("熔断持续");
      expect(result).toContain("建议");
    });

    it("formats gateway:circuit CLOSED event", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: {
          provider_name: "P1",
          prev_state: "OPEN",
          next_state: "CLOSED",
          reason: "recovered",
          failure_count: 0,
          failure_threshold: 5,
        },
        eventType: gatewayEventNames.circuit,
      };
      const result = formatConsoleLogDetailsSmart(entry)!;
      expect(result).toContain("熔断器恢复");
      expect(result).toContain("已恢复正常");
    });

    it("formats gateway:circuit other state", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: {
          provider_name: "P1",
          prev_state: "HALF_OPEN",
          next_state: "HALF_OPEN",
          reason: "probing",
          failure_count: 2,
          failure_threshold: 5,
        },
        eventType: gatewayEventNames.circuit,
      };
      const result = formatConsoleLogDetailsSmart(entry)!;
      expect(result).toContain("熔断器半开试探");
    });

    it("formats gateway:circuit with expired open_until", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const nowSec = Math.floor(Date.now() / 1000);
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: {
          provider_name: "P1",
          prev_state: "CLOSED",
          next_state: "熔断",
          reason: "fail",
          failure_count: 5,
          failure_threshold: 5,
          open_until: nowSec - 10,
        },
        eventType: gatewayEventNames.circuit,
      };
      const result = formatConsoleLogDetailsSmart(entry)!;
      expect(result).toContain("已到期");
    });

    it("formats gateway:log event", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: {
          error_code: "GW_PORT_IN_USE",
          message: "Port 8080 in use",
          requested_port: 8080,
          bound_port: 8081,
        },
        eventType: gatewayEventNames.log,
      };
      const result = formatConsoleLogDetailsSmart(entry)!;
      expect(result).toContain("网关事件");
      expect(result).toContain("GW_PORT_IN_USE");
      expect(result).toContain("Port 8080 in use");
      expect(result).toContain("请求端口");
      expect(result).toContain("实际端口");
    });

    it("formats gateway:log event without error code", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: {},
        eventType: gatewayEventNames.log,
      };

      const result = formatConsoleLogDetailsSmart(entry)!;
      expect(result).toContain("网关事件: 未知");
      expect(result).not.toContain("说明");
    });

    it("formats gateway:request_start event", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: {
          trace_id: "t-1",
          cli: "claude",
          method: "POST",
          path: "/v1/messages",
        },
        eventType: gatewayEventNames.requestStart,
      };
      const result = formatConsoleLogDetailsSmart(entry)!;
      expect(result).toContain("请求开始");
      expect(result).toContain("t-1");
      expect(result).toContain("POST");
      expect(result).toContain("/v1/messages");
    });

    it("falls back for unknown eventType", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: { foo: "bar" },
        eventType: "unknown:event",
      };
      const result = formatConsoleLogDetailsSmart(entry)!;
      expect(result).toContain("foo");
    });
  });

  // ---- logToConsole with eventType ----
  describe("logToConsole with eventType", () => {
    it("stores eventType in entry", async () => {
      const mod = await importFresh();
      mod.clearConsoleLogs();
      mod.setConsoleLogMinLevel("debug");

      const { result } = renderHook(() => mod.useConsoleLogs());
      expect(result.current).toEqual([]);

      act(() => {
        mod.logToConsole("info", "test", { trace_id: "t-1" }, gatewayEventNames.request);
      });

      await waitFor(() => {
        expect(result.current.length).toBe(1);
      });

      expect(result.current[0].eventType).toBe(gatewayEventNames.request);
    });
  });

  // ---- clearConsoleLogs ----
  describe("clearConsoleLogs", () => {
    it("clears all entries", async () => {
      const mod = await importFresh();
      mod.setConsoleLogMinLevel("debug");
      mod.clearConsoleLogs();

      const { result } = renderHook(() => mod.useConsoleLogs());

      act(() => {
        mod.logToConsole("info", "entry1");
        mod.logToConsole("info", "entry2");
      });

      await waitFor(() => {
        expect(result.current.length).toBe(2);
      });

      act(() => {
        mod.clearConsoleLogs();
      });

      await waitFor(() => {
        expect(result.current.length).toBe(0);
      });
    });
  });

  // ---- setConsoleDebugEnabled ----
  describe("setConsoleDebugEnabled", () => {
    it("toggles debug mode", async () => {
      const mod = await importFresh();
      mod.setConsoleDebugEnabled(true);
      expect(mod.getConsoleDebugEnabled()).toBe(true);
      expect(mod.getConsoleLogMinLevel()).toBe("debug");

      mod.setConsoleDebugEnabled(false);
      expect(mod.getConsoleDebugEnabled()).toBe(false);
      expect(mod.getConsoleLogMinLevel()).toBe("info");
    });
  });

  // ---- redaction via logToConsole ----
  describe("redaction", () => {
    it("redacts sensitive keys in nested objects", async () => {
      const mod = await importFresh();
      mod.clearConsoleLogs();
      mod.setConsoleLogMinLevel("debug");

      const { result } = renderHook(() => mod.useConsoleLogs());

      act(() => {
        mod.logToConsole("info", "test", {
          api_key: "secret",
          nested: { base_url: "http://x", authorization: "Bearer x", safe: "ok" },
          arr: [{ access_token: "tok" }],
        });
      });

      await waitFor(() => {
        expect(result.current.length).toBe(1);
      });

      const details = result.current[0].details as Record<string, unknown>;
      expect(details.api_key).toBe("[REDACTED]");
      const nested = details.nested as Record<string, unknown>;
      expect(nested.base_url).toBe("[REDACTED]");
      expect(nested.authorization).toBe("[REDACTED]");
      expect(nested.safe).toBe("ok");
    });
  });

  // ---- extractMeta via logToConsole ----
  describe("extractMeta", () => {
    it("extracts camelCase meta variants", async () => {
      const mod = await importFresh();
      mod.clearConsoleLogs();
      mod.setConsoleLogMinLevel("debug");

      const { result } = renderHook(() => mod.useConsoleLogs());

      act(() => {
        mod.logToConsole("info", "test", {
          traceId: "t-1",
          cliKey: "claude",
          errorCode: "GW_ERR",
          providerName: "P1",
          providers: ["P2", "P3"],
        });
      });

      await waitFor(() => {
        expect(result.current.length).toBe(1);
      });

      const meta = result.current[0].meta!;
      expect(meta.trace_id).toBe("t-1");
      expect(meta.cli_key).toBe("claude");
      expect(meta.error_code).toBe("GW_ERR");
      expect(meta.providers).toEqual(expect.arrayContaining(["P1", "P2", "P3"]));
    });
  });

  // ---- statusLabel via gateway:request formatting ----
  describe("statusLabel edge cases", () => {
    it("handles 4xx status", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: { status: 429 },
        eventType: gatewayEventNames.request,
      };
      const result = formatConsoleLogDetailsSmart(entry)!;
      expect(result).toContain("429 (客户端错误)");
    });

    it("handles 5xx status", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: { status: 502 },
        eventType: gatewayEventNames.request,
      };
      const result = formatConsoleLogDetailsSmart(entry)!;
      expect(result).toContain("502 (服务端错误)");
    });

    it("handles non-finite status", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: { status: "not-a-number" },
        eventType: gatewayEventNames.request,
      };
      const result = formatConsoleLogDetailsSmart(entry)!;
      expect(result).toContain("not-a-number");
    });

    it("handles null status", async () => {
      const { formatConsoleLogDetailsSmart } = await importFresh();
      const entry: ConsoleLogEntry = {
        id: "1",
        ts: Date.now(),
        tsText: "",
        level: "info",
        title: "test",
        details: { status: null },
        eventType: gatewayEventNames.request,
      };
      const result = formatConsoleLogDetailsSmart(entry)!;
      expect(result).toContain("—");
    });
  });
});
