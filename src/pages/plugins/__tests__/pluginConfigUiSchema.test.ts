import { describe, expect, it } from "vitest";
import {
  configFieldDescription,
  configFieldLabel,
  configFieldOrder,
  configFieldPlaceholder,
  configFieldWarning,
  configFieldWidgetHint,
  configSchemaSections,
  enumOptionDescription,
  enumOptionLabel,
} from "../pluginConfigUiSchema";

describe("pluginConfigUiSchema", () => {
  const schema = {
    type: "object",
    "x-aio-ui": {
      sections: [
        { id: "routing", title: "处理位置", description: "选择插件在哪些阶段生效。", order: 20 },
        { id: "content", title: "要保护的内容", order: 10 },
      ],
    },
    properties: {
      redactBeforeUpstream: {
        type: "boolean",
        title: "发送给模型前处理",
        description: "在请求离开本机前替换敏感内容。",
        "x-aio-ui": {
          section: "routing",
          widget: "switch",
          order: 5,
          warning: "关闭后请求正文会原样发送。",
        },
      },
      sensitiveTypes: {
        type: "array",
        title: "要保护的内容",
        description: "选择需要处理的内容类型。",
        items: {
          type: "string",
          enum: ["email", "cn_phone"],
          "x-aio-ui": {
            enumLabels: {
              email: "邮箱地址",
              cn_phone: "中国手机号",
            },
            enumDescriptions: {
              email: "例如 name@example.com。",
              cn_phone: "例如 13344441520。",
            },
          },
        },
        "x-aio-ui": {
          section: "content",
          widget: "checkboxGroup",
          placeholder: "选择至少一种内容类型",
          warningWhenPartial: "关闭后，这类内容会原样发送给模型。",
        },
      },
    },
  };

  it("reads section metadata in stable order", () => {
    expect(configSchemaSections(schema)).toEqual([
      { id: "content", title: "要保护的内容", description: null, order: 10 },
      { id: "routing", title: "处理位置", description: "选择插件在哪些阶段生效。", order: 20 },
    ]);
  });

  it("prefers title and description over raw keys", () => {
    const field = schema.properties.redactBeforeUpstream;
    expect(configFieldLabel("redactBeforeUpstream", field, false)).toBe("发送给模型前处理");
    expect(configFieldLabel("redactBeforeUpstream", field, true)).toBe("发送给模型前处理 *");
    expect(configFieldDescription(field)).toBe("在请求离开本机前替换敏感内容。");
  });

  it("reads widget hints, order, placeholder, and warning copy", () => {
    const field = schema.properties.sensitiveTypes;
    expect(configFieldWidgetHint(field)).toBe("checkboxGroup");
    expect(configFieldOrder(field)).toBe(Number.POSITIVE_INFINITY);
    expect(configFieldPlaceholder(field)).toBe("选择至少一种内容类型");
    expect(configFieldWarning(field, "partial")).toBe("关闭后，这类内容会原样发送给模型。");
  });

  it("reads enum option labels and descriptions from item metadata", () => {
    const items = schema.properties.sensitiveTypes.items;
    expect(enumOptionLabel(items, "email")).toBe("邮箱地址");
    expect(enumOptionDescription(items, "cn_phone")).toBe("例如 13344441520。");
    expect(enumOptionLabel(items, "unknown")).toBe("unknown");
  });
});
