import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { PersistentPinsCard } from "../PersistentPinsCard";
import { createQueryWrapper, createTestQueryClient } from "../../../test/utils/reactQuery";

// Mutable per-test state, read live by the mock factories via vi.hoisted.
const state = vi.hoisted(() => ({
  pins: [] as any[],
  loading: false,
  metadata: new Map<string, any>(),
  persist: vi.fn(),
  unpersist: vi.fn(),
}));

vi.mock("../../../query/gateway", () => ({
  useGatewayPersistentPinsListQuery: () => ({ data: state.pins, isLoading: state.loading }),
  useGatewaySessionPersistSortModeMutation: () => ({ mutate: state.persist, isPending: false }),
  useGatewaySessionUnpersistSortModeMutation: () => ({ mutate: state.unpersist, isPending: false }),
}));

vi.mock("../../../query/sortModes", () => ({
  useSortModesListQuery: () => ({ data: SORT_MODES, isLoading: false }),
}));

vi.mock("../../../query/cliSessions", () => ({
  useCliSessionsMetadataLookupByIdsQuery: () => ({ data: state.metadata }),
}));

const SORT_MODES = [
  { id: 3, name: "Cheap-first", created_at: 1, updated_at: 1 },
  { id: 7, name: "Fast-first", created_at: 2, updated_at: 2 },
];

function renderCard() {
  return render(<PersistentPinsCard />, {
    wrapper: createQueryWrapper(createTestQueryClient()),
  });
}

