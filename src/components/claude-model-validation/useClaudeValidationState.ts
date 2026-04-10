import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { logToConsole } from "../../services/consoleLog";
import { copyText } from "../../services/clipboard";
import { useProvidersListQuery } from "../../query/providers";
import { claudeProviderGetApiKeyPlaintext } from "../../services/claude/claudeModelValidation";
import type { ClaudeModelValidationResult } from "../../services/claude/claudeModelValidation";
import {
  claudeValidationHistoryClearProvider,
  claudeValidationHistoryList,
} from "../../services/claude/claudeModelValidationHistory";
import { baseUrlPingMs, type ProviderSummary } from "../../services/providers/providers";
import {
  DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY,
  evaluateClaudeValidation,
  extractTemplateKeyFromRequestJson,
  getClaudeTemplateApplicability,
  getClaudeValidationTemplate,
  listClaudeValidationTemplates,
  type ClaudeValidationTemplateKey,
} from "../../services/claude/claudeValidationTemplates";
import { runValidationSuite as runValidationSuiteImpl } from "./runValidationSuite";

import type {
  ClaudeModelValidationRunView,
  ClaudeValidationSuiteStep,
  SuiteSummaryRow,
  ValidationDetailsTab,
} from "./types";
import {
  getHistoryGroupKey,
  parseClaudeModelValidationResultJson,
  getTemplateDisplayLabel,
  buildSuiteSummary,
  DEFAULT_MODEL,
} from "./helpers";
import { deriveHistoryGroups } from "./deriveHistoryGroups";

