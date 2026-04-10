import type { RefObject } from "react";
import { toast } from "sonner";
import { logToConsole } from "../../services/consoleLog";
import {
  claudeProviderGetApiKeyPlaintext,
  claudeProviderValidateModel,
} from "../../services/claude/claudeModelValidation";
import type { ClaudeModelValidationResult } from "../../services/claude/claudeModelValidation";
import type { ProviderSummary } from "../../services/providers/providers";
import {
  buildClaudeValidationRequestJson,
  getClaudeTemplateApplicability,
  getClaudeValidationTemplate,
  type ClaudeValidationTemplateKey,
} from "../../services/claude/claudeValidationTemplates";
import {
  buildClaudeModelValidationRequestSnapshotTextFromResult,
  buildClaudeModelValidationRequestSnapshotTextFromWrapper,
} from "../../services/claude/claudeModelValidationRequestSnapshot";
import {
  buildClaudeCliMetadataUserId,
  newUuidV4,
  rotateClaudeCliUserIdSession,
} from "../../constants/claudeValidation";

import type { ClaudeValidationSuiteStep } from "./types";
import { isPlainObject, getTemplateDisplayLabel } from "./helpers";

export type RunValidationSuiteContext = {
  open: boolean;
  providerRef: RefObject<ProviderSummary | null>;
  abortRef: RefObject<boolean>;
  baseUrl: string;
  model: string;
  apiKeyPlaintext: string | null;
  templates: ReturnType<
    typeof import("../../services/claude/claudeValidationTemplates").listClaudeValidationTemplates
  >;
  crossProviderId: number | null;
  allClaudeProviders: ProviderSummary[];
  suiteRounds: number;
  historyReqSeqRef: RefObject<number>;

  setValidating: (v: boolean) => void;
  setApiKeyPlaintext: (v: string | null) => void;
  setCrossProviderId: (v: number | null) => void;
  setSelectedHistoryKey: (v: string | null) => void;
  setSuiteActiveStepIndex: (v: number | null) => void;
  setSuiteProgress: (
    v: { current: number; total: number; round: number; totalRounds: number } | null
  ) => void;
  setSuiteSteps: (
    v:
      | ClaudeValidationSuiteStep[]
      | ((prev: ClaudeValidationSuiteStep[]) => ClaudeValidationSuiteStep[])
  ) => void;
  setRequestJson: (v: string) => void;
  setResultTemplateKey: (v: ClaudeValidationTemplateKey) => void;
  setResult: (v: ClaudeModelValidationResult | null) => void;
  setHistoryLoading: (v: boolean) => void;

  refreshHistory: (options?: {
    selectLatest?: boolean;
    allowAutoSelectWhenNone?: boolean;
  }) => Promise<void>;
};

