import type { ClaudeModelValidationResult } from "../../services/claude/claudeModelValidation";
import { evaluateClaudeValidation } from "../../services/claude/claudeValidationTemplates";
import type { SuiteSummaryRow, SuiteProtocolItem } from "./types";
import {
  isPlainObject,
  normalizeNonEmptyString,
  truncateText,
  firstLine,
  suiteAggregateOk,
} from "./helpers";

// ---------------------------------------------------------------------------
// Suite template flag / signal readers
// ---------------------------------------------------------------------------

export function suiteTemplateRequiresFlag(
  evaluation: ReturnType<typeof evaluateClaudeValidation>,
  flagKey: string
): boolean {
  const obj = evaluation.template.evaluation as unknown;
  if (!isPlainObject(obj)) return false;
  return (obj as Record<string, unknown>)[flagKey] === true;
}

export function suiteSignalString(
  result: ClaudeModelValidationResult | null,
  key: string
): string | null {
  if (!result) return null;
  const v = isPlainObject(result.signals) ? (result.signals as Record<string, unknown>)[key] : null;
  return normalizeNonEmptyString(v);
}

export function suiteSignalBool(
  result: ClaudeModelValidationResult | null,
  key: string
): boolean | null {
  if (!result) return null;
  const v = isPlainObject(result.signals) ? (result.signals as Record<string, unknown>)[key] : null;
  return typeof v === "boolean" ? v : null;
}

export function suiteIsSseParseMode(mode: string | null): boolean | null {
  if (!mode) return null;
  return mode === "sse" || mode === "sse_fallback";
}

export function suiteTemplateWantsSignatureTamper(
  evaluation: ReturnType<typeof evaluateClaudeValidation>
) {
  const req = evaluation.template.request as unknown;
  if (!isPlainObject(req)) return false;
  const roundtrip = (req as Record<string, unknown>).roundtrip as unknown;
  if (!isPlainObject(roundtrip)) return false;
  return roundtrip.kind === "signature" && roundtrip.enable_tamper === true;
}

// ---------------------------------------------------------------------------
// buildSuiteProtocolItems
// ---------------------------------------------------------------------------

