import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  cliManagerClaudeInfoGet,
  cliManagerClaudeSettingsGet,
  cliManagerClaudeSettingsSet,
  cliManagerCodexConfigGet,
  cliManagerCodexConfigSet,
  cliManagerCodexConfigTomlGet,
  cliManagerCodexConfigTomlSet,
  cliManagerCodexInfoGet,
  cliManagerGeminiInfoGet,
} from "../../services/cli/cliManager";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { cliManagerKeys } from "../keys";
import {
  pickCliAvailable,
  useCliManagerClaudeInfoQuery,
  useCliManagerClaudeSettingsQuery,
  useCliManagerClaudeSettingsSetMutation,
  useCliManagerCodexConfigQuery,
  useCliManagerCodexConfigSetMutation,
  useCliManagerCodexConfigTomlQuery,
  useCliManagerCodexConfigTomlSetMutation,
  useCliManagerCodexInfoQuery,
  useCliManagerGeminiInfoQuery,
} from "../cliManager";

vi.mock("../../services/cli/cliManager", async () => {
  const actual = await vi.importActual<typeof import("../../services/cli/cliManager")>(
    "../../services/cli/cliManager"
  );
  return {
    ...actual,
    cliManagerClaudeInfoGet: vi.fn(),
    cliManagerClaudeSettingsGet: vi.fn(),
    cliManagerClaudeSettingsSet: vi.fn(),
    cliManagerCodexInfoGet: vi.fn(),
    cliManagerCodexConfigGet: vi.fn(),
    cliManagerCodexConfigSet: vi.fn(),
    cliManagerCodexConfigTomlGet: vi.fn(),
    cliManagerCodexConfigTomlSet: vi.fn(),
    cliManagerGeminiInfoGet: vi.fn(),
  };
});

describe("query/cliManager", () => {
  it("calls cliManager queries with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(cliManagerClaudeInfoGet).mockResolvedValue({ found: true } as any);
    vi.mocked(cliManagerClaudeSettingsGet).mockResolvedValue({ exists: true } as any);
    vi.mocked(cliManagerCodexInfoGet).mockResolvedValue({ found: true } as any);
    vi.mocked(cliManagerCodexConfigGet).mockResolvedValue({ exists: true } as any);
    vi.mocked(cliManagerCodexConfigTomlGet).mockResolvedValue({ exists: true, toml: "" } as any);
    vi.mocked(cliManagerGeminiInfoGet).mockResolvedValue({ found: true } as any);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useCliManagerClaudeInfoQuery(), { wrapper });
    renderHook(() => useCliManagerClaudeSettingsQuery(), { wrapper });
    renderHook(() => useCliManagerCodexInfoQuery(), { wrapper });
    renderHook(() => useCliManagerCodexConfigQuery(), { wrapper });
    renderHook(() => useCliManagerCodexConfigTomlQuery(), { wrapper });
    renderHook(() => useCliManagerGeminiInfoQuery(), { wrapper });

    await waitFor(() => {
      expect(cliManagerClaudeInfoGet).toHaveBeenCalled();
      expect(cliManagerClaudeSettingsGet).toHaveBeenCalled();
      expect(cliManagerCodexInfoGet).toHaveBeenCalled();
      expect(cliManagerCodexConfigGet).toHaveBeenCalled();
      expect(cliManagerCodexConfigTomlGet).toHaveBeenCalled();
      expect(cliManagerGeminiInfoGet).toHaveBeenCalled();
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

  it("respects options.enabled=false for all cliManager info/config queries", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useCliManagerClaudeInfoQuery({ enabled: false }), { wrapper });
    renderHook(() => useCliManagerClaudeSettingsQuery({ enabled: false }), { wrapper });
    renderHook(() => useCliManagerCodexInfoQuery({ enabled: false }), { wrapper });
    renderHook(() => useCliManagerCodexConfigQuery({ enabled: false }), { wrapper });
    renderHook(() => useCliManagerCodexConfigTomlQuery({ enabled: false }), { wrapper });
    renderHook(() => useCliManagerGeminiInfoQuery({ enabled: false }), { wrapper });

    await Promise.resolve();

    expect(cliManagerClaudeInfoGet).not.toHaveBeenCalled();
    expect(cliManagerClaudeSettingsGet).not.toHaveBeenCalled();
    expect(cliManagerCodexInfoGet).not.toHaveBeenCalled();
    expect(cliManagerCodexConfigGet).not.toHaveBeenCalled();
    expect(cliManagerCodexConfigTomlGet).not.toHaveBeenCalled();
    expect(cliManagerGeminiInfoGet).not.toHaveBeenCalled();
  });

  it("useCliManagerClaudeSettingsSetMutation updates cache and invalidates", async () => {
    setTauriRuntime();

    const updated = { exists: true, model: "claude" } as any;
    vi.mocked(cliManagerClaudeSettingsSet).mockResolvedValue(updated);

    const client = createTestQueryClient();
    client.setQueryData(cliManagerKeys.claudeSettings(), { exists: true, model: "old" });
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useCliManagerClaudeSettingsSetMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ model: "claude" });
    });

    expect(client.getQueryData(cliManagerKeys.claudeSettings())).toEqual(updated);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: cliManagerKeys.claudeSettings() });
  });

  it("useCliManagerCodexConfigSetMutation updates cache and invalidates", async () => {
    setTauriRuntime();

    const updated = { exists: true, model: "gpt-5" } as any;
    vi.mocked(cliManagerCodexConfigSet).mockResolvedValue(updated);

    const client = createTestQueryClient();
    client.setQueryData(cliManagerKeys.codexConfig(), { exists: true, model: "old" });
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

    const updated = { exists: true, model: "gpt-5" } as any;
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

  it("mutation hooks keep cache unchanged when service returns null", async () => {
    setTauriRuntime();

    vi.mocked(cliManagerClaudeSettingsSet).mockResolvedValue(null);
    vi.mocked(cliManagerCodexConfigSet).mockResolvedValue(null);
    vi.mocked(cliManagerCodexConfigTomlSet).mockResolvedValue(null);

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
    expect(pickCliAvailable({ found: false } as any)).toBe("unavailable");
    expect(pickCliAvailable({ found: true } as any)).toBe("available");
  });
});
