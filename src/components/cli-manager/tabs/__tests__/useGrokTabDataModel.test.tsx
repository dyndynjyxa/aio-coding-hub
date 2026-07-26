import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import type { GrokConfigState, GrokProxyPreferences } from "../../../../services/cli/cliManager";
import {
  useCliManagerGrokConfigQuery,
  useCliManagerGrokConfigSetMutation,
  useCliManagerGrokInfoQuery,
} from "../../../../query/cliManager";
import { useCliEnvConflictsQuery } from "../../../../query/cliProxy";
import { openDesktopPath } from "../../../../services/desktop/opener";
import { logToConsole } from "../../../../services/consoleLog";
import { useGrokTabDataModel } from "../useGrokTabDataModel";

const DEFAULT_CONFIG: GrokConfigState = {
  config_path: "/Users/test/.grok/config.toml",
  file_exists: true,
  preferences: {
    model_id: "grok-existing",
    api_backend: "chat_completions",
    context_window: null,
    telemetry: null,
    supports_backend_search: null,
  },
  aio_preferences: null,
  effective_preferences: {
    model_id: "grok-existing",
    api_backend: "chat_completions",
    context_window: null,
    telemetry: null,
    supports_backend_search: null,
  },
  preference_source: "existing_config",
  default_profile: "grok-existing",
  session_summary_profile: null,
  web_search_profile: null,
  image_description_profile: null,
  policy_files: [],
};

vi.mock("../../../../query/cliManager", async () => {
  const actual = await vi.importActual<typeof import("../../../../query/cliManager")>(
    "../../../../query/cliManager"
  );
  return {
    ...actual,
    useCliManagerGrokInfoQuery: vi.fn(),
    useCliManagerGrokConfigQuery: vi.fn(),
    useCliManagerGrokConfigSetMutation: vi.fn(),
  };
});

