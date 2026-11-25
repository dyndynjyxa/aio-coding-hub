import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ButtonHTMLAttributes, ReactElement, ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import {
  type McpImportReport,
  useMcpImportFromWorkspaceCliMutation,
  useMcpServerDeleteMutation,
  useMcpServerSetEnabledMutation,
  useMcpServersListQuery,
} from "../../../query/mcp";
import { logToConsole } from "../../../services/consoleLog";
import { type McpServerSummary } from "../../../services/workspace/mcp";
import { createTestQueryClient } from "../../../test/utils/reactQuery";
import type { McpDeleteDialogProps } from "../components/McpDeleteDialog";
import type { McpServerCardProps } from "../components/McpServerCard";
import type { McpServerDialogProps } from "../components/McpServerDialog";
import { McpServersView } from "../McpServersView";

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));

vi.mock("../../../ui/Button", () => ({
  Button: ({
    children,
    onClick,
    disabled,
    variant: _variant,
    size: _size,
    className: _className,
    ...rest
  }: {
    children?: ReactNode;
    onClick?: () => void;
    disabled?: boolean;
    variant?: unknown;
    size?: unknown;
    className?: string;
    [key: string]: unknown;
  }) => (
    <button
      type="button"
      data-disabled={disabled ? "true" : "false"}
      onClick={() => onClick?.()}
      {...(rest as Omit<ButtonHTMLAttributes<HTMLButtonElement>, "onClick">)}
    >
      {children}
    </button>
  ),
}));

vi.mock("../components/McpServerCard", () => ({
  McpServerCard: ({ server, toggling, onToggleEnabled, onEdit, onDelete }: McpServerCardProps) => (
    <div data-testid={`server-card-${server.id}`}>
      <span>{server.name}</span>
      <span>{toggling ? "切换中" : "空闲"}</span>
      <button type="button" onClick={() => onToggleEnabled(server)}>
        {`toggle-${server.id}`}
      </button>
      <button type="button" onClick={() => onEdit(server)}>
        {`edit-${server.id}`}
      </button>
      <button type="button" onClick={() => onDelete(server)}>
        {`delete-${server.id}`}
      </button>
    </div>
  ),
}));

vi.mock("../components/McpServerDialog", () => ({
  McpServerDialog: ({ open, editTarget, workspaceId, onOpenChange }: McpServerDialogProps) => (
    <div
      data-testid="server-dialog"
      data-open={open ? "true" : "false"}
      data-target={editTarget?.name ?? ""}
      data-workspace-id={String(workspaceId)}
    >
      <span>{open ? "dialog-open" : "dialog-closed"}</span>
      <span>{editTarget ? `editing:${editTarget.name}` : "editing:none"}</span>
      <button type="button" onClick={() => onOpenChange(false)}>
        close-server-dialog
      </button>
    </div>
  ),
}));

vi.mock("../components/McpDeleteDialog", () => ({
  McpDeleteDialog: ({ target, deleting, onConfirm, onClose }: McpDeleteDialogProps) => (
    <div
      data-testid="delete-dialog"
      data-target={target?.name ?? ""}
      data-deleting={deleting ? "true" : "false"}
    >
      <span>{target ? `delete:${target.name}` : "delete:none"}</span>
      <button type="button" onClick={onConfirm}>
        confirm-delete
      </button>
      <button type="button" onClick={onClose}>
        close-delete
      </button>
    </div>
  ),
}));

vi.mock("../../../query/mcp", async () => {
  const actual = await vi.importActual<typeof import("../../../query/mcp")>("../../../query/mcp");
  return {
    ...actual,
    useMcpServersListQuery: vi.fn(),
    useMcpServerSetEnabledMutation: vi.fn(),
    useMcpServerDeleteMutation: vi.fn(),
    useMcpImportFromWorkspaceCliMutation: vi.fn(),
  };
});

