import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import type { ReactElement } from "react";
import { toast } from "sonner";
import { WorkspacesPage } from "../WorkspacesPage";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import {
  useWorkspaceApplyMutation,
  useWorkspaceCreateMutation,
  useWorkspaceDeleteMutation,
  useWorkspacePreviewQuery,
  useWorkspaceRenameMutation,
  useWorkspacesListQuery,
} from "../../query/workspaces";
import { useMcpServersListQuery } from "../../query/mcp";
import { usePromptsListQuery } from "../../query/prompts";
import { useSettingsQuery } from "../../query/settings";
import { useSkillsInstalledListQuery } from "../../query/skills";
import { logToConsole } from "../../services/consoleLog";
import { createTestAppSettings } from "../../test/fixtures/settings";

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../services/consoleLog", () => ({ logToConsole: vi.fn() }));

vi.mock("../mcp/McpServersView", () => ({ McpServersView: () => <div>mcp-view</div> }));
vi.mock("../prompts/PromptsView", () => ({ PromptsView: () => <div>prompts-view</div> }));
vi.mock("../skills/SkillsView", () => ({ SkillsView: () => <div>skills-view</div> }));

vi.mock("../../query/workspaces", async () => {
  const actual =
    await vi.importActual<typeof import("../../query/workspaces")>("../../query/workspaces");
  return {
    ...actual,
    useWorkspacesListQuery: vi.fn(),
    useWorkspaceCreateMutation: vi.fn(),
    useWorkspaceRenameMutation: vi.fn(),
    useWorkspaceDeleteMutation: vi.fn(),
    useWorkspaceApplyMutation: vi.fn(),
    useWorkspacePreviewQuery: vi.fn(),
  };
});

vi.mock("../../query/prompts", async () => {
  const actual = await vi.importActual<typeof import("../../query/prompts")>("../../query/prompts");
  return { ...actual, usePromptsListQuery: vi.fn() };
});

vi.mock("../../query/settings", async () => {
  const actual =
    await vi.importActual<typeof import("../../query/settings")>("../../query/settings");
  return { ...actual, useSettingsQuery: vi.fn() };
});

vi.mock("../../query/mcp", async () => {
  const actual = await vi.importActual<typeof import("../../query/mcp")>("../../query/mcp");
  return { ...actual, useMcpServersListQuery: vi.fn() };
});

vi.mock("../../query/skills", async () => {
  const actual = await vi.importActual<typeof import("../../query/skills")>("../../query/skills");
  return { ...actual, useSkillsInstalledListQuery: vi.fn() };
});

function renderWithProviders(element: ReactElement) {
  const client = createTestQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>{element}</MemoryRouter>
    </QueryClientProvider>
  );
}

