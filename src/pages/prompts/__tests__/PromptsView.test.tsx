import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { PromptsView } from "../PromptsView";
import {
  usePromptDeleteMutation,
  usePromptSetEnabledMutation,
  usePromptUpsertMutation,
  usePromptsListQuery,
} from "../../../query/prompts";

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));

// MDXEditor renders contentEditable which lacks a native value setter.
// Replace with a plain textarea so fireEvent.change works in tests.
vi.mock("@mdxeditor/editor", () => {
  function MDXEditor(props: { markdown: string; onChange?: (v: string) => void }) {
    return (
      <textarea
        role="textbox"
        value={props.markdown}
        onChange={(e) => props.onChange?.(e.target.value)}
      />
    );
  }
  return {
    MDXEditor,
    headingsPlugin: () => ({}),
    listsPlugin: () => ({}),
    quotePlugin: () => ({}),
    thematicBreakPlugin: () => ({}),
    markdownShortcutPlugin: () => ({}),
    toolbarPlugin: () => ({}),
    BoldItalicUnderlineToggles: () => null,
    BlockTypeSelect: () => null,
    ListsToggle: () => null,
  };
});

vi.mock("../../../query/prompts", async () => {
  const actual =
    await vi.importActual<typeof import("../../../query/prompts")>("../../../query/prompts");
  return {
    ...actual,
    usePromptsListQuery: vi.fn(),
    usePromptUpsertMutation: vi.fn(),
    usePromptSetEnabledMutation: vi.fn(),
    usePromptDeleteMutation: vi.fn(),
  };
});

