import type { JsonValue } from "../../services/plugins";
import {
  configFieldDescription,
  configFieldLabel,
  configFieldOrder,
  configFieldPlaceholder,
  configFieldSection,
  configFieldWarning,
  configFieldWidgetHint,
  configSchemaSections,
  enumOptionDescription,
  enumOptionLabel,
} from "./pluginConfigUiSchema";
import {
  isRecord,
  schemaArrayItemEnum,
  schemaDefault,
  schemaEnum,
  schemaItems,
  schemaProperties,
  schemaRequired,
  schemaType,
} from "./pluginConfigValidation";

export type PluginConfigWidget =
  | "text"
  | "textarea"
  | "password"
  | "number"
  | "switch"
  | "select"
  | "checkboxGroup"
  | "json";

export type PluginConfigOptionModel = {
  value: JsonValue;
  label: string;
  description: string | null;
};

export type PluginConfigFieldModel = {
  key: string;
  label: string;
  description: string | null;
  placeholder: string | null;
  required: boolean;
  type: string | null;
  widget: PluginConfigWidget;
  value: JsonValue | undefined;
  options: PluginConfigOptionModel[];
  warning: string | null;
  order: number;
};

export type PluginConfigSectionModel = {
  id: string;
  title: string;
  description: string | null;
  order: number;
  fields: PluginConfigFieldModel[];
};

export type PluginConfigRenderModel = {
  editable: boolean;
  sections: PluginConfigSectionModel[];
};

const UNORDERED_BASE = 1_000_000;

function valueForField(
  config: Record<string, JsonValue>,
  key: string,
  fieldSchema: JsonValue
): JsonValue | undefined {
  if (Object.prototype.hasOwnProperty.call(config, key)) return config[key];
  return schemaDefault(fieldSchema);
}

function widgetForField(
  fieldSchema: JsonValue,
  hasEnum: boolean,
  hasArrayEnum: boolean
): PluginConfigWidget {
  const type = schemaType(fieldSchema);
  const hint = configFieldWidgetHint(fieldSchema);

  if (hint === "checkboxGroup" && hasArrayEnum) return "checkboxGroup";
  if (hint === "select" && hasEnum) return "select";
  if (hint === "switch" && type === "boolean") return "switch";
  if (hint === "textarea" && type === "string") return "textarea";
  if (hint === "password") return "password";
  if (hint === "number" && (type === "number" || type === "integer")) return "number";
  if (hint === "json") return "json";

  if (type === "boolean") return "switch";
  if (hasArrayEnum) return "checkboxGroup";
  if (hasEnum) return "select";
  if (type === "password") return "password";
  if (type === "number" || type === "integer") return "number";
  if (type === "string" || type == null) return "text";
  return "json";
}

function normalizedOrder(order: number, fallbackIndex: number): number {
  return Number.isFinite(order) ? order : UNORDERED_BASE + fallbackIndex;
}

export function buildPluginConfigRenderModel(input: {
  schema: JsonValue | null | undefined;
  value: JsonValue;
}): PluginConfigRenderModel {
  const properties = schemaProperties(input.schema);
  const entries = Object.entries(properties);
  if (schemaType(input.schema) !== "object" || entries.length === 0) {
    return { editable: false, sections: [] };
  }

  const config = isRecord(input.value) ? input.value : {};
  const required = schemaRequired(input.schema);
  const sectionMap = new Map<string, PluginConfigSectionModel>();

  for (const section of configSchemaSections(input.schema)) {
    sectionMap.set(section.id, { ...section, fields: [] });
  }

  function ensureSection(id: string | null): PluginConfigSectionModel {
    const normalizedId = id ?? "default";
    const existing = sectionMap.get(normalizedId);
    if (existing) return existing;

    const section = {
      id: normalizedId,
      title: normalizedId === "default" ? "常规设置" : normalizedId,
      description: null,
      order: UNORDERED_BASE + sectionMap.size,
      fields: [],
    };
    sectionMap.set(normalizedId, section);
    return section;
  }

  entries.forEach(([key, fieldSchema], index) => {
    const enumValues = schemaEnum(fieldSchema);
    const arrayEnumValues = schemaArrayItemEnum(fieldSchema);
    const items = schemaItems(fieldSchema);
    const hasEnum = enumValues.length > 0;
    const hasArrayEnum = arrayEnumValues.length > 0;
    const widget = widgetForField(fieldSchema, hasEnum, hasArrayEnum);
    const optionSource = hasArrayEnum ? arrayEnumValues : enumValues;
    const optionSchema = hasArrayEnum ? items : fieldSchema;
    const fieldOrder = normalizedOrder(configFieldOrder(fieldSchema), index);

    ensureSection(configFieldSection(fieldSchema)).fields.push({
      key,
      label: configFieldLabel(key, fieldSchema, required.has(key)),
      description: configFieldDescription(fieldSchema),
      placeholder: configFieldPlaceholder(fieldSchema),
      required: required.has(key),
      type: schemaType(fieldSchema),
      widget,
      value: valueForField(config, key, fieldSchema),
      options: optionSource.map((value) => ({
        value,
        label: enumOptionLabel(optionSchema, value),
        description: enumOptionDescription(optionSchema, value),
      })),
      warning:
        widget === "checkboxGroup"
          ? configFieldWarning(fieldSchema, "partial")
          : configFieldWarning(fieldSchema),
      order: fieldOrder,
    });
  });

  const sections = [...sectionMap.values()]
    .map((section) => ({
      ...section,
      fields: section.fields.sort((left, right) => left.order - right.order),
    }))
    .filter((section) => section.fields.length > 0)
    .sort((left, right) => left.order - right.order || left.title.localeCompare(right.title));

  return { editable: true, sections };
}
