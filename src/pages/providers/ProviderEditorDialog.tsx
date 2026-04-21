import type { CliKey, ProviderSummary } from "../../services/providers/providers";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { Switch } from "../../ui/Switch";
import { TabList } from "../../ui/TabList";
import type { ProviderEditorInitialValues } from "./providerDuplicate";
import { useProviderEditorForm } from "./useProviderEditorForm";
import { OAuthSection } from "./OAuthSection";
import { Cx2ccSection } from "./Cx2ccSection";
import { ApiKeySection } from "./ApiKeySection";
import { LimitsSection } from "./LimitsSection";
import { ClaudeModelSection } from "./ClaudeModelSection";

type ProviderEditorDialogBaseProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSaved: (cliKey: CliKey) => void;
  codexProviders?: ProviderSummary[];
};

export type ProviderEditorDialogProps =
  | (ProviderEditorDialogBaseProps & {
      mode: "create";
      cliKey: CliKey;
      initialValues?: ProviderEditorInitialValues | null;
    })
  | (ProviderEditorDialogBaseProps & {
      mode: "edit";
      provider: ProviderSummary;
    });

export function ProviderEditorDialog(props: ProviderEditorDialogProps) {
  const f = useProviderEditorForm(props);

  return (
    <Dialog
      open={f.open}
      onOpenChange={(nextOpen) => {
        if (!nextOpen && f.saving) return;
        f.onOpenChange(nextOpen);
      }}
      title={f.title}
      description={f.description}
      className="max-w-4xl"
    >
      <div className="space-y-4">
        {/* ── Auth mode selector ── */}
        {f.supportsOAuth ? (
          <FormField label="认证方式" hint="选择后下方表单会相应变化">
            <TabList<"api_key" | "oauth">
              ariaLabel="认证方式"
              items={[
                { key: "api_key", label: "API 密钥" },
                { key: "oauth", label: "OAuth 登录" },
              ]}
              value={f.authMode as "api_key" | "oauth"}
              onChange={(next) => {
                f.setAuthMode(next);
                f.setValue("auth_mode", next, { shouldDirty: true });
              }}
            />
          </FormField>
        ) : f.supportsCx2cc ? (
          <FormField label="认证方式" hint="选择后下方表单会相应变化">
            <TabList<"api_key" | "oauth" | "cx2cc">
              ariaLabel="认证方式"
              items={[
                { key: "api_key", label: "API 密钥" },
                { key: "oauth", label: "OAuth" },
                { key: "cx2cc", label: "CX2CC 转译" },
              ]}
              value={f.authMode as "api_key" | "oauth" | "cx2cc"}
              onChange={(next) => {
                f.setAuthMode(next);
                f.setValue("auth_mode", next === "cx2cc" ? "api_key" : next, { shouldDirty: true });
              }}
            />
          </FormField>
        ) : null}

        {f.authMode === "oauth" ? (
          <OAuthSection form={f} />
        ) : f.authMode === "cx2cc" ? (
          <Cx2ccSection form={f} />
        ) : (
          <ApiKeySection form={f} />
        )}

        <FormField
          label="流式空闲超时覆盖（秒）"
          hint="留空或 0 表示沿用全局设置；仅对当前 Provider 的流式请求生效。"
        >
          <Input
            type="number"
            min="0"
            max="3600"
            step="1"
            placeholder="0"
            value={f.streamIdleTimeoutSeconds}
            onChange={(e) => f.setStreamIdleTimeoutSeconds(e.currentTarget.value)}
            disabled={f.saving}
          />
        </FormField>

        <LimitsSection form={f} />
        <ClaudeModelSection form={f} />

        <div className="flex items-center justify-between border-t border-slate-100 pt-3 dark:border-slate-700">
          <div className="flex items-center gap-2">
            <span className="text-sm text-slate-700 dark:text-slate-300">启用</span>
            <Switch
              checked={f.enabled}
              onCheckedChange={(checked) => f.setValue("enabled", checked, { shouldDirty: true })}
              disabled={f.saving}
            />
          </div>
          <div className="flex items-center gap-2">
            <Button onClick={() => f.onOpenChange(false)} variant="secondary" disabled={f.saving}>
              取消
            </Button>
            <Button onClick={f.save} variant="primary" disabled={f.saving}>
              {f.saving ? "保存中…" : "保存"}
            </Button>
          </div>
        </div>
      </div>
    </Dialog>
  );
}
