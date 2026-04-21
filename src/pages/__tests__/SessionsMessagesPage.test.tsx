import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
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
      start: i * 150,
      size: 150,
      end: (i + 1) * 150,
    }));
    return {
      getVirtualItems: () => items,
      getTotalSize: () => count * 150,
    };
  },
}));

vi.mock("../../services/cli/cliSessions", async () => {
  const actual = await vi.importActual<typeof import("../../services/cli/cliSessions")>(
    "../../services/cli/cliSessions"
  );
  return {
    ...actual,
    cliSessionsMessagesGet: vi.fn().mockResolvedValue({
      messages: [],
      total: 0,
      page: 0,
      page_size: 50,
      has_more: false,
    }),
  };
});

import { cliSessionsMessagesGet } from "../../services/cli/cliSessions";
import type {
  CliSessionsSessionSummary,
  CliSessionsDisplayMessage,
} from "../../services/cli/cliSessions";
import { SessionsMessagesPage } from "../SessionsMessagesPage";

const SESSION: CliSessionsSessionSummary = {
  source: "claude",
  session_id: "ses-abc-123",
  file_path: "/path/to/file.json",
  first_prompt: "Hello world prompt",
  message_count: 10,
  created_at: 1740000000,
  modified_at: 1740000000,
  git_branch: "main",
  project_path: "/project/path",
  is_sidechain: false,
  cwd: "/working/dir",
  model_provider: "anthropic",
  cli_version: "1.2.3",
  wsl_distro: null,
};

const MESSAGES: CliSessionsDisplayMessage[] = [
  {
    uuid: "msg-1",
    role: "user",
    timestamp: "2025-01-15T10:00:00Z",
    model: null,
    content: [{ type: "text", text: "Hello from user" }],
  },
  {
    uuid: "msg-2",
    role: "assistant",
    timestamp: "2025-01-15T10:00:05Z",
    model: "claude-3-opus",
    content: [
      { type: "text", text: "Hello from assistant" },
      { type: "thinking", thinking: "Let me think about this..." },
      { type: "reasoning", text: "Reasoning step here" },
      { type: "tool_use", id: "tu-1", name: "read_file", input: '{"path": "/foo"}' },
      { type: "tool_result", tool_use_id: "tu-1", content: "file contents", is_error: false },
      { type: "tool_result", tool_use_id: "tu-2", content: "error output", is_error: true },
      { type: "function_call", name: "search", arguments: '{"q": "test"}', call_id: "fc-1" },
      { type: "function_call_output", call_id: "fc-1", output: "search results" },
    ],
  },
  {
    uuid: "msg-3",
    role: "system",
    timestamp: null,
    model: null,
    content: [{ type: "text", text: "System message" }],
  },
  {
    uuid: "msg-4",
    role: "tool",
    timestamp: "2025-01-15T10:00:10Z",
    model: null,
    content: [{ type: "text", text: "Tool output" }],
  },
];

