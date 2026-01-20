import { useEffect, useMemo, useRef, useState, type MouseEvent, type ReactElement } from "react";
import { createPortal } from "react-dom";
import { toast } from "sonner";
import { logToConsole } from "../services/consoleLog";
import {
  claudeProviderGetApiKeyPlaintext,
  claudeProviderValidateModel,
} from "../services/claudeModelValidation";
import type { ClaudeModelValidationResult } from "../services/claudeModelValidation";
import {
  claudeValidationHistoryClearProvider,
  claudeValidationHistoryList,
  type ClaudeModelValidationRunRow,
} from "../services/claudeModelValidationHistory";
import { modelPricesList, type ModelPriceSummary } from "../services/modelPrices";
import { baseUrlPingMs, type ProviderSummary } from "../services/providers";
import {
  DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY,
  buildClaudeValidationRequestJson,
  evaluateClaudeValidation,
  extractTemplateKeyFromRequestJson,
  getClaudeTemplateApplicability,
  getClaudeValidationTemplate,
  listClaudeValidationTemplates,
  type ClaudeValidationTemplateKey,
} from "../services/claudeValidationTemplates";
import {
  buildClaudeCliMetadataUserId,
  newUuidV4,
  rotateClaudeCliUserIdSession,
} from "../constants/claudeValidation";
import { ClaudeModelValidationResultPanel } from "./ClaudeModelValidationResultPanel";
import { ClaudeModelValidationHistoryStepCard } from "./ClaudeModelValidationHistoryStepCard";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Dialog } from "../ui/Dialog";
import { FormField } from "../ui/FormField";
import { Select } from "../ui/Select";
import { Textarea } from "../ui/Textarea";
import { cn } from "../utils/cn";
import { formatUnixSeconds } from "../utils/formatters";
import {
  Play,
  Settings2,
  History,
  Trash2,
  RefreshCw,
  Server,
  Network,
  Cpu,
  CheckCircle2,
  XCircle,
  ChevronRight,
  ChevronDown,
  Activity,
  Copy,
  FileJson,
} from "lucide-react";

type ClaudeModelValidationDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  provider: ProviderSummary | null;
};

type SuiteMeta = {
  suiteRunId: string | null;
  suiteStepIndex: number | null;
  suiteStepTotal: number | null;
};

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function parseJsonObjectSafe(text: string): Record<string, unknown> | null {
  const raw = text.trim();
  if (!raw) return null;
  try {
    const obj = JSON.parse(raw);
    return isPlainObject(obj) ? obj : null;
  } catch {
    return null;
  }
}

