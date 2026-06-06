import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SettingsPage } from "../SettingsPage";
import { useGatewayMeta } from "../../../hooks/useGatewayMeta";
import { useUpdateMeta } from "../../../hooks/useUpdateMeta";
import { useSettingsPersistence } from "../useSettingsPersistence";
import { useSystemNotification } from "../useSystemNotification";

let lastMainColumnProps: any = null;
let lastSidebarProps: any = null;

vi.mock("../../../hooks/useGatewayMeta", async () => {
  const actual = await vi.importActual<typeof import("../../../hooks/useGatewayMeta")>(
    "../../../hooks/useGatewayMeta"
  );
  return { ...actual, useGatewayMeta: vi.fn() };
});

vi.mock("../../../hooks/useUpdateMeta", async () => {
  const actual = await vi.importActual<typeof import("../../../hooks/useUpdateMeta")>(
    "../../../hooks/useUpdateMeta"
  );
  return { ...actual, useUpdateMeta: vi.fn() };
});

vi.mock("../useSettingsPersistence", async () => {
  const actual = await vi.importActual<typeof import("../useSettingsPersistence")>(
    "../useSettingsPersistence"
  );
  return { ...actual, useSettingsPersistence: vi.fn() };
});

vi.mock("../useSystemNotification", async () => {
  const actual = await vi.importActual<typeof import("../useSystemNotification")>(
    "../useSystemNotification"
  );
  return { ...actual, useSystemNotification: vi.fn() };
});

vi.mock("../SettingsMainColumn", () => ({
  SettingsMainColumn: (props: any) => {
    lastMainColumnProps = props;
    return <div data-testid="settings-main-column" />;
  },
}));

vi.mock("../SettingsSidebar", () => ({
  SettingsSidebar: (props: any) => {
    lastSidebarProps = props;
    return <div data-testid="settings-sidebar" />;
  },
}));

describe("pages/settings/SettingsPage", () => {
  it("wires gateway/update/persistence/notification into SettingsMainColumn/Sidebar", () => {
    vi.mocked(useGatewayMeta).mockReturnValue({
      gateway: { status: "running" },
      gatewayAvailable: true,
    } as any);

    vi.mocked(useUpdateMeta).mockReturnValue({
      about: { app_version: "0.0.0", run_mode: "installed" },
      dialogOpen: false,
    } as any);

    vi.mocked(useSettingsPersistence).mockReturnValue({
      settingsReady: true,
      settingsReadErrorMessage: null,
      settingsWriteBlocked: false,
      settingsSaving: false,
      port: "1234",
      setPort: vi.fn(),
      showHomeHeatmap: true,
      setShowHomeHeatmap: vi.fn(),
      showHomeUsage: true,
      setShowHomeUsage: vi.fn(),
      homeUsagePeriod: "last15",
      setHomeUsagePeriod: vi.fn(),
      commitNumberField: vi.fn(),
      autoStart: false,
      setAutoStart: vi.fn(),
      startMinimized: false,
      setStartMinimized: vi.fn(),
      trayEnabled: true,
      setTrayEnabled: vi.fn(),
      logRetentionDays: 7,
      setLogRetentionDays: vi.fn(),
      enableDebugLog: false,
      setEnableDebugLog: vi.fn(),
      enableAssociationAudit: true,
      setEnableAssociationAudit: vi.fn(),
      associationAuditProviderId: 12,
      setAssociationAuditProviderId: vi.fn(),
      associationAuditModel: "claude-sonnet-4-5",
      setAssociationAuditModel: vi.fn(),
      associationAuditMode: "prefiltered",
      setAssociationAuditMode: vi.fn(),
      associationAuditSampleRate: 10,
      setAssociationAuditSampleRate: vi.fn(),
      associationAuditTimeoutSeconds: 8,
      setAssociationAuditTimeoutSeconds: vi.fn(),
      associationAuditMaxInputChars: 6000,
      setAssociationAuditMaxInputChars: vi.fn(),
      associationAuditMaxOutputChars: 12000,
      setAssociationAuditMaxOutputChars: vi.fn(),
      requestPersist: vi.fn(),
    } as any);

    vi.mocked(useSystemNotification).mockReturnValue({
      noticePermissionStatus: "unknown",
      requestingNoticePermission: false,
      sendingNoticeTest: false,
      requestSystemNotificationPermission: vi.fn(),
      sendSystemNotificationTest: vi.fn(),
    } as any);

    render(<SettingsPage />);

    expect(screen.getByRole("heading", { level: 1, name: "设置" })).toBeInTheDocument();
    expect(screen.getByTestId("settings-main-column")).toBeInTheDocument();
    expect(screen.getByTestId("settings-sidebar")).toBeInTheDocument();

    expect(lastMainColumnProps?.gatewayAvailable).toBe(true);
    expect(lastMainColumnProps?.settingsWriteBlocked).toBe(false);
    expect(lastMainColumnProps?.enableAssociationAudit).toBe(true);
    expect(lastMainColumnProps?.associationAuditProviderId).toBe(12);
    expect(lastMainColumnProps?.associationAuditModel).toBe("claude-sonnet-4-5");
    expect(lastMainColumnProps?.associationAuditMode).toBe("prefiltered");
    expect(lastMainColumnProps?.associationAuditMaxOutputChars).toBe(12000);
    expect(lastSidebarProps?.updateMeta).toBeTruthy();
  });
});
