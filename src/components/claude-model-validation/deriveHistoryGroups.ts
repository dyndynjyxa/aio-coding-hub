import {
  evaluateClaudeValidation,
  extractTemplateKeyFromRequestJson,
} from "../../services/claude/claudeValidationTemplates";
import type { ClaudeModelValidationRunView, ClaudeModelValidationHistoryGroup } from "./types";
import {
  extractSuiteMetaFromRequestJson,
  getHistoryGroupKey,
  suitePickEvidenceGrade,
} from "./helpers";

/**
 * Pure function: transform flat history runs into grouped history entries.
 */
export function deriveHistoryGroups(
  historyRuns: ClaudeModelValidationRunView[]
): ClaudeModelValidationHistoryGroup[] {
  const groups = new Map<
    string,
    {
      key: string;
      suiteRunId: string | null;
      createdAt: number;
      latestRunId: number;
      runs: Array<{
        run: ClaudeModelValidationRunView;
        meta: {
          suiteRunId: string | null;
          suiteStepIndex: number | null;
          suiteStepTotal: number | null;
        };
        templateKeyLike: string | null;
      }>;
    }
  >();

  for (const run of historyRuns) {
    const meta = extractSuiteMetaFromRequestJson(run.request_json ?? "");
    const groupKey = getHistoryGroupKey(run);
    const existing = groups.get(groupKey);
    const next = existing ?? {
      key: groupKey,
      suiteRunId: meta.suiteRunId,
      createdAt: run.created_at,
      latestRunId: run.id,
      runs: [],
    };

    next.suiteRunId = next.suiteRunId ?? meta.suiteRunId;
    next.createdAt = Math.max(next.createdAt, run.created_at);
    next.latestRunId = Math.max(next.latestRunId, run.id);
    next.runs.push({
      run,
      meta,
      templateKeyLike: extractTemplateKeyFromRequestJson(run.request_json ?? ""),
    });

    groups.set(groupKey, next);
  }

  const out: ClaudeModelValidationHistoryGroup[] = [];
  for (const group of groups.values()) {
    const sortedRuns = [...group.runs].sort((a, b) => {
      const ia = a.meta.suiteStepIndex ?? Number.MAX_SAFE_INTEGER;
      const ib = b.meta.suiteStepIndex ?? Number.MAX_SAFE_INTEGER;
      if (ia !== ib) return ia - ib;
      return a.run.id - b.run.id;
    });

    const expectedTotal = (() => {
      const totals = sortedRuns
        .map((r) => r.meta.suiteStepTotal)
        .filter((v): v is number => typeof v === "number" && Number.isFinite(v) && v > 0);
      if (totals.length > 0) return Math.max(...totals);
      return sortedRuns.length;
    })();

    const evaluatedRuns = sortedRuns.map((r) => ({
      run: r.run,
      meta: r.meta,
      evaluation: evaluateClaudeValidation(r.templateKeyLike, r.run.parsed_result),
    }));

    const passCount = evaluatedRuns.filter((r) => r.evaluation.overallPass === true).length;
    const failCount = Math.max(0, evaluatedRuns.length - passCount);
    const grade = suitePickEvidenceGrade(evaluatedRuns.map((r) => r.evaluation.grade));
    const allPass =
      expectedTotal === evaluatedRuns.length &&
      evaluatedRuns.every((r) => r.evaluation.overallPass === true);

    const modelName =
      evaluatedRuns[evaluatedRuns.length - 1]?.evaluation.derived.modelName ??
      evaluatedRuns[0]?.evaluation.derived.modelName ??
      "\u2014";

    out.push({
      key: group.key,
      suiteRunId: group.suiteRunId,
      isSuite: Boolean(group.suiteRunId),
      createdAt: group.createdAt,
      latestRunId: group.latestRunId,
      expectedTotal,
      missingCount: Math.max(0, expectedTotal - evaluatedRuns.length),
      passCount,
      failCount,
      overallPass: allPass,
      grade,
      modelName,
      runs: evaluatedRuns,
    });
  }

  return out.sort((a, b) => b.latestRunId - a.latestRunId);
}