function normalizeNonEmptyString(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function normalizePositiveInt(value: unknown): number | null {
  if (typeof value !== "number" || !Number.isFinite(value)) return null;
  const int = Math.floor(value);
  return int > 0 ? int : null;
}

function extractSuiteMetaFromRequestJson(requestJson: string): SuiteMeta {
  const obj = parseJsonObjectSafe(requestJson);
  if (!obj) return { suiteRunId: null, suiteStepIndex: null, suiteStepTotal: null };
  return {
    suiteRunId: normalizeNonEmptyString(obj.suite_run_id),
    suiteStepIndex: normalizePositiveInt(obj.suite_step_index),
    suiteStepTotal: normalizePositiveInt(obj.suite_step_total),
  };
}

function getHistoryGroupKey(run: { id: number; request_json: string }): string {
  const meta = extractSuiteMetaFromRequestJson(run.request_json ?? "");
  if (meta.suiteRunId) return `suite:${meta.suiteRunId}`;
  return `run:${run.id}`;
}

function sortClaudeModelsFromPrices(rows: ModelPriceSummary[]) {
  const unique = new Set<string>();
  for (const row of rows) {
    const model = row.model.trim();
    if (!model) continue;
    unique.add(model);
  }

  const priority = (model: string) => {
    const m = model.toLowerCase();
    if (m.startsWith("claude-opus-4-5")) return 0;
    if (m.startsWith("claude-sonnet-4-5")) return 1;
    if (m.includes("opus-4-5")) return 2;
    if (m.includes("sonnet-4-5")) return 3;
    if (m.startsWith("claude-opus")) return 10;
    if (m.startsWith("claude-sonnet")) return 11;
    if (m.startsWith("claude-haiku")) return 12;
    return 50;
  };

  return [...unique].sort((a, b) => {
    const pa = priority(a);
    const pb = priority(b);
    if (pa !== pb) return pa - pb;
    return a.localeCompare(b);
  });
}

type ClaudeModelValidationRunView = ClaudeModelValidationRunRow & {
  parsed_result: ClaudeModelValidationResult | null;
};

type ClaudeValidationSuiteStep = {
  index: number;
  templateKey: ClaudeValidationTemplateKey;
  label: string;
  status: "pending" | "running" | "done" | "error";
  request_json: string;
  result_json: string;
  result: ClaudeModelValidationResult | null;
  error: string | null;
};

function parseClaudeModelValidationResultJson(text: string): ClaudeModelValidationResult | null {
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

function prettyJsonOrFallback(text: string): string {
  const raw = text.trim();
  if (!raw) return "";
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}

function stopDetailsToggle(e: MouseEvent) {
  e.preventDefault();
  e.stopPropagation();
}

function OutcomePill({ pass }: { pass: boolean | null }) {
  if (pass == null) {
    return (
      <span className="rounded bg-slate-100 px-1.5 py-0.5 text-[10px] font-semibold text-slate-600">
        未知
      </span>
    );
  }
  return (
    <span
      className={cn(
        "rounded px-1.5 py-0.5 text-[10px] font-semibold",
        pass ? "bg-emerald-100 text-emerald-700" : "bg-rose-100 text-rose-700"
      )}
    >
      {pass ? "通过" : "不通过"}
    </span>
  );
}

export function ClaudeModelValidationDialog({
  open,
  onOpenChange,
  provider,
}: ClaudeModelValidationDialogProps) {
  const providerRef = useRef(provider);
  useEffect(() => {
    providerRef.current = provider;
  }, [provider]);

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

  const [apiKeyPlaintext, setApiKeyPlaintext] = useState<string | null>(null);
  const [apiKeyLoading, setApiKeyLoading] = useState(false);

  const [requestJson, setRequestJson] = useState("");
  const [requestDirty, setRequestDirty] = useState(false);

  const [result, setResult] = useState<ClaudeModelValidationResult | null>(null);

  const [validating, setValidating] = useState(false);
  const [suiteSteps, setSuiteSteps] = useState<ClaudeValidationSuiteStep[]>([]);
  const [suiteProgress, setSuiteProgress] = useState<{ current: number; total: number } | null>(
    null
  );

  const [historyRuns, setHistoryRuns] = useState<ClaudeModelValidationRunView[]>([]);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [historyAvailable, setHistoryAvailable] = useState<boolean | null>(null);
  const [selectedHistoryKey, setSelectedHistoryKey] = useState<string | null>(null);
  const historyReqSeqRef = useRef(0);
  const [historyClearing, setHistoryClearing] = useState(false);
  const [confirmClearOpen, setConfirmClearOpen] = useState(false);

  const [modelPrices, setModelPrices] = useState<ModelPriceSummary[]>([]);
  const [modelPricesLoading, setModelPricesLoading] = useState(false);
  const modelOptions = useMemo(() => sortClaudeModelsFromPrices(modelPrices), [modelPrices]);

  useEffect(() => {
    if (!open) {
      setBaseUrl("");
      setBaseUrlPicking(false);
      setTemplateKey(DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY);
      setResultTemplateKey(DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY);
      setModel("claude-sonnet-4-5-20250929");
      setApiKeyPlaintext(null);
      setApiKeyLoading(false);
      setRequestJson("");
      setRequestDirty(false);
      setResult(null);
      setValidating(false);
      setSuiteSteps([]);
      setSuiteProgress(null);
      setHistoryRuns([]);
      setHistoryLoading(false);
      setHistoryAvailable(null);
      setSelectedHistoryKey(null);
      historyReqSeqRef.current = 0;
      setHistoryClearing(false);
      setConfirmClearOpen(false);
      setModelPrices([]);
      setModelPricesLoading(false);
      setModelPricesLoading(false);
      return;
    }

    setTemplateKey(DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY);
    setResultTemplateKey(DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY);
    setModel("claude-sonnet-4-5-20250929");
    setRequestJson(
      buildClaudeValidationRequestJson(
        DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY,
        "claude-sonnet-4-5-20250929",
        apiKeyPlaintext
      )
    );
    setRequestDirty(false);
    setResult(null);
    setSuiteSteps([]);
    setSuiteProgress(null);
  }, [open]);

  function handleOpenChange(nextOpen: boolean) {
    // 防止确认弹层打开时误关主 Dialog（ESC/点遮罩/点右上角关闭等）。
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
    const providerId = curProvider.id;

    const reqSeq = (historyReqSeqRef.current += 1);
    setHistoryLoading(true);
    try {
      const rows = await claudeValidationHistoryList({ provider_id: providerId, limit: 50 });
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
      logToConsole("error", "Claude 模型验证历史加载失败", { error: String(err) });
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
    const providerId = provider?.id ?? null;
    if (!providerId) return;
    void refreshHistory({ selectLatest: true });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, provider?.id]);

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

  useEffect(() => {
    if (!open) return;
    const providerId = provider?.id ?? null;
    if (!providerId) return;

    let cancelled = false;
    setApiKeyLoading(true);
    claudeProviderGetApiKeyPlaintext(providerId)
      .then((key) => {
        if (cancelled) return;
        if (key == null) {
          setApiKeyPlaintext(null);
          return;
        }
        setApiKeyPlaintext(key);
      })
      .catch(() => {
        if (cancelled) return;
        setApiKeyPlaintext(null);
      })
      .finally(() => {
        if (cancelled) return;
        setApiKeyLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [open, provider]);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;

    setModelPricesLoading(true);
    modelPricesList("claude")
      .then((rows) => {
        if (cancelled) return;
        if (!rows) {
          setModelPrices([]);
          return;
        }

        setModelPrices(rows);
      })
      .catch(() => {
        if (cancelled) return;

        setModelPrices([]);
      })
      .finally(() => {
        if (cancelled) return;
        setModelPricesLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const apiKey = apiKeyPlaintext?.trim();
    if (!apiKey) return;

    const reqText = requestJson.trim();
    if (!reqText) return;

    try {
      const parsed = JSON.parse(reqText);
      if (!parsed || typeof parsed !== "object") return;
      if (!("headers" in parsed)) return;

      const next = { ...(parsed as any) };
      const nextHeaders =
        next.headers && typeof next.headers === "object" ? { ...(next.headers as any) } : {};

      nextHeaders["x-api-key"] = apiKey;
      nextHeaders.authorization = `Bearer ${apiKey}`;
      next.headers = nextHeaders;

      setRequestJson(JSON.stringify(next, null, 2));
    } catch {
      // Ignore: user may be mid-edit or using non-wrapper JSON.
    }
  }, [open, apiKeyPlaintext]);

  useEffect(() => {
    if (!open) return;
    const normalizedModel = model.trim();
    if (!normalizedModel) return;
    if (requestDirty) return;

    setRequestJson(buildClaudeValidationRequestJson(templateKey, normalizedModel, apiKeyPlaintext));
  }, [open, model, templateKey, apiKeyPlaintext, requestDirty]);

  async function copyTextOrToast(text: string, okMessage: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast(okMessage);
    } catch (err) {
      logToConsole("error", "复制失败", { error: String(err) });
      toast("复制失败：当前环境不支持剪贴板");
    }
  }

  async function runValidationSuite() {
    if (validating) return;

    const curProvider = providerRef.current;
    if (!open || !curProvider) return;

    if (!baseUrl.trim()) {
      toast("请先选择 Endpoint（Base URL）");
      return;
    }

    const normalizedModel = model.trim();
    if (!normalizedModel) {
      toast("请先填写/选择模型");
      return;
    }

    let key = apiKeyPlaintext?.trim() ?? "";
    if (!key && !apiKeyLoading) {
      try {
        const fetched = await claudeProviderGetApiKeyPlaintext(curProvider.id);
        if (typeof fetched === "string" && fetched.trim()) {
          key = fetched.trim();
          setApiKeyPlaintext(key);
        }
      } catch {
        // ignore
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

    if (skippedTemplates.length > 0) {
      const shown = skippedTemplates
        .slice(0, 3)
        .map(
          (t) =>
            `${t.template.label}${t.applicability.reason ? `（${t.applicability.reason}）` : ""}`
        )
        .join("；");
      const rest = skippedTemplates.length - Math.min(3, skippedTemplates.length);
      toast(
        `已跳过 ${skippedTemplates.length} 个不适用模板：${shown}${rest > 0 ? `；+${rest}` : ""}`
      );
    }

    if (suiteTemplateKeys.length === 0) {
      toast("暂无适用验证模板");
      return;
    }

    // Cancel any in-flight history refresh (dialog open / manual refresh). Otherwise a late
    // refreshHistory({selectLatest:true}) can switch the right pane into “历史记录详情” mid-suite,
    // making it look like only部分卡片/步骤执行了。
    historyReqSeqRef.current += 1;
    setHistoryLoading(false);

    setValidating(true);
    const suiteRunId = newUuidV4();
    setSelectedHistoryKey(null);
    setSuiteProgress({ current: 0, total: suiteTemplateKeys.length });
    setSuiteSteps(
      suiteTemplateKeys.map((k, idx) => {
        const t = getClaudeValidationTemplate(k);
        return {
          index: idx + 1,
          templateKey: t.key,
          label: t.label,
          status: "pending",
          request_json: "",
          result_json: "",
          result: null,
          error: null,
        };
      })
    );
    try {
      for (let idx = 0; idx < suiteTemplateKeys.length; idx += 1) {
        const stepKey = suiteTemplateKeys[idx];
        const stepTemplate = getClaudeValidationTemplate(stepKey);
        setSuiteProgress({ current: idx + 1, total: suiteTemplateKeys.length });

        setSuiteSteps((prev) =>
          prev.map((s) =>
            s.index === idx + 1
              ? { ...s, status: "running", error: null }
              : s.status === "pending"
                ? { ...s }
                : s
          )
        );

        const sessionId = newUuidV4();
        let reqTextToSend = buildClaudeValidationRequestJson(
          stepTemplate.key,
          normalizedModel,
          key
        );
        try {
          const parsedForSend = JSON.parse(reqTextToSend);
          const bodyForSend =
            parsedForSend && typeof parsedForSend === "object" && "body" in parsedForSend
              ? (parsedForSend as any).body
              : parsedForSend;

          if (bodyForSend && typeof bodyForSend === "object") {
            const nextBody = { ...(bodyForSend as any) };
            const nextMetadata =
              nextBody.metadata && typeof nextBody.metadata === "object"
                ? { ...(nextBody.metadata as any) }
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

            if (parsedForSend && typeof parsedForSend === "object" && "body" in parsedForSend) {
              const nextParsed = { ...(parsedForSend as any) };
              const nextHeaders =
                nextParsed.headers && typeof nextParsed.headers === "object"
                  ? { ...(nextParsed.headers as any) }
                  : {};
              // 用于历史聚合显示：同一次“综合验证”共享同一个 suite_run_id。
              nextParsed.suite_run_id = suiteRunId;
              nextParsed.suite_step_index = idx + 1;
              nextParsed.suite_step_total = suiteTemplateKeys.length;
              if (key) {
                nextHeaders["x-api-key"] = key;
                nextHeaders.authorization = `Bearer ${key}`;
              }
              nextParsed.headers = nextHeaders;
              nextParsed.body = nextBody;
              reqTextToSend = JSON.stringify(nextParsed, null, 2);
            } else {
              reqTextToSend = JSON.stringify(nextBody, null, 2);
            }
          }
        } catch {
          // ignore
        }

        setRequestJson(reqTextToSend);

        setSuiteSteps((prev) =>
          prev.map((s) => (s.index === idx + 1 ? { ...s, request_json: reqTextToSend } : s))
        );

        let resp: ClaudeModelValidationResult | null = null;
        try {
          resp = await claudeProviderValidateModel({
            provider_id: curProvider.id,
            base_url: baseUrl.trim(),
            request_json: reqTextToSend,
          });
        } catch (err) {
          logToConsole("error", "Claude Provider 模型验证失败（批量）", {
            error: String(err),
            provider_id: curProvider.id,
            attempt: idx + 1,
            template_key: stepTemplate.key,
          });
          setSuiteSteps((prev) =>
            prev.map((s) =>
              s.index === idx + 1
                ? { ...s, status: "error", error: String(err), result_json: "" }
                : s
            )
          );
          continue;
        }

        if (!resp) {
          toast("仅在 Tauri Desktop 环境可用");
          setSuiteSteps((prev) =>
            prev.map((s) =>
              s.index === idx + 1
                ? { ...s, status: "error", error: "仅在 Tauri Desktop 环境可用" }
                : s
            )
          );
          return;
        }

        setResultTemplateKey(stepTemplate.key);
        setSelectedHistoryKey(null);
        setResult(resp);

        const suiteResultJson = (() => {
          try {
            return JSON.stringify(resp, null, 2);
          } catch {
            return "";
          }
        })();

        setSuiteSteps((prev) =>
          prev.map((s) =>
            s.index === idx + 1
              ? { ...s, status: "done", result: resp, result_json: suiteResultJson, error: null }
              : s
          )
        );
      }

      // 刷新历史用于左侧列表更新，但不要自动切到“历史详情”，避免用户误以为本次 suite
      // 只执行了部分步骤（右侧需保持“当前运行”视图，历史仅用于回溯）。
      await refreshHistory({ selectLatest: false, allowAutoSelectWhenNone: false });
      setSelectedHistoryKey(null);
    } catch (err) {
      logToConsole("error", "Claude Provider 模型验证失败", {
        error: String(err),
        provider_id: curProvider.id,
      });
      toast(`验证失败：${String(err)}`);
    } finally {
      setValidating(false);
      setSuiteProgress(null);
    }
  }

  async function clearProviderHistory() {
    if (historyClearing) return;

    const curProvider = providerRef.current;
    if (!open || !curProvider) return;

    setHistoryClearing(true);
    try {
      // 防止“历史刷新 in-flight”在清空后把旧数据又写回到 UI。
      historyReqSeqRef.current += 1;
      setHistoryRuns([]);
      setSelectedHistoryKey(null);

      const ok = await claudeValidationHistoryClearProvider({ provider_id: curProvider.id });
      if (ok == null) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }
      if (!ok) {
        toast("清空失败");
        return;
      }

      toast("已清空历史");
      await refreshHistory({ selectLatest: true });
    } catch (err) {
      toast(`清空失败：${String(err)}`);
      void refreshHistory({ selectLatest: true });
    } finally {
      setHistoryClearing(false);
      setConfirmClearOpen(false);
    }
  }

  const title = provider ? `Claude Code · 模型验证：${provider.name}` : "Claude Code · 模型验证";

  type ClaudeModelValidationHistoryGroup = {
    key: string;
    suiteRunId: string | null;
    isSuite: boolean;
    createdAt: number;
    latestRunId: number;
    expectedTotal: number;
    missingCount: number;
    passCount: number;
    overallPass: boolean;
    modelName: string;
    runs: Array<{
      run: ClaudeModelValidationRunView;
      meta: SuiteMeta;
      evaluation: ReturnType<typeof evaluateClaudeValidation>;
    }>;
  };

  const historyGroups = useMemo((): ClaudeModelValidationHistoryGroup[] => {
    const groups = new Map<
      string,
      {
        key: string;
        suiteRunId: string | null;
        createdAt: number;
        latestRunId: number;
        runs: Array<{
          run: ClaudeModelValidationRunView;
          meta: SuiteMeta;
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
      const allPass =
        expectedTotal === evaluatedRuns.length &&
        evaluatedRuns.every((r) => r.evaluation.overallPass === true);

      const modelName =
        evaluatedRuns[evaluatedRuns.length - 1]?.evaluation.derived.modelName ??
        evaluatedRuns[0]?.evaluation.derived.modelName ??
        "—";

      out.push({
        key: group.key,
        suiteRunId: group.suiteRunId,
        isSuite: Boolean(group.suiteRunId),
        createdAt: group.createdAt,
        latestRunId: group.latestRunId,
        expectedTotal,
        missingCount: Math.max(0, expectedTotal - evaluatedRuns.length),
        passCount,
        overallPass: allPass,
        modelName,
        runs: evaluatedRuns,
      });
    }

    return out.sort((a, b) => b.latestRunId - a.latestRunId);
  }, [historyRuns]);

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

  return (
    <Dialog open={open} onOpenChange={handleOpenChange} title={title} className="max-w-6xl">
      {!provider ? (
        <div className="flex h-40 items-center justify-center text-sm text-slate-500">
          未选择服务商
        </div>
      ) : (
        <div className="space-y-6">
          {/* Provider Info Banner */}
          <div className="flex items-center justify-between rounded-xl border border-slate-200 bg-slate-50/50 px-4 py-3 text-sm backdrop-blur-sm">
            <div className="flex items-center gap-4 text-slate-700">
              <div className="flex items-center gap-2">
                <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-white shadow-sm ring-1 ring-slate-200">
                  <Server className="h-4 w-4 text-slate-500" />
                </div>
                <div>
                  <div className="text-xs text-slate-500">服务商</div>
                  <div className="font-semibold text-slate-900">{provider.name}</div>
                </div>
              </div>
              <div className="h-8 w-px bg-slate-200" />
              <div className="flex items-center gap-2">
                <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-white shadow-sm ring-1 ring-slate-200">
                  <Network className="h-4 w-4 text-slate-500" />
                </div>
                <div>
                  <div className="text-xs text-slate-500">模式</div>
                  <div className="flex items-center gap-1.5">
                    <span className="font-medium text-slate-900">
                      {provider.base_url_mode === "ping" ? "自动测速" : "顺序轮询"}
                    </span>
                    <span className="inline-flex items-center rounded-full bg-slate-100 px-1.5 py-0.5 text-[10px] text-slate-500">
                      {provider.base_urls.length} 个地址
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div className="grid gap-4 rounded-xl border border-slate-200 bg-slate-50/30 p-4 sm:grid-cols-12">
            <div className="sm:col-span-4">
              <FormField
                label="服务端点 (Endpoint)"
                hint={provider.base_url_mode === "ping" && baseUrlPicking ? "测试中..." : null}
              >
                <Select
                  value={baseUrl}
                  onChange={(e) => setBaseUrl(e.currentTarget.value)}
                  disabled={validating}
                  mono
                  className="h-9 bg-white text-xs"
                >
                  <option value="" disabled>
                    选择服务端点...
                  </option>
                  {provider.base_urls.map((url) => (
                    <option key={url} value={url}>
                      {url}
                    </option>
                  ))}
                </Select>
              </FormField>
            </div>

            <div className="sm:col-span-4">
              <FormField label="模型 (Model)" hint={modelPricesLoading ? "加载中..." : null}>
                <Select
                  value={model}
                  onChange={(e) => setModel(e.currentTarget.value)}
                  disabled={validating || modelPricesLoading || modelOptions.length === 0}
                  mono
                  className="h-9 bg-white text-xs"
                >
                  <option value="" disabled>
                    {modelOptions.length === 0 ? "无可用模型" : "选择模型..."}
                  </option>
                  {!modelOptions.includes(model) && model.trim() ? (
                    <option value={model}>{model} (当前)</option>
                  ) : null}
                  {modelOptions.map((m) => (
                    <option key={m} value={m}>
                      {m}
                    </option>
                  ))}
                </Select>
              </FormField>
            </div>

            <div className="flex items-end sm:col-span-4">
              <Button
                onClick={() => void runValidationSuite()}
                variant="primary"
                size="md"
                disabled={validating}
                className="w-full h-9 shadow-sm"
              >
                {validating ? (
                  <>
                    <RefreshCw className="mr-2 h-3.5 w-3.5 animate-spin" />
                    {suiteProgress
                      ? `执行中 (${suiteProgress.current}/${suiteProgress.total})...`
                      : "执行中..."}
                  </>
                ) : (
                  <>
                    <Play className="mr-2 h-3.5 w-3.5 fill-current" />
                    开始验证 ({templates.length})
                  </>
                )}
              </Button>
            </div>
          </div>

          <div className="grid gap-6 lg:grid-cols-7 h-[600px]">
            {/* Left Column: History List (2/7) */}
            <div className="flex flex-col gap-4 lg:col-span-2 h-full min-h-0">
              <Card padding="none" className="flex h-full flex-col overflow-hidden">
                <div className="flex items-center justify-between border-b border-slate-100 bg-slate-50/50 px-4 py-3">
                  <div className="flex items-center gap-2">
                    <History className="h-4 w-4 text-slate-500" />
                    <span className="text-sm font-semibold text-slate-900">历史记录</span>
                  </div>
                  <div className="flex items-center gap-1">
                    <Button
                      onClick={() => void refreshHistory({ selectLatest: false })}
                      variant="ghost"
                      size="sm"
                      className="h-8 w-8 p-0"
                      disabled={historyLoading || historyAvailable === false}
                      title="刷新"
                    >
                      <RefreshCw className={cn("h-4 w-4", historyLoading && "animate-spin")} />
                    </Button>
                    <Button
                      onClick={() => {
                        if (!provider) return;
                        setConfirmClearOpen(true);
                      }}
                      variant="ghost"
                      size="sm"
                      className="h-8 w-8 p-0 text-rose-500 hover:bg-rose-50 hover:text-rose-600"
                      disabled={historyLoading || historyAvailable === false || historyClearing}
                      title="清空历史"
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>

                <div className="flex-1 overflow-hidden">
                  {historyAvailable === false ? (
                    <div className="flex h-40 flex-col items-center justify-center gap-2 text-slate-400">
                      <Cpu className="h-8 w-8 text-slate-200" />
                      <span className="text-xs">仅限桌面端</span>
                    </div>
                  ) : historyLoading && historyGroups.length === 0 ? (
                    <div className="flex h-40 items-center justify-center text-xs text-slate-400">
                      加载中...
                    </div>
                  ) : historyGroups.length === 0 ? (
                    <div className="flex h-40 flex-col items-center justify-center gap-2 text-slate-400">
                      <History className="h-8 w-8 text-slate-200" />
                      <span className="text-xs">暂无历史记录</span>
                    </div>
                  ) : (
                    <div className="custom-scrollbar h-full overflow-y-auto p-3 space-y-2">
                      {historyGroups.map((group) => {
                        const active = group.key === selectedHistoryKey;
                        const mentionsBedrock = group.runs.some((r) =>
                          Boolean((r.run.parsed_result?.signals as any)?.mentions_amazon_bedrock)
                        );

                        return (
                          <div
                            key={group.key}
                            role="button"
                            tabIndex={0}
                            onClick={() => setSelectedHistoryKey(group.key)}
                            onKeyDown={(e) => {
                              if (e.key === "Enter" || e.key === " ")
                                setSelectedHistoryKey(group.key);
                            }}
                            className={cn(
                              "group relative rounded-xl border p-3 transition-all",
                              active
                                ? "border-indigo-500 bg-indigo-50/50 shadow-sm ring-1 ring-indigo-500/20"
                                : "border-slate-200 bg-white hover:border-indigo-200 hover:shadow-sm"
                            )}
                          >
                            <div className="flex items-start justify-between gap-3">
                              <div className="min-w-0 flex-1 space-y-1.5">
                                <div className="flex items-center gap-2">
                                  {group.overallPass ? (
                                    <CheckCircle2 className="h-4 w-4 text-emerald-500 shrink-0" />
                                  ) : (
                                    <XCircle className="h-4 w-4 text-rose-500 shrink-0" />
                                  )}
                                  <span className="font-mono text-xs font-semibold text-slate-700">
                                    #{group.latestRunId}
                                  </span>
                                  <span className="text-[10px] text-slate-400">
                                    {formatUnixSeconds(group.createdAt)}
                                  </span>
                                </div>

                                <div className="flex flex-wrap items-center gap-1.5">
                                  <span className="inline-flex items-center rounded-md bg-slate-100 px-1.5 py-0.5 text-[10px] font-medium text-slate-600 border border-slate-200">
                                    {group.modelName}
                                  </span>
                                  {group.isSuite && (
                                    <span className="inline-flex items-center rounded-md bg-indigo-50 px-1.5 py-0.5 text-[10px] font-medium text-indigo-600 border border-indigo-100">
                                      Suite
                                    </span>
                                  )}
                                  {mentionsBedrock && (
                                    <span className="inline-flex items-center rounded-md bg-rose-50 px-1.5 py-0.5 text-[10px] font-medium text-rose-600 border border-rose-100">
                                      Bedrock
                                    </span>
                                  )}
                                </div>

                                {group.isSuite && (
                                  <div className="flex items-center gap-0.5 mt-2">
                                    {group.runs.map((r, i) => (
                                      <div
                                        key={i}
                                        className={cn(
                                          "h-1.5 w-1.5 rounded-full",
                                          r.evaluation.overallPass
                                            ? "bg-emerald-400"
                                            : "bg-rose-400"
                                        )}
                                        title={`Step ${i + 1}: ${r.evaluation.overallPass ? "Pass" : "Fail"}`}
                                      />
                                    ))}
                                    {group.missingCount > 0 && (
                                      <div
                                        className="h-1.5 w-1.5 rounded-full bg-slate-200"
                                        title={`Missing ${group.missingCount}`}
                                      />
                                    )}
                                  </div>
                                )}
                              </div>
                              <ChevronRight
                                className={cn(
                                  "h-4 w-4 text-slate-300 transition-transform",
                                  active && "text-indigo-400"
                                )}
                              />
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>
              </Card>
            </div>

            {/* Right Column: Details Pane (5/7) */}
            <div className="flex flex-col gap-4 lg:col-span-5 h-full min-h-0 overflow-y-auto custom-scrollbar pr-1">
              <div className="flex flex-wrap items-start justify-between gap-2 border-b border-slate-100 pb-2">
                <div className="min-w-0">
                  <div className="flex items-center gap-2 text-sm font-semibold text-slate-900">
                    {suiteSteps.length > 0 && !selectedHistoryGroup ? (
                      <>
                        <Activity className="h-4 w-4 text-sky-500" />
                        当前运行
                      </>
                    ) : selectedHistoryGroup ? (
                      <>
                        <History className="h-4 w-4 text-slate-500" />
                        历史记录详情
                      </>
                    ) : (
                      <>
                        <Settings2 className="h-4 w-4 text-slate-400" />
                        准备就绪
                      </>
                    )}
                  </div>
                  <div className="mt-1 truncate text-xs text-slate-500">
                    {selectedHistoryGroup
                      ? selectedHistoryGroup.isSuite
                        ? `测试套件 #${selectedHistoryGroup.latestRunId} · ${formatUnixSeconds(selectedHistoryGroup.createdAt)}`
                        : `验证记录 #${selectedHistoryGroup.latestRunId} · ${formatUnixSeconds(selectedHistoryGroup.createdAt)}`
                      : suiteSteps.length > 0
                        ? `正在执行 ${suiteProgress?.current ?? 0}/${suiteSteps.length} 个模板...`
                        : activeResult
                          ? "最新结果 (未保存)"
                          : "等待验证..."}
                  </div>
                </div>
              </div>

              {suiteSteps.length > 0 && !selectedHistoryGroup ? (
                <div className="space-y-4">
                  {suiteSteps.map((step) => {
                    const statusLabel =
                      step.status === "running"
                        ? "执行中"
                        : step.status === "done"
                          ? "已完成"
                          : step.status === "error"
                            ? "失败"
                            : "待执行";
                    const statusClass =
                      step.status === "running"
                        ? "bg-sky-100 text-sky-700"
                        : step.status === "done"
                          ? "bg-emerald-100 text-emerald-700"
                          : step.status === "error"
                            ? "bg-rose-100 text-rose-700"
                            : "bg-slate-100 text-slate-600";

                    return (
                      <ClaudeModelValidationHistoryStepCard
                        key={`${step.templateKey}_${step.index}`}
                        title={`验证 ${step.index}/${suiteSteps.length}：${step.label}`}
                        rightBadge={
                          <span
                            className={cn(
                              "rounded px-1.5 py-0.5 text-[10px] font-semibold",
                              statusClass
                            )}
                          >
                            {statusLabel}
                          </span>
                        }
                        templateKey={step.templateKey}
                        result={step.result}
                        requestJsonText={step.request_json ?? ""}
                        resultJsonText={step.result_json ?? ""}
                        sseRawText={step.result?.raw_excerpt ?? ""}
                        errorText={step.error}
                        copyText={copyTextOrToast}
                      />
                    );
                  })}
                </div>
              ) : selectedHistoryGroup?.isSuite ? (
                <div className="space-y-4">
                  {(() => {
                    const expectedTotal = selectedHistoryGroup.expectedTotal;
                    const expectedKeys = templates
                      .filter(
                        (t) =>
                          getClaudeTemplateApplicability(t, selectedHistoryGroup.modelName)
                            .applicable
                      )
                      .map((t) => t.key);

                    const byIndex = new Map<number, (typeof selectedHistoryGroup.runs)[number]>();
                    for (const r of selectedHistoryGroup.runs) {
                      const idx = r.meta.suiteStepIndex ?? 0;
                      if (!Number.isFinite(idx) || idx <= 0) continue;
                      const prev = byIndex.get(idx);
                      if (!prev || r.run.id > prev.run.id) byIndex.set(idx, r);
                    }

                    const out: ReactElement[] = [];
                    for (let idx = 1; idx <= expectedTotal; idx += 1) {
                      const step = byIndex.get(idx) ?? null;
                      const expectedKey = expectedKeys[idx - 1] ?? step?.evaluation.templateKey;
                      const templateKeyForUi = (expectedKey ??
                        DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY) as ClaudeValidationTemplateKey;
                      const template = getClaudeValidationTemplate(templateKeyForUi);

                      const title = `${idx}/${expectedTotal}`;
                      out.push(
                        <ClaudeModelValidationHistoryStepCard
                          key={
                            step
                              ? `${selectedHistoryGroup.key}_${step.run.id}`
                              : `${selectedHistoryGroup.key}_missing_${idx}`
                          }
                          title={`验证 ${title}：${template.label}`}
                          rightBadge={
                            step ? (
                              <OutcomePill pass={step.evaluation.overallPass} />
                            ) : (
                              <span className="rounded bg-slate-100 px-1.5 py-0.5 text-[10px] font-semibold text-slate-600">
                                未记录
                              </span>
                            )
                          }
                          templateKey={templateKeyForUi}
                          result={step?.run.parsed_result ?? null}
                          requestJsonText={step?.run.request_json ?? ""}
                          resultJsonText={prettyJsonOrFallback(step?.run.result_json ?? "")}
                          sseRawText={step?.run.parsed_result?.raw_excerpt ?? ""}
                          errorText={
                            step
                              ? null
                              : "该步骤未出现在历史中：可能是历史写入失败、被清空，或被保留数量上限淘汰。请在“当前运行”查看完整诊断。"
                          }
                          copyText={copyTextOrToast}
                        />
                      );
                    }
                    return out;
                  })()}
                </div>
              ) : selectedHistoryGroup ? (
                selectedHistoryLatest ? (
                  <ClaudeModelValidationHistoryStepCard
                    title={`验证：${selectedHistoryLatest.evaluation.template.label}`}
                    rightBadge={<OutcomePill pass={selectedHistoryLatest.evaluation.overallPass} />}
                    templateKey={selectedHistoryLatest.evaluation.templateKey}
                    result={selectedHistoryLatest.run.parsed_result}
                    requestJsonText={selectedHistoryLatest.run.request_json ?? ""}
                    resultJsonText={prettyJsonOrFallback(
                      selectedHistoryLatest.run.result_json ?? ""
                    )}
                    sseRawText={selectedHistoryLatest.run.parsed_result?.raw_excerpt ?? ""}
                    copyText={copyTextOrToast}
                  />
                ) : (
                  <div className="flex h-40 items-center justify-center text-xs text-slate-400">
                    暂无历史数据
                  </div>
                )
              ) : (
                <ClaudeModelValidationResultPanel
                  templateKey={activeResultTemplateKey}
                  result={activeResult}
                />
              )}

              {!selectedHistoryGroup && suiteSteps.length === 0 ? (
                <details className="group rounded-xl border border-slate-200 bg-white shadow-sm open:ring-2 open:ring-indigo-500/10 transition-all">
                  <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
                    <div className="flex items-center gap-2 text-sm font-medium text-slate-700 group-open:text-indigo-600">
                      <Settings2 className="h-4 w-4" />
                      <span>高级请求配置</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <Button
                        onClick={(e) => {
                          stopDetailsToggle(e);
                          return void copyTextOrToast(requestJson ?? "", "已复制请求 JSON");
                        }}
                        variant="ghost"
                        size="sm"
                        className="h-8 w-8 p-0"
                        disabled={!(requestJson ?? "").trim()}
                        title="复制请求 JSON"
                        aria-label="复制请求 JSON"
                      >
                        <FileJson className="h-4 w-4" />
                      </Button>
                      <ChevronDown className="h-4 w-4 text-slate-400 transition-transform group-open:rotate-180" />
                    </div>
                  </summary>

                  <div className="border-t border-slate-100 px-4 py-3">
                    <Textarea
                      mono
                      className="h-[220px] resize-none text-xs leading-5 bg-white shadow-sm focus:ring-indigo-500"
                      value={requestJson}
                      onChange={(e) => {
                        setRequestJson(e.currentTarget.value);
                        setRequestDirty(true);
                      }}
                      placeholder='{"template_key":"official_max_tokens_5","headers":{...},"body":{...},"expect":{...}}'
                    />
                  </div>
                </details>
              ) : null}

              {!selectedHistoryGroup && suiteSteps.length === 0 ? (
                <details className="group rounded-xl border border-slate-200 bg-white shadow-sm open:ring-2 open:ring-indigo-500/10 transition-all">
                  <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
                    <div className="flex items-center gap-2 text-sm font-medium text-slate-700 group-open:text-indigo-600">
                      <Activity className="h-4 w-4" />
                      <span>SSE 流式响应预览</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <Button
                        onClick={(e) => {
                          stopDetailsToggle(e);
                          return void copyTextOrToast(
                            activeResult?.raw_excerpt ?? "",
                            "已复制 SSE 原文"
                          );
                        }}
                        variant="ghost"
                        size="sm"
                        className="h-8 w-8 p-0"
                        disabled={!(activeResult?.raw_excerpt ?? "").trim()}
                        title="复制 SSE 原文"
                        aria-label="复制 SSE 原文"
                      >
                        <Copy className="h-4 w-4" />
                      </Button>
                      <ChevronDown className="h-4 w-4 text-slate-400 transition-transform group-open:rotate-180" />
                    </div>
                  </summary>
                  <div className="border-t border-slate-100 p-0">
                    <pre className="custom-scrollbar max-h-60 overflow-auto bg-slate-950 p-4 font-mono text-[10px] leading-relaxed text-slate-300">
                      <span className="text-slate-500">
                        {(() => {
                          const t = getClaudeValidationTemplate(activeResultTemplateKey);
                          return `// SSE: ${t.label} (${t.key})`;
                        })()}
                        {"\n"}
                      </span>
                      {activeResult?.raw_excerpt || (
                        <span className="text-slate-600 italic">// 暂无 SSE 数据</span>
                      )}
                    </pre>
                  </div>
                </details>
              ) : null}
            </div>
          </div>
        </div>
      )}

      {confirmClearOpen && typeof document !== "undefined"
        ? createPortal(
            <div className="fixed inset-0 z-[60]">
              <div
                className="absolute inset-0 bg-black/40"
                onClick={() => {
                  if (historyClearing) return;
                  setConfirmClearOpen(false);
                }}
              />
              <div className="absolute inset-0 flex items-center justify-center p-4">
                <div className="w-full max-w-md overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-card">
                  <div className="border-b border-slate-200 px-5 py-4">
                    <div className="text-sm font-semibold text-slate-900">确认清空历史？</div>
                    <div className="mt-1 text-xs text-slate-600">
                      将清空{" "}
                      <span className="font-medium text-slate-900">
                        {provider?.name ?? "当前 Provider"}
                      </span>{" "}
                      的模型验证历史（按 provider_id 隔离），且无法撤销。
                    </div>
                  </div>
                  <div className="flex items-center justify-end gap-2 px-5 py-4">
                    <Button
                      variant="secondary"
                      size="md"
                      disabled={historyClearing}
                      onClick={() => setConfirmClearOpen(false)}
                    >
                      取消
                    </Button>
                    <Button
                      variant="danger"
                      size="md"
                      disabled={historyClearing}
                      onClick={() => void clearProviderHistory()}
                    >
                      {historyClearing ? "清空中…" : "确认清空"}
                    </Button>
                  </div>
                </div>
              </div>
            </div>,
            document.body
          )
        : null}
    </Dialog>
  );
}
