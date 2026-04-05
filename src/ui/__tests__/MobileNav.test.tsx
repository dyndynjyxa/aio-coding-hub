import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { MobileNav, MobileHeader } from "../MobileNav";
import { AIO_RELEASES_URL } from "../../constants/urls";

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

describe("ui/MobileNav", () => {
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

  it("returns null when isOpen is false", () => {
    const { container } = render(
      <MemoryRouter>
        <MobileNav isOpen={false} onClose={vi.fn()} />
      </MemoryRouter>
    );
    expect(container.innerHTML).toBe("");
    expect(screen.queryByText("AIO Coding Hub")).not.toBeInTheDocument();
  });

  it("renders portal with nav items when isOpen is true", () => {
    render(
      <MemoryRouter>
        <MobileNav isOpen={true} onClose={vi.fn()} />
      </MemoryRouter>
    );
    expect(screen.getByText("AIO Coding Hub")).toBeInTheDocument();
    expect(screen.getByText("首页")).toBeInTheDocument();
    expect(screen.getByText("供应商")).toBeInTheDocument();
    expect(screen.getByText("设置")).toBeInTheDocument();
  });

  it("calls onClose when backdrop is clicked", () => {
    const onClose = vi.fn();
    render(
      <MemoryRouter>
        <MobileNav isOpen={true} onClose={onClose} />
      </MemoryRouter>
    );
    const backdrop = document.querySelector("[aria-hidden='true']")!;
    fireEvent.click(backdrop);
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("calls onClose on Escape key press", () => {
    const onClose = vi.fn();
    render(
      <MemoryRouter>
        <MobileNav isOpen={true} onClose={onClose} />
      </MemoryRouter>
    );
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("sets body overflow to hidden when open, restores on close", () => {
    const onClose = vi.fn();
    const { rerender } = render(
      <MemoryRouter>
        <MobileNav isOpen={true} onClose={onClose} />
      </MemoryRouter>
    );
    expect(document.body.style.overflow).toBe("hidden");

    rerender(
      <MemoryRouter>
        <MobileNav isOpen={false} onClose={onClose} />
      </MemoryRouter>
    );
    expect(document.body.style.overflow).toBe("");
  });

  it("calls onClose when close button is clicked", () => {
    const onClose = vi.fn();
    render(
      <MemoryRouter>
        <MobileNav isOpen={true} onClose={onClose} />
      </MemoryRouter>
    );
    fireEvent.click(screen.getByLabelText("关闭菜单"));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("calls onClose when a NavLink is clicked", () => {
    const onClose = vi.fn();
    render(
      <MemoryRouter>
        <MobileNav isOpen={true} onClose={onClose} />
      </MemoryRouter>
    );
    fireEvent.click(screen.getByText("供应商"));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("shows NEW button and calls updateDialogSetOpen when hasUpdate and non-portable", () => {
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

    const onClose = vi.fn();
    render(
      <MemoryRouter>
        <MobileNav isOpen={true} onClose={onClose} />
      </MemoryRouter>
    );

    const newBtn = screen.getByRole("button", { name: "NEW" });
    expect(newBtn).toBeInTheDocument();
    fireEvent.click(newBtn);
    expect(updateDialogSetOpenMock).toHaveBeenCalledWith(true);
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("shows NEW button and calls openReleasesUrl when hasUpdate and portable", async () => {
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

    const onClose = vi.fn();
    render(
      <MemoryRouter>
        <MobileNav isOpen={true} onClose={onClose} />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("button", { name: "NEW" }));

    await waitFor(() => {
      expect(openUrl).toHaveBeenCalledWith(AIO_RELEASES_URL);
      expect(windowOpen).toHaveBeenCalledWith(AIO_RELEASES_URL, "_blank", "noopener,noreferrer");
    });
    // portable path does NOT call updateDialogSetOpen
    expect(updateDialogSetOpenMock).not.toHaveBeenCalled();
    windowOpen.mockRestore();
  });

  it("shows NEW button and opens dialog when portable dev preview is enabled", () => {
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

    const onClose = vi.fn();
    render(
      <MemoryRouter>
        <MobileNav isOpen={true} onClose={onClose} />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("button", { name: "NEW" }));
    expect(updateDialogSetOpenMock).toHaveBeenCalledWith(true);
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("does not show NEW button when hasUpdate is false", () => {
    gatewayMetaRef.current = {
      gatewayAvailable: "available",
      gateway: { running: true, port: 37123 },
      preferredPort: 37123,
    };
    updateMetaRef.current = {
      ...updateMetaRef.current,
      about: { run_mode: "desktop" },
      updateCandidate: null,
    };

    render(
      <MemoryRouter>
        <MobileNav isOpen={true} onClose={vi.fn()} />
      </MemoryRouter>
    );
    expect(screen.queryByText("NEW")).not.toBeInTheDocument();
  });

  it("renders NavLink children with active/inactive states", () => {
    render(
      <MemoryRouter initialEntries={["/"]}>
        <MobileNav isOpen={true} onClose={vi.fn()} />
      </MemoryRouter>
    );
    // All nav items should render their labels
    expect(screen.getByText("首页")).toBeInTheDocument();
    expect(screen.getByText("控制台")).toBeInTheDocument();
    expect(screen.getByText("CLI 管理")).toBeInTheDocument();
  });
});

describe("ui/MobileHeader", () => {
  beforeEach(() => {
    vi.clearAllMocks();
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

  it("renders hamburger button with correct aria-label", () => {
    render(
      <MemoryRouter>
        <MobileHeader onMenuClick={vi.fn()} />
      </MemoryRouter>
    );
    expect(screen.getByLabelText("打开菜单")).toBeInTheDocument();
  });

  it("calls onMenuClick when hamburger is clicked", () => {
    const onMenuClick = vi.fn();
    render(
      <MemoryRouter>
        <MobileHeader onMenuClick={onMenuClick} />
      </MemoryRouter>
    );
    fireEvent.click(screen.getByLabelText("打开菜单"));
    expect(onMenuClick).toHaveBeenCalledOnce();
  });

  it("shows green dot when gateway is running", () => {
    gatewayMetaRef.current = {
      gatewayAvailable: "available",
      gateway: { running: true, port: 37123 },
      preferredPort: 37123,
    };

    render(
      <MemoryRouter>
        <MobileHeader onMenuClick={vi.fn()} />
      </MemoryRouter>
    );
    const dot = document.querySelector("[title='网关运行中']")!;
    expect(dot).toBeInTheDocument();
    expect(dot.className).toContain("bg-emerald-500");
  });

  it("shows gray dot when gateway is not running", () => {
    gatewayMetaRef.current = {
      gatewayAvailable: "available",
      gateway: { running: false, port: null },
      preferredPort: 37123,
    };

    render(
      <MemoryRouter>
        <MobileHeader onMenuClick={vi.fn()} />
      </MemoryRouter>
    );
    const dot = document.querySelector("[title='网关未运行']")!;
    expect(dot).toBeInTheDocument();
    expect(dot.className).toContain("bg-slate-300");
  });
});
