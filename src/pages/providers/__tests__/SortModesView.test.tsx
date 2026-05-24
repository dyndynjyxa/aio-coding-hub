import {
  act,
  fireEvent,
  render,
  renderHook,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import type { ReactElement, ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { SortModesView } from "../SortModesView";
import { useSortModesDataModel } from "../useSortModesDataModel";
import { logToConsole } from "../../../services/consoleLog";
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

function queryWrapper() {
  queryClient.clear();
  return function QueryWrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
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

  it("covers providers list cancellation and active auto-selection edge cases", async () => {
    vi.mocked(toast).mockClear();
    sortableIsDragging = false;

    vi.mocked(sortModesList).mockResolvedValue([{ id: 1, name: "Work" }] as any);
    vi.mocked(sortModeActiveList).mockResolvedValue([
      { cli_key: "claude", mode_id: null },
      { cli_key: "codex", mode_id: 999 },
    ] as any);

    vi.mocked(sortModeProvidersList).mockResolvedValueOnce([]);

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

  it("serializes manual sort-mode refreshes and suppresses unmounted failures", async () => {
    vi.mocked(toast).mockClear();

    let resolveModes: (rows: any[]) => void = () => {
      throw new Error("resolveModes not set");
    };
    let rejectActiveRows: (err: Error) => void = () => {
      throw new Error("rejectActiveRows not set");
    };
    const pendingModes = new Promise<any[]>((resolve) => {
      resolveModes = resolve;
    });
    const pendingActiveRows = new Promise<any[]>((_resolve, reject) => {
      rejectActiveRows = reject;
    });

    vi.mocked(sortModesList)
      .mockResolvedValueOnce([] as any)
      .mockReturnValueOnce(pendingModes as any);
    vi.mocked(sortModeActiveList)
      .mockResolvedValueOnce([] as any)
      .mockReturnValueOnce(pendingActiveRows as any);
    vi.mocked(sortModeProvidersList).mockResolvedValue([] as any);

    const { unmount } = renderWithQueryClient(
      <SortModesView
        activeCli="claude"
        setActiveCli={vi.fn()}
        providers={[] as any}
        providersLoading={false}
      />
    );

    await waitFor(() => expect(screen.getByRole("button", { name: "刷新" })).toBeEnabled());

    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    expect(screen.getByRole("button", { name: "刷新" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "刷新" }));

    expect(vi.mocked(sortModesList)).toHaveBeenCalledTimes(2);
    expect(vi.mocked(sortModeActiveList)).toHaveBeenCalledTimes(2);

    unmount();

    await act(async () => {
      resolveModes([]);
      rejectActiveRows(new Error("unmounted boom"));
      await pendingModes;
      await pendingActiveRows.catch(() => undefined);
    });

    expect(toast).not.toHaveBeenCalledWith(expect.stringContaining("读取排序模板失败"));
  });

  it("keeps rapid sort-mode CRUD submissions behind one in-flight mutation", async () => {
    vi.mocked(toast).mockClear();

    vi.mocked(sortModesList).mockResolvedValue([{ id: 1, name: "Work" }] as any);
    vi.mocked(sortModeActiveList).mockResolvedValue([{ cli_key: "claude", mode_id: 1 }] as any);
    vi.mocked(sortModeProvidersList).mockResolvedValue([] as any);

    const { result } = renderHook(
      () =>
        useSortModesDataModel({
          activeCli: "claude",
          setActiveCli: vi.fn(),
          providers: [] as any,
          providersLoading: false,
        }),
      { wrapper: queryWrapper() }
    );

    await waitFor(() => expect(result.current.selectedMode?.id).toBe(1));

    let resolveCreate: (mode: any) => void = () => {
      throw new Error("resolveCreate not set");
    };
    const pendingCreate = new Promise<any>((resolve) => {
      resolveCreate = resolve;
    });
    vi.mocked(sortModeCreate).mockReturnValueOnce(pendingCreate as any);
    act(() => result.current.setCreateModeName("Life"));
    await waitFor(() => expect(result.current.createModeName).toBe("Life"));

    let createPromise: Promise<void> | undefined;
    act(() => {
      createPromise = result.current.createSortMode();
      void result.current.createSortMode();
    });

    await waitFor(() => expect(vi.mocked(sortModeCreate)).toHaveBeenCalledTimes(1));
    expect(vi.mocked(sortModeCreate)).toHaveBeenCalledWith({ name: "Life" });

    await act(async () => {
      resolveCreate({ id: 2, name: "Life" });
      await createPromise;
    });
    await waitFor(() => expect(result.current.selectedMode?.id).toBe(2));

    let resolveRename: (mode: any) => void = () => {
      throw new Error("resolveRename not set");
    };
    const pendingRename = new Promise<any>((resolve) => {
      resolveRename = resolve;
    });
    vi.mocked(sortModeRename).mockReturnValueOnce(pendingRename as any);
    act(() => result.current.setRenameModeName("Life2"));
    await waitFor(() => expect(result.current.renameModeName).toBe("Life2"));

    let renamePromise: Promise<void> | undefined;
    act(() => {
      renamePromise = result.current.renameSortMode();
      void result.current.renameSortMode();
    });

    await waitFor(() => expect(vi.mocked(sortModeRename)).toHaveBeenCalledTimes(1));
    expect(vi.mocked(sortModeRename)).toHaveBeenCalledWith({ mode_id: 2, name: "Life2" });

    await act(async () => {
      resolveRename({ id: 2, name: "Life2" });
      await renamePromise;
    });
    await waitFor(() => expect(result.current.selectedMode?.name).toBe("Life2"));

    let resolveDelete: (ok: boolean) => void = () => {
      throw new Error("resolveDelete not set");
    };
    const pendingDelete = new Promise<boolean>((resolve) => {
      resolveDelete = resolve;
    });
    vi.mocked(sortModeDelete).mockReturnValueOnce(pendingDelete as any);
    act(() => result.current.setDeleteModeTarget(result.current.selectedMode));
    await waitFor(() => expect(result.current.deleteModeTarget?.id).toBe(2));

    let deletePromise: Promise<void> | undefined;
    act(() => {
      deletePromise = result.current.deleteSortMode();
      void result.current.deleteSortMode();
    });

    await waitFor(() => expect(vi.mocked(sortModeDelete)).toHaveBeenCalledTimes(1));
    expect(vi.mocked(sortModeDelete)).toHaveBeenCalledWith({ mode_id: 2 });

    await act(async () => {
      resolveDelete(true);
      await deletePromise;
    });
    await waitFor(() => expect(result.current.deleteModeTarget).toBeNull());
  });

  it("suppresses mode-provider save feedback after the sort modes view unmounts", async () => {
    vi.mocked(toast).mockClear();

    vi.mocked(sortModesList).mockResolvedValue([{ id: 1, name: "Work" }] as any);
    vi.mocked(sortModeActiveList).mockResolvedValue([{ cli_key: "claude", mode_id: 1 }] as any);
    vi.mocked(sortModeProvidersList).mockResolvedValue([
      { provider_id: 101, enabled: true },
    ] as any);

    let rejectOrder: (err: Error) => void = () => {
      throw new Error("rejectOrder not set");
    };
    const pendingOrder = new Promise<any[]>((_resolve, reject) => {
      rejectOrder = reject;
    });
    vi.mocked(sortModeProvidersSetOrder).mockReturnValueOnce(pendingOrder as any);

    const { result, unmount } = renderHook(
      () =>
        useSortModesDataModel({
          activeCli: "claude",
          setActiveCli: vi.fn(),
          providers: [
            { id: 101, name: "P1", enabled: true, base_urls: ["https://a"] },
            { id: 102, name: "P2", enabled: true, base_urls: ["https://b"] },
          ] as any,
          providersLoading: false,
        }),
      { wrapper: queryWrapper() }
    );

    await waitFor(() => expect(result.current.modeProviders).toHaveLength(1));

    act(() => {
      result.current.addProviderToMode(102);
    });
    expect(result.current.modeProvidersSaving).toBe(true);

    unmount();

    await act(async () => {
      rejectOrder(new Error("unmounted order boom"));
      await pendingOrder.catch(() => undefined);
    });

    expect(toast).not.toHaveBeenCalledWith(expect.stringContaining("模式顺序更新失败"));
  });

  it("suppresses stale mode-provider save failures after switching away from the mode", async () => {
    vi.mocked(toast).mockClear();
    vi.mocked(logToConsole).mockClear();

    vi.mocked(sortModesList).mockResolvedValue([{ id: 1, name: "Work" }] as any);
    vi.mocked(sortModeActiveList).mockResolvedValue([{ cli_key: "claude", mode_id: 1 }] as any);
    vi.mocked(sortModeProvidersList).mockResolvedValue([
      { provider_id: 101, enabled: true },
    ] as any);

    let rejectOrder: (err: Error) => void = () => {
      throw new Error("rejectOrder not set");
    };
    const pendingOrder = new Promise<any[]>((_resolve, reject) => {
      rejectOrder = reject;
    });
    vi.mocked(sortModeProvidersSetOrder).mockReturnValueOnce(pendingOrder as any);

    const { result } = renderHook(
      () =>
        useSortModesDataModel({
          activeCli: "claude",
          setActiveCli: vi.fn(),
          providers: [
            { id: 101, name: "P1", enabled: true, base_urls: ["https://a"] },
            { id: 102, name: "P2", enabled: true, base_urls: ["https://b"] },
          ] as any,
          providersLoading: false,
        }),
      { wrapper: queryWrapper() }
    );

    await waitFor(() => expect(result.current.selectedMode?.id).toBe(1));
    await waitFor(() => expect(result.current.modeProviders).toHaveLength(1));

    act(() => {
      result.current.addProviderToMode(102);
    });
    expect(result.current.modeProvidersSaving).toBe(true);

    act(() => {
      result.current.selectEditingMode(null);
    });
    await waitFor(() => expect(result.current.activeModeId).toBeNull());

    await act(async () => {
      rejectOrder(new Error("stale order boom"));
      await pendingOrder.catch(() => undefined);
    });

    expect(toast).not.toHaveBeenCalledWith(expect.stringContaining("模式顺序更新失败"));
    expect(logToConsole).not.toHaveBeenCalledWith(
      "error",
      "更新排序模板顺序失败",
      expect.anything()
    );
    await waitFor(() => expect(result.current.modeProvidersSaving).toBe(false));
  });

  it("clears rename and delete targets when the selected sort mode disappears", async () => {
    vi.mocked(sortModesList)
      .mockResolvedValueOnce([{ id: 1, name: "Work" }] as any)
      .mockResolvedValueOnce([] as any);
    vi.mocked(sortModeActiveList)
      .mockResolvedValueOnce([{ cli_key: "claude", mode_id: 1 }] as any)
      .mockResolvedValueOnce([] as any);
    vi.mocked(sortModeProvidersList).mockResolvedValue([] as any);

    const { result } = renderHook(
      () =>
        useSortModesDataModel({
          activeCli: "claude",
          setActiveCli: vi.fn(),
          providers: [] as any,
          providersLoading: false,
        }),
      { wrapper: queryWrapper() }
    );

    await waitFor(() => expect(result.current.selectedMode?.id).toBe(1));

    act(() => {
      result.current.setRenameModeDialogOpen(true);
      result.current.setDeleteModeTarget(result.current.selectedMode);
    });
    await waitFor(() => expect(result.current.renameModeDialogOpen).toBe(true));
    await waitFor(() => expect(result.current.deleteModeTarget?.id).toBe(1));

    await act(async () => {
      await result.current.refreshSortModes();
    });

    await waitFor(() => expect(result.current.renameModeDialogOpen).toBe(false));
    expect(result.current.renameModeName).toBe("");
    expect(result.current.deleteModeTarget).toBeNull();
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
    vi.mocked(sortModeDelete).mockResolvedValue(true);
    fireEvent.click(deleteDialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(vi.mocked(sortModeDelete)).toHaveBeenCalledWith({ mode_id: 2 }));
  });

  it("reorders all template rows from the order panel", async () => {
    vi.mocked(sortModesList).mockResolvedValue([{ id: 1, name: "Work" }] as any);
    vi.mocked(sortModeActiveList).mockResolvedValue([{ cli_key: "claude", mode_id: 1 }] as any);
    vi.mocked(sortModeProvidersList).mockResolvedValue([
      { provider_id: 101, enabled: true },
      { provider_id: 102, enabled: false },
      { provider_id: 103, enabled: true },
    ] as any);
    vi.mocked(sortModeProvidersSetOrder).mockClear();
    vi.mocked(sortModeProvidersSetOrder).mockResolvedValue([
      { provider_id: 102, enabled: false },
      { provider_id: 103, enabled: true },
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
            {
              id: 102,
              name: "P2",
              enabled: true,
              base_urls: ["https://b"],
              base_url_mode: "order",
            },
            {
              id: 103,
              name: "P3",
              enabled: true,
              base_urls: ["https://c"],
              base_url_mode: "order",
            },
          ] as any
        }
        providersLoading={false}
      />
    );

    await waitFor(() => expect(screen.getByRole("button", { name: "Work" })).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: "Work" }));
    const orderPanel = () =>
      within(screen.getByRole("complementary", { name: "排序模板调用顺序" }));
    await waitFor(() => expect(orderPanel().getByText("P1")).toBeInTheDocument());
    expect(orderPanel().getByText("P2")).toBeInTheDocument();
    expect(orderPanel().getByText("P3")).toBeInTheDocument();

    latestOnDragEnd?.({ active: { id: 101 }, over: { id: 103 } });
    await waitFor(() =>
      expect(vi.mocked(sortModeProvidersSetOrder)).toHaveBeenCalledWith(
        expect.objectContaining({ ordered_provider_ids: [102, 103, 101] })
      )
    );
  });

  it("keeps rapid drag saves behind one in-flight provider order mutation", async () => {
    vi.mocked(sortModesList).mockResolvedValue([{ id: 1, name: "Work" }] as any);
    vi.mocked(sortModeActiveList).mockResolvedValue([{ cli_key: "claude", mode_id: 1 }] as any);
    vi.mocked(sortModeProvidersList).mockResolvedValue([
      { provider_id: 101, enabled: true },
      { provider_id: 102, enabled: true },
      { provider_id: 103, enabled: true },
    ] as any);

    let resolveOrder: (rows: any[]) => void = () => {
      throw new Error("resolveOrder not set");
    };
    const pendingOrder = new Promise<any[]>((resolve) => {
      resolveOrder = resolve;
    });
    vi.mocked(sortModeProvidersSetOrder).mockReturnValue(pendingOrder as any);

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
              enabled: true,
              base_urls: ["https://b"],
              base_url_mode: "order",
            },
            {
              id: 103,
              name: "P3",
              enabled: true,
              base_urls: ["https://c"],
              base_url_mode: "order",
            },
          ] as any
        }
        providersLoading={false}
      />
    );

    const orderPanel = () =>
      within(screen.getByRole("complementary", { name: "排序模板调用顺序" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Work" })).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: "Work" }));
    await waitFor(() =>
      expect(orderPanel().getAllByRole("button", { name: "移除" })).toHaveLength(3)
    );

    await act(async () => {
      latestOnDragEnd?.({ active: { id: 101 }, over: { id: 102 } });
      latestOnDragEnd?.({ active: { id: 102 }, over: { id: 103 } });
    });

    expect(vi.mocked(sortModeProvidersSetOrder)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(sortModeProvidersSetOrder)).toHaveBeenCalledWith(
      expect.objectContaining({ ordered_provider_ids: [102, 101, 103] })
    );

    resolveOrder([
      { provider_id: 102, enabled: true },
      { provider_id: 101, enabled: true },
      { provider_id: 103, enabled: true },
    ]);
    await waitFor(() => expect(vi.mocked(toast)).toHaveBeenCalledWith("模式顺序已更新"));
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
    fireEvent.click(screen.getByRole("button", { name: "Work" }));
    const orderPanel = () =>
      within(screen.getByRole("complementary", { name: "排序模板调用顺序" }));
    await waitFor(() => expect(orderPanel().getByText("P1")).toBeInTheDocument());
    expect(orderPanel().getByText("P2")).toBeInTheDocument();
    await waitFor(() =>
      expect(orderPanel().getByRole("switch", { name: "启用 P2" })).toBeInTheDocument()
    );

    fireEvent.click(orderPanel().getByRole("switch", { name: "启用 P2" }));
    await waitFor(() =>
      expect(vi.mocked(sortModeProviderSetEnabled)).toHaveBeenCalledWith({
        mode_id: 1,
        cli_key: "claude",
        provider_id: 102,
        enabled: true,
      })
    );
    await waitFor(() => expect(orderPanel().getByText("P2")).toBeInTheDocument());
  });

  it("covers create/rename validation, error branches, and delete dialog gating", async () => {
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

    fireEvent.click(screen.getByRole("button", { name: "新建排序模板" }));
    const createDialog = within(screen.getByRole("dialog"));
    fireEvent.change(createDialog.getByPlaceholderText("工作"), { target: { value: "Life" } });

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

    fireEvent.change(renameDialog.getByRole("textbox"), { target: { value: "Work2" } });

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
    vi.mocked(sortModeDelete).mockResolvedValueOnce(false);
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
    vi.mocked(sortModeDelete).mockImplementationOnce(() => deletePromise);
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

    const orderPanel = within(screen.getByRole("complementary", { name: "排序模板调用顺序" }));
    await waitFor(() =>
      expect(orderPanel.getByRole("button", { name: "移除" })).toBeInTheDocument()
    );
    expect(
      screen.getByRole("complementary", { name: "排序模板调用顺序" }).querySelector(".ring-2")
    ).toBeTruthy();

    sortableIsDragging = false;
  });

  it("covers remove provider and persist order error/success branches", async () => {
    vi.mocked(toast).mockClear();
    vi.mocked(sortModesList).mockResolvedValue([{ id: 1, name: "Work" }] as any);
    vi.mocked(sortModeActiveList).mockResolvedValue([{ cli_key: "claude", mode_id: 1 }] as any);
    vi.mocked(sortModeProvidersList).mockResolvedValue([
      { provider_id: 101, enabled: true },
      { provider_id: 102, enabled: true },
    ] as any);

    vi.mocked(sortModeProvidersSetOrder)
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
    const orderPanel = () =>
      within(screen.getByRole("complementary", { name: "排序模板调用顺序" }));
    await waitFor(() =>
      expect(orderPanel().getAllByRole("button", { name: "移除" })).toHaveLength(2)
    );

    // pointerdown handler stops propagation (coverage)
    fireEvent.pointerDown(orderPanel().getAllByRole("button", { name: "移除" })[0]!);

    // 1) persist throws -> toast and revert
    fireEvent.click(orderPanel().getAllByRole("button", { name: "移除" })[0]!);
    await waitFor(() => expect(vi.mocked(sortModeProvidersSetOrder)).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(vi.mocked(toast)).toHaveBeenCalled());
    await waitFor(() =>
      expect(orderPanel().getAllByRole("button", { name: "移除" })).toHaveLength(2)
    );

    // 2) persist succeeds -> P1 removed from mode
    fireEvent.click(orderPanel().getAllByRole("button", { name: "移除" })[0]!);
    await waitFor(() => expect(vi.mocked(sortModeProvidersSetOrder)).toHaveBeenCalledTimes(2));
    await waitFor(() =>
      expect(orderPanel().getAllByRole("button", { name: "移除" })).toHaveLength(1)
    );

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
