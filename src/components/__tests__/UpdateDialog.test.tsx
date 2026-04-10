import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AIO_RELEASES_URL } from "../../constants/urls";
import { logToConsole } from "../../services/consoleLog";
import { appRestart } from "../../services/app/dataManagement";
import {
  updateDialogSetOpen,
  updateDownloadAndInstall,
  useUpdateMeta,
} from "../../hooks/useUpdateMeta";
import { UpdateDialog } from "../UpdateDialog";

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { loading: vi.fn().mockReturnValue("toast-id") }),
}));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("../../services/consoleLog", () => ({ logToConsole: vi.fn() }));
vi.mock("../../services/app/dataManagement", async () => {
  const actual = await vi.importActual<typeof import("../../services/app/dataManagement")>(
    "../../services/app/dataManagement"
  );
  return { ...actual, appRestart: vi.fn() };
});
vi.mock("../../hooks/useUpdateMeta", async () => {
  const actual = await vi.importActual<typeof import("../../hooks/useUpdateMeta")>(
    "../../hooks/useUpdateMeta"
  );
  return {
    ...actual,
    useUpdateMeta: vi.fn(),
    updateDialogSetOpen: vi.fn(),
    updateDownloadAndInstall: vi.fn(),
  };
});

describe("components/UpdateDialog", () => {
  it("renders fallback state when no update candidate is available", () => {
    vi.mocked(useUpdateMeta).mockReturnValue({
      about: null,
      updateCandidate: null,
      checkingUpdate: false,
      dialogOpen: true,
      installingUpdate: false,
      installError: null,
      installTotalBytes: null,
      installDownloadedBytes: 0,
    } as any);

    render(<UpdateDialog />);

    expect(screen.getAllByText("—")).toHaveLength(2);
    expect(screen.getByText("未发现可安装更新。")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "下载并安装" })).toBeDisabled();
  });

  it("renders publish date, installing progress, and install error state", () => {
    vi.mocked(useUpdateMeta).mockReturnValue({
      about: { run_mode: "desktop", app_version: "0.0.0" },
      updateCandidate: {
        version: "1.2.0",
        currentVersion: "1.1.0",
        date: Date.UTC(2026, 2, 24, 8, 0, 0),
        rid: "rid",
      },
      checkingUpdate: false,
      dialogOpen: true,
      installingUpdate: true,
      installError: "磁盘空间不足",
      installTotalBytes: 2048,
      installDownloadedBytes: 1024,
    } as any);

    render(<UpdateDialog />);

    expect(screen.getByText("发布日期")).toBeInTheDocument();
    expect(screen.getByText("下载并安装中…")).toBeInTheDocument();
    expect(screen.getByText(/1\.0 KB\s*\/\s*2\.0 KB/)).toBeInTheDocument();
    expect(screen.getByText("安装失败：磁盘空间不足")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "取消" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "安装中…" })).toBeDisabled();
  });

  it("toasts when download/install is unavailable in non-portable mode", async () => {
    vi.mocked(useUpdateMeta).mockReturnValue({
      about: { run_mode: "desktop", app_version: "0.0.0" },
      updateCandidate: { version: "1.0.0", currentVersion: "0.0.0", date: null, rid: "rid" },
      checkingUpdate: false,
      dialogOpen: true,
      installingUpdate: false,
      installError: null,
      installTotalBytes: null,
      installDownloadedBytes: 0,
    } as any);

    vi.mocked(updateDownloadAndInstall).mockResolvedValue(null);

    render(<UpdateDialog />);

    fireEvent.click(screen.getByRole("button", { name: "下载并安装" }));

    await waitFor(() => {
      expect(updateDownloadAndInstall).toHaveBeenCalled();
    });
    expect(updateDialogSetOpen).toHaveBeenCalledWith(false);
  });

  it("does not close the dialog when installer explicitly returns false", async () => {
    vi.mocked(useUpdateMeta).mockReturnValue({
      about: { run_mode: "desktop", app_version: "0.0.0" },
      updateCandidate: { version: "1.0.0", currentVersion: "0.0.0", date: null, rid: "rid" },
      checkingUpdate: false,
      dialogOpen: true,
      installingUpdate: false,
      installError: null,
      installTotalBytes: null,
      installDownloadedBytes: 0,
    } as any);

    vi.mocked(updateDownloadAndInstall).mockResolvedValue(false);

    render(<UpdateDialog />);

    fireEvent.click(screen.getByRole("button", { name: "下载并安装" }));

    await waitFor(() => {
      expect(updateDownloadAndInstall).toHaveBeenCalled();
    });
    expect(updateDialogSetOpen).not.toHaveBeenCalled();
  });

  it("logs and toasts when restart fails after install", async () => {
    vi.useFakeTimers();
    try {
      vi.mocked(useUpdateMeta).mockReturnValue({
        about: { run_mode: "desktop", app_version: "0.0.0" },
        updateCandidate: {
          version: "1.0.0",
          currentVersion: "0.0.0",
          date: Date.UTC(2026, 2, 24, 8, 0, 0),
          rid: "rid",
        },
        checkingUpdate: false,
        dialogOpen: true,
        installingUpdate: false,
        installError: null,
        installTotalBytes: null,
        installDownloadedBytes: 0,
      } as any);

      vi.mocked(updateDownloadAndInstall).mockResolvedValue(true);
      vi.mocked(appRestart).mockRejectedValue(new Error("restart boom"));

      render(<UpdateDialog />);

      fireEvent.click(screen.getByRole("button", { name: "下载并安装" }));

      await Promise.resolve();
      await vi.advanceTimersByTimeAsync(3000);

      await Promise.resolve();
      await Promise.resolve();

      expect(logToConsole).toHaveBeenCalledWith(
        "error",
        "自动重启失败",
        expect.objectContaining({ error: expect.stringContaining("restart boom") })
      );
      expect(toast).toHaveBeenCalledWith("自动重启失败：请手动重启应用以生效", expect.any(Object));
    } finally {
      vi.useRealTimers();
    }
  });

  it("runs restart countdown after a successful install", async () => {
    vi.useFakeTimers();
    try {
      vi.mocked(useUpdateMeta).mockReturnValue({
        about: { run_mode: "desktop", app_version: "0.0.0" },
        updateCandidate: { version: "1.0.0", currentVersion: "0.0.0", date: 0, rid: "rid" },
        checkingUpdate: false,
        dialogOpen: true,
        installingUpdate: false,
        installError: null,
        installTotalBytes: null,
        installDownloadedBytes: 0,
      } as any);

      vi.mocked(updateDownloadAndInstall).mockResolvedValue(true);
      vi.mocked(appRestart).mockResolvedValue(false);

      render(<UpdateDialog />);

      fireEvent.click(screen.getByRole("button", { name: "下载并安装" }));

      // flush the awaited updateDownloadAndInstall promise
      await Promise.resolve();

      expect(updateDialogSetOpen).toHaveBeenCalledWith(false);
      expect((toast as any).loading).toHaveBeenCalledWith("准备重启（3s）");

      await vi.advanceTimersByTimeAsync(3000);
      await Promise.resolve();

      expect(appRestart).toHaveBeenCalled();
      expect(toast).toHaveBeenCalledWith("更新已安装：请手动重启应用以生效", expect.any(Object));
    } finally {
      vi.useRealTimers();
    }
  });

  it("opens releases successfully in portable mode", async () => {
    vi.mocked(useUpdateMeta).mockReturnValue({
      about: { run_mode: "portable", app_version: "0.0.0" },
      updateCandidate: { version: "1.0.0", currentVersion: "0.0.0", date: null, rid: "rid" },
      checkingUpdate: false,
      dialogOpen: true,
      installingUpdate: false,
      installError: null,
      installTotalBytes: null,
      installDownloadedBytes: 0,
    } as any);

    vi.mocked(openUrl).mockResolvedValue(undefined as never);

    render(<UpdateDialog />);

    fireEvent.click(screen.getByRole("button", { name: "打开下载页" }));

    await waitFor(() => {
      expect(openUrl).toHaveBeenCalledWith(AIO_RELEASES_URL);
    });
    expect(logToConsole).not.toHaveBeenCalled();
  });

  it("opens releases (portable mode) and reports openUrl errors", async () => {
    vi.mocked(useUpdateMeta).mockReturnValue({
      about: { run_mode: "portable", app_version: "0.0.0" },
      updateCandidate: { version: "1.0.0", currentVersion: "0.0.0", date: null, rid: "rid" },
      checkingUpdate: false,
      dialogOpen: true,
      installingUpdate: false,
      installError: null,
      installTotalBytes: null,
      installDownloadedBytes: 0,
    } as any);

    vi.mocked(openUrl).mockRejectedValue(new Error("blocked"));
    vi.spyOn(window, "open").mockImplementation(() => null);

    render(<UpdateDialog />);

    fireEvent.click(screen.getByRole("button", { name: "打开下载页" }));

    await waitFor(() => {
      expect(logToConsole).toHaveBeenCalledWith("error", "打开 Releases 失败", expect.any(Object));
    });
    expect(toast).toHaveBeenCalledWith("打开下载页失败：请查看控制台日志");
  });
});
