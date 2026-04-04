import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { WslSettingsCard } from "../WslSettingsCard";
import { useAppAboutQuery } from "../../../query/appAbout";
import { useSettingsSetMutation } from "../../../query/settings";
import { useWslConfigureClientsMutation, useWslOverviewQuery } from "../../../query/wsl";
import { toast } from "sonner";
import { emitTauriEvent, tauriListen } from "../../../test/mocks/tauri";

vi.mock("sonner", () => ({ toast: vi.fn() }));

vi.mock("../../../query/appAbout", async () => {
  const actual =
    await vi.importActual<typeof import("../../../query/appAbout")>("../../../query/appAbout");
  return { ...actual, useAppAboutQuery: vi.fn() };
});

vi.mock("../../../query/settings", async () => {
  const actual =
    await vi.importActual<typeof import("../../../query/settings")>("../../../query/settings");
  return { ...actual, useSettingsSetMutation: vi.fn() };
});

vi.mock("../../../query/wsl", async () => {
  const actual = await vi.importActual<typeof import("../../../query/wsl")>("../../../query/wsl");
  return { ...actual, useWslOverviewQuery: vi.fn(), useWslConfigureClientsMutation: vi.fn() };
});

describe("components/cli-manager/WslSettingsCard", () => {
  it("renders unavailable state when not available", () => {
    vi.mocked(useSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useAppAboutQuery).mockReturnValue({ data: null } as any);
    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: null,
      isFetched: false,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useWslConfigureClientsMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    render(<WslSettingsCard available={false} saving={false} settings={{} as any} />);

    expect(screen.getByText("WSL 配置")).toBeInTheDocument();
    expect(screen.getByText("数据不可用")).toBeInTheDocument();
  });

  it("refreshes overview and runs configure flow", async () => {
    const overviewRefetch = vi.fn().mockResolvedValue({ data: {} });
    vi.mocked(useSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useAppAboutQuery).mockReturnValue({ data: { os: "windows" } } as any);
    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: {
        detection: { detected: true, distros: ["Ubuntu"] },
        hostIp: "172.20.0.1",
        statusRows: [
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
        ],
      },
      isFetched: true,
      isFetching: false,
      refetch: overviewRefetch,
    } as any);

    const configureMutation = { isPending: false, mutateAsync: vi.fn() };
    configureMutation.mutateAsync.mockResolvedValue({ ok: true, message: "OK" });
    vi.mocked(useWslConfigureClientsMutation).mockReturnValue(configureMutation as any);

    const settings = {
      wsl_auto_config: true,
      wsl_target_cli: { claude: true, codex: false, gemini: true },
      gateway_listen_mode: "wsl_auto",
      wsl_host_address_mode: "auto",
      wsl_custom_host_address: "127.0.0.1",
      preferred_port: 37123,
      auto_start: false,
      log_retention_days: 7,
      failover_max_attempts_per_provider: 5,
      failover_max_providers_to_try: 5,
    } as any;

    render(<WslSettingsCard available={true} saving={false} settings={settings} />);

    expect(screen.getAllByText("Ubuntu").length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    await waitFor(() => expect(overviewRefetch).toHaveBeenCalled());

    // Configure now always configures all targets.
    fireEvent.click(screen.getByRole("button", { name: "立即配置" }));
    await waitFor(() => {
      expect(configureMutation.mutateAsync).toHaveBeenCalledWith();
    });
    await waitFor(() => expect(overviewRefetch).toHaveBeenCalledTimes(2));
  });

  it("shows configure guard toasts (unsupported OS / not detected)", async () => {
    vi.mocked(useSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useWslConfigureClientsMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: { detection: { detected: true, distros: [] }, hostIp: null, statusRows: null },
      isFetched: true,
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    const settings = {
      wsl_auto_config: true,
      wsl_target_cli: { claude: true, codex: false, gemini: false },
      gateway_listen_mode: "wsl_auto",
      wsl_host_address_mode: "auto",
      wsl_custom_host_address: "127.0.0.1",
      preferred_port: 37123,
      auto_start: false,
      log_retention_days: 7,
      failover_max_attempts_per_provider: 5,
      failover_max_providers_to_try: 5,
    } as any;

    // Unsupported OS.
    vi.mocked(useAppAboutQuery).mockReturnValue({ data: { os: "mac" } } as any);
    const { rerender } = render(
      <WslSettingsCard available={true} saving={false} settings={settings} />
    );
    expect(screen.getByText("仅 Windows 支持 WSL 配置")).toBeInTheDocument();

    // not detected.
    vi.mocked(toast).mockClear();
    vi.mocked(useAppAboutQuery).mockReturnValue({ data: { os: "windows" } } as any);
    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: { detection: { detected: false, distros: [] }, hostIp: null, statusRows: null },
      isFetched: true,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    rerender(<WslSettingsCard available={true} saving={false} settings={settings} />);
    fireEvent.click(screen.getByRole("button", { name: "立即配置" }));
    expect(toast).toHaveBeenCalledWith("未检测到 WSL");
  });

  it("handles listener setup failures without leaking unhandled rejections", async () => {
    vi.mocked(useSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useAppAboutQuery).mockReturnValue({ data: { os: "windows" } } as any);
    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: { detection: { detected: true, distros: ["Ubuntu"] }, hostIp: null, statusRows: [] },
      isFetched: true,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useWslConfigureClientsMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    vi.mocked(tauriListen).mockRejectedValueOnce(new Error("listen boom"));

    render(
      <WslSettingsCard
        available={true}
        saving={false}
        settings={
          {
            wsl_auto_config: true,
            wsl_target_cli: { claude: true, codex: false, gemini: false },
            gateway_listen_mode: "wsl_auto",
            wsl_host_address_mode: "auto",
            wsl_custom_host_address: "127.0.0.1",
            preferred_port: 37123,
            auto_start: false,
            log_retention_days: 7,
            failover_max_attempts_per_provider: 5,
            failover_max_providers_to_try: 5,
          } as any
        }
      />
    );

    await waitFor(() => expect(tauriListen).toHaveBeenCalled());
  });

  it("handles configure report null + failure fallback + errors", async () => {
    vi.mocked(useSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useAppAboutQuery).mockReturnValue({ data: { os: "windows" } } as any);

    const overviewRefetch = vi.fn().mockResolvedValue({ data: {} });
    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: { detection: { detected: true, distros: ["Ubuntu"] }, hostIp: null, statusRows: [] },
      isFetched: true,
      isFetching: false,
      refetch: overviewRefetch,
    } as any);

    const configureMutation = { isPending: false, mutateAsync: vi.fn() };
    vi.mocked(useWslConfigureClientsMutation).mockReturnValue(configureMutation as any);

    const settings = {
      wsl_auto_config: true,
      wsl_target_cli: { claude: true, codex: false, gemini: false },
      gateway_listen_mode: "wsl_auto",
      wsl_host_address_mode: "auto",
      wsl_custom_host_address: "127.0.0.1",
      preferred_port: 37123,
      auto_start: false,
      log_retention_days: 7,
      failover_max_attempts_per_provider: 5,
      failover_max_providers_to_try: 5,
    } as any;

    const { rerender } = render(
      <WslSettingsCard available={true} saving={false} settings={settings} />
    );

    // report=null -> silent return.
    configureMutation.mutateAsync.mockResolvedValueOnce(null);
    fireEvent.click(screen.getByRole("button", { name: "立即配置" }));
    await waitFor(() => expect(configureMutation.mutateAsync).toHaveBeenCalledTimes(1));

    // ok=false + empty message -> fallback "配置失败" + refresh called.
    vi.mocked(toast).mockClear();
    configureMutation.mutateAsync.mockResolvedValueOnce({ ok: false, message: "" });
    fireEvent.click(screen.getByRole("button", { name: "立即配置" }));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("配置失败"));
    await waitFor(() => expect(overviewRefetch).toHaveBeenCalled());

    // ok=true + empty message -> fallback "配置成功" + refresh called.
    vi.mocked(toast).mockClear();
    configureMutation.mutateAsync.mockResolvedValueOnce({ ok: true, message: "" });
    fireEvent.click(screen.getByRole("button", { name: "立即配置" }));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("配置成功"));
    await waitFor(() => expect(overviewRefetch).toHaveBeenCalled());

    // throw -> error toast.
    vi.mocked(toast).mockClear();
    configureMutation.mutateAsync.mockRejectedValueOnce(new Error("boom"));
    fireEvent.click(screen.getByRole("button", { name: "立即配置" }));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("WSL 一键配置失败：请查看控制台日志"));

    // refreshAll catch path on refresh button.
    vi.mocked(toast).mockClear();
    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: { detection: { detected: true, distros: ["Ubuntu"] }, hostIp: null, statusRows: [] },
      isFetched: true,
      isFetching: false,
      refetch: vi.fn().mockRejectedValue(new Error("nope")),
    } as any);
    rerender(<WslSettingsCard available={true} saving={false} settings={settings} />);
    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("刷新 WSL 状态失败：请稍后重试"));
  });

  it("renders config status table when statusRows available", () => {
    vi.mocked(useSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useAppAboutQuery).mockReturnValue({ data: { os: "windows" } } as any);
    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: {
        detection: { detected: true, distros: ["Ubuntu-22.04", "Debian"] },
        hostIp: "172.20.0.1",
        statusRows: [
          {
            distro: "Ubuntu-22.04",
            claude: true,
            codex: true,
            gemini: true,
            claude_mcp: true,
            codex_mcp: true,
            gemini_mcp: true,
            claude_prompt: true,
            codex_prompt: true,
            gemini_prompt: true,
          },
          {
            distro: "Debian",
            claude: true,
            codex: false,
            gemini: true,
            claude_mcp: false,
            codex_mcp: false,
            gemini_mcp: true,
            claude_prompt: false,
            codex_prompt: false,
            gemini_prompt: true,
          },
        ],
      },
      isFetched: true,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useWslConfigureClientsMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    render(
      <WslSettingsCard
        available={true}
        saving={false}
        settings={
          {
            gateway_listen_mode: "wsl_auto",
            wsl_host_address_mode: "auto",
            wsl_custom_host_address: "127.0.0.1",
            preferred_port: 37123,
            auto_start: false,
            log_retention_days: 7,
            failover_max_attempts_per_provider: 5,
            failover_max_providers_to_try: 5,
          } as any
        }
      />
    );

    expect(screen.getByText("配置状态")).toBeInTheDocument();
    expect(screen.getAllByText("Ubuntu-22.04").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Debian").length).toBeGreaterThan(0);

    // Table headers
    expect(screen.getByText("Claude Code")).toBeInTheDocument();
    expect(screen.getByText("Codex")).toBeInTheDocument();
    expect(screen.getByText("Gemini")).toBeInTheDocument();
    expect(screen.getAllByTitle("Auth: yes, MCP: yes, Prompt: yes").length).toBeGreaterThan(0);
  });

  it("shows localhost listen mode warning", () => {
    vi.mocked(useSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useAppAboutQuery).mockReturnValue({ data: { os: "windows" } } as any);
    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: { detection: { detected: false, distros: [] }, hostIp: null, statusRows: null },
      isFetched: true,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useWslConfigureClientsMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    render(
      <WslSettingsCard
        available={true}
        saving={false}
        settings={
          {
            gateway_listen_mode: "localhost",
            wsl_auto_config: true,
            wsl_host_address_mode: "auto",
            wsl_custom_host_address: "127.0.0.1",
            preferred_port: 37123,
            auto_start: false,
            log_retention_days: 7,
            failover_max_attempts_per_provider: 5,
            failover_max_providers_to_try: 5,
          } as any
        }
      />
    );

    expect(
      screen.getByText(/当前监听模式为"仅本地"，WSL 无法访问网关。启动时会提示切换监听模式。/)
    ).toBeInTheDocument();
  });

  it("does not show localhost warning when listen mode is not localhost", () => {
    vi.mocked(useSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useAppAboutQuery).mockReturnValue({ data: { os: "windows" } } as any);
    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: { detection: { detected: false, distros: [] }, hostIp: null, statusRows: null },
      isFetched: true,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useWslConfigureClientsMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    render(
      <WslSettingsCard
        available={true}
        saving={false}
        settings={
          {
            gateway_listen_mode: "wsl_auto",
            wsl_host_address_mode: "auto",
            wsl_custom_host_address: "127.0.0.1",
            preferred_port: 37123,
            auto_start: false,
            log_retention_days: 7,
            failover_max_attempts_per_provider: 5,
            failover_max_providers_to_try: 5,
          } as any
        }
      />
    );

    expect(screen.queryByText(/当前监听模式为"仅本地"，WSL 无法访问网关/)).not.toBeInTheDocument();
  });

  it("shows auto-config hint", () => {
    vi.mocked(useSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useAppAboutQuery).mockReturnValue({ data: { os: "windows" } } as any);
    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: { detection: { detected: false, distros: [] }, hostIp: null, statusRows: null },
      isFetched: true,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useWslConfigureClientsMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    render(
      <WslSettingsCard
        available={true}
        saving={false}
        settings={
          {
            gateway_listen_mode: "wsl_auto",
            wsl_auto_config: true,
            wsl_host_address_mode: "auto",
            wsl_custom_host_address: "127.0.0.1",
            preferred_port: 37123,
            auto_start: false,
            log_retention_days: 7,
            failover_max_attempts_per_provider: 5,
            failover_max_providers_to_try: 5,
          } as any
        }
      />
    );

    expect(
      screen.getByText("已启用：应用启动时自动检测并配置 WSL 环境，修改相关设置时自动同步。")
    ).toBeInTheDocument();
  });

  it("listens for wsl:auto_config_result event and updates state", async () => {
    const overviewRefetch = vi.fn().mockResolvedValue({ data: {} });
    vi.mocked(useSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useAppAboutQuery).mockReturnValue({ data: { os: "windows" } } as any);
    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: { detection: { detected: true, distros: ["Ubuntu"] }, hostIp: null, statusRows: [] },
      isFetched: true,
      isFetching: false,
      refetch: overviewRefetch,
    } as any);
    vi.mocked(useWslConfigureClientsMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    render(
      <WslSettingsCard
        available={true}
        saving={false}
        settings={
          {
            gateway_listen_mode: "wsl_auto",
            wsl_host_address_mode: "auto",
            wsl_custom_host_address: "127.0.0.1",
            preferred_port: 37123,
            auto_start: false,
            log_retention_days: 7,
            failover_max_attempts_per_provider: 5,
            failover_max_providers_to_try: 5,
          } as any
        }
      />
    );

    // tauriListen should have been called with "wsl:auto_config_result"
    await waitFor(() => {
      expect(tauriListen).toHaveBeenCalledWith("wsl:auto_config_result", expect.any(Function));
    });

    // Emit the event and verify the report is displayed
    emitTauriEvent("wsl:auto_config_result", {
      ok: true,
      message: "自动配置完成",
      distros: [],
    });

    await waitFor(() => {
      expect(screen.getByText("自动配置完成")).toBeInTheDocument();
    });
    expect(overviewRefetch).toHaveBeenCalled();
  });

  it("shows distro config summary count", () => {
    vi.mocked(useSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useAppAboutQuery).mockReturnValue({ data: { os: "windows" } } as any);
    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: {
        detection: { detected: true, distros: ["Ubuntu", "Debian"] },
        hostIp: null,
        statusRows: [
          {
            distro: "Ubuntu",
            claude: true,
            codex: true,
            gemini: true,
            claude_mcp: true,
            codex_mcp: true,
            gemini_mcp: true,
            claude_prompt: true,
            codex_prompt: true,
            gemini_prompt: true,
          },
          {
            distro: "Debian",
            claude: false,
            codex: false,
            gemini: false,
            claude_mcp: false,
            codex_mcp: false,
            gemini_mcp: false,
            claude_prompt: false,
            codex_prompt: false,
            gemini_prompt: false,
          },
        ],
      },
      isFetched: true,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useWslConfigureClientsMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    render(
      <WslSettingsCard
        available={true}
        saving={false}
        settings={
          {
            gateway_listen_mode: "wsl_auto",
            wsl_host_address_mode: "auto",
            wsl_custom_host_address: "127.0.0.1",
            preferred_port: 37123,
            auto_start: false,
            log_retention_days: 7,
            failover_max_attempts_per_provider: 5,
            failover_max_providers_to_try: 5,
          } as any
        }
      />
    );

    // 1/2 because Ubuntu has at least one CLI configured, Debian has none
    expect(screen.getByText(/1\/\s*2 个 distro/)).toBeInTheDocument();
  });

  it("persists custom host address via advanced options", async () => {
    const settingsSetMutation = { isPending: false, mutateAsync: vi.fn() };
    settingsSetMutation.mutateAsync.mockResolvedValue({});
    vi.mocked(useSettingsSetMutation).mockReturnValue(settingsSetMutation as any);

    vi.mocked(useAppAboutQuery).mockReturnValue({ data: { os: "windows" } } as any);
    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: {
        detection: { detected: true, distros: ["Ubuntu"] },
        hostIp: "172.20.0.1",
        statusRows: [],
      },
      isFetched: true,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useWslConfigureClientsMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    vi.mocked(toast).mockClear();

    const settings = {
      gateway_listen_mode: "wsl_auto",
      wsl_host_address_mode: "auto",
      wsl_custom_host_address: "127.0.0.1",
      preferred_port: 37123,
      auto_start: false,
      log_retention_days: 7,
      failover_max_attempts_per_provider: 5,
      failover_max_providers_to_try: 5,
    } as any;

    render(<WslSettingsCard available={true} saving={false} settings={settings} />);

    // Open advanced section (details content is hidden from accessibility tree when collapsed).
    fireEvent.click(screen.getByText("高级选项（地址兜底）"));

    // Enable custom host address mode (second switch — first is "自动同步配置").
    const switches = screen.getAllByRole("switch", { hidden: true });
    fireEvent.click(switches[1]);

    await waitFor(() => {
      expect(settingsSetMutation.mutateAsync).toHaveBeenCalledWith({
        preferredPort: 37123,
        autoStart: false,
        logRetentionDays: 7,
        failoverMaxAttemptsPerProvider: 5,
        failoverMaxProvidersToTry: 5,
        wslHostAddressMode: "custom",
      });
    });
    expect(toast).toHaveBeenCalledWith("已保存");

    // Update custom host address.
    const input = screen.getByPlaceholderText("172.20.0.1");
    fireEvent.change(input, { target: { value: "172.20.0.99" } });
    fireEvent.blur(input);

    await waitFor(() => {
      expect(settingsSetMutation.mutateAsync).toHaveBeenCalledWith({
        preferredPort: 37123,
        autoStart: false,
        logRetentionDays: 7,
        failoverMaxAttemptsPerProvider: 5,
        failoverMaxProvidersToTry: 5,
        wslHostAddressMode: "custom",
        wslCustomHostAddress: "172.20.0.99",
      });
    });
  });

  it("toasts when custom host address is invalid and does not persist", async () => {
    const settingsSetMutation = { isPending: false, mutateAsync: vi.fn() };
    settingsSetMutation.mutateAsync.mockResolvedValue({});
    vi.mocked(useSettingsSetMutation).mockReturnValue(settingsSetMutation as any);

    vi.mocked(useAppAboutQuery).mockReturnValue({ data: { os: "windows" } } as any);
    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: {
        detection: { detected: true, distros: ["Ubuntu"] },
        hostIp: "172.20.0.1",
        statusRows: [],
      },
      isFetched: true,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useWslConfigureClientsMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    vi.mocked(toast).mockClear();

    const settings = {
      gateway_listen_mode: "wsl_auto",
      wsl_host_address_mode: "auto",
      wsl_custom_host_address: "127.0.0.1",
      preferred_port: 37123,
      auto_start: false,
      log_retention_days: 7,
      failover_max_attempts_per_provider: 5,
      failover_max_providers_to_try: 5,
    } as any;

    render(<WslSettingsCard available={true} saving={false} settings={settings} />);

    // Open advanced section and enable custom mode (second switch).
    fireEvent.click(screen.getByText("高级选项（地址兜底）"));
    const switches = screen.getAllByRole("switch", { hidden: true });
    fireEvent.click(switches[1]);
    await waitFor(() => {
      expect(settingsSetMutation.mutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({ wslHostAddressMode: "custom" })
      );
    });

    // Enter invalid address (contains port) and blur.
    vi.mocked(toast).mockClear();
    const input = screen.getByPlaceholderText("172.20.0.1");
    fireEvent.change(input, { target: { value: "172.20.0.1:123" } });
    fireEvent.blur(input);

    await waitFor(() => {
      expect(toast).toHaveBeenCalledWith("宿主机地址不支持端口；请只填写 IP（例如 172.20.0.1）");
    });
    expect(settingsSetMutation.mutateAsync).not.toHaveBeenCalledWith(
      expect.objectContaining({ wslCustomHostAddress: "172.20.0.1:123" })
    );
  });

  it("disables listen-mode confirmation while settings are saving", async () => {
    const settingsSetMutation = { isPending: false, mutateAsync: vi.fn() };
    vi.mocked(useSettingsSetMutation).mockReturnValue(settingsSetMutation as any);
    vi.mocked(useAppAboutQuery).mockReturnValue({ data: { os: "windows" } } as any);
    vi.mocked(useWslOverviewQuery).mockReturnValue({
      data: { detection: { detected: true, distros: ["Ubuntu"] }, hostIp: null, statusRows: [] },
      isFetched: true,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useWslConfigureClientsMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    render(
      <WslSettingsCard
        available={true}
        saving={true}
        settings={
          {
            gateway_listen_mode: "localhost",
            wsl_auto_config: true,
            wsl_host_address_mode: "auto",
            wsl_custom_host_address: "127.0.0.1",
            preferred_port: 37123,
            auto_start: false,
            log_retention_days: 7,
            failover_max_attempts_per_provider: 5,
            failover_max_providers_to_try: 5,
          } as any
        }
      />
    );

    await waitFor(() => expect(tauriListen).toHaveBeenCalled());
    emitTauriEvent("wsl:localhost_switch_prompt", null);

    expect(await screen.findByRole("button", { name: "切换" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "切换" }));
    expect(settingsSetMutation.mutateAsync).not.toHaveBeenCalled();
  });
});
