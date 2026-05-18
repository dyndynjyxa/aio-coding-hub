import { AlertTriangle } from "lucide-react";

export function DataSyncWarning() {
  return (
    <div className="rounded-lg border border-amber-200 bg-amber-50/60 px-4 py-3 dark:border-amber-800 dark:bg-amber-950/30">
      <div className="flex items-start gap-2.5">
        <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-amber-600 dark:text-amber-400" />
        <div>
          <h4 className="mb-1.5 text-xs font-semibold text-amber-900 dark:text-amber-200">
            重要提示
          </h4>
          <ul className="space-y-0.5 text-[11px] leading-relaxed text-amber-800 dark:text-amber-300">
            <li>• 导入数据将覆盖现有的相同类型数据，请谨慎操作</li>
            <li>• 建议在导入前对当前数据进行备份</li>
            <li>• 仅支持本程序导出的JSON格式文件</li>
            <li>
              • 导入的数据可能包含敏感信息（账号密码与 API Key），请妥善保管备份文件并确保来源可信
            </li>
          </ul>
        </div>
      </div>
    </div>
  );
}