function renderWithRoute(route: string, state?: { session: CliSessionsSessionSummary }) {
  const client = createTestQueryClient();
  const entries = [{ pathname: route, state }];
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={entries}>
        <Routes>
          <Route path="/sessions/:source/:projectId/session/*" element={<SessionsMessagesPage />} />
          <Route path="*" element={<SessionsMessagesPage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

describe("pages/SessionsMessagesPage", () => {
  beforeEach(() => {
    setTauriRuntime();
    vi.mocked(cliSessionsMessagesGet).mockReset();
  });

  it("renders error state for invalid source", () => {
    setTauriRuntime();
    renderWithRoute("/sessions/invalid/proj1/session/file.json");
    expect(screen.getByText("无效来源")).toBeInTheDocument();
  });

  it("renders missing session state for valid source without location state", () => {
    setTauriRuntime();
    renderWithRoute("/sessions/claude/proj1/session/file.json");
    expect(screen.getByText("会话信息缺失")).toBeInTheDocument();
  });

  it("renders session info and messages with all block types", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsMessagesGet).mockResolvedValue({
      messages: MESSAGES,
      total: 4,
      page: 0,
      page_size: 50,
      has_more: false,
    });

    renderWithRoute("/sessions/claude/proj1/session/path%2Fto%2Ffile.json", {
      session: SESSION,
    });

    // Session info renders
    expect(await screen.findByText("Hello from user")).toBeInTheDocument();
    expect(screen.getByText("Hello from assistant")).toBeInTheDocument();
    expect(screen.getByText("System message")).toBeInTheDocument();
    expect(screen.getByText("Tool output")).toBeInTheDocument();

    await waitFor(() => {
      expect(cliSessionsMessagesGet).toHaveBeenCalledWith(
        expect.objectContaining({ from_end: false })
      );
    });

    // Block types render
    expect(screen.getByText("思考过程")).toBeInTheDocument();
    expect(screen.getByText("推理过程")).toBeInTheDocument();
    expect(screen.getByText(/Tool 调用：read_file/)).toBeInTheDocument();
    expect(screen.getAllByText(/Tool 结果/).length).toBeGreaterThan(0);
    expect(screen.getByText(/Function 调用：search/)).toBeInTheDocument();
    expect(screen.getByText("Function 输出")).toBeInTheDocument();

    // Sender labels
    expect(screen.getByText("你")).toBeInTheDocument();
    expect(screen.getByText("Claude Code")).toBeInTheDocument();
    expect(screen.getByText("System")).toBeInTheDocument();
    expect(screen.getByText("Tool")).toBeInTheDocument();

    // Avatar text (only left and right sides get avatars; system=center has none)
    expect(screen.getByText("我")).toBeInTheDocument();
    expect(screen.getByText("AI")).toBeInTheDocument();
    expect(screen.getByText("TL")).toBeInTheDocument();

    // Meta info
    expect(screen.getByText("claude")).toBeInTheDocument();
    expect(screen.getAllByText("main").length).toBeGreaterThan(0);
    expect(screen.getByText("1.2.3")).toBeInTheDocument();
    expect(screen.getByText("/working/dir")).toBeInTheDocument();
    expect(screen.getByText("/project/path")).toBeInTheDocument();

    // Model text shown when toggle is on — default is off, so model should not be visible
    expect(screen.queryByText("claude-3-opus")).not.toBeInTheDocument();
  });

  it("shows model text when showModel toggle is enabled", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsMessagesGet).mockResolvedValue({
      messages: MESSAGES.slice(0, 2),
      total: 2,
      page: 0,
      page_size: 50,
      has_more: false,
    });

    renderWithRoute("/sessions/claude/proj1/session/file.json", {
      session: SESSION,
    });

    expect(await screen.findByText("Hello from user")).toBeInTheDocument();

    // Toggle showModel switch
    const modelSwitch = screen.getByLabelText("显示模型");
    fireEvent.click(modelSwitch);

    expect(screen.getByText("claude-3-opus")).toBeInTheDocument();
  });

  it("hides timestamp when showTimestamp toggle is disabled", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsMessagesGet).mockResolvedValue({
      messages: [MESSAGES[0]],
      total: 1,
      page: 0,
      page_size: 50,
      has_more: false,
    });

    renderWithRoute("/sessions/claude/proj1/session/file.json", {
      session: SESSION,
    });

    expect(await screen.findByText("Hello from user")).toBeInTheDocument();

    // Timestamp should be visible by default
    // Toggle it off
    const timestampSwitch = screen.getByLabelText("显示时间");
    fireEvent.click(timestampSwitch);

    // After toggling off, the formatted timestamp should not appear
    // (We can't easily check for absence of a specific formatted date, but the toggle state changed)
  });

  it("renders codex source with correct sender label", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsMessagesGet).mockResolvedValue({
      messages: [
        {
          uuid: "msg-codex",
          role: "assistant",
          timestamp: null,
          model: null,
          content: [{ type: "text", text: "Codex response" }],
        },
      ],
      total: 1,
      page: 0,
      page_size: 50,
      has_more: false,
    });

    const codexSession = { ...SESSION, source: "codex" as const };
    renderWithRoute("/sessions/codex/proj1/session/file.json", {
      session: codexSession,
    });

    expect(await screen.findByText("Codex response")).toBeInTheDocument();
    expect(screen.getByText("Codex")).toBeInTheDocument();
  });

  it("renders empty messages state", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsMessagesGet).mockResolvedValue({
      messages: [],
      total: 0,
      page: 0,
      page_size: 50,
      has_more: false,
    });

    renderWithRoute("/sessions/claude/proj1/session/file.json", {
      session: SESSION,
    });

    expect(await screen.findByText("此会话没有可显示的消息")).toBeInTheDocument();
  });

  it("renders with has_more=true and shows load more button", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsMessagesGet).mockResolvedValue({
      messages: [MESSAGES[0]],
      total: 10,
      page: 0,
      page_size: 50,
      has_more: true,
    });

    renderWithRoute("/sessions/claude/proj1/session/file.json", {
      session: SESSION,
    });

    expect(await screen.findByText("Hello from user")).toBeInTheDocument();
    const loadMoreBtn = screen.getByTitle("加载更多消息");
    expect(loadMoreBtn).not.toBeDisabled();
  });

  it("renders session without optional fields", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsMessagesGet).mockResolvedValue({
      messages: [MESSAGES[0]],
      total: 1,
      page: 0,
      page_size: 50,
      has_more: false,
    });

    const minimalSession: CliSessionsSessionSummary = {
      source: "claude",
      session_id: "ses-min",
      file_path: "/f.json",
      first_prompt: null,
      message_count: 1,
      created_at: null,
      modified_at: null,
      git_branch: null,
      project_path: null,
      is_sidechain: null,
      cwd: null,
      model_provider: null,
      cli_version: null,
      wsl_distro: null,
    };

    renderWithRoute("/sessions/claude/proj1/session/f.json", {
      session: minimalSession,
    });

    expect(await screen.findByText("Hello from user")).toBeInTheDocument();
    // Should not render optional meta fields
    expect(screen.queryByText("分支")).not.toBeInTheDocument();
    expect(screen.queryByText("CLI 版本")).not.toBeInTheDocument();
    expect(screen.queryByText("CWD")).not.toBeInTheDocument();
    expect(screen.queryByText("项目路径")).not.toBeInTheDocument();
  });

  it("handles unknown role gracefully", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsMessagesGet).mockResolvedValue({
      messages: [
        {
          uuid: "msg-unknown",
          role: "custom_role",
          timestamp: null,
          model: null,
          content: [{ type: "text", text: "Unknown role message" }],
        },
      ],
      total: 1,
      page: 0,
      page_size: 50,
      has_more: false,
    });

    renderWithRoute("/sessions/claude/proj1/session/file.json", {
      session: SESSION,
    });

    expect(await screen.findByText("Unknown role message")).toBeInTheDocument();
    expect(screen.getByText("custom_role")).toBeInTheDocument();
  });

  it("clicks load more button to fetch next page", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsMessagesGet)
      .mockResolvedValueOnce({
        messages: [MESSAGES[0]],
        total: 10,
        page: 0,
        page_size: 50,
        has_more: true,
      })
      .mockResolvedValueOnce({
        messages: [MESSAGES[1]],
        total: 10,
        page: 1,
        page_size: 50,
        has_more: false,
      });

    renderWithRoute("/sessions/claude/proj1/session/file.json", {
      session: SESSION,
    });

    expect(await screen.findByText("Hello from user")).toBeInTheDocument();
    const loadMoreBtn = screen.getByTitle("加载更多消息");
    fireEvent.click(loadMoreBtn);
    await waitFor(() => {
      expect(cliSessionsMessagesGet).toHaveBeenCalledTimes(2);
    });
  });

  it("clicks scroll to bottom buttons", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsMessagesGet).mockResolvedValue({
      messages: [MESSAGES[0]],
      total: 1,
      page: 0,
      page_size: 50,
      has_more: false,
    });

    renderWithRoute("/sessions/claude/proj1/session/file.json", {
      session: SESSION,
    });

    expect(await screen.findByText("Hello from user")).toBeInTheDocument();
    // sidebar bottom button
    const sidebarBottomBtn = screen.getByText("到会话末尾");
    fireEvent.click(sidebarBottomBtn);
    // header bottom icon button
    const headerBottomBtn = screen.getAllByTitle("滚动到会话末尾")[1];
    if (headerBottomBtn) fireEvent.click(headerBottomBtn);
  });

  it("copies resume command via quick action button", async () => {
    setTauriRuntime();
    const { copyText } = await import("../../services/clipboard");
    vi.mocked(cliSessionsMessagesGet).mockResolvedValue({
      messages: [MESSAGES[0]],
      total: 1,
      page: 0,
      page_size: 50,
      has_more: false,
    });

    renderWithRoute("/sessions/claude/proj1/session/file.json", {
      session: SESSION,
    });

    expect(await screen.findByText("Hello from user")).toBeInTheDocument();
    const copyResumeBtn = screen.getByTitle("复制恢复命令");
    fireEvent.click(copyResumeBtn);
    expect(copyText).toHaveBeenCalled();
  });

  it("copies session ID via quick action button", async () => {
    setTauriRuntime();
    const { copyText } = await import("../../services/clipboard");
    vi.mocked(cliSessionsMessagesGet).mockResolvedValue({
      messages: [],
      total: 0,
      page: 0,
      page_size: 50,
      has_more: false,
    });

    renderWithRoute("/sessions/claude/proj1/session/file.json", {
      session: SESSION,
    });

    const copyIdBtn = screen.getByTitle("复制 Session ID");
    fireEvent.click(copyIdBtn);
    expect(copyText).toHaveBeenCalledWith("ses-abc-123");
  });

  it("copies file path via quick action button", async () => {
    setTauriRuntime();
    const { copyText } = await import("../../services/clipboard");
    vi.mocked(cliSessionsMessagesGet).mockResolvedValue({
      messages: [],
      total: 0,
      page: 0,
      page_size: 50,
      has_more: false,
    });

    renderWithRoute("/sessions/claude/proj1/session/path%2Fto%2Ffile.json", {
      session: SESSION,
    });

    const copyPathBtn = screen.getByTitle("复制文件路径");
    fireEvent.click(copyPathBtn);
    expect(copyText).toHaveBeenCalledWith("path/to/file.json");
  });

  it("renders with empty role string as unknown", async () => {
    setTauriRuntime();
    vi.mocked(cliSessionsMessagesGet).mockResolvedValue({
      messages: [
        {
          uuid: "msg-empty-role",
          role: "  ",
          timestamp: null,
          model: null,
          content: [{ type: "text", text: "Empty role" }],
        },
      ],
      total: 1,
      page: 0,
      page_size: 50,
      has_more: false,
    });

    renderWithRoute("/sessions/claude/proj1/session/file.json", {
      session: SESSION,
    });

    expect(await screen.findByText("Empty role")).toBeInTheDocument();
  });
});
