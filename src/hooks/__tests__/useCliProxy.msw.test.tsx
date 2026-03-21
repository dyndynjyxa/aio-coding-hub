import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import { tauriInvoke } from "../../test/mocks/tauri";
import { setCliProxyStatusAllState } from "../../test/msw/state";
import { useCliProxy } from "../useCliProxy";

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../services/consoleLog", () => ({ logToConsole: vi.fn() }));

function Harness() {
  const cliProxy = useCliProxy();
  return (
    <div>
      <div data-testid="codex-enabled">{String(cliProxy.enabled.codex)}</div>
      <div data-testid="codex-applied">{String(cliProxy.appliedToCurrentGateway.codex)}</div>
      <button onClick={() => cliProxy.setCliProxyEnabled("codex", !cliProxy.enabled.codex)}>
        toggle-codex
      </button>
    </div>
  );
}

describe("hooks/useCliProxy (msw integration)", () => {
  it("runs through invoke->fetch->msw handlers and toggles state via user-event", async () => {
    setTauriRuntime();
    setCliProxyStatusAllState([
      { cli_key: "claude", enabled: false, base_origin: null, applied_to_current_gateway: null },
      { cli_key: "codex", enabled: false, base_origin: null, applied_to_current_gateway: null },
      { cli_key: "gemini", enabled: false, base_origin: null, applied_to_current_gateway: null },
    ]);

    const client = createTestQueryClient();
    render(
      <QueryClientProvider client={client}>
        <Harness />
      </QueryClientProvider>
    );

    // status_all should be invoked by the query.
    await waitFor(() =>
      expect(
        vi.mocked(tauriInvoke).mock.calls.some((call) => call[0] === "cli_proxy_status_all")
      ).toBe(true)
    );

    expect(screen.getByTestId("codex-enabled").textContent).toBe("false");
    expect(screen.getByTestId("codex-applied").textContent).toBe("null");

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "toggle-codex" }));

    await waitFor(() =>
      expect(vi.mocked(tauriInvoke)).toHaveBeenCalledWith("cli_proxy_set_enabled", {
        cliKey: "codex",
        enabled: true,
      })
    );
    await waitFor(() => expect(vi.mocked(toast)).toHaveBeenCalledWith("已开启代理"));
    await waitFor(() => expect(screen.getByTestId("codex-enabled").textContent).toBe("true"));
    await waitFor(() => expect(screen.getByTestId("codex-applied").textContent).toBe("true"));
  });
});
