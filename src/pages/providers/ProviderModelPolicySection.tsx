import { ArrowRight, ChevronDown, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Button } from "../../ui/Button";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { RadioGroup } from "../../ui/RadioGroup";
import type {
  ClaudeModels,
  CliKey,
  ProviderModelPolicyStatus,
  ProviderModelPolicyV1,
} from "../../services/providers/providers";
import {
  cloneProviderModelPolicy,
  DEFAULT_PROVIDER_MODEL_POLICY,
  type ProviderModelDiscoveryUiState,
  normalizeProviderModelPolicyDraft,
  validateProviderModelPolicy,
} from "./providerModelPolicy";

export type ProviderModelPolicySectionProps = {
  cliKey: CliKey;
  status: ProviderModelPolicyStatus;
  policy: ProviderModelPolicyV1 | null;
  legacyClaudeModels?: ClaudeModels | null;
  saving: boolean;
  onChange: (policy: ProviderModelPolicyV1) => void;
  modelDiscoveryState: ProviderModelDiscoveryUiState;
  onDiscoverModels: () => void | Promise<void>;
  hasMultipleBaseUrls: boolean;
  // cx2cc providers map models via CX2CC 模型映射; hide the generic mapping editor there.
  showMappings?: boolean;
};

const MODE_OPTIONS = [
  {
    value: "all",
    label: "全部可用",
    description: "接收所有模型；列出常用模型可让它们优先走本供应商",
  },
  { value: "selected", label: "仅这些可用", description: "只接收列出或映射的模型" },
  { value: "excluded", label: "排除这些", description: "列出的模型不可用，其余模型默认可用" },
];

