import { useReducer, type ReactNode } from "react";
import { toast } from "sonner";
import { Settings } from "lucide-react";
import type { AppSettings } from "../../../services/settings/settings";
import {
  validateCx2ccFallbackModel,
  validateCx2ccOptionalField,
} from "../../../services/settings/settingsValidation";
import { cn } from "../../../utils/cn";
import { Card } from "../../../ui/Card";
import { Input } from "../../../ui/Input";
import { RadioGroup } from "../../../ui/RadioGroup";
import { Switch } from "../../../ui/Switch";

export type CliManagerCx2ccTabProps = {
  appSettings: AppSettings | null;
  commonSettingsSaving: boolean;
  onPersistCommonSettings: (patch: Partial<AppSettings>) => Promise<AppSettings | null>;
};

type Cx2ccTextSettingKey =
  | "cx2cc_fallback_model_opus"
  | "cx2cc_fallback_model_sonnet"
  | "cx2cc_fallback_model_haiku"
  | "cx2cc_fallback_model_main"
  | "cx2cc_service_tier";

type Cx2ccDraftKey = Cx2ccTextSettingKey | "cx2cc_model_reasoning_effort";

type Cx2ccDraftState = {
  sourceKey: string;
  values: Record<Cx2ccDraftKey, string>;
};

type Cx2ccDraftAction =
  | { type: "resetFromSettings"; state: Cx2ccDraftState }
  | { type: "setValue"; key: Cx2ccDraftKey; value: string };

const EMPTY_CX2CC_DRAFT_VALUES: Record<Cx2ccDraftKey, string> = {
  cx2cc_fallback_model_opus: "",
  cx2cc_fallback_model_sonnet: "",
  cx2cc_fallback_model_haiku: "",
  cx2cc_fallback_model_main: "",
  cx2cc_model_reasoning_effort: "",
  cx2cc_service_tier: "",
};

const CX2CC_REASONING_EFFORT_LABEL = "推理强度";

function createCx2ccDraftState(appSettings: AppSettings | null): Cx2ccDraftState {
  if (!appSettings) {
    return { sourceKey: "empty", values: EMPTY_CX2CC_DRAFT_VALUES };
  }

  const values: Record<Cx2ccDraftKey, string> = {
    cx2cc_fallback_model_opus: appSettings.cx2cc_fallback_model_opus,
    cx2cc_fallback_model_sonnet: appSettings.cx2cc_fallback_model_sonnet,
    cx2cc_fallback_model_haiku: appSettings.cx2cc_fallback_model_haiku,
    cx2cc_fallback_model_main: appSettings.cx2cc_fallback_model_main,
    cx2cc_model_reasoning_effort: appSettings.cx2cc_model_reasoning_effort,
    cx2cc_service_tier: appSettings.cx2cc_service_tier,
  };

  return {
    sourceKey: Object.values(values).join("\u0000"),
    values,
  };
}

function cx2ccDraftReducer(state: Cx2ccDraftState, action: Cx2ccDraftAction): Cx2ccDraftState {
  if (action.type === "resetFromSettings") {
    return action.state;
  }
  return {
    ...state,
    values: {
      ...state.values,
      [action.key]: action.value,
    },
  };
}

function SettingItem({
  label,
  subtitle,
  children,
  className,
}: {
  label: string;
  subtitle: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex flex-col gap-2 py-3 sm:flex-row sm:items-start sm:justify-between",
        className
      )}
    >
      <div className="min-w-0">
        <div className="text-sm text-secondary-foreground">{label}</div>
        <div className="mt-1 text-xs text-muted-foreground leading-relaxed">{subtitle}</div>
      </div>
      <div className="flex flex-wrap items-center justify-end gap-2">{children}</div>
    </div>
  );
}

