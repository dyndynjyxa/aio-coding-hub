import { QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { useTheme } from "../../../hooks/useTheme";
import { gatewayKeys } from "../../../query/keys";
import { logToConsole } from "../../../services/consoleLog";
import { gatewayStart, gatewayStop } from "../../../services/gateway";
import { createTestQueryClient } from "../../../test/utils/reactQuery";
import { SettingsMainColumn } from "../SettingsMainColumn";
import type { ComponentProps } from "react";

let latestOnDragEnds: Array<(event: any) => void> = [];
let sortableIsDragging = false;

vi.mock("@dnd-kit/core", () => ({
  DndContext: ({ children, onDragEnd }: any) => {
    if (onDragEnd) latestOnDragEnds.push(onDragEnd);
    return <div data-testid="dnd">{children}</div>;
  },
  PointerSensor: function PointerSensor() {},
  closestCenter: () => null,
  useSensor: () => null,
  useSensors: () => [],
}));

vi.mock("@dnd-kit/sortable", () => ({
  SortableContext: ({ children }: any) => <div data-testid="sortable">{children}</div>,
  arrayMove: (array: any[], from: number, to: number) => {
    const next = array.slice();
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item);
    return next;
  },
  useSortable: () => ({
    attributes: {},
    listeners: {},
    setNodeRef: () => {},
    transform: null,
    transition: undefined,
    isDragging: sortableIsDragging,
  }),
  horizontalListSortingStrategy: {},
}));

vi.mock("@dnd-kit/utilities", () => ({
  CSS: { Transform: { toString: () => "" } },
}));

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));
vi.mock("../../../hooks/useTheme", () => ({ useTheme: vi.fn() }));
vi.mock("../../../services/gateway", async () => {
  const actual = await vi.importActual<typeof import("../../../services/gateway")>(
    "../../../services/gateway"
  );
  return { ...actual, gatewayStart: vi.fn(), gatewayStop: vi.fn() };
});

function renderSettingsMainColumn(
  overrides: Partial<ComponentProps<typeof SettingsMainColumn>> = {}
) {
  const client = createTestQueryClient();
  const base: ComponentProps<typeof SettingsMainColumn> = {
    gateway: { running: false, port: null, base_url: null, listen_addr: null } as any,
    gatewayAvailable: "available",
    settingsReady: true,
    settingsReadErrorMessage: null,
    settingsWriteBlocked: false,
    settingsSaving: false,
    port: 37123,
    setPort: vi.fn(),
    showHomeHeatmap: true,
    setShowHomeHeatmap: vi.fn(),
    showHomeUsage: true,
    setShowHomeUsage: vi.fn(),
    homeUsagePeriod: "last15",
    setHomeUsagePeriod: vi.fn(),
    cliPriorityOrder: ["claude", "codex", "gemini"],
    setCliPriorityOrder: vi.fn(),
    commitNumberField: vi.fn(),
    autoStart: false,
    setAutoStart: vi.fn(),
    startMinimized: false,
    setStartMinimized: vi.fn(),
    trayEnabled: true,
    setTrayEnabled: vi.fn(),
    logRetentionDays: 30,
    setLogRetentionDays: vi.fn(),
    requestPersist: vi.fn(),
    noticePermissionStatus: "checking",
    requestingNoticePermission: false,
    sendingNoticeTest: false,
    requestSystemNotificationPermission: vi.fn().mockResolvedValue(undefined),
    sendSystemNotificationTest: vi.fn().mockResolvedValue(undefined),
  };

  return {
    client,
    ...render(
      <QueryClientProvider client={client}>
        <SettingsMainColumn {...base} {...overrides} />
      </QueryClientProvider>
    ),
  };
}

