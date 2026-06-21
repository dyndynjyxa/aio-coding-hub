import { describe, expect, it } from "vitest";
import { buildPluginConfigRenderModel } from "../pluginConfigRenderModel";

describe("buildPluginConfigRenderModel", () => {
  it("compiles titled fields into ordered sections", () => {
    const model = buildPluginConfigRenderModel({
      schema: {
        type: "object",
        required: ["enabled"],
        "x-aio-ui": {
          sections: [
            { id: "content", title: "内容策略", order: 20 },
            { id: "routing", title: "处理位置", order: 10 },
          ],
        },
        properties: {
          enabled: {
            type: "boolean",
            title: "启用处理",
            description: "关闭后插件不会修改内容。",
            default: true,
            "x-aio-ui": { section: "routing", widget: "switch", order: 10 },
          },
          mode: {
            type: "string",
            title: "处理模式",
            default: "balanced",
            enum: ["balanced", "strict"],
            "x-aio-ui": {
              section: "content",
              widget: "select",
              order: 5,
              enumLabels: { balanced: "平衡", strict: "严格" },
            },
          },
        },
      },
      value: {},
    });

    expect(model.sections.map((section) => section.title)).toEqual(["处理位置", "内容策略"]);
    expect(model.sections[0].fields[0]).toMatchObject({
      key: "enabled",
      label: "启用处理 *",
      description: "关闭后插件不会修改内容。",
      widget: "switch",
      value: true,
    });
    expect(model.sections[1].fields[0]).toMatchObject({
      key: "mode",
      label: "处理模式",
      widget: "select",
      value: "balanced",
      options: [
        { value: "balanced", label: "平衡", description: null },
        { value: "strict", label: "严格", description: null },
      ],
    });
  });

  it("compiles array enum fields into checkbox groups with option descriptions", () => {
    const model = buildPluginConfigRenderModel({
      schema: {
        type: "object",
        properties: {
          sensitiveTypes: {
            type: "array",
            title: "要保护的内容",
            default: ["email", "cn_phone"],
            items: {
              type: "string",
              enum: ["email", "cn_phone"],
              "x-aio-ui": {
                enumLabels: { email: "邮箱地址", cn_phone: "中国手机号" },
                enumDescriptions: {
                  email: "例如 name@example.com。",
                  cn_phone: "例如 13344441520。",
                },
              },
            },
            "x-aio-ui": {
              widget: "checkboxGroup",
              warningWhenPartial: "关闭后，这类内容会原样发送给模型。",
            },
          },
        },
      },
      value: { sensitiveTypes: ["email"] },
    });

    expect(model.sections[0].fields[0]).toMatchObject({
      key: "sensitiveTypes",
      widget: "checkboxGroup",
      value: ["email"],
      warning: "关闭后，这类内容会原样发送给模型。",
      options: [
        { value: "email", label: "邮箱地址", description: "例如 name@example.com。" },
        { value: "cn_phone", label: "中国手机号", description: "例如 13344441520。" },
      ],
    });
  });

  it("falls back unsupported structured fields to json widgets", () => {
    const model = buildPluginConfigRenderModel({
      schema: {
        type: "object",
        properties: {
          advanced: {
            type: "object",
            title: "高级配置",
          },
        },
      },
      value: { advanced: { retries: 2 } },
    });

    expect(model.sections[0].fields[0]).toMatchObject({
      key: "advanced",
      label: "高级配置",
      widget: "json",
      value: { retries: 2 },
    });
  });
});