describe("pages/prompts/PromptsView", () => {
  it("creates, toggles, edits and deletes prompts", async () => {
    const prompt = { id: 1, name: "P1", content: "hello ".repeat(80), enabled: false } as any;

    vi.mocked(usePromptsListQuery).mockReturnValue({
      data: [prompt],
      isFetching: false,
      error: null,
    } as any);

    const upsertMutation = { isPending: false, mutateAsync: vi.fn() };
    upsertMutation.mutateAsync.mockResolvedValue({ id: 2, name: "P2", enabled: false });
    vi.mocked(usePromptUpsertMutation).mockReturnValue(upsertMutation as any);

    const toggleMutation = { isPending: false, mutateAsync: vi.fn() };
    toggleMutation.mutateAsync.mockResolvedValue({ ...prompt, enabled: true });
    vi.mocked(usePromptSetEnabledMutation).mockReturnValue(toggleMutation as any);

    const deleteMutation = { isPending: false, mutateAsync: vi.fn() };
    deleteMutation.mutateAsync.mockResolvedValue(true);
    vi.mocked(usePromptDeleteMutation).mockReturnValue(deleteMutation as any);

    render(<PromptsView workspaceId={1} cliKey="claude" isActiveWorkspace={false} />);

    fireEvent.click(screen.getByRole("switch"));
    await waitFor(() =>
      expect(toggleMutation.mutateAsync).toHaveBeenCalledWith({ promptId: 1, enabled: true })
    );

    fireEvent.click(screen.getByRole("button", { name: "新增提示词" }));
    const createDialog = within(screen.getByRole("dialog"));
    const [nameInput, contentTextarea] = createDialog.getAllByRole("textbox");
    fireEvent.change(nameInput, { target: { value: "P2" } });
    fireEvent.change(contentTextarea, { target: { value: "" } });
    fireEvent.click(createDialog.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(upsertMutation.mutateAsync).toHaveBeenCalledWith({
        promptId: null,
        name: "P2",
        content: "",
        enabled: false,
      })
    );

    fireEvent.click(screen.getByTitle("编辑"));
    const editDialog = within(screen.getByRole("dialog"));
    fireEvent.click(editDialog.getByRole("button", { name: "关闭" }));

    fireEvent.click(screen.getByTitle("删除"));
    const deleteDialog = within(screen.getByRole("dialog"));
    fireEvent.click(deleteDialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(deleteMutation.mutateAsync).toHaveBeenCalledWith(1));
  });

  it("formats save errors into user-friendly toasts", async () => {
    vi.mocked(usePromptsListQuery).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
    } as any);

    const upsertMutation = { isPending: false, mutateAsync: vi.fn() };
    upsertMutation.mutateAsync.mockRejectedValue(
      new Error("SEC_INVALID_INPUT: prompt name is required")
    );
    vi.mocked(usePromptUpsertMutation).mockReturnValue(upsertMutation as any);
    vi.mocked(usePromptSetEnabledMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(usePromptDeleteMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    render(<PromptsView workspaceId={1} cliKey="codex" isActiveWorkspace />);

    fireEvent.click(screen.getByRole("button", { name: "新增提示词" }));
    const dialog = within(screen.getByRole("dialog"));
    const [nameInput, contentTextarea] = dialog.getAllByRole("textbox");
    fireEvent.change(nameInput, { target: { value: " " } });
    fireEvent.change(contentTextarea, { target: { value: "x" } });
    fireEvent.change(nameInput, { target: { value: "ok" } });
    fireEvent.click(dialog.getByRole("button", { name: "保存" }));

    await waitFor(() =>
      expect(vi.mocked(toast)).toHaveBeenCalledWith(
        expect.stringContaining("保存失败：名称不能为空")
      )
    );
  });

  it("covers tauri-only / db constraint / content required save toasts", async () => {
    vi.mocked(usePromptsListQuery).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
    } as any);

    const upsertMutation = { isPending: false, mutateAsync: vi.fn() };
    upsertMutation.mutateAsync
      .mockResolvedValueOnce(null)
      .mockRejectedValueOnce(new Error("DB_CONSTRAINT: prompt name=dup"))
      .mockRejectedValueOnce(new Error("SEC_INVALID_INPUT: prompt content is required"))
      .mockRejectedValueOnce(new Error("DB_CONSTRAINT: other"));
    vi.mocked(usePromptUpsertMutation).mockReturnValue(upsertMutation as any);
    vi.mocked(usePromptSetEnabledMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(usePromptDeleteMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    render(<PromptsView workspaceId={1} cliKey="gemini" isActiveWorkspace />);

    fireEvent.click(screen.getByRole("button", { name: "新增提示词" }));
    const dialog = within(screen.getByRole("dialog"));
    const [nameInput, contentTextarea] = dialog.getAllByRole("textbox");
    fireEvent.change(nameInput, { target: { value: "dup" } });
    fireEvent.change(contentTextarea, { target: { value: "x" } });

    fireEvent.click(dialog.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(upsertMutation.mutateAsync).toHaveBeenCalledTimes(1));

    vi.mocked(toast).mockClear();
    fireEvent.click(dialog.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(vi.mocked(toast)).toHaveBeenCalledWith(
        expect.stringContaining("保存失败：名称重复（同一工作区下名称必须唯一）")
      )
    );

    vi.mocked(toast).mockClear();
    fireEvent.click(dialog.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(vi.mocked(toast)).toHaveBeenCalledWith(
        expect.stringContaining("保存失败：内容不能为空")
      )
    );

    vi.mocked(toast).mockClear();
    fireEvent.click(dialog.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(vi.mocked(toast)).toHaveBeenCalledWith(
        expect.stringContaining("保存失败：数据库约束冲突（请检查名称是否重复）")
      )
    );
  });

  it("covers toggle/delete tauri-only + error branches and load error toast", async () => {
    const prompts = [
      { id: 1, name: "P1", content: "x", enabled: false },
      { id: 2, name: "P2", content: "y", enabled: true },
    ] as any[];

    vi.mocked(usePromptsListQuery).mockReturnValue({
      data: prompts,
      isFetching: false,
      error: new Error("load fail"),
    } as any);

    const toggleMutation = { isPending: false, mutateAsync: vi.fn() };
    toggleMutation.mutateAsync
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce({ ...prompts[0], enabled: true })
      .mockResolvedValueOnce({ ...prompts[1], enabled: false })
      .mockRejectedValueOnce(new Error("boom"));
    vi.mocked(usePromptSetEnabledMutation).mockReturnValue(toggleMutation as any);

    vi.mocked(usePromptUpsertMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const deleteMutation = { isPending: false, mutateAsync: vi.fn() };
    deleteMutation.mutateAsync
      .mockResolvedValueOnce(false)
      .mockRejectedValueOnce(new Error("boom"));
    vi.mocked(usePromptDeleteMutation).mockReturnValue(deleteMutation as any);

    render(<PromptsView workspaceId={1} cliKey="claude" isActiveWorkspace />);

    await waitFor(() =>
      expect(vi.mocked(toast)).toHaveBeenCalledWith("加载失败：请查看控制台日志")
    );
    vi.mocked(toast).mockClear();

    const switches = screen.getAllByRole("switch");
    fireEvent.click(switches[0]!); // enable P1 -> null branch
    await waitFor(() => expect(toggleMutation.mutateAsync).toHaveBeenCalledTimes(1));

    vi.mocked(toast).mockClear();
    fireEvent.click(switches[0]!); // enable P1 -> enabled true
    await waitFor(() => expect(toggleMutation.mutateAsync).toHaveBeenCalledTimes(2));
    expect(vi.mocked(toast)).toHaveBeenCalledWith(expect.stringContaining("已启用并同步到"));

    vi.mocked(toast).mockClear();
    fireEvent.click(switches[1]!); // disable P2
    await waitFor(() => expect(toggleMutation.mutateAsync).toHaveBeenCalledTimes(3));
    expect(vi.mocked(toast)).toHaveBeenCalledWith("已停用并同步");

    vi.mocked(toast).mockClear();
    fireEvent.click(switches[1]!); // error branch
    await waitFor(() => expect(toggleMutation.mutateAsync).toHaveBeenCalledTimes(4));
    expect(vi.mocked(toast)).toHaveBeenCalledWith(expect.stringContaining("操作失败："));

    // delete: false + error branches
    fireEvent.click(screen.getAllByTitle("删除")[0]!);
    const deleteDialog = within(screen.getByRole("dialog"));
    fireEvent.click(deleteDialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(deleteMutation.mutateAsync).toHaveBeenCalledTimes(1));

    fireEvent.click(deleteDialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(deleteMutation.mutateAsync).toHaveBeenCalledTimes(2));
    expect(vi.mocked(toast)).toHaveBeenCalledWith(expect.stringContaining("删除失败："));
  });
});
