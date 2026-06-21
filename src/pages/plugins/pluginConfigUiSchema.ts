import type { JsonValue } from "../../services/plugins";
import { isRecord } from "./pluginConfigValidation";

export type PluginConfigWidgetHint =
  | "text"
  | "textarea"
  | "password"
  | "number"
  | "switch"
  | "select"
  | "checkboxGroup"
  | "json";

export type PluginConfigUiSection = {
  id: string;
  title: string;
  description: string | null;
  order: number;
};

function stringOrNull(value: JsonValue | undefined): string | null {
  return typeof value === "string" && value.trim() ? value : null;
}

function numberOrInfinity(value: JsonValue | undefined): number {
  return typeof value === "number" && Number.isFinite(value) ? value : Number.POSITIVE_INFINITY;
}

function ui(schema: JsonValue | undefined): Record<string, JsonValue> {
  if (!isRecord(schema) || !isRecord(schema["x-aio-ui"])) return {};
  return schema["x-aio-ui"];
}

export function configSchemaSections(schema: JsonValue | undefined): PluginConfigUiSection[] {
  const rawSections = ui(schema).sections;
  if (!Array.isArray(rawSections)) return [];

  return rawSections
    .filter(isRecord)
    .map((section) => {
      const id = stringOrNull(section.id) ?? "default";
      return {
        id,
        title: stringOrNull(section.title) ?? id,
        description: stringOrNull(section.description),
        order: numberOrInfinity(section.order),
      };
    })
    .sort((left, right) => left.order - right.order || left.title.localeCompare(right.title));
}

export function configFieldLabel(
  key: string,
  fieldSchema: JsonValue | undefined,
  required: boolean
): string {
  const base = isRecord(fieldSchema) ? (stringOrNull(fieldSchema.title) ?? key) : key;
  return required ? `${base} *` : base;
}

export function configFieldDescription(fieldSchema: JsonValue | undefined): string | null {
  return isRecord(fieldSchema) ? stringOrNull(fieldSchema.description) : null;
}

export function configFieldSection(fieldSchema: JsonValue | undefined): string | null {
  return stringOrNull(ui(fieldSchema).section);
}

export function configFieldOrder(fieldSchema: JsonValue | undefined): number {
  return numberOrInfinity(ui(fieldSchema).order);
}

export function configFieldWidgetHint(
  fieldSchema: JsonValue | undefined
): PluginConfigWidgetHint | null {
  const widget = stringOrNull(ui(fieldSchema).widget);
  switch (widget) {
    case "text":
    case "textarea":
    case "password":
    case "number":
    case "switch":
    case "select":
    case "checkboxGroup":
    case "json":
      return widget;
    default:
      return null;
  }
}

export function configFieldPlaceholder(fieldSchema: JsonValue | undefined): string | null {
  return stringOrNull(ui(fieldSchema).placeholder);
}

export function configFieldWarning(
  fieldSchema: JsonValue | undefined,
  state: "always" | "partial" = "always"
): string | null {
  const fieldUi = ui(fieldSchema);
  if (state === "partial") {
    return stringOrNull(fieldUi.warningWhenPartial) ?? stringOrNull(fieldUi.warning);
  }
  return stringOrNull(fieldUi.warning);
}

export function enumOptionLabel(itemSchema: JsonValue | undefined, value: JsonValue): string {
  const labels = ui(itemSchema).enumLabels;
  const key = String(value);
  if (!isRecord(labels)) return key;

  const label = labels[key];
  return typeof label === "string" ? label : key;
}

export function enumOptionDescription(
  itemSchema: JsonValue | undefined,
  value: JsonValue
): string | null {
  const descriptions = ui(itemSchema).enumDescriptions;
  const key = String(value);
  if (!isRecord(descriptions)) return null;

  const description = descriptions[key];
  return typeof description === "string" ? description : null;
}
