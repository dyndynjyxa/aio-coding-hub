import { PageHeader } from "../../ui/PageHeader";
import { ExportSection } from "./ExportSection";
import { ImportSection } from "./ImportSection";
import { WebDavSyncSection } from "./WebDavSyncSection";
import { WebDavAutoSyncSection } from "./WebDavAutoSyncSection";
import { DataSyncWarning } from "./DataSyncWarning";

export function DataSyncPage() {
  return (
    <div className="flex h-full flex-col gap-6 overflow-hidden">
      <PageHeader title="导入/导出" subtitle="备份和恢复操作数据" />
      <div className="min-h-0 flex-1 overflow-y-auto scrollbar-overlay">
        <div className="space-y-6 pb-8">
          {/* 导出 + 导入 并排 */}
          <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
            <ExportSection />
            <ImportSection />
          </div>

          {/* WebDAV 同步 + 自动同步：宽屏一行，窄屏换行 */}
          <div className="grid grid-cols-1 gap-6 xl:grid-cols-2">
            <WebDavSyncSection />
            <WebDavAutoSyncSection />
          </div>

          {/* 重要提示 */}
          <DataSyncWarning />
        </div>
      </div>
    </div>
  );
}
