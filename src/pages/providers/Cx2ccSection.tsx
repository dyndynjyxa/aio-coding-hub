import { useEffect, useState } from "react";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { TagsField } from "./TagsField";
import { CX2CC_GLOBAL_SOURCE_VALUE, CX2CC_PROXY_TOKEN } from "./providerEditorUtils";
import type { UseProviderEditorFormReturn } from "./useProviderEditorForm";

const CX2CC_DEFAULT_MODEL = "gpt-5.5";
const CX2CC_DEFAULT_MODEL_OPTIONS = [CX2CC_DEFAULT_MODEL, "gpt-5.4"] as const;
const CX2CC_MANUAL_MODEL_VALUE = "__manual__";
const CX2CC_MODEL_MAPPING_KEYS = [
  "main_model",
  "reasoning_model",
  "haiku_model",
  "sonnet_model",
  "opus_model",
] as const;
type Cx2ccFallbackModels = { main: string; haiku: string; sonnet: string; opus: string } | null;

export function Cx2ccSection(props: { form: UseProviderEditorFormReturn }) {
  const {
    register,
    saving,
    tags,
    setTags,
    tagInput,
    setTagInput,
    cx2ccSourceValue,
    setCx2ccSourceValue,
    isCodexGatewaySource,
    selectedCx2ccSourceProvider,
    codexGatewayBaseUrl,
    cx2ccFallbackModels,
    claudeModels,
    setClaudeModels,
    codexProviders,
  } = props.form;
  const [manualModelSelected, setManualModelSelected] = useState(false);
  const selectedDefaultModel = manualModelSelected
    ? CX2CC_MANUAL_MODEL_VALUE
    : resolveCx2ccDefaultModelSelectValue(claudeModels);
  const defaultModelOptions =
    selectedDefaultModel !== CX2CC_MANUAL_MODEL_VALUE &&
    !CX2CC_DEFAULT_MODEL_OPTIONS.includes(
      selectedDefaultModel as (typeof CX2CC_DEFAULT_MODEL_OPTIONS)[number]
    )
      ? ([selectedDefaultModel, ...CX2CC_DEFAULT_MODEL_OPTIONS] as const)
      : CX2CC_DEFAULT_MODEL_OPTIONS;

  useEffect(() => {
    setClaudeModels((prev) => {
      if (normalizeModelName(prev.main_model)) return prev;
      return withCx2ccDefaultModel(prev, CX2CC_DEFAULT_MODEL);
    });
  }, [setClaudeModels]);

  return (
    <>
      <div className="grid gap-3 sm:grid-cols-2">
        <FormField label="名称">
          <Input placeholder="default" {...register("name")} />
        </FormField>

        <TagsField
          tags={tags}
          setTags={setTags}
          tagInput={tagInput}
          setTagInput={setTagInput}
          saving={saving}
        />
      </div>

      <FormField label="备注">
        <Input placeholder="可选备注信息" disabled={saving} {...register("note")} />
      </FormField>

      <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_minmax(12rem,16rem)]">
        <FormField label="源 Codex 来源">
          {(id) => (
            <Select
              id={id}
              value={cx2ccSourceValue}
              onChange={(e) => {
                setCx2ccSourceValue(e.target.value);
              }}
              disabled={saving}
              className="w-full"
            >
              <option value="">请选择 Codex 来源…</option>
              <option value={CX2CC_GLOBAL_SOURCE_VALUE}>
                当前 AIO 服务 Codex 网关（跟随当前分流）
              </option>
              {codexProviders
                .filter((p) => p.enabled && p.source_provider_id == null && p.bridge_type == null)
                .map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name} ({p.auth_mode === "oauth" ? "OAuth" : "API Key"})
                  </option>
                ))}
            </Select>
          )}
        </FormField>

        <FormField label="默认模型" hint="用于 CX2CC 转译">
          {(id) => (
            <Select
              id={id}
              value={selectedDefaultModel}
              onChange={(e) => {
                const value = e.currentTarget.value;
                if (value === CX2CC_MANUAL_MODEL_VALUE) {
                  setManualModelSelected(true);
                  return;
                }
                setManualModelSelected(false);
                setClaudeModels((prev) => ({
                  ...prev,
                  main_model: value,
                  reasoning_model: value,
                  haiku_model: value,
                  sonnet_model: value,
                  opus_model: value,
                }));
              }}
              disabled={saving}
              mono
            >
              <option value={CX2CC_MANUAL_MODEL_VALUE}>手动</option>
              {defaultModelOptions.map((model) => (
                <option key={model} value={model}>
                  {model.toUpperCase()}
                </option>
              ))}
            </Select>
          )}
        </FormField>
      </div>

      <div>
        {isCodexGatewaySource ? (
          <Cx2ccGatewaySourceInfo
            codexGatewayBaseUrl={codexGatewayBaseUrl}
            cx2ccFallbackModels={cx2ccFallbackModels}
            claudeModels={claudeModels}
          />
        ) : selectedCx2ccSourceProvider ? (
          <Cx2ccProviderSourceInfo
            provider={selectedCx2ccSourceProvider}
            cx2ccFallbackModels={cx2ccFallbackModels}
            claudeModels={claudeModels}
          />
        ) : null}
      </div>
    </>
  );
}

