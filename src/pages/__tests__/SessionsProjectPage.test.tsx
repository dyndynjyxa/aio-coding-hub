import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../services/clipboard", () => ({ copyText: vi.fn().mockResolvedValue(undefined) }));
vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: ({ count }: { count: number }) => {
    const items = Array.from({ length: count }, (_, i) => ({
      index: i,
      key: String(i),
      start: i * 100,
      size: 100,
      end: (i + 1) * 100,
    }));
    return {
      getVirtualItems: () => items,
      getTotalSize: () => count * 100,
    };
  },
}));
vi.mock("../../services/cli/cliSessions", async () => {
  const actual = await vi.importActual<typeof import("../../services/cli/cliSessions")>(
    "../../services/cli/cliSessions"
  );
  return {
    ...actual,
    cliSessionsProjectsList: vi.fn().mockResolvedValue([]),
    cliSessionsSessionsList: vi.fn().mockResolvedValue([]),
    cliSessionsSessionDelete: vi.fn().mockResolvedValue([]),
  };
});
import {
  cliSessionsSessionDelete,
  cliSessionsSessionsList,
  cliSessionsProjectsList,
} from "../../services/cli/cliSessions";
import { SessionsProjectPage } from "../SessionsProjectPage";
function renderWithRoute(route: string) {
  const client = createTestQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[route]}>
        <Routes>
          <Route path="/sessions/:source/:projectId" element={<SessionsProjectPage />} />
          <Route path="/sessions/:source" element={<SessionsProjectPage />} />
          <Route path="*" element={<SessionsProjectPage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>
  );
}
describe("pages/SessionsProjectPage", () => {
  beforeEach(() => {
    setTauriRuntime();
    vi.mocked(cliSessionsProjectsList).mockResolvedValue([]);
    vi.mocked(cliSessionsSessionsList).mockResolvedValue([]);
    vi.mocked(cliSessionsSessionDelete).mockResolvedValue([]);
  });
  it("renders error state for invalid source", () => {
    setTauriRuntime();
    renderWithRoute("/sessions/invalid/proj1");
    expect(screen.getByText("无效来源")).toBeInTheDocument();
  });
  it("renders sessions list with data", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsProjectsList).mockResolvedValue([
      {
        source: "claude",
        id: "proj1",
        display_path: "/path",
        short_name: "Proj",
        session_count: 2,
        last_modified: 1740000000,
        model_provider: null,
        wsl_distro: null,
      },
    ]);
    vi.mocked(cliSessionsSessionsList).mockResolvedValue([
      {
        source: "claude",
        session_id: "s-1",
        file_path: "/f.json",
        first_prompt: "Hello world",
        message_count: 10,
        created_at: 1740000000,
        modified_at: 1740000000,
        git_branch: "main",
        project_path: "/path",
        is_sidechain: false,
        cwd: "/path",
        model_provider: "anthropic",
        cli_version: "1.0",
        wsl_distro: null,
      },
    ]);
    renderWithRoute("/sessions/claude/proj1");
    expect(await screen.findByText("Hello world")).toBeInTheDocument();
    expect(screen.getAllByText("main").length).toBeGreaterThan(0);
    expect(screen.getAllByText("anthropic").length).toBeGreaterThan(0);
  });
  it("filters sessions by search text", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsSessionsList).mockResolvedValue([
      {
        source: "claude",
        session_id: "s-1",
        file_path: "/f1.json",
        first_prompt: "Alpha task",
        message_count: 5,
        created_at: 1740000000,
        modified_at: 1740000000,
        git_branch: null,
        project_path: null,
        is_sidechain: null,
        cwd: null,
        model_provider: null,
        cli_version: null,
        wsl_distro: null,
      },
      {
        source: "claude",
        session_id: "s-2",
        file_path: "/f2.json",
        first_prompt: "Beta task",
        message_count: 3,
        created_at: 1740000000,
        modified_at: 1740000000,
        git_branch: null,
        project_path: null,
        is_sidechain: null,
        cwd: null,
        model_provider: null,
        cli_version: null,
        wsl_distro: null,
      },
    ]);
    renderWithRoute("/sessions/claude/proj1");
    expect(await screen.findByText("Alpha task")).toBeInTheDocument();
    const searchInput = screen.getByLabelText("搜索会话");
    fireEvent.change(searchInput, { target: { value: "Beta" } });
    expect(screen.queryByText("Alpha task")).not.toBeInTheDocument();
    expect(screen.getByText("Beta task")).toBeInTheDocument();
  });

  it("changes sort key", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsSessionsList).mockResolvedValue([
      {
        source: "claude",
        session_id: "s-1",
        file_path: "/f1.json",
        first_prompt: "First",
        message_count: 10,
        created_at: 1740000000,
        modified_at: 1740000100,
        git_branch: null,
        project_path: null,
        is_sidechain: null,
        cwd: null,
        model_provider: null,
        cli_version: null,
        wsl_distro: null,
      },
      {
        source: "claude",
        session_id: "s-2",
        file_path: "/f2.json",
        first_prompt: "Second",
        message_count: 20,
        created_at: 1740000100,
        modified_at: 1740000000,
        git_branch: null,
        project_path: null,
        is_sidechain: null,
        cwd: null,
        model_provider: null,
        cli_version: null,
        wsl_distro: null,
      },
    ]);
    renderWithRoute("/sessions/claude/proj1");
    expect(await screen.findByText("First")).toBeInTheDocument();
    const sortSelect = screen.getByLabelText("排序");
    fireEvent.change(sortSelect, { target: { value: "messages" } });
    expect(screen.getByText("Second")).toBeInTheDocument();
    fireEvent.change(sortSelect, { target: { value: "created" } });
    expect(screen.getByText("Second")).toBeInTheDocument();
  });

  it("toggles select all checkbox and opens delete dialog", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsSessionsList).mockResolvedValue([
      {
        source: "claude",
        session_id: "s-1",
        file_path: "/f1.json",
        first_prompt: "Task A",
        message_count: 1,
        created_at: 1740000000,
        modified_at: 1740000000,
        git_branch: null,
        project_path: null,
        is_sidechain: null,
        cwd: null,
        model_provider: null,
        cli_version: null,
        wsl_distro: null,
      },
    ]);
    renderWithRoute("/sessions/claude/proj1");
    expect(await screen.findByText("Task A")).toBeInTheDocument();
    const selectAll = screen.getByLabelText("全选");
    fireEvent.click(selectAll);
    // Delete button should appear
    expect(screen.getByText(/删除/)).toBeInTheDocument();
    // Deselect all
    fireEvent.click(selectAll);
  });

  it("handles single delete button click", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsSessionsList).mockResolvedValue([
      {
        source: "claude",
        session_id: "s-1",
        file_path: "/f1.json",
        first_prompt: "Task A",
        message_count: 1,
        created_at: 1740000000,
        modified_at: 1740000000,
        git_branch: null,
        project_path: null,
        is_sidechain: null,
        cwd: null,
        model_provider: null,
        cli_version: null,
        wsl_distro: null,
      },
    ]);
    renderWithRoute("/sessions/claude/proj1");
    expect(await screen.findByText("Task A")).toBeInTheDocument();
    const deleteBtn = screen.getByTitle("删除会话");
    fireEvent.click(deleteBtn);
    expect(screen.getByText("确认删除会话")).toBeInTheDocument();
  });

  it("copies resume command on button click", async () => {
    setTauriRuntime();
    const { copyText } = await import("../../services/clipboard");
    vi.mocked(cliSessionsSessionsList).mockResolvedValue([
      {
        source: "claude",
        session_id: "s-abc",
        file_path: "/f1.json",
        first_prompt: "Task",
        message_count: 1,
        created_at: 1740000000,
        modified_at: 1740000000,
        git_branch: null,
        project_path: null,
        is_sidechain: null,
        cwd: null,
        model_provider: null,
        cli_version: null,
        wsl_distro: null,
      },
    ]);
    renderWithRoute("/sessions/claude/proj1");
    expect(await screen.findByText("Task")).toBeInTheDocument();
    const copyBtn = screen.getByTitle("复制恢复命令");
    fireEvent.click(copyBtn);
    expect(copyText).toHaveBeenCalled();
  });

  it("renders session with replacement chars stripped from title", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsSessionsList).mockResolvedValue([
      {
        source: "claude",
        session_id: "s-1",
        file_path: "/f1.json",
        first_prompt: "Hello\uFFFDWorld",
        message_count: 1,
        created_at: 1740000000,
        modified_at: 1740000000,
        git_branch: null,
        project_path: null,
        is_sidechain: null,
        cwd: null,
        model_provider: null,
        cli_version: null,
        wsl_distro: null,
      },
    ]);
    renderWithRoute("/sessions/claude/proj1");
    expect(await screen.findByText("HelloWorld")).toBeInTheDocument();
  });

  it("renders empty state when no sessions", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsSessionsList).mockResolvedValue([]);
    renderWithRoute("/sessions/claude/proj1");
    expect(await screen.findByText("此项目没有会话记录")).toBeInTheDocument();
  });

  it("renders with WSL distro param", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsSessionsList).mockResolvedValue([]);
    renderWithRoute("/sessions/claude/proj1?distro=Ubuntu");
    expect(await screen.findByText(/WSL: Ubuntu/)).toBeInTheDocument();
  });

  it("deletes only currently visible selected sessions after filtering", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsSessionsList).mockResolvedValue([
      {
        source: "claude",
        session_id: "s-1",
        file_path: "/f1.json",
        first_prompt: "Alpha task",
        message_count: 1,
        created_at: 1740000000,
        modified_at: 1740000000,
        git_branch: null,
        project_path: null,
        is_sidechain: null,
        cwd: null,
        model_provider: null,
        cli_version: null,
        wsl_distro: null,
      },
      {
        source: "claude",
        session_id: "s-2",
        file_path: "/f2.json",
        first_prompt: "Beta task",
        message_count: 1,
        created_at: 1740000000,
        modified_at: 1740000000,
        git_branch: null,
        project_path: null,
        is_sidechain: null,
        cwd: null,
        model_provider: null,
        cli_version: null,
        wsl_distro: null,
      },
    ]);

    renderWithRoute("/sessions/claude/proj1");

    expect(await screen.findByText("Alpha task")).toBeInTheDocument();
    fireEvent.click(screen.getByLabelText("选择会话 Alpha task"));
    fireEvent.click(screen.getByLabelText("选择会话 Beta task"));
    expect(screen.getByRole("button", { name: "删除 (2)" })).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("搜索会话"), { target: { value: "Beta" } });

    await screen.findByText("Beta task");
    expect(screen.queryByText("Alpha task")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "删除 (1)" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "删除 (1)" }));
    fireEvent.click(screen.getByRole("button", { name: "确认删除 (1)" }));

    await waitFor(() => {
      expect(cliSessionsSessionDelete).toHaveBeenCalledWith({
        source: "claude",
        file_paths: ["/f2.json"],
        wsl_distro: undefined,
      });
    });
  });
});