export function CliManagerCx2ccTab({
  appSettings,
  commonSettingsSaving,
  onPersistCommonSettings,
}: CliManagerCx2ccTabProps) {
  const nextDraftState = createCx2ccDraftState(appSettings);
  const [draftState, dispatchDraft] = useReducer(cx2ccDraftReducer, nextDraftState);
  const effectiveDraftState =
    draftState.sourceKey === nextDraftState.sourceKey ? draftState : nextDraftState;
  if (draftState.sourceKey !== nextDraftState.sourceKey) {
    dispatchDraft({ type: "resetFromSettings", state: nextDraftState });
  }

  const fallbackModelOpusText = effectiveDraftState.values.cx2cc_fallback_model_opus;
  const fallbackModelSonnetText = effectiveDraftState.values.cx2cc_fallback_model_sonnet;
  const fallbackModelHaikuText = effectiveDraftState.values.cx2cc_fallback_model_haiku;
  const fallbackModelMainText = effectiveDraftState.values.cx2cc_fallback_model_main;
  const reasoningEffortText = effectiveDraftState.values.cx2cc_model_reasoning_effort;
  const serviceTierText = effectiveDraftState.values.cx2cc_service_tier;

  const controlsDisabled = commonSettingsSaving || !appSettings;

  function setDraftValue(key: Cx2ccDraftKey, value: string) {
    dispatchDraft({ type: "setValue", key, value });
  }

  async function persistReasoningEffort(value: string) {
    if (!appSettings) return;

    const previous = appSettings.cx2cc_model_reasoning_effort;
    setDraftValue("cx2cc_model_reasoning_effort", value);

    const updated = await onPersistCommonSettings({ cx2cc_model_reasoning_effort: value });
    if (!updated) {
      setDraftValue("cx2cc_model_reasoning_effort", previous);
      return;
    }

    setDraftValue("cx2cc_model_reasoning_effort", updated.cx2cc_model_reasoning_effort);
  }

  async function persistFallbackModel(
    key: Exclude<Cx2ccTextSettingKey, "cx2cc_service_tier">,
    label: string,
    value: string
  ) {
    if (!appSettings) return;

    const previous = appSettings[key];
    const trimmed = value.trim();
    setDraftValue(key, trimmed);

    const validationMessage = validateCx2ccFallbackModel(label, trimmed);
    if (validationMessage) {
      toast(validationMessage);
      setDraftValue(key, previous);
      return;
    }

    const updated = await onPersistCommonSettings({ [key]: trimmed } as Partial<AppSettings>);
    setDraftValue(key, updated ? updated[key] : previous);
  }

  async function persistOptionalTextSetting(
    key: Extract<Cx2ccTextSettingKey, "cx2cc_service_tier">,
    label: string,
    value: string
  ) {
    if (!appSettings) return;

    const previous = appSettings[key];
    const trimmed = value.trim();
    setDraftValue(key, trimmed);

    const validationMessage = validateCx2ccOptionalField(label, trimmed);
    if (validationMessage) {
      toast(validationMessage);
      setDraftValue(key, previous);
      return;
    }

    const updated = await onPersistCommonSettings({ [key]: trimmed } as Partial<AppSettings>);
    setDraftValue(key, updated ? updated[key] : previous);
  }

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden p-5">
        <h3 className="mb-3 flex items-center gap-2 text-sm font-semibold text-foreground">
          <Settings className="h-4 w-4 text-muted-foreground" />
          模型 Fallback 映射
        </h3>
        <div className="divide-y divide-border">
          <SettingItem label="Opus 默认模型" subtitle="当 Provider 未设置 Opus 覆盖时使用此模型">
            <Input
              value={fallbackModelOpusText}
              onChange={(e) => setDraftValue("cx2cc_fallback_model_opus", e.currentTarget.value)}
              onBlur={(e) => {
                void persistFallbackModel(
                  "cx2cc_fallback_model_opus",
                  "Opus 默认模型",
                  e.currentTarget.value
                );
              }}
              placeholder="gpt-5.4"
              className="font-mono w-[240px] max-w-full"
              disabled={controlsDisabled}
            />
          </SettingItem>

          <SettingItem
            label="Sonnet 默认模型"
            subtitle="当 Provider 未设置 Sonnet 覆盖时使用此模型"
          >
            <Input
              value={fallbackModelSonnetText}
              onChange={(e) => setDraftValue("cx2cc_fallback_model_sonnet", e.currentTarget.value)}
              onBlur={(e) => {
                void persistFallbackModel(
                  "cx2cc_fallback_model_sonnet",
                  "Sonnet 默认模型",
                  e.currentTarget.value
                );
              }}
              placeholder="gpt-5.4"
              className="font-mono w-[240px] max-w-full"
              disabled={controlsDisabled}
            />
          </SettingItem>

          <SettingItem label="Haiku 默认模型" subtitle="当 Provider 未设置 Haiku 覆盖时使用此模型">
            <Input
              value={fallbackModelHaikuText}
              onChange={(e) => setDraftValue("cx2cc_fallback_model_haiku", e.currentTarget.value)}
              onBlur={(e) => {
                void persistFallbackModel(
                  "cx2cc_fallback_model_haiku",
                  "Haiku 默认模型",
                  e.currentTarget.value
                );
              }}
              placeholder="gpt-5.4"
              className="font-mono w-[240px] max-w-full"
              disabled={controlsDisabled}
            />
          </SettingItem>

          <SettingItem label="主模型默认" subtitle="当 Provider 未设置 Main 覆盖时使用此模型">
            <Input
              value={fallbackModelMainText}
              onChange={(e) => setDraftValue("cx2cc_fallback_model_main", e.currentTarget.value)}
              onBlur={(e) => {
                void persistFallbackModel(
                  "cx2cc_fallback_model_main",
                  "主模型默认",
                  e.currentTarget.value
                );
              }}
              placeholder="gpt-5.4"
              className="font-mono w-[240px] max-w-full"
              disabled={controlsDisabled}
            />
          </SettingItem>
        </div>
      </Card>

      <Card className="overflow-hidden p-5">
        <h3 className="mb-3 flex items-center gap-2 text-sm font-semibold text-foreground">
          <Settings className="h-4 w-4 text-muted-foreground" />
          上游请求注入
        </h3>
        <div className="divide-y divide-border">
          <SettingItem
            label={CX2CC_REASONING_EFFORT_LABEL}
            subtitle="注入 reasoning.effort 到上游请求；默认表示不注入。"
          >
            <RadioGroup
              name="cx2cc_model_reasoning_effort"
              ariaLabel={CX2CC_REASONING_EFFORT_LABEL}
              value={reasoningEffortText}
              onChange={(value) => {
                void persistReasoningEffort(value);
              }}
              options={[
                { value: "", label: "默认 / 不注入" },
                { value: "low", label: "low" },
                { value: "medium", label: "medium" },
                { value: "high", label: "high" },
                { value: "xhigh", label: "xhigh" },
              ]}
              disabled={controlsDisabled}
            />
          </SettingItem>

          <SettingItem label="服务层级" subtitle="注入 service_tier 到上游请求；留空表示不注入。">
            <Input
              value={serviceTierText}
              onChange={(e) => setDraftValue("cx2cc_service_tier", e.currentTarget.value)}
              onBlur={(e) => {
                void persistOptionalTextSetting(
                  "cx2cc_service_tier",
                  "服务层级",
                  e.currentTarget.value
                );
              }}
              placeholder="例如: fast"
              className="font-mono w-[240px] max-w-full"
              disabled={controlsDisabled}
            />
          </SettingItem>

          <SettingItem label="禁用响应存储" subtitle="注入 store: false 到上游请求">
            <Switch
              checked={appSettings?.cx2cc_disable_response_storage ?? true}
              onCheckedChange={(checked) => {
                void onPersistCommonSettings({ cx2cc_disable_response_storage: checked });
              }}
              disabled={controlsDisabled}
            />
          </SettingItem>
        </div>
      </Card>

      <Card className="overflow-hidden p-5">
        <h3 className="mb-3 flex items-center gap-2 text-sm font-semibold text-foreground">
          <Settings className="h-4 w-4 text-muted-foreground" />
          转换行为开关
        </h3>
        <div className="divide-y divide-border">
          <SettingItem
            label="启用推理转思考"
            subtitle="将上游 reasoning 输出转换为 Claude thinking 格式"
          >
            <Switch
              checked={appSettings?.cx2cc_enable_reasoning_to_thinking ?? true}
              onCheckedChange={(checked) => {
                void onPersistCommonSettings({
                  cx2cc_enable_reasoning_to_thinking: checked,
                });
              }}
              disabled={controlsDisabled}
            />
          </SettingItem>

          <SettingItem label="丢弃停止序列" subtitle="丢弃 stop_sequences（Responses API 不支持）">
            <Switch
              checked={appSettings?.cx2cc_drop_stop_sequences ?? true}
              onCheckedChange={(checked) => {
                void onPersistCommonSettings({ cx2cc_drop_stop_sequences: checked });
              }}
              disabled={controlsDisabled}
            />
          </SettingItem>

          <SettingItem
            label="清理 Schema"
            subtitle='移除工具 schema 中的 format: "uri"（Responses API 不支持）'
          >
            <Switch
              checked={appSettings?.cx2cc_clean_schema ?? true}
              onCheckedChange={(checked) => {
                void onPersistCommonSettings({ cx2cc_clean_schema: checked });
              }}
              disabled={controlsDisabled}
            />
          </SettingItem>

          <SettingItem label="过滤 BatchTool" subtitle="过滤掉 BatchTool 类型的工具">
            <Switch
              checked={appSettings?.cx2cc_filter_batch_tool ?? true}
              onCheckedChange={(checked) => {
                void onPersistCommonSettings({ cx2cc_filter_batch_tool: checked });
              }}
              disabled={controlsDisabled}
            />
          </SettingItem>
        </div>
      </Card>
    </div>
  );
}
