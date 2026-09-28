// check-pnpm-audit.mjs 自检：依赖收集、漏洞判定，以及审计请求的重试/失败关闭。
import assert from "node:assert/strict";

import {
  collectPackageVersions,
  extractSeverityCounts,
  fetchBulkAdvisories,
  fetchOsvAdvisories,
  formatBlockingAdvisories,
  hasBlockingVulnerabilities,
  queryAdvisories,
} from "./check-pnpm-audit.mjs";

function jsonResponse(payload, status = 200, detail = "") {
  return {
    ok: status >= 200 && status < 300,
    status,
    async json() {
      return payload;
    },
    async text() {
      return detail;
    },
  };
}

function timeoutError() {
  const error = new Error("The operation was aborted due to timeout");
  error.name = "TimeoutError";
  return error;
}

const noTimeoutSignal = () => undefined;
const noLog = () => {};

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

// 瞬时超时会退避后重试，并复用同一请求体。
{
  let requestCount = 0;
  const delays = [];
  const requestBody = { foo: ["1.0.0"] };
  const advisories = await fetchBulkAdvisories(requestBody, {
    fetchImpl: async (_endpoint, options) => {
      requestCount += 1;
      assert.equal(options.body, JSON.stringify(requestBody));
      if (requestCount === 1) {
        throw timeoutError();
      }
      return jsonResponse({});
    },
    createTimeoutSignal: noTimeoutSignal,
    retryBaseDelayMs: 10,
    sleep: async (milliseconds) => delays.push(milliseconds),
    log: noLog,
  });
  assert.deepEqual(advisories, {});
  assert.equal(requestCount, 2);
  assert.deepEqual(delays, [10]);
}

// 临时 HTTP 状态会重试，退避时间按尝试次数增长。
{
  let requestCount = 0;
  const delays = [];
  const advisories = await fetchBulkAdvisories(
    { foo: ["1.0.0"] },
    {
      fetchImpl: async () => {
        requestCount += 1;
        return requestCount < 3
          ? jsonResponse(null, 503, "temporarily unavailable")
          : jsonResponse({ foo: [] });
      },
      createTimeoutSignal: noTimeoutSignal,
      maxAttempts: 3,
      retryBaseDelayMs: 10,
      sleep: async (milliseconds) => delays.push(milliseconds),
      log: noLog,
    }
  );
  assert.deepEqual(advisories, { foo: [] });
  assert.equal(requestCount, 3);
  assert.deepEqual(delays, [10, 20]);
}

// 重试耗尽后仍然失败关闭，错误中保留尝试次数和最后原因。
{
  let requestCount = 0;
  await assert.rejects(
    fetchBulkAdvisories(
      { foo: ["1.0.0"] },
      {
        fetchImpl: async () => {
          requestCount += 1;
          throw timeoutError();
        },
        createTimeoutSignal: noTimeoutSignal,
        maxAttempts: 3,
        retryBaseDelayMs: 0,
        sleep: async () => {},
        log: noLog,
      }
    ),
    /bulk advisory request failed after 3 attempts: TimeoutError/
  );
  assert.equal(requestCount, 3);
}

// 永久 HTTP 错误和非法响应结构不应通过重试掩盖。
{
  let requestCount = 0;
  await assert.rejects(
    fetchBulkAdvisories(
      { foo: ["1.0.0"] },
      {
        fetchImpl: async () => {
          requestCount += 1;
          return jsonResponse(null, 400, "bad request");
        },
        createTimeoutSignal: noTimeoutSignal,
        sleep: async () => {},
        log: noLog,
      }
    ),
    /responded with 400: bad request/
  );
  assert.equal(requestCount, 1);

  await assert.rejects(
    fetchBulkAdvisories(
      { foo: ["1.0.0"] },
      {
        fetchImpl: async () => jsonResponse([]),
        createTimeoutSignal: noTimeoutSignal,
        sleep: async () => {},
        log: noLog,
      }
    ),
    /unexpected payload shape/
  );
}