export function buildSuiteProtocolItems(rows: SuiteSummaryRow[]): SuiteProtocolItem[] {
  const doneRows = rows.filter((r) => r.status === "done");

  const reverseProxy = (() => {
    const values = doneRows
      .map((r) => r.evaluation.checks.reverseProxy?.ok)
      .filter((v) => v !== undefined);
    const ok = suiteAggregateOk(values);
    const failing = doneRows.find((r) => r.evaluation.checks.reverseProxy?.ok === false);
    return {
      key: "reverse_proxy",
      label: "\u9006\u5411/\u53cd\u4ee3\u5173\u952e\u8bcd\uff08\u9ad8\u98ce\u9669\uff09",
      ok,
      required: doneRows.length > 0,
      detail: failing
        ? truncateText(firstLine(failing.evaluation.checks.reverseProxy?.title ?? ""))
        : null,
    } satisfies SuiteProtocolItem;
  })();

  const requestOk = (() => {
    const values = doneRows.map((r) => r.result?.ok).filter((v) => v !== undefined);
    const ok = suiteAggregateOk(values);
    const failing = doneRows.find((r) => r.result?.ok === false);
    return {
      key: "request_ok",
      label: "\u8bf7\u6c42\u6210\u529f\uff08ok=true\uff09",
      ok,
      required: doneRows.length > 0,
      detail: failing
        ? `status=${failing.result?.status ?? "\u2014"}; error=${truncateText(String(failing.result?.error ?? "\u2014"), 120)}`
        : null,
    } satisfies SuiteProtocolItem;
  })();

  const sseParse = (() => {
    const values = doneRows.map((r) =>
      suiteIsSseParseMode(suiteSignalString(r.result, "response_parse_mode"))
    );
    const ok = suiteAggregateOk(values);
    const modes = [
      ...new Set(
        doneRows.map((r) => suiteSignalString(r.result, "response_parse_mode")).filter(Boolean)
      ),
    ];
    return {
      key: "sse_parse_mode",
      label: "SSE \u6d41\u5f0f\u89e3\u6790\uff08response_parse_mode=sse\uff09",
      ok,
      required: doneRows.length > 0,
      detail: modes.length > 0 ? `parse_mode=${modes.join(", ")}` : null,
    } satisfies SuiteProtocolItem;
  })();

  const streamRead = (() => {
    const values = doneRows.map((r) => {
      const hasErr = suiteSignalBool(r.result, "stream_read_error");
      if (hasErr == null) return null;
      return !hasErr;
    });
    const ok = suiteAggregateOk(values);
    const failing = doneRows.find((r) => suiteSignalBool(r.result, "stream_read_error") === true);
    const msg = failing ? suiteSignalString(failing.result, "stream_read_error_message") : null;
    return {
      key: "stream_read_error",
      label: "SSE \u8bfb\u53d6\u65e0\u4e2d\u65ad\uff08stream_read_error=false\uff09",
      ok,
      required: doneRows.length > 0,
      detail: msg ? truncateText(firstLine(msg), 120) : null,
    } satisfies SuiteProtocolItem;
  })();

  const byRequiredFlag = (
    key: string,
    label: string,
    flagKey: string,
    readOk: (row: SuiteSummaryRow) => boolean | null | undefined,
    detailOf: (row: SuiteSummaryRow) => string | null
  ): SuiteProtocolItem => {
    const relevant = rows.filter((r) => suiteTemplateRequiresFlag(r.evaluation, flagKey));
    const required = relevant.length > 0;
    const values = relevant.map((r) => (r.status === "done" ? readOk(r) : null));
    const ok = suiteAggregateOk(values);
    const failing = relevant.find((r) => r.status === "done" && readOk(r) === false);
    return {
      key,
      label,
      ok,
      required,
      detail: failing ? truncateText(firstLine(detailOf(failing) ?? ""), 120) : null,
    };
  };

  const modelConsistency = byRequiredFlag(
    "model_consistency",
    "\u6a21\u578b\u4e00\u81f4\uff08requested_model==responded_model\uff09",
    "requireModelConsistency",
    (r) => r.evaluation.derived.modelConsistency,
    (r) => r.evaluation.checks.modelConsistency?.title ?? null
  );

  const outputTokens = (() => {
    const relevant = rows.filter((r) => r.templateKey === "official_max_tokens_5");
    const required = relevant.length > 0;
    const values = relevant.map((r) =>
      r.status === "done" ? r.evaluation.checks.outputTokens?.ok : null
    );
    const ok = suiteAggregateOk(values);
    const failing = relevant.find(
      (r) => r.status === "done" && r.evaluation.checks.outputTokens?.ok === false
    );
    return {
      key: "max_tokens_output_tokens",
      label: "max_tokens \u751f\u6548\uff08usage.output_tokens\uff09",
      ok,
      required,
      detail: failing
        ? truncateText(firstLine(failing.evaluation.checks.outputTokens?.title ?? ""), 120)
        : null,
    } satisfies SuiteProtocolItem;
  })();

  const thinkingOutput = byRequiredFlag(
    "thinking_output",
    "Extended Thinking\uff08thinking block\uff09",
    "requireThinkingOutput",
    (r) => r.evaluation.checks.thinkingOutput?.ok,
    (r) => r.evaluation.checks.thinkingOutput?.title ?? null
  );

  const signature = byRequiredFlag(
    "signature",
    "Signature\uff08step1\uff09",
    "requireSignature",
    (r) => r.evaluation.checks.signature?.ok,
    (r) => r.evaluation.checks.signature?.title ?? null
  );

  const signatureRoundtrip = byRequiredFlag(
    "signature_roundtrip",
    "Signature \u56de\u4f20\u9a8c\u8bc1\uff08Step2\uff09",
    "requireSignatureRoundtrip",
    (r) => r.evaluation.checks.signatureRoundtrip?.ok,
    (r) => r.evaluation.checks.signatureRoundtrip?.title ?? null
  );

  const crossProviderSignature = byRequiredFlag(
    "cross_provider_signature",
    "\u8de8\u4f9b\u5e94\u5546 Signature\uff08Step3\uff09",
    "requireCrossProviderSignatureRoundtrip",
    (r) => r.evaluation.checks.crossProviderSignatureRoundtrip?.ok,
    (r) => r.evaluation.checks.crossProviderSignatureRoundtrip?.title ?? null
  );

  const thinkingPreserved = byRequiredFlag(
    "thinking_preserved",
    "Thinking \u8de8\u6b65\u9aa4\u4fdd\u7559\uff08Step3\uff09",
    "requireThinkingPreserved",
    (r) => r.evaluation.checks.thinkingPreserved?.ok,
    (r) => r.evaluation.checks.thinkingPreserved?.title ?? null
  );

  const responseId = byRequiredFlag(
    "response_id",
    "response.id",
    "requireResponseId",
    (r) => r.evaluation.checks.responseId?.ok,
    (r) => r.evaluation.checks.responseId?.title ?? null
  );

  const serviceTier = byRequiredFlag(
    "service_tier",
    "service_tier",
    "requireServiceTier",
    (r) => r.evaluation.checks.serviceTier?.ok,
    (r) => r.evaluation.checks.serviceTier?.title ?? null
  );

  const outputConfig = byRequiredFlag(
    "output_config",
    "Output Config\uff08\u7f13\u5b58/\u670d\u52a1\u5c42\u7ea7\uff09",
    "requireOutputConfig",
    (r) => r.evaluation.checks.outputConfig?.ok,
    (r) => r.evaluation.checks.outputConfig?.title ?? null
  );

  const toolSupport = byRequiredFlag(
    "tool_support",
    "\u5de5\u5177\u80fd\u529b\u611f\u77e5\uff08tool keywords\uff09",
    "requireToolSupport",
    (r) => r.evaluation.checks.toolSupport?.ok,
    (r) => r.evaluation.checks.toolSupport?.title ?? null
  );

  const multiTurn = byRequiredFlag(
    "multi_turn",
    "\u591a\u8f6e\u5bf9\u8bdd\uff08\u6697\u53f7\u7b2c\u4e00\u884c\uff09",
    "requireMultiTurn",
    (r) => r.evaluation.checks.multiTurn?.ok,
    (r) => r.evaluation.checks.multiTurn?.title ?? null
  );

  const signatureTamper = (() => {
    const relevant = rows.filter(
      (r) =>
        suiteTemplateRequiresFlag(r.evaluation, "requireSignatureRoundtrip") &&
        suiteTemplateWantsSignatureTamper(r.evaluation)
    );
    const values = relevant.map((r) =>
      r.status === "done" ? (r.evaluation.checks.signatureTamper?.ok ?? null) : null
    );
    const ok = suiteAggregateOk(values);
    const failing = relevant.find(
      (r) => r.status === "done" && r.evaluation.checks.signatureTamper?.ok === false
    );
    const unknown = relevant.find(
      (r) => r.status === "done" && r.evaluation.checks.signatureTamper == null
    );
    const unknownDetail = (() => {
      if (!unknown) return null;
      const enabled = suiteSignalBool(unknown.result, "roundtrip_step3_enabled");
      if (enabled === false)
        return "Step3 \u672a\u542f\u7528\uff08\u672a\u6267\u884c\u7be1\u6539\u9a8c\u8bc1\uff09";
      if (enabled === true)
        return "Step3 \u5df2\u542f\u7528\uff0c\u4f46\u7f3a\u5c11 rejected \u4fe1\u53f7\uff08\u65e0\u6cd5\u5224\u65ad\u662f\u5426\u771f\u5b9e\u9a8c\u7b7e\uff09";
      return "Step3 \u4fe1\u53f7\u7f3a\u5931\uff08\u65e0\u6cd5\u5224\u65ad\u662f\u5426\u771f\u5b9e\u9a8c\u7b7e\uff09";
    })();
    return {
      key: "signature_tamper",
      label: "Signature \u7be1\u6539\u5e94\u88ab\u62d2\u7edd\uff08Step3\uff09",
      ok,
      required: false,
      detail: failing
        ? truncateText(firstLine(failing.evaluation.checks.signatureTamper?.title ?? ""), 120)
        : unknownDetail,
    } satisfies SuiteProtocolItem;
  })();

  return [
    requestOk,
    sseParse,
    streamRead,
    reverseProxy,
    modelConsistency,
    outputTokens,
    responseId,
    serviceTier,
    outputConfig,
    thinkingOutput,
    signature,
    signatureRoundtrip,
    signatureTamper,
    crossProviderSignature,
    thinkingPreserved,
    toolSupport,
    multiTurn,
  ].filter((it) => it.required || it.ok != null || it.detail != null);
}
