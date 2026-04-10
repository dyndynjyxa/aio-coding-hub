import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { logToConsole } from "../../../services/consoleLog";
import { cliProxySyncEnabled } from "../../../services/cli/cliProxy";
import {
  gatewayCheckPortAvailable,
  gatewayStart,
  gatewayStop,
} from "../../../services/gateway/gateway";
import { useSettingsQuery, useSettingsSetMutation } from "../../../query/settings";
import { useSettingsPersistence } from "../useSettingsPersistence";

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));
vi.mock("../../../services/cli/cliProxy", async () => {
  const actual = await vi.importActual<typeof import("../../../services/cli/cliProxy")>(
    "../../../services/cli/cliProxy"
  );
  return { ...actual, cliProxySyncEnabled: vi.fn() };
});
vi.mock("../../../services/gateway/gateway", async () => {
  const actual = await vi.importActual<typeof import("../../../services/gateway/gateway")>(
    "../../../services/gateway/gateway"
  );
  return {
    ...actual,
    gatewayCheckPortAvailable: vi.fn(),
    gatewayStart: vi.fn(),
    gatewayStop: vi.fn(),
  };
});

vi.mock("../../../query/settings", async () => {
  const actual =
    await vi.importActual<typeof import("../../../query/settings")>("../../../query/settings");
  return { ...actual, useSettingsQuery: vi.fn(), useSettingsSetMutation: vi.fn() };
});

function createSettings(overrides: Partial<any> = {}) {
  return {
    schema_version: 1,
    preferred_port: 37123,
    show_home_heatmap: true,
    show_home_usage: true,
    home_usage_period: "last15",
    cli_priority_order: ["claude", "codex", "gemini"],
    auto_start: false,
    tray_enabled: true,
    log_retention_days: 7,
    provider_cooldown_seconds: 30,
    provider_base_url_ping_cache_ttl_seconds: 60,
    upstream_first_byte_timeout_seconds: 0,
    upstream_stream_idle_timeout_seconds: 0,
    upstream_request_timeout_non_streaming_seconds: 0,
    intercept_anthropic_warmup_requests: false,
    enable_thinking_signature_rectifier: true,
    enable_cache_anomaly_monitor: false,
    enable_response_fixer: true,
    response_fixer_fix_encoding: true,
    response_fixer_fix_sse_format: true,
    response_fixer_fix_truncated_json: true,
    failover_max_attempts_per_provider: 5,
    failover_max_providers_to_try: 5,
    circuit_breaker_failure_threshold: 5,
    circuit_breaker_open_duration_minutes: 30,
    ...overrides,
  };
}

