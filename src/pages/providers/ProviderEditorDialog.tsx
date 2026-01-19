// Usage: Used by ProvidersView to create/edit a Provider with toast-based validation.

import { useEffect, useRef, useState } from "react";
import { ChevronDown } from "lucide-react";
import { toast } from "sonner";
import { CLIS } from "../../constants/clis";
import { logToConsole } from "../../services/consoleLog";
import { providerUpsert, type CliKey, type ProviderSummary } from "../../services/providers";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { Switch } from "../../ui/Switch";
import { cn } from "../../utils/cn";
import { normalizeBaseUrlRows } from "./baseUrl";
import { BaseUrlEditor } from "./BaseUrlEditor";
import { ModelMappingEditor } from "./ModelMappingEditor";
import { ModelWhitelistEditor } from "./ModelWhitelistEditor";
import type { BaseUrlRow, ProviderBaseUrlMode } from "./types";
import {
  parseAndValidateCostMultiplier,
  validateProviderModelConfig,
  validateProviderApiKeyForCreate,
  validateProviderName,
} from "./validators";

type BaseUrlModeRadioGroupProps = {
  value: ProviderBaseUrlMode;
  onChange: (mode: ProviderBaseUrlMode) => void;
  disabled?: boolean;
};

function BaseUrlModeRadioGroup({ value, onChange, disabled }: BaseUrlModeRadioGroupProps) {
  const items = [
    { value: "order" as const, label: "顺序" },
    { value: "ping" as const, label: "Ping" },
  ];

  return (
    <div
      role="radiogroup"
      aria-label="Base URL 模式"
      className={cn(
        "inline-flex w-full overflow-hidden rounded-lg border border-slate-200 bg-white shadow-sm",
        disabled ? "opacity-60" : null
      )}
    >
      {items.map((item, index) => {
        const active = value === item.value;
        return (
          <button
            key={item.value}
            type="button"
            onClick={() => onChange(item.value)}
            role="radio"
            aria-checked={active}
            disabled={disabled}
            className={cn(
              "flex-1 px-3 py-2 text-sm font-medium transition",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#0052FF]/30 focus-visible:ring-offset-2 focus-visible:ring-offset-[#FAFAFA]",
              index < items.length - 1 ? "border-r border-slate-200" : null,
              active ? "bg-gradient-to-br from-[#0052FF] to-[#4D7CFF] text-white" : null,
              !active ? "bg-white text-slate-700 hover:bg-slate-50" : null,
              disabled ? "cursor-not-allowed" : null
            )}
          >
            {item.label}
          </button>
        );
      })}
    </div>
  );
}

type ProviderEditorDialogBaseProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSaved: (cliKey: CliKey) => void;
};

export type ProviderEditorDialogProps =
  | (ProviderEditorDialogBaseProps & {
      mode: "create";
      cliKey: CliKey;
    })
  | (ProviderEditorDialogBaseProps & {
      mode: "edit";
      provider: ProviderSummary;
    });

function cliNameFromKey(cliKey: CliKey) {
  return CLIS.find((cli) => cli.key === cliKey)?.name ?? cliKey;
}

