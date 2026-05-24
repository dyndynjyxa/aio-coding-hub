import { useCallback, useState } from "react";
import { toast } from "sonner";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { openDesktopSinglePath } from "../../services/desktop/dialog";
import { useConfigImportMutation } from "../../query/configMigrate";
import { Upload } from "lucide-react";

export function ImportSection() {
  const configImportMutation = useConfigImportMutation();
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [importDescription, setImportDescription] = useState("");

  const selectFile = useCallback(async () => {
    try {
      const filePath = await openDesktopSinglePath({
        multiple: false,
        title: "选择备份文件",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (filePath) {
        setSelectedFile(filePath);
      }
    } catch (error) {
      toast.error(`选择文件失败：${error instanceof Error ? error.message : String(error)}`);
    }
  }, []);

  const doImport = useCallback(async () => {
    if (!selectedFile || configImportMutation.isPending) return;
    try {
      const result = await configImportMutation.mutateAsync({ filePath: selectedFile });
      if (result) {
        toast.success(
          `导入成功：${result.providers_imported} 个供应商，${result.workspaces_imported} 个工作区`
        );
        setSelectedFile(null);
        setImportDescription("");
      }
    } catch (error) {
      toast.error(`导入失败：${error instanceof Error ? error.message : String(error)}`);
    }
  }, [selectedFile, configImportMutation]);

  return (
    <Card className="p-5">
      <div className="mb-3 flex items-center gap-2">
        <Upload className="h-4 w-4 text-blue-600 dark:text-blue-400" />
        <h3 className="text-sm font-semibold text-foreground">导入数据</h3>
      </div>
      <p className="mb-4 text-xs leading-relaxed text-muted-foreground">从备份文件恢复数据</p>

      <div className="space-y-3">
        <div>
          <label className="mb-1.5 block text-xs font-medium text-foreground">选择备份文件</label>
          <div className="flex items-center gap-3">
            <Button variant="secondary" size="sm" onClick={() => void selectFile()}>
              选择文件
            </Button>
            <span className="truncate text-xs text-muted-foreground">
              {selectedFile ? selectedFile.split("/").pop() : "未选择任何文件"}
            </span>
          </div>
        </div>

        <div>
          <label className="mb-1.5 block text-xs font-medium text-foreground">数据描述</label>
          <textarea
            className="h-[72px] w-full resize-none rounded-lg border border-input bg-card px-3 py-2 text-sm text-foreground shadow-sm outline-none transition placeholder:text-muted-foreground focus:border-ring focus:ring-2 focus:ring-ring/20"
            placeholder="给该JSON数据做说明以上面的文件进行导入..."
            value={importDescription}
            onChange={(e) => setImportDescription(e.target.value)}
          />
        </div>

        <Button
          variant="primary"
          size="md"
          className="w-full"
          onClick={() => void doImport()}
          disabled={!selectedFile || configImportMutation.isPending}
        >
          {configImportMutation.isPending ? "导入中…" : "导入"}
        </Button>
      </div>
    </Card>
  );
}