// OSV 回退按精确版本查询，同一漏洞跨版本命中时只为每个包记录一次。
{
  const versionsByName = new Map([
    ["foo", new Set(["1.0.0", "1.0.1"])],
    ["bar", new Set(["2.0.0"])],
  ]);
  let detailRequestCount = 0;
  const advisories = await fetchOsvAdvisories(versionsByName, {
    queryEndpoint: "https://example.test/querybatch",
    vulnerabilityEndpoint: "https://example.test/vulns",
    fetchImpl: async (endpoint, options) => {
      if (endpoint.endsWith("/querybatch")) {
        assert.deepEqual(JSON.parse(options.body), {
          queries: [
            { package: { ecosystem: "npm", name: "foo" }, version: "1.0.0" },
            { package: { ecosystem: "npm", name: "foo" }, version: "1.0.1" },
            { package: { ecosystem: "npm", name: "bar" }, version: "2.0.0" },
          ],
        });
        return jsonResponse({
          results: [{ vulns: [{ id: "GHSA-test" }] }, { vulns: [{ id: "GHSA-test" }] }, {}],
        });
      }

      detailRequestCount += 1;
      assert.equal(endpoint, "https://example.test/vulns/GHSA-test");
      return jsonResponse({
        id: "GHSA-test",
        summary: "test advisory",
        database_specific: { severity: "HIGH" },
        references: [{ type: "ADVISORY", url: "https://example.test/advisory" }],
      });
    },
    createTimeoutSignal: noTimeoutSignal,
    sleep: async () => {},
    log: noLog,
  });
  assert.deepEqual(advisories, {
    foo: [
      {
        severity: "high",
        title: "test advisory",
        url: "https://example.test/advisory",
      },
    ],
  });
  assert.equal(detailRequestCount, 1);
}

// OSV 每批最多提交 1000 个版本，避免超过官方 querybatch 上限。
{
  const versions = new Set(Array.from({ length: 1_001 }, (_value, index) => `1.0.${index}`));
  const batchSizes = [];
  const advisories = await fetchOsvAdvisories(new Map([["foo", versions]]), {
    fetchImpl: async (_endpoint, options) => {
      const request = JSON.parse(options.body);
      batchSizes.push(request.queries.length);
      return jsonResponse({ results: request.queries.map(() => ({})) });
    },
    createTimeoutSignal: noTimeoutSignal,
    sleep: async () => {},
    log: noLog,
  });
  assert.deepEqual(advisories, {});
  assert.deepEqual(batchSizes, [1_000, 1]);
}

// OSV 命中但没有可验证的分类严重级别时必须失败关闭。
{
  await assert.rejects(
    fetchOsvAdvisories(new Map([["foo", new Set(["1.0.0"])]]), {
      fetchImpl: async (endpoint) =>
        endpoint.endsWith("/querybatch")
          ? jsonResponse({ results: [{ vulns: [{ id: "OSV-no-severity" }] }] })
          : jsonResponse({ id: "OSV-no-severity", severity: [] }),
      createTimeoutSignal: noTimeoutSignal,
      sleep: async () => {},
      log: noLog,
    }),
    /has no supported categorical severity/
  );
}

// npm 源不可用时回退到 OSV；两个来源都失败时仍然阻断。
{
  const versionsByName = new Map([["foo", new Set(["1.0.0"])]]);
  let fallbackCount = 0;
  const advisories = await queryAdvisories(versionsByName, {
    fetchNpm: async (requestBody) => {
      assert.deepEqual(requestBody, { foo: ["1.0.0"] });
      throw timeoutError();
    },
    fetchOsv: async (versions) => {
      fallbackCount += 1;
      assert.equal(versions, versionsByName);
      return { foo: [{ severity: "low" }] };
    },
    log: noLog,
  });
  assert.deepEqual(advisories, { foo: [{ severity: "low" }] });
  assert.equal(fallbackCount, 1);

  await assert.rejects(
    queryAdvisories(versionsByName, {
      fetchNpm: async () => {
        throw new Error("npm unavailable");
      },
      fetchOsv: async () => {
        throw new Error("OSV unavailable");
      },
      log: noLog,
    }),
    (error) => {
      assert.equal(error instanceof AggregateError, true);
      assert.equal(error.errors.length, 2);
      assert.match(error.message, /均失败，拒绝 fail-open/);
      return true;
    }
  );
}

console.error("[pnpm-audit:selftest] 全部断言通过。");
