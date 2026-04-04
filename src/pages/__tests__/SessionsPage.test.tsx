import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
vi.mock("../../services/clipboard", () => ({ copyText: vi.fn().mockResolvedValue(undefined) }));
vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: ({ count }: { count: number }) => {
    const items = Array.from({ length: count }, (_, i) => ({
      index: i,
      key: String(i),
      start: i * 80,
      size: 80,
      end: (i + 1) * 80,
    }));
    return {
      getVirtualItems: () => items,
      getTotalSize: () => count * 80,
    };
  },
}));
vi.mock("../../services/cliSessions", async () => {
  const actual = await vi.importActual<typeof import("../../services/cliSessions")>(
    "../../services/cliSessions"
  );
  return { ...actual, cliSessionsProjectsList: vi.fn().mockResolvedValue([]) };
});
vi.mock("../../query/wsl", async () => {
  const actual = await vi.importActual<typeof import("../../query/wsl")>("../../query/wsl");
  return { ...actual, useWslDetectionQuery: vi.fn() };
});
vi.mock("../../query/settings", async () => {
  const actual =
    await vi.importActual<typeof import("../../query/settings")>("../../query/settings");
  return { ...actual, useSettingsQuery: vi.fn() };
});
import { cliSessionsProjectsList } from "../../services/cliSessions";
import { useSettingsQuery } from "../../query/settings";
import { useWslDetectionQuery } from "../../query/wsl";
import { createTestAppSettings } from "../../test/fixtures/settings";
import { SessionsPage } from "../SessionsPage";

function LocationDisplay() {
  const location = useLocation();
  return <div data-testid="location-display">{`${location.pathname}${location.search}`}</div>;
}

