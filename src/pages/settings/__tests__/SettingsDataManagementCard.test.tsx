import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SettingsDataManagementCard } from "../SettingsDataManagementCard";
import type { AppAboutInfo } from "../../../services/app/appAbout";
import type { DbDiskUsage } from "../../../services/app/dataManagement";

function createAboutInfo(overrides: Partial<AppAboutInfo> = {}): AppAboutInfo {
  return {
    app_version: "1.0.0",
    platform: "darwin",
    os_version: "14.0",
    arch: "arm64",
    data_dir_path: "/path/to/data",
    log_dir_path: "/path/to/logs",
    ...overrides,
  } as AppAboutInfo;
}

function createDbDiskUsage(overrides: Partial<DbDiskUsage> = {}): DbDiskUsage {
  return {
    db_bytes: 1024 * 1024 * 40,
    wal_bytes: 1024 * 1024 * 5,
    shm_bytes: 1024 * 1024 * 5,
    total_bytes: 1024 * 1024 * 50, // 50 MB
    ...overrides,
  };
}

describe("pages/settings/SettingsDataManagementCard", () => {
  const defaultProps = {
    about: createAboutInfo(),
    dbDiskUsageAvailable: "available" as const,
    dbDiskUsage: createDbDiskUsage(),
    requestLogRetentionDays: 0,
    refreshDbDiskUsage: vi.fn().mockResolvedValue(undefined),
    openAppDataDir: vi.fn().mockResolvedValue(undefined),
    onCompactDb: vi.fn().mockResolvedValue(undefined),
    compactingDb: false,
    openClearRequestLogsDialog: vi.fn(),
    openResetAllDialog: vi.fn(),
    onExportConfig: vi.fn().mockResolvedValue(undefined),
    onImportConfig: vi.fn(),
    exportingConfig: false,
  };

  it("renders card title", () => {
    render(<SettingsDataManagementCard {...defaultProps} />);
    expect(screen.getByText("数据管理")).toBeInTheDocument();
  });

  it("renders open data directory button", () => {
    render(<SettingsDataManagementCard {...defaultProps} />);
    expect(screen.getByText("打开数据/日志目录")).toBeInTheDocument();
  });

  it("calls openAppDataDir when clicking open directory button", () => {
    const openAppDataDir = vi.fn().mockResolvedValue(undefined);
    render(<SettingsDataManagementCard {...defaultProps} openAppDataDir={openAppDataDir} />);

    fireEvent.click(screen.getByText("打开数据/日志目录"));
    expect(openAppDataDir).toHaveBeenCalled();
  });

  it("displays disk usage when available", () => {
    render(<SettingsDataManagementCard {...defaultProps} />);
    expect(screen.getByText("50.0 MB")).toBeInTheDocument();
  });

  it("displays loading state when checking disk usage", () => {
    render(<SettingsDataManagementCard {...defaultProps} dbDiskUsageAvailable="checking" />);
    expect(screen.getByText("加载中…")).toBeInTheDocument();
  });

  it("displays dash when disk usage unavailable", () => {
    render(<SettingsDataManagementCard {...defaultProps} dbDiskUsageAvailable="unavailable" />);
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("calls refreshDbDiskUsage when clicking refresh button", () => {
    const refreshDbDiskUsage = vi.fn().mockResolvedValue(undefined);
    render(
      <SettingsDataManagementCard {...defaultProps} refreshDbDiskUsage={refreshDbDiskUsage} />
    );

    fireEvent.click(screen.getByText("刷新"));
    expect(refreshDbDiskUsage).toHaveBeenCalled();
  });

  it("shows permanent retention label when retention days is 0", () => {
    render(<SettingsDataManagementCard {...defaultProps} requestLogRetentionDays={0} />);
    expect(screen.getByText("永久保留")).toBeInTheDocument();
  });

  it("shows day-count retention label when retention days is positive", () => {
    render(<SettingsDataManagementCard {...defaultProps} requestLogRetentionDays={30} />);
    expect(screen.getByText("30 天")).toBeInTheDocument();
  });

  it("shows dash when retention days is unknown", () => {
    render(
      <SettingsDataManagementCard
        {...defaultProps}
        requestLogRetentionDays={null}
        dbDiskUsage={null}
      />
    );
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("calls onCompactDb when clicking compact button", () => {
    const onCompactDb = vi.fn().mockResolvedValue(undefined);
    render(<SettingsDataManagementCard {...defaultProps} onCompactDb={onCompactDb} />);

    fireEvent.click(screen.getByRole("button", { name: "压缩" }));
    expect(onCompactDb).toHaveBeenCalled();
  });

  it("shows compacting state and disables compact button while compacting", () => {
    render(<SettingsDataManagementCard {...defaultProps} compactingDb={true} />);
    expect(screen.getByRole("button", { name: "压缩中…" })).toBeDisabled();
  });

  it("disables compact button when about is null", () => {
    render(<SettingsDataManagementCard {...defaultProps} about={null} />);
    expect(screen.getByRole("button", { name: "压缩" })).toBeDisabled();
  });

  it("calls openClearRequestLogsDialog when clicking clear logs button", () => {
    const openClearRequestLogsDialog = vi.fn();
    render(
      <SettingsDataManagementCard
        {...defaultProps}
        openClearRequestLogsDialog={openClearRequestLogsDialog}
      />
    );

    // There are two "清理" buttons, get all and click the first one (clear logs)
    const clearButtons = screen.getAllByRole("button", { name: "清理" });
    fireEvent.click(clearButtons[0]);
    expect(openClearRequestLogsDialog).toHaveBeenCalled();
  });

  it("calls openResetAllDialog when clicking reset all button", () => {
    const openResetAllDialog = vi.fn();
    render(
      <SettingsDataManagementCard {...defaultProps} openResetAllDialog={openResetAllDialog} />
    );

    const clearButtons = screen.getAllByRole("button", { name: "清理" });
    fireEvent.click(clearButtons[1]); // Second "清理" button is for reset all
    expect(openResetAllDialog).toHaveBeenCalled();
  });

  it("calls onExportConfig when clicking export button", () => {
    const onExportConfig = vi.fn().mockResolvedValue(undefined);
    render(<SettingsDataManagementCard {...defaultProps} onExportConfig={onExportConfig} />);

    fireEvent.click(screen.getByRole("button", { name: "导出配置" }));
    expect(onExportConfig).toHaveBeenCalled();
  });

  it("shows exporting state when exportingConfig is true", () => {
    render(<SettingsDataManagementCard {...defaultProps} exportingConfig={true} />);
    expect(screen.getByRole("button", { name: "导出中…" })).toBeInTheDocument();
  });

  it("calls onImportConfig when clicking import button", () => {
    const onImportConfig = vi.fn();
    render(<SettingsDataManagementCard {...defaultProps} onImportConfig={onImportConfig} />);

    fireEvent.click(screen.getByRole("button", { name: "导入配置" }));
    expect(onImportConfig).toHaveBeenCalled();
  });

  it("disables buttons when about is null", () => {
    render(<SettingsDataManagementCard {...defaultProps} about={null} />);

    expect(screen.getByRole("button", { name: "打开数据/日志目录" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "导出配置" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "导入配置" })).toBeDisabled();
  });

  it("disables refresh button when checking disk usage", () => {
    render(<SettingsDataManagementCard {...defaultProps} dbDiskUsageAvailable="checking" />);

    expect(screen.getByText("刷新")).toBeDisabled();
  });

  it("renders warning message about sensitive data in export", () => {
    render(<SettingsDataManagementCard {...defaultProps} />);
    expect(screen.getByText("包含 API Key 等敏感信息，请妥善保管")).toBeInTheDocument();
  });
});
