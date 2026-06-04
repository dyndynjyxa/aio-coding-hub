import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { NAV, NAV_SECTIONS, Sidebar } from "../Sidebar";
import { AIO_RELEASES_URL, AIO_REPO_URL } from "../../constants/urls";
import { tauriOpenUrl } from "../../test/mocks/tauri";

const gatewayMetaRef = vi.hoisted(() => ({
  current: { gatewayAvailable: "checking", gateway: null, preferredPort: 37123 } as any,
}));

const updateMetaRef = vi.hoisted(() => ({
  current: {
    about: null,
    updateCandidate: null,
    checkingUpdate: false,
    dialogOpen: false,
    installingUpdate: false,
    installError: null,
    installTotalBytes: null,
    installDownloadedBytes: 0,
  } as any,
}));

const updateDialogSetOpenMock = vi.hoisted(() => vi.fn());
const devPreviewRef = vi.hoisted(() => ({
  current: { enabled: false, setEnabled: vi.fn(), toggle: vi.fn() } as any,
}));
const themeRef = vi.hoisted(() => ({
  current: { theme: "system", resolvedTheme: "light", setTheme: vi.fn() } as any,
}));
const cliProxyMocks = vi.hoisted(() => {
  const requestCliProxyEnabledSwitch = vi.fn();
  const setPendingCliProxyEnablePrompt = vi.fn();
  const confirmPendingCliProxyEnable = vi.fn();

  return {
    requestCliProxyEnabledSwitch,
    setPendingCliProxyEnablePrompt,
    confirmPendingCliProxyEnable,
    current: {
      cliProxyLoading: false,
      cliProxyAvailable: true,
      cliProxyEnabled: { claude: true, codex: false, gemini: false },
      cliProxyAppliedToCurrentGateway: { claude: true, codex: null, gemini: null },
      cliProxyToggling: { claude: false, codex: false, gemini: false },
      pendingCliProxyEnablePrompt: null,
      requestCliProxyEnabledSwitch,
      setPendingCliProxyEnablePrompt,
      confirmPendingCliProxyEnable,
    } as any,
  };
});

vi.mock("../../hooks/useGatewayMeta", () => ({
  useGatewayMeta: () => gatewayMetaRef.current,
}));

vi.mock("../../hooks/useUpdateMeta", () => ({
  useUpdateMeta: () => updateMetaRef.current,
  updateDialogSetOpen: updateDialogSetOpenMock,
}));
vi.mock("../../hooks/useDevPreviewData", () => ({
  useDevPreviewData: () => devPreviewRef.current,
}));
vi.mock("../../hooks/useTheme", () => ({
  useTheme: () => themeRef.current,
}));
vi.mock("../../hooks/useCliProxyControls", () => ({
  useCliProxyControls: () => cliProxyMocks.current,
}));

