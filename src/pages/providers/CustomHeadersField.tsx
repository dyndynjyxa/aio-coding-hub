import { Plus, X } from "lucide-react";
import type { ProviderCustomHeader } from "../../services/providers/providers";
import { Button } from "../../ui/Button";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { isProtectedCustomHeaderName, isValidCustomHeaderName } from "./providerCustomHeaders";

type CustomHeadersFieldProps = {
  headers: ProviderCustomHeader[];
  setHeaders: React.Dispatch<React.SetStateAction<ProviderCustomHeader[]>>;
  saving: boolean;
};

function headerNameError(name: string): string | null {
  const trimmed = name.trim();
  if (!trimmed) return null;
  if (!isValidCustomHeaderName(trimmed)) return "名称含非法字符";
  if (isProtectedCustomHeaderName(trimmed)) return "该请求头由网关管理";
  return null;
}

export function CustomHeadersField({ headers, setHeaders, saving }: CustomHeadersFieldProps) {
  const updateAt = (index: number, patch: Partial<ProviderCustomHeader>) => {
    setHeaders((prev) => prev.map((header, i) => (i === index ? { ...header, ...patch } : header)));
  };

  const removeAt = (index: number) => {
    setHeaders((prev) => prev.filter((_, i) => i !== index));
  };

  const addRow = () => {
    setHeaders((prev) => [...prev, { name: "", value: "" }]);
  };

  return (
    <FormField
      label="自定义请求头"
      hint="转发到上游时附加；适用于需要额外身份/鉴权头的网关。名称大小写不敏感、自动去重。"
    >
      <div className="space-y-2">
        {headers.map((header, index) => {
          const nameError = headerNameError(header.name);
          return (
            <div key={index} className="space-y-1">
              <div className="flex items-center gap-2">
                <Input
                  type="text"
                  value={header.name}
                  onChange={(e) => updateAt(index, { name: e.currentTarget.value })}
                  placeholder="名称，如 X-User-Id"
                  className="flex-1"
                  disabled={saving}
                  aria-label={`请求头名称 ${index + 1}`}
                  aria-invalid={nameError != null}
                />
                <Input
                  type="text"
                  value={header.value}
                  onChange={(e) => updateAt(index, { value: e.currentTarget.value })}
                  placeholder="值"
                  className="flex-1"
                  disabled={saving}
                  aria-label={`请求头值 ${index + 1}`}
                />
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => removeAt(index)}
                  disabled={saving}
                  aria-label={`移除请求头 ${index + 1}`}
                >
                  <X className="h-4 w-4" />
                </Button>
              </div>
              {nameError ? <p className="text-xs text-destructive">{nameError}</p> : null}
            </div>
          );
        })}
        <Button variant="secondary" size="sm" onClick={addRow} disabled={saving}>
          <Plus className="mr-1 h-3.5 w-3.5" />
          添加请求头
        </Button>
      </div>
    </FormField>
  );
}