export async function runValidationSuite(ctx: RunValidationSuiteContext) {
  const {
    open,
    providerRef,
    abortRef,
    baseUrl,
    model,
    templates,
    crossProviderId,
    allClaudeProviders,
    suiteRounds,
    historyReqSeqRef,
    setValidating,
    setApiKeyPlaintext,
    setCrossProviderId,
    setSelectedHistoryKey,
    setSuiteActiveStepIndex,
    setSuiteProgress,
    setSuiteSteps,
    setRequestJson,
    setResultTemplateKey,
    setResult,
    setHistoryLoading,
    refreshHistory,
  } = ctx;
  let { apiKeyPlaintext } = ctx;

  // Reset abort flag at the start of each run.
  abortRef.current = false;

  const curProvider = providerRef.current;
  if (!open || !curProvider) return;

  if (!baseUrl.trim()) {
    toast("\u8bf7\u5148\u9009\u62e9 Endpoint\uff08Base URL\uff09");
    return;
  }

  const normalizedModel = model.trim();
  if (!normalizedModel) {
    toast("\u8bf7\u5148\u586b\u5199/\u9009\u62e9\u6a21\u578b");
    return;
  }

  let apiKeyPlaintextForSnapshot = apiKeyPlaintext;
  if (!apiKeyPlaintextForSnapshot) {
    try {
      const fetched = await claudeProviderGetApiKeyPlaintext(curProvider.id);
      apiKeyPlaintextForSnapshot = typeof fetched === "string" && fetched.trim() ? fetched : null;
      if (apiKeyPlaintextForSnapshot) {
        setApiKeyPlaintext(apiKeyPlaintextForSnapshot);
        apiKeyPlaintext = apiKeyPlaintextForSnapshot;
      }
    } catch (err) {
      logToConsole("warn", "获取 API Key 失败（非关键）", { error: String(err) });
    }
  }

  const templateApplicability = templates.map((t) => ({
    template: t,
    applicability: getClaudeTemplateApplicability(t, normalizedModel),
  }));
  const skippedTemplates = templateApplicability.filter((t) => !t.applicability.applicable);
  const suiteTemplateKeys = templateApplicability
    .filter((t) => t.applicability.applicable)
    .map((t) => t.template.key);
  const suiteRequiresCrossProvider = templateApplicability.some(
    (t) =>
      t.applicability.applicable &&
      (t.template as unknown as Record<string, unknown>).requiresCrossProvider === true
  );

  if (skippedTemplates.length > 0) {
    const shown = skippedTemplates
      .slice(0, 3)
      .map(
        (t) =>
          `${getTemplateDisplayLabel(t.template)}${
            t.applicability.reason ? `\uff08${t.applicability.reason}\uff09` : ""
          }`
      )
      .join("\uff1b");
    const rest = skippedTemplates.length - Math.min(3, skippedTemplates.length);
    toast(
      `\u5df2\u8df3\u8fc7 ${skippedTemplates.length} \u4e2a\u4e0d\u9002\u7528\u6a21\u677f\uff1a${shown}${rest > 0 ? `\uff1b+${rest}` : ""}`
    );
  }

  if (suiteTemplateKeys.length === 0) {
    toast("\u6682\u65e0\u9002\u7528\u9a8c\u8bc1\u6a21\u677f");
    return;
  }

  if (suiteRequiresCrossProvider) {
    const availableCrossProviders = allClaudeProviders.filter((p) => p.id !== curProvider.id);
    if (availableCrossProviders.length === 0) {
      toast(
        "\u8de8\u4f9b\u5e94\u5546\u9a8c\u8bc1\u9700\u8981\u81f3\u5c11\u914d\u7f6e 2 \u4e2a\u5b98\u65b9\u4f9b\u5e94\u5546"
      );
      return;
    }
    if (!crossProviderId) {
      toast(
        "\u8bf7\u5148\u9009\u62e9\u8de8\u4f9b\u5e94\u5546\u9a8c\u8bc1\u7684\u5b98\u65b9\u4f9b\u5e94\u5546\uff08\u7528\u4e8e Step3\uff09"
      );
      return;
    }
    if (crossProviderId === curProvider.id) {
      toast(
        "\u8de8\u4f9b\u5e94\u5546\u9a8c\u8bc1\u5fc5\u987b\u9009\u62e9\u4e0d\u540c\u4e8e\u5f53\u524d\u670d\u52a1\u5546\u7684\u4f9b\u5e94\u5546"
      );
      setCrossProviderId(null);
      return;
    }
    if (!availableCrossProviders.some((p) => p.id === crossProviderId)) {
      toast("\u6240\u9009\u8de8\u4f9b\u5e94\u5546\u65e0\u6548\uff0c\u8bf7\u91cd\u65b0\u9009\u62e9");
      setCrossProviderId(null);
      return;
    }
  }

  historyReqSeqRef.current += 1;
  setHistoryLoading(false);

  setValidating(true);
  const totalRounds = suiteRounds;
  setSelectedHistoryKey(null);
  setSuiteActiveStepIndex(null);

  try {
    for (let round = 1; round <= totalRounds; round += 1) {
      if (abortRef.current) break;
      const suiteRunId = newUuidV4();
      setSuiteProgress({ current: 0, total: suiteTemplateKeys.length, round, totalRounds });
      setSuiteSteps(
        suiteTemplateKeys.map((k, idx) => {
          const t = getClaudeValidationTemplate(k);
          return {
            index: idx + 1,
            templateKey: t.key,
            label: getTemplateDisplayLabel(t),
            status: "pending",
            request_json: "",
            result_json: "",
            result: null,
            error: null,
          };
        })
      );

      for (let idx = 0; idx < suiteTemplateKeys.length; idx += 1) {
        if (abortRef.current) break;
        const stepKey = suiteTemplateKeys[idx];
        const stepTemplate = getClaudeValidationTemplate(stepKey);
        setSuiteProgress({
          current: idx + 1,
          total: suiteTemplateKeys.length,
          round,
          totalRounds,
        });

        setSuiteSteps((prev: ClaudeValidationSuiteStep[]) =>
          prev.map((s) =>
            s.index === idx + 1
              ? { ...s, status: "running", error: null }
              : s.status === "pending"
                ? { ...s }
                : s
          )
        );

        const sessionId = newUuidV4();
        let reqTextToSendWrapper = buildClaudeValidationRequestJson(
          stepTemplate.key,
          normalizedModel,
          null
        );
        try {
          const parsedForSend: unknown = JSON.parse(reqTextToSendWrapper);
          const bodyForSend =
            isPlainObject(parsedForSend) && "body" in parsedForSend
              ? parsedForSend.body
              : parsedForSend;

          if (isPlainObject(bodyForSend)) {
            const nextBody: Record<string, unknown> = { ...bodyForSend };
            const nextMetadata: Record<string, unknown> = isPlainObject(nextBody.metadata)
              ? { ...(nextBody.metadata as Record<string, unknown>) }
              : {};

            const existingUserId =
              typeof nextMetadata.user_id === "string" ? nextMetadata.user_id.trim() : "";
            const rotated = existingUserId
              ? rotateClaudeCliUserIdSession(existingUserId, sessionId)
              : null;
            if (rotated) {
              nextMetadata.user_id = rotated;
            } else if (!existingUserId) {
              nextMetadata.user_id = buildClaudeCliMetadataUserId(sessionId);
            }
            nextBody.metadata = nextMetadata;

            if (isPlainObject(parsedForSend) && "body" in parsedForSend) {
              const nextParsed: Record<string, unknown> = { ...parsedForSend };
              const nextHeaders: Record<string, unknown> = isPlainObject(nextParsed.headers)
                ? { ...(nextParsed.headers as Record<string, unknown>) }
                : {};
              nextParsed.suite_run_id = suiteRunId;
              nextParsed.suite_step_index = idx + 1;
              nextParsed.suite_step_total = suiteTemplateKeys.length;
              nextParsed.headers = nextHeaders;
              nextParsed.body = nextBody;

              const templateRequiresCrossProvider =
                (stepTemplate as unknown as Record<string, unknown>).requiresCrossProvider === true;
              if (
                templateRequiresCrossProvider &&
                crossProviderId &&
                isPlainObject(nextParsed.roundtrip)
              ) {
                nextParsed.roundtrip = {
                  ...(nextParsed.roundtrip as Record<string, unknown>),
                  cross_provider_id: crossProviderId,
                };
              }

              reqTextToSendWrapper = JSON.stringify(nextParsed, null, 2);
            } else {
              reqTextToSendWrapper = JSON.stringify(nextBody, null, 2);
            }
          }
        } catch (err) {
          logToConsole("warn", "构建请求 JSON 失败（使用原始模板）", { error: String(err) });
        }

        const preSendRequestSnapshotText =
          buildClaudeModelValidationRequestSnapshotTextFromWrapper({
            baseUrl: baseUrl.trim(),
            wrapperJsonText: reqTextToSendWrapper,
            apiKeyPlaintext: apiKeyPlaintextForSnapshot,
          }) || reqTextToSendWrapper;

        setRequestJson(preSendRequestSnapshotText);

        setSuiteSteps((prev: ClaudeValidationSuiteStep[]) =>
          prev.map((s) =>
            s.index === idx + 1 ? { ...s, request_json: preSendRequestSnapshotText } : s
          )
        );

        let resp: ClaudeModelValidationResult | null = null;
        try {
          resp = await claudeProviderValidateModel({
            provider_id: curProvider.id,
            base_url: baseUrl.trim(),
            request_json: reqTextToSendWrapper,
          });
        } catch (err) {
          logToConsole(
            "error",
            "Claude Provider \u6a21\u578b\u9a8c\u8bc1\u5931\u8d25\uff08\u6279\u91cf\uff09",
            {
              error: String(err),
              provider_id: curProvider.id,
              attempt: idx + 1,
              template_key: stepTemplate.key,
            }
          );
          setSuiteSteps((prev: ClaudeValidationSuiteStep[]) =>
            prev.map((s) =>
              s.index === idx + 1
                ? { ...s, status: "error", error: String(err), result_json: "" }
                : s
            )
          );
          continue;
        }

        if (!resp) {
          setSuiteSteps((prev: ClaudeValidationSuiteStep[]) =>
            prev.map((s) =>
              s.index === idx + 1
                ? { ...s, status: "error", error: "IPC \u8c03\u7528\u8fd4\u56de\u7a7a" }
                : s
            )
          );
          return;
        }

        setResultTemplateKey(stepTemplate.key);
        setSelectedHistoryKey(null);
        setResult(resp);

        const executedRequestSnapshotCandidate =
          buildClaudeModelValidationRequestSnapshotTextFromResult(resp, apiKeyPlaintextForSnapshot);
        const executedRequestSnapshotText = executedRequestSnapshotCandidate.trim()
          ? executedRequestSnapshotCandidate
          : preSendRequestSnapshotText;

        setRequestJson(executedRequestSnapshotText);

        const suiteResultJson = (() => {
          try {
            return JSON.stringify(resp, null, 2);
          } catch {
            return "";
          }
        })();

        setSuiteSteps((prev: ClaudeValidationSuiteStep[]) =>
          prev.map((s) =>
            s.index === idx + 1
              ? {
                  ...s,
                  status: "done",
                  result: resp,
                  request_json: executedRequestSnapshotText,
                  result_json: suiteResultJson,
                  error: null,
                }
              : s
          )
        );
      }

      await refreshHistory({ selectLatest: false, allowAutoSelectWhenNone: false });
      setSelectedHistoryKey(null);
    }
  } catch (err) {
    logToConsole("error", "Claude Provider \u6a21\u578b\u9a8c\u8bc1\u5931\u8d25", {
      error: String(err),
      provider_id: curProvider.id,
    });
    toast(`\u9a8c\u8bc1\u5931\u8d25\uff1a${String(err)}`);
  } finally {
    setValidating(false);
    setSuiteProgress(null);
  }
}
