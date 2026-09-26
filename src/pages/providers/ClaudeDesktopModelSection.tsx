// Usage: Quick inputs for Claude Desktop's model-menu roles inside the model mapping section.

import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import type { ProviderModelMapping } from "../../services/providers/providers";

const ROUTES = [
  { id: "claude-sonnet-5", label: "Sonnet", hint: "常用模型" },
  { id: "claude-opus-5", label: "Opus", hint: "高能力模型" },
  { id: "claude-fable-5", label: "Fable", hint: "新增角色" },
  { id: "claude-haiku-4-5", label: "Haiku", hint: "快速模型" },
] as const;

export function ClaudeDesktopModelSection({
  mappings,
  onChange,
  supports1m,
  onSupports1mChange,
  disabled,
}: {
  mappings: ProviderModelMapping[];
  onChange: (mappings: ProviderModelMapping[]) => void;
  supports1m: boolean;
  onSupports1mChange: (checked: boolean) => void;
  disabled: boolean;
}) {
  function setMapping(source: string, target: string) {
    if (!target.trim()) {
      onChange(mappings.filter((mapping) => mapping.source !== source));
    } else if (mappings.some((mapping) => mapping.source === source)) {
      onChange(
        mappings.map((mapping) => (mapping.source === source ? { source, target } : mapping))
      );
    } else {
      onChange([...mappings, { source, target }]);
    }
  }

  return (
    <div className="space-y-3">
      <p className="text-xs text-muted-foreground">
        Desktop 菜单只显示下列 Claude 角色名。上游不接受这些名称时，填写真实模型 ID；请求仍由 AIO
        网关按当前供应商转发和统计。Code 页签、历史会话等还可能请求其他模型（例如
        claude-opus-5-5），请在下方添加映射。供应商接口须兼容 Anthropic
        Messages；只填写模型名不会转换 API 协议。
      </p>

      <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
        {ROUTES.map((route) => (
          <FormField key={route.id} label={`${route.label} · ${route.hint}`} hint={route.id}>
            <Input
              aria-label={`${route.label} 上游模型`}
              value={mappings.find((mapping) => mapping.source === route.id)?.target ?? ""}
              onChange={(event) => setMapping(route.id, event.currentTarget.value)}
              placeholder={route.id}
              disabled={disabled}
              mono
            />
          </FormField>
        ))}
      </div>

      <label className="flex items-start gap-2 rounded-lg border border-border bg-muted p-3">
        <input
          type="checkbox"
          aria-label="支持 1M 上下文"
          checked={supports1m}
          onChange={(event) => onSupports1mChange(event.currentTarget.checked)}
          disabled={disabled}
          className="mt-0.5 h-4 w-4 shrink-0 rounded border-border bg-background text-primary accent-primary focus:ring-ring"
        />
        <span className="min-w-0">
          <span className="block text-sm font-medium text-foreground">支持 1M 上下文</span>
          <span className="mt-1 block text-xs text-muted-foreground">
            勾选后 Desktop 模型菜单会出现 1M 版本，1M 请求只转发给勾选了此项的供应商；
            没有供应商勾选时菜单不显示 1M。修改后需完全退出并重新打开 Desktop 才会生效。
          </span>
        </span>
      </label>
    </div>
  );
}
