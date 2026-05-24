import { ChevronDown } from "lucide-react";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import type { UseProviderEditorFormReturn } from "./useProviderEditorForm";

export function ClaudeModelSection(props: { form: UseProviderEditorFormReturn }) {
  const { claudeModels, setClaudeModels, claudeModelCount, saving, cliKey, authMode } = props.form;

  // Show for all claude providers (api_key, oauth, cx2cc) — each provider can
  // independently override which upstream model to use per model tier.
  if (cliKey !== "claude") return null;

  const isCx2cc = authMode === "cx2cc";
  const sectionTitle = isCx2cc ? "CX2CC 模型映射" : "Claude 模型映射";
  const mainPlaceholder = isCx2cc
    ? "例如: gpt-5.4 / o3"
    : "例如: glm-4-plus / minimax-text-01 / kimi-k2";

  return (
    <details className="group rounded-xl border border-border bg-white shadow-sm open:ring-2 open:ring-accent/10 transition-all dark:border-border dark:bg-secondary">
      <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-secondary-foreground group-open:text-accent dark:text-secondary-foreground">
            {sectionTitle}
          </span>
          <span className="text-xs font-mono text-muted-foreground">
            已配置 {claudeModelCount}/5
          </span>
        </div>
        <ChevronDown className="h-4 w-4 text-muted-foreground transition-transform group-open:rotate-180" />
      </summary>

      <div className="space-y-4 border-t border-border px-4 py-3 dark:border-border">
        <FormField
          label="主模型"
          hint="默认兜底模型；未命中 haiku/sonnet/opus 且未启用 Thinking 时使用"
        >
          <Input
            value={claudeModels.main_model ?? ""}
            onChange={(e) => {
              const value = e.currentTarget.value;
              setClaudeModels((prev) => {
                const oldMain = (prev.main_model ?? "").trim();
                const syncIfMatch = (field: string | null | undefined) => {
                  const trimmed = (field ?? "").trim();
                  return !trimmed || trimmed === oldMain ? value : field;
                };
                return {
                  ...prev,
                  main_model: value,
                  haiku_model: syncIfMatch(prev.haiku_model),
                  sonnet_model: syncIfMatch(prev.sonnet_model),
                  opus_model: syncIfMatch(prev.opus_model),
                };
              });
            }}
            placeholder={mainPlaceholder}
            disabled={saving}
          />
        </FormField>

        <FormField label="推理模型 (Thinking)" hint="当请求中 thinking.type=enabled 时优先使用">
          <Input
            value={claudeModels.reasoning_model ?? ""}
            onChange={(e) => {
              const value = e.currentTarget.value;
              setClaudeModels((prev) => ({
                ...prev,
                reasoning_model: value,
              }));
            }}
            placeholder="例如: kimi-k2-thinking / glm-4-plus-thinking"
            disabled={saving}
          />
        </FormField>

        <FormField label="Haiku 默认模型" hint="当请求模型名包含 haiku 时使用（子串匹配）">
          <Input
            value={claudeModels.haiku_model ?? ""}
            onChange={(e) => {
              const value = e.currentTarget.value;
              setClaudeModels((prev) => ({ ...prev, haiku_model: value }));
            }}
            placeholder="例如: glm-4-plus-haiku"
            disabled={saving}
          />
        </FormField>

        <FormField label="Sonnet 默认模型" hint="当请求模型名包含 sonnet 时使用（子串匹配）">
          <Input
            value={claudeModels.sonnet_model ?? ""}
            onChange={(e) => {
              const value = e.currentTarget.value;
              setClaudeModels((prev) => ({ ...prev, sonnet_model: value }));
            }}
            placeholder="例如: glm-4-plus-sonnet"
            disabled={saving}
          />
        </FormField>

        <FormField label="Opus 默认模型" hint="当请求模型名包含 opus 时使用（子串匹配）">
          <Input
            value={claudeModels.opus_model ?? ""}
            onChange={(e) => {
              const value = e.currentTarget.value;
              setClaudeModels((prev) => ({ ...prev, opus_model: value }));
            }}
            placeholder="例如: glm-4-plus-opus"
            disabled={saving}
          />
        </FormField>
      </div>
    </details>
  );
}
