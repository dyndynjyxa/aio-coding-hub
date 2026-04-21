import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../../../services/consoleLog";
import { envConflictsCheck } from "../../../../services/cli/envConflicts";
import { useCliProxy } from "../../../../hooks/useCliProxy";
import { useHomeCliProxy } from "../useHomeCliProxy";

vi.mock("../../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));

vi.mock("../../../../services/cli/envConflicts", async () => {
  const actual = await vi.importActual<
    typeof import("../../../../services/cli/envConflicts")
  >("../../../../services/cli/envConflicts");
  return { ...actual, envConflictsCheck: vi.fn() };
});

vi.mock("../../../../hooks/useCliProxy", async () => {
  const actual =
    await vi.importActual<typeof import("../../../../hooks/useCliProxy")>(
      "../../../../hooks/useCliProxy"
    );
  return { ...actual, useCliProxy: vi.fn() };
});

function makeCliProxyState() {
  return {
    loading: false,
    available: true,
    enabled: { claude: false, codex: false, gemini: false },
    appliedToCurrentGateway: { claude: null, codex: null, gemini: null },
    toggling: { claude: false, codex: false, gemini: false },
    setCliProxyEnabled: vi.fn(),
  };
}

describe("pages/home/hooks/useHomeCliProxy", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("immediately disables the target cli without running env conflict checks", () => {
    const cliProxyState = makeCliProxyState();
    vi.mocked(useCliProxy).mockReturnValue(cliProxyState as any);

    const { result } = renderHook(() => useHomeCliProxy());

    act(() => {
      result.current.requestCliProxyEnabledSwitch("codex", false);
    });

    expect(cliProxyState.setCliProxyEnabled).toHaveBeenCalledWith("codex", false);
    expect(envConflictsCheck).not.toHaveBeenCalled();
  });

  it("marks the current cli as checking and ignores duplicate enable requests", async () => {
    const cliProxyState = makeCliProxyState();
    vi.mocked(useCliProxy).mockReturnValue(cliProxyState as any);

    let resolveCheck: (value: []) => void = () => {};
    vi.mocked(envConflictsCheck).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveCheck = resolve as (value: []) => void;
        })
    );

    const { result } = renderHook(() => useHomeCliProxy());

    act(() => {
      result.current.requestCliProxyEnabledSwitch("codex", true);
    });

    expect(result.current.cliProxyToggling.codex).toBe(true);

    act(() => {
      result.current.requestCliProxyEnabledSwitch("codex", true);
    });

    expect(envConflictsCheck).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveCheck([]);
      await Promise.resolve();
    });

    await waitFor(() => expect(cliProxyState.setCliProxyEnabled).toHaveBeenCalledWith("codex", true));
    expect(result.current.cliProxyToggling.codex).toBe(false);
  });

  it("logs and still enables the cli when env conflict checks fail", async () => {
    const cliProxyState = makeCliProxyState();
    vi.mocked(useCliProxy).mockReturnValue(cliProxyState as any);
    vi.mocked(envConflictsCheck).mockRejectedValueOnce(new Error("env check boom"));

    const { result } = renderHook(() => useHomeCliProxy());

    act(() => {
      result.current.requestCliProxyEnabledSwitch("codex", true);
    });

    await waitFor(() => expect(cliProxyState.setCliProxyEnabled).toHaveBeenCalledWith("codex", true));
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "检查环境变量冲突失败，仍尝试开启 CLI 代理",
      expect.objectContaining({ cli: "codex", error: "Error: env check boom" })
    );
  });
});
