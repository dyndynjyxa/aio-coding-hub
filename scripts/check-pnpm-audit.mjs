// npm 已于 2026-07 下线旧审计端点 /-/npm/v1/security/audits(/quick)，一律返回 410，
// pnpm 10.x 的 `pnpm audit` 因此不可用（修复只在 pnpm 11，大版本升级另行处理）。
// 这里改为直连 npm CLI / pnpm 11 使用的 bulk advisory 端点：提交 name -> versions 清单，
// 注册表按提交的版本过滤并返回命中的 advisory；端点不可用时回退到 OSV 精确版本查询。
// ponytail: 端点写死 registry.npmjs.org；如迁移私有 registry，需要改为从配置推导。
import { spawnSync } from "node:child_process";
import { dirname } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const logger = {
  info(message, ...args) {
    console.error(message, ...args);
  },
  error(message, ...args) {
    console.error(message, ...args);
  },
};

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = dirname(scriptDir);
const BLOCKING_SEVERITIES = Object.freeze(["high", "critical"]);
const KNOWN_SEVERITIES = Object.freeze(["info", "low", "moderate", ...BLOCKING_SEVERITIES]);
const BULK_ADVISORY_ENDPOINT = "https://registry.npmjs.org/-/npm/v1/security/advisories/bulk";
const OSV_QUERY_BATCH_ENDPOINT = "https://api.osv.dev/v1/querybatch";
const OSV_VULNERABILITY_ENDPOINT = "https://api.osv.dev/v1/vulns";
const OSV_QUERY_BATCH_SIZE = 1_000;
const AUDIT_REQUEST_TIMEOUT_MS = 30_000;
const AUDIT_REQUEST_MAX_ATTEMPTS = 2;
const AUDIT_RETRY_BASE_DELAY_MS = 2_000;
const RETRYABLE_HTTP_STATUSES = new Set([408, 425, 429]);

class AuditHttpError extends Error {
  constructor(label, status, detail) {
    super(`[pnpm-audit] ${label} endpoint responded with ${status}: ${detail}`);
    this.name = "AuditHttpError";
    this.retryable = RETRYABLE_HTTP_STATUSES.has(status) || (status >= 500 && status < 600);
  }
}

function isRetryableRequestError(error) {
  return (
    error?.retryable === true ||
    error?.name === "TimeoutError" ||
    error?.name === "AbortError" ||
    error instanceof TypeError
  );
}

function formatError(error) {
  return error instanceof Error ? `${error.name}: ${error.message}` : String(error);
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

async function fetchJsonWithRetry(
  { endpoint, label, requestInit, validatePayload },
  {
    fetchImpl = globalThis.fetch,
    createTimeoutSignal = (timeoutMs) => AbortSignal.timeout(timeoutMs),
    timeoutMs = AUDIT_REQUEST_TIMEOUT_MS,
    maxAttempts = AUDIT_REQUEST_MAX_ATTEMPTS,
    retryBaseDelayMs = AUDIT_RETRY_BASE_DELAY_MS,
    sleep = delay,
    log = (...args) => logger.info(...args),
  } = {}
) {
  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    try {
      const response = await fetchImpl(endpoint, {
        ...requestInit,
        signal: createTimeoutSignal(timeoutMs),
      });

      if (!response.ok) {
        const detail = (await response.text()).slice(0, 500);
        throw new AuditHttpError(label, response.status, detail);
      }

      const payload = await response.json();
      validatePayload(payload);
      return payload;
    } catch (error) {
      if (!isRetryableRequestError(error) || attempt === maxAttempts) {
        if (attempt === 1) {
          throw error;
        }
        throw new Error(
          `[pnpm-audit] ${label} request failed after ${attempt} attempts: ${formatError(error)}`,
          { cause: error }
        );
      }

      const retryDelayMs = retryBaseDelayMs * 2 ** (attempt - 1);
      log(
        "[pnpm-audit] %s 请求第 %d/%d 次失败（%s），%dms 后重试...",
        label,
        attempt,
        maxAttempts,
        formatError(error),
        retryDelayMs
      );
      await sleep(retryDelayMs);
    }
  }

  throw new Error(`[pnpm-audit] ${label} request exhausted without a result.`);
}

export async function fetchBulkAdvisories(
  requestBody,
  { endpoint = BULK_ADVISORY_ENDPOINT, ...requestOptions } = {}
) {
  return fetchJsonWithRetry(
    {
      endpoint,
      label: "bulk advisory",
      requestInit: {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(requestBody),
      },
      validatePayload(payload) {
        if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
          throw new Error(
            "[pnpm-audit] bulk advisory endpoint returned an unexpected payload shape."
          );
        }
      },
    },
    requestOptions
  );
}

