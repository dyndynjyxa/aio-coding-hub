// Usage: 用量表格加载态骨架屏。

import { TABLE_COLUMNS } from "./UsageTableColumns";

const SKELETON_ROWS = 5;

const TH_CLASS =
  "border-b border-border bg-secondary/60 dark:bg-secondary/60 px-3 py-2.5 backdrop-blur-sm";

export function UsageTableSkeleton() {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-separate border-spacing-0 text-left text-sm">
        <thead>
          <tr className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {TABLE_COLUMNS.map((col) => (
              <th key={col.key} className={TH_CLASS}>
                {col.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="animate-pulse">
          {Array.from({ length: SKELETON_ROWS }).map((_, idx) => (
            <tr key={idx} className="align-top">
              {TABLE_COLUMNS.map((col) => (
                <td key={col.key} className="border-b border-border px-3 py-3.5">
                  <div className={`h-3 ${col.width} rounded-md bg-muted dark:bg-secondary`} />
                  {col.key === "name" ? (
                    <div className="mt-2 h-3 w-48 rounded-md bg-secondary dark:bg-secondary" />
                  ) : null}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
