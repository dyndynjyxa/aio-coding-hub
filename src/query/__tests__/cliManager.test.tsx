import { act, renderHook, waitFor } from "@testing-library/react";
import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import {
  type ClaudeCliInfo,
  type ClaudeHooksState,
  type ClaudeSettingsState,
  type CodexConfigState,
  type CodexConfigTomlState,
  type CodexModelCatalogState,
  type SimpleCliInfo,
  type GrokConfigState,
  cliManagerClaudeHooksGet,
  cliManagerClaudeHooksSet,
  cliManagerClaudeInfoGet,
  cliManagerClaudeSettingsGet,
  cliManagerClaudeSettingsSet,
  cliManagerCodexConfigGet,
  cliManagerCodexConfigSet,
  cliManagerCodexConfigTomlGet,
  cliManagerCodexConfigTomlSet,
  cliManagerCodexInfoGet,
  cliManagerCodexModelCatalogGet,
  cliManagerGeminiInfoGet,
  cliManagerGrokConfigGet,
  cliManagerGrokConfigSet,
  cliManagerGrokInfoGet,
} from "../../services/cli/cliManager";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { cliManagerKeys } from "../keys";
import {
  pickCliAvailable,
  useCliManagerClaudeHooksQuery,
  useCliManagerClaudeHooksSetMutation,
  useCliManagerClaudeInfoQuery,
  useCliManagerClaudeSettingsQuery,
  useCliManagerClaudeSettingsSetMutation,
  useCliManagerCodexConfigQuery,
  useCliManagerCodexConfigSetMutation,
  useCliManagerCodexConfigTomlQuery,
  useCliManagerCodexConfigTomlSetMutation,
  useCliManagerCodexInfoQuery,
  useCliManagerCodexModelCatalogQuery,
  useCliManagerCodexModelCatalogRefresh,
  useCliManagerGeminiInfoQuery,
  useCliManagerGrokConfigQuery,
  useCliManagerGrokConfigSetMutation,
  useCliManagerGrokInfoQuery,
} from "../cliManager";

vi.mock("../../services/cli/cliManager", async () => {
  const actual = await vi.importActual<typeof import("../../services/cli/cliManager")>(
    "../../services/cli/cliManager"
  );
  return {
    ...actual,
    cliManagerClaudeInfoGet: vi.fn(),
    cliManagerClaudeHooksGet: vi.fn(),
    cliManagerClaudeHooksSet: vi.fn(),
    cliManagerClaudeSettingsGet: vi.fn(),
    cliManagerClaudeSettingsSet: vi.fn(),
    cliManagerCodexInfoGet: vi.fn(),
    cliManagerCodexConfigGet: vi.fn(),
    cliManagerCodexConfigSet: vi.fn(),
    cliManagerCodexConfigTomlGet: vi.fn(),
    cliManagerCodexConfigTomlSet: vi.fn(),
    cliManagerCodexModelCatalogGet: vi.fn(),
    cliManagerGeminiInfoGet: vi.fn(),
    cliManagerGrokInfoGet: vi.fn(),
    cliManagerGrokConfigGet: vi.fn(),
    cliManagerGrokConfigSet: vi.fn(),
  };
});

function makeSimpleCliInfo(overrides: Partial<SimpleCliInfo> = {}): SimpleCliInfo {
  return {
    found: true,
    executable_path: "/usr/bin/codex",
    version: "0.0.0",
    error: null,
    shell: "zsh",
    resolved_via: "PATH",
    ...overrides,
  };
}

function makeClaudeCliInfo(overrides: Partial<ClaudeCliInfo> = {}): ClaudeCliInfo {
  return {
    ...makeSimpleCliInfo(),
    config_dir: "/tmp/.claude",
    settings_path: "/tmp/.claude/settings.json",
    mcp_timeout_ms: null,
    disable_error_reporting: false,
    ...overrides,
  };
}

