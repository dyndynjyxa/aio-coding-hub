import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { SkillsView } from "../SkillsView";
import {
  useSkillCheckUpdatesMutation,
  useSkillImportLocalMutation,
  useSkillLocalDeleteMutation,
  useSkillReturnToLocalMutation,
  useSkillSetEnabledMutation,
  useSkillUninstallMutation,
  useSkillUpdateMutation,
  useSkillsInstalledListQuery,
  useSkillsLocalListQuery,
} from "../../../query/skills";
import { tauriOpenPath, tauriRevealItemInDir } from "../../../test/mocks/tauri";

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));

vi.mock("../../../query/skills", async () => {
  const actual =
    await vi.importActual<typeof import("../../../query/skills")>("../../../query/skills");
  return {
    ...actual,
    useSkillsInstalledListQuery: vi.fn(),
    useSkillsLocalListQuery: vi.fn(),
    useSkillSetEnabledMutation: vi.fn(),
    useSkillUninstallMutation: vi.fn(),
    useSkillReturnToLocalMutation: vi.fn(),
    useSkillLocalDeleteMutation: vi.fn(),
    useSkillImportLocalMutation: vi.fn(),
    useSkillCheckUpdatesMutation: vi.fn(),
    useSkillUpdateMutation: vi.fn(),
  };
});

