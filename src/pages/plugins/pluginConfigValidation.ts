// Usage: Small JSON-schema subset helpers for plugin config editing.

import type { JsonValue } from "../../services/plugins";

export type PluginConfigObject = Record<string, JsonValue>;

export type ConfigFieldParseResult =
  | { ok: true; value: JsonValue | undefined }
  | { ok: false; error: string };

export function isRecord(value: JsonValue | unknown): value is Record<string, JsonValue> {
  return value != null && typeof value === "object" && !Array.isArray(value);
}

export function schemaType(schema: JsonValue | undefined): string | null {
  if (!isRecord(schema)) return null;
  const type = schema.type;
  return typeof type === "string" ? type : null;
}

export function schemaProperties(schema: JsonValue | undefined): Record<string, JsonValue> {
  if (!isRecord(schema) || !isRecord(schema.properties)) return {};
  return schema.properties;
}

export function schemaRequired(schema: JsonValue | undefined): Set<string> {
  if (!isRecord(schema) || !Array.isArray(schema.required)) return new Set();
  return new Set(schema.required.filter((item): item is string => typeof item === "string"));
}

export function schemaEnum(schema: JsonValue | undefined): JsonValue[] {
  if (!isRecord(schema) || !Array.isArray(schema.enum)) return [];
  return schema.enum as JsonValue[];
}

export function schemaDefault(schema: JsonValue | undefined): JsonValue | undefined {
  if (!isRecord(schema) || !("default" in schema)) return undefined;
  return schema.default;
}

export function schemaItems(schema: JsonValue | undefined): JsonValue | undefined {
  if (!isRecord(schema)) return undefined;
  return schema.items;
}

export function schemaArrayItemEnum(schema: JsonValue | undefined): JsonValue[] {
  const items = schemaItems(schema);
  if (!isRecord(schema) || schema.type !== "array" || !isRecord(items)) return [];
  return schemaEnum(items);
}

export function parseConfigField(raw: string, type: string | null): ConfigFieldParseResult {
  if (type === "integer" || type === "number") {
    if (raw.trim() === "") return { ok: true, value: undefined };
    const parsed = Number(raw);
    if (!Number.isFinite(parsed) || (type === "integer" && !Number.isInteger(parsed))) {
      return { ok: false, error: "请输入有效数字。" };
    }
    return { ok: true, value: parsed };
  }

  if (type === "array" || type === "object") {
    if (raw.trim() === "") return { ok: true, value: undefined };
    try {
      const parsed = JSON.parse(raw) as JsonValue;
      if (type === "array" && !Array.isArray(parsed)) {
        return { ok: false, error: "请输入合法的 JSON 数组。" };
      }
      if (type === "object" && !isRecord(parsed)) {
        return { ok: false, error: "请输入合法的 JSON 对象。" };
      }
      return { ok: true, value: parsed };
    } catch {
      return {
        ok: false,
        error: type === "array" ? "请输入合法的 JSON 数组。" : "请输入合法的 JSON 对象。",
      };
    }
  }

  return { ok: true, value: raw };
}