function makeClaudeSettingsState(
  overrides: Partial<ClaudeSettingsState> = {}
): ClaudeSettingsState {
  return {
    config_dir: "/tmp/.claude",
    settings_path: "/tmp/.claude/settings.json",
    exists: true,
    model: null,
    output_style: null,
    language: null,
    always_thinking_enabled: null,
    show_turn_duration: null,
    spinner_tips_enabled: null,
    terminal_progress_bar_enabled: null,
    respect_gitignore: null,
    disable_git_participant: false,
    permissions_allow: [],
    permissions_ask: [],
    permissions_deny: [],
    env_mcp_timeout_ms: null,
    env_mcp_tool_timeout_ms: null,
    env_experimental_agent_teams: false,
    env_claude_code_auto_compact_window: null,
    env_disable_background_tasks: false,
    env_disable_terminal_title: false,
    env_claude_bash_no_login: false,
    env_claude_code_attribution_header: false,
    env_claude_code_blocking_limit_override: null,
    env_claude_code_max_output_tokens: null,
    env_enable_experimental_mcp_cli: false,
    env_enable_tool_search: false,
    env_max_mcp_output_tokens: null,
    env_claude_code_disable_nonessential_traffic: false,
    env_claude_code_disable_1m_context: false,
    env_claude_code_proxy_resolves_hosts: false,
    env_claude_code_skip_prompt_history: false,
    ...overrides,
  };
}

function makeClaudeHooksState(overrides: Partial<ClaudeHooksState> = {}): ClaudeHooksState {
  return {
    settings_path: "/tmp/.claude/settings.json",
    groups: [],
    ...overrides,
  };
}

function makeCodexConfigState(overrides: Partial<CodexConfigState> = {}): CodexConfigState {
  return {
    config_dir: "/tmp/.codex",
    config_path: "/tmp/.codex/config.toml",
    user_home_default_dir: "/tmp/.codex",
    user_home_default_path: "/tmp/.codex/config.toml",
    follow_codex_home_dir: "/tmp/.codex",
    follow_codex_home_path: "/tmp/.codex/config.toml",
    can_open_config_dir: true,
    exists: true,
    model: null,
    approval_policy: null,
    sandbox_mode: null,
    model_reasoning_effort: null,
    plan_mode_reasoning_effort: null,
    web_search: null,
    personality: null,
    model_context_window: null,
    model_auto_compact_token_limit: null,
    service_tier: null,
    sandbox_workspace_write_network_access: null,
    features_unified_exec: null,
    features_shell_snapshot: null,
    features_apply_patch_freeform: null,
    features_shell_tool: null,
    features_exec_policy: null,
    features_remote_compaction: null,
    features_fast_mode: null,
    features_responses_websockets_v2: null,
    features_multi_agent: null,
    ...overrides,
  };
}

function makeCodexConfigTomlState(
  overrides: Partial<CodexConfigTomlState> = {}
): CodexConfigTomlState {
  return {
    config_path: "/tmp/.codex/config.toml",
    exists: true,
    toml: "",
    ...overrides,
  };
}

function makeCodexModelCatalogState(
  overrides: Partial<CodexModelCatalogState> = {}
): CodexModelCatalogState {
  return {
    status: "ready",
    issue: null,
    snapshot: {
      config_path: "/tmp/.codex/config.toml",
      executable_path: "/usr/bin/codex",
      cli_version: "0.0.0",
    },
    models: [],
    ...overrides,
  };
}

function makeGrokConfigState(overrides: Partial<GrokConfigState> = {}): GrokConfigState {
  return {
    config_path: "/tmp/.grok/config.toml",
    file_exists: false,
    preferences: {
      model_id: "grok-build",
      api_backend: "responses",
      context_window: null,
      telemetry: null,
      supports_backend_search: null,
    },
    aio_preferences: null,
    effective_preferences: {
      model_id: "grok-build",
      api_backend: "responses",
      context_window: null,
      telemetry: null,
      supports_backend_search: null,
    },
    preference_source: "fallback",
    default_profile: null,
    session_summary_profile: null,
    web_search_profile: null,
    image_description_profile: null,
    policy_files: [],
    ...overrides,
  };
}