vi.mock("../../../../query/cliProxy", async () => {
  const actual = await vi.importActual<typeof import("../../../../query/cliProxy")>(
    "../../../../query/cliProxy"
  );
  return { ...actual, useCliEnvConflictsQuery: vi.fn() };
});

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { success: vi.fn(), error: vi.fn() }),
}));
vi.mock("../../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));
vi.mock("../../../../services/desktop/opener", async () => {
  const actual = await vi.importActual<typeof import("../../../../services/desktop/opener")>(
    "../../../../services/desktop/opener"
  );
  return { ...actual, openDesktopPath: vi.fn() };
});

function mockQueries(config: GrokConfigState = DEFAULT_CONFIG) {
  vi.mocked(useCliManagerGrokInfoQuery).mockReturnValue({
    data: {
      found: true,
      executable_path: "/usr/local/bin/grok",
      version: "0.2.93",
      error: null,
      shell: "zsh",
      resolved_via: "path",
    },
    isFetching: false,
    refetch: vi.fn(),
  } as never);
  vi.mocked(useCliManagerGrokConfigQuery).mockReturnValue({
    data: config,
    isFetching: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
  } as never);
  vi.mocked(useCliManagerGrokConfigSetMutation).mockReturnValue({
    isPending: false,
    mutateAsync: vi.fn(),
  } as never);
  vi.mocked(useCliEnvConflictsQuery).mockReturnValue({
    data: [],
    isFetching: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
  } as never);
}

describe("components/cli-manager/tabs/useGrokTabDataModel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockQueries();
  });

  it.each([
    {
      source: "aio_settings" as const,
      aio: {
        model_id: "grok-aio",
        api_backend: "responses" as const,
        context_window: null,
        telemetry: null,
        supports_backend_search: null,
      },
      effective: {
        model_id: "grok-aio",
        api_backend: "responses" as const,
        context_window: null,
        telemetry: null,
        supports_backend_search: null,
      },
    },
    {
      source: "existing_config" as const,
      aio: null,
      effective: {
        model_id: "grok-existing",
        api_backend: "chat_completions" as const,
        context_window: null,
        telemetry: null,
        supports_backend_search: null,
      },
    },
    {
      source: "fallback" as const,
      aio: null,
      effective: {
        model_id: "grok-build",
        api_backend: "responses" as const,
        context_window: null,
        telemetry: null,
        supports_backend_search: null,
      },
    },
  ])("使用 $source 的 effective preferences 初始化草稿", async ({ source, aio, effective }) => {
    mockQueries({
      ...DEFAULT_CONFIG,
      aio_preferences: aio,
      effective_preferences: effective,
      preference_source: source,
    });

    const { result } = renderHook(() => useGrokTabDataModel({ enabled: true }));

    await waitFor(() => expect(result.current.preferencesDraft).toEqual(effective));
    expect(useCliManagerGrokInfoQuery).toHaveBeenCalledWith({ enabled: true });
    expect(useCliManagerGrokConfigQuery).toHaveBeenCalledWith({ enabled: true });
    expect(useCliEnvConflictsQuery).toHaveBeenCalledWith("grok", { enabled: true });
  });

  it("用户编辑后不被外部配置刷新或短暂错误覆盖", async () => {
    const { result, rerender } = renderHook(() => useGrokTabDataModel({ enabled: true }));
    await waitFor(() => expect(result.current.preferencesDraft.model_id).toBe("grok-existing"));

    act(() => result.current.setModelIdDraft("grok-unsaved"));
    vi.mocked(useCliManagerGrokConfigQuery).mockReturnValue({
      data: {
        ...DEFAULT_CONFIG,
        effective_preferences: {
          model_id: "grok-external",
          api_backend: "responses",
          context_window: null,
          telemetry: null,
          supports_backend_search: null,
        },
      },
      isFetching: false,
      isError: false,
      error: null,
      refetch: vi.fn(),
    } as never);
    rerender();

    await waitFor(() => expect(result.current.preferencesDraft.model_id).toBe("grok-unsaved"));

    vi.mocked(useCliManagerGrokConfigQuery).mockReturnValue({
      data: null,
      isFetching: false,
      isError: true,
      error: new Error("temporary read failure"),
      refetch: vi.fn(),
    } as never);
    rerender();

    await waitFor(() => expect(result.current.preferencesDraft.model_id).toBe("grok-unsaved"));
  });

  it("映射安装、配置和环境诊断", async () => {
    const { result } = renderHook(() => useGrokTabDataModel({ enabled: true }));

    await waitFor(() => expect(result.current.grokAvailable).toBe("available"));
    expect(result.current.grokConfig).toEqual(DEFAULT_CONFIG);
    expect(result.current.envConflicts).toEqual([]);
  });

  it("无效配置保持空草稿并阻止保存，不按缺失配置回退", async () => {
    const mutateAsync = vi.fn();
    vi.mocked(useCliManagerGrokConfigQuery).mockReturnValue({
      data: null,
      isFetching: false,
      isError: true,
      error: new Error("GROK_CONFIG_INVALID: invalid config.toml"),
      refetch: vi.fn(),
    } as never);
    vi.mocked(useCliManagerGrokConfigSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync,
    } as never);

    const { result } = renderHook(() => useGrokTabDataModel({ enabled: true }));

    expect(result.current.preferencesDraft).toEqual({
      model_id: "",
      api_backend: "responses",
      context_window: null,
      telemetry: null,
      supports_backend_search: null,
    });
    expect(result.current.grokConfigError).toBe("GROK_CONFIG_INVALID: invalid config.toml");

    await act(async () => result.current.persistModelId(""));
    expect(mutateAsync).not.toHaveBeenCalled();
  });

  it("模型 ID 为空时明确提示且不提交", async () => {
    const mutateAsync = vi.fn();
    vi.mocked(useCliManagerGrokConfigSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync,
    } as never);

    const { result } = renderHook(() => useGrokTabDataModel({ enabled: true }));
    await waitFor(() => expect(result.current.preferencesDraft.model_id).toBe("grok-existing"));

    await act(async () => result.current.persistModelId("   "));

    expect(mutateAsync).not.toHaveBeenCalled();
    expect(toast).toHaveBeenCalledWith("模型 ID 不能为空");
  });

  it("所有保存入口保留其他偏好字段并拒绝非正 safe integer", async () => {
    const preferences: GrokProxyPreferences = {
      model_id: "grok-existing",
      api_backend: "chat_completions",
      context_window: 500_000,
      telemetry: false,
      supports_backend_search: false,
    };
    const config: GrokConfigState = {
      ...DEFAULT_CONFIG,
      preferences,
      effective_preferences: preferences,
    };
    const mutateAsync = vi.fn(async (next: GrokProxyPreferences) => ({
      ...config,
      aio_preferences: next,
      effective_preferences: next,
      preference_source: "aio_settings" as const,
    }));
    mockQueries(config);
    vi.mocked(useCliManagerGrokConfigSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync,
    } as never);

    const { result } = renderHook(() => useGrokTabDataModel({ enabled: true }));
    await waitFor(() => expect(result.current.preferencesDraft).toEqual(preferences));

    await act(async () => result.current.persistModelId("grok-new"));
    expect(mutateAsync).toHaveBeenLastCalledWith({
      ...preferences,
      model_id: "grok-new",
    });

    await act(async () => result.current.persistApiBackend("responses"));
    expect(mutateAsync).toHaveBeenLastCalledWith({
      ...preferences,
      model_id: "grok-new",
      api_backend: "responses",
    });

    await act(async () => result.current.persistContextWindow(Number.MAX_SAFE_INTEGER + 1));
    expect(mutateAsync).toHaveBeenLastCalledWith({
      ...preferences,
      model_id: "grok-new",
      api_backend: "responses",
      context_window: null,
    });

    act(() => result.current.setContextWindowDraft(1.5));
    expect(result.current.preferencesDraft.context_window).toBeNull();
  });

  it("变更后立即持久化并刷新诊断，打开配置目录", async () => {
    const infoRefetch = vi.fn().mockResolvedValue({ data: null });
    const configRefetch = vi.fn().mockResolvedValue({ data: null });
    const envRefetch = vi.fn().mockResolvedValue({ data: null });
    const mutateAsync = vi.fn().mockResolvedValue({
      ...DEFAULT_CONFIG,
      aio_preferences: {
        model_id: "grok-4.1-fast",
        api_backend: "responses",
        context_window: null,
        telemetry: null,
        supports_backend_search: null,
      },
      effective_preferences: {
        model_id: "grok-4.1-fast",
        api_backend: "responses",
        context_window: null,
        telemetry: null,
        supports_backend_search: null,
      },
      preference_source: "aio_settings",
    });

    vi.mocked(useCliManagerGrokInfoQuery).mockReturnValue({
      data: {
        found: true,
        executable_path: "/usr/local/bin/grok",
        version: "0.2.93",
        error: null,
        shell: "zsh",
        resolved_via: "path",
      },
      isFetching: false,
      refetch: infoRefetch,
    } as never);
    vi.mocked(useCliManagerGrokConfigQuery).mockReturnValue({
      data: DEFAULT_CONFIG,
      isFetching: false,
      isError: false,
      error: null,
      refetch: configRefetch,
    } as never);
    vi.mocked(useCliManagerGrokConfigSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync,
    } as never);
    vi.mocked(useCliEnvConflictsQuery).mockReturnValue({
      data: [],
      isFetching: false,
      isError: false,
      error: null,
      refetch: envRefetch,
    } as never);

    const { result, rerender } = renderHook(() => useGrokTabDataModel({ enabled: true }));
    await waitFor(() => expect(result.current.preferencesDraft.model_id).toBe("grok-existing"));

    act(() => result.current.setModelIdDraft("  grok-4.1-fast  "));
    act(() => result.current.setApiBackendDraft("responses"));
    expect(mutateAsync).not.toHaveBeenCalled();

    await act(async () => result.current.persistModelId("grok-4.1-fast"));
    expect(mutateAsync).toHaveBeenCalledWith({
      model_id: "grok-4.1-fast",
      api_backend: "responses",
      context_window: null,
      telemetry: null,
      supports_backend_search: null,
    });
    expect(result.current.preferencesDraft).toEqual({
      model_id: "grok-4.1-fast",
      api_backend: "responses",
      context_window: null,
      telemetry: null,
      supports_backend_search: null,
    });
    expect(toast).toHaveBeenCalledWith("已保存 Grok 网关偏好");

    vi.mocked(useCliManagerGrokConfigQuery).mockReturnValue({
      data: {
        ...DEFAULT_CONFIG,
        effective_preferences: {
          model_id: "grok-after-save",
          api_backend: "chat_completions",
          context_window: null,
          telemetry: null,
          supports_backend_search: null,
        },
      },
      isFetching: false,
      isError: false,
      error: null,
      refetch: configRefetch,
    } as never);
    rerender();
    await waitFor(() =>
      expect(result.current.preferencesDraft).toEqual({
        model_id: "grok-after-save",
        api_backend: "chat_completions",
        context_window: null,
        telemetry: null,
        supports_backend_search: null,
      })
    );

    await act(async () => result.current.refreshGrok());
    expect(infoRefetch).toHaveBeenCalledTimes(1);
    expect(configRefetch).toHaveBeenCalledTimes(1);
    expect(envRefetch).toHaveBeenCalledTimes(1);

    await act(async () => result.current.openGrokConfigDir());
    expect(openDesktopPath).toHaveBeenCalledWith("/Users/test/.grok");
  });

  it("保存或打开目录失败时保留草稿并给出可诊断错误", async () => {
    const mutateAsync = vi.fn().mockRejectedValue(new Error("GROK_SAVE_DENIED: denied"));
    vi.mocked(useCliManagerGrokConfigSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync,
    } as never);
    vi.mocked(openDesktopPath).mockRejectedValueOnce(new Error("open failed"));

    const { result, rerender } = renderHook(() => useGrokTabDataModel({ enabled: true }));
    await waitFor(() => expect(result.current.preferencesDraft.model_id).toBe("grok-existing"));

    act(() => result.current.setModelIdDraft("grok-unsaved"));
    await act(async () => result.current.persistModelId("grok-unsaved"));

    expect(result.current.preferencesDraft.model_id).toBe("grok-unsaved");
    expect(toast).toHaveBeenCalledWith("保存 Grok 网关偏好失败（code GROK_SAVE_DENIED）：denied");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "保存 Grok 网关偏好失败",
      expect.objectContaining({ error_code: "GROK_SAVE_DENIED" })
    );

    vi.mocked(useCliManagerGrokConfigQuery).mockReturnValue({
      data: {
        ...DEFAULT_CONFIG,
        effective_preferences: {
          model_id: "grok-external",
          api_backend: "responses",
          context_window: null,
          telemetry: null,
          supports_backend_search: null,
        },
      },
      isFetching: false,
      isError: false,
      error: null,
      refetch: vi.fn(),
    } as never);
    rerender();
    await waitFor(() => expect(result.current.preferencesDraft.model_id).toBe("grok-unsaved"));

    await act(async () => result.current.openGrokConfigDir());
    expect(toast).toHaveBeenCalledWith("打开 Grok 配置目录失败：请查看控制台日志");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "打开 Grok 配置目录失败",
      expect.objectContaining({ error: "open failed" })
    );
  });

  it("保存进行中产生的新编辑不被较早的成功响应覆盖", async () => {
    let resolveSave: ((value: GrokConfigState) => void) | undefined;
    const saveResult = new Promise<GrokConfigState>((resolve) => {
      resolveSave = resolve;
    });
    vi.mocked(useCliManagerGrokConfigSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn().mockReturnValue(saveResult),
    } as never);
    const { result } = renderHook(() => useGrokTabDataModel({ enabled: true }));
    await waitFor(() => expect(result.current.preferencesDraft.model_id).toBe("grok-existing"));

    act(() => result.current.setModelIdDraft("grok-submitted"));
    let persistPromise: Promise<void> | undefined;
    act(() => {
      persistPromise = result.current.persistModelId("grok-submitted");
    });
    act(() => result.current.setModelIdDraft("grok-edited-later"));

    await act(async () => {
      resolveSave?.({
        ...DEFAULT_CONFIG,
        aio_preferences: {
          model_id: "grok-submitted",
          api_backend: "chat_completions",
          context_window: null,
          telemetry: null,
          supports_backend_search: null,
        },
        effective_preferences: {
          model_id: "grok-submitted",
          api_backend: "chat_completions",
          context_window: null,
          telemetry: null,
          supports_backend_search: null,
        },
        preference_source: "aio_settings",
      });
      await persistPromise;
    });

    expect(result.current.preferencesDraft.model_id).toBe("grok-edited-later");
  });
});