export function useClaudeValidationState(
  open: boolean,
  provider: ProviderSummary | null,
  onOpenChange: (open: boolean) => void
) {
  const providerRef = useRef(provider);
  useEffect(() => {
    providerRef.current = provider;
  }, [provider]);

  const abortRef = useRef(false);

  const [baseUrl, setBaseUrl] = useState("");
  const [baseUrlPicking, setBaseUrlPicking] = useState(false);

  const templates = useMemo(() => listClaudeValidationTemplates(), []);
  const [templateKey, setTemplateKey] = useState<ClaudeValidationTemplateKey>(
    DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY
  );
  const [resultTemplateKey, setResultTemplateKey] = useState<ClaudeValidationTemplateKey>(
    DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY
  );

  const [model, setModel] = useState("claude-sonnet-4-5-20250929");

  const [requestJson, setRequestJson] = useState("");
  const [apiKeyPlaintext, setApiKeyPlaintext] = useState<string | null>(null);

  const [result, setResult] = useState<ClaudeModelValidationResult | null>(null);

  const [validating, setValidating] = useState(false);
  const [suiteSteps, setSuiteSteps] = useState<ClaudeValidationSuiteStep[]>([]);
  const [suiteProgress, setSuiteProgress] = useState<{
    current: number;
    total: number;
    round: number;
    totalRounds: number;
  } | null>(null);
  const [suiteIssuesOnly, setSuiteIssuesOnly] = useState(false);
  const [suiteActiveStepIndex, setSuiteActiveStepIndex] = useState<number | null>(null);
  const [detailsTab, setDetailsTab] = useState<ValidationDetailsTab>("overview");

  const [historyRuns, setHistoryRuns] = useState<ClaudeModelValidationRunView[]>([]);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [historyAvailable, setHistoryAvailable] = useState<boolean | null>(null);
  const [selectedHistoryKey, setSelectedHistoryKey] = useState<string | null>(null);
  const historyReqSeqRef = useRef(0);
  const [historyClearing, setHistoryClearing] = useState(false);
  const [confirmClearOpen, setConfirmClearOpen] = useState(false);

  const [suiteRounds, setSuiteRounds] = useState(1);

  // Cross-provider signature validation
  const allClaudeProvidersQuery = useProvidersListQuery("claude", { enabled: open });
  const allClaudeProviders = useMemo<ProviderSummary[]>(
    () => (open ? (allClaudeProvidersQuery.data ?? []) : []),
    [open, allClaudeProvidersQuery.data]
  );
  const [crossProviderId, setCrossProviderId] = useState<number | null>(null);

  const hasCrossProviderTemplate = useMemo(
    () =>
      templates.some(
        (t) => (t as unknown as Record<string, unknown>).requiresCrossProvider === true
      ),
    [templates]
  );

  const crossProviderOptions = useMemo(() => {
    if (!provider) return [];
    return allClaudeProviders.filter((p) => p.id !== provider.id);
  }, [allClaudeProviders, provider]);

  // ---------------------------------------------------------------------------
  // Reset effects
  // ---------------------------------------------------------------------------

  useEffect(() => {
    if (!open) {
      abortRef.current = true;
      setBaseUrl("");
      setBaseUrlPicking(false);
      setTemplateKey(DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY);
      setResultTemplateKey(DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY);
      setModel(DEFAULT_MODEL);
      setRequestJson("");
      setApiKeyPlaintext(null);
      setResult(null);
      setValidating(false);
      setSuiteSteps([]);
      setSuiteProgress(null);
      setSuiteIssuesOnly(false);
      setSuiteActiveStepIndex(null);
      setDetailsTab("overview");
      setHistoryRuns([]);
      setHistoryLoading(false);
      setHistoryAvailable(null);
      setSelectedHistoryKey(null);
      historyReqSeqRef.current = 0;
      setHistoryClearing(false);
      setConfirmClearOpen(false);
      setCrossProviderId(null);
      setSuiteRounds(1);
      return;
    }

    setTemplateKey(DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY);
    setResultTemplateKey(DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY);
    setModel(DEFAULT_MODEL);
    setRequestJson("");
    setApiKeyPlaintext(null);
    setResult(null);
    setSuiteSteps([]);
    setSuiteProgress(null);
    setSuiteIssuesOnly(false);
    setSuiteActiveStepIndex(null);
    setDetailsTab("overview");
  }, [open]);

  const providerId = provider?.id ?? null;

  useEffect(() => {
    if (!open || providerId == null) return;
    let cancelled = false;

    claudeProviderGetApiKeyPlaintext(providerId)
      .then((key) => {
        if (cancelled) return;
        setApiKeyPlaintext(typeof key === "string" && key.trim() ? key : null);
      })
      .catch(() => {
        if (cancelled) return;
        setApiKeyPlaintext(null);
      });

    return () => {
      cancelled = true;
    };
  }, [open, providerId]);

  // ---------------------------------------------------------------------------
  // Handlers
  // ---------------------------------------------------------------------------

  function handleOpenChange(nextOpen: boolean) {
    if (!nextOpen && confirmClearOpen) {
      setConfirmClearOpen(false);
      return;
    }
    onOpenChange(nextOpen);
  }

  async function refreshHistory(options?: {
    selectLatest?: boolean;
    allowAutoSelectWhenNone?: boolean;
  }) {
    const curProvider = providerRef.current;
    if (!open || !curProvider) return;
    const pid = curProvider.id;

    const reqSeq = (historyReqSeqRef.current += 1);
    setHistoryLoading(true);
    try {
      const rows = await claudeValidationHistoryList({ provider_id: pid, limit: 50 });
      if (reqSeq !== historyReqSeqRef.current) return;
      if (!rows) {
        setHistoryAvailable(false);
        setHistoryRuns([]);
        setSelectedHistoryKey(null);
        return;
      }

      setHistoryAvailable(true);
      const mapped: ClaudeModelValidationRunView[] = rows.map((r) => ({
        ...r,
        parsed_result: parseClaudeModelValidationResultJson(r.result_json),
      }));
      setHistoryRuns(mapped);

      const nextSelected = (() => {
        const keys = mapped.map((it) => getHistoryGroupKey(it));
        const uniqueKeys = new Set(keys);
        const allowAutoSelectWhenNone =
          typeof options?.allowAutoSelectWhenNone === "boolean"
            ? options.allowAutoSelectWhenNone
            : true;

        if (options?.selectLatest) return keys[0] ?? null;
        if (selectedHistoryKey && uniqueKeys.has(selectedHistoryKey)) return selectedHistoryKey;
        if (!selectedHistoryKey && !allowAutoSelectWhenNone) return null;
        return keys[0] ?? null;
      })();
      setSelectedHistoryKey(nextSelected);
    } catch (err) {
      if (reqSeq !== historyReqSeqRef.current) return;
      logToConsole("error", "Claude \u6a21\u578b\u9a8c\u8bc1\u5386\u53f2\u52a0\u8f7d\u5931\u8d25", {
        error: String(err),
      });
      setHistoryAvailable(true);
      setHistoryRuns([]);
      setSelectedHistoryKey(null);
    } finally {
      if (reqSeq === historyReqSeqRef.current) {
        setHistoryLoading(false);
      }
    }
  }

  useEffect(() => {
    if (!open) return;
    if (!providerId) return;
    void refreshHistory({ selectLatest: true });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, providerId]);

  useEffect(() => {
    if (!open || !provider) return;

    setBaseUrl(provider.base_urls[0] ?? "");
    setBaseUrlPicking(false);

    if (provider.base_url_mode !== "ping") return;
    if (provider.base_urls.length <= 1) return;

    let cancelled = false;
    setBaseUrlPicking(true);

    Promise.all(
      provider.base_urls.map(async (url) => {
        try {
          const ms = await baseUrlPingMs(url);
          return { url, ms };
        } catch {
          return { url, ms: null as number | null };
        }
      })
    )
      .then((rows) => {
        if (cancelled) return;
        const fastest = rows
          .filter((r) => typeof r.ms === "number")
          .sort((a, b) => (a.ms ?? 0) - (b.ms ?? 0))[0];
        if (fastest?.url) {
          setBaseUrl(fastest.url);
        }
      })
      .finally(() => {
        if (cancelled) return;
        setBaseUrlPicking(false);
      });

    return () => {
      cancelled = true;
    };
  }, [open, provider]);

  async function copyTextOrToast(text: string, okMessage: string) {
    try {
      await copyText(text);
      toast(okMessage);
    } catch (err) {
      logToConsole("error", "\u590d\u5236\u5931\u8d25", { error: String(err) });
      toast(
        "\u590d\u5236\u5931\u8d25\uff1a\u5f53\u524d\u73af\u5883\u4e0d\u652f\u6301\u526a\u8d34\u677f"
      );
    }
  }

  const runValidationSuite = useCallback(
    () => {
      if (validating) return Promise.resolve();
      return runValidationSuiteImpl({
        open,
        providerRef,
        abortRef,
        baseUrl,
        model,
        apiKeyPlaintext,
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
      });
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [
      open,
      baseUrl,
      model,
      apiKeyPlaintext,
      templates,
      crossProviderId,
      allClaudeProviders,
      suiteRounds,
      validating,
    ]
  );

  async function clearProviderHistory() {
    if (historyClearing) return;

    const curProvider = providerRef.current;
    if (!open || !curProvider) return;

    setHistoryClearing(true);
    try {
      historyReqSeqRef.current += 1;
      setHistoryRuns([]);
      setSelectedHistoryKey(null);

      const ok = await claudeValidationHistoryClearProvider({ provider_id: curProvider.id });
      if (ok == null) {
        return;
      }
      if (!ok) {
        toast("\u6e05\u7a7a\u5931\u8d25");
        return;
      }

      toast("\u5df2\u6e05\u7a7a\u5386\u53f2");
      await refreshHistory({ selectLatest: true });
    } catch (err) {
      toast(`\u6e05\u7a7a\u5931\u8d25\uff1a${String(err)}`);
      void refreshHistory({ selectLatest: true });
    } finally {
      setHistoryClearing(false);
      setConfirmClearOpen(false);
    }
  }

  // ---------------------------------------------------------------------------
  // Derived state
  // ---------------------------------------------------------------------------

  const title = provider
    ? `Claude \u00b7 \u6a21\u578b\u9a8c\u8bc1\uff1a${provider.name}`
    : "Claude \u00b7 \u6a21\u578b\u9a8c\u8bc1";

  const historyGroups = useMemo(() => deriveHistoryGroups(historyRuns), [historyRuns]);

  const selectedHistoryGroup = useMemo(() => {
    if (!selectedHistoryKey) return null;
    return historyGroups.find((g) => g.key === selectedHistoryKey) ?? null;
  }, [historyGroups, selectedHistoryKey]);

  const selectedHistoryLatest =
    selectedHistoryGroup?.runs[selectedHistoryGroup.runs.length - 1] ?? null;
  const activeResult = selectedHistoryLatest?.run.parsed_result ?? result;
  const activeResultTemplateKey = useMemo(() => {
    if (selectedHistoryLatest?.run.request_json) {
      const key = extractTemplateKeyFromRequestJson(selectedHistoryLatest.run.request_json);
      return getClaudeValidationTemplate(key).key;
    }
    if (result) return resultTemplateKey;
    return templateKey;
  }, [selectedHistoryLatest?.run.request_json, result, resultTemplateKey, templateKey]);

  const currentSuiteSummary = useMemo(() => {
    if (suiteSteps.length === 0) return null;
    if (selectedHistoryGroup) return null;
    const normalizedModel = model.trim();
    const rows: SuiteSummaryRow[] = suiteSteps.map((s) => ({
      templateKey: s.templateKey,
      label: s.label,
      status: s.status,
      evaluation: evaluateClaudeValidation(s.templateKey, s.result),
      result: s.result,
      errorText: s.error,
    }));
    return buildSuiteSummary(rows, normalizedModel);
  }, [model, selectedHistoryGroup, suiteSteps]);

  const historySuiteSummary = useMemo(() => {
    if (!selectedHistoryGroup?.isSuite) return null;

    const expectedTotal = selectedHistoryGroup.expectedTotal;
    const expectedKeys = templates
      .filter((t) => getClaudeTemplateApplicability(t, selectedHistoryGroup.modelName).applicable)
      .map((t) => t.key);

    const byIndex = new Map<number, (typeof selectedHistoryGroup.runs)[number]>();
    for (const r of selectedHistoryGroup.runs) {
      const idx = r.meta.suiteStepIndex ?? 0;
      if (!Number.isFinite(idx) || idx <= 0) continue;
      const prev = byIndex.get(idx);
      if (!prev || r.run.id > prev.run.id) byIndex.set(idx, r);
    }

    const rows: SuiteSummaryRow[] = [];
    for (let idx = 1; idx <= expectedTotal; idx += 1) {
      const step = byIndex.get(idx) ?? null;
      const expectedKey = expectedKeys[idx - 1] ?? step?.evaluation.templateKey;
      const templateKeyForUi = (expectedKey ??
        DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY) as ClaudeValidationTemplateKey;
      const template = getClaudeValidationTemplate(templateKeyForUi);
      rows.push({
        templateKey: templateKeyForUi,
        label: getTemplateDisplayLabel(template),
        status: step ? "done" : "missing",
        evaluation: step ? step.evaluation : evaluateClaudeValidation(templateKeyForUi, null),
        result: step?.run.parsed_result ?? null,
        errorText: null,
      });
    }

    return buildSuiteSummary(rows, selectedHistoryGroup.modelName);
  }, [selectedHistoryGroup, templates]);

  const suiteSummaryForHeader = currentSuiteSummary ?? historySuiteSummary;
  const hasSuiteContext =
    (suiteSteps.length > 0 && !selectedHistoryGroup) || selectedHistoryGroup?.isSuite === true;

  useEffect(() => {
    if (hasSuiteContext) return;
    if (detailsTab !== "steps") return;
    setDetailsTab("overview");
  }, [detailsTab, hasSuiteContext]);

  const detailsTabItems: Array<{ key: ValidationDetailsTab; label: string; disabled?: boolean }> =
    useMemo(() => {
      const overviewLabel = hasSuiteContext ? "\u603b\u7ed3" : "\u7ed3\u679c";
      return [
        { key: "overview" as ValidationDetailsTab, label: overviewLabel },
        ...(hasSuiteContext
          ? [{ key: "steps" as ValidationDetailsTab, label: "\u6b65\u9aa4" }]
          : []),
        { key: "debug" as ValidationDetailsTab, label: "\u8c03\u8bd5" },
      ];
    }, [hasSuiteContext]);

  const suiteHeaderMetaText = (() => {
    if (!suiteSummaryForHeader) return null;
    const nonPass =
      suiteSummaryForHeader.stats.fail +
      suiteSummaryForHeader.stats.error +
      suiteSummaryForHeader.stats.missing;
    const parts: string[] = [];
    parts.push(
      `\u5b8c\u6210 ${suiteSummaryForHeader.stats.done}/${suiteSummaryForHeader.stats.total}`
    );
    parts.push(`\u901a\u8fc7 ${suiteSummaryForHeader.stats.pass}`);
    parts.push(`\u672a\u901a\u8fc7 ${nonPass}`);
    if (suiteSummaryForHeader.stats.missing > 0)
      parts.push(`\u7f3a\u5931 ${suiteSummaryForHeader.stats.missing}`);
    return parts.join(" \u00b7 ");
  })();

  return {
    // State
    baseUrl,
    setBaseUrl,
    baseUrlPicking,
    templates,
    model,
    setModel,
    requestJson,
    setRequestJson,
    apiKeyPlaintext,
    result,
    validating,
    suiteSteps,
    suiteProgress,
    suiteIssuesOnly,
    setSuiteIssuesOnly,
    suiteActiveStepIndex,
    setSuiteActiveStepIndex,
    detailsTab,
    setDetailsTab,
    historyLoading,
    historyAvailable,
    selectedHistoryKey,
    setSelectedHistoryKey,
    historyClearing,
    confirmClearOpen,
    setConfirmClearOpen,
    suiteRounds,
    setSuiteRounds,
    crossProviderId,
    setCrossProviderId,
    hasCrossProviderTemplate,
    crossProviderOptions,

    // Derived
    title,
    historyGroups,
    selectedHistoryGroup,
    selectedHistoryLatest,
    activeResult,
    activeResultTemplateKey,
    currentSuiteSummary,
    historySuiteSummary,
    suiteSummaryForHeader,
    hasSuiteContext,
    detailsTabItems,
    suiteHeaderMetaText,

    // Actions
    handleOpenChange,
    refreshHistory,
    copyTextOrToast,
    runValidationSuite,
    clearProviderHistory,
  };
}
