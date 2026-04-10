import { useEffect, useState, type ReactNode } from "react";
import { Settings } from "lucide-react";
import type { AppSettings } from "../../../services/settings/settings";
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
        <div className="text-sm text-slate-700 dark:text-slate-300">{label}</div>
        <div className="mt-1 text-xs text-slate-500 dark:text-slate-400 leading-relaxed">
          {subtitle}
        </div>
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
  const [fallbackModelOpusText, setFallbackModelOpusText] = useState("");
  const [fallbackModelSonnetText, setFallbackModelSonnetText] = useState("");
  const [fallbackModelHaikuText, setFallbackModelHaikuText] = useState("");
  const [fallbackModelMainText, setFallbackModelMainText] = useState("");
  const [serviceTierText, setServiceTierText] = useState("");

  useEffect(() => {
    if (!appSettings) return;
    setFallbackModelOpusText(appSettings.cx2cc_fallback_model_opus);
    setFallbackModelSonnetText(appSettings.cx2cc_fallback_model_sonnet);
    setFallbackModelHaikuText(appSettings.cx2cc_fallback_model_haiku);
    setFallbackModelMainText(appSettings.cx2cc_fallback_model_main);
    setServiceTierText(appSettings.cx2cc_service_tier);
  }, [
    appSettings?.cx2cc_fallback_model_opus,
    appSettings?.cx2cc_fallback_model_sonnet,
    appSettings?.cx2cc_fallback_model_haiku,
    appSettings?.cx2cc_fallback_model_main,
    appSettings?.cx2cc_service_tier,
  ]);

  const controlsDisabled = commonSettingsSaving || !appSettings;

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden p-5">
        <h3 className="mb-3 flex items-center gap-2 text-sm font-semibold text-slate-900 dark:text-slate-100">
          <Settings className="h-4 w-4 text-slate-400 dark:text-slate-500" />
          模型 Fallback 映射
        </h3>
        <div className="divide-y divide-slate-100 dark:divide-slate-700">
          <SettingItem label="Opus 默认模型" subtitle="当 Provider 未设置 Opus 覆盖时使用此模型">
            <Input
              value={fallbackModelOpusText}
              onChange={(e) => setFallbackModelOpusText(e.currentTarget.value)}
              onBlur={(e) => {
                const value = e.currentTarget.value.trim();
                setFallbackModelOpusText(value);
                if (value) {
                  void onPersistCommonSettings({ cx2cc_fallback_model_opus: value });
                }
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
              onChange={(e) => setFallbackModelSonnetText(e.currentTarget.value)}
              onBlur={(e) => {
                const value = e.currentTarget.value.trim();
                setFallbackModelSonnetText(value);
                if (value) {
                  void onPersistCommonSettings({ cx2cc_fallback_model_sonnet: value });
                }
              }}
              placeholder="gpt-5.4"
              className="font-mono w-[240px] max-w-full"
              disabled={controlsDisabled}
            />
          </SettingItem>

          <SettingItem label="Haiku 默认模型" subtitle="当 Provider 未设置 Haiku 覆盖时使用此模型">
            <Input
              value={fallbackModelHaikuText}
              onChange={(e) => setFallbackModelHaikuText(e.currentTarget.value)}
              onBlur={(e) => {
                const value = e.currentTarget.value.trim();
                setFallbackModelHaikuText(value);
                if (value) {
                  void onPersistCommonSettings({ cx2cc_fallback_model_haiku: value });
                }
              }}
              placeholder="gpt-5.4"
              className="font-mono w-[240px] max-w-full"
              disabled={controlsDisabled}
            />
          </SettingItem>

          <SettingItem label="主模型默认" subtitle="当 Provider 未设置 Main 覆盖时使用此模型">
            <Input
              value={fallbackModelMainText}
              onChange={(e) => setFallbackModelMainText(e.currentTarget.value)}
              onBlur={(e) => {
                const value = e.currentTarget.value.trim();
                setFallbackModelMainText(value);
                if (value) {
                  void onPersistCommonSettings({ cx2cc_fallback_model_main: value });
                }
              }}
              placeholder="gpt-5.4"
              className="font-mono w-[240px] max-w-full"
              disabled={controlsDisabled}
            />
          </SettingItem>
        </div>
      </Card>

      <Card className="overflow-hidden p-5">
        <h3 className="mb-3 flex items-center gap-2 text-sm font-semibold text-slate-900 dark:text-slate-100">
          <Settings className="h-4 w-4 text-slate-400 dark:text-slate-500" />
          上游请求注入
        </h3>
        <div className="divide-y divide-slate-100 dark:divide-slate-700">
          <SettingItem
            label="推理强度"
            subtitle="注入 reasoning.effort 到上游请求；默认表示不注入。"
          >
            <RadioGroup
              name="cx2cc_model_reasoning_effort"
              value={appSettings?.cx2cc_model_reasoning_effort ?? ""}
              onChange={(value) => {
                void onPersistCommonSettings({ cx2cc_model_reasoning_effort: value });
              }}
              options={[
                { value: "", label: "默认 / 不注入" },
                { value: "low", label: "low" },
                { value: "medium", label: "medium" },
                { value: "high", label: "high" },
              ]}
              disabled={controlsDisabled}
            />
          </SettingItem>

          <SettingItem label="服务层级" subtitle="注入 service_tier 到上游请求；留空表示不注入。">
            <Input
              value={serviceTierText}
              onChange={(e) => setServiceTierText(e.currentTarget.value)}
              onBlur={(e) => {
                const value = e.currentTarget.value.trim();
                setServiceTierText(value);
                void onPersistCommonSettings({ cx2cc_service_tier: value });
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
        <h3 className="mb-3 flex items-center gap-2 text-sm font-semibold text-slate-900 dark:text-slate-100">
          <Settings className="h-4 w-4 text-slate-400 dark:text-slate-500" />
          转换行为开关
        </h3>
        <div className="divide-y divide-slate-100 dark:divide-slate-700">
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
