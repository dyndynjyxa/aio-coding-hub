import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { CliProxyStatus } from "../../services/cli/cliProxy";
import { cliProxySetEnabled, cliProxyStatusAll } from "../../services/cli/cliProxy";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { cliProxyKeys } from "../keys";
import { useCliProxySetEnabledMutation, useCliProxyStatusAllQuery } from "../cliProxy";

vi.mock("../../services/cli/cliProxy", async () => {
  const actual = await vi.importActual<typeof import("../../services/cli/cliProxy")>(
    "../../services/cli/cliProxy"
  );
  return {
    ...actual,
    cliProxyStatusAll: vi.fn(),
    cliProxySetEnabled: vi.fn(),
  };
});

describe("query/cliProxy", () => {
  it("useCliProxyStatusAllQuery respects enabled=false", () => {
    setTauriRuntime();
    vi.mocked(cliProxyStatusAll).mockClear();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useCliProxyStatusAllQuery({ enabled: false }), { wrapper });

    expect(cliProxyStatusAll).not.toHaveBeenCalled();
  });

  it("calls cliProxyStatusAll with tauri runtime", async () => {
    setTauriRuntime();
    vi.mocked(cliProxyStatusAll).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useCliProxyStatusAllQuery(), { wrapper });

    await waitFor(() => {
      expect(cliProxyStatusAll).toHaveBeenCalled();
    });
  });

  it("useCliProxyStatusAllQuery enters error state when service rejects", async () => {
    setTauriRuntime();
    vi.mocked(cliProxyStatusAll).mockRejectedValue(new Error("cli proxy query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useCliProxyStatusAllQuery(), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("optimistically updates status cache on setEnabled", async () => {
    setTauriRuntime();

    const initial: CliProxyStatus[] = [
      { cli_key: "claude", enabled: true, base_origin: null, applied_to_current_gateway: true },
      { cli_key: "codex", enabled: false, base_origin: null, applied_to_current_gateway: null },
      { cli_key: "gemini", enabled: false, base_origin: null, applied_to_current_gateway: null },
    ];
    vi.mocked(cliProxySetEnabled).mockResolvedValue({
      trace_id: "t1",
      cli_key: "codex",
      enabled: true,
      ok: true,
      error_code: null,
      message: "ok",
      base_origin: "http://127.0.0.1:37123",
    });

    const client = createTestQueryClient();
    client.setQueryData(cliProxyKeys.statusAll(), initial);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useCliProxySetEnabledMutation(), { wrapper });

    await act(async () => {
      const promise = result.current.mutateAsync({ cliKey: "codex", enabled: true });

      const optimistic = client.getQueryData<CliProxyStatus[] | null>(cliProxyKeys.statusAll());
      expect(optimistic?.find((r) => r.cli_key === "codex")?.enabled).toBe(true);
      expect(optimistic?.find((r) => r.cli_key === "codex")?.applied_to_current_gateway).toBe(true);

      await promise;
    });

    expect(cliProxySetEnabled).toHaveBeenCalledWith({ cli_key: "codex", enabled: true });
  });

  it("adds a missing cli row during optimistic update", async () => {
    setTauriRuntime();

    const initial: CliProxyStatus[] = [
      { cli_key: "claude", enabled: true, base_origin: null, applied_to_current_gateway: true },
    ];
    vi.mocked(cliProxySetEnabled).mockResolvedValue({
      trace_id: "t2",
      cli_key: "codex",
      enabled: false,
      ok: true,
      error_code: null,
      message: "ok",
      base_origin: null,
    });

    const client = createTestQueryClient();
    client.setQueryData(cliProxyKeys.statusAll(), initial);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useCliProxySetEnabledMutation(), { wrapper });

    await act(async () => {
      const promise = result.current.mutateAsync({ cliKey: "codex", enabled: false });

      const optimistic = client.getQueryData<CliProxyStatus[] | null>(cliProxyKeys.statusAll());
      expect(optimistic?.[0]).toEqual({
        cli_key: "codex",
        enabled: false,
        base_origin: null,
        applied_to_current_gateway: null,
      });

      await promise;
    });
  });

  it("keeps an empty cache unchanged when optimistic update starts without data", async () => {
    setTauriRuntime();
    vi.mocked(cliProxySetEnabled).mockResolvedValue({
      trace_id: "t3",
      cli_key: "codex",
      enabled: true,
      ok: true,
      error_code: null,
      message: "ok",
      base_origin: "http://127.0.0.1:37123",
    });

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useCliProxySetEnabledMutation(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync({ cliKey: "codex", enabled: true });
    });

    expect(client.getQueryData(cliProxyKeys.statusAll())).toBeUndefined();
  });

  it("clears applied gateway flag when disabling an existing cli row", async () => {
    setTauriRuntime();

    const initial: CliProxyStatus[] = [
      { cli_key: "codex", enabled: true, base_origin: null, applied_to_current_gateway: true },
    ];
    vi.mocked(cliProxySetEnabled).mockResolvedValue({
      trace_id: "t4",
      cli_key: "codex",
      enabled: false,
      ok: true,
      error_code: null,
      message: "ok",
      base_origin: null,
    });

    const client = createTestQueryClient();
    client.setQueryData(cliProxyKeys.statusAll(), initial);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useCliProxySetEnabledMutation(), { wrapper });

    await act(async () => {
      const promise = result.current.mutateAsync({ cliKey: "codex", enabled: false });

      const optimistic = client.getQueryData<CliProxyStatus[] | null>(cliProxyKeys.statusAll());
      expect(optimistic).toEqual([
        {
          cli_key: "codex",
          enabled: false,
          base_origin: null,
          applied_to_current_gateway: null,
        },
      ]);

      await promise;
    });
  });

  it("rolls back cache when setEnabled fails", async () => {
    setTauriRuntime();

    const initial: CliProxyStatus[] = [
      { cli_key: "codex", enabled: false, base_origin: null, applied_to_current_gateway: null },
    ];
    vi.mocked(cliProxySetEnabled).mockRejectedValue(new Error("boom"));

    const client = createTestQueryClient();
    client.setQueryData(cliProxyKeys.statusAll(), initial);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useCliProxySetEnabledMutation(), { wrapper });

    await act(async () => {
      await expect(result.current.mutateAsync({ cliKey: "codex", enabled: true })).rejects.toThrow(
        "boom"
      );
    });

    expect(client.getQueryData(cliProxyKeys.statusAll())).toEqual(initial);
  });
});