describe("settings/useSettingsPersistence", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("defers initialization while settings query is loading and ignores persistence until ready", async () => {
    const mutation = { mutateAsync: vi.fn() };
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    let loading = true;
    vi.mocked(useSettingsQuery).mockImplementation(() => {
      return loading
        ? ({ data: null, isLoading: true, isError: false, error: null } as any)
        : ({ data: createSettings(), isLoading: false, isError: false, error: null } as any);
    });

    const { result, rerender } = renderHook(() =>
      useSettingsPersistence({ gateway: null, about: null })
    );

    expect(result.current.settingsReady).toBe(false);

    act(() => {
      // should no-op until settingsReady
      result.current.requestPersist({ auto_start: true });
      result.current.commitNumberField({
        key: "log_retention_days",
        next: 10,
        min: 1,
        max: 3650,
        invalidMessage: "bad",
      });
    });
    expect(mutation.mutateAsync).not.toHaveBeenCalled();

    loading = false;
    rerender();

    await waitFor(() => expect(result.current.settingsReady).toBe(true));
  });

  it("marks ready when settings query returns null", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: null,
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    vi.mocked(useSettingsSetMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    const { result } = renderHook(() => useSettingsPersistence({ gateway: null, about: null }));

    await waitFor(() => {
      expect(result.current.settingsReady).toBe(true);
    });
  });

  it("initializes missing optional fields with defaults", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings({
        tray_enabled: undefined,
        show_home_usage: undefined,
        home_usage_period: undefined,
        provider_cooldown_seconds: undefined,
        provider_base_url_ping_cache_ttl_seconds: undefined,
        upstream_first_byte_timeout_seconds: undefined,
        upstream_stream_idle_timeout_seconds: undefined,
        upstream_request_timeout_non_streaming_seconds: undefined,
        intercept_anthropic_warmup_requests: undefined,
        enable_thinking_signature_rectifier: undefined,
        enable_cache_anomaly_monitor: undefined,
        enable_response_fixer: undefined,
        response_fixer_fix_encoding: undefined,
        response_fixer_fix_sse_format: undefined,
        response_fixer_fix_truncated_json: undefined,
        failover_max_attempts_per_provider: undefined,
        failover_max_providers_to_try: undefined,
        circuit_breaker_failure_threshold: undefined,
        circuit_breaker_open_duration_minutes: undefined,
      }),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    vi.mocked(useSettingsSetMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    const { result } = renderHook(() => useSettingsPersistence({ gateway: null, about: null }));
    await waitFor(() => expect(result.current.settingsReady).toBe(true));
    expect(result.current.trayEnabled).toBe(true);
    expect(result.current.showHomeUsage).toBe(true);
    expect(result.current.homeUsagePeriod).toBe("last15");
    expect(result.current.cliPriorityOrder).toEqual(["claude", "codex", "gemini"]);
  });

  it("marks ready and toasts when settings query errors", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: null,
      isLoading: false,
      isError: true,
      error: new Error("boom"),
    } as any);

    vi.mocked(useSettingsSetMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    const { result } = renderHook(() => useSettingsPersistence({ gateway: null, about: null }));

    await waitFor(() => {
      expect(result.current.settingsReady).toBe(true);
    });
    expect(result.current.settingsWriteBlocked).toBe(true);
    expect(result.current.settingsReadErrorMessage).toContain("已进入只读保护");
    expect(logToConsole).toHaveBeenCalledWith("error", "读取设置失败", { error: "Error: boom" });
    expect(toast).toHaveBeenCalledWith(
      "设置文件读取失败，已进入只读保护。请先修复或恢复 settings.json 后刷新页面。"
    );
  });

  it("clears readonly protection when a later query succeeds", async () => {
    const mutation = { mutateAsync: vi.fn() };
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    let queryState: any = {
      data: null,
      isLoading: false,
      isError: true,
      error: new Error("boom"),
    };

    vi.mocked(useSettingsQuery).mockImplementation(() => queryState);

    const { result, rerender } = renderHook(() =>
      useSettingsPersistence({ gateway: null, about: null })
    );

    await waitFor(() => expect(result.current.settingsWriteBlocked).toBe(true));
    expect(result.current.port).toBe(37123);
    expect(toast).toHaveBeenCalledTimes(1);

    queryState = {
      data: createSettings({ preferred_port: 38001, show_home_heatmap: false }),
      isLoading: false,
      isError: false,
      error: null,
    };
    rerender();

    await waitFor(() => expect(result.current.settingsWriteBlocked).toBe(false));
    expect(result.current.settingsReadErrorMessage).toBeNull();
    expect(result.current.port).toBe(38001);
    expect(result.current.showHomeHeatmap).toBe(false);
    expect(toast).toHaveBeenCalledTimes(1);
  });

  it("keeps showing cached data but blocks writes when refetch falls back to stale data", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings({ preferred_port: 38002 }),
      isLoading: false,
      isError: true,
      error: new Error("stale boom"),
    } as any);

    vi.mocked(useSettingsSetMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    const { result } = renderHook(() => useSettingsPersistence({ gateway: null, about: null }));

    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    expect(result.current.settingsWriteBlocked).toBe(true);
    expect(result.current.settingsReadErrorMessage).toContain("已进入只读保护");
    expect(result.current.port).toBe(38002);
    expect(logToConsole).toHaveBeenCalledWith("error", "读取设置失败", {
      error: "Error: stale boom",
    });
    expect(toast).toHaveBeenCalledWith(
      "设置文件读取失败，已进入只读保护。请先修复或恢复 settings.json 后刷新页面。"
    );
  });

  it("blocks requestPersist and commitNumberField when settings are readonly", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: null,
      isLoading: false,
      isError: true,
      error: new Error("boom"),
    } as any);

    const mutation = { mutateAsync: vi.fn() };
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() => useSettingsPersistence({ gateway: null, about: null }));
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    vi.mocked(toast).mockClear();

    act(() => {
      result.current.setShowHomeHeatmap(false);
      result.current.requestPersist({ show_home_heatmap: false });
    });

    act(() => {
      result.current.setPort(40000);
      result.current.commitNumberField({
        key: "preferred_port",
        next: 40000,
        min: 1024,
        max: 65535,
        invalidMessage: "bad-port",
      });
    });

    expect(mutation.mutateAsync).not.toHaveBeenCalled();
    expect(result.current.showHomeHeatmap).toBe(true);
    expect(result.current.port).toBe(37123);
    expect(toast).toHaveBeenCalledWith(
      "设置文件读取失败，已进入只读保护。请先修复或恢复 settings.json 后刷新页面。"
    );
  });

  it("reverts when settings_set returns null (tauri unavailable)", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings(),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    vi.mocked(gatewayCheckPortAvailable).mockResolvedValue(true);

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync.mockResolvedValue(null);
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() => useSettingsPersistence({ gateway: null, about: null }));

    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    await act(async () => {
      result.current.commitNumberField({
        key: "preferred_port",
        next: 40000,
        min: 1024,
        max: 65535,
        invalidMessage: "bad",
      });
    });

    await waitFor(() => {
      expect(mutation.mutateAsync).toHaveBeenCalled();
    });

    expect(result.current.port).toBe(37123);
  });

  it("toasts and reverts when committing an invalid numeric field", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings(),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    const mutation = { mutateAsync: vi.fn() };
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() => useSettingsPersistence({ gateway: null, about: null }));
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.commitNumberField({
        key: "preferred_port",
        next: 1,
        min: 1024,
        max: 65535,
        invalidMessage: "bad-port",
      });
    });

    expect(toast).toHaveBeenCalledWith("bad-port");
    expect(result.current.port).toBe(37123);
    expect(mutation.mutateAsync).not.toHaveBeenCalled();
  });

  it("checks port availability and reverts when occupied", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings(),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    vi.mocked(gatewayCheckPortAvailable).mockResolvedValue(false);

    const mutation = { mutateAsync: vi.fn() };
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() =>
      useSettingsPersistence({
        gateway: { running: false, port: null, base_url: null, listen_addr: null },
        about: null,
      })
    );
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.commitNumberField({
        key: "preferred_port",
        next: 40000,
        min: 1024,
        max: 65535,
        invalidMessage: "bad",
      });
    });

    await waitFor(() => expect(gatewayCheckPortAvailable).toHaveBeenCalledWith(40000));
    expect(toast).toHaveBeenCalledWith("端口 40000 已被占用，请换一个端口");
    expect(mutation.mutateAsync).not.toHaveBeenCalled();
  });

  it("switches to readonly protection when port check fails because settings.json is broken", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings(),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    vi.mocked(gatewayCheckPortAvailable).mockRejectedValue(
      new Error("SEC_INVALID_INPUT: invalid settings.json: boom")
    );

    const mutation = { mutateAsync: vi.fn() };
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() =>
      useSettingsPersistence({
        gateway: { running: false, port: null, base_url: null, listen_addr: null },
        about: null,
      })
    );
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.commitNumberField({
        key: "preferred_port",
        next: 40000,
        min: 1024,
        max: 65535,
        invalidMessage: "bad",
      });
    });

    await waitFor(() => expect(gatewayCheckPortAvailable).toHaveBeenCalledWith(40000));
    await waitFor(() => expect(result.current.settingsWriteBlocked).toBe(true));
    expect(result.current.port).toBe(37123);
    expect(mutation.mutateAsync).not.toHaveBeenCalled();
    expect(toast).toHaveBeenCalledWith(
      "设置文件读取失败，已进入只读保护。请先修复或恢复 settings.json 后刷新页面。"
    );
    expect(toast).not.toHaveBeenCalledWith("端口 40000 已被占用，请换一个端口");
  });

  it("queues pending persists and validates numeric bounds", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings(),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    const resolveFirst: { fn?: (v: any) => void } = {};
    const firstPromise = new Promise<any>((resolve) => {
      resolveFirst.fn = resolve;
    });

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync
      .mockReturnValueOnce(firstPromise)
      .mockResolvedValueOnce(createSettings({ provider_base_url_ping_cache_ttl_seconds: 120 }));
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() => useSettingsPersistence({ gateway: null, about: null }));
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.requestPersist({ provider_cooldown_seconds: 12 });
      result.current.requestPersist({ provider_base_url_ping_cache_ttl_seconds: 120 });
    });

    await waitFor(() => {
      expect(mutation.mutateAsync).toHaveBeenCalledTimes(1);
    });

    resolveFirst.fn?.(createSettings({ provider_cooldown_seconds: 12 }));

    await waitFor(() => {
      expect(mutation.mutateAsync).toHaveBeenCalledTimes(2);
    });

    act(() => {
      result.current.requestPersist({ provider_base_url_ping_cache_ttl_seconds: 0 });
    });

    await waitFor(() => {
      expect(toast).toHaveBeenCalledWith("Ping 选择缓存 TTL 必须为 1-3600 秒");
    });
    expect(mutation.mutateAsync).toHaveBeenCalledTimes(2);
  });

  it("persists homepage usage period changes", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings(),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync.mockResolvedValue(createSettings({ home_usage_period: "month" }));
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() => useSettingsPersistence({ gateway: null, about: null }));
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.setHomeUsagePeriod("month");
      result.current.requestPersist({ home_usage_period: "month" });
    });

    await waitFor(() =>
      expect(mutation.mutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({ homeUsagePeriod: "month" })
      )
    );
    expect(result.current.homeUsagePeriod).toBe("month");
  });

  it("no-ops when requestPersist does not change any keys", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings({ tray_enabled: true }),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    const mutation = { mutateAsync: vi.fn() };
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() =>
      useSettingsPersistence({
        gateway: { running: false, port: null, base_url: null, listen_addr: null },
        about: null,
      })
    );
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.requestPersist({ tray_enabled: true } as any);
    });

    await Promise.resolve();
    expect(mutation.mutateAsync).not.toHaveBeenCalled();
  });

  it("validates additional numeric bounds and commits log retention days", async () => {
    vi.mocked(toast).mockClear();

    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings(),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    vi.mocked(gatewayCheckPortAvailable).mockResolvedValue(true);

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync.mockResolvedValue(createSettings({ log_retention_days: 10 }));
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() => useSettingsPersistence({ gateway: null, about: null }));
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    // commit log retention
    act(() => {
      result.current.commitNumberField({
        key: "log_retention_days",
        next: 10,
        min: 1,
        max: 3650,
        invalidMessage: "bad",
      });
    });
    await waitFor(() => expect(mutation.mutateAsync).toHaveBeenCalled());

    // upstream first byte timeout invalid
    act(() => {
      result.current.requestPersist({ upstream_first_byte_timeout_seconds: -1 } as any);
    });
    await waitFor(() => expect(toast).toHaveBeenCalledWith("上游首字节超时必须为 0-3600 秒"));

    // upstream stream idle timeout invalid
    act(() => {
      result.current.requestPersist({ upstream_stream_idle_timeout_seconds: 3601 } as any);
    });
    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith("上游流式空闲超时必须为 0（禁用）或 60-3600 秒")
    );

    // upstream non-streaming timeout invalid
    act(() => {
      result.current.requestPersist({
        upstream_request_timeout_non_streaming_seconds: 86401,
      } as any);
    });
    await waitFor(() => expect(toast).toHaveBeenCalledWith("上游非流式总超时必须为 0-86400 秒"));

    // circuit breaker open duration invalid
    act(() => {
      result.current.requestPersist({ circuit_breaker_open_duration_minutes: 0 } as any);
    });
    await waitFor(() => expect(toast).toHaveBeenCalledWith("熔断时长必须为 1-1440 分钟"));
  });

  it("validates remaining numeric bounds (upper/lower) via requestPersist", async () => {
    vi.mocked(toast).mockClear();

    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings(),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    const mutation = { mutateAsync: vi.fn() };
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() => useSettingsPersistence({ gateway: null, about: null }));
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.requestPersist({ preferred_port: 70000 } as any);
    });
    await waitFor(() => expect(toast).toHaveBeenCalledWith("端口号必须为 1024-65535"));

    act(() => {
      result.current.requestPersist({ log_retention_days: 4000 } as any);
    });
    await waitFor(() => expect(toast).toHaveBeenCalledWith("日志保留必须为 1-3650 天"));

    act(() => {
      result.current.requestPersist({ provider_cooldown_seconds: 3601 } as any);
    });
    await waitFor(() => expect(toast).toHaveBeenCalledWith("短熔断冷却必须为 0-3600 秒"));

    act(() => {
      result.current.requestPersist({ upstream_first_byte_timeout_seconds: 3601 } as any);
    });
    await waitFor(() => expect(toast).toHaveBeenCalledWith("上游首字节超时必须为 0-3600 秒"));

    act(() => {
      result.current.requestPersist({ upstream_stream_idle_timeout_seconds: -1 } as any);
    });
    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith("上游流式空闲超时必须为 0（禁用）或 60-3600 秒")
    );

    act(() => {
      result.current.requestPersist({ upstream_request_timeout_non_streaming_seconds: -1 } as any);
    });
    await waitFor(() => expect(toast).toHaveBeenCalledWith("上游非流式总超时必须为 0-86400 秒"));

    act(() => {
      result.current.requestPersist({ circuit_breaker_failure_threshold: 0 } as any);
    });
    await waitFor(() => expect(toast).toHaveBeenCalledWith("熔断阈值必须为 1-50"));

    expect(mutation.mutateAsync).not.toHaveBeenCalled();
  });

  it("applies desired fallbacks when settings_set omits optional fields", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings({ tray_enabled: true, enable_response_fixer: true }),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    vi.mocked(gatewayCheckPortAvailable).mockResolvedValue(true);

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync.mockResolvedValue({
      ...createSettings({ preferred_port: 37123 }),
      tray_enabled: undefined,
      provider_cooldown_seconds: undefined,
      provider_base_url_ping_cache_ttl_seconds: undefined,
      upstream_first_byte_timeout_seconds: undefined,
      upstream_stream_idle_timeout_seconds: undefined,
      upstream_request_timeout_non_streaming_seconds: undefined,
      intercept_anthropic_warmup_requests: undefined,
      enable_thinking_signature_rectifier: undefined,
      enable_cache_anomaly_monitor: undefined,
      enable_response_fixer: undefined,
      response_fixer_fix_encoding: undefined,
      response_fixer_fix_sse_format: undefined,
      response_fixer_fix_truncated_json: undefined,
      failover_max_attempts_per_provider: undefined,
      failover_max_providers_to_try: undefined,
      circuit_breaker_failure_threshold: undefined,
      circuit_breaker_open_duration_minutes: undefined,
    });
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() =>
      useSettingsPersistence({
        gateway: { running: false, port: null, base_url: null, listen_addr: null },
        about: null,
      })
    );
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.requestPersist({
        tray_enabled: false,
        intercept_anthropic_warmup_requests: true,
        enable_response_fixer: false,
        response_fixer_fix_encoding: false,
        response_fixer_fix_sse_format: false,
        response_fixer_fix_truncated_json: false,
        failover_max_attempts_per_provider: 7,
        failover_max_providers_to_try: 8,
        circuit_breaker_failure_threshold: 6,
        circuit_breaker_open_duration_minutes: 40,
      } as any);
    });

    await waitFor(() => expect(mutation.mutateAsync).toHaveBeenCalled());
    await waitFor(() =>
      expect(logToConsole).toHaveBeenCalledWith(
        "info",
        "更新设置",
        expect.objectContaining({
          settings: expect.objectContaining({
            tray_enabled: false,
            intercept_anthropic_warmup_requests: true,
            enable_response_fixer: false,
            response_fixer_fix_encoding: false,
            response_fixer_fix_sse_format: false,
            response_fixer_fix_truncated_json: false,
            failover_max_attempts_per_provider: 7,
            failover_max_providers_to_try: 8,
            circuit_breaker_failure_threshold: 6,
            circuit_breaker_open_duration_minutes: 40,
          }),
        })
      )
    );
  });

  it("skips occupied port revert when preferred_port changes during check", async () => {
    vi.mocked(toast).mockClear();

    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings({ preferred_port: 37123 }),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    let resolveFirst: (v: boolean) => void = () => {
      throw new Error("resolveFirst not set");
    };
    const firstPortPromise = new Promise<boolean>((resolve) => {
      resolveFirst = resolve;
    });

    vi.mocked(gatewayCheckPortAvailable)
      .mockReturnValueOnce(firstPortPromise as any)
      .mockResolvedValueOnce(true);

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync.mockResolvedValue(createSettings({ preferred_port: 40001 }));
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() =>
      useSettingsPersistence({
        gateway: { running: false, port: null, base_url: null, listen_addr: null },
        about: null,
      })
    );
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.commitNumberField({
        key: "preferred_port",
        next: 40000,
        min: 1024,
        max: 65535,
        invalidMessage: "bad",
      });
      // while port check is in-flight, user changes port again
      result.current.commitNumberField({
        key: "preferred_port",
        next: 40001,
        min: 1024,
        max: 65535,
        invalidMessage: "bad",
      });
    });

    resolveFirst(false);

    await waitFor(() => expect(gatewayCheckPortAvailable).toHaveBeenCalledTimes(2));
    expect(toast).not.toHaveBeenCalledWith("端口 40000 已被占用，请换一个端口");
    await waitFor(() => expect(mutation.mutateAsync).toHaveBeenCalled());
  });

  it("updates pending queue snapshot when port is occupied (prevents rechecking same invalid port)", async () => {
    vi.mocked(toast).mockClear();

    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings({ preferred_port: 37123 }),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    let resolvePort: (v: boolean) => void = () => {
      throw new Error("resolvePort not set");
    };
    const portPromise = new Promise<boolean>((resolve) => {
      resolvePort = resolve;
    });
    vi.mocked(gatewayCheckPortAvailable).mockReturnValue(portPromise as any);

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync.mockResolvedValue(createSettings({ provider_cooldown_seconds: 12 }));
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() =>
      useSettingsPersistence({
        gateway: { running: false, port: null, base_url: null, listen_addr: null },
        about: null,
      })
    );
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.commitNumberField({
        key: "preferred_port",
        next: 40000,
        min: 1024,
        max: 65535,
        invalidMessage: "bad",
      });
      // enqueue another persist while port check is in-flight
      result.current.requestPersist({ provider_cooldown_seconds: 12 });
    });

    expect(gatewayCheckPortAvailable).toHaveBeenCalledTimes(1);

    resolvePort(false);

    await waitFor(() => expect(toast).toHaveBeenCalledWith("端口 40000 已被占用，请换一个端口"));

    // pending snapshot should have been updated to revert the invalid port, so it doesn't re-check port again
    await waitFor(() => expect(mutation.mutateAsync).toHaveBeenCalledTimes(1));
    expect(gatewayCheckPortAvailable).toHaveBeenCalledTimes(1);
  });

  it("toasts circuit parameters note when saved while gateway running", async () => {
    const base = createSettings({ preferred_port: 37123, circuit_breaker_failure_threshold: 5 });
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: base,
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync.mockResolvedValue(
      createSettings({ circuit_breaker_failure_threshold: 6 })
    );
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() =>
      useSettingsPersistence({
        gateway: { running: true, port: 37123, base_url: null, listen_addr: null },
        about: null,
      })
    );
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.requestPersist({ circuit_breaker_failure_threshold: 6 });
    });

    await waitFor(() => expect(mutation.mutateAsync).toHaveBeenCalled());
    expect(toast).toHaveBeenCalledWith("熔断参数已保存：重启网关后生效");
  });

  it("toasts when auto start save fails and is reverted", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings({ auto_start: false }),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync.mockResolvedValue(createSettings({ auto_start: false }));
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() =>
      useSettingsPersistence({
        gateway: { running: false, port: null, base_url: null, listen_addr: null },
        about: null,
      })
    );

    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.requestPersist({ auto_start: true });
    });

    await waitFor(() => expect(mutation.mutateAsync).toHaveBeenCalled());
    expect(toast).toHaveBeenCalledWith("开机自启设置失败，已回退");
  });

  it("syncs cli proxy when port changes while gateway not running", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings({ preferred_port: 37123 }),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    vi.mocked(gatewayCheckPortAvailable).mockResolvedValue(true);
    vi.mocked(cliProxySyncEnabled).mockResolvedValue([{ ok: true }] as any);

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync.mockResolvedValue(createSettings({ preferred_port: 40000 }));
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() =>
      useSettingsPersistence({
        gateway: { running: false, port: null, base_url: null, listen_addr: null },
        about: null,
      })
    );

    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.commitNumberField({
        key: "preferred_port",
        next: 40000,
        min: 1024,
        max: 65535,
        invalidMessage: "bad",
      });
    });

    await waitFor(() => expect(mutation.mutateAsync).toHaveBeenCalled());
    await waitFor(() =>
      expect(cliProxySyncEnabled).toHaveBeenCalledWith("http://127.0.0.1:40000", {
        apply_live: false,
      })
    );
    expect(toast).toHaveBeenCalledWith("已同步 1/1 个 CLI 代理配置");
  });

  it("restarts gateway and syncs cli proxy when port changes while running", async () => {
    const base = createSettings({ preferred_port: 37123 });
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: base,
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    vi.mocked(gatewayCheckPortAvailable).mockResolvedValue(true);
    vi.mocked(gatewayStop).mockResolvedValue({
      running: false,
      port: null,
      base_url: null,
      listen_addr: null,
    } as any);
    vi.mocked(gatewayStart).mockResolvedValue({
      running: true,
      port: 40001,
      base_url: null,
      listen_addr: null,
    } as any);

    vi.mocked(cliProxySyncEnabled).mockResolvedValue([{ ok: true }, { ok: false }] as any);

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync.mockResolvedValue(createSettings({ preferred_port: 40000 }));
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() =>
      useSettingsPersistence({
        gateway: { running: true, port: 37123, base_url: null, listen_addr: null },
        about: {
          os: "mac",
          arch: "arm64",
          profile: "dev",
          app_version: "0.0.0",
          bundle_type: null,
          run_mode: "desktop",
        },
      })
    );

    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.commitNumberField({
        key: "preferred_port",
        next: 40000,
        min: 1024,
        max: 65535,
        invalidMessage: "bad",
      });
    });

    await waitFor(() => expect(mutation.mutateAsync).toHaveBeenCalled());
    await waitFor(() => expect(gatewayStop).toHaveBeenCalled());
    await waitFor(() => expect(gatewayStart).toHaveBeenCalled());

    expect(cliProxySyncEnabled).toHaveBeenCalledWith("http://127.0.0.1:40001", {
      apply_live: true,
    });
    expect(toast).toHaveBeenCalledWith("端口被占用，已切换到 40001");
    expect(toast).toHaveBeenCalledWith("已同步 1/2 个 CLI 代理配置");
  });

  it("toasts when gateway restart stop/start fails and uses base_url fallback when present", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings({ preferred_port: 37123 }),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    vi.mocked(gatewayCheckPortAvailable).mockResolvedValue(true);

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync.mockImplementation(async (payload: any) => {
      return createSettings({ preferred_port: payload?.preferredPort ?? 37123 });
    });
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    // 1) stop fails
    vi.mocked(gatewayStop).mockResolvedValue(null as any);
    vi.mocked(gatewayStart).mockResolvedValue(null as any);

    const { result } = renderHook(() =>
      useSettingsPersistence({
        gateway: { running: true, port: 37123, base_url: null, listen_addr: null },
        about: {
          os: "mac",
          arch: "arm64",
          profile: "dev",
          app_version: "0.0.0",
          bundle_type: null,
          run_mode: "desktop",
        },
      })
    );
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.commitNumberField({
        key: "preferred_port",
        next: 40000,
        min: 1024,
        max: 65535,
        invalidMessage: "bad",
      });
    });

    await waitFor(() => expect(toast).toHaveBeenCalledWith("自动重启失败：无法停止网关"));

    // 2) start fails
    vi.mocked(gatewayStop).mockResolvedValue({
      running: false,
      port: null,
      base_url: null,
      listen_addr: null,
    } as any);
    vi.mocked(gatewayStart).mockResolvedValue(null as any);

    act(() => {
      result.current.commitNumberField({
        key: "preferred_port",
        next: 40001,
        min: 1024,
        max: 65535,
        invalidMessage: "bad",
      });
    });

    await waitFor(() => expect(toast).toHaveBeenCalledWith("自动重启失败：无法启动网关"));

    // 3) happy path toast when started.port matches desired and base_url is provided
    vi.mocked(gatewayStop).mockResolvedValue({
      running: false,
      port: null,
      base_url: null,
      listen_addr: null,
    } as any);
    vi.mocked(gatewayStart).mockResolvedValue({
      running: true,
      port: 40002,
      base_url: "http://127.0.0.1:40002",
      listen_addr: null,
    } as any);
    vi.mocked(cliProxySyncEnabled).mockResolvedValue([{ ok: true }] as any);

    act(() => {
      result.current.commitNumberField({
        key: "preferred_port",
        next: 40002,
        min: 1024,
        max: 65535,
        invalidMessage: "bad",
      });
    });

    await waitFor(() =>
      expect(cliProxySyncEnabled).toHaveBeenCalledWith("http://127.0.0.1:40002", {
        apply_live: true,
      })
    );
    expect(toast).toHaveBeenCalledWith("网关已按新端口重启");
  });

  it("uses preferred_port fallback when restarted gateway returns null port/base_url", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings({ preferred_port: 37123 }),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    vi.mocked(gatewayCheckPortAvailable).mockResolvedValue(true);
    vi.mocked(gatewayStop).mockResolvedValue({
      running: false,
      port: null,
      base_url: null,
      listen_addr: null,
    } as any);
    vi.mocked(gatewayStart).mockResolvedValue({
      running: true,
      port: null,
      base_url: null,
      listen_addr: null,
    } as any);
    vi.mocked(cliProxySyncEnabled).mockResolvedValue([{ ok: true }] as any);

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync.mockResolvedValue(createSettings({ preferred_port: 40000 }));
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() =>
      useSettingsPersistence({
        gateway: { running: true, port: 37123, base_url: null, listen_addr: null },
        about: {
          os: "mac",
          arch: "arm64",
          profile: "dev",
          app_version: "0.0.0",
          bundle_type: null,
          run_mode: "desktop",
        },
      })
    );
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.commitNumberField({
        key: "preferred_port",
        next: 40000,
        min: 1024,
        max: 65535,
        invalidMessage: "bad",
      });
    });

    await waitFor(() => expect(gatewayStart).toHaveBeenCalled());
    await waitFor(() =>
      expect(cliProxySyncEnabled).toHaveBeenCalledWith("http://127.0.0.1:40000", {
        apply_live: true,
      })
    );
  });

  it("keeps saved state when post-save sync throws", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings({ preferred_port: 37123 }),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    vi.mocked(gatewayCheckPortAvailable).mockResolvedValue(true);
    vi.mocked(cliProxySyncEnabled).mockRejectedValue(new Error("sync boom"));

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync.mockResolvedValue(createSettings({ preferred_port: 40000 }));
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() =>
      useSettingsPersistence({
        gateway: { running: false, port: null, base_url: null, listen_addr: null },
        about: null,
      })
    );
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.commitNumberField({
        key: "preferred_port",
        next: 40000,
        min: 1024,
        max: 65535,
        invalidMessage: "bad",
      });
    });

    await waitFor(() => expect(mutation.mutateAsync).toHaveBeenCalled());
    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith("设置已保存，但后续动作失败：请检查网关和 CLI 代理状态")
    );
    expect(result.current.port).toBe(40000);
    expect(toast).not.toHaveBeenCalledWith("更新设置失败：请稍后重试");
  });

  it("toasts and reverts when settings_set throws", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings(),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    vi.mocked(gatewayCheckPortAvailable).mockResolvedValue(true);

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync.mockRejectedValue(new Error("boom"));
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() =>
      useSettingsPersistence({
        gateway: { running: false, port: null, base_url: null, listen_addr: null },
        about: null,
      })
    );
    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.requestPersist({ tray_enabled: false } as any);
    });

    await waitFor(() => expect(toast).toHaveBeenCalledWith("更新设置失败：请稍后重试"));
    expect(logToConsole).toHaveBeenCalledWith("error", "更新设置失败", { error: "Error: boom" });
  });

  it("toasts portable warning when enabling auto start", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createSettings({ auto_start: false }),
      isLoading: false,
      isError: false,
      error: null,
    } as any);

    const mutation = { mutateAsync: vi.fn() };
    mutation.mutateAsync.mockResolvedValue(createSettings({ auto_start: true }));
    vi.mocked(useSettingsSetMutation).mockReturnValue(mutation as any);

    const { result } = renderHook(() =>
      useSettingsPersistence({
        gateway: { running: false, port: null, base_url: null, listen_addr: null },
        about: {
          os: "mac",
          arch: "arm64",
          profile: "dev",
          app_version: "0.0.0",
          bundle_type: null,
          run_mode: "portable",
        },
      })
    );

    await waitFor(() => expect(result.current.settingsReady).toBe(true));

    act(() => {
      result.current.requestPersist({ auto_start: true });
    });

    await waitFor(() => expect(mutation.mutateAsync).toHaveBeenCalled());
    expect(toast).toHaveBeenCalledWith("portable 模式开启自启：移动应用位置可能导致自启失效");
  });
});
