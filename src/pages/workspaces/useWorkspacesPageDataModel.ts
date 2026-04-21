// Usage: Data-model hook for workspace page orchestration.

import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { CLIS } from "../../constants/clis";
import { useMcpServersListQuery } from "../../query/mcp";
import { usePromptsListQuery } from "../../query/prompts";
import { useSettingsQuery } from "../../query/settings";
import { useSkillsInstalledListQuery } from "../../query/skills";
import {
  pickWorkspaceById,
  useWorkspaceApplyMutation,
  useWorkspaceCreateMutation,
  useWorkspaceDeleteMutation,
  useWorkspacePreviewQuery,
  useWorkspaceRenameMutation,
  useWorkspacesListQuery,
} from "../../query/workspaces";
import { logToConsole } from "../../services/consoleLog";
import { getOrderedClis, pickDefaultCliByPriority } from "../../services/cli/cliPriorityOrder";
import type { CliKey } from "../../services/providers/providers";
import { type WorkspaceApplyReport, type WorkspaceSummary } from "../../services/workspace/workspaces";

export type WorkspacesRightTab = "overview" | "prompts" | "mcp" | "skills";

type OverviewStats = {
  prompts: { total: number; enabled: number };
  mcp: { total: number; enabled: number };
  skills: { total: number; enabled: number };
};

type CreateMode = "clone_active" | "blank";

export const WORKSPACES_RIGHT_TABS: Array<{ key: WorkspacesRightTab; label: string }> = [
  { key: "overview", label: "总览" },
  { key: "prompts", label: "Prompts" },
  { key: "mcp", label: "MCP" },
  { key: "skills", label: "Skills" },
];

function normalizeWorkspaceName(raw: string) {
  return raw.trim();
}

function isDuplicateWorkspaceName(
  items: WorkspaceSummary[],
  name: string,
  ignoreId?: number | null
) {
  const normalized = normalizeWorkspaceName(name).toLowerCase();
  if (!normalized) return false;

  return items.some((workspace) => {
    if (ignoreId && workspace.id === ignoreId) return false;
    return normalizeWorkspaceName(workspace.name).toLowerCase() === normalized;
  });
}