export function ProviderModelPolicySection({
  cliKey,
  status,
  policy,
  legacyClaudeModels,
  saving,
  onChange,
  modelDiscoveryState,
  onDiscoverModels,
  hasMultipleBaseUrls,
  showMappings = true,
}: ProviderModelPolicySectionProps) {
  const [localDraft, setLocalDraft] = useState<ProviderModelPolicyV1>(() =>
    cloneProviderModelPolicy(policy ?? DEFAULT_PROVIDER_MODEL_POLICY)
  );
  const [patternInput, setPatternInput] = useState("");
  const [mappingSourceInput, setMappingSourceInput] = useState("");
  const [mappingTargetInput, setMappingTargetInput] = useState("");
  const [editingLegacy, setEditingLegacy] = useState(false);
  const [showCutoverWarning, setShowCutoverWarning] = useState(false);
  const patternRefs = useRef<Record<number, HTMLInputElement | null>>({});
  const mappingRefs = useRef<Record<number, HTMLInputElement | null>>({});
  const patternComposerRef = useRef<HTMLInputElement | null>(null);
  const mappingComposerRef = useRef<HTMLInputElement | null>(null);
  const focusRef = useRef<{ kind: "pattern" | "mapping"; index: number } | null>(null);
  useEffect(() => {
    if (status === "ready" && policy) setLocalDraft(cloneProviderModelPolicy(policy));
    const focus = focusRef.current;
    if (!focus) return;
    (focus.kind === "pattern" ? patternRefs : mappingRefs).current[focus.index]?.focus();
    focusRef.current = null;
  }, [policy, status]);

  const currentPolicy = status === "ready" && policy ? policy : localDraft;
  const candidateModels = modelDiscoveryState.status === "ready" ? modelDiscoveryState.models : [];
  const candidateListId = `${cliKey}-provider-model-candidates`;
  const policyError = validateProviderModelPolicy(currentPolicy);

  const emit = (next: ProviderModelPolicyV1) => {
    const normalized = normalizeProviderModelPolicyDraft(next);
    setLocalDraft(normalized);
    onChange(normalized);
  };

  const updatePattern = (index: number, value: string) => {
    emit({
      ...currentPolicy,
      modelPatterns: currentPolicy.modelPatterns.map((pattern, patternIndex) =>
        patternIndex === index ? value : pattern
      ),
    });
  };

  const addPattern = (rawValue = patternInput) => {
    const value = rawValue.trim();
    if (!value) return;
    if (!currentPolicy.modelPatterns.some((pattern) => pattern.trim() === value)) {
      emit({ ...currentPolicy, modelPatterns: [...currentPolicy.modelPatterns, value] });
    }
    setPatternInput("");
  };

  const deletePattern = (index: number) => {
    const modelPatterns = currentPolicy.modelPatterns.filter(
      (_, patternIndex) => patternIndex !== index
    );
    emit({ ...currentPolicy, modelPatterns });
    if (modelPatterns.length === 0) patternComposerRef.current?.focus();
    else focusRef.current = { kind: "pattern", index: Math.min(index, modelPatterns.length - 1) };
  };

  const updateMapping = (index: number, field: "source" | "target", value: string) => {
    emit({
      ...currentPolicy,
      mappings: currentPolicy.mappings.map((mapping, mappingIndex) =>
        mappingIndex === index ? { ...mapping, [field]: value } : mapping
      ),
    });
  };

  const addMapping = () => {
    const source = mappingSourceInput.trim();
    const target = mappingTargetInput.trim();
    if (!source || !target) return;
    emit({
      ...currentPolicy,
      mappings: [...currentPolicy.mappings, { source, target }],
    });
    setMappingSourceInput("");
    setMappingTargetInput("");
    mappingComposerRef.current?.focus();
  };

  const deleteMapping = (index: number) => {
    const mappings = currentPolicy.mappings.filter((_, mappingIndex) => mappingIndex !== index);
    emit({ ...currentPolicy, mappings });
    if (mappings.length === 0) mappingComposerRef.current?.focus();
    else focusRef.current = { kind: "mapping", index: Math.min(index, mappings.length - 1) };
  };

  const legacyMappings = [
    ["主模型", legacyClaudeModels?.main_model],
    ["推理模型 (Thinking)", legacyClaudeModels?.reasoning_model],
    ["Haiku", legacyClaudeModels?.haiku_model],
    ["Sonnet", legacyClaudeModels?.sonnet_model],
    ["Opus", legacyClaudeModels?.opus_model],
  ].filter(([, value]) => typeof value === "string" && value.trim());

  const enterGenericPolicy = () => {
    setEditingLegacy(true);
    setShowCutoverWarning(true);
    emit(cloneProviderModelPolicy(DEFAULT_PROVIDER_MODEL_POLICY));
  };

  const resetInvalidPolicy = () => {
    setEditingLegacy(true);
    emit(cloneProviderModelPolicy(DEFAULT_PROVIDER_MODEL_POLICY));
  };

  const discoveryRow = (legacy: boolean) => (
    <div className="flex flex-wrap items-center justify-between gap-2">
      <p role="status" aria-live="polite" className="min-w-0 text-xs text-muted-foreground">
        {discoveryMessage(modelDiscoveryState)}
        {discoveryEndpoint(modelDiscoveryState)}
        {hasMultipleBaseUrls ? " · 多地址建议拆分 Provider" : ""}
        {legacy && modelDiscoveryState.status === "ready" ? " · 切换后可选" : ""}
      </p>
      <Button
        type="button"
        variant="secondary"
        onClick={() => void onDiscoverModels()}
        disabled={saving || modelDiscoveryState.status === "loading"}
        aria-busy={modelDiscoveryState.status === "loading"}
      >
        <RefreshCw
          className={`h-4 w-4 ${modelDiscoveryState.status === "loading" ? "animate-spin" : ""}`}
          aria-hidden="true"
        />
        获取上游模型
      </Button>
    </div>
  );

  return (
    <details
      data-cli-key={cliKey}
      className="group rounded-lg border border-border bg-surface-panel shadow-sm open:ring-2 open:ring-ring/10"
    >
      <summary className="flex cursor-pointer list-none items-center justify-between gap-3 px-4 py-3 select-none">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <span className="text-sm font-semibold text-foreground">模型路由</span>
          <span className="text-xs text-muted-foreground">
            {policySummary(status, currentPolicy, showMappings)}
          </span>
        </div>
        <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground transition-transform group-open:rotate-180" />
      </summary>

      <div className="space-y-4 border-t border-border px-4 py-3">
        {status === "legacy" && !editingLegacy ? (
          <div className="space-y-3 text-sm text-muted-foreground">
            <p className="font-medium text-foreground">当前 Claude 使用旧版模型映射</p>
            <div aria-label="旧版模型映射摘要" className="space-y-1 text-xs">
              {legacyMappings.length > 0 ? (
                <ul className="space-y-1">
                  {legacyMappings.map(([label, value]) => (
                    <li key={label} className="flex flex-wrap gap-x-2">
                      <span>{label}：</span>
                      <code className="break-all text-foreground">{value}</code>
                    </li>
                  ))}
                </ul>
              ) : (
                <p>未配置，沿用请求模型。</p>
              )}
            </div>
            {discoveryRow(true)}
            <Button
              type="button"
              variant="secondary"
              onClick={enterGenericPolicy}
              disabled={saving}
            >
              改用通用模型策略
            </Button>
          </div>
        ) : status === "invalid" && !editingLegacy ? (
          <div
            role="alert"
            className="space-y-3 rounded-md border border-warning/40 bg-warning/10 p-3 text-sm text-foreground"
          >
            <p>模型策略无效，当前请求不会使用该 Provider。</p>
            {discoveryRow(false)}
            <Button
              type="button"
              variant="secondary"
              onClick={resetInvalidPolicy}
              disabled={saving}
            >
              重置为全部可用
            </Button>
          </div>
        ) : (
          <>
            <datalist id={candidateListId}>
              {candidateModels.map((model) => (
                <option key={model} value={model} />
              ))}
            </datalist>

            {showCutoverWarning ? (
              <p
                role="alert"
                className="rounded-md border border-warning/40 bg-warning/10 p-3 text-sm text-foreground"
              >
                保存后旧版映射不再生效，且无法在界面切回旧策略
              </p>
            ) : null}

            {editingLegacy && legacyMappings.length > 0 && showMappings ? (
              <div
                aria-label="旧版模型映射参考"
                className="rounded-md border border-border p-3 text-xs text-muted-foreground"
              >
                <p className="mb-1 font-medium text-foreground">
                  旧版映射（供照抄为下方的模型映射）
                </p>
                <ul className="space-y-1">
                  {legacyMappings.map(([label, value]) => (
                    <li key={label} className="flex flex-wrap gap-x-2">
                      <span>{label}：</span>
                      <code className="break-all text-foreground">{value}</code>
                    </li>
                  ))}
                </ul>
              </div>
            ) : null}

            <section className="space-y-3" aria-labelledby={`${cliKey}-model-range-title`}>
              <div className="space-y-1">
                <h3
                  id={`${cliKey}-model-range-title`}
                  className="text-sm font-semibold text-foreground"
                >
                  模型范围
                </h3>
                <p className="text-xs text-muted-foreground">{modeHint(currentPolicy.mode)}</p>
              </div>

              <RadioGroup
                name={`${cliKey}-provider-model-mode`}
                ariaLabel="模型范围"
                value={currentPolicy.mode}
                onChange={(mode) =>
                  emit({
                    ...currentPolicy,
                    mode: mode as ProviderModelPolicyV1["mode"],
                  })
                }
                options={MODE_OPTIONS}
                disabled={saving}
              />

              {discoveryRow(false)}

              <div className="flex flex-col gap-2 sm:flex-row sm:items-end">
                <FormField label={rangeAddLabel(currentPolicy.mode)}>
                  <Input
                    ref={patternComposerRef}
                    aria-label={`新增${rangeItemLabel(currentPolicy.mode)}`}
                    list={candidateListId}
                    value={patternInput}
                    onChange={(event) => setPatternInput(event.currentTarget.value)}
                    onKeyDown={(event) => {
                      if (event.key !== "Enter") return;
                      event.preventDefault();
                      addPattern();
                    }}
                    placeholder="例如 gpt-5.6-luna 或 gpt-*"
                    disabled={saving}
                    mono
                  />
                </FormField>
                <Button
                  type="button"
                  variant="secondary"
                  onClick={() => addPattern()}
                  disabled={saving || !patternInput.trim()}
                >
                  <Plus className="h-4 w-4" aria-hidden="true" />
                  {rangeAddLabel(currentPolicy.mode)}
                </Button>
              </div>

              <div className="space-y-2">
                <p className="text-xs font-semibold text-muted-foreground">
                  {rangeListLabel(currentPolicy.mode)}
                </p>
                {currentPolicy.modelPatterns.length === 0 ? (
                  <p className="text-sm text-muted-foreground">
                    暂无{rangeListLabel(currentPolicy.mode)}
                  </p>
                ) : null}
                {currentPolicy.modelPatterns.map((pattern, index) => (
                  <div key={index} className="flex items-end gap-2">
                    <FormField label={`${rangeItemLabel(currentPolicy.mode)} ${index + 1}`}>
                      <Input
                        ref={(element) => {
                          patternRefs.current[index] = element;
                        }}
                        aria-label={`${rangeItemLabel(currentPolicy.mode)} ${index + 1}`}
                        value={pattern}
                        onChange={(event) => updatePattern(index, event.currentTarget.value)}
                        placeholder="例如 gpt-5.6-luna 或 gpt-*"
                        disabled={saving}
                        mono
                      />
                    </FormField>
                    <Button
                      type="button"
                      variant="secondary"
                      size="icon"
                      className="h-10 w-10 shrink-0"
                      aria-label={`删除${rangeItemLabel(currentPolicy.mode)} ${index + 1}`}
                      title="删除"
                      onClick={() => deletePattern(index)}
                      disabled={saving}
                    >
                      <Trash2 className="h-4 w-4" aria-hidden="true" />
                    </Button>
                  </div>
                ))}
              </div>
            </section>

            {showMappings ? (
              <section
                className="space-y-3 border-t border-border pt-4"
                aria-labelledby={`${cliKey}-model-mapping-title`}
              >
                <h3
                  id={`${cliKey}-model-mapping-title`}
                  className="text-sm font-semibold text-foreground"
                >
                  模型映射（可选）
                </h3>

                <div className="grid grid-cols-1 gap-2 md:grid-cols-[minmax(0,1fr)_1rem_minmax(0,1fr)_auto] md:items-end">
                  <FormField label="请求模型">
                    <Input
                      ref={mappingComposerRef}
                      aria-label="映射请求模型"
                      list={candidateListId}
                      value={mappingSourceInput}
                      onChange={(event) => setMappingSourceInput(event.currentTarget.value)}
                      onKeyDown={(event) => {
                        if (event.key !== "Enter") return;
                        event.preventDefault();
                        addMapping();
                      }}
                      placeholder="例如 gpt-5.6-luna"
                      disabled={saving}
                      mono
                    />
                  </FormField>
                  <ArrowRight
                    className="mb-3 hidden h-4 w-4 text-muted-foreground md:block"
                    aria-hidden="true"
                  />
                  <FormField label="上游模型">
                    <Input
                      aria-label="映射上游模型"
                      list={candidateListId}
                      value={mappingTargetInput}
                      onChange={(event) => setMappingTargetInput(event.currentTarget.value)}
                      onKeyDown={(event) => {
                        if (event.key !== "Enter") return;
                        event.preventDefault();
                        addMapping();
                      }}
                      placeholder="例如 deepseek-v4-flash"
                      disabled={saving}
                      mono
                    />
                  </FormField>
                  <Button
                    type="button"
                    variant="secondary"
                    onClick={addMapping}
                    disabled={saving || !mappingSourceInput.trim() || !mappingTargetInput.trim()}
                  >
                    <Plus className="h-4 w-4" aria-hidden="true" />
                    添加映射
                  </Button>
                </div>

                {currentPolicy.mappings.length === 0 ? (
                  <p className="text-sm text-muted-foreground">暂无模型映射</p>
                ) : null}
                <div className="space-y-2">
                  {currentPolicy.mappings.map((mapping, index) => (
                    <div
                      key={index}
                      className="grid grid-cols-1 gap-2 md:grid-cols-[minmax(0,1fr)_1rem_minmax(0,1fr)_2.5rem] md:items-end"
                    >
                      <FormField label="请求模型">
                        <Input
                          ref={(element) => {
                            mappingRefs.current[index] = element;
                          }}
                          aria-label={`请求模型 ${index + 1}`}
                          value={mapping.source}
                          onChange={(event) =>
                            updateMapping(index, "source", event.currentTarget.value)
                          }
                          placeholder="例如 gpt-5.6-luna"
                          disabled={saving}
                          mono
                        />
                      </FormField>
                      <ArrowRight
                        className="mb-3 hidden h-4 w-4 text-muted-foreground md:block"
                        aria-hidden="true"
                      />
                      <FormField label="上游模型">
                        <Input
                          aria-label={`上游模型 ${index + 1}`}
                          value={mapping.target}
                          onChange={(event) =>
                            updateMapping(index, "target", event.currentTarget.value)
                          }
                          placeholder="例如 deepseek-v4-flash"
                          disabled={saving}
                          mono
                        />
                      </FormField>
                      <Button
                        type="button"
                        variant="secondary"
                        size="icon"
                        className="h-10 w-10 justify-self-end"
                        aria-label={`删除模型映射 ${index + 1}`}
                        title="删除映射"
                        onClick={() => deleteMapping(index)}
                        disabled={saving}
                      >
                        <Trash2 className="h-4 w-4" aria-hidden="true" />
                      </Button>
                    </div>
                  ))}
                </div>
              </section>
            ) : null}

            {policyError ? (
              <p role="alert" className="text-xs text-destructive">
                {policyError}
              </p>
            ) : null}
          </>
        )}
      </div>
    </details>
  );
}

