import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import type { ReactElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { SortModesView } from "../SortModesView";
import {
  sortModeActiveList,
  sortModeCreate,
  sortModeDelete,
  sortModeProvidersList,
  sortModeProviderSetEnabled,
  sortModeProvidersSetOrder,
  sortModeRename,
  sortModesList,
} from "../../../services/providers/sortModes";
import { queryClient } from "../../../query/queryClient";

let latestOnDragEnd: ((event: any) => void) | null = null;
let sortableIsDragging = false;

vi.mock("@dnd-kit/core", () => ({
  DndContext: ({ children, onDragEnd }: any) => {
    latestOnDragEnd = onDragEnd ?? null;
    return <div data-testid="dnd">{children}</div>;
  },
  PointerSensor: function PointerSensor() {},
  closestCenter: () => null,
  useSensor: () => null,
  useSensors: () => [],
}));

vi.mock("@dnd-kit/sortable", () => ({
  SortableContext: ({ children }: any) => <div data-testid="sortable">{children}</div>,
  arrayMove: (array: any[], from: number, to: number) => {
    const next = array.slice();
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item);
    return next;
  },
  useSortable: () => ({
    attributes: {},
    listeners: {},
    setNodeRef: () => {},
    transform: null,
    transition: undefined,
    isDragging: sortableIsDragging,
  }),
  verticalListSortingStrategy: {},
}));

vi.mock("@dnd-kit/utilities", () => ({
  CSS: { Transform: { toString: () => "" } },
}));

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));

vi.mock("../../../services/providers/sortModes", async () => {
  const actual = await vi.importActual<typeof import("../../../services/providers/sortModes")>(
    "../../../services/providers/sortModes"
  );
  return {
    ...actual,
    sortModesList: vi.fn(),
    sortModeActiveList: vi.fn(),
    sortModeProvidersList: vi.fn(),
    sortModeProvidersSetOrder: vi.fn(),
    sortModeProviderSetEnabled: vi.fn(),
    sortModeCreate: vi.fn(),
    sortModeRename: vi.fn(),
    sortModeDelete: vi.fn(),
  };
});

function renderWithQueryClient(ui: ReactElement) {
  queryClient.clear();
  const rendered = render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
  return {
    ...rendered,
    rerender: (nextUi: ReactElement) =>
      rendered.rerender(<QueryClientProvider client={queryClient}>{nextUi}</QueryClientProvider>),
  };
}

