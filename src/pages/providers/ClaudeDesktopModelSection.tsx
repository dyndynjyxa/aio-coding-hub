// Usage: Quick inputs for Claude Desktop's model families inside the model mapping section.

import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import type { ProviderModelMapping } from "../../services/providers/providers";

const FAMILIES = [
  { source: "claude-sonnet-*", label: "Sonnet" },
  { source: "claude-opus-*", label: "Opus" },
  { source: "claude-fable-*", label: "Fable" },
  { source: "claude-haiku-*", label: "Haiku" },
] as const;

export function ClaudeDesktopModelSection({
  mappings,
  onChange,
  disabled,
}: {
  mappings: ProviderModelMapping[];
  onChange: (mappings: ProviderModelMapping[]) => void;
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
        Desktop 模型菜单来自 Desktop 自带的模型目录，新版本发布后会自动加入，1M
        版本按目录中的上下文长度提供；菜单变化需完全退出并重新打开 Desktop 才会显示。上游不接受
        Claude 模型名时，按系列填写上游模型，该系列所有版本（例如
        claude-opus-5-5）都会映射过去；个别版本需要单独处理时，在下方添加精确映射，精确映射优先。请求仍由
        AIO 网关按当前供应商转发和统计。供应商接口须兼容 Anthropic Messages；只填写模型名不会转换
        API 协议。
      </p>

      <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
        {FAMILIES.map((family) => (
          <FormField key={family.source} label={`${family.label} 系列`} hint={family.source}>
            <Input
              aria-label={`${family.label} 上游模型`}
              value={mappings.find((mapping) => mapping.source === family.source)?.target ?? ""}
              onChange={(event) => setMapping(family.source, event.currentTarget.value)}
              placeholder="沿用请求模型"
              disabled={disabled}
              mono
            />
          </FormField>
        ))}
      </div>
    </div>
  );
}
