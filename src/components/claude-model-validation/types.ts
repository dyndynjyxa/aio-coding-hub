import type { ClaudeModelValidationResult } from "../../services/claude/claudeModelValidation";
import type { ClaudeModelValidationRunRow } from "../../services/claude/claudeModelValidationHistory";
import type { ClaudeValidationTemplateKey } from "../../services/claude/claudeValidationTemplates";
import type { evaluateClaudeValidation } from "../../services/claude/claudeValidationTemplates";

export type ClaudeModelValidationDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  provider: import("../../services/providers/providers").ProviderSummary | null;
};

export type SuiteMeta = {
  suiteRunId: string | null;
  suiteStepIndex: number | null;
  suiteStepTotal: number | null;
};

export type ClaudeModelValidationRunView = ClaudeModelValidationRunRow & {
  parsed_result: ClaudeModelValidationResult | null;
};

export type ClaudeValidationSuiteStep = {
  index: number;
  templateKey: ClaudeValidationTemplateKey;
  label: string;
  status: "pending" | "running" | "done" | "error";
  request_json: string;
  result_json: string;
  result: ClaudeModelValidationResult | null;
  error: string | null;
};

export type SuiteStepView = {
  index: number;
  total: number;
  templateKey: ClaudeValidationTemplateKey;
  label: string;
  status: "pending" | "running" | "done" | "error" | "missing";
  evaluation: ReturnType<typeof evaluateClaudeValidation>;
  result: ClaudeModelValidationResult | null;
  requestJsonText: string;
  resultJsonText: string;
  sseRawText: string;
  errorText: string | null;
};

export type ClaudeValidationGrade = NonNullable<
  ReturnType<typeof evaluateClaudeValidation>["grade"]
>;

export type SuiteSummaryRow = {
  templateKey: ClaudeValidationTemplateKey;
  label: string;
  status: "pending" | "running" | "done" | "error" | "missing";
  evaluation: ReturnType<typeof evaluateClaudeValidation>;
  result: ClaudeModelValidationResult | null;
  errorText: string | null;
};

export type SuiteProtocolItem = {
  key: string;
  label: string;
  ok: boolean | null;
  required: boolean;
  detail: string | null;
};

export type SuiteSummary = {
  overallPass: boolean | null;
  isRunning: boolean;
  modelName: string;
  stats: {
    total: number;
    done: number;
    pass: number;
    fail: number;
    error: number;
    missing: number;
  };
  grade: ClaudeValidationGrade | null;
  templateRows: Array<{
    templateKey: ClaudeValidationTemplateKey;
    label: string;
    status: SuiteSummaryRow["status"];
    overallPass: boolean | null;
    grade: ClaudeValidationGrade | null;
  }>;
  protocol: SuiteProtocolItem[];
  issues: Array<{ kind: "error" | "warn"; title: string; detail: string | null }>;
  plainText: string;
};

export type ValidationDetailsTab = "overview" | "steps" | "debug";

export type ClaudeModelValidationHistoryGroup = {
  key: string;
  suiteRunId: string | null;
  isSuite: boolean;
  createdAt: number;
  latestRunId: number;
  expectedTotal: number;
  missingCount: number;
  passCount: number;
  failCount: number;
  overallPass: boolean;
  grade: ClaudeValidationGrade | null;
  modelName: string;
  runs: Array<{
    run: ClaudeModelValidationRunView;
    meta: SuiteMeta;
    evaluation: ReturnType<typeof evaluateClaudeValidation>;
  }>;
};
