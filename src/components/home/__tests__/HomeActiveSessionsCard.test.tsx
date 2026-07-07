import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { HomeActiveSessionsCard } from "../HomeActiveSessionsCard";
import { createQueryWrapper, createTestQueryClient } from "../../../test/utils/reactQuery";

// Mutable metadata so per-test cases can supply cwd/title.
const state = vi.hoisted(() => ({ metadata: new Map<string, any>() }));

vi.mock("../../../query/sortModes", () => ({
  useSortModesListQuery: () => ({
    data: [
      { id: 3, name: "Cheap-first", created_at: 1, updated_at: 1 },
      { id: 7, name: "Fast-first", created_at: 2, updated_at: 2 },
    ],
  }),
}));

vi.mock("../../../query/cliSessions", () => ({
  useCliSessionsMetadataLookupByIdsQuery: () => ({ data: state.metadata, isLoading: false }),
}));

vi.mock("../../../query/gateway", () => ({
  useGatewaySessionPinSortModeMutation: () => ({ mutate: vi.fn(), isPending: false }),
  useGatewaySessionUnpinSortModeMutation: () => ({ mutate: vi.fn(), isPending: false }),
  useGatewaySessionPersistSortModeMutation: () => ({ mutate: vi.fn(), isPending: false }),
  useGatewaySessionUnpersistSortModeMutation: () => ({ mutate: vi.fn(), isPending: false }),
}));

function renderCard(ui: React.ReactElement) {
  return render(ui, { wrapper: createQueryWrapper(createTestQueryClient()) });
}

function session(idx: number, overrides: Partial<any> = {}) {
  return {
    cli_key: idx % 2 === 0 ? "claude" : "codex",
    session_id: `s-${idx}`,
    session_suffix: String(idx).padStart(4, "0"),
    provider_id: 1,
    provider_name: idx === 9 ? "Unknown" : `P${idx}`,
    sort_mode_pinned: false,
    pinned_sort_mode_id: null,
    persistent_pinned: false,
    persistent_pinned_sort_mode_id: null,
    expires_at: 10_000 + idx * 100,
    request_count: idx + 1,
    total_input_tokens: 100 + idx,
    total_output_tokens: 200 + idx,
    total_cost_usd: 0.000001 * idx,
    total_duration_ms: 1000 + idx,
    ...overrides,
  };
}

describe("components/home/HomeActiveSessionsCard", () => {
  it("renders loading/unavailable/empty states", () => {
    renderCard(
      <HomeActiveSessionsCard
        activeSessions={[]}
        activeSessionsLoading={true}
        activeSessionsAvailable={null}
      />
    );
    expect(screen.getByText("加载中…")).toBeInTheDocument();

    renderCard(
      <HomeActiveSessionsCard
        activeSessions={[]}
        activeSessionsLoading={false}
        activeSessionsAvailable={false}
      />
    );
    expect(screen.getByText("数据不可用")).toBeInTheDocument();

    renderCard(
      <HomeActiveSessionsCard
        activeSessions={[]}
        activeSessionsLoading={false}
        activeSessionsAvailable={true}
      />
    );
    expect(screen.getByText("暂无活跃 Session。")).toBeInTheDocument();
  });

  it("renders full list with per-session routing-template dropdowns", () => {
    const sessions = Array.from({ length: 10 }, (_, idx) => session(idx));
    renderCard(
      <HomeActiveSessionsCard
        activeSessions={sessions as any}
        activeSessionsLoading={false}
        activeSessionsAvailable={true}
      />
    );

    expect(screen.getByText("活跃 Session")).toBeInTheDocument();
    expect(screen.queryByText("+2 个")).not.toBeInTheDocument();
    expect(screen.getByText("0000")).toBeInTheDocument();
    expect(screen.getByText("$0.000000")).toBeInTheDocument();
    // One sort_mode dropdown + one pin button per row.
    expect(screen.getAllByLabelText("路由策略")).toHaveLength(10);
    expect(screen.getAllByLabelText("pin 状态：未 pin")).toHaveLength(10);
  });

  it("shows the '本次' (ephemeral) pin tier when a session has sort_mode_pinned", () => {
    renderCard(
      <HomeActiveSessionsCard
        activeSessions={[session(0, { sort_mode_pinned: true, pinned_sort_mode_id: 3 })] as any}
        activeSessionsLoading={false}
        activeSessionsAvailable={true}
      />
    );
    expect(screen.getByLabelText("pin 状态：本次")).toBeInTheDocument();
    expect(screen.getByLabelText("路由策略")).toHaveValue("3");
  });

  it("shows the '持久' (persistent) pin tier when a session has persistent_pinned", () => {
    renderCard(
      <HomeActiveSessionsCard
        activeSessions={
          [session(0, { persistent_pinned: true, persistent_pinned_sort_mode_id: 7 })] as any
        }
        activeSessionsLoading={false}
        activeSessionsAvailable={true}
      />
    );
    expect(screen.getByLabelText("pin 状态：持久")).toBeInTheDocument();
    expect(screen.getByLabelText("路由策略")).toHaveValue("7");
  });

  it("renders metadata line (目录 / 会话) when session metadata resolves cwd + title", () => {
    state.metadata.set("claude:s-0", {
      source: "claude",
      session_id: "s-0",
      cwd: "/home/me/myproject",
      title: "Project Title",
    });
    renderCard(
      <HomeActiveSessionsCard
        activeSessions={[session(0)] as any}
        activeSessionsLoading={false}
        activeSessionsAvailable={true}
      />
    );
    expect(screen.getByText("myproject")).toBeInTheDocument();
    expect(screen.getByText("Project Title")).toBeInTheDocument();
  });

  it("renders folder-only metadata when cwd present but title empty", () => {
    state.metadata.set("claude:s-0", {
      source: "claude",
      session_id: "s-0",
      cwd: "/home/me/another",
      title: "",
    });
    renderCard(
      <HomeActiveSessionsCard
        activeSessions={[session(0)] as any}
        activeSessionsLoading={false}
        activeSessionsAvailable={true}
      />
    );
    expect(screen.getByText("another")).toBeInTheDocument();
    expect(screen.queryByText("会话")).not.toBeInTheDocument();
  });

  it("omits metadata line when cli_key is gemini (no session source)", () => {
    renderCard(
      <HomeActiveSessionsCard
        activeSessions={[session(0, { cli_key: "gemini" })] as any}
        activeSessionsLoading={false}
        activeSessionsAvailable={true}
      />
    );
    // gemini has no cli_sessions source → no metadata lookup, no 目录/会话 line.
    expect(screen.queryByText("目录")).not.toBeInTheDocument();
    expect(screen.queryByText("会话")).not.toBeInTheDocument();
  });
});