describe("pages/providers/SortModesView", () => {
  it("keeps the internal cli switcher available in sort modes view", () => {
    const setActiveCli = vi.fn();

    vi.mocked(sortModesList).mockResolvedValue([] as any);
    vi.mocked(sortModeActiveList).mockResolvedValue([] as any);
    vi.mocked(sortModeProvidersList).mockResolvedValue([] as any);

    renderWithQueryClient(
      <SortModesView
        activeCli="claude"
        setActiveCli={setActiveCli}
        providers={[] as any}
        providersLoading={false}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: "Codex" }));
    expect(setActiveCli).toHaveBeenCalledWith("codex");
    expect(screen.getByText("选择要配置的 CLI")).toBeInTheDocument();
  });

  it("covers providers list cancelled + ids null branches and active auto-selection edge cases", async () => {
    vi.mocked(toast).mockClear();
    sortableIsDragging = false;

    vi.mocked(sortModesList).mockResolvedValue([{ id: 1, name: "Work" }] as any);
    vi.mocked(sortModeActiveList).mockResolvedValue([
      { cli_key: "claude", mode_id: null },
      { cli_key: "codex", mode_id: 999 },
    ] as any);

    // 1) ids null -> modeProvidersAvailable=false
    vi.mocked(sortModeProvidersList).mockResolvedValueOnce(null as any);

    renderWithQueryClient(
      <SortModesView
        activeCli="claude"
        setActiveCli={vi.fn()}
        providers={
          [
            {
              id: 101,
              name: "P1",
              enabled: true,
              base_urls: ["https://a"],
              base_url_mode: "order",
            },
          ] as any
        }
        providersLoading={false}
      />
    );

    await waitFor(() => expect(screen.getByRole("button", { name: "Work" })).toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: "Work" }));
    await waitFor(() => expect(vi.mocked(sortModeProvidersList)).toHaveBeenCalledTimes(1));

    // 2) cancellation: reject after switching away; catch should skip due to cancelled
    let rejectProviders: (err: Error) => void = () => {
      throw new Error("rejectProviders not set");
    };
    const pendingProviders = new Promise<number[]>((_resolve, reject) => {
      rejectProviders = reject;
    });
    vi.mocked(sortModeProvidersList).mockReturnValueOnce(pendingProviders as any);

    fireEvent.click(screen.getByRole("button", { name: "Default" }));
    fireEvent.click(screen.getByRole("button", { name: "Work" }));
    fireEvent.click(screen.getByRole("button", { name: "Default" }));
    rejectProviders(new Error("boom"));
    await Promise.resolve();

    expect(vi.mocked(toast)).not.toHaveBeenCalledWith(
      expect.stringContaining("读取排序模板 Provider 列表失败")
    );
  });

  it("loads modes, joins providers, reorders, and supports CRUD", async () => {
    vi.mocked(sortModesList).mockResolvedValue([{ id: 1, name: "Work" }] as any);
    vi.mocked(sortModeActiveList).mockResolvedValue([{ cli_key: "claude", mode_id: 1 }] as any);
    vi.mocked(sortModeProvidersList).mockResolvedValue([
      { provider_id: 101, enabled: true },
    ] as any);
    vi.mocked(sortModeProvidersSetOrder).mockResolvedValue([
      { provider_id: 101, enabled: true },
      { provider_id: 102, enabled: true },
    ] as any);

    const providers = [
      {
        id: 101,
        name: "P1",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "order",
      },
      {
        id: 102,
        name: "P2",
        enabled: false,
        base_urls: ["https://b"],
        base_url_mode: "order",
      },
    ] as any[];

    renderWithQueryClient(
      <SortModesView
        activeCli="claude"
        setActiveCli={vi.fn()}
        providers={providers}
        providersLoading={false}
      />
    );

    await waitFor(() => expect(screen.getByRole("button", { name: "Work" })).toBeInTheDocument());

    // Join provider 102 into current mode -> persist order.
    await waitFor(() => expect(screen.getByRole("button", { name: "加入" })).toBeEnabled());
    fireEvent.click(screen.getByRole("button", { name: "加入" }));
    await waitFor(() =>
      expect(vi.mocked(sortModeProvidersSetOrder)).toHaveBeenCalledWith(
        expect.objectContaining({ mode_id: 1, cli_key: "claude", ordered_provider_ids: [101, 102] })
      )
    );

    // Simulate drag reorder via mocked DndContext.
    vi.mocked(sortModeProvidersSetOrder).mockResolvedValueOnce([
      { provider_id: 102, enabled: true },
      { provider_id: 101, enabled: true },
    ] as any);
    latestOnDragEnd?.({ active: { id: 101 }, over: { id: 102 } });
    await waitFor(() =>
      expect(vi.mocked(sortModeProvidersSetOrder)).toHaveBeenCalledWith(
        expect.objectContaining({ ordered_provider_ids: [102, 101] })
      )
    );

    // Create mode validation
    fireEvent.click(screen.getByRole("button", { name: "新建排序模板" }));
    const createDialog = within(screen.getByRole("dialog"));
    fireEvent.click(createDialog.getByRole("button", { name: "创建" }));
    expect(vi.mocked(toast)).toHaveBeenCalledWith("模式名称不能为空");

    vi.mocked(sortModeCreate).mockResolvedValue({ id: 2, name: "Life" } as any);
    fireEvent.change(createDialog.getByPlaceholderText("工作"), { target: { value: "Life" } });
    fireEvent.click(createDialog.getByRole("button", { name: "创建" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Life" })).toBeInTheDocument());

    // Rename mode
    fireEvent.click(screen.getByRole("button", { name: "Life" }));
    fireEvent.click(screen.getByRole("button", { name: "重命名" }));
    const renameDialog = within(screen.getByRole("dialog"));
    vi.mocked(sortModeRename).mockResolvedValue({ id: 2, name: "Life2" } as any);
    fireEvent.change(renameDialog.getByRole("textbox"), { target: { value: "Life2" } });
    fireEvent.click(renameDialog.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Life2" })).toBeInTheDocument());

    // Delete mode
    fireEvent.click(screen.getByRole("button", { name: "删除" }));
    const deleteDialog = within(screen.getByRole("dialog"));
    vi.mocked(sortModeDelete).mockResolvedValue(true as any);
    fireEvent.click(deleteDialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(vi.mocked(sortModeDelete)).toHaveBeenCalledWith({ mode_id: 2 }));
  });

  it("supports toggling provider enabled state inside a sort mode", async () => {
    vi.mocked(sortModesList).mockResolvedValue([{ id: 1, name: "Work" }] as any);
    vi.mocked(sortModeActiveList).mockResolvedValue([{ cli_key: "claude", mode_id: 1 }] as any);
    vi.mocked(sortModeProvidersList).mockResolvedValue([
      { provider_id: 101, enabled: true },
      { provider_id: 102, enabled: false },
    ] as any);
    vi.mocked(sortModeProviderSetEnabled).mockResolvedValue({
      provider_id: 102,
      enabled: true,
    } as any);

    renderWithQueryClient(
      <SortModesView
        activeCli="claude"
        setActiveCli={vi.fn()}
        providers={
          [
            {
              id: 101,
              name: "P1",
              enabled: true,
              base_urls: ["https://a"],
              base_url_mode: "order",
            },
            {
              id: 102,
              name: "P2",
              enabled: false,
              base_urls: ["https://b"],
              base_url_mode: "order",
            },
          ] as any
        }
        providersLoading={false}
      />
    );

    await waitFor(() => expect(screen.getByRole("button", { name: "Work" })).toBeInTheDocument());
    await waitFor(() => expect(screen.getAllByRole("switch")).toHaveLength(2));

    const switches = screen.getAllByRole("switch");
    expect(switches[1]).toHaveAttribute("aria-checked", "false");

    fireEvent.click(switches[1]!);
    await waitFor(() =>
      expect(vi.mocked(sortModeProviderSetEnabled)).toHaveBeenCalledWith({
        mode_id: 1,
        cli_key: "claude",
        provider_id: 102,
        enabled: true,
      })
    );
  });

  it("covers create/rename/delete null + error branches and delete dialog onOpenChange gating", async () => {
    vi.mocked(toast).mockClear();
    sortableIsDragging = false;

    vi.mocked(sortModesList).mockResolvedValue([{ id: 1, name: "Work" }] as any);
    vi.mocked(sortModeActiveList).mockResolvedValue([{ cli_key: "claude", mode_id: 1 }] as any);
    vi.mocked(sortModeProvidersList).mockResolvedValue([
      { provider_id: 101, enabled: true },
    ] as any);
    vi.mocked(sortModeProvidersSetOrder).mockResolvedValue([
      { provider_id: 101, enabled: true },
    ] as any);

    const providers = [
      { id: 101, name: "P1", enabled: true, base_urls: ["https://a"], base_url_mode: "order" },
    ] as any[];

    renderWithQueryClient(
      <SortModesView
        activeCli="claude"
        setActiveCli={vi.fn()}
        providers={providers}
        providersLoading={false}
      />
    );

    await waitFor(() => expect(screen.getByRole("button", { name: "Work" })).toBeInTheDocument());

    // create: null -> no-op
    vi.mocked(sortModeCreate).mockResolvedValueOnce(null as any);
    fireEvent.click(screen.getByRole("button", { name: "新建排序模板" }));
    const createDialog = within(screen.getByRole("dialog"));
    fireEvent.change(createDialog.getByPlaceholderText("工作"), { target: { value: "Life" } });
    fireEvent.click(createDialog.getByRole("button", { name: "创建" }));
    await waitFor(() => expect(vi.mocked(sortModeCreate)).toHaveBeenCalledTimes(1));

    // create: throws -> error toast
    vi.mocked(sortModeCreate).mockRejectedValueOnce(new Error("boom"));
    fireEvent.click(createDialog.getByRole("button", { name: "创建" }));
    await waitFor(() =>
      expect(vi.mocked(toast)).toHaveBeenCalledWith(
        expect.stringContaining("创建失败：Error: boom")
      )
    );
    // close create dialog before continuing
    fireEvent.click(document.querySelector(".bg-black\\/30") as HTMLElement);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

    // rename: empty -> toast
    fireEvent.click(screen.getByRole("button", { name: "Work" }));
    fireEvent.click(screen.getByRole("button", { name: "重命名" }));
    const renameDialog = within(screen.getByRole("dialog"));
    fireEvent.change(renameDialog.getByRole("textbox"), { target: { value: "  " } });
    fireEvent.click(renameDialog.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(vi.mocked(toast)).toHaveBeenCalledWith("模式名称不能为空"));

    // rename: null -> no-op
    vi.mocked(sortModeRename).mockResolvedValueOnce(null as any);
    fireEvent.change(renameDialog.getByRole("textbox"), { target: { value: "Work2" } });
    fireEvent.click(renameDialog.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(vi.mocked(sortModeRename)).toHaveBeenCalledTimes(1));

    // rename: throws -> error toast
    vi.mocked(sortModeRename).mockRejectedValueOnce(new Error("boom"));
    fireEvent.click(renameDialog.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(vi.mocked(toast)).toHaveBeenCalledWith(
        expect.stringContaining("重命名失败：Error: boom")
      )
    );
    // close rename dialog before continuing
    fireEvent.click(document.querySelector(".bg-black\\/30") as HTMLElement);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

    // delete: ok=false -> no-op
    fireEvent.click(screen.getByRole("button", { name: "删除" }));
    const deleteDialog = within(screen.getByRole("dialog"));
    vi.mocked(sortModeDelete).mockResolvedValueOnce(false as any);
    fireEvent.click(deleteDialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(vi.mocked(sortModeDelete)).toHaveBeenCalledTimes(1));

    // delete: throws -> error toast
    vi.mocked(sortModeDelete).mockRejectedValueOnce(new Error("boom"));
    fireEvent.click(deleteDialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() =>
      expect(vi.mocked(toast)).toHaveBeenCalledWith(
        expect.stringContaining("删除失败：Error: boom")
      )
    );

    // delete: deleting blocks onOpenChange close
    let resolveDelete: (v: boolean) => void = () => {
      throw new Error("resolveDelete not set");
    };
    const deletePromise = new Promise<boolean>((resolve) => {
      resolveDelete = resolve;
    });
    vi.mocked(sortModeDelete).mockReturnValueOnce(deletePromise as any);
    fireEvent.click(deleteDialog.getByRole("button", { name: "确认删除" }));
    // Attempt to close by overlay while deleting (should stay open)
    fireEvent.click(document.querySelector(".bg-black\\/30") as HTMLElement);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    resolveDelete(true);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("applies dragging class when sortable row is dragging", async () => {
    sortableIsDragging = true;

    vi.mocked(sortModesList).mockResolvedValue([{ id: 1, name: "Work" }] as any);
    vi.mocked(sortModeActiveList).mockResolvedValue([{ cli_key: "claude", mode_id: 1 }] as any);
    vi.mocked(sortModeProvidersList).mockResolvedValue([
      { provider_id: 101, enabled: true },
    ] as any);

    renderWithQueryClient(
      <SortModesView
        activeCli="claude"
        setActiveCli={vi.fn()}
        providers={
          [
            {
              id: 101,
              name: "P1",
              enabled: true,
              base_urls: ["https://a"],
              base_url_mode: "order",
            },
          ] as any
        }
        providersLoading={false}
      />
    );

    await waitFor(() => expect(screen.getByRole("button", { name: "移除" })).toBeInTheDocument());
    const removeButton = screen.getByRole("button", { name: "移除" });
    // Card uses rounded-xl on mobile, rounded-2xl on larger screens
    const draggingCard = removeButton.closest("div[class*='rounded-']");
    expect(draggingCard?.className).toContain("ring-2");

    sortableIsDragging = false;
  });

  it("covers remove provider and persist order null/error branches", async () => {
    vi.mocked(toast).mockClear();
    vi.mocked(sortModesList).mockResolvedValue([{ id: 1, name: "Work" }] as any);
    vi.mocked(sortModeActiveList).mockResolvedValue([{ cli_key: "claude", mode_id: 1 }] as any);
    vi.mocked(sortModeProvidersList).mockResolvedValue([
      { provider_id: 101, enabled: true },
      { provider_id: 102, enabled: true },
    ] as any);

    vi.mocked(sortModeProvidersSetOrder)
      .mockResolvedValueOnce(null as any)
      .mockRejectedValueOnce(new Error("boom"))
      .mockResolvedValueOnce([{ provider_id: 102, enabled: true }] as any);

    const providers = [
      { id: 101, name: "P1", enabled: true, base_urls: ["https://a"], base_url_mode: "order" },
      { id: 102, name: "", enabled: false, base_urls: ["https://b"], base_url_mode: "order" },
    ] as any[];

    renderWithQueryClient(
      <SortModesView
        activeCli="claude"
        setActiveCli={vi.fn()}
        providers={providers}
        providersLoading={false}
      />
    );

    await waitFor(() => expect(screen.getByRole("button", { name: "Work" })).toBeInTheDocument());

    // right list should be populated (provider rows rendered)
    await waitFor(() =>
      expect(screen.getAllByRole("button", { name: "移除" }).length).toBeGreaterThan(0)
    );

    // pointerdown handler stops propagation (coverage)
    fireEvent.pointerDown(screen.getAllByRole("button", { name: "移除" })[0]!);

    // 1) persist returns null -> revert
    fireEvent.click(screen.getAllByRole("button", { name: "移除" })[0]!);
    await waitFor(() => expect(vi.mocked(sortModeProvidersSetOrder)).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(screen.getAllByRole("button", { name: "移除" })).toHaveLength(2));

    // 2) persist throws -> toast and revert
    fireEvent.click(screen.getAllByRole("button", { name: "移除" })[0]!);
    await waitFor(() => expect(vi.mocked(sortModeProvidersSetOrder)).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(vi.mocked(toast)).toHaveBeenCalled());
    await waitFor(() => expect(screen.getAllByRole("button", { name: "移除" })).toHaveLength(2));

    // 3) persist succeeds -> P1 removed from mode
    fireEvent.click(screen.getAllByRole("button", { name: "移除" })[0]!);
    await waitFor(() => expect(vi.mocked(sortModeProvidersSetOrder)).toHaveBeenCalledTimes(3));
    await waitFor(() => expect(screen.getAllByRole("button", { name: "移除" })).toHaveLength(1));

    // drag end edge cases
    latestOnDragEnd?.({ active: { id: 101 }, over: null });
    latestOnDragEnd?.({ active: { id: 101 }, over: { id: 101 } });
    latestOnDragEnd?.({ active: { id: 999 }, over: { id: 102 } });
  });

  it("covers providers loading/empty branches and dialog onOpenChange close paths", async () => {
    vi.mocked(toast).mockClear();

    vi.mocked(sortModesList).mockResolvedValue([{ id: 1, name: "Work" }] as any);
    vi.mocked(sortModeActiveList).mockResolvedValue([{ cli_key: "claude", mode_id: 1 }] as any);
    vi.mocked(sortModeProvidersList).mockRejectedValueOnce(new Error("boom"));
    vi.mocked(sortModeProvidersSetOrder).mockResolvedValue([
      { provider_id: 101, enabled: true },
    ] as any);

    const { rerender } = renderWithQueryClient(
      <SortModesView
        activeCli="claude"
        setActiveCli={vi.fn()}
        providers={[]}
        providersLoading={true}
      />
    );

    await waitFor(() =>
      expect(vi.mocked(toast)).toHaveBeenCalledWith(
        expect.stringContaining("读取排序模板 Provider 列表失败")
      )
    );

    // left list loading branch
    expect(screen.getAllByText("加载中…").length).toBeGreaterThan(0);

    // left list empty branch
    rerender(
      <SortModesView
        activeCli="claude"
        setActiveCli={vi.fn()}
        providers={[]}
        providersLoading={false}
      />
    );
    expect(screen.getByText(/暂无 Provider/)).toBeInTheDocument();

    // create dialog onOpenChange (close by overlay)
    fireEvent.click(screen.getByRole("button", { name: "新建排序模板" }));
    fireEvent.click(document.querySelector(".bg-black\\/30") as HTMLElement);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

    // rename dialog onOpenChange
    fireEvent.click(screen.getByRole("button", { name: "Work" }));
    fireEvent.click(screen.getByRole("button", { name: "重命名" }));
    fireEvent.click(document.querySelector(".bg-black\\/30") as HTMLElement);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

    // delete dialog onOpenChange
    fireEvent.click(screen.getByRole("button", { name: "删除" }));
    fireEvent.click(document.querySelector(".bg-black\\/30") as HTMLElement);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });
});
