// check-pnpm-audit.mjs 纯逻辑自检：树遍历收集、级别计数、阻断判定、阻断明细。
import assert from "node:assert/strict";

import {
  collectPackageVersions,
  extractSeverityCounts,
  formatBlockingAdvisories,
  hasBlockingVulnerabilities,
} from "./check-pnpm-audit.mjs";

// 正常路径：跨项目递归收集 name -> versions，去重且覆盖 optionalDependencies。
{
  const projects = [
    {
      dependencies: {
        foo: { version: "1.0.0", dependencies: { bar: { version: "2.0.0" } } },
        dup: { version: "3.0.0" },
      },
      optionalDependencies: {
        opt: { version: "4.0.0" },
      },
    },
    {
      dependencies: {
        dup: { version: "3.0.0" },
        linked: { version: "link:packages/x" },
        workspacePkg: { version: "workspace:*" },
      },
    },
  ];

  const collected = collectPackageVersions(projects);
  assert.deepEqual(
    Object.fromEntries([...collected].map(([name, versions]) => [name, [...versions].sort()])),
    {
      foo: ["1.0.0"],
      bar: ["2.0.0"],
      dup: ["3.0.0"],
      opt: ["4.0.0"],
    }
  );
}

// 边界：空输入、非法节点、缺失 version 都不产出也不抛错。
{
  assert.equal(collectPackageVersions([]).size, 0);
  assert.equal(collectPackageVersions(null).size, 0);
  assert.equal(collectPackageVersions([{ dependencies: { bad: null, noVersion: {} } }]).size, 0);
}

// 正常路径：按 advisory 逐条计数，大小写归一，忽略未知级别与非法条目。
{
  const advisoriesByPackage = {
    lodash: [{ severity: "high" }, { severity: "moderate" }],
    minimatch: [{ severity: "HIGH" }, { severity: "unknown" }, null],
    weird: "not-an-array",
  };
  const counts = extractSeverityCounts(advisoriesByPackage);
  assert.deepEqual(counts, { info: 0, low: 0, moderate: 1, high: 2, critical: 0 });
  assert.equal(hasBlockingVulnerabilities(counts), true);
}

// 失败路径反例：只有低危不阻断。
{
  const counts = extractSeverityCounts({ lodash: [{ severity: "low" }] });
  assert.deepEqual(counts, { info: 0, low: 1, moderate: 0, high: 0, critical: 0 });
  assert.equal(hasBlockingVulnerabilities(counts), false);
}

// 阻断明细：只列出 high / critical，带标题与链接。
{
  const lines = formatBlockingAdvisories({
    lodash: [
      { severity: "critical", title: "RCE", url: "https://example.test/a" },
      { severity: "low", title: "noise", url: "https://example.test/b" },
    ],
  });
  assert.deepEqual(lines, ["[pnpm-audit] critical: lodash — RCE (https://example.test/a)"]);
}

console.error("[pnpm-audit:selftest] 全部断言通过。");