describe("pages/WorkspacesPage", () => {
  it("renders empty state and toasts when workspaces query fails", async () => {
    vi.mocked(toast).mockClear();
    vi.mocked(logToConsole).mockClear();
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createTestAppSettings({ cli_priority_order: ["codex", "claude", "gemini"] }),
    } as any);

    vi.mocked(useWorkspacesListQuery).mockReturnValue({
      data: { items: [], active_id: null },
      isFetching: false,
      error: new Error("boom"),
    } as any);
    vi.mocked(useWorkspaceCreateMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useWorkspaceRenameMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useWorkspaceDeleteMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useWorkspaceApplyMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useWorkspacePreviewQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(usePromptsListQuery).mockReturnValue({ data: [], isFetching: false } as any);
    vi.mocked(useMcpServersListQuery).mockReturnValue({ data: [], isFetching: false } as any);
    vi.mocked(useSkillsInstalledListQuery).mockReturnValue({ data: [], isFetching: false } as any);

    renderWithProviders(<WorkspacesPage />);

    expect(screen.getByText("暂无工作区")).toBeInTheDocument();
    expect(screen.getByText("请选择一个工作区")).toBeInTheDocument();

    await waitFor(() => expect(toast).toHaveBeenCalledWith("加载失败：请查看控制台日志"));
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "加载工作区失败",
      expect.objectContaining({ cli: "codex" })
    );
  });

  it("supports filtering, create/rename/delete, preview and apply flows", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createTestAppSettings(),
    } as any);
    const closeDialogByOverlay = async () => {
      fireEvent.click(document.querySelector(".bg-black\\/30") as HTMLElement);
      await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    };

    const items = [
      { id: 1, cli_key: "claude", name: "W1", created_at: 1, updated_at: 1 },
      { id: 2, cli_key: "claude", name: "W2", created_at: 2, updated_at: 2 },
    ] as any[];

    vi.mocked(useWorkspacesListQuery).mockReturnValue({
      data: { items, active_id: 1 },
      isFetching: false,
      error: null,
    } as any);

    const createMutation = { isPending: false, mutateAsync: vi.fn() };
    createMutation.mutateAsync.mockResolvedValue({ id: 3, cli_key: "claude", name: "W3" });
    vi.mocked(useWorkspaceCreateMutation).mockReturnValue(createMutation as any);

    const renameMutation = { isPending: false, mutateAsync: vi.fn() };
    renameMutation.mutateAsync.mockResolvedValue({ ok: true });
    vi.mocked(useWorkspaceRenameMutation).mockReturnValue(renameMutation as any);

    const deleteMutation = { isPending: false, mutateAsync: vi.fn() };
    deleteMutation.mutateAsync.mockResolvedValue(true);
    vi.mocked(useWorkspaceDeleteMutation).mockReturnValue(deleteMutation as any);

    const applyMutation = { isPending: false, mutateAsync: vi.fn() };
    applyMutation.mutateAsync.mockResolvedValue({
      to_workspace_id: 2,
      from_workspace_id: 1,
      applied_at: 123,
    });
    vi.mocked(useWorkspaceApplyMutation).mockReturnValue(applyMutation as any);

    vi.mocked(useWorkspacePreviewQuery).mockReturnValue({
      data: {
        prompts: {
          will_change: true,
          from_enabled: { name: "Old", excerpt: "old" },
          to_enabled: { name: "New", excerpt: "new" },
        },
        mcp: { added: ["fetch"], removed: [] },
        skills: { added: [], removed: ["tool"] },
      },
      isFetching: false,
      refetch: vi.fn().mockResolvedValue({ data: {} }),
    } as any);

    vi.mocked(usePromptsListQuery).mockReturnValue({
      data: [{ enabled: true }, { enabled: false }],
    } as any);
    vi.mocked(useMcpServersListQuery).mockReturnValue({ data: [{ enabled: true }] } as any);
    vi.mocked(useSkillsInstalledListQuery).mockReturnValue({ data: [{ enabled: true }] } as any);

    renderWithProviders(<WorkspacesPage />);

    // Filter list
    fireEvent.change(screen.getByLabelText("搜索工作区"), { target: { value: "W2" } });
    const w2Card = screen
      .getAllByRole("button")
      .find((el) => el.getAttribute("tabindex") === "0" && el.textContent?.includes("W2")) as
      | HTMLElement
      | undefined;
    if (!w2Card) throw new Error("W2 workspace card not found");
    expect(within(w2Card).getByRole("button", { name: "切换…" })).toBeInTheDocument();

    // Create workspace (duplicate blocked, then create blank)
    fireEvent.click(screen.getByRole("button", { name: "新建" }));
    await closeDialogByOverlay();
    fireEvent.click(screen.getByRole("button", { name: "新建" }));
    let dialog = within(screen.getByRole("dialog"));
    const nameInput = dialog.getByRole("textbox");
    fireEvent.change(nameInput, { target: { value: "W2" } });
    expect(dialog.getByText("名称重复：同一 CLI 下必须唯一")).toBeInTheDocument();
    fireEvent.change(nameInput, { target: { value: "W3" } });
    fireEvent.click(dialog.getByLabelText("空白创建（推荐）"));
    fireEvent.click(dialog.getByRole("button", { name: "创建" }));
    await waitFor(() =>
      expect(createMutation.mutateAsync).toHaveBeenCalledWith({
        cliKey: "claude",
        name: "W3",
        cloneFromActive: false,
      })
    );

    // Rename workspace 2
    fireEvent.click(screen.getByLabelText("重命名"));
    await closeDialogByOverlay();
    fireEvent.click(screen.getByLabelText("重命名"));
    dialog = within(screen.getByRole("dialog"));
    fireEvent.change(dialog.getByRole("textbox"), { target: { value: "W2-renamed" } });
    fireEvent.click(dialog.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(renameMutation.mutateAsync).toHaveBeenCalledWith({
        cliKey: "claude",
        workspaceId: 2,
        name: "W2-renamed",
      })
    );

    // Switch dialog (compare -> confirm switch)
    fireEvent.click(within(w2Card).getByRole("button", { name: "切换…" }));
    dialog = within(screen.getByRole("dialog"));
    expect(dialog.getByText("+1 / -0")).toBeInTheDocument();
    await closeDialogByOverlay();
    fireEvent.click(within(w2Card).getByRole("button", { name: "切换…" }));
    dialog = within(screen.getByRole("dialog"));
    fireEvent.change(dialog.getByRole("textbox"), { target: { value: "APPLY" } });
    fireEvent.click(dialog.getByRole("button", { name: "确认切换" }));
    await waitFor(() =>
      expect(applyMutation.mutateAsync).toHaveBeenCalledWith({ cliKey: "claude", workspaceId: 2 })
    );

    // Rollback to previous workspace after applying.
    fireEvent.click(screen.getByRole("button", { name: "回滚到上一个" }));
    await waitFor(() =>
      expect(applyMutation.mutateAsync).toHaveBeenCalledWith({ cliKey: "claude", workspaceId: 1 })
    );

    // Delete workspace 2 (non-active)
    fireEvent.click(screen.getByLabelText("删除"));
    dialog = within(screen.getByRole("dialog"));
    await closeDialogByOverlay();
    fireEvent.click(screen.getByLabelText("删除"));
    dialog = within(screen.getByRole("dialog"));
    fireEvent.click(dialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() =>
      expect(deleteMutation.mutateAsync).toHaveBeenCalledWith({ cliKey: "claude", workspaceId: 2 })
    );

    // Switch right tabs renders subviews (mocked)
    fireEvent.click(screen.getByRole("tab", { name: "Prompts" }));
    expect(screen.getByText("prompts-view")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("tab", { name: "MCP" }));
    expect(screen.getByText("mcp-view")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("tab", { name: "Skills" }));
    expect(screen.getByText("skills-view")).toBeInTheDocument();
  });

  it("covers switch dialog and preview branches across CLIs", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createTestAppSettings(),
    } as any);
    const items = [
      { id: 1, cli_key: "claude", name: "W1", created_at: 1, updated_at: 1 },
      { id: 2, cli_key: "codex", name: "CodexW", created_at: 2, updated_at: 2 },
      { id: 3, cli_key: "gemini", name: "GeminiW", created_at: 3, updated_at: 3 },
    ] as any[];

    vi.mocked(useWorkspacesListQuery).mockReturnValue({
      data: { items, active_id: 2 },
      isFetching: false,
      error: null,
    } as any);

    vi.mocked(useWorkspaceCreateMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useWorkspaceRenameMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useWorkspaceDeleteMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const applyMutation = { isPending: false, mutateAsync: vi.fn() };
    applyMutation.mutateAsync.mockResolvedValue({
      to_workspace_id: 3,
      from_workspace_id: null,
      applied_at: 123,
    });
    vi.mocked(useWorkspaceApplyMutation).mockReturnValue(applyMutation as any);

    vi.mocked(useWorkspacePreviewQuery).mockImplementation(
      (workspaceId: number | null, opts: any) => {
        if (!opts?.enabled) return { data: null, isFetching: false, refetch: vi.fn() } as any;
        if (workspaceId === 2) {
          return {
            data: {
              from_workspace_id: 0,
              prompts: { will_change: false, from_enabled: null, to_enabled: null },
              mcp: { added: [], removed: [] },
              skills: { added: [], removed: [] },
            },
            isFetching: false,
            refetch: vi.fn(),
          } as any;
        }
        return {
          data: {
            from_workspace_id: 999,
            prompts: { will_change: false, from_enabled: null, to_enabled: null },
            mcp: { added: [], removed: [] },
            skills: { added: [], removed: [] },
          },
          isFetching: false,
          refetch: vi.fn(),
        } as any;
      }
    );

    vi.mocked(usePromptsListQuery).mockReturnValue({ data: [], isFetching: false } as any);
    vi.mocked(useMcpServersListQuery).mockReturnValue({ data: [], isFetching: false } as any);
    vi.mocked(useSkillsInstalledListQuery).mockReturnValue({ data: [], isFetching: false } as any);

    renderWithProviders(<WorkspacesPage />);

    const codexCard = screen
      .getAllByRole("button")
      .find(
        (el) => el.getAttribute("tabindex") === "0" && el.getAttribute("aria-current") === "true"
      ) as HTMLElement | undefined;
    if (!codexCard) throw new Error("Active workspace card not found");
    expect(within(codexCard).queryByRole("button", { name: "切换…" })).not.toBeInTheDocument();

    // active workspace => mcp tab has no non-active hint
    fireEvent.click(screen.getByRole("tab", { name: "MCP" }));
    expect(
      screen.queryByText("非当前工作区：启用/停用仅写入数据库，不会同步到 CLI。")
    ).not.toBeInTheDocument();

    const geminiCard = screen
      .getAllByRole("button")
      .find((el) => el.getAttribute("tabindex") === "0" && el.textContent?.includes("GeminiW")) as
      | HTMLElement
      | undefined;
    if (!geminiCard) throw new Error("Gemini workspace card not found");
    fireEvent.click(within(geminiCard).getByRole("button", { name: "切换…" }));
    const dialog = within(screen.getByRole("dialog"));

    // no paths shown (requirement: do not display ~/.xxx hints)
    expect(screen.queryByText("Prompts：~/.codex/AGENTS.md")).not.toBeInTheDocument();
    expect(screen.queryByText("MCP：~/.codex/config.toml")).not.toBeInTheDocument();
    expect(screen.queryByText("Skills：~/.codex/skills")).not.toBeInTheDocument();
    expect(screen.queryByText("Prompts：~/.gemini/GEMINI.md")).not.toBeInTheDocument();
    expect(screen.queryByText("MCP：~/.gemini/settings.json")).not.toBeInTheDocument();
    expect(screen.queryByText("Skills：~/.gemini/skills")).not.toBeInTheDocument();

    // preview branches (no changes + will_change=false)
    expect(dialog.getByText(/当前：.*#999/)).toBeInTheDocument();
    expect(dialog.getByText("不变")).toBeInTheDocument();
    expect(dialog.getAllByText("无变化")).toHaveLength(2);

    fireEvent.change(dialog.getByRole("textbox"), {
      target: { value: "APPLY" },
      currentTarget: { value: "APPLY" },
    });
    fireEvent.click(dialog.getByRole("button", { name: "确认切换" }));
    await waitFor(() =>
      expect(applyMutation.mutateAsync).toHaveBeenCalledWith({ cliKey: "claude", workspaceId: 3 })
    );

    // apply success is surfaced to user; rollback is hidden when from_workspace_id is null
    await waitFor(() => expect(toast).toHaveBeenCalledWith("已切换为当前工作区"));
    expect(screen.queryByRole("button", { name: "回滚到上一个" })).not.toBeInTheDocument();
  });

  it("covers workspace card selection keyboard handlers, overview quick links, preview refresh, and dialog cancel controls", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createTestAppSettings(),
    } as any);
    const createMutation = { isPending: false, mutateAsync: vi.fn() };
    const renameMutation = { isPending: false, mutateAsync: vi.fn() };
    const deleteMutation = { isPending: false, mutateAsync: vi.fn() };
    const applyMutation = { isPending: true, mutateAsync: vi.fn() };

    const previewRefetch = vi.fn();

    const itemsA = [
      { id: 1, cli_key: "claude", name: "W1", created_at: 1, updated_at: 1 },
      { id: 2, cli_key: "claude", name: "W2", created_at: 2, updated_at: 2 },
    ] as any[];

    const workspacesState: any = {
      data: { items: itemsA, active_id: 1 },
      isFetching: false,
      error: null,
    };
    vi.mocked(useWorkspacesListQuery).mockImplementation(() => workspacesState);

    vi.mocked(useWorkspaceCreateMutation).mockReturnValue(createMutation as any);
    vi.mocked(useWorkspaceRenameMutation).mockReturnValue(renameMutation as any);
    vi.mocked(useWorkspaceDeleteMutation).mockReturnValue(deleteMutation as any);
    vi.mocked(useWorkspaceApplyMutation).mockReturnValue(applyMutation as any);

    vi.mocked(useWorkspacePreviewQuery).mockReturnValue({
      data: {
        from_workspace_id: 1,
        prompts: { will_change: false, from_enabled: null, to_enabled: null },
        mcp: { added: [], removed: [] },
        skills: { added: [], removed: [] },
      },
      isFetching: false,
      refetch: previewRefetch,
    } as any);

    let promptsState: any = { data: null, isFetching: true };
    let mcpState: any = { data: [], isFetching: false };
    let skillsState: any = { data: [], isFetching: false };
    vi.mocked(usePromptsListQuery).mockImplementation(() => promptsState);
    vi.mocked(useMcpServersListQuery).mockImplementation(() => mcpState);
    vi.mocked(useSkillsInstalledListQuery).mockImplementation(() => skillsState);

    const client = createTestQueryClient();
    const { rerender } = render(
      <QueryClientProvider client={client}>
        <MemoryRouter>
          <WorkspacesPage />
        </MemoryRouter>
      </QueryClientProvider>
    );

    await waitFor(() => expect(screen.getAllByText("W2").length).toBeGreaterThan(0));

    // cover workspace card click + keydown handlers
    const w2Card = screen
      .getAllByRole("button")
      .find((el) => el.getAttribute("tabindex") === "0" && el.textContent?.includes("W2")) as
      | HTMLElement
      | undefined;
    if (!w2Card) throw new Error("W2 workspace card not found");
    fireEvent.click(w2Card);
    expect(screen.getByText("非当前")).toBeInTheDocument();

    const w1Card = screen
      .getAllByRole("button")
      .find((el) => el.getAttribute("tabindex") === "0" && el.textContent?.includes("W1")) as
      | HTMLElement
      | undefined;
    if (!w1Card) throw new Error("W1 workspace card not found");
    fireEvent.keyDown(w1Card, { key: "Enter" });
    await waitFor(() => expect(screen.queryByText("非当前")).not.toBeInTheDocument());

    fireEvent.keyDown(w2Card, { key: " " });
    await waitFor(() => expect(screen.getByText("非当前")).toBeInTheDocument());

    // overview loading -> stats missing -> stats present
    expect(screen.getAllByText("加载中…").length).toBeGreaterThan(0);
    promptsState = { data: null, isFetching: false };
    rerender(
      <QueryClientProvider client={client}>
        <MemoryRouter>
          <WorkspacesPage />
        </MemoryRouter>
      </QueryClientProvider>
    );
    expect(screen.getAllByText("—").length).toBeGreaterThan(0);

    promptsState = { data: [{ enabled: true }, { enabled: false }], isFetching: false };
    mcpState = { data: [{ enabled: true }], isFetching: false };
    skillsState = { data: [{ enabled: true }], isFetching: false };
    rerender(
      <QueryClientProvider client={client}>
        <MemoryRouter>
          <WorkspacesPage />
        </MemoryRouter>
      </QueryClientProvider>
    );
    expect(screen.getAllByText(/已启用/).length).toBeGreaterThan(0);

    // overview quick links (buttons inside overview cards)
    fireEvent.click(screen.getAllByRole("button", { name: "去配置" })[0]!);
    expect(screen.getByText("prompts-view")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("tab", { name: "总览" }));

    fireEvent.click(screen.getAllByRole("button", { name: "去配置" })[1]!);
    expect(screen.getByText("mcp-view")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("tab", { name: "总览" }));

    fireEvent.click(screen.getAllByRole("button", { name: "去配置" })[2]!);
    expect(screen.getByText("skills-view")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("tab", { name: "总览" }));

    // switch dialog refresh button
    fireEvent.click(within(w2Card).getByRole("button", { name: "切换…" }));
    let dialog = within(screen.getByRole("dialog"));
    fireEvent.click(dialog.getByRole("button", { name: "刷新对比" }));
    expect(previewRefetch).toHaveBeenCalled();

    // switch dialog cancel button (and pending label)
    expect(dialog.getByText("切换中…")).toBeInTheDocument();
    fireEvent.click(dialog.getByRole("button", { name: "取消" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

    // create dialog: toggle create mode and cancel
    fireEvent.click(screen.getByRole("button", { name: "新建" }));
    dialog = within(screen.getByRole("dialog"));
    fireEvent.click(dialog.getByLabelText("空白创建（推荐）"));
    fireEvent.click(dialog.getByLabelText("从当前工作区克隆"));
    fireEvent.click(dialog.getByRole("button", { name: "取消" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

    // rename dialog: show error then cancel
    fireEvent.click(screen.getAllByLabelText("重命名")[0]!);
    dialog = within(screen.getByRole("dialog"));
    fireEvent.change(dialog.getByRole("textbox"), { target: { value: "   " } });
    expect(dialog.getByText("名称不能为空")).toBeInTheDocument();
    fireEvent.click(dialog.getByRole("button", { name: "取消" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

    // delete dialog cancel button
    const enabledDeleteButton = screen
      .getAllByLabelText("删除")
      .find((el) => !(el as HTMLButtonElement).disabled);
    if (!enabledDeleteButton) throw new Error("Enabled delete button not found");
    fireEvent.click(enabledDeleteButton);
    dialog = within(screen.getByRole("dialog"));
    fireEvent.click(dialog.getByRole("button", { name: "取消" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });
});