function policySummary(
  status: ProviderModelPolicyStatus,
  policy: ProviderModelPolicyV1,
  showMappings: boolean
) {
  if (status === "legacy") return "旧版";
  if (status === "invalid") return "无效";
  // Hidden mapping editor (cx2cc) → mappings are neither editable nor applied;
  // counting them in the summary would advertise a knob that does nothing.
  const mapping = showMappings && policy.mappings.length ? ` · 映射 ${policy.mappings.length}` : "";
  if (policy.mode === "all") {
    const explicit = policy.modelPatterns.length ? ` · 显式 ${policy.modelPatterns.length}` : "";
    return `全部可用${explicit}${mapping}`;
  }
  if (policy.mode === "selected") {
    return `仅 ${new Set([...policy.modelPatterns, ...policy.mappings.map((item) => item.source)]).size} 个模型${mapping}`;
  }
  return `排除 ${policy.modelPatterns.length} 个${mapping}`;
}

function modeHint(mode: ProviderModelPolicyV1["mode"]) {
  if (mode === "all")
    return "未列出的模型也可用。一个模型只要被任何供应商显式列出，请求它时就只走列出它的供应商。";
  if (mode === "selected") return "只接收下列模型和映射中的请求模型。";
  return "下列模型不可用；其余模型保持可用。";
}