describe("pages/providers/PersistentPinsCard", () => {
  beforeEach(() => {
    state.persist.mockClear();
    state.unpersist.mockClear();
    state.pins = [];
    state.loading = false;
    state.metadata = new Map();
  });

  it("renders loading state", () => {
    state.loading = true;
    renderCard();
    expect(screen.getByText("加载持久指派…")).toBeInTheDocument();
  });

  it("renders empty state when no pins", () => {
    renderCard();
    expect(screen.getByText("暂无持久指派。")).toBeInTheDocument();
    expect(screen.getByText("0")).toBeInTheDocument();
  });

  it("renders pin count and rows", () => {
    state.pins = [
      {
        cli_key: "claude",
        session_id: "sess-aaaa1111",
        sort_mode_id: 3,
        created_at: 1,
        updated_at: 100,
      },
    ];
    renderCard();
    expect(screen.getByText("1")).toBeInTheDocument();
    expect(screen.getAllByRole("combobox", { name: "修改持久策略" })).toHaveLength(1);
  });

  it("shows 'Default' label when sort_mode_id is null", () => {
    state.pins = [
      {
        cli_key: "claude",
        session_id: "sess-aaaa1111",
        sort_mode_id: null,
        created_at: 1,
        updated_at: 100,
      },
    ];
    renderCard();
    expect(screen.getByTitle("持久策略：Default")).toHaveTextContent("Default");
  });

  it("shows the mode name when sort_mode_id is set", () => {
    state.pins = [
      {
        cli_key: "claude",
        session_id: "sess-aaaa1111",
        sort_mode_id: 7,
        created_at: 1,
        updated_at: 100,
      },
    ];
    renderCard();
    expect(screen.getByTitle("持久策略：Fast-first")).toHaveTextContent("Fast-first");
  });

  it("shows '#id' fallback when mode id is unknown", () => {
    state.pins = [
      {
        cli_key: "claude",
        session_id: "sess-aaaa1111",
        sort_mode_id: 999,
        created_at: 1,
        updated_at: 100,
      },
    ];
    renderCard();
    expect(screen.getByTitle("持久策略：#999")).toHaveTextContent("#999");
  });

  it("disables controls when cliKey is unsupported", () => {
    state.pins = [
      {
        cli_key: "bogus",
        session_id: "sess-aaaa1111",
        sort_mode_id: 3,
        created_at: 1,
        updated_at: 100,
      },
    ];
    renderCard();
    expect(screen.getByRole("combobox", { name: "修改持久策略" })).toBeDisabled();
  });

  it("changing the select to a mode id persists with that id", () => {
    state.pins = [
      {
        cli_key: "claude",
        session_id: "sess-aaaa1111",
        sort_mode_id: 3,
        created_at: 1,
        updated_at: 100,
      },
    ];
    renderCard();
    fireEvent.change(screen.getByRole("combobox", { name: "修改持久策略" }), {
      target: { value: "7" },
    });
    expect(state.persist).toHaveBeenCalledWith({
      cliKey: "claude",
      sessionId: "sess-aaaa1111",
      sortModeId: 7,
    });
  });

  it("changing the select to 'default' persists with null mode", () => {
    state.pins = [
      {
        cli_key: "codex",
        session_id: "sess-bbbb2222",
        sort_mode_id: 3,
        created_at: 1,
        updated_at: 100,
      },
    ];
    renderCard();
    fireEvent.change(screen.getByRole("combobox", { name: "修改持久策略" }), {
      target: { value: "default" },
    });
    expect(state.persist).toHaveBeenCalledWith({
      cliKey: "codex",
      sessionId: "sess-bbbb2222",
      sortModeId: null,
    });
  });

  it("re-selecting the current value is a no-op", () => {
    state.pins = [
      {
        cli_key: "claude",
        session_id: "sess-aaaa1111",
        sort_mode_id: 3,
        created_at: 1,
        updated_at: 100,
      },
    ];
    renderCard();
    fireEvent.change(screen.getByRole("combobox", { name: "修改持久策略" }), {
      target: { value: "3" },
    });
    expect(state.persist).not.toHaveBeenCalled();
  });

  it("clicking '解除持久 pin' unpersists", () => {
    state.pins = [
      {
        cli_key: "claude",
        session_id: "sess-aaaa1111",
        sort_mode_id: 3,
        created_at: 1,
        updated_at: 100,
      },
    ];
    renderCard();
    fireEvent.click(screen.getByRole("button", { name: "解除持久 pin" }));
    expect(state.unpersist).toHaveBeenCalledWith({
      cliKey: "claude",
      sessionId: "sess-aaaa1111",
    });
  });

  describe("row name resolution (cwd/title combinations)", () => {
    it("renders 'folder / title' when both cwd and title present", () => {
      state.pins = [
        {
          cli_key: "claude",
          session_id: "sess-aaaa1111",
          sort_mode_id: 3,
          created_at: 1,
          updated_at: 100,
        },
      ];
      state.metadata.set("claude:sess-aaaa1111", {
        source: "claude",
        session_id: "sess-aaaa1111",
        cwd: "/home/me/myproject",
        title: "Project Title",
      });
      renderCard();
      expect(screen.getByText("myproject / Project Title")).toBeInTheDocument();
    });

    it("renders folder basename only when title empty", () => {
      state.pins = [
        {
          cli_key: "claude",
          session_id: "sess-aaaa1111",
          sort_mode_id: 3,
          created_at: 1,
          updated_at: 100,
        },
      ];
      state.metadata.set("claude:sess-aaaa1111", {
        source: "claude",
        session_id: "sess-aaaa1111",
        cwd: "/home/me/another-project",
        title: "",
      });
      renderCard();
      expect(screen.getByText("another-project")).toBeInTheDocument();
    });

    it("renders title only when cwd is null", () => {
      state.pins = [
        {
          cli_key: "claude",
          session_id: "sess-aaaa1111",
          sort_mode_id: 3,
          created_at: 1,
          updated_at: 100,
        },
      ];
      state.metadata.set("claude:sess-aaaa1111", {
        source: "claude",
        session_id: "sess-aaaa1111",
        cwd: null,
        title: "Just A Title",
      });
      renderCard();
      expect(screen.getByText("Just A Title")).toBeInTheDocument();
    });

    it("falls back to last 8 chars of session_id when no cwd and no title", () => {
      state.pins = [
        {
          cli_key: "claude",
          session_id: "sess-aaaa1111",
          sort_mode_id: 3,
          created_at: 1,
          updated_at: 100,
        },
      ];
      // no metadata entry → no cwd, no title
      renderCard();
      expect(screen.getByText("aaaa1111")).toBeInTheDocument();
    });
  });
});
