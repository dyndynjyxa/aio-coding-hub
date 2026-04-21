import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";
import { BaseUrlEditor } from "./BaseUrlEditor";
import { RadioButtonGroup } from "./RadioButtonGroup";
import { TagsField } from "./TagsField";
import { isZeroMultiplier } from "./providerEditorUtils";
import type { ProviderBaseUrlMode } from "./types";
import type { UseProviderEditorFormReturn } from "./useProviderEditorForm";

export function ApiKeySection(props: { form: UseProviderEditorFormReturn }) {
  const {
    mode,
    register,
    setValue,
    saving,
    copyingApiKey,
    tags,
    setTags,
    tagInput,
    setTagInput,
    baseUrlMode,
    setBaseUrlMode,
    baseUrlRows,
    setBaseUrlRows,
    pingingAll,
    setPingingAll,
    newBaseUrlRow,
    apiKeyField,
    apiKeyConfigured,
    apiKeyValue,
    costMultiplierValue,
    copyApiKey,
  } = props.form;

  const canCopyApiKey =
    Boolean(apiKeyValue.trim()) || (mode === "edit" && apiKeyConfigured);
  const apiKeyHint =
    mode === "edit"
      ? apiKeyConfigured
        ? "已配置。留空表示不改，输入新值表示替换。"
        : "当前未配置。请输入新 API Key 后保存。"
      : undefined;
  const apiKeyPlaceholder =
    mode === "edit" && apiKeyConfigured ? "留空表示不改；输入新值表示替换" : "sk-…";

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

      <FormField label="Base URLs">
        <BaseUrlEditor
          rows={baseUrlRows}
          setRows={setBaseUrlRows}
          pingingAll={pingingAll}
          setPingingAll={setPingingAll}
          newRow={newBaseUrlRow}
          placeholder="中转 endpoint（例如：https://example.com/v1）"
          disabled={saving}
          footerStart={
            <div className="flex items-center gap-2">
              <span className="shrink-0 text-[11px] font-medium text-slate-500 dark:text-slate-400">
                URL 选择策略
              </span>
              <RadioButtonGroup<ProviderBaseUrlMode>
                items={[
                  { value: "order", label: "按顺序" },
                  { value: "ping", label: "按 Ping" },
                ]}
                ariaLabel="Base URL 选择策略"
                value={baseUrlMode}
                onChange={setBaseUrlMode}
                disabled={saving}
                size="compact"
                fullWidth={false}
              />
            </div>
          }
        />
      </FormField>

      <div className="grid gap-3 sm:grid-cols-2">
        <FormField label="API Key / Token" hint={apiKeyHint}>
          <div className="flex items-center gap-2">
            <Input
              {...apiKeyField}
              type="text"
              placeholder={apiKeyPlaceholder}
              autoComplete="off"
            />
            <Button
              type="button"
              onClick={() => void copyApiKey()}
              variant="secondary"
              size="sm"
              className="h-9 shrink-0"
              disabled={saving || copyingApiKey || !canCopyApiKey}
            >
              {copyingApiKey ? "复制中…" : "复制"}
            </Button>
          </div>
        </FormField>

        <FormField label="价格倍率">
          <div className="flex items-center gap-2">
            <Input
              type="number"
              min="0"
              step="0.01"
              placeholder="1.0"
              {...register("cost_multiplier")}
            />
            <Button
              type="button"
              variant="secondary"
              size="sm"
              className={
                isZeroMultiplier(costMultiplierValue)
                  ? "h-9 shrink-0 border-emerald-200 bg-emerald-50 text-emerald-700 hover:bg-emerald-100 dark:border-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-300 dark:hover:bg-emerald-900/40"
                  : "h-9 shrink-0"
              }
              disabled={saving}
              onClick={() =>
                setValue("cost_multiplier", "0", {
                  shouldDirty: true,
                  shouldTouch: true,
                  shouldValidate: true,
                })
              }
            >
              免费
            </Button>
          </div>
        </FormField>
      </div>
    </>
  );
}
