import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useMcpServersListQuery } from "../../../../query/mcp";
import { usePromptsListQuery } from "../../../../query/prompts";
import { useSkillsInstalledListQuery } from "../../../../query/skills";
import { useWorkspacesListQuery } from "../../../../query/workspaces";
import type { CliKey } from "../../../../services/providers/providers";
import { useHomeWorkspaceConfigs } from "../useHomeWorkspaceConfigs";

vi.mock("../../../../query/mcp", async () => {
  const actual =
    await vi.importActual<typeof import("../../../../query/mcp")>("../../../../query/mcp");
  return { ...actual, useMcpServersListQuery: vi.fn() };
});

vi.mock("../../../../query/prompts", async () => {
  const actual = await vi.importActual<typeof import("../../../../query/prompts")>(
    "../../../../query/prompts"
  );
  return { ...actual, usePromptsListQuery: vi.fn() };
});

vi.mock("../../../../query/skills", async () => {
  const actual = await vi.importActual<typeof import("../../../../query/skills")>(
    "../../../../query/skills"
  );
  return { ...actual, useSkillsInstalledListQuery: vi.fn() };
});

vi.mock("../../../../query/workspaces", async () => {
  const actual = await vi.importActual<typeof import("../../../../query/workspaces")>(
    "../../../../query/workspaces"
  );
  return { ...actual, useWorkspacesListQuery: vi.fn() };
});

type QueryState<T> = {
  data: T;
  isLoading: boolean;
};

function asMockedQueryResult<T>(state: QueryState<T>) {
  return state as any;
}

const queryState = vi.hoisted(() => ({
  workspaces: {
    claude: { data: null, isLoading: false } as QueryState<any>,
    codex: { data: null, isLoading: false } as QueryState<any>,
    gemini: { data: null, isLoading: false } as QueryState<any>,
  },
  prompts: new Map<number | null, QueryState<any>>(),
  mcp: new Map<number | null, QueryState<any>>(),
  skills: new Map<number | null, QueryState<any>>(),
}));

function setQueryState(input?: {
  workspaces?: Partial<typeof queryState.workspaces>;
  prompts?: Array<[number | null, QueryState<any>]>;
  mcp?: Array<[number | null, QueryState<any>]>;
  skills?: Array<[number | null, QueryState<any>]>;
}) {
  queryState.workspaces.claude = input?.workspaces?.claude ?? { data: null, isLoading: false };
  queryState.workspaces.codex = input?.workspaces?.codex ?? { data: null, isLoading: false };
  queryState.workspaces.gemini = input?.workspaces?.gemini ?? { data: null, isLoading: false };
  queryState.prompts = new Map(input?.prompts ?? []);
  queryState.mcp = new Map(input?.mcp ?? []);
  queryState.skills = new Map(input?.skills ?? []);
}

beforeEach(() => {
  vi.clearAllMocks();
  setQueryState();

  vi.mocked(useWorkspacesListQuery).mockImplementation(((cliKey: CliKey) =>
    asMockedQueryResult(queryState.workspaces[cliKey])) as typeof useWorkspacesListQuery);
  vi.mocked(usePromptsListQuery).mockImplementation(((workspaceId: number | null) =>
    asMockedQueryResult(
      queryState.prompts.get(workspaceId) ?? { data: null, isLoading: false }
    )) as typeof usePromptsListQuery);
  vi.mocked(useMcpServersListQuery).mockImplementation(((workspaceId: number | null) =>
    asMockedQueryResult(
      queryState.mcp.get(workspaceId) ?? { data: null, isLoading: false }
    )) as typeof useMcpServersListQuery);
  vi.mocked(useSkillsInstalledListQuery).mockImplementation(((workspaceId: number | null) =>
    asMockedQueryResult(
      queryState.skills.get(workspaceId) ?? { data: null, isLoading: false }
    )) as typeof useSkillsInstalledListQuery);
});