function getOsvSeverity(vulnerability) {
  const candidates = [
    vulnerability?.database_specific?.severity,
    ...(Array.isArray(vulnerability?.affected)
      ? vulnerability.affected.flatMap((affected) => [
          affected?.ecosystem_specific?.severity,
          affected?.database_specific?.severity,
        ])
      : []),
  ];
  const severities = candidates
    .filter((severity) => typeof severity === "string")
    .map((severity) => severity.toLowerCase())
    .filter((severity) => KNOWN_SEVERITIES.includes(severity));

  return severities.sort(
    (left, right) => KNOWN_SEVERITIES.indexOf(right) - KNOWN_SEVERITIES.indexOf(left)
  )[0];
}

export async function fetchOsvAdvisories(
  versionsByName,
  {
    queryEndpoint = OSV_QUERY_BATCH_ENDPOINT,
    vulnerabilityEndpoint = OSV_VULNERABILITY_ENDPOINT,
    ...requestOptions
  } = {}
) {
  const packageVersions = [...versionsByName].flatMap(([name, versions]) =>
    [...versions].map((version) => ({ name, version }))
  );
  const packagesByVulnerabilityId = new Map();

  for (let start = 0; start < packageVersions.length; start += OSV_QUERY_BATCH_SIZE) {
    const batch = packageVersions.slice(start, start + OSV_QUERY_BATCH_SIZE);
    const payload = await fetchJsonWithRetry(
      {
        endpoint: queryEndpoint,
        label: "OSV querybatch",
        requestInit: {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({
            queries: batch.map(({ name, version }) => ({
              package: { ecosystem: "npm", name },
              version,
            })),
          }),
        },
        validatePayload(value) {
          if (!value || !Array.isArray(value.results) || value.results.length !== batch.length) {
            throw new Error("[pnpm-audit] OSV querybatch returned an unexpected payload shape.");
          }
        },
      },
      requestOptions
    );

    payload.results.forEach((result, index) => {
      if (!result || typeof result !== "object" || Array.isArray(result)) {
        throw new Error("[pnpm-audit] OSV querybatch returned an invalid result entry.");
      }
      if (result.next_page_token) {
        throw new Error("[pnpm-audit] OSV querybatch result requires unsupported pagination.");
      }
      if (result.vulns !== undefined && !Array.isArray(result.vulns)) {
        throw new Error("[pnpm-audit] OSV querybatch returned an invalid vulnerability list.");
      }

      for (const vulnerability of result.vulns ?? []) {
        if (!vulnerability || typeof vulnerability.id !== "string" || !vulnerability.id) {
          throw new Error("[pnpm-audit] OSV querybatch returned a vulnerability without an id.");
        }
        const packageNames = packagesByVulnerabilityId.get(vulnerability.id) ?? new Set();
        packageNames.add(batch[index].name);
        packagesByVulnerabilityId.set(vulnerability.id, packageNames);
      }
    });
  }

  const advisoriesByPackage = {};
  for (const [id, packageNames] of packagesByVulnerabilityId) {
    const vulnerability = await fetchJsonWithRetry(
      {
        endpoint: `${vulnerabilityEndpoint}/${encodeURIComponent(id)}`,
        label: `OSV vulnerability ${id}`,
        requestInit: { method: "GET", headers: { accept: "application/json" } },
        validatePayload(value) {
          if (!value || typeof value !== "object" || value.id !== id) {
            throw new Error(`[pnpm-audit] OSV returned invalid details for ${id}.`);
          }
        },
      },
      requestOptions
    );

    if (vulnerability.withdrawn) {
      continue;
    }
    const severity = getOsvSeverity(vulnerability);
    if (!severity) {
      throw new Error(
        `[pnpm-audit] OSV vulnerability ${id} has no supported categorical severity.`
      );
    }
    const advisory = {
      severity,
      title: vulnerability.summary ?? id,
      url:
        vulnerability.references?.find((reference) => reference?.type === "ADVISORY")?.url ??
        `https://osv.dev/vulnerability/${encodeURIComponent(id)}`,
    };
    for (const packageName of packageNames) {
      const advisories = advisoriesByPackage[packageName] ?? [];
      advisories.push(advisory);
      advisoriesByPackage[packageName] = advisories;
    }
  }

  return advisoriesByPackage;
}

export async function queryAdvisories(
  versionsByName,
  {
    fetchNpm = (requestBody) => fetchBulkAdvisories(requestBody),
    fetchOsv = (versions) => fetchOsvAdvisories(versions),
    log = (...args) => logger.info(...args),
  } = {}
) {
  const requestBody = Object.fromEntries(
    [...versionsByName].map(([name, versions]) => [name, [...versions]])
  );
  try {
    return await fetchNpm(requestBody);
  } catch (npmError) {
    log(
      "[pnpm-audit] npm bulk advisory 不可用（%s），回退到 OSV 精确版本审计...",
      formatError(npmError)
    );
    try {
      return await fetchOsv(versionsByName);
    } catch (osvError) {
      throw new AggregateError(
        [npmError, osvError],
        "[pnpm-audit] npm bulk advisory 与 OSV 审计均失败，拒绝 fail-open。"
      );
    }
  }
}

