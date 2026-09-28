import { Plus, X } from "lucide-react";
import type { ProviderCustomHeader } from "../../services/providers/providers";
import { Button } from "../../ui/Button";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { validateCustomHeaders } from "./providerCustomHeaders";

type CustomHeadersFieldProps = {
  headers: ProviderCustomHeader[];
  setHeaders: React.Dispatch<React.SetStateAction<ProviderCustomHeader[]>>;
  saving: boolean;
};

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
          const rowError = validateCustomHeaders([header]);
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
                  aria-invalid={rowError != null}
                />
                <Input
                  type="password"
                  autoComplete="off"
                  aria-invalid={rowError != null}
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
              {rowError ? <p className="text-xs text-destructive">{rowError}</p> : null}
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
