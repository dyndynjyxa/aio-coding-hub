import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { HomeWorkspaceConfigPanel } from "../HomeWorkspaceConfigPanel";
import type { HomeCliWorkspaceConfig } from "../homeWorkspaceConfigTypes";

const configs: HomeCliWorkspaceConfig[] = [
  {
    cliKey: "claude",
    cliLabel: "Claude Code",
    workspaceId: 1,
    workspaceName: "工作区 A",
    loading: false,
    items: [
      { id: "prompt:1", type: "prompts", label: "Prompt", name: "默认提示词" },
      { id: "mcp:1", type: "mcp", label: "MCP", name: "filesystem" },
    ],
  },
  {
    cliKey: "codex",
    cliLabel: "Codex",
    workspaceId: 2,
    workspaceName: "Default",
    loading: false,
    items: [{ id: "skill:1", type: "skills", label: "Skill", name: "code-review" }],
  },
];

describe("components/home/HomeWorkspaceConfigPanel", () => {
  it("renders workspace info and items without a built-in route strategy control", () => {
    render(
      <HomeWorkspaceConfigPanel
        configs={configs}
        selectedCliKey="claude"
        onSelectCliKey={vi.fn()}
      />
    );

    expect(screen.getByText("工作区：")).toBeInTheDocument();
    expect(screen.getByText("工作区 A")).toBeInTheDocument();
    expect(screen.getByText("默认提示词")).toBeInTheDocument();
    expect(screen.getByText("filesystem")).toBeInTheDocument();
    expect(screen.queryByText("路由策略：")).not.toBeInTheDocument();
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
  });

  it("renders the optional header addon and keeps cli switching available", () => {
    const onSelectCliKey = vi.fn();

    render(
      <HomeWorkspaceConfigPanel
        configs={configs}
        selectedCliKey="claude"
        onSelectCliKey={onSelectCliKey}
        headerAddon={<div>route-addon</div>}
      />
    );

    expect(screen.getByText("route-addon")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Codex" }));
    expect(onSelectCliKey).toHaveBeenCalledWith("codex");
  });
});