function renderWithQuery(element: ReactElement) {
  const client = createTestQueryClient();
  return render(<QueryClientProvider client={client}>{element}</QueryClientProvider>);
}

function createServer(overrides: Partial<McpServerSummary> = {}): McpServerSummary {
  return {
    id: 1,
    server_key: "fetch",
    name: "Fetch Tool",
    transport: "http",
    url: "https://example.com/mcp",
    enabled: false,
    command: null,
    args: [],
    env_keys: [],
    cwd: null,
    header_keys: [],
    created_at: 1,
    updated_at: 1,
    ...overrides,
  };
}

function createImportReport(overrides: Partial<McpImportReport> = {}): McpImportReport {
  return {
    inserted: 0,
    updated: 0,
    skipped: [],
    ...overrides,
  };
}

type ListQueryMock = Pick<
  ReturnType<typeof useMcpServersListQuery>,
  "data" | "isFetching" | "error"
>;
type ToggleMutationMock = Pick<
  ReturnType<typeof useMcpServerSetEnabledMutation>,
  "isPending" | "mutateAsync"
>;
type DeleteMutationMock = Pick<
  ReturnType<typeof useMcpServerDeleteMutation>,
  "isPending" | "mutateAsync"
>;
type ImportMutationMock = Pick<
  ReturnType<typeof useMcpImportFromWorkspaceCliMutation>,
  "isPending" | "mutateAsync"
>;

function createMutation(options: { isPending?: boolean } = {}) {
  return {
    isPending: options.isPending ?? false,
    mutateAsync: vi.fn(),
  };
}

function mockView(
  options: {
    data?: McpServerSummary[] | null;
    isFetching?: boolean;
    error?: Error | null;
    toggleMutation?: ToggleMutationMock;
    deleteMutation?: DeleteMutationMock;
    importMutation?: ImportMutationMock;
  } = {}
) {
  const toggleMutation = options.toggleMutation ?? createMutation();
  const deleteMutation = options.deleteMutation ?? createMutation();
  const importMutation = options.importMutation ?? createMutation();

  const listQuery: ListQueryMock = {
    data: options.data ?? [],
    isFetching: options.isFetching ?? false,
    error: options.error ?? null,
  };

  vi.mocked(useMcpServersListQuery).mockReturnValue(
    listQuery as ReturnType<typeof useMcpServersListQuery>
  );
  vi.mocked(useMcpServerSetEnabledMutation).mockReturnValue(
    toggleMutation as ReturnType<typeof useMcpServerSetEnabledMutation>
  );
  vi.mocked(useMcpServerDeleteMutation).mockReturnValue(
    deleteMutation as ReturnType<typeof useMcpServerDeleteMutation>
  );
  vi.mocked(useMcpImportFromWorkspaceCliMutation).mockReturnValue(
    importMutation as ReturnType<typeof useMcpImportFromWorkspaceCliMutation>
  );

  return { toggleMutation, deleteMutation, importMutation };
}

beforeEach(() => {
  vi.clearAllMocks();
  mockView();
});