describe("pages/home/hooks/useHomeWorkspaceConfigs", () => {
  it("builds sorted workspace config items, filters disabled rows, and derives loading state", () => {
    setQueryState({
      workspaces: {
        claude: {
          data: {
            active_id: 2,
            items: [
              { id: 1, name: "Other Workspace" },
              { id: 2, name: "Claude Workspace" },
            ],
          },
          isLoading: false,
        },
        codex: {
          data: {
            active_id: 9,
            items: [{ id: 9, name: "Codex Workspace" }],
          },
          isLoading: false,
        },
        gemini: {
          data: {
            active_id: 77,
            items: [],
          },
          isLoading: false,
        },
      },
      prompts: [
        [
          2,
          {
            data: [
              { id: 2, name: "Z Prompt" },
              { id: 1, name: "A Prompt" },
            ],
            isLoading: true,
          },
        ],
        [9, { data: null, isLoading: false }],
        [77, { data: [{ id: 7, name: "Gemini Prompt" }], isLoading: false }],
      ],
      mcp: [
        [
          2,
          {
            data: [
              { id: 11, name: "B MCP", enabled: true },
              { id: 12, name: "A MCP", enabled: true },
              { id: 13, name: "Disabled MCP", enabled: false },
            ],
            isLoading: false,
          },
        ],
        [9, { data: [], isLoading: false }],
        [77, { data: [{ id: 70, name: "Gemini MCP", enabled: false }], isLoading: false }],
      ],
      skills: [
        [
          2,
          {
            data: [
              { id: 21, name: "B Skill", enabled: true },
              { id: 22, name: "A Skill", enabled: true },
              { id: 23, name: "Disabled Skill", enabled: false },
            ],
            isLoading: false,
          },
        ],
        [9, { data: [], isLoading: false }],
        [77, { data: [{ id: 71, name: "Gemini Skill", enabled: true }], isLoading: false }],
      ],
    });

    const { result } = renderHook(() => useHomeWorkspaceConfigs());

    expect(result.current).toEqual([
      {
        cliKey: "claude",
        cliLabel: "Claude Code",
        workspaceId: 2,
        workspaceName: "Claude Workspace",
        loading: true,
        items: [
          { id: "prompt:1", type: "prompts", label: "Prompt", name: "A Prompt" },
          { id: "prompt:2", type: "prompts", label: "Prompt", name: "Z Prompt" },
          { id: "mcp:12", type: "mcp", label: "MCP", name: "A MCP" },
          { id: "mcp:11", type: "mcp", label: "MCP", name: "B MCP" },
          { id: "skill:22", type: "skills", label: "Skill", name: "A Skill" },
          { id: "skill:21", type: "skills", label: "Skill", name: "B Skill" },
        ],
      },
      {
        cliKey: "codex",
        cliLabel: "Codex",
        workspaceId: 9,
        workspaceName: "Codex Workspace",
        loading: false,
        items: [],
      },
      {
        cliKey: "gemini",
        cliLabel: "Gemini",
        workspaceId: 77,
        workspaceName: null,
        loading: false,
        items: [
          { id: "prompt:7", type: "prompts", label: "Prompt", name: "Gemini Prompt" },
          { id: "skill:71", type: "skills", label: "Skill", name: "Gemini Skill" },
        ],
      },
    ]);
  });

  it("passes enabled false to downstream queries and suppresses loading even when queries are busy", () => {
    setQueryState({
      workspaces: {
        claude: {
          data: { active_id: null, items: [] },
          isLoading: true,
        },
        codex: {
          data: { active_id: 3, items: [{ id: 3, name: "Codex WS" }] },
          isLoading: true,
        },
        gemini: {
          data: null,
          isLoading: true,
        },
      },
      prompts: [
        [null, { data: null, isLoading: true }],
        [3, { data: [{ id: 30, name: "Codex Prompt" }], isLoading: true }],
      ],
      mcp: [
        [null, { data: null, isLoading: true }],
        [3, { data: [{ id: 31, name: "Codex MCP", enabled: true }], isLoading: true }],
      ],
      skills: [
        [null, { data: null, isLoading: true }],
        [3, { data: [{ id: 32, name: "Codex Skill", enabled: true }], isLoading: true }],
      ],
    });

    const { result } = renderHook(() => useHomeWorkspaceConfigs({ enabled: false }));

    expect(result.current.map((item) => item.loading)).toEqual([false, false, false]);
    expect(result.current[0]).toMatchObject({
      workspaceId: null,
      workspaceName: null,
      items: [],
    });
    expect(result.current[1]).toMatchObject({
      workspaceId: 3,
      workspaceName: "Codex WS",
      items: [
        { id: "prompt:30", type: "prompts", label: "Prompt", name: "Codex Prompt" },
        { id: "mcp:31", type: "mcp", label: "MCP", name: "Codex MCP" },
        { id: "skill:32", type: "skills", label: "Skill", name: "Codex Skill" },
      ],
    });

    expect(useWorkspacesListQuery).toHaveBeenNthCalledWith(1, "claude", { enabled: false });
    expect(useWorkspacesListQuery).toHaveBeenNthCalledWith(2, "codex", { enabled: false });
    expect(useWorkspacesListQuery).toHaveBeenNthCalledWith(3, "gemini", { enabled: false });

    expect(usePromptsListQuery).toHaveBeenNthCalledWith(1, null, { enabled: false });
    expect(usePromptsListQuery).toHaveBeenNthCalledWith(2, 3, { enabled: false });
    expect(usePromptsListQuery).toHaveBeenNthCalledWith(3, null, { enabled: false });

    expect(useMcpServersListQuery).toHaveBeenNthCalledWith(1, null, { enabled: false });
    expect(useMcpServersListQuery).toHaveBeenNthCalledWith(2, 3, { enabled: false });
    expect(useMcpServersListQuery).toHaveBeenNthCalledWith(3, null, { enabled: false });

    expect(useSkillsInstalledListQuery).toHaveBeenNthCalledWith(1, null, { enabled: false });
    expect(useSkillsInstalledListQuery).toHaveBeenNthCalledWith(2, 3, { enabled: false });
    expect(useSkillsInstalledListQuery).toHaveBeenNthCalledWith(3, null, { enabled: false });
  });
});
