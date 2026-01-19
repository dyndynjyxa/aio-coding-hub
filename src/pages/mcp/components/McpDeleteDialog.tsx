import type { McpServerSummary } from "../../../services/mcp";
import { Button } from "../../../ui/Button";
import { Dialog } from "../../../ui/Dialog";

export type McpDeleteDialogProps = {
  target: McpServerSummary | null;
  deleting: boolean;
  onConfirm: () => void;
  onClose: () => void;
};

export function McpDeleteDialog({ target, deleting, onConfirm, onClose }: McpDeleteDialogProps) {
  return (
    <Dialog
      open={Boolean(target)}
      title="确认删除"
      description={
        target ? `将删除「${target.name}」并从已启用的 CLI 配置中移除（不可恢复）。` : undefined
      }
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      className="max-w-xl"
    >
      <div className="flex flex-wrap items-center gap-2">
        <Button onClick={onConfirm} variant="primary" disabled={deleting}>
          {deleting ? "删除中…" : "确认删除"}
        </Button>
        <Button onClick={onClose} variant="secondary" disabled={deleting}>
          取消
        </Button>
      </div>
    </Dialog>
  );
}
