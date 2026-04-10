import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  wslConfigStatusGet,
  wslConfigureClients,
  wslDetect,
  wslHostAddressGet,
} from "../../services/app/wsl";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { wslKeys } from "../keys";
import {
  useWslConfigStatusQuery,
  useWslConfigureClientsMutation,
  useWslDetectionQuery,
  useWslHostAddressQuery,
  useWslOverviewQuery,
} from "../wsl";

vi.mock("../../services/app/wsl", async () => {
  const actual =
    await vi.importActual<typeof import("../../services/app/wsl")>("../../services/app/wsl");
  return {
    ...actual,
    wslDetect: vi.fn(),
    wslHostAddressGet: vi.fn(),
    wslConfigStatusGet: vi.fn(),
    wslConfigureClients: vi.fn(),
  };
});

describe("query/wsl", () => {
  it("fetches detection and host address with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(wslDetect).mockResolvedValue({ detected: true, distros: [] });
    vi.mocked(wslHostAddressGet).mockResolvedValue("172.20.1.1");

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useWslDetectionQuery(), { wrapper });
    renderHook(() => useWslHostAddressQuery(), { wrapper });

    await waitFor(() => {
      expect(wslDetect).toHaveBeenCalledTimes(1);
      expect(wslHostAddressGet).toHaveBeenCalledTimes(1);
    });
  });

  it("useWslDetectionQuery enters error state when wslDetect rejects", async () => {
    setTauriRuntime();

    vi.mocked(wslDetect).mockRejectedValue(new Error("wsl detection query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useWslDetectionQuery(), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("respects options.enabled=false for detection and host address queries", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useWslDetectionQuery({ enabled: false }), { wrapper });
    renderHook(() => useWslHostAddressQuery({ enabled: false }), { wrapper });
    await Promise.resolve();

    expect(wslDetect).not.toHaveBeenCalled();
    expect(wslHostAddressGet).not.toHaveBeenCalled();
  });

  it("useWslConfigStatusQuery does not call API when distros is empty", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useWslConfigStatusQuery([]), { wrapper });
    await Promise.resolve();

    expect(wslConfigStatusGet).not.toHaveBeenCalled();
  });

  it("useWslConfigStatusQuery fetches status rows for non-empty distros", async () => {
    setTauriRuntime();

    vi.mocked(wslConfigStatusGet).mockResolvedValue([
      {
        distro: "Ubuntu",
        claude: true,
        codex: false,
        gemini: true,
        claude_mcp: true,
        codex_mcp: false,
        gemini_mcp: true,
        claude_prompt: true,
        codex_prompt: false,
        gemini_prompt: true,
      },
    ]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useWslConfigStatusQuery(["Ubuntu"]), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(wslConfigStatusGet).toHaveBeenCalledWith(["Ubuntu"]);
    expect(result.current.data?.[0]?.distro).toBe("Ubuntu");
  });

  it("useWslConfigStatusQuery respects options.enabled=false", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useWslConfigStatusQuery(["Ubuntu"], { enabled: false }), { wrapper });
    await Promise.resolve();

    expect(wslConfigStatusGet).not.toHaveBeenCalled();
  });

  it("overview returns null payload when wslDetect returns null", async () => {
    setTauriRuntime();

    vi.mocked(wslDetect).mockResolvedValue(null);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useWslOverviewQuery({ enabled: true }), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data).toEqual({ detection: null, hostIp: null, statusRows: null });
    expect(wslHostAddressGet).not.toHaveBeenCalled();
    expect(wslConfigStatusGet).not.toHaveBeenCalled();
  });

  it("overview respects options.enabled=false", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useWslOverviewQuery({ enabled: false }), { wrapper });
    await Promise.resolve();

    expect(wslDetect).not.toHaveBeenCalled();
    expect(wslHostAddressGet).not.toHaveBeenCalled();
    expect(wslConfigStatusGet).not.toHaveBeenCalled();
  });

  it("overview returns early when no distros are detected", async () => {
    setTauriRuntime();

    vi.mocked(wslDetect).mockResolvedValue({ detected: false, distros: [] });

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useWslOverviewQuery({ enabled: true }), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(wslDetect).toHaveBeenCalledTimes(1);
    expect(wslHostAddressGet).not.toHaveBeenCalled();
    expect(wslConfigStatusGet).not.toHaveBeenCalled();
    expect(result.current.data?.detection?.detected).toBe(false);
    expect(result.current.data?.hostIp).toBeNull();
  });

  it("overview fetches host ip + config status when distros exist", async () => {
    setTauriRuntime();

    vi.mocked(wslDetect).mockResolvedValue({ detected: true, distros: ["Ubuntu"] });
    vi.mocked(wslHostAddressGet).mockResolvedValue("172.20.1.1");
    vi.mocked(wslConfigStatusGet).mockResolvedValue([
      {
        distro: "Ubuntu",
        claude: true,
        codex: false,
        gemini: false,
        claude_mcp: true,
        codex_mcp: false,
        gemini_mcp: false,
        claude_prompt: true,
        codex_prompt: false,
        gemini_prompt: false,
      },
    ]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useWslOverviewQuery({ enabled: true }), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(wslHostAddressGet).toHaveBeenCalledTimes(1);
    expect(wslConfigStatusGet).toHaveBeenCalledWith(["Ubuntu"]);
    expect(result.current.data?.hostIp).toBe("172.20.1.1");
    expect(result.current.data?.statusRows?.[0]?.distro).toBe("Ubuntu");
  });

  it("overview falls back to null when host/status APIs return null", async () => {
    setTauriRuntime();

    vi.mocked(wslDetect).mockResolvedValue({ detected: true, distros: ["Ubuntu"] });
    vi.mocked(wslHostAddressGet).mockResolvedValue(null);
    vi.mocked(wslConfigStatusGet).mockResolvedValue(null);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useWslOverviewQuery({ enabled: true }), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data?.hostIp).toBeNull();
    expect(result.current.data?.statusRows).toBeNull();
  });

  it("configure mutation invalidates wsl keys", async () => {
    setTauriRuntime();

    vi.mocked(wslConfigureClients).mockResolvedValue({ ok: true, message: "ok", distros: [] });

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useWslConfigureClientsMutation(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync();
    });

    expect(wslConfigureClients).toHaveBeenCalled();
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: wslKeys.all });
  });
});
