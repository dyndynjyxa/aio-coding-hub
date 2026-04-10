import type { MouseEvent } from "react";
import type { ClaudeModelValidationResult } from "../../services/claude/claudeModelValidation";
import { getClaudeValidationTemplate } from "../../services/claude/claudeValidationTemplates";
import type { SuiteMeta, ClaudeValidationGrade } from "./types";

// ---------------------------------------------------------------------------
// Generic helpers
// ---------------------------------------------------------------------------

/** Prevent event bubbling inside `<details>` / `<summary>` toggles. */
export function stopDetailsToggle(e: MouseEvent) {
  e.preventDefault();
  e.stopPropagation();
}

/** Map evidence grade level to Tailwind CSS classes. */
export function gradeColorClass(level: ClaudeValidationGrade["level"]): string {
  switch (level) {
    case "A":
      return "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400";
    case "B":
      return "bg-indigo-100 text-indigo-700 dark:bg-indigo-900/30 dark:text-indigo-400";
    case "C":
      return "bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-200";
    default:
      return "bg-rose-100 text-rose-700 dark:bg-rose-900/30 dark:text-rose-400";
  }
}

export function isPlainObject(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

export function parseJsonObjectSafe(text: string): Record<string, unknown> | null {
  const raw = text.trim();
  if (!raw) return null;
  try {
    const obj = JSON.parse(raw);
    return isPlainObject(obj) ? obj : null;
  } catch {
    return null;
  }
}

export function normalizeNonEmptyString(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

export function normalizePositiveInt(value: unknown): number | null {
  if (typeof value !== "number" || !Number.isFinite(value)) return null;
  const int = Math.floor(value);
  return int > 0 ? int : null;
}

// ---------------------------------------------------------------------------
// Suite-meta extraction
// ---------------------------------------------------------------------------

export function extractSuiteMetaFromRequestJson(requestJson: string): SuiteMeta {
  const obj = parseJsonObjectSafe(requestJson);
  if (!obj) return { suiteRunId: null, suiteStepIndex: null, suiteStepTotal: null };
  return {
    suiteRunId: normalizeNonEmptyString(obj.suite_run_id),
    suiteStepIndex: normalizePositiveInt(obj.suite_step_index),
    suiteStepTotal: normalizePositiveInt(obj.suite_step_total),
  };
}

export function getHistoryGroupKey(run: { id: number; request_json: string }): string {
  const meta = extractSuiteMetaFromRequestJson(run.request_json ?? "");
  if (meta.suiteRunId) return `suite:${meta.suiteRunId}`;
  return `run:${run.id}`;
}

// ---------------------------------------------------------------------------
// JSON / text helpers
// ---------------------------------------------------------------------------

export function parseClaudeModelValidationResultJson(
  text: string
): ClaudeModelValidationResult | null {
  const raw = text.trim();
  if (!raw) return null;
  try {
    const obj = JSON.parse(raw);
    if (!obj || typeof obj !== "object") return null;
    return obj as ClaudeModelValidationResult;
  } catch {
    return null;
  }
}

export function prettyJsonOrFallback(text: string): string {
  const raw = text.trim();
  if (!raw) return "";
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}

export function firstLine(text: string) {
  const t = text.trim();
  if (!t) return "";
  const idx = t.indexOf("\n");
  return idx >= 0 ? t.slice(0, idx).trim() : t;
}

export function truncateText(text: string, max = 120) {
  if (!text) return "";
  if (text.length <= max) return text;
  return `${text.slice(0, max)}...`;
}

// ---------------------------------------------------------------------------
// Template display helpers
// ---------------------------------------------------------------------------

type ClaudeValidationTemplateView = ReturnType<typeof getClaudeValidationTemplate>;

export function getTemplateDisplayLabel(template: ClaudeValidationTemplateView): string {
  const summary = typeof template.summary === "string" ? template.summary.trim() : "";
  return summary || template.label;
}

export function getTemplateDisplayTitle(template: ClaudeValidationTemplateView): string {
  const channel = typeof template.channelLabel === "string" ? template.channelLabel.trim() : "";
  const label = getTemplateDisplayLabel(template);
  return channel ? `${channel} \u00b7 ${label}` : label;
}

// ---------------------------------------------------------------------------
// Suite grade / aggregation helpers
// ---------------------------------------------------------------------------

export function suitePickEvidenceGrade(
  grades: Array<ClaudeValidationGrade | null | undefined>
): ClaudeValidationGrade | null {
  const order: Record<ClaudeValidationGrade["level"], number> = { A: 0, B: 1, C: 2, D: 3 };
  const normalized = grades.filter((g): g is ClaudeValidationGrade => Boolean(g));
  if (normalized.length === 0) return null;

  const evidence = normalized.filter(
    (g) => g.label !== "\u901a\u8fc7" && g.label !== "\u672a\u901a\u8fc7"
  );
  const candidates = evidence.length > 0 ? evidence : normalized;

  const anyD = candidates.find((g) => g.level === "D");
  if (anyD) return anyD;

  return candidates.reduce((best, cur) => (order[cur.level] < order[best.level] ? cur : best));
}

export function suiteAggregateOk(values: Array<boolean | null | undefined>): boolean | null {
  if (values.length === 0) return null;
  let hasUnknown = false;
  for (const v of values) {
    if (v === false) return false;
    if (v == null) hasUnknown = true;
  }
  return hasUnknown ? null : true;
}

/** Preset model options (fixed list, allows free input) */
export const PRESET_MODEL_OPTIONS = [
  "claude-sonnet-4-6",
  "claude-sonnet-4-5-20250929",
  "claude-opus-4-6",
  "claude-opus-4-5-20251101",
] as const;

export const DEFAULT_MODEL = "claude-sonnet-4-6";

// Re-export from split modules for backward compatibility
export { buildSuiteProtocolItems } from "./suiteProtocol";
export {
  suiteTemplateRequiresFlag,
  suiteSignalString,
  suiteSignalBool,
  suiteIsSseParseMode,
  suiteTemplateWantsSignatureTamper,
} from "./suiteProtocol";
export { buildSuiteSummary } from "./suiteSummary";