describe("query/cliManager", () => {
  it("calls cliManager queries with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(cliManagerClaudeInfoGet).mockResolvedValue(makeClaudeCliInfo());
    vi.mocked(cliManagerClaudeHooksGet).mockResolvedValue(makeClaudeHooksState());
    vi.mocked(cliManagerClaudeSettingsGet).mockResolvedValue(makeClaudeSettingsState());
    vi.mocked(cliManagerCodexInfoGet).mockResolvedValue(makeSimpleCliInfo());
    vi.mocked(cliManagerCodexModelCatalogGet).mockResolvedValue(makeCodexModelCatalogState());
    vi.mocked(cliManagerCodexConfigGet).mockResolvedValue(makeCodexConfigState());
    vi.mocked(cliManagerCodexConfigTomlGet).mockResolvedValue(makeCodexConfigTomlState());
    vi.mocked(cliManagerGeminiInfoGet).mockResolvedValue(makeSimpleCliInfo());
    vi.mocked(cliManagerGrokInfoGet).mockResolvedValue(
      makeSimpleCliInfo({ executable_path: "/usr/bin/grok" })
    );
    vi.mocked(cliManagerGrokConfigGet).mockResolvedValue(makeGrokConfigState());

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useCliManagerClaudeInfoQuery(), { wrapper });
    renderHook(() => useCliManagerClaudeHooksQuery(), { wrapper });
    renderHook(() => useCliManagerClaudeSettingsQuery(), { wrapper });
    renderHook(() => useCliManagerCodexInfoQuery(), { wrapper });
    renderHook(
      () =>
        useCliManagerCodexModelCatalogQuery({
          snapshot: {
            configPath: "/tmp/.codex/config.toml",
            executablePath: "/usr/bin/codex",
            cliVersion: "0.0.0",
          },
        }),
      { wrapper }
    );
    renderHook(() => useCliManagerCodexConfigQuery(), { wrapper });
    renderHook(() => useCliManagerCodexConfigTomlQuery(), { wrapper });
    renderHook(() => useCliManagerGeminiInfoQuery(), { wrapper });
    renderHook(() => useCliManagerGrokInfoQuery(), { wrapper });
    renderHook(() => useCliManagerGrokConfigQuery(), { wrapper });

    await waitFor(() => {
      expect(cliManagerClaudeInfoGet).toHaveBeenCalled();
      expect(cliManagerClaudeHooksGet).toHaveBeenCalled();
      expect(cliManagerClaudeSettingsGet).toHaveBeenCalled();
      expect(cliManagerCodexInfoGet).toHaveBeenCalled();
      expect(cliManagerCodexModelCatalogGet).toHaveBeenCalled();
      expect(cliManagerCodexConfigGet).toHaveBeenCalled();
      expect(cliManagerCodexConfigTomlGet).toHaveBeenCalled();
      expect(cliManagerGeminiInfoGet).toHaveBeenCalled();
      expect(cliManagerGrokInfoGet).toHaveBeenCalled();
      expect(cliManagerGrokConfigGet).toHaveBeenCalled();
    });
  });

  it("useCliManagerClaudeInfoQuery enters error state when service rejects", async () => {
    setTauriRuntime();

    vi.mocked(cliManagerClaudeInfoGet).mockRejectedValue(new Error("cli manager query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useCliManagerClaudeInfoQuery(), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("useCliManagerCodexModelCatalogQuery enters error state when service rejects", async () => {
    setTauriRuntime();

    vi.mocked(cliManagerCodexModelCatalogGet).mockRejectedValue(new Error("catalog query boom"));

    const client = new QueryClient({
      defaultOptions: { queries: { retry: 1, retryDelay: 0 } },
    });
    const wrapper = createQueryWrapper(client);

    const { result, unmount } = renderHook(
      () =>
        useCliManagerCodexModelCatalogQuery({
          snapshot: {
            configPath: "/tmp/.codex/config.toml",
            executablePath: "/usr/bin/codex",
            cliVersion: "0.0.0",
          },
        }),
      { wrapper }
    );

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
    expect(cliManagerCodexModelCatalogGet).toHaveBeenCalledTimes(1);

    unmount();
    const remounted = renderHook(
      () =>
        useCliManagerCodexModelCatalogQuery({
          snapshot: {
            configPath: "/tmp/.codex/config.toml",
            executablePath: "/usr/bin/codex",
            cliVersion: "0.0.0",
          },
        }),
      { wrapper }
    );
    expect(remounted.result.current.isError).toBe(true);
    expect(cliManagerCodexModelCatalogGet).toHaveBeenCalledTimes(1);
  });

  it("respects options.enabled=false for all cliManager info/config queries", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useCliManagerClaudeInfoQuery({ enabled: false }), { wrapper });
    renderHook(() => useCliManagerClaudeHooksQuery({ enabled: false }), { wrapper });
    renderHook(() => useCliManagerClaudeSettingsQuery({ enabled: false }), { wrapper });
    renderHook(() => useCliManagerCodexInfoQuery({ enabled: false }), { wrapper });
    renderHook(
      () =>
        useCliManagerCodexModelCatalogQuery({
          enabled: false,
          snapshot: {
            configPath: "/tmp/.codex/config.toml",
            executablePath: "/usr/bin/codex",
            cliVersion: "0.0.0",
          },
        }),
      { wrapper }
    );
    renderHook(() => useCliManagerCodexConfigQuery({ enabled: false }), { wrapper });
    renderHook(() => useCliManagerCodexConfigTomlQuery({ enabled: false }), { wrapper });
    renderHook(() => useCliManagerGeminiInfoQuery({ enabled: false }), { wrapper });
    renderHook(() => useCliManagerGrokInfoQuery({ enabled: false }), { wrapper });
    renderHook(() => useCliManagerGrokConfigQuery({ enabled: false }), { wrapper });

    await Promise.resolve();

    expect(cliManagerClaudeInfoGet).not.toHaveBeenCalled();
    expect(cliManagerClaudeHooksGet).not.toHaveBeenCalled();
    expect(cliManagerClaudeSettingsGet).not.toHaveBeenCalled();
    expect(cliManagerCodexInfoGet).not.toHaveBeenCalled();
    expect(cliManagerCodexModelCatalogGet).not.toHaveBeenCalled();
    expect(cliManagerCodexConfigGet).not.toHaveBeenCalled();
    expect(cliManagerCodexConfigTomlGet).not.toHaveBeenCalled();
    expect(cliManagerGeminiInfoGet).not.toHaveBeenCalled();
    expect(cliManagerGrokInfoGet).not.toHaveBeenCalled();
    expect(cliManagerGrokConfigGet).not.toHaveBeenCalled();
  });

  it("useCliManagerGrokConfigSetMutation updates and invalidates Grok config cache", async () => {
    setTauriRuntime();

    const updated = makeGrokConfigState({
      aio_preferences: {
        model_id: "grok-4-fast",
        api_backend: "chat_completions",
        context_window: null,
      },
      effective_preferences: {
        model_id: "grok-4-fast",
        api_backend: "chat_completions",
        context_window: null,
      },
      preference_source: "aio_settings",
    });
    vi.mocked(cliManagerGrokConfigSet).mockResolvedValue(updated);

    const client = createTestQueryClient();
    client.setQueryData(cliManagerKeys.grokConfig(), makeGrokConfigState());
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useCliManagerGrokConfigSetMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        model_id: "grok-4-fast",
        api_backend: "chat_completions",
        context_window: null,
      });
    });

    expect(client.getQueryData(cliManagerKeys.grokConfig())).toEqual(updated);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: cliManagerKeys.grokConfig() });
  });

  it("useCliManagerClaudeSettingsSetMutation updates cache and invalidates", async () => {
    setTauriRuntime();

    const updated = makeClaudeSettingsState({ model: "claude" });
    vi.mocked(cliManagerClaudeSettingsSet).mockResolvedValue(updated);

    const client = createTestQueryClient();
    client.setQueryData(cliManagerKeys.claudeSettings(), makeClaudeSettingsState({ model: "old" }));
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useCliManagerClaudeSettingsSetMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ model: "claude" });
    });

    expect(client.getQueryData(cliManagerKeys.claudeSettings())).toEqual(updated);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: cliManagerKeys.claudeSettings() });
  });

  it("useCliManagerClaudeHooksSetMutation updates cache and invalidates", async () => {
    setTauriRuntime();

    const updated = makeClaudeHooksState({
      groups: [
        {
          event: "PreToolUse",
          matcher: "",
          hooks: [{ hook_type: "command", command: "echo ok", timeout: null }],
        },
      ],
    });
    vi.mocked(cliManagerClaudeHooksSet).mockResolvedValue(updated);

    const client = createTestQueryClient();
    client.setQueryData(cliManagerKeys.claudeHooks(), makeClaudeHooksState());
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useCliManagerClaudeHooksSetMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ groups: updated.groups });
    });

    expect(client.getQueryData(cliManagerKeys.claudeHooks())).toEqual(updated);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: cliManagerKeys.claudeHooks() });
  });

  it("useCliManagerCodexConfigSetMutation updates cache and invalidates", async () => {
    setTauriRuntime();

    const updated = makeCodexConfigState({ model: "gpt-5" });
    vi.mocked(cliManagerCodexConfigSet).mockResolvedValue(updated);

    const client = createTestQueryClient();
    client.setQueryData(cliManagerKeys.codexConfig(), makeCodexConfigState({ model: "old" }));
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useCliManagerCodexConfigSetMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ model: "gpt-5" });
    });

    expect(client.getQueryData(cliManagerKeys.codexConfig())).toEqual(updated);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: cliManagerKeys.codexConfig() });
  });

  it("useCliManagerCodexConfigTomlSetMutation updates config cache and invalidates config+toml", async () => {
    setTauriRuntime();

    const updated = makeCodexConfigState({ model: "gpt-5" });
    vi.mocked(cliManagerCodexConfigTomlSet).mockResolvedValue(updated);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useCliManagerCodexConfigTomlSetMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ toml: 'model = "gpt-5"' });
    });

    expect(cliManagerCodexConfigTomlSet).toHaveBeenCalledWith('model = "gpt-5"');
    expect(client.getQueryData(cliManagerKeys.codexConfig())).toEqual(updated);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: cliManagerKeys.codexConfig() });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: cliManagerKeys.codexConfigToml() });
  });

  it("refreshes only the requested model catalog snapshot", async () => {
    setTauriRuntime();

    const oldSnapshot = {
      configPath: "/tmp/.codex/config.toml",
      executablePath: "/usr/bin/codex",
      cliVersion: "0.0.0",
    };
    const nextSnapshot = {
      configPath: "/tmp/next/.codex/config.toml",
      executablePath: "/opt/codex/bin/codex",
      cliVersion: "1.0.0",
    };
    const nextCatalog = makeCodexModelCatalogState({
      snapshot: {
        config_path: nextSnapshot.configPath,
        executable_path: nextSnapshot.executablePath,
        cli_version: nextSnapshot.cliVersion,
      },
    });
    vi.mocked(cliManagerCodexModelCatalogGet)
      .mockResolvedValueOnce(makeCodexModelCatalogState())
      .mockResolvedValueOnce(nextCatalog);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const { result, rerender } = renderHook(
      ({ snapshot }) => ({
        catalog: useCliManagerCodexModelCatalogQuery({ snapshot }),
        refresh: useCliManagerCodexModelCatalogRefresh(),
      }),
      { wrapper, initialProps: { snapshot: oldSnapshot } }
    );

    await waitFor(() => expect(result.current.catalog.isSuccess).toBe(true));
    vi.mocked(cliManagerCodexModelCatalogGet).mockClear();

    await act(async () => {
      await result.current.refresh(nextSnapshot);
    });

    expect(cliManagerCodexModelCatalogGet).toHaveBeenCalledTimes(1);
    rerender({ snapshot: nextSnapshot });
    await waitFor(() => expect(result.current.catalog.isSuccess).toBe(true));
    expect(cliManagerCodexModelCatalogGet).toHaveBeenCalledTimes(1);
    expect(client.getQueryData(cliManagerKeys.codexModelCatalog(nextSnapshot))).toEqual(
      nextCatalog
    );
    expect(client.getQueryState(cliManagerKeys.codexModelCatalog(oldSnapshot))?.isInvalidated).toBe(
      false
    );

    vi.mocked(cliManagerCodexModelCatalogGet).mockClear().mockResolvedValueOnce(nextCatalog);
    await act(async () => {
      await result.current.refresh(nextSnapshot);
    });
    expect(cliManagerCodexModelCatalogGet).toHaveBeenCalledTimes(1);
  });

  it("does not duplicate a failed target snapshot prefetch when the observer switches keys", async () => {
    setTauriRuntime();

    const oldSnapshot = {
      configPath: "/tmp/.codex/config.toml",
      executablePath: "/usr/bin/codex",
      cliVersion: "0.0.0",
    };
    const nextSnapshot = {
      configPath: "/tmp/next/.codex/config.toml",
      executablePath: "/opt/codex/bin/codex",
      cliVersion: "1.0.0",
    };
    vi.mocked(cliManagerCodexModelCatalogGet).mockResolvedValueOnce(makeCodexModelCatalogState());

    const client = new QueryClient({
      defaultOptions: { queries: { retry: 1, retryDelay: 0 } },
    });
    const wrapper = createQueryWrapper(client);
    const { result, rerender } = renderHook(
      ({ snapshot }) => ({
        catalog: useCliManagerCodexModelCatalogQuery({ snapshot }),
        refresh: useCliManagerCodexModelCatalogRefresh(),
      }),
      { wrapper, initialProps: { snapshot: oldSnapshot } }
    );

    await waitFor(() => expect(result.current.catalog.isSuccess).toBe(true));
    vi.mocked(cliManagerCodexModelCatalogGet)
      .mockReset()
      .mockRejectedValue(new Error("next snapshot failed"));

    await act(async () => {
      await result.current.refresh(nextSnapshot);
    });
    expect(cliManagerCodexModelCatalogGet).toHaveBeenCalledTimes(1);

    rerender({ snapshot: nextSnapshot });
    await waitFor(() => expect(result.current.catalog.isError).toBe(true));
    expect(cliManagerCodexModelCatalogGet).toHaveBeenCalledTimes(1);
  });

  it("refreshes an inactive failed target snapshot before the observer switches keys", async () => {
    setTauriRuntime();

    const oldSnapshot = {
      configPath: "/tmp/.codex/config.toml",
      executablePath: "/usr/bin/codex",
      cliVersion: "0.0.0",
    };
    const nextSnapshot = {
      configPath: "/tmp/next/.codex/config.toml",
      executablePath: "/opt/codex/bin/codex",
      cliVersion: "1.0.0",
    };
    const nextCatalog = makeCodexModelCatalogState({
      snapshot: {
        config_path: nextSnapshot.configPath,
        executable_path: nextSnapshot.executablePath,
        cli_version: nextSnapshot.cliVersion,
      },
    });
    vi.mocked(cliManagerCodexModelCatalogGet).mockResolvedValueOnce(makeCodexModelCatalogState());

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const { result, rerender } = renderHook(
      ({ snapshot }) => ({
        catalog: useCliManagerCodexModelCatalogQuery({ snapshot }),
        refresh: useCliManagerCodexModelCatalogRefresh(),
      }),
      { wrapper, initialProps: { snapshot: oldSnapshot } }
    );

    await waitFor(() => expect(result.current.catalog.isSuccess).toBe(true));
    vi.mocked(cliManagerCodexModelCatalogGet)
      .mockReset()
      .mockRejectedValueOnce(new Error("cached target failure"));
    await client.prefetchQuery({
      queryKey: cliManagerKeys.codexModelCatalog(nextSnapshot),
      queryFn: () => cliManagerCodexModelCatalogGet(),
      retry: false,
    });
    expect(client.getQueryState(cliManagerKeys.codexModelCatalog(nextSnapshot))?.status).toBe(
      "error"
    );

    vi.mocked(cliManagerCodexModelCatalogGet).mockReset().mockResolvedValue(nextCatalog);
    await act(async () => {
      await result.current.refresh(nextSnapshot);
    });
    expect(cliManagerCodexModelCatalogGet).toHaveBeenCalledTimes(1);

    rerender({ snapshot: nextSnapshot });
    await waitFor(() => expect(result.current.catalog.isSuccess).toBe(true));
    expect(cliManagerCodexModelCatalogGet).toHaveBeenCalledTimes(1);
    expect(client.getQueryData(cliManagerKeys.codexModelCatalog(nextSnapshot))).toEqual(
      nextCatalog
    );
  });

  it("reuses an in-flight stale target snapshot fetch during refresh", async () => {
    setTauriRuntime();

    const oldSnapshot = {
      configPath: "/tmp/.codex/config.toml",
      executablePath: "/usr/bin/codex",
      cliVersion: "0.0.0",
    };
    const nextSnapshot = {
      configPath: "/tmp/next/.codex/config.toml",
      executablePath: "/opt/codex/bin/codex",
      cliVersion: "1.0.0",
    };
    const staleCatalog = makeCodexModelCatalogState({
      snapshot: {
        config_path: nextSnapshot.configPath,
        executable_path: nextSnapshot.executablePath,
        cli_version: nextSnapshot.cliVersion,
      },
    });
    const nextCatalog = makeCodexModelCatalogState({
      snapshot: {
        config_path: nextSnapshot.configPath,
        executable_path: nextSnapshot.executablePath,
        cli_version: nextSnapshot.cliVersion,
      },
    });
    vi.mocked(cliManagerCodexModelCatalogGet).mockResolvedValueOnce(makeCodexModelCatalogState());

    const client = createTestQueryClient();
    client.setQueryData(cliManagerKeys.codexModelCatalog(nextSnapshot), staleCatalog, {
      updatedAt: 1,
    });
    const wrapper = createQueryWrapper(client);
    const { result, rerender } = renderHook(
      ({ snapshot }) => ({
        catalog: useCliManagerCodexModelCatalogQuery({ snapshot }),
        refresh: useCliManagerCodexModelCatalogRefresh(),
      }),
      { wrapper, initialProps: { snapshot: oldSnapshot } }
    );

    await waitFor(() => expect(result.current.catalog.isSuccess).toBe(true));

    let resolveNextCatalog!: (value: CodexModelCatalogState) => void;
    const nextCatalogRequest = new Promise<CodexModelCatalogState>((resolve) => {
      resolveNextCatalog = resolve;
    });
    vi.mocked(cliManagerCodexModelCatalogGet).mockReset().mockReturnValue(nextCatalogRequest);

    rerender({ snapshot: nextSnapshot });
    await waitFor(() => expect(cliManagerCodexModelCatalogGet).toHaveBeenCalledTimes(1));

    let refreshRequest!: Promise<void>;
    act(() => {
      refreshRequest = result.current.refresh(nextSnapshot);
    });
    await Promise.resolve();
    expect(cliManagerCodexModelCatalogGet).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveNextCatalog(nextCatalog);
      await nextCatalogRequest;
      await refreshRequest;
    });
    expect(cliManagerCodexModelCatalogGet).toHaveBeenCalledTimes(1);
    expect(client.getQueryData(cliManagerKeys.codexModelCatalog(nextSnapshot))).toEqual(
      nextCatalog
    );
  });

  it("serializes ordinary Codex config and TOML writes in one mutation scope", async () => {
    setTauriRuntime();

    let resolveConfig!: (value: CodexConfigState) => void;
    let resolveToml!: (value: CodexConfigState) => void;
    const configPromise = new Promise<CodexConfigState>((resolve) => {
      resolveConfig = resolve;
    });
    const tomlPromise = new Promise<CodexConfigState>((resolve) => {
      resolveToml = resolve;
    });
    vi.mocked(cliManagerCodexConfigSet).mockReset().mockReturnValueOnce(configPromise);
    vi.mocked(cliManagerCodexConfigTomlSet).mockReset().mockReturnValueOnce(tomlPromise);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const configMutation = renderHook(() => useCliManagerCodexConfigSetMutation(), { wrapper });
    const tomlMutation = renderHook(() => useCliManagerCodexConfigTomlSetMutation(), { wrapper });

    let configCall!: Promise<CodexConfigState>;
    await act(async () => {
      configCall = configMutation.result.current.mutateAsync({ model: "first" });
      await Promise.resolve();
    });
    await waitFor(() => expect(cliManagerCodexConfigSet).toHaveBeenCalledTimes(1));

    let tomlCall!: Promise<CodexConfigState>;
    await act(async () => {
      tomlCall = tomlMutation.result.current.mutateAsync({ toml: 'model = "second"' });
      await Promise.resolve();
    });
    expect(cliManagerCodexConfigTomlSet).not.toHaveBeenCalled();

    await act(async () => {
      resolveConfig(makeCodexConfigState({ model: "first" }));
      await configCall;
    });
    await waitFor(() => expect(cliManagerCodexConfigTomlSet).toHaveBeenCalledTimes(1));

    await act(async () => {
      resolveToml(makeCodexConfigState({ model: "second" }));
      await tomlCall;
    });
    expect(cliManagerCodexConfigTomlSet).toHaveBeenCalledWith('model = "second"');
  });

  it("mutation hooks keep cache unchanged when service returns null", async () => {
    setTauriRuntime();

    vi.mocked(cliManagerClaudeSettingsSet).mockResolvedValue(null as never);
    vi.mocked(cliManagerCodexConfigSet).mockResolvedValue(null as never);
    vi.mocked(cliManagerCodexConfigTomlSet).mockResolvedValue(null as never);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    client.setQueryData(cliManagerKeys.claudeSettings(), { exists: true, model: "old-claude" });
    client.setQueryData(cliManagerKeys.codexConfig(), { exists: true, model: "old-codex" });

    const claudeMutation = renderHook(() => useCliManagerClaudeSettingsSetMutation(), { wrapper });
    const codexMutation = renderHook(() => useCliManagerCodexConfigSetMutation(), { wrapper });
    const tomlMutation = renderHook(() => useCliManagerCodexConfigTomlSetMutation(), { wrapper });

    await act(async () => {
      await claudeMutation.result.current.mutateAsync({ model: "new-claude" });
      await codexMutation.result.current.mutateAsync({ model: "new-codex" });
      await tomlMutation.result.current.mutateAsync({ toml: 'model = "new-codex"' });
    });

    expect(client.getQueryData(cliManagerKeys.claudeSettings())).toEqual({
      exists: true,
      model: "old-claude",
    });
    expect(client.getQueryData(cliManagerKeys.codexConfig())).toEqual({
      exists: true,
      model: "old-codex",
    });
  });

  it("pickCliAvailable maps info to availability state", () => {
    expect(pickCliAvailable(null)).toBe("unavailable");
    expect(pickCliAvailable(makeSimpleCliInfo({ found: false }))).toBe("unavailable");
    expect(pickCliAvailable(makeSimpleCliInfo({ found: true }))).toBe("available");
  });
});
