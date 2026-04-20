import { spawnSync } from "node:child_process";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

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

function parseAuditPayload(stdout, stderr) {
  const combinedOutput = [stdout, stderr].filter(Boolean).join("\n").trim();
  if (combinedOutput.length === 0) {
    throw new Error("pnpm audit produced no output.");
  }

  try {
    return JSON.parse(combinedOutput);
  } catch {
    // Fall back to scanning for the last valid JSON line when pnpm mixes logs with JSON.
  }

  const trimmedLines = combinedOutput
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);

  for (let index = trimmedLines.length - 1; index >= 0; index -= 1) {
    try {
      return JSON.parse(trimmedLines[index]);
    } catch {
      // Keep scanning for the last valid JSON line.
    }
  }

  return JSON.parse(combinedOutput);
}

function extractSeverityCounts(payload) {
  const counts = {
    info: 0,
    low: 0,
    moderate: 0,
    high: 0,
    critical: 0,
  };

  const metadataVulnerabilities =
    payload &&
    typeof payload === "object" &&
    payload.metadata &&
    typeof payload.metadata === "object" &&
    payload.metadata.vulnerabilities &&
    typeof payload.metadata.vulnerabilities === "object"
      ? payload.metadata.vulnerabilities
      : null;

  if (metadataVulnerabilities) {
    for (const severity of Object.keys(counts)) {
      counts[severity] = Number(metadataVulnerabilities[severity] ?? 0);
    }
    return counts;
  }

  const advisories =
    payload &&
    typeof payload === "object" &&
    payload.advisories &&
    typeof payload.advisories === "object"
      ? Object.values(payload.advisories)
      : [];

  for (const advisory of advisories) {
    if (!advisory || typeof advisory !== "object") {
      continue;
    }

    const severity = typeof advisory.severity === "string" ? advisory.severity.toLowerCase() : "";
    if (severity in counts) {
      counts[severity] += 1;
    }
  }

  return counts;
}

function hasBlockingVulnerabilities(counts) {
  return BLOCKING_SEVERITIES.some((severity) => counts[severity] > 0);
}

function formatCounts(counts) {
  return Object.entries(counts)
    .map(([severity, count]) => `${severity}=${count}`)
    .join(", ");
}

function main() {
  /*
   * ============================================================================
   * 步骤1：执行 fail-close 的 pnpm audit
   * ============================================================================
   * 目标：
   *   1) 只把 high / critical 视为阻断阈值
   *   2) 任何网络异常、命令异常、输出异常都按失败处理
   * 数据源：
   *   1) pnpm audit --json 输出
   *   2) pnpm 进程退出状态
   * 操作要点：
   *   1) 退出码不能直接当成唯一判断，因为 pnpm 会对低危漏洞也返回非零
   *   2) 只有在 JSON 可解析且 blocking 计数为 0 时才允许通过
   */
  logger.info("[pnpm-audit] 开始执行依赖审计...");

  // 1.1 运行 pnpm audit，并捕获 stdout / stderr 供后续解析
  const result = spawnSync("pnpm", ["audit", "--audit-level=high", "--json"], {
    cwd: repoRoot,
    encoding: "utf8",
    env: process.env,
  });

  if (result.error) {
    logger.error(result.stderr || result.stdout || "");
    throw result.error;
  }
  if (result.signal) {
    throw new Error(`pnpm audit terminated by signal: ${result.signal}`);
  }

  // 1.2 解析 JSON 输出，并提取各级别漏洞计数
  const payload = parseAuditPayload(result.stdout, result.stderr);
  if (payload && typeof payload === "object" && "error" in payload && payload.error) {
    throw new Error(`pnpm audit returned an error payload: ${JSON.stringify(payload.error)}`);
  }

  const counts = extractSeverityCounts(payload);
  logger.info("[pnpm-audit] 审计结果：%s", formatCounts(counts));

  // 1.3 只要出现 blocking 漏洞，或命令状态异常且无法归因于低危漏洞，就直接失败
  if (hasBlockingVulnerabilities(counts)) {
    throw new Error("[pnpm-audit] Detected blocking vulnerabilities (high/critical).");
  }

  if (result.status !== 0 && counts.low === 0 && counts.moderate === 0 && counts.info === 0) {
    throw new Error(
      `[pnpm-audit] pnpm audit exited with status ${result.status}, refusing to fail-open.`
    );
  }

  logger.info("[pnpm-audit] 依赖审计通过。");
}

main();
