import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { Sidebar } from "../Sidebar";
import { AIO_RELEASES_URL, AIO_REPO_URL } from "../../constants/urls";

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

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

describe("ui/Sidebar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    devPreviewRef.current = { enabled: false, setEnabled: vi.fn(), toggle: vi.fn() };
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

    expect(screen.getByText("检查中")).toBeInTheDocument();
    expect(screen.queryByText("NEW")).not.toBeInTheDocument();
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("renders the GitHub link before the app name when no update candidate exists", () => {
    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    const repoLink = screen.getByRole("link", { name: "AIO Coding Hub GitHub 仓库" });
    const title = screen.getByText("AIO Coding Hub");

    expect(repoLink).toHaveAttribute("href", AIO_REPO_URL);
    expect(repoLink.compareDocumentPosition(title) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
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

    expect(screen.getByRole("link", { name: "AIO Coding Hub GitHub 仓库" })).toHaveAttribute(
      "href",
      AIO_REPO_URL
    );
    fireEvent.click(screen.getByRole("button", { name: "NEW" }));
    expect(updateDialogSetOpenMock).toHaveBeenCalledWith(true);
  });

  it("opens releases page when update candidate exists and app is portable", async () => {
    const { openUrl } = await import("@tauri-apps/plugin-opener");

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

    vi.mocked(openUrl).mockRejectedValue(new Error("boom"));
    const windowOpen = vi.spyOn(window, "open").mockImplementation(() => null as any);

    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("button", { name: "NEW" }));

    await waitFor(() => {
      expect(openUrl).toHaveBeenCalledWith(AIO_RELEASES_URL);
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

    fireEvent.click(screen.getByRole("button", { name: "NEW" }));
    expect(updateDialogSetOpenMock).toHaveBeenCalledWith(true);
  });

  it("calls onNavClick when a nav item is clicked", () => {
    const onNavClick = vi.fn();
    render(
      <MemoryRouter>
        <Sidebar onNavClick={onNavClick} />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByText("首页"));
    expect(onNavClick).toHaveBeenCalledTimes(1);
  });

  it("uses stopped tone for the port pill when gateway is stopped", () => {
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

    const portPill = screen.getByText("37123");
    expect(portPill.className).toContain("bg-slate-100");
    expect(portPill.className).not.toContain("bg-emerald-50");
  });
});
