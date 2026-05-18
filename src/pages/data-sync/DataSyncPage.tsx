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
        <div className="mx-auto max-w-4xl space-y-8 pb-8">
          {/* 导出 + 导入 并排 */}
          <div className="grid grid-cols-1 gap-6 md:grid-cols-2">
            <ExportSection />
            <ImportSection />
          </div>

          {/* WebDAV 同步 */}
          <WebDavSyncSection />

          {/* WebDAV 自动同步 */}
          <WebDavAutoSyncSection />

          {/* 重要提示 */}
          <DataSyncWarning />
        </div>
      </div>
    </div>
  );
}