function Cx2ccGatewaySourceInfo(props: {
  codexGatewayBaseUrl: string;
  cx2ccFallbackModels: Cx2ccFallbackModels;
  claudeModels: UseProviderEditorFormReturn["claudeModels"];
}) {
  const { codexGatewayBaseUrl, cx2ccFallbackModels, claudeModels } = props;

  return (
    <div className="rounded-md border border-border bg-secondary px-3 py-2.5 text-xs text-muted-foreground dark:border-border dark:bg-secondary/50 dark:text-muted-foreground">
      <p>
        已选择
        <span className="mx-1 font-medium text-secondary-foreground dark:text-foreground">
          当前 AIO 服务 Codex 网关
        </span>
      </p>
      <div className="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-[11px] leading-5">
        <span>
          认证：
          <span className="ml-1 text-secondary-foreground dark:text-foreground">App Token</span>
        </span>
        <span>
          价格倍率：
          <span className="ml-1 font-mono text-secondary-foreground dark:text-foreground">
            免费
          </span>
        </span>
        <span className="min-w-0 max-w-full truncate" title={codexGatewayBaseUrl}>
          Base URL：
          <span className="ml-1 font-mono text-secondary-foreground dark:text-foreground">
            {codexGatewayBaseUrl}
          </span>
        </span>
        <span>
          Token：
          <span className="ml-1 font-mono text-secondary-foreground dark:text-foreground">
            {CX2CC_PROXY_TOKEN}
          </span>
        </span>
      </div>
      <p className="mt-1 text-[11px] leading-5">
        说明：转译后的请求会进入当前 AIO 服务 Codex 网关，再按当前 Codex 分流继续路由。
      </p>
      <Cx2ccFallbackModelsInfo
        cx2ccFallbackModels={cx2ccFallbackModels}
        claudeModels={claudeModels}
      />
    </div>
  );
}

