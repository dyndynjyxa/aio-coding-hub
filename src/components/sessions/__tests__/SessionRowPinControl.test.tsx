import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { SessionRowPinControl } from "../SessionRowPinControl";
import { createQueryWrapper, createTestQueryClient } from "../../../test/utils/reactQuery";

// vitest hoists vi.mock factories; use vi.hoisted so the factory reads live data.
const state = vi.hoisted(() => ({
  sessions: [] as any[],
  pins: [] as any[],
  pin: vi.fn(),
  unpin: vi.fn(),
  persist: vi.fn(),
  unpersist: vi.fn(),
}));

vi.mock("../../../query/gateway", () => ({
  useGatewaySessionsListQuery: () => ({ data: state.sessions, isLoading: false }),
  useGatewayPersistentPinsListQuery: () => ({ data: state.pins, isLoading: false }),
  useGatewaySessionPinSortModeMutation: () => ({ mutate: state.pin, isPending: false }),
  useGatewaySessionUnpinSortModeMutation: () => ({ mutate: state.unpin, isPending: false }),
  useGatewaySessionPersistSortModeMutation: () => ({ mutate: state.persist, isPending: false }),
  useGatewaySessionUnpersistSortModeMutation: () => ({ mutate: state.unpersist, isPending: false }),
}));

vi.mock("../../../query/sortModes", () => ({
  useSortModesListQuery: () => ({
    data: [
      { id: 3, name: "Cheap-first", created_at: 1, updated_at: 1 },
      { id: 7, name: "Fast-first", created_at: 2, updated_at: 2 },
    ],
  }),
}));

function renderRow(cliKey = "claude", sessionId = "s1") {
  return render(<SessionRowPinControl cliKey={cliKey} sessionId={sessionId} />, {
    wrapper: createQueryWrapper(createTestQueryClient()),
  });
}

describe("components/sessions/SessionRowPinControl", () => {
  beforeEach(() => {
    state.sessions = [];
    state.pins = [];
    state.pin = vi.fn();
    state.unpin = vi.fn();
    state.persist = vi.fn();
    state.unpersist = vi.fn();
  });

  it("resolves tier 'none' when neither ephemeral nor persistent pin exists", () => {
    renderRow();
    expect(screen.getByRole("combobox", { name: "路由策略" })).toHaveValue("auto");
    expect(screen.getByRole("button", { name: /pin 状态/ })).toHaveTextContent("未 pin");
  });

  it("resolves tier 'ephemeral' when session has sort_mode_pinned (overrides persistent)", () => {
    state.sessions = [
      {
        cli_key: "claude",
        session_id: "s1",
        sort_mode_pinned: true,
        pinned_sort_mode_id: 3,
        persistent_pinned: true,
        persistent_pinned_sort_mode_id: 7,
      },
    ];
    renderRow();
    expect(screen.getByRole("combobox", { name: "路由策略" })).toHaveValue("3");
    expect(screen.getByRole("button", { name: /pin 状态/ })).toHaveTextContent("本次");
  });

  it("resolves tier 'persistent' from the persistent list when session is not active", () => {
    state.pins = [
      { cli_key: "claude", session_id: "s1", sort_mode_id: 7, created_at: 1, updated_at: 1 },
    ];
    renderRow();
    expect(screen.getByRole("combobox", { name: "路由策略" })).toHaveValue("7");
    expect(screen.getByRole("button", { name: /pin 状态/ })).toHaveTextContent("持久");
  });

  it("resolves tier 'persistent' from session metadata when active session marks persistent_pinned", () => {
    state.sessions = [
      {
        cli_key: "claude",
        session_id: "s1",
        sort_mode_pinned: false,
        pinned_sort_mode_id: null,
        persistent_pinned: true,
        persistent_pinned_sort_mode_id: 7,
      },
    ];
    renderRow();
    expect(screen.getByRole("combobox", { name: "路由策略" })).toHaveValue("7");
    expect(screen.getByRole("button", { name: /pin 状态/ })).toHaveTextContent("持久");
  });
});