export function useWorkspacesPageDataModel() {
  const settingsQuery = useSettingsQuery();
  const orderedCliTabs = getOrderedClis(settingsQuery.data?.cli_priority_order);
  const orderedCliKeys = orderedCliTabs.map((cli) => cli.key);
  const defaultCli =
    pickDefaultCliByPriority(settingsQuery.data?.cli_priority_order, orderedCliKeys) ?? CLIS[0].key;

  const [activeCli, setActiveCli] = useState<CliKey | null>(null);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<number | null>(null);
  const [filterText, setFilterText] = useState("");
  const [rightTab, setRightTab] = useState<WorkspacesRightTab>("overview");
  const [createOpen, setCreateOpen] = useState(false);
  const [createName, setCreateName] = useState("");
  const [createMode, setCreateMode] = useState<CreateMode>("blank");
  const [renameTargetId, setRenameTargetId] = useState<number | null>(null);
  const [renameOpen, setRenameOpen] = useState(false);
  const [renameName, setRenameName] = useState("");
  const [deleteTargetId, setDeleteTargetId] = useState<number | null>(null);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [applyReport, setApplyReport] = useState<WorkspaceApplyReport | null>(null);
  const [switchOpen, setSwitchOpen] = useState(false);
  const [switchTargetId, setSwitchTargetId] = useState<number | null>(null);
  const [switchConfirm, setSwitchConfirm] = useState("");

  const effectiveCli = activeCli ?? defaultCli;
  const workspacesQuery = useWorkspacesListQuery(effectiveCli);
  const createMutation = useWorkspaceCreateMutation();
  const renameMutation = useWorkspaceRenameMutation();
  const deleteMutation = useWorkspaceDeleteMutation();
  const applyMutation = useWorkspaceApplyMutation();

  const items = useMemo<WorkspaceSummary[]>(
    () => workspacesQuery.data?.items ?? [],
    [workspacesQuery.data?.items]
  );
  const activeWorkspaceId = workspacesQuery.data?.active_id ?? null;
  const loading = workspacesQuery.isFetching;

  useEffect(() => {
    if (!workspacesQuery.error) return;

    logToConsole("error", "加载工作区失败", {
      error: String(workspacesQuery.error),
      cli: effectiveCli,
    });
    toast("加载失败：请查看控制台日志");
  }, [effectiveCli, workspacesQuery.error]);

  useEffect(() => {
    const result = workspacesQuery.data;
    if (!result) return;

    setSelectedWorkspaceId((previousId) => {
      const stillExists = previousId != null && result.items.some((workspace) => workspace.id === previousId);
      if (stillExists) return previousId;
      if (result.active_id != null) return result.active_id;
      return result.items[0]?.id ?? null;
    });
  }, [workspacesQuery.data]);

  const filtered = useMemo(() => {
    const normalizedQuery = filterText.trim().toLowerCase();
    if (!normalizedQuery) return items;

    return items.filter((workspace) =>
      normalizeWorkspaceName(workspace.name).toLowerCase().includes(normalizedQuery)
    );
  }, [filterText, items]);

  const selectedWorkspace = useMemo(
    () => pickWorkspaceById(items, selectedWorkspaceId),
    [items, selectedWorkspaceId]
  );

  const workspaceById = useMemo(() => new Map(items.map((workspace) => [workspace.id, workspace])), [items]);
  const switchTarget =
    switchTargetId != null ? (workspaceById.get(switchTargetId) ?? null) : null;

  useEffect(() => {
    if (!filterText.trim()) return;
    if (!selectedWorkspace) return;
    if (filtered.some((workspace) => workspace.id === selectedWorkspace.id)) return;
    if (filtered.length === 0) return;

    setSelectedWorkspaceId(filtered[0].id);
  }, [filterText, filtered, selectedWorkspace]);

  const overviewWorkspaceId = rightTab === "overview" ? (selectedWorkspace?.id ?? null) : null;
  const promptsQuery = usePromptsListQuery(overviewWorkspaceId, {
    enabled: overviewWorkspaceId != null,
  });
  const mcpServersQuery = useMcpServersListQuery(overviewWorkspaceId, {
    enabled: overviewWorkspaceId != null,
  });
  const skillsQuery = useSkillsInstalledListQuery(overviewWorkspaceId, {
    enabled: overviewWorkspaceId != null,
  });

  const overviewLoading =
    promptsQuery.isFetching || mcpServersQuery.isFetching || skillsQuery.isFetching;

  const overviewStats: OverviewStats | null = useMemo(() => {
    if (!overviewWorkspaceId) return null;
    if (!promptsQuery.data || !mcpServersQuery.data || !skillsQuery.data) return null;

    return {
      prompts: {
        total: promptsQuery.data.length,
        enabled: promptsQuery.data.filter((prompt) => prompt.enabled).length,
      },
      mcp: {
        total: mcpServersQuery.data.length,
        enabled: mcpServersQuery.data.filter((server) => server.enabled).length,
      },
      skills: {
        total: skillsQuery.data.length,
        enabled: skillsQuery.data.filter((skill) => skill.enabled).length,
      },
    };
  }, [mcpServersQuery.data, overviewWorkspaceId, promptsQuery.data, skillsQuery.data]);

  const createError = useMemo(() => {
    const normalizedName = normalizeWorkspaceName(createName);
    if (!normalizedName) return "名称不能为空";
    if (isDuplicateWorkspaceName(items, normalizedName)) return "名称重复：同一 CLI 下必须唯一";
    return null;
  }, [createName, items]);

  const renameTarget = useMemo(() => {
    if (!renameTargetId) return null;
    return items.find((workspace) => workspace.id === renameTargetId) ?? null;
  }, [items, renameTargetId]);

  const renameError = useMemo(() => {
    if (!renameOpen) return null;

    const normalizedName = normalizeWorkspaceName(renameName);
    if (!normalizedName) return "名称不能为空";
    if (isDuplicateWorkspaceName(items, normalizedName, renameTargetId)) {
      return "名称重复：同一 CLI 下必须唯一";
    }
    return null;
  }, [items, renameName, renameOpen, renameTargetId]);

  const deleteTarget = useMemo(() => {
    if (!deleteTargetId) return null;
    return items.find((workspace) => workspace.id === deleteTargetId) ?? null;
  }, [items, deleteTargetId]);

  const previewQuery = useWorkspacePreviewQuery(switchTarget?.id ?? null, {
    enabled: switchOpen,
  });
  const preview = previewQuery.data ?? null;
  const previewLoading = previewQuery.isFetching;
  const applying = applyMutation.isPending;

  function openCreateDialog() {
    setCreateName("");
    setCreateMode("blank");
    setCreateOpen(true);
  }

  async function createWorkspace() {
    if (createError) return;

    const normalizedName = normalizeWorkspaceName(createName);
    if (!normalizedName) return;

    try {
      const created = await createMutation.mutateAsync({
        cliKey: effectiveCli,
        name: normalizedName,
        cloneFromActive: createMode === "clone_active",
      });
      if (!created) return;

      toast("已创建");
      setCreateOpen(false);
      setSelectedWorkspaceId(created.id);
      setRightTab("overview");
    } catch (error) {
      logToConsole("error", "创建工作区失败", {
        error: String(error),
        cli: effectiveCli,
      });
      toast(`创建失败：${String(error)}`);
    }
  }

  function openRenameDialog(target: WorkspaceSummary) {
    setRenameTargetId(target.id);
    setRenameName(target.name);
    setRenameOpen(true);
  }

  async function renameWorkspace() {
    if (!renameTarget || renameError) return;

    const normalizedName = normalizeWorkspaceName(renameName);
    if (!normalizedName) return;

    try {
      const next = await renameMutation.mutateAsync({
        cliKey: effectiveCli,
        workspaceId: renameTarget.id,
        name: normalizedName,
      });
      if (!next) return;

      toast("已重命名");
      setRenameOpen(false);
      setRenameTargetId(null);
    } catch (error) {
      logToConsole("error", "重命名工作区失败", {
        error: String(error),
        id: renameTarget.id,
      });
      toast(`重命名失败：${String(error)}`);
    }
  }

  function openDeleteDialog(target: WorkspaceSummary) {
    setDeleteTargetId(target.id);
    setDeleteOpen(true);
  }

  async function deleteWorkspace() {
    if (!deleteTarget) return;

    try {
      const deleted = await deleteMutation.mutateAsync({
        cliKey: effectiveCli,
        workspaceId: deleteTarget.id,
      });
      if (!deleted) return;

      toast("已删除");
      setDeleteOpen(false);
      setDeleteTargetId(null);
    } catch (error) {
      logToConsole("error", "删除工作区失败", {
        error: String(error),
        id: deleteTarget.id,
      });
      toast(`删除失败：${String(error)}`);
    }
  }

  function openSwitchDialog(workspaceId: number) {
    setRightTab("overview");
    setSwitchTargetId(workspaceId);
    setSwitchConfirm("");
    setSwitchOpen(true);
  }

  async function applySwitchTarget() {
    if (!switchTarget) return;
    if (switchTarget.id === activeWorkspaceId || applying) return;

    try {
      const next = await applyMutation.mutateAsync({
        cliKey: effectiveCli,
        workspaceId: switchTarget.id,
      });
      if (!next) return;

      setApplyReport(next);
      toast("已切换为当前工作区");
      setSwitchOpen(false);
      setSwitchConfirm("");
    } catch (error) {
      logToConsole("error", "应用工作区失败", {
        error: String(error),
        workspace_id: switchTarget.id,
      });
      toast(`应用失败：${String(error)}`);
    }
  }

  async function rollbackToPrevious() {
    if (!applyReport?.from_workspace_id || applying) return;

    try {
      const next = await applyMutation.mutateAsync({
        cliKey: effectiveCli,
        workspaceId: applyReport.from_workspace_id,
      });
      if (!next) return;

      setApplyReport(next);
      toast("已回滚到上一个工作区");
    } catch (error) {
      logToConsole("error", "回滚工作区失败", {
        error: String(error),
        from_workspace_id: applyReport.from_workspace_id,
      });
      toast(`回滚失败：${String(error)}`);
    }
  }

  return {
    orderedCliTabs,
    effectiveCli,
    setActiveCli,
    items,
    loading,
    selectedWorkspaceId,
    setSelectedWorkspaceId,
    filterText,
    setFilterText,
    filtered,
    activeWorkspaceId,
    selectedWorkspace,
    rightTab,
    setRightTab,
    overviewLoading,
    overviewStats,
    applyReport,
    applying,
    rollbackToPrevious,
    openCreateDialog,
    openRenameDialog,
    openDeleteDialog,
    openSwitchDialog,
    createOpen,
    setCreateOpen,
    createName,
    setCreateName,
    createMode,
    setCreateMode,
    createError,
    createWorkspace,
    renameOpen,
    setRenameOpen,
    renameTarget,
    renameName,
    setRenameName,
    renameError,
    renameWorkspace,
    setRenameTargetId,
    deleteOpen,
    setDeleteOpen,
    deleteTarget,
    deleteWorkspace,
    setDeleteTargetId,
    switchOpen,
    setSwitchOpen,
    setSwitchTargetId,
    switchTarget,
    switchConfirm,
    setSwitchConfirm,
    previewQuery,
    preview,
    previewLoading,
    workspaceById,
    applySwitchTarget,
  };
}