describe("ui/Sidebar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    devPreviewRef.current = { enabled: false, setEnabled: vi.fn(), toggle: vi.fn() };
    themeRef.current = { theme: "system", resolvedTheme: "light", setTheme: vi.fn() };
    cliProxyMocks.current = {
      cliProxyLoading: false,
      cliProxyAvailable: true,
      cliProxyEnabled: { claude: true, codex: false, gemini: false },
      cliProxyAppliedToCurrentGateway: { claude: true, codex: null, gemini: null },
      cliProxyToggling: { claude: false, codex: false, gemini: false },
      pendingCliProxyEnablePrompt: null,
      requestCliProxyEnabledSwitch: cliProxyMocks.requestCliProxyEnabledSwitch,
      setPendingCliProxyEnablePrompt: cliProxyMocks.setPendingCliProxyEnablePrompt,
      confirmPendingCliProxyEnable: cliProxyMocks.confirmPendingCliProxyEnable,
    };
    gatewayMetaRef.current = { gatewayAvailable: "checking", gateway: null, preferredPort: 37123 };
    updateMetaRef.current = {
      about: null,
      updateCandidate: null,
      checkingUpdate: false,
      dialogOpen: false,
      installingUpdate: false,
      installError: null,
      installTotalBytes: null,
      installDownloadedBytes: 0,
    };
  });

  it("renders base status without update candidate", () => {
    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    const gatewayStatus = screen.getByLabelText("网关状态：检查中，端口 —");

    expect(gatewayStatus).toBeInTheDocument();
    expect(within(gatewayStatus).getByText("网关检查中")).toBeInTheDocument();
    expect(screen.getByText("Port: —")).toBeInTheDocument();
    expect(screen.queryByText("NEW")).not.toBeInTheDocument();
  });

  it("switches theme from the sidebar control", () => {
    const setTheme = vi.fn();
    themeRef.current = { theme: "system", resolvedTheme: "light", setTheme };

    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    const themeSwitcher = screen.getByLabelText("主题切换");
    const icons = themeSwitcher.querySelectorAll("svg");

    expect(screen.getByRole("button", { name: /切换到 System 主题/ })).toHaveAttribute(
      "aria-pressed",
      "true"
    );
    expect(within(themeSwitcher).queryByText("Light")).not.toBeInTheDocument();
    expect(within(themeSwitcher).queryByText("Dark")).not.toBeInTheDocument();
    expect(within(themeSwitcher).queryByText("System")).not.toBeInTheDocument();
    expect(Array.from(icons).every((icon) => icon.getAttribute("aria-hidden") === "true")).toBe(
      true
    );

    fireEvent.click(screen.getByRole("button", { name: /切换到 Light 主题/ }));
    expect(setTheme).toHaveBeenCalledWith("light");

    fireEvent.click(screen.getByRole("button", { name: /切换到 Dark 主题/ }));
    expect(setTheme).toHaveBeenCalledWith("dark");

    fireEvent.click(screen.getByRole("button", { name: /切换到 System 主题/ }));
    expect(setTheme).toHaveBeenCalledWith("system");
  });

  it("renders grouped navigation sections and keeps navigation exports compatible", () => {
    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    expect(screen.getByRole("heading", { name: "MAIN" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "TOOLS" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "SETTING" })).toBeInTheDocument();
    expect(
      NAV_SECTIONS.map((section) => [section.label, section.items.map((item) => item.to)])
    ).toEqual([
      ["MAIN", ["/", "/providers", "/sessions"]],
      ["TOOLS", ["/workspaces", "/prompts", "/mcp", "/skills", "/usage", "/logs", "/cli-manager"]],
      ["SETTING", ["/console", "/settings"]],
    ]);
    expect(NAV.map((item) => item.to)).toEqual(
      NAV_SECTIONS.flatMap((section) => section.items.map((item) => item.to))
    );

    for (const item of NAV) {
      expect(screen.getByRole("link", { name: item.label })).toBeInTheDocument();
    }
  });

  it("renders the GitHub link when no update candidate exists", () => {
    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    const repoLink = screen.getByRole("link", { name: "AIO Coding Hub GitHub 仓库" });

    expect(repoLink).toHaveAttribute("href", AIO_REPO_URL);
  });

  it("opens the GitHub link through the desktop opener", async () => {
    vi.mocked(tauriOpenUrl).mockResolvedValue(undefined as never);

    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("link", { name: "AIO Coding Hub GitHub 仓库" }));

    await waitFor(() => {
      expect(tauriOpenUrl).toHaveBeenCalledWith(AIO_REPO_URL);
    });
  });

  it("opens update dialog when update candidate exists (non-portable)", () => {
    gatewayMetaRef.current = {
      gatewayAvailable: "available",
      gateway: { running: true, port: 37123 },
      preferredPort: 37123,
    };
    updateMetaRef.current = {
      ...updateMetaRef.current,
      about: { run_mode: "desktop" },
      updateCandidate: { version: "0.0.0" },
    };

    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    const updateLink = screen.getByRole("link", {
      name: "AIO Coding Hub GitHub：发现新版本，打开更新对话框",
    });

    expect(updateLink).toHaveAttribute("href", AIO_REPO_URL);
    expect(screen.getByText("NEW")).toBeInTheDocument();

    fireEvent.click(updateLink);
    expect(updateDialogSetOpenMock).toHaveBeenCalledWith(true);
  });

  it("opens releases page when update candidate exists and app is portable", async () => {
    gatewayMetaRef.current = {
      gatewayAvailable: "available",
      gateway: { running: false, port: null },
      preferredPort: 37123,
    };
    updateMetaRef.current = {
      ...updateMetaRef.current,
      about: { run_mode: "portable" },
      updateCandidate: { version: "0.0.0" },
    };

    vi.mocked(tauriOpenUrl).mockRejectedValue(new Error("boom"));
    const windowOpen = vi.spyOn(window, "open").mockImplementation(() => null as any);

    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    const updateLink = screen.getByRole("link", {
      name: "AIO Coding Hub GitHub：发现新版本，打开下载页",
    });

    expect(updateLink).toHaveAttribute("href", AIO_REPO_URL);
    expect(screen.getByText("NEW")).toBeInTheDocument();

    fireEvent.click(updateLink);

    await waitFor(() => {
      expect(tauriOpenUrl).toHaveBeenCalledWith(AIO_RELEASES_URL);
      expect(windowOpen).toHaveBeenCalledWith(AIO_RELEASES_URL, "_blank", "noopener,noreferrer");
    });
    windowOpen.mockRestore();
  });

  it("opens update dialog when portable app has dev preview enabled", () => {
    gatewayMetaRef.current = {
      gatewayAvailable: "available",
      gateway: { running: true, port: 37123 },
      preferredPort: 37123,
    };
    updateMetaRef.current = {
      ...updateMetaRef.current,
      about: { run_mode: "portable" },
      updateCandidate: { version: "0.0.0" },
    };
    devPreviewRef.current = { enabled: true, setEnabled: vi.fn(), toggle: vi.fn() };

    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    const updateLink = screen.getByRole("link", {
      name: "AIO Coding Hub GitHub：发现新版本，打开更新对话框",
    });

    expect(updateLink).toHaveAttribute("href", AIO_REPO_URL);
    expect(screen.getByText("NEW")).toBeInTheDocument();

    fireEvent.click(updateLink);
    expect(updateDialogSetOpenMock).toHaveBeenCalledWith(true);
  });

  it("renders stopped status with the preferred port when gateway is stopped", () => {
    gatewayMetaRef.current = {
      gatewayAvailable: "available",
      gateway: { running: false, port: null },
      preferredPort: 37123,
    };

    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    const gatewayStatus = screen.getByLabelText("网关状态：已停止，端口 37123");
    expect(gatewayStatus).toBeInTheDocument();
    expect(gatewayStatus).toHaveAttribute("aria-label", expect.stringContaining("已停止"));
    expect(within(gatewayStatus).getByText("网关已关闭")).toBeInTheDocument();
    expect(screen.getByText("Port: 37123")).toBeInTheDocument();
  });

  it("renders gateway status and Claude/Codex/Gemini proxy switches in the bottom panel", () => {
    gatewayMetaRef.current = {
      gatewayAvailable: "available",
      gateway: { running: true, port: 37124 },
      preferredPort: 37123,
    };

    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    const gatewayStatus = screen.getByLabelText("网关状态：运行中，端口 37124");
    expect(gatewayStatus).toBeInTheDocument();
    expect(gatewayStatus).toHaveAttribute("aria-label", expect.stringContaining("运行中"));
    expect(within(gatewayStatus).getByText("网关已开启")).toBeInTheDocument();
    expect(screen.getByText("Port: 37124")).toBeInTheDocument();

    expect(screen.getByRole("switch", { name: "Claude 代理开关" })).toHaveAttribute(
      "data-state",
      "checked"
    );
    expect(screen.getByRole("switch", { name: "Codex 代理开关" })).toHaveAttribute(
      "data-state",
      "unchecked"
    );
    expect(screen.getByRole("switch", { name: "Gemini 代理开关" })).toHaveAttribute(
      "data-state",
      "unchecked"
    );
  });

  it("forwards proxy switch requests through useCliProxyControls", () => {
    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("switch", { name: "Codex 代理开关" }));
    expect(cliProxyMocks.requestCliProxyEnabledSwitch).toHaveBeenCalledWith("codex", true);

    fireEvent.click(screen.getByRole("switch", { name: "Claude 代理开关" }));
    expect(cliProxyMocks.requestCliProxyEnabledSwitch).toHaveBeenCalledWith("claude", false);

    fireEvent.click(screen.getByRole("switch", { name: "Gemini 代理开关" }));
    expect(cliProxyMocks.requestCliProxyEnabledSwitch).toHaveBeenCalledWith("gemini", true);
  });

  it("shows repair for drifted proxy rows and requests enable on repair", () => {
    cliProxyMocks.current = {
      ...cliProxyMocks.current,
      cliProxyEnabled: { claude: true, codex: true, gemini: false },
      cliProxyAppliedToCurrentGateway: { claude: true, codex: false, gemini: null },
    };

    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("button", { name: "修复 Codex 代理" }));
    expect(cliProxyMocks.requestCliProxyEnabledSwitch).toHaveBeenCalledWith("codex", true);
    expect(screen.queryByRole("button", { name: "修复 Claude 代理" })).not.toBeInTheDocument();
  });

  it("keeps the CLI proxy conflict confirmation dialog in the sidebar flow", () => {
    cliProxyMocks.current = {
      ...cliProxyMocks.current,
      pendingCliProxyEnablePrompt: {
        cliKey: "gemini",
        conflicts: [
          {
            var_name: "GEMINI_API_KEY",
            source_type: "system",
            source_path: "Process Environment",
          },
        ],
      },
    };

    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText("GEMINI_API_KEY")).toBeInTheDocument();
    expect(within(dialog).getByText("Process Environment")).toBeInTheDocument();

    fireEvent.click(within(dialog).getByRole("button", { name: "继续启用" }));
    expect(cliProxyMocks.confirmPendingCliProxyEnable).toHaveBeenCalledTimes(1);

    fireEvent.click(within(dialog).getByRole("button", { name: "取消" }));
    expect(cliProxyMocks.setPendingCliProxyEnablePrompt).toHaveBeenCalledWith(null);
  });
});