function rangeListLabel(mode: ProviderModelPolicyV1["mode"]) {
  if (mode === "all") return "显式模型";
  if (mode === "selected") return "可用模型";
  return "排除模型";
}

function rangeItemLabel(mode: ProviderModelPolicyV1["mode"]) {
  if (mode === "all") return "显式模型";
  if (mode === "selected") return "可用模型";
  return "排除模型";
}

function rangeAddLabel(mode: ProviderModelPolicyV1["mode"]) {
  if (mode === "all") return "添加显式模型";
  if (mode === "selected") return "添加可用模型";
  return "添加排除模型";
}

function discoveryEndpoint(state: ProviderModelDiscoveryUiState) {
  if (state.status !== "ready" && state.status !== "empty") {
    return "";
  }
  const index = state.baseUrlIndex == null ? "" : ` · 地址 ${state.baseUrlIndex}`;
  return ` · ${state.origin}${index}`;
}

function discoveryMessage(state: ProviderModelDiscoveryUiState) {
  switch (state.status) {
    case "idle":
      return "尚未获取上游模型";
    case "loading":
      return "正在获取上游模型…";
    case "changed":
      return "连接已变化，请重新获取";
    case "ready":
      return `已获取 ${state.models.length} 个候选`;
    case "empty":
      return "上游未返回模型";
    case "unsupported":
      return state.reason === "cx_2cc"
        ? "CX2CC 请在对应 Codex Provider 获取"
        : "当前 OAuth 连接不支持获取";
    case "oauth_unsaved":
      return "请先保存并完成 OAuth 登录后再获取";
    case "error": {
      if (state.httpStatus === 429) return "上游限流（HTTP 429），请稍后重试";
      if (state.httpStatus != null && state.httpStatus >= 500) {
        return `上游服务异常（HTTP ${state.httpStatus}），请稍后重试`;
      }
      if (state.httpStatus === 401 || state.httpStatus === 403) {
        return `认证失败（HTTP ${state.httpStatus}），请检查 API Key 或 OAuth 登录状态`;
      }
      if (state.httpStatus != null && state.httpStatus >= 300) {
        return state.code === "redirect"
          ? `端点发生重定向（HTTP ${state.httpStatus}），请配置最终 endpoint`
          : `上游请求失败（HTTP ${state.httpStatus}）`;
      }
      return {
        invalid_config: "连接配置不完整，请检查 Base URL、认证方式和 API Key",
        redirect: "端点发生重定向，请配置最终 endpoint",
        unauthorized: "认证失败，请检查 API Key 或 OAuth 登录状态",
        timeout: "获取超时，请重试",
        network: "无法连接上游，请检查 endpoint、代理和网络",
        invalid_response: "上游模型目录格式无法使用",
        too_large: "上游模型目录响应超过 8 MiB",
      }[state.code];
    }
    case "unexpected_error":
      return "获取失败，请查看应用日志后重试";
  }
}