function renderWithProviders(ui: React.ReactElement, { route = "/" } = {}) {
  const client = createTestQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[route]}>
        <Routes>
          <Route
            path="/"
            element={
              <>
                {ui}
                <LocationDisplay />
              </>
            }
          />
          <Route path="/sessions/:source/:projectId" element={<LocationDisplay />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>
  );
}
describe("pages/SessionsPage", () => {
  beforeEach(() => {
    setTauriRuntime();
    vi.mocked(cliSessionsProjectsList).mockResolvedValue([]);
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createTestAppSettings(),
    } as any);
    vi.mocked(useWslDetectionQuery).mockReturnValue({
      data: null,
      isFetched: false,
    } as any);
  });
  it("renders loading state with Tauri runtime", () => {
    setTauriRuntime();
    renderWithProviders(<SessionsPage />, { route: "/?source=claude" });
    expect(screen.getByText("Session 会话")).toBeInTheDocument();
  });
  it("renders projects when data is available", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsProjectsList).mockResolvedValue([
      {
        source: "claude",
        id: "proj-1",
        display_path: "/home/user/proj",
        short_name: "My Project",
        session_count: 5,
        last_modified: 1740000000,
        model_provider: "anthropic",
        wsl_distro: null,
      },
    ]);
    renderWithProviders(<SessionsPage />, { route: "/?source=claude" });
    expect(await screen.findByText("My Project")).toBeInTheDocument();
    expect(screen.getByText("anthropic")).toBeInTheDocument();
  });
  it("filters projects by search text", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsProjectsList).mockResolvedValue([
      {
        source: "claude",
        id: "proj-1",
        display_path: "/home/user/proj",
        short_name: "Alpha",
        session_count: 3,
        last_modified: 1740000000,
        model_provider: null,
        wsl_distro: null,
      },
      {
        source: "claude",
        id: "proj-2",
        display_path: "/home/user/beta",
        short_name: "Beta",
        session_count: 1,
        last_modified: 1740000000,
        model_provider: null,
        wsl_distro: null,
      },
    ]);
    renderWithProviders(<SessionsPage />, { route: "/?source=claude" });
    expect(await screen.findByText("Alpha")).toBeInTheDocument();
    const searchInput = screen.getByLabelText("搜索项目");
    fireEvent.change(searchInput, { target: { value: "Beta" } });
    expect(screen.queryByText("Alpha")).not.toBeInTheDocument();
    expect(screen.getByText("Beta")).toBeInTheDocument();
  });

  it("switches source tab", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsProjectsList).mockResolvedValue([
      {
        source: "claude",
        id: "proj-1",
        display_path: "/home/user/proj",
        short_name: "CProject",
        session_count: 1,
        last_modified: 1740000000,
        model_provider: null,
        wsl_distro: null,
      },
    ]);
    renderWithProviders(<SessionsPage />, { route: "/?source=claude" });
    expect(await screen.findByText("CProject")).toBeInTheDocument();
    const codexTab = screen.getByText("Codex");
    fireEvent.click(codexTab);
    // After switching, projects query re-fetches
    expect(cliSessionsProjectsList).toHaveBeenCalledWith("codex", undefined);
  });

  it("uses the global CLI priority when source is missing from the URL", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createTestAppSettings({ cli_priority_order: ["codex", "claude", "gemini"] }),
    } as any);
    vi.mocked(cliSessionsProjectsList).mockResolvedValue([
      {
        source: "codex",
        id: "proj-1",
        display_path: "/home/user/proj",
        short_name: "Codex Project",
        session_count: 1,
        last_modified: 1740000000,
        model_provider: null,
        wsl_distro: null,
      },
    ]);

    renderWithProviders(<SessionsPage />, { route: "/" });
    expect(await screen.findByText("Codex Project")).toBeInTheDocument();
    expect(cliSessionsProjectsList).toHaveBeenCalledWith("codex", undefined);
  });

  it("changes sort key", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsProjectsList).mockResolvedValue([
      {
        source: "claude",
        id: "proj-1",
        display_path: "/a",
        short_name: "Zulu",
        session_count: 1,
        last_modified: 1740000000,
        model_provider: null,
        wsl_distro: null,
      },
      {
        source: "claude",
        id: "proj-2",
        display_path: "/b",
        short_name: "Alpha",
        session_count: 10,
        last_modified: 1740000100,
        model_provider: null,
        wsl_distro: null,
      },
    ]);
    renderWithProviders(<SessionsPage />, { route: "/?source=claude" });
    expect(await screen.findByText("Zulu")).toBeInTheDocument();
    const sortSelect = screen.getByLabelText("排序");
    fireEvent.change(sortSelect, { target: { value: "sessions" } });
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    fireEvent.change(sortSelect, { target: { value: "name" } });
    expect(screen.getByText("Alpha")).toBeInTheDocument();
  });

  it("renders empty state when no projects", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsProjectsList).mockResolvedValue([]);
    renderWithProviders(<SessionsPage />, { route: "/?source=claude" });
    expect(await screen.findByText("未找到任何项目")).toBeInTheDocument();
  });

  it("copies source dir hint on button click", async () => {
    setTauriRuntime();
    const { copyText } = await import("../../services/clipboard");
    vi.mocked(cliSessionsProjectsList).mockResolvedValue([
      {
        source: "claude",
        id: "proj-1",
        display_path: "/a",
        short_name: "P1",
        session_count: 1,
        last_modified: 1740000000,
        model_provider: null,
        wsl_distro: null,
      },
    ]);
    renderWithProviders(<SessionsPage />, { route: "/?source=claude" });
    expect(await screen.findByText("P1")).toBeInTheDocument();
    const copyBtn = screen.getByTitle("复制数据源路径提示");
    fireEvent.click(copyBtn);
    expect(copyText).toHaveBeenCalled();
  });

  it("handles refresh button", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsProjectsList).mockResolvedValue([
      {
        source: "claude",
        id: "proj-1",
        display_path: "/a",
        short_name: "P1",
        session_count: 1,
        last_modified: 1740000000,
        model_provider: null,
        wsl_distro: null,
      },
    ]);
    renderWithProviders(<SessionsPage />, { route: "/?source=claude" });
    expect(await screen.findByText("P1")).toBeInTheDocument();
    const refreshBtn = screen.getByText("刷新");
    fireEvent.click(refreshBtn);
    // Refetch should be triggered
    expect(cliSessionsProjectsList).toHaveBeenCalledTimes(2);
  });

  it("navigates to project on click", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsProjectsList).mockResolvedValue([
      {
        source: "claude",
        id: "proj-1",
        display_path: "/a",
        short_name: "ClickMe",
        session_count: 1,
        last_modified: 1740000000,
        model_provider: null,
        wsl_distro: null,
      },
    ]);
    renderWithProviders(<SessionsPage />, { route: "/?source=claude" });
    const projectBtn = await screen.findByText("ClickMe");
    fireEvent.click(projectBtn);
  });

  it("preserves distro from URL while WSL detection is still loading", async () => {
    const originalUserAgent = window.navigator.userAgent;
    Object.defineProperty(window.navigator, "userAgent", {
      value: "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
      configurable: true,
    });

    vi.mocked(useWslDetectionQuery).mockReturnValue({
      data: null,
      isFetched: false,
    } as any);
    vi.mocked(cliSessionsProjectsList).mockResolvedValue([
      {
        source: "claude",
        id: "proj-1",
        display_path: "/wsl/project",
        short_name: "WSL Project",
        session_count: 1,
        last_modified: 1740000000,
        model_provider: null,
        wsl_distro: "Ubuntu",
      },
    ]);

    renderWithProviders(<SessionsPage />, { route: "/?source=claude&distro=Ubuntu" });

    expect(await screen.findByText("WSL Project")).toBeInTheDocument();
    expect(cliSessionsProjectsList).toHaveBeenCalledWith("claude", "Ubuntu");

    fireEvent.click(screen.getByText("WSL Project"));
    expect(screen.getByTestId("location-display")).toHaveTextContent(
      "/sessions/claude/proj-1?distro=Ubuntu"
    );

    Object.defineProperty(window.navigator, "userAgent", {
      value: originalUserAgent,
      configurable: true,
    });
  });
});
