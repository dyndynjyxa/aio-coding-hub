import { useCallback, useState } from "react";
import { toast } from "sonner";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { saveDesktopFilePath } from "../../services/desktop/dialog";
import { useConfigExportMutation } from "../../query/configMigrate";
import { Download } from "lucide-react";

export function ExportSection() {
  const configExportMutation = useConfigExportMutation();
  const [exportingAccounts, setExportingAccounts] = useState(false);
  const [exportingSettings, setExportingSettings] = useState(false);

  const doExport = useCallback(
    async (defaultName: string) => {
      const filePath = await saveDesktopFilePath({
        title: "导出配置",
        defaultPath: defaultName,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!filePath) return null;
      await configExportMutation.mutateAsync({ filePath });
      return filePath;
    },
    [configExportMutation]
  );

  const exportFull = useCallback(async () => {
    if (configExportMutation.isPending) return;
    try {
      const result = await doExport("aio-coding-hub-config-export.json");
      if (result) toast.success("配置导出成功");
    } catch (error) {
      toast.error(`导出失败：${error instanceof Error ? error.message : String(error)}`);
    }
  }, [configExportMutation, doExport]);

  const exportAccounts = useCallback(async () => {
    setExportingAccounts(true);
    try {
      const result = await doExport("aio-coding-hub-accounts-export.json");
      if (result) toast.success("账号数据导出成功");
    } catch (error) {
      toast.error(`导出失败：${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setExportingAccounts(false);
    }
  }, [doExport]);

  const exportSettings = useCallback(async () => {
    setExportingSettings(true);
    try {
      const result = await doExport("aio-coding-hub-settings-export.json");
      if (result) toast.success("用户设置导出成功");
    } catch (error) {
      toast.error(`导出失败：${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setExportingSettings(false);
    }
  }, [doExport]);

  return (
    <Card className="p-5">
      <div className="mb-3 flex items-center gap-2">
        <Download className="h-4 w-4 text-blue-600 dark:text-blue-400" />
        <h3 className="text-sm font-semibold text-foreground">导出数据</h3>
      </div>
      <p className="mb-4 text-xs leading-relaxed text-muted-foreground">
        将数据导出为JSON文件进行备份
      </p>

      <div className="divide-y divide-border">
        <div className="flex items-start justify-between gap-4 py-3 first:pt-0">
          <div className="min-w-0 flex-1">
            <div className="text-sm font-medium text-foreground">完整导出</div>
            <div className="mt-0.5 text-xs leading-relaxed text-muted-foreground">
              导出账号数据和用户设置。如需对移设备本地设置，请优先使用导出/导入，而不是 WebDAV
              同步。
            </div>
          </div>
          <Button
            variant="primary"
            size="sm"
            className="shrink-0"
            onClick={() => void exportFull()}
            disabled={configExportMutation.isPending}
          >
            {configExportMutation.isPending ? "导出中…" : "导出"}
          </Button>
        </div>

        <div className="flex items-start justify-between gap-4 py-3">
          <div className="min-w-0 flex-1">
            <div className="text-sm font-medium text-foreground">账号数据</div>
            <div className="mt-0.5 text-xs text-muted-foreground">仅导出账号信息和相关数据</div>
          </div>
          <Button
            variant="primary"
            size="sm"
            className="shrink-0"
            onClick={() => void exportAccounts()}
            disabled={exportingAccounts}
          >
            {exportingAccounts ? "导出中…" : "导出"}
          </Button>
        </div>

        <div className="flex items-start justify-between gap-4 py-3 last:pb-0">
          <div className="min-w-0 flex-1">
            <div className="text-sm font-medium text-foreground">用户设置</div>
            <div className="mt-0.5 text-xs text-muted-foreground">仅导出偏好设置和编好配置</div>
          </div>
          <Button
            variant="secondary"
            size="sm"
            className="shrink-0"
            onClick={() => void exportSettings()}
            disabled={exportingSettings}
          >
            {exportingSettings ? "导出中…" : "导出"}
          </Button>
        </div>
      </div>
    </Card>
  );
}
