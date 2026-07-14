import type { PendingCliProxyEnablePrompt } from "../../hooks/useCliProxyControls";
import { cliShortLabel } from "../../constants/clis";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";

export type CliProxyConflictDialogProps = {
  prompt: PendingCliProxyEnablePrompt | null;
  onCancel: () => void;
  onConfirm: () => void;
};

export function CliProxyConflictDialog({
  prompt,
  onCancel,
  onConfirm,
}: CliProxyConflictDialogProps) {
  return (
    <Dialog
      open={prompt != null}
      onOpenChange={(open) => {
        if (!open) onCancel();
      }}
      title={
        prompt
          ? `检测到 ${cliShortLabel(prompt.cliKey)} 代理相关环境变量冲突`
          : "检测到环境变量冲突"
      }
      description="继续启用可能会被这些环境变量覆盖（不会显示变量值）。是否继续？"
      className="max-w-lg"
    >
      {prompt ? (
        <div className="space-y-4">
          <ul className="space-y-2">
            {prompt.conflicts.map((row) => (
              <li
                key={`${row.var_name}:${row.source_type}:${row.source_path}`}
                className="rounded-lg border border-border bg-secondary px-3 py-2"
              >
                <div className="font-mono text-xs text-foreground">{row.var_name}</div>
                <div className="mt-1 break-all text-xs text-muted-foreground">
                  {row.source_path}
                </div>
              </li>
            ))}
          </ul>

          <div className="flex items-center justify-end gap-2">
            <Button variant="secondary" size="md" onClick={onCancel}>
              取消
            </Button>
            <Button variant="primary" size="md" onClick={onConfirm}>
              继续启用
            </Button>
          </div>
        </div>
      ) : null}
    </Dialog>
  );
}