describe("pages/mcp/McpServersView", () => {
  it("renders empty state when there are no servers", () => {
    renderWithQuery(<McpServersView cliKey="claude" workspaceId={1} />);

    expect(screen.getByText("暂无 MCP 服务")).toBeInTheDocument();
    expect(screen.getByText("共 0 条")).toBeInTheDocument();
  });

  it("renders loading state and reports list query errors", async () => {
    mockView({
      data: null,
      isFetching: true,
      error: new Error("boom"),
    });

    renderWithQuery(<McpServersView cliKey="claude" workspaceId={9} />);

    expect(screen.getAllByText("加载中…")).not.toHaveLength(0);

    await waitFor(() => {
      expect(logToConsole).toHaveBeenCalledWith("error", "加载 MCP Servers 失败", {
        error: "Error: boom",
      });
    });
    expect(toast).toHaveBeenCalledWith("加载失败：请查看控制台日志");
  });

  it("covers toggle pending guard plus success and error branches", async () => {
    const server = createServer({ id: 7, enabled: true, name: "Runner" });

    const pendingToggle = createMutation({ isPending: true });
    mockView({
      data: [server],
      toggleMutation: pendingToggle,
    });

    const pendingView = renderWithQuery(<McpServersView cliKey="claude" workspaceId={1} />);
    fireEvent.click(screen.getByRole("button", { name: "toggle-7" }));
    expect(pendingToggle.mutateAsync).not.toHaveBeenCalled();
    pendingView.unmount();

    const toggleMutation = createMutation();
    toggleMutation.mutateAsync
      .mockResolvedValueOnce({ ...server, enabled: false })
      .mockRejectedValueOnce(new Error("toggle boom"));

    mockView({
      data: [server],
      toggleMutation,
    });

    renderWithQuery(<McpServersView cliKey="claude" workspaceId={1} />);

    fireEvent.click(screen.getByRole("button", { name: "toggle-7" }));
    await waitFor(() => {
      expect(toggleMutation.mutateAsync).toHaveBeenNthCalledWith(1, {
        serverId: 7,
        enabled: false,
      });
    });
    expect(logToConsole).toHaveBeenCalledWith("info", "切换 MCP Server 生效范围", {
      id: 7,
      server_key: "fetch",
      workspace_id: 1,
      enabled: false,
    });
    expect(toast).toHaveBeenCalledWith("已停用");

    fireEvent.click(screen.getByRole("button", { name: "toggle-7" }));
    await waitFor(() => {
      expect(logToConsole).toHaveBeenCalledWith("error", "切换 MCP Server 生效范围失败", {
        error: "Error: toggle boom",
        id: 7,
        workspace_id: 1,
      });
    });
    expect(toast).toHaveBeenCalledWith("操作失败：Error: toggle boom");
  });

  it("covers delete guards plus false, error, success, and manual close branches", async () => {
    const server = createServer({ id: 3, name: "Delete Me" });

    const pendingDelete = createMutation({ isPending: true });
    mockView({
      data: [server],
      deleteMutation: pendingDelete,
    });

    const pendingView = renderWithQuery(<McpServersView cliKey="claude" workspaceId={1} />);
    fireEvent.click(screen.getByRole("button", { name: "confirm-delete" }));
    expect(pendingDelete.mutateAsync).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "delete-3" }));
    fireEvent.click(screen.getByRole("button", { name: "confirm-delete" }));
    expect(pendingDelete.mutateAsync).not.toHaveBeenCalled();
    pendingView.unmount();

    const deleteMutation = createMutation();
    deleteMutation.mutateAsync
      .mockResolvedValueOnce(false)
      .mockRejectedValueOnce(new Error("delete boom"))
      .mockResolvedValueOnce(true);

    mockView({
      data: [server],
      deleteMutation,
    });

    renderWithQuery(<McpServersView cliKey="claude" workspaceId={1} />);

    fireEvent.click(screen.getByRole("button", { name: "delete-3" }));
    expect(screen.getByText("delete:Delete Me")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "confirm-delete" }));
    await waitFor(() => {
      expect(deleteMutation.mutateAsync).toHaveBeenNthCalledWith(1, 3);
    });
    expect(logToConsole).not.toHaveBeenCalled();
    expect(toast).not.toHaveBeenCalled();
    expect(screen.getByText("delete:Delete Me")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "confirm-delete" }));
    await waitFor(() => {
      expect(logToConsole).toHaveBeenCalledWith("error", "删除 MCP Server 失败", {
        error: "Error: delete boom",
        id: 3,
      });
    });
    expect(toast).toHaveBeenCalledWith("删除失败：Error: delete boom");
    expect(screen.getByText("delete:Delete Me")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "confirm-delete" }));
    await waitFor(() => {
      expect(logToConsole).toHaveBeenCalledWith("info", "删除 MCP Server", {
        id: 3,
        server_key: "fetch",
      });
    });
    expect(toast).toHaveBeenCalledWith("已删除");
    expect(screen.getByText("delete:none")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "delete-3" }));
    expect(screen.getByText("delete:Delete Me")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "close-delete" }));
    expect(screen.getByText("delete:none")).toBeInTheDocument();
  });

  it("covers import guard, both summary branches, and error branch", async () => {
    const pendingImport = createMutation({ isPending: true });
    mockView({
      importMutation: pendingImport,
    });

    const pendingView = renderWithQuery(<McpServersView cliKey="claude" workspaceId={11} />);
    expect(screen.getByRole("button", { name: "导入中…" })).toHaveAttribute(
      "data-disabled",
      "true"
    );
    fireEvent.click(screen.getByRole("button", { name: "导入中…" }));
    expect(pendingImport.mutateAsync).not.toHaveBeenCalled();
    pendingView.unmount();

    const importMutation = createMutation();
    importMutation.mutateAsync
      .mockResolvedValueOnce(createImportReport({ inserted: 2, updated: 1 }))
      .mockResolvedValueOnce(
        createImportReport({
          inserted: 1,
          updated: 0,
          skipped: [{ name: "Fetch Tool", reason: "duplicate" }],
        })
      )
      .mockRejectedValueOnce(new Error("import boom"));

    mockView({
      importMutation,
    });

    renderWithQuery(<McpServersView cliKey="claude" workspaceId={11} />);

    fireEvent.click(screen.getByRole("button", { name: "导入已有" }));
    await waitFor(() => {
      expect(toast).toHaveBeenCalledWith("导入完成：新增 2，更新 1");
    });
    expect(logToConsole).toHaveBeenCalledWith("info", "从当前 CLI 自动导入 MCP 完成", {
      workspace_id: 11,
      inserted: 2,
      updated: 1,
      skipped: [],
    });

    fireEvent.click(screen.getByRole("button", { name: "导入已有" }));
    await waitFor(() => {
      expect(toast).toHaveBeenCalledWith("导入完成：新增 1，更新 0，跳过 1");
    });
    expect(logToConsole).toHaveBeenCalledWith("info", "从当前 CLI 自动导入 MCP 完成", {
      workspace_id: 11,
      inserted: 1,
      updated: 0,
      skipped: [{ name: "Fetch Tool", reason: "duplicate" }],
    });

    fireEvent.click(screen.getByRole("button", { name: "导入已有" }));
    await waitFor(() => {
      expect(logToConsole).toHaveBeenCalledWith("error", "从当前 CLI 自动导入 MCP 失败", {
        error: "Error: import boom",
        workspace_id: 11,
      });
    });
    expect(toast).toHaveBeenCalledWith("导入失败：Error: import boom");
  });

  it("opens add and edit dialogs and clears edit target when closing", () => {
    const server = createServer({ id: 5, name: "Editable Server" });
    mockView({
      data: [server],
    });

    renderWithQuery(<McpServersView cliKey="claude" workspaceId={1} />);

    expect(screen.getByTestId("server-dialog")).toHaveAttribute("data-open", "false");
    expect(screen.getByText("editing:none")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "添加 MCP" }));
    expect(screen.getByTestId("server-dialog")).toHaveAttribute("data-open", "true");
    expect(screen.getByText("editing:none")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "close-server-dialog" }));
    expect(screen.getByTestId("server-dialog")).toHaveAttribute("data-open", "false");
    expect(screen.getByText("editing:none")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "edit-5" }));
    expect(screen.getByTestId("server-dialog")).toHaveAttribute("data-open", "true");
    expect(screen.getByText("editing:Editable Server")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "close-server-dialog" }));
    expect(screen.getByTestId("server-dialog")).toHaveAttribute("data-open", "false");
    expect(screen.getByText("editing:none")).toBeInTheDocument();
  });
});