describe("pages/settings/SettingsMainColumn", () => {
  beforeEach(() => {
    window.localStorage.clear();
    latestOnDragEnds = [];
    sortableIsDragging = false;
  });

  it("switches theme from settings", () => {
    const setTheme = vi.fn();
    vi.mocked(useTheme).mockReturnValue({
      theme: "system",
      resolvedTheme: "light",
      setTheme,
    } as any);

    renderSettingsMainColumn();

    fireEvent.click(screen.getByRole("button", { name: "Light" }));
    expect(setTheme).toHaveBeenCalledWith("light");

    fireEvent.click(screen.getByRole("button", { name: "Dark" }));
    expect(setTheme).toHaveBeenCalledWith("dark");

    fireEvent.click(screen.getByRole("button", { name: "System" }));
    expect(setTheme).toHaveBeenCalledWith("system");
  });

  it("toggles homepage heatmap visibility setting", () => {
    const setShowHomeHeatmap = vi.fn();
    const requestPersist = vi.fn();
    vi.mocked(useTheme).mockReturnValue({
      theme: "system",
      resolvedTheme: "light",
      setTheme: vi.fn(),
    } as any);

    renderSettingsMainColumn({
      showHomeHeatmap: true,
      setShowHomeHeatmap,
      requestPersist,
    });

    const row = screen.getByText("显示首页热力图").parentElement?.parentElement;
    expect(row).toBeTruthy();
    fireEvent.click(within(row as HTMLElement).getByRole("switch"));
    expect(setShowHomeHeatmap).toHaveBeenCalledWith(false);
    expect(requestPersist).toHaveBeenCalledWith({ show_home_heatmap: false });
  });

  it("shows readonly protection and disables config writes after settings read failure", () => {
    vi.mocked(useTheme).mockReturnValue({
      theme: "system",
      resolvedTheme: "light",
      setTheme: vi.fn(),
    } as any);

    renderSettingsMainColumn({
      gateway: { running: true, port: 37123, base_url: null, listen_addr: null } as any,
      settingsReadErrorMessage:
        "设置文件读取失败，已进入只读保护。请先修复或恢复 settings.json 后刷新页面。",
      settingsWriteBlocked: true,
      autoStart: true,
    });

    expect(screen.getByRole("alert")).toHaveTextContent("已进入只读保护");
    expect(screen.getByRole("button", { name: "重启" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "停止" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "最近30天" })).toBeDisabled();

    const portRow = screen.getByText("监听端口").parentElement?.parentElement;
    expect(portRow).toBeTruthy();
    expect(within(portRow as HTMLElement).getByRole("spinbutton")).toBeDisabled();

    const trayRow = screen.getByText("托盘常驻").parentElement?.parentElement;
    expect(trayRow).toBeTruthy();
    expect(within(trayRow as HTMLElement).getByRole("switch")).toBeDisabled();
  });

  it("disables gateway controls while settings save is in flight", () => {
    vi.mocked(useTheme).mockReturnValue({
      theme: "system",
      resolvedTheme: "light",
      setTheme: vi.fn(),
    } as any);

    renderSettingsMainColumn({
      gateway: { running: true, port: 37123, base_url: null, listen_addr: null } as any,
      settingsSaving: true,
    });

    expect(screen.getByRole("button", { name: "重启" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "停止" })).toBeDisabled();
  });

  it("toggles homepage usage visibility setting", () => {
    const setShowHomeUsage = vi.fn();
    const requestPersist = vi.fn();
    vi.mocked(useTheme).mockReturnValue({
      theme: "system",
      resolvedTheme: "light",
      setTheme: vi.fn(),
    } as any);

    renderSettingsMainColumn({
      showHomeUsage: true,
      setShowHomeUsage,
      requestPersist,
    });

    const row = screen.getByText("显示首页用量统计").parentElement?.parentElement;
    expect(row).toBeTruthy();
    fireEvent.click(within(row as HTMLElement).getByRole("switch"));
    expect(setShowHomeUsage).toHaveBeenCalledWith(false);
    expect(requestPersist).toHaveBeenCalledWith({ show_home_usage: false });
  });

  it("persists homepage usage period setting", () => {
    const setHomeUsagePeriod = vi.fn();
    const requestPersist = vi.fn();
    vi.mocked(useTheme).mockReturnValue({
      theme: "system",
      resolvedTheme: "light",
      setTheme: vi.fn(),
    } as any);

    renderSettingsMainColumn({
      homeUsagePeriod: "last15",
      setHomeUsagePeriod,
      requestPersist,
    });

    fireEvent.click(screen.getByRole("button", { name: "最近30天" }));
    expect(setHomeUsagePeriod).toHaveBeenCalledWith("last30");
    expect(requestPersist).toHaveBeenCalledWith({ home_usage_period: "last30" });
  });

  it("reorders homepage overview tabs from settings", () => {
    vi.mocked(useTheme).mockReturnValue({
      theme: "system",
      resolvedTheme: "light",
      setTheme: vi.fn(),
    } as any);

    renderSettingsMainColumn();

    act(() => {
      latestOnDragEnds.forEach((onDragEnd) =>
        onDragEnd({
          active: { id: "providerLimit" },
          over: { id: "workspaceConfig" },
        })
      );
    });

    expect(screen.getByRole("button", { name: "供应商限额" })).toBeInTheDocument();
    expect(window.localStorage.getItem("aio-home-overview-tab-order")).toBe(
      JSON.stringify(["providerLimit", "workspaceConfig", "circuit", "sessions"])
    );
  });

  it("reorders CLI priority from settings", () => {
    const setCliPriorityOrder = vi.fn();
    const requestPersist = vi.fn();
    vi.mocked(useTheme).mockReturnValue({
      theme: "system",
      resolvedTheme: "light",
      setTheme: vi.fn(),
    } as any);

    renderSettingsMainColumn({
      cliPriorityOrder: ["claude", "codex", "gemini"],
      setCliPriorityOrder,
      requestPersist,
    });

    act(() => {
      latestOnDragEnds.forEach((onDragEnd) =>
        onDragEnd({
          active: { id: "gemini" },
          over: { id: "claude" },
        })
      );
    });

    expect(setCliPriorityOrder).toHaveBeenCalledWith(["gemini", "claude", "codex"]);
    expect(requestPersist).toHaveBeenCalledWith({
      cli_priority_order: ["gemini", "claude", "codex"],
    });
  });

  it.each([
    ["checking", "检查中"],
    ["granted", "已授权"],
    ["denied", "已拒绝"],
    ["not_granted", "未授权"],
    ["unknown", "未知"],
  ] as const)("renders notice permission status %s", (status, expected) => {
    vi.mocked(useTheme).mockReturnValue({
      theme: "system",
      resolvedTheme: "light",
      setTheme: vi.fn(),
    } as any);
    renderSettingsMainColumn({ noticePermissionStatus: status });
    expect(screen.getByText(expected)).toBeInTheDocument();
  });

  it("validates port before restarting gateway", () => {
    vi.mocked(useTheme).mockReturnValue({
      theme: "system",
      resolvedTheme: "light",
      setTheme: vi.fn(),
    } as any);
    renderSettingsMainColumn({
      gateway: { running: true, port: 37123, base_url: null, listen_addr: null } as any,
      port: 80,
    });

    fireEvent.click(screen.getByRole("button", { name: "重启" }));
    expect(toast).toHaveBeenCalledWith("端口号必须为 1024-65535");
    expect(gatewayStart).not.toHaveBeenCalled();
    expect(gatewayStop).not.toHaveBeenCalled();
  });

  it("toasts when gateway stop fails during restart", async () => {
    vi.mocked(useTheme).mockReturnValue({
      theme: "system",
      resolvedTheme: "light",
      setTheme: vi.fn(),
    } as any);
    vi.mocked(gatewayStop).mockResolvedValueOnce(null as any);
    vi.mocked(gatewayStart).mockResolvedValue({
      running: true,
      port: 37123,
      base_url: "http://127.0.0.1:37123",
      listen_addr: "127.0.0.1:37123",
    } as any);

    renderSettingsMainColumn({
      gateway: { running: true, port: 37123, base_url: null, listen_addr: null } as any,
      port: 37123,
    });

    fireEvent.click(screen.getByRole("button", { name: "重启" }));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("重启失败：无法停止网关"));
  });

  it("restarts gateway and persists toggles", async () => {
    vi.mocked(useTheme).mockReturnValue({
      theme: "system",
      resolvedTheme: "light",
      setTheme: vi.fn(),
    } as any);
    vi.mocked(gatewayStop).mockResolvedValue({
      running: false,
      port: null,
      base_url: null,
      listen_addr: null,
    } as any);
    vi.mocked(gatewayStart).mockResolvedValue({
      running: true,
      port: 37123,
      base_url: "http://127.0.0.1:37123",
      listen_addr: "127.0.0.1:37123",
    } as any);

    const setAutoStart = vi.fn();
    const setTrayEnabled = vi.fn();
    const setPort = vi.fn();
    const requestPersist = vi.fn();
    const commitNumberField = vi.fn();

    renderSettingsMainColumn({
      gateway: { running: true, port: 37123, base_url: null, listen_addr: null } as any,
      port: 37123,
      setPort,
      autoStart: false,
      setAutoStart,
      trayEnabled: true,
      setTrayEnabled,
      requestPersist,
      commitNumberField,
    });

    fireEvent.click(screen.getByRole("button", { name: "重启" }));
    await waitFor(() => expect(gatewayStart).toHaveBeenCalledWith(37123));
    expect(logToConsole).toHaveBeenCalledWith(
      "info",
      "启动本地网关",
      expect.objectContaining({ port: 37123, base_url: "http://127.0.0.1:37123" })
    );
    expect(toast).toHaveBeenCalledWith("本地网关已重启");

    // Persist switches.
    const autoRow = screen.getByText("开机自启").parentElement?.parentElement;
    expect(autoRow).toBeTruthy();
    fireEvent.click(within(autoRow as HTMLElement).getByRole("switch"));
    expect(setAutoStart).toHaveBeenCalledWith(true);
    expect(requestPersist).toHaveBeenCalledWith({ auto_start: true });

    const trayRow = screen.getByText("托盘常驻").parentElement?.parentElement;
    expect(trayRow).toBeTruthy();
    fireEvent.click(within(trayRow as HTMLElement).getByRole("switch"));
    expect(setTrayEnabled).toHaveBeenCalledWith(false);
    expect(requestPersist).toHaveBeenCalledWith({ tray_enabled: false });

    // Commit number fields.
    const portRow = screen.getByText("监听端口").parentElement?.parentElement;
    expect(portRow).toBeTruthy();
    const portInput = within(portRow as HTMLElement).getByRole("spinbutton");
    fireEvent.change(portInput, { target: { value: "40000" } });
    expect(setPort).toHaveBeenCalledWith(40000);
    fireEvent.blur(portInput);
    expect(commitNumberField).toHaveBeenCalledWith(
      expect.objectContaining({ key: "preferred_port" })
    );
  });

  it("stops gateway and triggers system notification actions", async () => {
    vi.mocked(useTheme).mockReturnValue({
      theme: "system",
      resolvedTheme: "light",
      setTheme: vi.fn(),
    } as any);
    vi.mocked(gatewayStop).mockResolvedValue({
      running: false,
      port: null,
      base_url: null,
      listen_addr: null,
    } as any);
    const requestPermission = vi.fn().mockResolvedValue(undefined);
    const sendTest = vi.fn().mockResolvedValue(undefined);

    const { client } = renderSettingsMainColumn({
      gateway: { running: true, port: 37123, base_url: null, listen_addr: null } as any,
      requestSystemNotificationPermission: requestPermission,
      sendSystemNotificationTest: sendTest,
    });

    fireEvent.click(screen.getByRole("button", { name: "停止" }));
    await waitFor(() => expect(gatewayStop).toHaveBeenCalled());
    expect(logToConsole).toHaveBeenCalledWith("info", "停止本地网关");
    expect(toast).toHaveBeenCalledWith("本地网关已停止");
    expect(client.getQueryData(gatewayKeys.status())).toEqual({
      running: false,
      port: null,
      base_url: null,
      listen_addr: null,
    });

    fireEvent.click(screen.getByRole("button", { name: "请求通知权限" }));
    await waitFor(() => expect(requestPermission).toHaveBeenCalled());

    fireEvent.click(screen.getByRole("button", { name: "发送测试通知" }));
    await waitFor(() => expect(sendTest).toHaveBeenCalled());
  });
});
