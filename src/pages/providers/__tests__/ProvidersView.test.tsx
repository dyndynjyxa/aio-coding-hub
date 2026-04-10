import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClientProvider } from "@tanstack/react-query";
import type { ReactElement } from "react";
import { toast } from "sonner";
import { ProvidersView } from "../ProvidersView";
import { createTestQueryClient } from "../../../test/utils/reactQuery";
import { copyText } from "../../../services/clipboard";
import { logToConsole } from "../../../services/consoleLog";
import { providerGetApiKey } from "../../../services/providers/providers";
import { gatewayKeys, providersKeys } from "../../../query/keys";
import {
  useGatewayCircuitResetCliMutation,
  useGatewayCircuitResetProviderMutation,
  useGatewayCircuitStatusQuery,
} from "../../../query/gateway";
import {
  useProviderClaudeTerminalLaunchCommandMutation,
  useProviderDeleteMutation,
  useProviderSetEnabledMutation,
  useProvidersListQuery,
  useProvidersReorderMutation,
} from "../../../query/providers";

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
vi.mock("../../../services/clipboard", () => ({ copyText: vi.fn() }));
vi.mock("../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));
vi.mock("../../../services/providers/providers", async () => {
  const actual = await vi.importActual<typeof import("../../../services/providers/providers")>(
    "../../../services/providers/providers"
  );
  return {
    ...actual,
    providerGetApiKey: vi.fn(),
  };
});

vi.mock("../../../components/ClaudeModelValidationDialog", () => ({
  ClaudeModelValidationDialog: ({ open, onOpenChange }: any) =>
    open ? (
      <div>
        validate
        <button type="button" onClick={() => onOpenChange?.(false)}>
          close-validate
        </button>
      </div>
    ) : null,
}));

vi.mock("../ProviderEditorDialog", () => ({
  ProviderEditorDialog: ({ mode, cliKey, provider, initialValues, onSaved, onOpenChange }: any) => (
    <div
      data-testid="provider-editor"
      data-initial-name={initialValues?.name ?? ""}
      data-api-key={initialValues?.api_key ?? ""}
      data-auth-mode={initialValues?.auth_mode ?? ""}
    >
      {mode}
      <button type="button" onClick={() => onSaved?.(cliKey ?? provider?.cli_key)}>
        saved
      </button>
      <button type="button" onClick={() => onOpenChange?.(false)}>
        close-editor
      </button>
    </div>
  ),
}));

vi.mock("../../../query/gateway", async () => {
  const actual =
    await vi.importActual<typeof import("../../../query/gateway")>("../../../query/gateway");
  return {
    ...actual,
    useGatewayCircuitStatusQuery: vi.fn(),
    useGatewayCircuitResetProviderMutation: vi.fn(),
    useGatewayCircuitResetCliMutation: vi.fn(),
  };
});

vi.mock("../../../query/providers", async () => {
  const actual = await vi.importActual<typeof import("../../../query/providers")>(
    "../../../query/providers"
  );
  return {
    ...actual,
    useProvidersListQuery: vi.fn(),
    useProviderClaudeTerminalLaunchCommandMutation: vi.fn(),
    useProviderSetEnabledMutation: vi.fn(),
    useProviderDeleteMutation: vi.fn(),
    useProvidersReorderMutation: vi.fn(),
  };
});

function renderWithQuery(element: ReactElement) {
  const client = createTestQueryClient();
  return render(<QueryClientProvider client={client}>{element}</QueryClientProvider>);
}

beforeEach(() => {
  vi.mocked(copyText).mockResolvedValue(undefined);
  vi.mocked(providerGetApiKey).mockResolvedValue("sk-dup");
  vi.mocked(useProviderClaudeTerminalLaunchCommandMutation).mockReturnValue({
    mutateAsync: vi.fn().mockResolvedValue("bash '/tmp/aio.sh'"),
  } as any);
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  sortableIsDragging = false;
});

describe("pages/providers/ProvidersView", () => {
  it("treats cooldown-only circuits as unavailable for reset-all visibility", () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);

    vi.mocked(useProvidersListQuery).mockReturnValue({
      data: [
        {
          id: 1,
          cli_key: "claude",
          name: "P1",
          enabled: true,
          base_urls: ["https://a"],
          base_url_mode: "order",
          cost_multiplier: 1,
          claude_models: {},
        },
      ],
      isFetching: false,
      error: null,
    } as any);

    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [
        {
          provider_id: 1,
          state: "CLOSED",
          open_until: null,
          cooldown_until: Math.floor(Date.now() / 1000) + 30,
        },
      ],
      isFetching: false,
      error: null,
      refetch: vi.fn().mockResolvedValue({ data: [] }),
    } as any);
    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    renderWithQuery(<ProvidersView activeCli="claude" setActiveCli={vi.fn()} />);

    expect(screen.getByRole("button", { name: "解除熔断（全部）" })).toBeInTheDocument();
    expect(screen.getByText(/^熔断\s*00:30$/)).toBeInTheDocument();
  });

  it("does not show reset-all visibility for HALF_OPEN probe state", () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);

    vi.mocked(useProvidersListQuery).mockReturnValue({
      data: [
        {
          id: 1,
          cli_key: "claude",
          name: "P1",
          enabled: true,
          base_urls: ["https://a"],
          base_url_mode: "order",
          cost_multiplier: 1,
          claude_models: {},
        },
      ],
      isFetching: false,
      error: null,
    } as any);

    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [
        {
          provider_id: 1,
          state: "HALF_OPEN",
          open_until: null,
          cooldown_until: null,
        },
      ],
      isFetching: false,
      error: null,
      refetch: vi.fn().mockResolvedValue({ data: [] }),
    } as any);
    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    renderWithQuery(<ProvidersView activeCli="claude" setActiveCli={vi.fn()} />);

    expect(screen.queryByRole("button", { name: "解除熔断（全部）" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "解除熔断" })).not.toBeInTheDocument();
  });

  it("shows cx2cc source provider name on claude cards", () => {
    vi.mocked(useProvidersListQuery).mockImplementation((cliKey: any) => {
      if (cliKey === "codex") {
        return {
          data: [
            {
              id: 7,
              cli_key: "codex",
              name: "OpenAI Primary",
              enabled: true,
              base_urls: ["https://codex.example.com"],
              base_url_mode: "order",
              cost_multiplier: 1,
              claude_models: {},
            },
          ],
          isFetching: false,
          error: null,
        } as any;
      }

      return {
        data: [
          {
            id: 2,
            cli_key: "claude",
            name: "Claude Bridge",
            enabled: true,
            base_urls: [],
            base_url_mode: "order",
            cost_multiplier: 1,
            claude_models: { main_model: "claude-sonnet-4-5" },
            source_provider_id: 7,
          },
        ],
        isFetching: false,
        error: null,
      } as any;
    });

    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
      refetch: vi.fn().mockResolvedValue({ data: [] }),
    } as any);
    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    renderWithQuery(<ProvidersView activeCli="claude" setActiveCli={vi.fn()} />);

    expect(
      screen.getAllByText((_, el) => el?.textContent === "源: OpenAI Primary").length
    ).toBeGreaterThan(0);
  });

  it("supports toggling, circuit reset, create/edit/delete, and drag reorder", async () => {
    const providers = [
      {
        id: 1,
        cli_key: "claude",
        name: "P1",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: { main_model: "claude-3" },
      },
      {
        id: 2,
        cli_key: "claude",
        name: "P2",
        enabled: false,
        base_urls: ["https://b"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: {},
      },
    ] as any[];

    vi.mocked(useProvidersListQuery).mockReturnValue({
      data: providers,
      isFetching: false,
      error: null,
    } as any);

    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [
        { provider_id: 1, state: "OPEN", open_until: null, cooldown_until: null },
        { provider_id: 2, state: "CLOSED", open_until: null, cooldown_until: null },
      ],
      isFetching: false,
      error: null,
      refetch: vi.fn().mockResolvedValue({ data: [] }),
    } as any);

    const toggleMutation = { isPending: false, mutateAsync: vi.fn() };
    toggleMutation.mutateAsync.mockResolvedValue({ ...providers[1], enabled: true });
    vi.mocked(useProviderSetEnabledMutation).mockReturnValue(toggleMutation as any);

    const deleteMutation = { isPending: false, mutateAsync: vi.fn() };
    deleteMutation.mutateAsync.mockResolvedValue(true);
    vi.mocked(useProviderDeleteMutation).mockReturnValue(deleteMutation as any);

    const reorderMutation = { isPending: false, mutateAsync: vi.fn() };
    reorderMutation.mutateAsync.mockResolvedValue([providers[1], providers[0]]);
    vi.mocked(useProvidersReorderMutation).mockReturnValue(reorderMutation as any);

    const resetProviderMutation = { isPending: false, mutateAsync: vi.fn(), variables: null };
    resetProviderMutation.mutateAsync.mockResolvedValue(true);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue(resetProviderMutation as any);

    const resetCliMutation = { isPending: false, mutateAsync: vi.fn(), variables: null };
    resetCliMutation.mutateAsync.mockResolvedValue(1);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue(resetCliMutation as any);

    const copyLaunchMutation = { mutateAsync: vi.fn().mockResolvedValue("bash '/tmp/aio.sh'") };
    vi.mocked(useProviderClaudeTerminalLaunchCommandMutation).mockReturnValue(
      copyLaunchMutation as any
    );
    vi.mocked(copyText).mockResolvedValue(undefined);

    renderWithQuery(<ProvidersView activeCli="claude" setActiveCli={vi.fn()} />);

    // Toggle provider 2 to enabled.
    fireEvent.click(screen.getAllByRole("switch")[1]!);
    await waitFor(() =>
      expect(toggleMutation.mutateAsync).toHaveBeenCalledWith({ providerId: 2, enabled: true })
    );

    // Reset circuit for provider 1 (OPEN).
    fireEvent.click(screen.getByRole("button", { name: "解除熔断" }));
    await waitFor(() =>
      expect(resetProviderMutation.mutateAsync).toHaveBeenCalledWith({
        cliKey: "claude",
        providerId: 1,
      })
    );

    // Reset circuit all.
    fireEvent.click(screen.getByRole("button", { name: "解除熔断（全部）" }));
    await waitFor(() =>
      expect(resetCliMutation.mutateAsync).toHaveBeenCalledWith({ cliKey: "claude" })
    );

    // Copy launch command.
    fireEvent.click(screen.getAllByRole("button", { name: "终端启动" })[0]!);
    await waitFor(() =>
      expect(copyLaunchMutation.mutateAsync).toHaveBeenCalledWith({ providerId: 1 })
    );
    await waitFor(() => expect(copyText).toHaveBeenCalledWith("bash '/tmp/aio.sh'"));
    expect(toast).toHaveBeenCalledWith("已复制, 请在目标文件夹终端粘贴执行");

    // Open create dialog (mocked ProviderEditorDialog).
    fireEvent.click(screen.getByRole("button", { name: "添加" }));
    expect(
      screen.getAllByTestId("provider-editor").some((el) => el.textContent?.includes("create"))
    ).toBe(true);

    // Open edit dialog.
    fireEvent.click(screen.getAllByTitle("编辑")[0]!);
    expect(
      screen.getAllByTestId("provider-editor").some((el) => el.textContent?.includes("edit"))
    ).toBe(true);

    // Delete provider 1.
    fireEvent.click(screen.getAllByTitle("删除")[0]!);
    fireEvent.click(screen.getByRole("button", { name: "确认删除" }));
    await waitFor(() =>
      expect(deleteMutation.mutateAsync).toHaveBeenCalledWith({ cliKey: "claude", providerId: 1 })
    );

    // Drag reorder providers (1 -> 2).
    latestOnDragEnd?.({ active: { id: 1 }, over: { id: 2 } });
    await waitFor(() =>
      expect(reorderMutation.mutateAsync).toHaveBeenCalledWith({
        cliKey: "claude",
        orderedProviderIds: [2, 1],
      })
    );
  });

  it("duplicates a provider into a prefilled create dialog", async () => {
    const providers = [
      {
        id: 1,
        cli_key: "claude",
        name: "P1",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: { main_model: "claude-3" },
        limit_5h_usd: 5,
        limit_daily_usd: 10,
        daily_reset_mode: "fixed",
        daily_reset_time: "01:02:03",
        limit_weekly_usd: 15,
        limit_monthly_usd: 20,
        limit_total_usd: 25,
        tags: ["prod"],
        note: "copied",
        auth_mode: "api_key",
      },
      {
        id: 2,
        cli_key: "claude",
        name: "P1 副本",
        enabled: true,
        base_urls: ["https://b"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: {},
        auth_mode: "api_key",
      },
    ] as any[];

    vi.mocked(useProvidersListQuery).mockReturnValue({
      data: providers,
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
      refetch: vi.fn().mockResolvedValue({ data: [] }),
    } as any);
    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    renderWithQuery(<ProvidersView activeCli="claude" setActiveCli={vi.fn()} />);

    fireEvent.click(screen.getAllByRole("button", { name: "复制" })[0]!);

    await waitFor(() => expect(providerGetApiKey).toHaveBeenCalledWith(1));

    const createEditor = screen
      .getAllByTestId("provider-editor")
      .find((el) => el.textContent?.includes("create"));
    expect(createEditor).toBeTruthy();
    expect(createEditor).toHaveAttribute("data-initial-name", "P1 副本 2");
    expect(createEditor).toHaveAttribute("data-api-key", "sk-dup");
    expect(createEditor).toHaveAttribute("data-auth-mode", "api_key");
  });

  it("shows an explicit toast when duplicating a provider fails", async () => {
    vi.mocked(providerGetApiKey).mockRejectedValueOnce(new Error("boom"));

    const providers = [
      {
        id: 1,
        cli_key: "claude",
        name: "P1",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: {},
        auth_mode: "api_key",
      },
    ] as any[];

    vi.mocked(useProvidersListQuery).mockReturnValue({
      data: providers,
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
      refetch: vi.fn().mockResolvedValue({ data: [] }),
    } as any);
    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    renderWithQuery(<ProvidersView activeCli="claude" setActiveCli={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "复制" }));

    await waitFor(() => expect(toast).toHaveBeenCalledWith("复制失败：Error: boom"));
    expect(screen.queryByTestId("provider-editor")).not.toBeInTheDocument();
  });

  it("shows generate error when launch command mutation fails", async () => {
    vi.mocked(toast).mockClear();

    const providers = [
      {
        id: 1,
        cli_key: "claude",
        name: "P1",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: {},
      },
    ] as any[];

    vi.mocked(useProvidersListQuery).mockReturnValue({
      data: providers,
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
      refetch: vi.fn().mockResolvedValue({ data: [] }),
    } as any);

    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderClaudeTerminalLaunchCommandMutation).mockReturnValue({
      mutateAsync: vi.fn().mockRejectedValue(new Error("boom")),
    } as any);

    renderWithQuery(<ProvidersView activeCli="claude" setActiveCli={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "终端启动" }));

    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith(expect.stringContaining("生成启动命令失败"))
    );
  });

  it("shows PowerShell-specific toast when copied command targets Windows terminal", async () => {
    vi.mocked(toast).mockClear();

    const providers = [
      {
        id: 1,
        cli_key: "claude",
        name: "P1",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: {},
      },
    ] as any[];

    vi.mocked(useProvidersListQuery).mockReturnValue({
      data: providers,
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
      refetch: vi.fn().mockResolvedValue({ data: [] }),
    } as any);

    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderClaudeTerminalLaunchCommandMutation).mockReturnValue({
      mutateAsync: vi
        .fn()
        .mockResolvedValue(
          'powershell -NoLogo -NoExit -ExecutionPolicy Bypass -File "C:\\\\Temp\\\\aio.ps1"'
        ),
    } as any);

    renderWithQuery(<ProvidersView activeCli="claude" setActiveCli={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "终端启动" }));

    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith("已复制, 请在目标文件夹 PowerShell 粘贴执行")
    );
  });

  it("filters providers by name and restores the list after clearing search", async () => {
    const providers = [
      {
        id: 1,
        cli_key: "claude",
        name: "Alpha Relay",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: {},
        tags: ["prod"],
      },
      {
        id: 2,
        cli_key: "claude",
        name: "Beta Gateway",
        enabled: true,
        base_urls: ["https://b"],
        base_url_mode: "ping",
        cost_multiplier: 1,
        claude_models: {},
        tags: ["prod"],
      },
    ] as any[];

    vi.mocked(useProvidersListQuery).mockReturnValue({ data: providers, isFetching: false } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    renderWithQuery(<ProvidersView activeCli="claude" setActiveCli={vi.fn()} />);

    expect(screen.getByText("共 2 / 2 条")).toBeInTheDocument();

    const searchInput = screen.getByRole("textbox", { name: "搜索供应商名称" });
    fireEvent.change(searchInput, { target: { value: "beta" } });

    expect(screen.getByText("Beta Gateway")).toBeInTheDocument();
    expect(screen.queryByText("Alpha Relay")).not.toBeInTheDocument();
    expect(screen.getByText("共 1 / 2 条")).toBeInTheDocument();

    fireEvent.change(searchInput, { target: { value: "" } });

    expect(screen.getByText("Alpha Relay")).toBeInTheDocument();
    expect(screen.getByText("Beta Gateway")).toBeInTheDocument();
    expect(screen.getByText("共 2 / 2 条")).toBeInTheDocument();
  });

  it("always shows the 全部 tag even when providers have no custom tags", () => {
    const providers = [
      {
        id: 1,
        cli_key: "claude",
        name: "Alpha Relay",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: {},
        tags: [],
      },
    ] as any[];

    vi.mocked(useProvidersListQuery).mockReturnValue({ data: providers, isFetching: false } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    renderWithQuery(<ProvidersView activeCli="claude" setActiveCli={vi.fn()} />);

    expect(screen.getByRole("button", { name: "全部(1)" })).toBeInTheDocument();
  });

  it("refreshes the current providers list from the toolbar", async () => {
    const refetchClaudeProviders = vi.fn().mockResolvedValue({ data: [], error: null });
    const refetchCodexProviders = vi.fn().mockResolvedValue({ data: [], error: null });

    vi.mocked(useProvidersListQuery).mockImplementation((cliKey: any) => {
      if (cliKey === "codex") {
        return {
          data: [],
          isFetching: false,
          refetch: refetchCodexProviders,
        } as any;
      }

      return {
        data: [],
        isFetching: false,
        refetch: refetchClaudeProviders,
      } as any;
    });
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    renderWithQuery(<ProvidersView activeCli="claude" setActiveCli={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "刷新" }));

    await waitFor(() => expect(refetchClaudeProviders).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(refetchCodexProviders).toHaveBeenCalledTimes(1));
  });

  it("opens validate dialog and closes it when switching activeCli", async () => {
    vi.mocked(toast).mockClear();
    vi.mocked(logToConsole).mockClear();

    const providers = [
      {
        id: 1,
        cli_key: "claude",
        name: "P1",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: { main_model: "claude-3" },
      },
    ] as any[];

    vi.mocked(useProvidersListQuery).mockReturnValue({ data: providers, isFetching: false } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: vi.fn().mockResolvedValue({ data: [] }),
    } as any);

    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    const client = createTestQueryClient();
    const { rerender } = render(
      <QueryClientProvider client={client}>
        <ProvidersView activeCli="claude" setActiveCli={vi.fn()} />
      </QueryClientProvider>
    );

    fireEvent.pointerDown(screen.getByText("已启用"));
    fireEvent.click(screen.getByTitle("模型验证"));
    expect(screen.getByText("validate")).toBeInTheDocument();

    rerender(
      <QueryClientProvider client={client}>
        <ProvidersView activeCli="codex" setActiveCli={vi.fn()} />
      </QueryClientProvider>
    );

    await waitFor(() => expect(screen.queryByText("validate")).not.toBeInTheDocument());

    rerender(
      <QueryClientProvider client={client}>
        <ProvidersView activeCli="claude" setActiveCli={vi.fn()} />
      </QueryClientProvider>
    );
    fireEvent.click(screen.getByTitle("模型验证"));
    expect(screen.getByText("validate")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "close-validate" }));
    await waitFor(() => expect(screen.queryByText("validate")).not.toBeInTheDocument());
  });

  it("covers dialog onOpenChange/onSaved callbacks and delete dialog close gating", async () => {
    const providers = [
      {
        id: 1,
        cli_key: "claude",
        name: "P1",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: {},
      },
    ] as any[];

    vi.mocked(useProvidersListQuery).mockReturnValue({ data: providers, isFetching: false } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: vi.fn().mockResolvedValue({ data: [] }),
    } as any);

    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    let resolveDelete: (v: boolean) => void = () => {
      throw new Error("resolveDelete not set");
    };
    const deleteMutation = {
      mutateAsync: vi.fn().mockImplementation(
        () =>
          new Promise<boolean>((resolve) => {
            resolveDelete = resolve;
          })
      ),
    };
    vi.mocked(useProviderDeleteMutation).mockReturnValue(deleteMutation as any);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const setActiveCli = vi.fn();

    render(
      <QueryClientProvider client={client}>
        <ProvidersView activeCli="claude" setActiveCli={setActiveCli} />
      </QueryClientProvider>
    );
    expect(setActiveCli).not.toHaveBeenCalled();

    // create dialog onSaved + onOpenChange
    fireEvent.click(screen.getByRole("button", { name: "添加" }));
    const createEditor = screen
      .getAllByTestId("provider-editor")
      .find((el) => el.textContent?.includes("create"));
    expect(createEditor).toBeTruthy();
    fireEvent.click(within(createEditor as HTMLElement).getByRole("button", { name: "saved" }));
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: providersKeys.list("claude") });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: gatewayKeys.circuitStatus("claude") });

    fireEvent.click(
      within(createEditor as HTMLElement).getByRole("button", { name: "close-editor" })
    );
    await waitFor(() => expect(screen.queryByTestId("provider-editor")).not.toBeInTheDocument());

    // edit dialog onSaved + onOpenChange
    fireEvent.click(screen.getByTitle("编辑"));
    const editEditor = screen
      .getAllByTestId("provider-editor")
      .find((el) => el.textContent?.includes("edit"));
    expect(editEditor).toBeTruthy();
    fireEvent.click(within(editEditor as HTMLElement).getByRole("button", { name: "saved" }));
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: providersKeys.list("claude") });

    fireEvent.click(
      within(editEditor as HTMLElement).getByRole("button", { name: "close-editor" })
    );
    await waitFor(() => expect(screen.queryByTestId("provider-editor")).not.toBeInTheDocument());

    // delete dialog close gating while deleting
    fireEvent.click(screen.getByTitle("删除"));
    fireEvent.click(screen.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(deleteMutation.mutateAsync).toHaveBeenCalled());
    // try close via overlay while deleting -> should stay open
    fireEvent.click(document.querySelector(".bg-black\\/30") as HTMLElement);
    expect(screen.getByRole("dialog")).toBeInTheDocument();

    resolveDelete(true);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

    // delete dialog cancel button
    fireEvent.click(screen.getByTitle("删除"));
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("covers providers loading and empty branches", () => {
    vi.mocked(useProvidersListQuery).mockReturnValue({ data: [], isFetching: true } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    const { rerender } = renderWithQuery(
      <ProvidersView activeCli="claude" setActiveCli={vi.fn()} />
    );
    expect(screen.getByText("加载中…")).toBeInTheDocument();

    vi.mocked(useProvidersListQuery).mockReturnValue({ data: [], isFetching: false } as any);
    rerender(
      <QueryClientProvider client={createTestQueryClient()}>
        <ProvidersView activeCli="claude" setActiveCli={vi.fn()} />
      </QueryClientProvider>
    );
    expect(screen.getByText("暂无 Provider")).toBeInTheDocument();
  });

  it("covers mutation null/error branches and drag end edge cases", async () => {
    const providers = [
      {
        id: 1,
        cli_key: "claude",
        name: "P1",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "ping",
        cost_multiplier: 1,
        claude_models: {},
      },
      {
        id: 2,
        cli_key: "claude",
        name: "P2",
        enabled: false,
        base_urls: ["https://b"],
        base_url_mode: "order",
        cost_multiplier: 2,
        claude_models: {},
      },
    ] as any[];

    vi.mocked(useProvidersListQuery).mockReturnValue({ data: providers, isFetching: false } as any);

    const refetchCircuits = vi.fn().mockResolvedValue({ data: [] });
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [
        // OPEN with no open_until/cooldown_until => until=null branch (auto refresh immediately)
        { provider_id: 1, state: "OPEN", open_until: null, cooldown_until: null },
        { provider_id: 2, state: "OPEN", open_until: null, cooldown_until: null },
      ],
      isFetching: false,
      refetch: refetchCircuits,
    } as any);

    const toggleMutation = { mutateAsync: vi.fn() };
    toggleMutation.mutateAsync.mockResolvedValueOnce(null).mockRejectedValueOnce(new Error("boom"));
    vi.mocked(useProviderSetEnabledMutation).mockReturnValue(toggleMutation as any);

    const resetProviderMutation = { mutateAsync: vi.fn() };
    resetProviderMutation.mutateAsync
      .mockResolvedValueOnce(false)
      .mockRejectedValueOnce(new Error("boom"));
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue(resetProviderMutation as any);

    const resetCliMutation = { mutateAsync: vi.fn() };
    resetCliMutation.mutateAsync
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce(0)
      .mockRejectedValueOnce(new Error("boom"));
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue(resetCliMutation as any);

    const deleteMutation = { mutateAsync: vi.fn() };
    deleteMutation.mutateAsync.mockResolvedValueOnce(true).mockRejectedValueOnce(new Error("boom"));
    vi.mocked(useProviderDeleteMutation).mockReturnValue(deleteMutation as any);

    const reorderMutation = { mutateAsync: vi.fn() };
    reorderMutation.mutateAsync
      .mockResolvedValueOnce(null)
      .mockRejectedValueOnce(new Error("boom"));
    vi.mocked(useProvidersReorderMutation).mockReturnValue(reorderMutation as any);

    renderWithQuery(<ProvidersView activeCli="claude" setActiveCli={vi.fn()} />);

    // toggle enabled: null + error branches
    fireEvent.click(screen.getAllByRole("switch")[1]!);
    fireEvent.click(screen.getAllByRole("switch")[1]!);
    await Promise.resolve();
    await Promise.resolve();
    expect(vi.mocked(toast)).toHaveBeenCalled();

    // reset circuit provider: ok false + error branches
    fireEvent.click(screen.getAllByRole("button", { name: "解除熔断" })[0]!);
    fireEvent.click(screen.getAllByRole("button", { name: "解除熔断" })[0]!);
    await Promise.resolve();
    await Promise.resolve();
    expect(vi.mocked(toast)).toHaveBeenCalled();

    // reset circuit all: null + 0 + error branches
    fireEvent.click(screen.getByRole("button", { name: "解除熔断（全部）" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "解除熔断（全部）" })).toBeEnabled()
    );
    fireEvent.click(screen.getByRole("button", { name: "解除熔断（全部）" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "解除熔断（全部）" })).toBeEnabled()
    );
    fireEvent.click(screen.getByRole("button", { name: "解除熔断（全部）" }));
    await waitFor(() => expect(resetCliMutation.mutateAsync).toHaveBeenCalledTimes(3));

    // delete: success + error branches
    fireEvent.click(screen.getAllByTitle("删除")[0]!);
    fireEvent.click(screen.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(deleteMutation.mutateAsync).toHaveBeenCalledTimes(1));

    // re-open delete dialog for error branch
    fireEvent.click(screen.getAllByTitle("删除")[0]!);
    fireEvent.click(screen.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(deleteMutation.mutateAsync).toHaveBeenCalledTimes(2));

    // drag end edge cases
    latestOnDragEnd?.({ active: { id: 1 }, over: null });
    latestOnDragEnd?.({ active: { id: 1 }, over: { id: 1 } });
    latestOnDragEnd?.({ active: { id: 999 }, over: { id: 2 } });
    latestOnDragEnd?.({ active: { id: 1 }, over: { id: 2 } });
    latestOnDragEnd?.({ active: { id: 1 }, over: { id: 2 } });
    await Promise.resolve();
    await Promise.resolve();
    expect(reorderMutation.mutateAsync).toHaveBeenCalled();

    // circuit auto refresh (until=null -> now)
    await waitFor(() => expect(refetchCircuits).toHaveBeenCalled(), { timeout: 1000 });
  });

  it("renders unavailable countdown, Claude models badge, and dragging class", () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);
    sortableIsDragging = true;

    const providers = [
      {
        id: 1,
        cli_key: "claude",
        name: "P1",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: {
          main_model: " claude-3 ",
          ignored: "   ",
          non_string: 123,
        },
      },
    ] as any[];

    vi.mocked(useProvidersListQuery).mockReturnValue({ data: providers, isFetching: false } as any);

    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [
        {
          provider_id: 1,
          state: "OPEN",
          open_until: Math.floor(Date.now() / 1000) + 10,
          cooldown_until: null,
        },
      ],
      isFetching: false,
      refetch: vi.fn().mockResolvedValue({ data: [] }),
    } as any);

    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    renderWithQuery(<ProvidersView activeCli="claude" setActiveCli={vi.fn()} />);

    expect(screen.getByText("模型映射 1/5")).toBeInTheDocument();
    expect(screen.getByText(/^熔断\s*00:10$/)).toBeInTheDocument();
    expect(screen.getByText("P1").closest(".shadow-lg")).toBeTruthy();
  });

  it("clears circuit auto-refresh timer when circuits recover", () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);

    const providers = [
      {
        id: 1,
        cli_key: "claude",
        name: "P1",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: {},
      },
    ] as any[];

    vi.mocked(useProvidersListQuery).mockReturnValue({ data: providers, isFetching: false } as any);

    const refetchCircuits = vi.fn().mockResolvedValue({ data: [] });
    let circuits: any[] = [
      {
        provider_id: 1,
        state: "OPEN",
        open_until: Math.floor(Date.now() / 1000) + 60,
        cooldown_until: null,
      },
    ];
    vi.mocked(useGatewayCircuitStatusQuery).mockImplementation(() => {
      return { data: circuits, isFetching: false, refetch: refetchCircuits } as any;
    });

    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    const client = createTestQueryClient();
    const { rerender } = render(
      <QueryClientProvider client={client}>
        <ProvidersView activeCli="claude" setActiveCli={vi.fn()} />
      </QueryClientProvider>
    );

    circuits = [];
    rerender(
      <QueryClientProvider client={client}>
        <ProvidersView activeCli="claude" setActiveCli={vi.fn()} />
      </QueryClientProvider>
    );

    vi.advanceTimersByTime(90_000);
    expect(refetchCircuits).not.toHaveBeenCalled();
  });

  it("closes delete dialog via overlay when not deleting", async () => {
    const providers = [
      {
        id: 1,
        cli_key: "claude",
        name: "P1",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: {},
      },
    ] as any[];

    vi.mocked(useProvidersListQuery).mockReturnValue({ data: providers, isFetching: false } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue(true),
    } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    renderWithQuery(<ProvidersView activeCli="claude" setActiveCli={vi.fn()} />);

    fireEvent.click(screen.getByTitle("删除"));
    expect(screen.getByRole("dialog")).toBeInTheDocument();

    fireEvent.click(document.querySelector(".bg-black\\/30") as HTMLElement);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("shows circuit-loading label for reset-all button", () => {
    const providers = [
      {
        id: 1,
        cli_key: "claude",
        name: "P1",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: {},
      },
    ] as any[];

    vi.mocked(useProvidersListQuery).mockReturnValue({ data: providers, isFetching: false } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [{ provider_id: 1, state: "OPEN", open_until: null, cooldown_until: null }],
      isFetching: true,
      refetch: vi.fn().mockResolvedValue({ data: [] }),
    } as any);

    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProvidersReorderMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    renderWithQuery(<ProvidersView activeCli="claude" setActiveCli={vi.fn()} />);
    expect(screen.getByRole("button", { name: "熔断加载中…" })).toBeInTheDocument();
  });

  it("skips reorder side effects when cli switches before mutation resolves", async () => {
    vi.mocked(toast).mockClear();
    vi.mocked(logToConsole).mockClear();

    const providers = [
      {
        id: 1,
        cli_key: "claude",
        name: "P1",
        enabled: true,
        base_urls: ["https://a"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: {},
      },
      {
        id: 2,
        cli_key: "claude",
        name: "P2",
        enabled: false,
        base_urls: ["https://b"],
        base_url_mode: "order",
        cost_multiplier: 1,
        claude_models: {},
      },
    ] as any[];

    vi.mocked(useProvidersListQuery).mockReturnValue({ data: providers, isFetching: false } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    let resolveReorder: (rows: any) => void = () => {
      throw new Error("resolveReorder not set");
    };
    const reorderPromise = new Promise<any>((resolve) => {
      resolveReorder = resolve;
    });
    const reorderMutation = { mutateAsync: vi.fn().mockReturnValue(reorderPromise) };
    vi.mocked(useProvidersReorderMutation).mockReturnValue(reorderMutation as any);

    vi.mocked(useProviderSetEnabledMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useProviderDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitResetCliMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    const client = createTestQueryClient();
    const { rerender } = render(
      <QueryClientProvider client={client}>
        <ProvidersView activeCli="claude" setActiveCli={vi.fn()} />
      </QueryClientProvider>
    );

    latestOnDragEnd?.({ active: { id: 1 }, over: { id: 2 } });
    await waitFor(() => expect(reorderMutation.mutateAsync).toHaveBeenCalled());

    rerender(
      <QueryClientProvider client={client}>
        <ProvidersView activeCli="codex" setActiveCli={vi.fn()} />
      </QueryClientProvider>
    );
    await Promise.resolve();

    resolveReorder([providers[1], providers[0]]);
    await Promise.resolve();
    await Promise.resolve();

    expect(vi.mocked(toast)).not.toHaveBeenCalledWith("顺序已更新");
  });
});
