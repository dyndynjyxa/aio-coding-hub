// Usage: Render and edit the manifest configSchema subset supported by the plugin host.

import { useEffect, useMemo, useRef, useState } from "react";
import type { JsonValue } from "../../services/plugins";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Switch } from "../../ui/Switch";
import { Textarea } from "../../ui/Textarea";
import {
  buildPluginConfigRenderModel,
  type PluginConfigFieldModel,
} from "./pluginConfigRenderModel";
import { isRecord, parseConfigField, type PluginConfigObject } from "./pluginConfigValidation";

export type PluginConfigSchemaFormProps = {
  identity: string;
  schema: JsonValue | null | undefined;
  value: JsonValue;
  pending: boolean;
  onSubmit: (value: JsonValue) => void;
};

function fieldToText(value: JsonValue | undefined, type: string | null): string {
  if (value == null) return "";
  if (type === "object" || type === "array") {
    return JSON.stringify(value, null, 2);
  }
  return String(value);
}

function initialObject(value: JsonValue): PluginConfigObject {
  return isRecord(value) ? { ...value } : {};
}

function selectValueForField(value: JsonValue | undefined): string {
  return value == null ? "" : String(value);
}

function selectedValueForField(field: PluginConfigFieldModel, raw: string): JsonValue {
  if (field.type === "number" || field.type === "integer") {
    const parsed = parseConfigField(raw, field.type);
    return parsed.ok ? (parsed.value ?? "") : raw;
  }
  const option = field.options.find((item) => String(item.value) === raw);
  return option?.value ?? raw;
}