function Cx2ccProviderSourceInfo(props: {
  provider: { name: string; auth_mode: string; cost_multiplier: number; base_urls: string[] };
  cx2ccFallbackModels: Cx2ccFallbackModels;
  claudeModels: UseProviderEditorFormReturn["claudeModels"];
}) {
  const { provider, cx2ccFallbackModels, claudeModels } = props;

  return (
    <div className="rounded-md border border-border bg-secondary px-3 py-2.5 text-xs text-muted-foreground dark:border-border dark:bg-secondary/50 dark:text-muted-foreground">
      <p>
        已选择
        <span className="mx-1 font-medium text-secondary-foreground dark:text-foreground">
          {provider.name}
        </span>
      </p>
      <div className="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-[11px] leading-5">
        <span>
          认证：
          <span className="ml-1 text-secondary-foreground dark:text-foreground">
            {provider.auth_mode === "oauth" ? "OAuth" : "API Key"}
          </span>
        </span>
        <span>
          价格倍率：
          <span className="ml-1 font-mono text-secondary-foreground dark:text-foreground">
            x{provider.cost_multiplier.toFixed(2)}
          </span>
        </span>
        <span
          className="min-w-0 max-w-full truncate"
          title={provider.base_urls[0] ?? "跟随网关默认路由"}
        >
          Base URL：
          <span className="ml-1 font-mono text-secondary-foreground dark:text-foreground">
            {provider.base_urls[0] ?? "跟随网关默认路由"}
          </span>
        </span>
      </div>
      <Cx2ccFallbackModelsInfo
        cx2ccFallbackModels={cx2ccFallbackModels}
        claudeModels={claudeModels}
      />
    </div>
  );
}

function Cx2ccFallbackModelsInfo(props: {
  cx2ccFallbackModels: Cx2ccFallbackModels;
  claudeModels: UseProviderEditorFormReturn["claudeModels"];
}) {
  const { cx2ccFallbackModels, claudeModels } = props;

  return (
    <p className="mt-1 text-[11px] leading-5">
      当前模型映射： 主模型
      <span className="mx-1 font-mono text-secondary-foreground dark:text-foreground">
        {effectiveCx2ccModel(claudeModels.main_model, cx2ccFallbackModels?.main)}
      </span>
      / Haiku
      <span className="mx-1 font-mono text-secondary-foreground dark:text-foreground">
        {effectiveCx2ccModel(claudeModels.haiku_model, cx2ccFallbackModels?.haiku)}
      </span>
      / Sonnet
      <span className="mx-1 font-mono text-secondary-foreground dark:text-foreground">
        {effectiveCx2ccModel(claudeModels.sonnet_model, cx2ccFallbackModels?.sonnet)}
      </span>
      / Opus
      <span className="mx-1 font-mono text-secondary-foreground dark:text-foreground">
        {effectiveCx2ccModel(claudeModels.opus_model, cx2ccFallbackModels?.opus)}
      </span>
    </p>
  );
}

function normalizeModelName(value: string | null | undefined) {
  const trimmed = value?.trim();
  return trimmed || null;
}

function withCx2ccDefaultModel(
  value: UseProviderEditorFormReturn["claudeModels"],
  defaultModel: string
) {
  return {
    ...value,
    main_model: normalizeModelName(value.main_model) ?? defaultModel,
    reasoning_model: normalizeModelName(value.reasoning_model) ?? defaultModel,
    haiku_model: normalizeModelName(value.haiku_model) ?? defaultModel,
    sonnet_model: normalizeModelName(value.sonnet_model) ?? defaultModel,
    opus_model: normalizeModelName(value.opus_model) ?? defaultModel,
  };
}

function resolveCx2ccDefaultModelSelectValue(value: UseProviderEditorFormReturn["claudeModels"]) {
  const normalizedValues = CX2CC_MODEL_MAPPING_KEYS.map((key) => normalizeModelName(value[key]));
  const configuredValues = normalizedValues.filter((model): model is string => model != null);
  if (configuredValues.length === 0) return CX2CC_DEFAULT_MODEL;

  const firstModel = normalizedValues[0];
  if (firstModel && normalizedValues.every((model) => model === firstModel)) {
    return firstModel;
  }

  return CX2CC_MANUAL_MODEL_VALUE;
}

function effectiveCx2ccModel(
  providerModel: string | null | undefined,
  fallbackModel: string | null | undefined
) {
  return normalizeModelName(providerModel) ?? normalizeModelName(fallbackModel) ?? "全局默认值";
}