describe("pages/skills/SkillsView", () => {
  it("supports enabling/deleting/returning installed skills and importing/deleting local skills", async () => {
    const installed = [
      {
        id: 1,
        name: "My Skill",
        description: "desc",
        enabled: false,
        source_git_url: "https://example.com/repo.git",
        source_branch: "main",
        source_subdir: "skills/my",
        updated_at: 123,
      },
    ] as any[];

    const localSkills = [
      { dir_name: "local-skill", name: "Local Skill", description: "d", path: "/tmp/local-skill" },
    ] as any[];

    vi.mocked(useSkillsInstalledListQuery).mockReturnValue({
      data: installed,
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(useSkillsLocalListQuery).mockReturnValue({
      data: localSkills,
      isFetching: false,
      error: null,
    } as any);

    const toggleMutation = { isPending: false, mutateAsync: vi.fn() };
    toggleMutation.mutateAsync.mockResolvedValue({ ...installed[0], enabled: true });
    vi.mocked(useSkillSetEnabledMutation).mockReturnValue(toggleMutation as any);

    const uninstallMutation = { isPending: false, mutateAsync: vi.fn(), variables: null };
    uninstallMutation.mutateAsync.mockResolvedValue(true);
    vi.mocked(useSkillUninstallMutation).mockReturnValue(uninstallMutation as any);

    const returnToLocalMutation = { isPending: false, mutateAsync: vi.fn(), variables: null };
    returnToLocalMutation.mutateAsync.mockResolvedValue(true);
    vi.mocked(useSkillReturnToLocalMutation).mockReturnValue(returnToLocalMutation as any);

    const localDeleteMutation = { isPending: false, mutateAsync: vi.fn(), variables: null };
    localDeleteMutation.mutateAsync.mockResolvedValue(true);
    vi.mocked(useSkillLocalDeleteMutation).mockReturnValue(localDeleteMutation as any);

    const importMutation = { isPending: false, mutateAsync: vi.fn(), variables: null };
    importMutation.mutateAsync.mockResolvedValue({ id: 2 });
    vi.mocked(useSkillImportLocalMutation).mockReturnValue(importMutation as any);

    const checkUpdatesMutation = { isPending: false, mutateAsync: vi.fn() };
    checkUpdatesMutation.mutateAsync.mockResolvedValue([]);
    vi.mocked(useSkillCheckUpdatesMutation).mockReturnValue(checkUpdatesMutation as any);

    const updateSkillMutation = { isPending: false, mutateAsync: vi.fn(), variables: null };
    vi.mocked(useSkillUpdateMutation).mockReturnValue(updateSkillMutation as any);

    tauriOpenPath.mockRejectedValueOnce(new Error("no opener"));
    tauriRevealItemInDir.mockResolvedValueOnce(undefined as any);

    render(<SkillsView workspaceId={1} cliKey="claude" isActiveWorkspace />);

    fireEvent.click(screen.getByRole("switch"));
    await waitFor(() =>
      expect(toggleMutation.mutateAsync).toHaveBeenCalledWith({ skillId: 1, enabled: true })
    );

    fireEvent.click(screen.getByRole("button", { name: "删除通用技能 My Skill" }));
    let dialog = within(screen.getByRole("dialog"));
    fireEvent.click(dialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(uninstallMutation.mutateAsync).toHaveBeenCalledWith(1));

    fireEvent.click(screen.getByRole("button", { name: "返回本机已安装" }));
    await waitFor(() => expect(returnToLocalMutation.mutateAsync).toHaveBeenCalledWith(1));

    const importButton = await screen.findByRole("button", { name: "导入技能库" });
    fireEvent.click(importButton);
    await waitFor(() => expect(importMutation.mutateAsync).toHaveBeenCalledWith("local-skill"));

    fireEvent.click(screen.getByRole("button", { name: "删除本机技能 Local Skill" }));
    dialog = within(screen.getByRole("dialog"));
    fireEvent.click(dialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() =>
      expect(localDeleteMutation.mutateAsync).toHaveBeenCalledWith("local-skill")
    );

    fireEvent.click(screen.getByRole("button", { name: "打开目录" }));
    await waitFor(() => expect(tauriRevealItemInDir).toHaveBeenCalledWith("/tmp/local-skill"));
  });

  it("supports batch deleting installed and local skills", async () => {
    const installed = [
      {
        id: 1,
        name: "Skill A",
        description: "A",
        enabled: true,
        source_git_url: "https://example.com/repo-a.git",
        source_branch: "main",
        source_subdir: "skills/a",
        updated_at: 100,
      },
      {
        id: 2,
        name: "Skill B",
        description: "B",
        enabled: false,
        source_git_url: "https://example.com/repo-b.git",
        source_branch: "main",
        source_subdir: "skills/b",
        updated_at: 200,
      },
    ] as any[];

    const localSkills = [
      { dir_name: "local-a", name: "Local A", description: "A", path: "/tmp/local-a" },
      { dir_name: "local-b", name: "Local B", description: "B", path: "/tmp/local-b" },
    ] as any[];

    vi.mocked(useSkillsInstalledListQuery).mockReturnValue({
      data: installed,
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(useSkillsLocalListQuery).mockReturnValue({
      data: localSkills,
      isFetching: false,
      error: null,
      refetch: vi.fn().mockResolvedValue({ data: localSkills }),
    } as any);
    vi.mocked(useSkillSetEnabledMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const uninstallMutation = { isPending: false, mutateAsync: vi.fn(), variables: null };
    uninstallMutation.mutateAsync.mockResolvedValue(true);
    vi.mocked(useSkillUninstallMutation).mockReturnValue(uninstallMutation as any);

    vi.mocked(useSkillReturnToLocalMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);

    const localDeleteMutation = { isPending: false, mutateAsync: vi.fn(), variables: null };
    localDeleteMutation.mutateAsync.mockResolvedValue(true);
    vi.mocked(useSkillLocalDeleteMutation).mockReturnValue(localDeleteMutation as any);

    vi.mocked(useSkillImportLocalMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);

    vi.mocked(useSkillCheckUpdatesMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn().mockResolvedValue([]),
    } as any);

    vi.mocked(useSkillUpdateMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);

    render(<SkillsView workspaceId={1} cliKey="claude" isActiveWorkspace />);

    fireEvent.click(screen.getByRole("checkbox", { name: "全选通用技能" }));
    fireEvent.click(screen.getByRole("button", { name: "删除通用技能 (2)" }));

    let dialog = within(screen.getByRole("dialog"));
    fireEvent.click(dialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(uninstallMutation.mutateAsync).toHaveBeenCalledTimes(2));
    expect(uninstallMutation.mutateAsync.mock.calls.map(([skillId]) => skillId)).toEqual([1, 2]);

    fireEvent.click(screen.getByRole("checkbox", { name: "全选本机技能" }));
    fireEvent.click(screen.getByRole("button", { name: "删除本机技能 (2)" }));

    dialog = within(screen.getByRole("dialog"));
    fireEvent.click(dialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(localDeleteMutation.mutateAsync).toHaveBeenCalledTimes(2));
    expect(localDeleteMutation.mutateAsync.mock.calls.map(([dirName]) => dirName)).toEqual([
      "local-a",
      "local-b",
    ]);
  });

  it("keeps batch_init entry as refresh-only for local skills", async () => {
    const localSkills = [
      { dir_name: "local-skill", name: "Local Skill", description: "d", path: "/tmp/local-skill" },
      {
        dir_name: " another-skill ",
        name: "Another Skill",
        description: "d2",
        path: "/tmp/another-skill",
      },
      {
        dir_name: "local-skill",
        name: "Local Skill Dup",
        description: "dup",
        path: "/tmp/local-skill-dup",
      },
    ] as any[];

    const refetch = vi.fn().mockResolvedValue({ data: localSkills });

    vi.mocked(useSkillsInstalledListQuery).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(useSkillsLocalListQuery).mockReturnValue({
      data: localSkills,
      isFetching: false,
      error: null,
      refetch,
    } as any);
    vi.mocked(useSkillSetEnabledMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSkillUninstallMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);
    vi.mocked(useSkillReturnToLocalMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);
    vi.mocked(useSkillLocalDeleteMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);
    vi.mocked(useSkillImportLocalMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);

    vi.mocked(useSkillCheckUpdatesMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn().mockResolvedValue([]),
    } as any);

    vi.mocked(useSkillUpdateMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);

    render(
      <SkillsView workspaceId={1} cliKey="claude" isActiveWorkspace localImportMode="batch_init" />
    );

    expect(screen.queryByRole("button", { name: "导入技能库" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "初始化同步" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "刷新本机技能" }));
    await waitFor(() => expect(refetch).toHaveBeenCalledTimes(1));
  });

  it("supports refreshing local list", async () => {
    const localSkills = [
      { dir_name: "local-skill", name: "Local Skill", description: "d", path: "/tmp/local-skill" },
    ] as any[];
    const refetch = vi.fn().mockResolvedValue({ data: localSkills });

    vi.mocked(useSkillsInstalledListQuery).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(useSkillsLocalListQuery).mockReturnValue({
      data: localSkills,
      isFetching: false,
      error: null,
      refetch,
    } as any);
    vi.mocked(useSkillSetEnabledMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSkillUninstallMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);
    vi.mocked(useSkillReturnToLocalMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);
    vi.mocked(useSkillLocalDeleteMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);
    vi.mocked(useSkillImportLocalMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);

    vi.mocked(useSkillCheckUpdatesMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn().mockResolvedValue([]),
    } as any);

    vi.mocked(useSkillUpdateMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);

    render(<SkillsView workspaceId={1} cliKey="claude" isActiveWorkspace />);

    fireEvent.click(screen.getByRole("button", { name: "刷新本机技能" }));
    await waitFor(() => expect(refetch).toHaveBeenCalledTimes(1));
  });

  it("renders read-only local section when workspace is not active", () => {
    vi.mocked(useSkillsInstalledListQuery).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(useSkillsLocalListQuery).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(useSkillSetEnabledMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSkillUninstallMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);
    vi.mocked(useSkillReturnToLocalMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);
    vi.mocked(useSkillLocalDeleteMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);
    vi.mocked(useSkillImportLocalMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);

    vi.mocked(useSkillCheckUpdatesMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn().mockResolvedValue([]),
    } as any);

    vi.mocked(useSkillUpdateMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);

    render(<SkillsView workspaceId={1} cliKey="gemini" isActiveWorkspace={false} />);
    expect(screen.getByText(/仅当前工作区可扫描\/导入本机 Skill/)).toBeInTheDocument();
  });

  it("covers tauri-only + error branches and local delete/import guards when workspace becomes inactive", async () => {
    const installed = [
      {
        id: 1,
        name: "S1",
        description: null,
        enabled: false,
        source_git_url: "https://example.com/repo.git",
        source_branch: "",
        source_subdir: "skills/s1",
        updated_at: 123,
      },
      {
        id: 2,
        name: "S2",
        description: "d",
        enabled: true,
        source_git_url: "https://example.com/repo2.git",
        source_branch: "main",
        source_subdir: "skills/s2",
        updated_at: 456,
      },
    ] as any[];

    const localSkills = [
      { dir_name: "local-skill", name: "", description: null, path: "/tmp/local-skill" },
    ] as any[];

    vi.mocked(useSkillsInstalledListQuery).mockReturnValue({
      data: installed,
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(useSkillsLocalListQuery).mockReturnValue({
      data: localSkills,
      isFetching: false,
      error: null,
      refetch: vi.fn().mockResolvedValue({ data: localSkills }),
    } as any);

    const toggleMutation = { isPending: false, mutateAsync: vi.fn(), variables: null };
    toggleMutation.mutateAsync
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce({ ...installed[1], enabled: false })
      .mockRejectedValueOnce(new Error("boom"));
    vi.mocked(useSkillSetEnabledMutation).mockReturnValue(toggleMutation as any);

    const uninstallMutation = { isPending: false, mutateAsync: vi.fn(), variables: null };
    uninstallMutation.mutateAsync
      .mockResolvedValueOnce(false)
      .mockRejectedValueOnce(new Error("boom"));
    vi.mocked(useSkillUninstallMutation).mockReturnValue(uninstallMutation as any);

    const returnToLocalMutation = { isPending: false, mutateAsync: vi.fn(), variables: null };
    returnToLocalMutation.mutateAsync
      .mockResolvedValueOnce(false)
      .mockRejectedValueOnce(new Error("boom"));
    vi.mocked(useSkillReturnToLocalMutation).mockReturnValue(returnToLocalMutation as any);

    const localDeleteMutation = { isPending: false, mutateAsync: vi.fn(), variables: null };
    localDeleteMutation.mutateAsync.mockResolvedValueOnce(false);
    vi.mocked(useSkillLocalDeleteMutation).mockReturnValue(localDeleteMutation as any);

    const importMutation = { isPending: false, mutateAsync: vi.fn(), variables: null };
    importMutation.mutateAsync.mockResolvedValueOnce(null);
    vi.mocked(useSkillImportLocalMutation).mockReturnValue(importMutation as any);

    vi.mocked(useSkillCheckUpdatesMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn().mockResolvedValue([]),
    } as any);

    vi.mocked(useSkillUpdateMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
      variables: null,
    } as any);

    tauriOpenPath
      .mockResolvedValueOnce(undefined as any)
      .mockRejectedValueOnce(new Error("no opener"));
    tauriRevealItemInDir.mockRejectedValueOnce(new Error("reveal failed"));

    function Wrapper() {
      const [active, setActive] = useState(true);
      return (
        <div>
          <button type="button" onClick={() => setActive(false)}>
            deactivate
          </button>
          <SkillsView workspaceId={1} cliKey="claude" isActiveWorkspace={active} />
        </div>
      );
    }

    render(<Wrapper />);

    fireEvent.click(screen.getAllByRole("switch")[0]!);
    await waitFor(() => expect(toggleMutation.mutateAsync).toHaveBeenCalledTimes(1));

    fireEvent.click(screen.getAllByRole("switch")[1]!);
    await waitFor(() => expect(toggleMutation.mutateAsync).toHaveBeenCalledTimes(2));

    fireEvent.click(screen.getAllByRole("switch")[0]!);
    await waitFor(() => expect(toggleMutation.mutateAsync).toHaveBeenCalledTimes(3));

    fireEvent.click(screen.getByRole("button", { name: "删除通用技能 S1" }));
    let dialog = within(screen.getByRole("dialog"));
    fireEvent.click(dialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(uninstallMutation.mutateAsync).toHaveBeenCalledTimes(1));

    fireEvent.click(screen.getByRole("button", { name: "删除通用技能 S1" }));
    dialog = within(screen.getByRole("dialog"));
    fireEvent.click(dialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(uninstallMutation.mutateAsync).toHaveBeenCalledTimes(2));

    fireEvent.click(screen.getByRole("button", { name: "打开目录" }));
    await waitFor(() => expect(tauriOpenPath).toHaveBeenCalledWith("/tmp/local-skill"));

    fireEvent.click(screen.getByRole("button", { name: "打开目录" }));
    await waitFor(() =>
      expect(vi.mocked(toast)).toHaveBeenCalledWith("打开目录失败：请查看控制台日志")
    );

    fireEvent.click(screen.getAllByRole("button", { name: "返回本机已安装" })[0]!);
    await waitFor(() => expect(returnToLocalMutation.mutateAsync).toHaveBeenCalledTimes(1));
    fireEvent.click(screen.getAllByRole("button", { name: "返回本机已安装" })[0]!);
    await waitFor(() => expect(returnToLocalMutation.mutateAsync).toHaveBeenCalledTimes(2));

    fireEvent.click(screen.getByRole("button", { name: "删除本机技能 local-skill" }));
    dialog = within(screen.getByRole("dialog"));
    fireEvent.click(dialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(localDeleteMutation.mutateAsync).toHaveBeenCalledTimes(1));

    const importButton = await screen.findByRole("button", { name: "导入技能库" });
    fireEvent.click(importButton);
    await waitFor(() => expect(importMutation.mutateAsync).toHaveBeenCalledTimes(1));

    fireEvent.click(screen.getByRole("button", { name: "删除本机技能 local-skill" }));
    dialog = within(screen.getByRole("dialog"));
    fireEvent.click(screen.getByRole("button", { name: "deactivate", hidden: true }));
    fireEvent.click(dialog.getByRole("button", { name: "确认删除" }));
    await waitFor(() =>
      expect(vi.mocked(toast)).toHaveBeenCalledWith(
        expect.stringContaining("仅当前工作区可删除本机 Skill")
      )
    );
    expect(localDeleteMutation.mutateAsync).toHaveBeenCalledTimes(1);
    expect(importMutation.mutateAsync).toHaveBeenCalledTimes(1);

    const refreshButton = screen.getByRole("button", { name: "刷新本机技能", hidden: true });
    expect(refreshButton).toBeDisabled();
  });
});
