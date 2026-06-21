import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { HomeWorkspaceConfigPanel } from "../HomeWorkspaceConfigPanel";
import type { HomeCliWorkspaceConfig } from "../homeWorkspaceConfigTypes";

const configs: HomeCliWorkspaceConfig[] = [
  {
    cliKey: "claude",
    cliLabel: "Claude",
    workspaceId: 1,
    workspaceName: "工作区 A",
    workspaces: [
      { id: 1, name: "工作区 A", isActive: true },
      { id: 3, name: "工作区 C", isActive: false },
    ],
    loading: false,
    items: [
      {
        id: "prompt:1",
        resourceId: 1,
        type: "prompts",
        label: "Prompt",
        name: "默认提示词",
        enabled: true,
      },
      {
        id: "mcp:1",
        resourceId: 1,
        type: "mcp",
        label: "MCP",
        name: "filesystem",
        enabled: false,
      },
    ],
  },
  {
    cliKey: "codex",
    cliLabel: "Codex",
    workspaceId: 2,
    workspaceName: "Default",
    workspaces: [{ id: 2, name: "Default", isActive: true }],
    loading: false,
    items: [
      {
        id: "skill:1",
        resourceId: 1,
        type: "skills",
        label: "Skill",
        name: "code-review",
        enabled: true,
      },
    ],
  },
];

describe("components/home/HomeWorkspaceConfigPanel", () => {
  it("renders workspace info and items without a built-in route strategy control", () => {
    render(
      <HomeWorkspaceConfigPanel
        configs={configs}
        selectedCliKey="claude"
        onSelectCliKey={vi.fn()}
        onSwitchWorkspace={vi.fn()}
      />
    );

    expect(screen.getByText("工作区：")).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Claude 工作区" })).toHaveValue("1");
    expect(screen.getByRole("option", { name: "工作区 A" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "工作区 A（当前）" })).not.toBeInTheDocument();
    expect(screen.getByText("默认提示词")).toBeInTheDocument();
    expect(screen.getByText("filesystem")).toBeInTheDocument();
    expect(screen.queryByRole("switch")).not.toBeInTheDocument();
    expect(screen.queryByText("路由策略：")).not.toBeInTheDocument();
  });

  it("switches workspace from the dropdown and ignores the active workspace", () => {
    const onSwitchWorkspace = vi.fn();

    render(
      <HomeWorkspaceConfigPanel
        configs={configs}
        selectedCliKey="claude"
        onSelectCliKey={vi.fn()}
        onSwitchWorkspace={onSwitchWorkspace}
      />
    );

    const select = screen.getByRole("combobox", { name: "Claude 工作区" });

    fireEvent.change(select, { target: { value: "1" } });
    expect(onSwitchWorkspace).not.toHaveBeenCalled();

    fireEvent.change(select, { target: { value: "3" } });
    expect(onSwitchWorkspace).toHaveBeenCalledWith("claude", 3);
  });

  it("renders the optional header addon and keeps cli switching available", () => {
    const onSelectCliKey = vi.fn();

    render(
      <HomeWorkspaceConfigPanel
        configs={configs}
        selectedCliKey="claude"
        onSelectCliKey={onSelectCliKey}
        headerAddon={<div>route-addon</div>}
        onSwitchWorkspace={vi.fn()}
      />
    );

    expect(screen.getByText("route-addon")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Codex" }));
    expect(onSelectCliKey).toHaveBeenCalledWith("codex");
  });

  it("renders quick toggles and emits item changes when enabled", () => {
    const onToggleItemEnabled = vi.fn();

    render(
      <HomeWorkspaceConfigPanel
        configs={configs}
        selectedCliKey="claude"
        onSelectCliKey={vi.fn()}
        showQuickToggle={true}
        onSwitchWorkspace={vi.fn()}
        onToggleItemEnabled={onToggleItemEnabled}
      />
    );

    expect(screen.queryByText("已启用")).not.toBeInTheDocument();
    expect(screen.queryByText("未启用")).not.toBeInTheDocument();
    expect(screen.getAllByRole("switch")).toHaveLength(2);

    fireEvent.click(screen.getByRole("switch", { name: "MCP filesystem 启用状态" }));

    expect(onToggleItemEnabled).toHaveBeenCalledWith(
      1,
      expect.objectContaining({ id: "mcp:1", resourceId: 1, type: "mcp" }),
      true
    );
  });

  it("disables quick toggles when workspace id is missing or item is toggling", () => {
    const missingWorkspaceConfig: HomeCliWorkspaceConfig = {
      ...configs[0],
      workspaceId: null,
    };

    const { rerender } = render(
      <HomeWorkspaceConfigPanel
        configs={[missingWorkspaceConfig]}
        selectedCliKey="claude"
        onSelectCliKey={vi.fn()}
        showQuickToggle={true}
        onSwitchWorkspace={vi.fn()}
        onToggleItemEnabled={vi.fn()}
      />
    );

    expect(screen.getByRole("switch", { name: "Prompt 默认提示词 启用状态" })).toBeDisabled();

    rerender(
      <HomeWorkspaceConfigPanel
        configs={configs}
        selectedCliKey="claude"
        onSelectCliKey={vi.fn()}
        showQuickToggle={true}
        togglingItemIds={new Set(["prompt:1"])}
        onSwitchWorkspace={vi.fn()}
        onToggleItemEnabled={vi.fn()}
      />
    );

    expect(screen.getByRole("switch", { name: "Prompt 默认提示词 启用状态" })).toBeDisabled();
  });

  it("disables workspace switching while loading or switching", () => {
    const { rerender } = render(
      <HomeWorkspaceConfigPanel
        configs={[{ ...configs[0], loading: true }]}
        selectedCliKey="claude"
        onSelectCliKey={vi.fn()}
        onSwitchWorkspace={vi.fn()}
      />
    );

    expect(screen.getByRole("combobox", { name: "Claude 工作区" })).toBeDisabled();

    rerender(
      <HomeWorkspaceConfigPanel
        configs={configs}
        selectedCliKey="claude"
        onSelectCliKey={vi.fn()}
        switchingWorkspaceKey="claude:3"
        onSwitchWorkspace={vi.fn()}
      />
    );

    expect(screen.getByRole("combobox", { name: "Claude 工作区" })).toBeDisabled();
    expect(screen.getByText("切换中…")).toBeInTheDocument();
  });
});