export function collectPackageVersions(projects) {
  const versionsByName = new Map();

  const visit = (dependencies) => {
    if (!dependencies || typeof dependencies !== "object") {
      return;
    }
    for (const [name, node] of Object.entries(dependencies)) {
      if (!node || typeof node !== "object") {
        continue;
      }
      // 只上报 registry 版本；跳过 link: / file: / workspace: 等本地依赖。
      if (typeof node.version === "string" && /^\d/.test(node.version)) {
        const versions = versionsByName.get(name) ?? new Set();
        versions.add(node.version);
        versionsByName.set(name, versions);
      }
      visit(node.dependencies);
      visit(node.optionalDependencies);
    }
  };

  for (const project of Array.isArray(projects) ? projects : []) {
    if (!project || typeof project !== "object") {
      continue;
    }
    visit(project.dependencies);
    visit(project.optionalDependencies);
  }

  return versionsByName;
}

export function extractSeverityCounts(advisoriesByPackage) {
  const counts = {
    info: 0,
    low: 0,
    moderate: 0,
    high: 0,
    critical: 0,
  };

  for (const advisories of Object.values(advisoriesByPackage)) {
    if (!Array.isArray(advisories)) {
      continue;
    }
    for (const advisory of advisories) {
      if (!advisory || typeof advisory !== "object") {
        continue;
      }
      const severity = typeof advisory.severity === "string" ? advisory.severity.toLowerCase() : "";
      if (severity in counts) {
        counts[severity] += 1;
      }
    }
  }

  return counts;
}

export function hasBlockingVulnerabilities(counts) {
  return BLOCKING_SEVERITIES.some((severity) => counts[severity] > 0);
}

export function formatCounts(counts) {
  return Object.entries(counts)
    .map(([severity, count]) => `${severity}=${count}`)
    .join(", ");
}

export function formatBlockingAdvisories(advisoriesByPackage) {
  const lines = [];
  for (const [name, advisories] of Object.entries(advisoriesByPackage)) {
    if (!Array.isArray(advisories)) {
      continue;
    }
    for (const advisory of advisories) {
      if (!advisory || typeof advisory !== "object") {
        continue;
      }
      const severity = typeof advisory.severity === "string" ? advisory.severity.toLowerCase() : "";
      if (BLOCKING_SEVERITIES.includes(severity)) {
        lines.push(
          `[pnpm-audit] ${severity}: ${name} — ${advisory.title ?? "untitled advisory"} (${advisory.url ?? "no url"})`
        );
      }
    }
  }
  return lines;
}

async function main() {
  /*
   * ============================================================================
   * 步骤1：执行 fail-close 的依赖审计（bulk advisory 端点）
   * ============================================================================
   * 目标：
   *   1) 只把 high / critical 视为阻断阈值
   *   2) 任何网络异常、命令异常、输出异常都按失败处理
   * 数据源：
   *   1) pnpm list -r --prod --depth Infinity --json（与旧 `pnpm audit --prod` 的
   *      workspace 生产依赖范围一致）
   *   2) npm bulk advisory 端点返回的 advisory 列表；端点不可用时使用 OSV API
   * 操作要点：
   *   1) 注册表按提交的版本做服务端过滤，返回即命中，无需本地 semver 比对
   *   2) 只有在响应可解析且 blocking 计数为 0 时才允许通过
   */
  logger.info("[pnpm-audit] 开始执行依赖审计...");

  // 1.1 枚举 workspace 全部生产依赖
  const result = spawnSync("pnpm", ["list", "-r", "--prod", "--depth", "Infinity", "--json"], {
    cwd: repoRoot,
    encoding: "utf8",
    env: process.env,
    maxBuffer: 64 * 1024 * 1024,
  });

  if (result.error) {
    logger.error(result.stderr || result.stdout || "");
    throw result.error;
  }
  if (result.signal) {
    throw new Error(`pnpm list terminated by signal: ${result.signal}`);
  }
  if (result.status !== 0) {
    logger.error(result.stderr || result.stdout || "");
    throw new Error(
      `[pnpm-audit] pnpm list exited with status ${result.status}, refusing to fail-open.`
    );
  }

  const versionsByName = collectPackageVersions(JSON.parse(result.stdout));
  if (versionsByName.size === 0) {
    throw new Error("[pnpm-audit] pnpm list produced no auditable packages.");
  }
  logger.info("[pnpm-audit] 待审计包数量：%d", versionsByName.size);

  // 1.2 查询 npm advisory，端点不可用时回退到 OSV
  const advisoriesByPackage = await queryAdvisories(versionsByName);

  // 1.3 统计各级别命中数，只要出现 blocking 漏洞就直接失败
  const counts = extractSeverityCounts(advisoriesByPackage);
  logger.info("[pnpm-audit] 审计结果：%s", formatCounts(counts));

  if (hasBlockingVulnerabilities(counts)) {
    for (const line of formatBlockingAdvisories(advisoriesByPackage)) {
      logger.error(line);
    }
    throw new Error("[pnpm-audit] Detected blocking vulnerabilities (high/critical).");
  }

  logger.info("[pnpm-audit] 依赖审计通过。");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main();
}