export function ProviderEditorDialog(props: ProviderEditorDialogProps) {
  const { open, onOpenChange, onSaved } = props;

  const mode = props.mode;
  const cliKey = mode === "create" ? props.cliKey : props.provider.cli_key;
  const editingProviderId = mode === "edit" ? props.provider.id : null;

  const baseUrlRowSeqRef = useRef(1);
  const newBaseUrlRow = (url = ""): BaseUrlRow => {
    const id = String(baseUrlRowSeqRef.current++);
    return { id, url, ping: { status: "idle" } };
  };

  const [name, setName] = useState("");
  const [baseUrlMode, setBaseUrlMode] = useState<ProviderBaseUrlMode>("order");
  const [baseUrlRows, setBaseUrlRows] = useState<BaseUrlRow[]>(() => [newBaseUrlRow()]);
  const [pingingAll, setPingingAll] = useState(false);
  const [apiKey, setApiKey] = useState("");
  const [costMultiplier, setCostMultiplier] = useState("1.0");
  const [supportedModels, setSupportedModels] = useState<Record<string, boolean>>({});
  const [modelMapping, setModelMapping] = useState<Record<string, string>>({});
  const [enabled, setEnabled] = useState(true);
  const [saving, setSaving] = useState(false);

  const title =
    mode === "create"
      ? `${cliNameFromKey(cliKey)} · 添加供应商`
      : `${cliNameFromKey(props.provider.cli_key)} · 编辑供应商`;
  const description = mode === "create" ? "已锁定创建 CLI；如需切换请先关闭弹窗。" : undefined;

  useEffect(() => {
    if (!open) return;

    baseUrlRowSeqRef.current = 1;

    if (mode === "create") {
      setName("");
      setBaseUrlMode("order");
      setBaseUrlRows([newBaseUrlRow()]);
      setPingingAll(false);
      setApiKey("");
      setCostMultiplier("1.0");
      setSupportedModels({});
      setModelMapping({});
      setEnabled(true);
      return;
    }

    setName(props.provider.name);
    setBaseUrlMode(props.provider.base_url_mode);
    setBaseUrlRows(props.provider.base_urls.map((url) => newBaseUrlRow(url)));
    setPingingAll(false);
    setApiKey("");
    setEnabled(props.provider.enabled);
    setCostMultiplier(String(props.provider.cost_multiplier ?? 1.0));
    setSupportedModels(props.provider.supported_models ?? {});
    setModelMapping(props.provider.model_mapping ?? {});
  }, [cliKey, editingProviderId, mode, open]);

  async function save() {
    if (saving) return;

    const nameError = validateProviderName(name);
    if (nameError) {
      toast(nameError);
      return;
    }

    if (mode === "create") {
      const apiKeyError = validateProviderApiKeyForCreate(apiKey);
      if (apiKeyError) {
        toast(apiKeyError);
        return;
      }
    }

    const parsedCost = parseAndValidateCostMultiplier(costMultiplier);
    if (!parsedCost.ok) {
      toast(parsedCost.message);
      return;
    }

    const normalized = normalizeBaseUrlRows(baseUrlRows);
    if (!normalized.ok) {
      toast(normalized.message);
      return;
    }

    if (cliKey === "claude") {
      const modelError = validateProviderModelConfig({ supportedModels, modelMapping });
      if (modelError) {
        toast(modelError);
        return;
      }
    }

    setSaving(true);
    try {
      const saved = await providerUpsert({
        ...(mode === "edit" ? { provider_id: props.provider.id } : {}),
        cli_key: cliKey,
        name,
        base_urls: normalized.baseUrls,
        base_url_mode: baseUrlMode,
        api_key: apiKey,
        enabled,
        cost_multiplier: parsedCost.value,
        ...(cliKey === "claude"
          ? { supported_models: supportedModels, model_mapping: modelMapping }
          : {}),
      });

      if (!saved) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      setApiKey("");
      logToConsole("info", mode === "create" ? "保存 Provider" : "更新 Provider", {
        cli: saved.cli_key,
        provider_id: saved.id,
        name: saved.name,
        base_urls: saved.base_urls,
        base_url_mode: saved.base_url_mode,
        enabled: saved.enabled,
        cost_multiplier: saved.cost_multiplier,
        supported_models: saved.supported_models,
        model_mapping: saved.model_mapping,
      });
      toast(mode === "create" ? "Provider 已保存" : "Provider 已更新");

      onSaved(saved.cli_key);
      onOpenChange(false);
    } catch (err) {
      logToConsole("error", mode === "create" ? "保存 Provider 失败" : "更新 Provider 失败", {
        error: String(err),
        cli: cliKey,
        provider_id: mode === "edit" ? props.provider.id : undefined,
      });
      toast(`${mode === "create" ? "保存" : "更新"}失败：${String(err)}`);
    } finally {
      setSaving(false);
    }
  }

  const supportedModelCount =
    cliKey === "claude"
      ? Object.keys(supportedModels).filter((key) => supportedModels[key]).length
      : 0;
  const modelMappingCount =
    cliKey === "claude"
      ? Object.entries(modelMapping).filter(([k, v]) => k.trim().length > 0 && v.trim().length > 0)
          .length
      : 0;

  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen) => {
        if (!nextOpen && saving) return;
        onOpenChange(nextOpen);
      }}
      title={title}
      description={description}
      className="max-w-xl"
    >
      <div className="space-y-4">
        <div className="grid gap-3 sm:grid-cols-2">
          <FormField label="名称">
            <Input
              value={name}
              onChange={(e) => setName(e.currentTarget.value)}
              placeholder="default"
            />
          </FormField>

          <FormField label="Base URL 模式">
            <BaseUrlModeRadioGroup
              value={baseUrlMode}
              onChange={setBaseUrlMode}
              disabled={saving}
            />
          </FormField>
        </div>

        <FormField label="Base URLs">
          <BaseUrlEditor
            rows={baseUrlRows}
            setRows={setBaseUrlRows}
            pingingAll={pingingAll}
            setPingingAll={setPingingAll}
            newRow={newBaseUrlRow}
            disabled={saving}
          />
        </FormField>

        <div className="grid gap-3 sm:grid-cols-2">
          <FormField label="API Key" hint={mode === "edit" ? "留空保持不变" : "保存后不回显"}>
            <Input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.currentTarget.value)}
              placeholder="sk-…"
              autoComplete="off"
            />
          </FormField>

          <FormField label="价格倍率">
            <Input
              type="number"
              min="0.0001"
              step="0.01"
              value={costMultiplier}
              onChange={(e) => setCostMultiplier(e.currentTarget.value)}
              placeholder="1.0"
            />
          </FormField>
        </div>

        {cliKey === "claude" ? (
          <details className="group rounded-xl border border-slate-200 bg-white shadow-sm open:ring-2 open:ring-[#0052FF]/10 transition-all">
            <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
              <div className="flex items-center gap-3">
                <span className="text-sm font-medium text-slate-700 group-open:text-[#0052FF]">
                  模型映射
                </span>
                <span className="text-xs font-mono text-slate-500">
                  白名单 {supportedModelCount} · 映射 {modelMappingCount}
                </span>
              </div>
              <ChevronDown className="h-4 w-4 text-slate-400 transition-transform group-open:rotate-180" />
            </summary>

            <div className="space-y-4 border-t border-slate-100 px-4 py-3">
              <FormField label="模型白名单 (supportedModels)" hint="支持 *；为空表示支持所有模型">
                <ModelWhitelistEditor
                  value={supportedModels}
                  onChange={setSupportedModels}
                  disabled={saving}
                />
              </FormField>

              <FormField
                label="模型映射 (modelMapping)"
                hint="支持 *；当同时配置白名单时会校验目标模型"
              >
                <ModelMappingEditor
                  value={modelMapping}
                  onChange={setModelMapping}
                  disabled={saving}
                />
              </FormField>
            </div>
          </details>
        ) : null}

        <div className="flex items-center justify-between border-t border-slate-100 pt-3">
          <div className="flex items-center gap-2">
            <span className="text-sm text-slate-700">启用</span>
            <Switch checked={enabled} onCheckedChange={setEnabled} disabled={saving} />
          </div>
          <div className="flex items-center gap-2">
            <Button onClick={() => onOpenChange(false)} variant="secondary" disabled={saving}>
              取消
            </Button>
            <Button onClick={save} variant="primary" disabled={saving}>
              {saving ? "保存中…" : "保存"}
            </Button>
          </div>
        </div>
      </div>
    </Dialog>
  );
}