export function PluginConfigSchemaForm({
  identity,
  schema,
  value,
  pending,
  onSubmit,
}: PluginConfigSchemaFormProps) {
  const [draft, setDraft] = useState<PluginConfigObject>(() => initialObject(value));
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const valueRef = useRef(value);
  const model = useMemo(
    () => buildPluginConfigRenderModel({ schema, value: draft }),
    [schema, draft]
  );

  valueRef.current = value;
  useEffect(() => {
    setDraft(initialObject(valueRef.current));
    setFieldErrors({});
  }, [identity]);

  if (!model.editable) {
    return (
      <div className="space-y-3">
        <div className="text-sm text-muted-foreground">此插件没有可编辑配置。</div>
        <pre className="max-h-48 overflow-auto rounded-md border border-border bg-muted/40 p-3 text-xs">
          {JSON.stringify(value ?? {}, null, 2)}
        </pre>
      </div>
    );
  }

  function setField(key: string, next: JsonValue) {
    setDraft((current) => ({ ...current, [key]: next }));
  }

  function setParsedField(key: string, raw: string, type: string | null) {
    const parsed = parseConfigField(raw, type);
    if (!parsed.ok) {
      setFieldErrors((current) => ({ ...current, [key]: parsed.error }));
      return;
    }
    setFieldErrors((current) => {
      const next = { ...current };
      delete next[key];
      return next;
    });
    setDraft((current) => {
      const next = { ...current };
      if (parsed.value === undefined) {
        delete next[key];
      } else {
        next[key] = parsed.value;
      }
      return next;
    });
  }

  function buildSubmitValue(): PluginConfigObject {
    const next = { ...draft };
    for (const section of model.sections) {
      for (const field of section.fields) {
        if (field.value !== undefined) {
          next[field.key] = field.value;
        }
      }
    }
    return next;
  }

  function fieldAriaLabel(field: PluginConfigFieldModel): string {
    return field.label === `${field.key} *` ? field.key : field.label;
  }

  function renderFieldError(field: PluginConfigFieldModel) {
    return fieldErrors[field.key] ? (
      <span className="text-xs text-destructive">{fieldErrors[field.key]}</span>
    ) : null;
  }

  function renderField(field: PluginConfigFieldModel, sectionTitle: string) {
    const label = field.label;
    const current = field.value;
    const ariaLabel = fieldAriaLabel(field);

    if (field.widget === "switch") {
      return (
        <label
          key={field.key}
          className="flex items-center justify-between gap-3 rounded-md border border-border px-3 py-2"
        >
          <span>
            <span className="block text-sm font-medium">{label}</span>
            {field.description ? (
              <span className="block text-xs text-muted-foreground">{field.description}</span>
            ) : null}
          </span>
          <Switch
            aria-label={ariaLabel}
            checked={Boolean(current)}
            onCheckedChange={(checked) => setField(field.key, checked)}
          />
        </label>
      );
    }

    if (field.widget === "select") {
      return (
        <label key={field.key} className="grid gap-1.5 text-sm">
          <span className="font-medium">{label}</span>
          {field.description ? (
            <span className="text-xs text-muted-foreground">{field.description}</span>
          ) : null}
          <select
            aria-label={ariaLabel}
            className="h-9 rounded-md border border-input bg-background px-3 text-sm"
            value={selectValueForField(current)}
            onChange={(event) => {
              setField(field.key, selectedValueForField(field, event.target.value));
            }}
          >
            {field.options.map((option) => (
              <option key={String(option.value)} value={String(option.value)}>
                {option.label}
              </option>
            ))}
          </select>
          {field.warning ? <span className="text-xs text-warning">{field.warning}</span> : null}
          {renderFieldError(field)}
        </label>
      );
    }

    if (field.widget === "checkboxGroup") {
      const currentArray = Array.isArray(current) ? current : [];
      const showLegend = label !== sectionTitle;
      return (
        <fieldset
          key={field.key}
          aria-label={showLegend ? undefined : label}
          className="grid gap-2 rounded-md border border-border px-3 py-2"
        >
          {showLegend ? <legend className="px-1 text-sm font-medium">{label}</legend> : null}
          {field.description ? (
            <div className="text-xs text-muted-foreground">{field.description}</div>
          ) : null}
          {field.options.map((option) => {
            const itemText = String(option.value);
            const checked = currentArray.some((item) => String(item) === itemText);
            return (
              <label key={itemText} className="flex items-start gap-2 text-sm">
                <input
                  aria-label={option.label}
                  className="mt-1"
                  type="checkbox"
                  checked={checked}
                  onChange={(event) => {
                    const next = event.target.checked
                      ? [...currentArray, option.value]
                      : currentArray.filter((item) => String(item) !== itemText);
                    setField(field.key, next);
                  }}
                />
                <span>
                  <span className="block font-medium">{option.label}</span>
                  {option.description ? (
                    <span className="block text-xs text-muted-foreground">
                      {option.description}
                    </span>
                  ) : null}
                </span>
              </label>
            );
          })}
          {field.warning && currentArray.length < field.options.length ? (
            <div className="rounded-md border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
              {field.warning}
            </div>
          ) : null}
          {renderFieldError(field)}
        </fieldset>
      );
    }

    if (field.widget === "textarea") {
      return (
        <label key={field.key} className="grid gap-1.5 text-sm">
          <span className="font-medium">{label}</span>
          {field.description ? (
            <span className="text-xs text-muted-foreground">{field.description}</span>
          ) : null}
          <Textarea
            aria-label={ariaLabel}
            placeholder={field.placeholder ?? undefined}
            value={fieldToText(current, field.type)}
            onChange={(event) => setParsedField(field.key, event.target.value, field.type)}
          />
          {field.warning ? <span className="text-xs text-warning">{field.warning}</span> : null}
          {renderFieldError(field)}
        </label>
      );
    }

    if (field.widget === "json") {
      return (
        <label key={field.key} className="grid gap-1.5 text-sm">
          <span className="font-medium">{label}</span>
          {field.description ? (
            <span className="text-xs text-muted-foreground">{field.description}</span>
          ) : null}
          <Textarea
            aria-label={ariaLabel}
            value={fieldToText(current, field.type)}
            onChange={(event) => setParsedField(field.key, event.target.value, field.type)}
          />
          {renderFieldError(field)}
        </label>
      );
    }

    return (
      <label key={field.key} className="grid gap-1.5 text-sm">
        <span className="font-medium">{label}</span>
        {field.description ? (
          <span className="text-xs text-muted-foreground">{field.description}</span>
        ) : null}
        <Input
          aria-label={ariaLabel}
          placeholder={field.placeholder ?? undefined}
          type={
            field.widget === "password" ? "password" : field.widget === "number" ? "number" : "text"
          }
          value={fieldToText(current, field.type)}
          onChange={(event) => setParsedField(field.key, event.target.value, field.type)}
        />
        {field.warning ? <span className="text-xs text-warning">{field.warning}</span> : null}
        {renderFieldError(field)}
      </label>
    );
  }

  return (
    <form
      className="space-y-4"
      onSubmit={(event) => {
        event.preventDefault();
        if (Object.keys(fieldErrors).length > 0) return;
        onSubmit(buildSubmitValue());
      }}
    >
      <div className="grid gap-4">
        {model.sections.map((section) => (
          <section key={section.id} className="grid gap-3">
            <div>
              <h3 className="text-sm font-semibold text-foreground">{section.title}</h3>
              {section.description ? (
                <p className="text-xs text-muted-foreground">{section.description}</p>
              ) : null}
            </div>
            <div className="grid gap-3">
              {section.fields.map((field) => renderField(field, section.title))}
            </div>
          </section>
        ))}
      </div>

      <div className="flex justify-end">
        <Button type="submit" disabled={pending || Object.keys(fieldErrors).length > 0}>
          保存配置
        </Button>
      </div>
    </form>
  );
}
