import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import {
  useMcpImportFromWorkspaceCliMutation,
  useMcpServerDeleteMutation,
  useMcpServerSetEnabledMutation,
  useMcpServersListQuery,
  type McpImportReport,
} from "../../query/mcp";
import { logToConsole } from "../../services/consoleLog";
import { type McpServerSummary } from "../../services/workspace/mcp";
import { Button } from "../../ui/Button";
import { EmptyState } from "../../ui/EmptyState";
import { Spinner } from "../../ui/Spinner";
import { McpDeleteDialog } from "./components/McpDeleteDialog";
import { McpServerCard } from "./components/McpServerCard";
import { McpServerDialog } from "./components/McpServerDialog";

export type McpServersViewProps = {
  workspaceId: number;
};

function formatImportSummary(report: McpImportReport) {
  const skippedCount = report.skipped?.length ?? 0;
  return `导入完成：新增 ${report.inserted}，更新 ${report.updated}${skippedCount > 0 ? `，跳过 ${skippedCount}` : ""}`;
}

export function McpServersView({ workspaceId }: McpServersViewProps) {
  const mcpServersQuery = useMcpServersListQuery(workspaceId);
  const toggleMutation = useMcpServerSetEnabledMutation(workspaceId);
  const deleteMutation = useMcpServerDeleteMutation(workspaceId);
  const importFromWorkspaceMutation = useMcpImportFromWorkspaceCliMutation(workspaceId);

  const items: McpServerSummary[] = mcpServersQuery.data ?? [];
  const loading = mcpServersQuery.isFetching;
  const toggling = toggleMutation.isPending;
  const deleting = deleteMutation.isPending;
  const importing = importFromWorkspaceMutation.isPending;

  const [dialogOpen, setDialogOpen] = useState(false);
  const [editTarget, setEditTarget] = useState<McpServerSummary | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<McpServerSummary | null>(null);

  useEffect(() => {
    if (!mcpServersQuery.error) return;
    logToConsole("error", "加载 MCP Servers 失败", { error: String(mcpServersQuery.error) });
    toast("加载失败：请查看控制台日志");
  }, [mcpServersQuery.error]);

  const toggleEnabled = useCallback(
    async (server: McpServerSummary) => {
      if (toggleMutation.isPending) return;
      const nextEnabled = !server.enabled;

      try {
        const next = await toggleMutation.mutateAsync({
          serverId: server.id,
          enabled: nextEnabled,
        });
        if (!next) {
          return;
        }

        logToConsole("info", "切换 MCP Server 生效范围", {
          id: next.id,
          server_key: next.server_key,
          workspace_id: workspaceId,
          enabled: nextEnabled,
        });
        toast(nextEnabled ? "已启用" : "已停用");
      } catch (err) {
        logToConsole("error", "切换 MCP Server 生效范围失败", {
          error: String(err),
          id: server.id,
          workspace_id: workspaceId,
        });
        toast(`操作失败：${String(err)}`);
      }
    },
    [toggleMutation, workspaceId]
  );

  const handleEdit = useCallback((server: McpServerSummary) => {
    setEditTarget(server);
    setDialogOpen(true);
  }, []);

  async function confirmDelete() {
    if (!deleteTarget) return;
    if (deleting) return;
    const target = deleteTarget;
    try {
      const ok = await deleteMutation.mutateAsync(target.id);
      if (!ok) {
        return;
      }
      logToConsole("info", "删除 MCP Server", { id: target.id, server_key: target.server_key });
      toast("已删除");
      setDeleteTarget(null);
    } catch (err) {
      logToConsole("error", "删除 MCP Server 失败", { error: String(err), id: target.id });
      toast(`删除失败：${String(err)}`);
    }
  }

  async function importFromCurrentCli() {
    if (importing) return;

    try {
      const report = await importFromWorkspaceMutation.mutateAsync();
      if (!report) {
        return;
      }

      const summary = formatImportSummary(report);
      toast(summary);
      logToConsole("info", "从当前 CLI 自动导入 MCP 完成", {
        workspace_id: workspaceId,
        inserted: report.inserted,
        updated: report.updated,
        skipped: report.skipped ?? [],
      });
    } catch (err) {
      const message = String(err);
      logToConsole("error", "从当前 CLI 自动导入 MCP 失败", {
        error: message,
        workspace_id: workspaceId,
      });
      toast(`导入失败：${message}`);
    }
  }

  return (
    <>
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-xs text-slate-500 dark:text-slate-400">
            {loading ? "加载中…" : `共 ${items.length} 条`}
          </span>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            variant="secondary"
            onClick={() => void importFromCurrentCli()}
            disabled={importing}
          >
            {importing ? "导入中…" : "导入已有"}
          </Button>
          <Button
            onClick={() => {
              setEditTarget(null);
              setDialogOpen(true);
            }}
            variant="primary"
          >
            添加 MCP
          </Button>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-400">
          <Spinner size="sm" />
          加载中…
        </div>
      ) : items.length === 0 ? (
        <EmptyState title="暂无 MCP 服务" description="点击右上角「添加 MCP」创建第一条。" />
      ) : (
        <div className="space-y-2">
          {items.map((server) => (
            <McpServerCard
              key={server.id}
              server={server}
              toggling={toggling}
              onToggleEnabled={toggleEnabled}
              onEdit={handleEdit}
              onDelete={setDeleteTarget}
            />
          ))}
        </div>
      )}

      <McpServerDialog
        workspaceId={workspaceId}
        open={dialogOpen}
        editTarget={editTarget}
        onOpenChange={(open) => {
          setDialogOpen(open);
          if (!open) setEditTarget(null);
        }}
      />

      <McpDeleteDialog
        target={deleteTarget}
        deleting={deleting}
        onConfirm={() => void confirmDelete()}
        onClose={() => setDeleteTarget(null)}
      />
    </>
  );
}
