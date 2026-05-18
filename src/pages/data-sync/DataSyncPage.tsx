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
        <div className="pb-8">
          {/*
            窄屏单列顺序：导出 → 导入 → WebDAV同步 → 自动同步 → 提示
            宽屏双列错落：左列（导出、WebDAV同步、提示）右列（导入、自动同步）
            用 flex + order 实现两种排列
          */}
          <div className="flex flex-col gap-6 lg:flex-row lg:items-start lg:gap-6">
            {/* 左列 */}
            <div className="order-1 flex-1 space-y-6 lg:order-none lg:basis-[58%]">
              <ExportSection />
              {/* 窄屏时导入插在这里（通过下面的 lg:hidden） */}
              <div className="lg:hidden">
                <ImportSection />
              </div>
              <WebDavSyncSection />
              <div className="lg:hidden">
                <WebDavAutoSyncSection />
              </div>
              <DataSyncWarning />
            </div>

            {/* 右列：仅宽屏显示 */}
            <div className="hidden space-y-6 lg:block lg:basis-[42%]">
              <ImportSection />
              <WebDavAutoSyncSection />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
