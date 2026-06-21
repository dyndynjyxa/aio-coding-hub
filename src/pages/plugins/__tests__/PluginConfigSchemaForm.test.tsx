import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PluginConfigSchemaForm } from "../PluginConfigSchemaForm";

describe("PluginConfigSchemaForm", () => {
  it("resets draft when the plugin config identity changes", () => {
    const onSubmit = vi.fn();
    const schema = {
      type: "object",
      properties: {
        mode: { type: "string", enum: ["strict", "balanced"] },
      },
    };

    const { rerender } = render(
      <PluginConfigSchemaForm
        identity="official.privacy-filter:1"
        schema={schema}
        value={{ mode: "strict" }}
        pending={false}
        onSubmit={onSubmit}
      />
    );
    fireEvent.change(screen.getByLabelText("mode"), { target: { value: "balanced" } });

    rerender(
      <PluginConfigSchemaForm
        identity="acme.other:1"
        schema={schema}
        value={{ mode: "strict" }}
        pending={false}
        onSubmit={onSubmit}
      />
    );
    fireEvent.click(screen.getByRole("button", { name: "保存配置" }));

    expect(onSubmit).toHaveBeenCalledWith({ mode: "strict" });
  });

  it("renders enum as a select when enum is a string-field keyword", () => {
    render(
      <PluginConfigSchemaForm
        identity="test.enum:1"
        schema={{
          type: "object",
          properties: {
            mode: { type: "string", enum: ["strict", "balanced"] },
          },
        }}
        value={{ mode: "strict" }}
        pending={false}
        onSubmit={vi.fn()}
      />
    );

    expect(screen.getByRole("combobox", { name: "mode" })).toBeInTheDocument();
  });

  it("renders password fields as password inputs without claiming host secret storage", () => {
    render(
      <PluginConfigSchemaForm
        identity="test.password:1"
        schema={{
          type: "object",
          properties: {
            token: { type: "password" },
          },
        }}
        value={{ token: "saved-token" }}
        pending={false}
        onSubmit={vi.fn()}
      />
    );

    expect(screen.getByLabelText("token")).toHaveAttribute("type", "password");
  });

  it("renders object schema fields and submits typed config", () => {
    const onSubmit = vi.fn();

    render(
      <PluginConfigSchemaForm
        identity="test.config:1"
        schema={{
          type: "object",
          required: ["mode", "enabled"],
          properties: {
            mode: { type: "string", enum: ["append_instruction", "rewrite_system_message"] },
            threshold: { type: "integer" },
            enabled: { type: "boolean" },
          },
        }}
        value={{ mode: "append_instruction", threshold: 2, enabled: false }}
        onSubmit={onSubmit}
        pending={false}
      />
    );

    fireEvent.change(screen.getByLabelText("mode"), {
      target: { value: "rewrite_system_message" },
    });
    fireEvent.change(screen.getByLabelText("threshold"), { target: { value: "4" } });
    fireEvent.click(screen.getByLabelText("enabled"));
    fireEvent.click(screen.getByRole("button", { name: "保存配置" }));

    expect(onSubmit).toHaveBeenCalledWith({
      mode: "rewrite_system_message",
      threshold: 4,
      enabled: true,
    });
  });

  it("renders titled sections, descriptions, switches, selects, and checkbox groups", () => {
    const onSubmit = vi.fn();

    render(
      <PluginConfigSchemaForm
        identity="publisher.sample:1"
        schema={{
          type: "object",
          required: ["redactBeforeUpstream"],
          "x-aio-ui": {
            sections: [
              {
                id: "routing",
                title: "处理位置",
                description: "选择插件在哪些阶段生效。",
                order: 10,
              },
              { id: "content", title: "要保护的内容", order: 20 },
            ],
          },
          properties: {
            redactBeforeUpstream: {
              type: "boolean",
              title: "发送给模型前处理",
              description: "在请求离开本机前替换敏感内容。",
              default: true,
              "x-aio-ui": { section: "routing", widget: "switch", order: 10 },
            },
            profile: {
              type: "string",
              title: "保护强度",
              default: "balanced",
              enum: ["balanced", "strict"],
              "x-aio-ui": {
                section: "routing",
                widget: "select",
                order: 20,
                enumLabels: { balanced: "平衡", strict: "严格" },
              },
            },
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
                section: "content",
                widget: "checkboxGroup",
                warningWhenPartial: "关闭后，这类内容会原样发送给模型。",
              },
            },
          },
        }}
        value={{ sensitiveTypes: ["email", "cn_phone"] }}
        pending={false}
        onSubmit={onSubmit}
      />
    );

    expect(screen.getByText("处理位置")).toBeInTheDocument();
    expect(screen.getByText("选择插件在哪些阶段生效。")).toBeInTheDocument();
    expect(screen.getByText("要保护的内容")).toBeInTheDocument();
    expect(screen.getByLabelText("发送给模型前处理 *")).toBeChecked();
    expect(screen.getByRole("combobox", { name: "保护强度" })).toHaveValue("balanced");
    expect(screen.getByText("平衡")).toBeInTheDocument();
    expect(screen.getByLabelText("邮箱地址")).toBeChecked();
    expect(screen.getByText("例如 name@example.com。")).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText("邮箱地址"));
    fireEvent.click(screen.getByRole("button", { name: "保存配置" }));

    expect(onSubmit).toHaveBeenCalledWith({
      redactBeforeUpstream: true,
      profile: "balanced",
      sensitiveTypes: ["cn_phone"],
    });
  });

  it("keeps unsupported object fields editable through json fallback", () => {
    const onSubmit = vi.fn();

    render(
      <PluginConfigSchemaForm
        identity="publisher.advanced:1"
        schema={{
          type: "object",
          properties: {
            advanced: { type: "object", title: "高级配置" },
          },
        }}
        value={{ advanced: { retries: 2 } }}
        pending={false}
        onSubmit={onSubmit}
      />
    );

    fireEvent.change(screen.getByLabelText("高级配置"), {
      target: { value: '{"retries":3}' },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存配置" }));

    expect(onSubmit).toHaveBeenCalledWith({ advanced: { retries: 3 } });
  });

  it("blocks submit and shows a field error for invalid json fields", () => {
    const onSubmit = vi.fn();

    render(
      <PluginConfigSchemaForm
        identity="publisher.advanced:1"
        schema={{
          type: "object",
          properties: {
            advanced: { type: "object", title: "高级配置" },
          },
        }}
        value={{ advanced: { retries: 2 } }}
        pending={false}
        onSubmit={onSubmit}
      />
    );

    fireEvent.change(screen.getByLabelText("高级配置"), {
      target: { value: '{"retries":' },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存配置" }));

    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByText("请输入合法的 JSON 对象。")).toBeInTheDocument();
  });

  it("keeps blank optional numbers unset instead of coercing them to zero", () => {
    const onSubmit = vi.fn();

    render(
      <PluginConfigSchemaForm
        identity="publisher.number:1"
        schema={{
          type: "object",
          properties: {
            threshold: { type: "integer", title: "阈值" },
            enabled: { type: "boolean", default: true },
          },
        }}
        value={{ threshold: 3 }}
        pending={false}
        onSubmit={onSubmit}
      />
    );

    fireEvent.change(screen.getByLabelText("阈值"), { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "保存配置" }));

    expect(onSubmit).toHaveBeenCalledWith({ enabled: true });
  });

  it("submits numeric enum select values without converting them to strings", () => {
    const onSubmit = vi.fn();

    render(
      <PluginConfigSchemaForm
        identity="publisher.numeric-enum:1"
        schema={{
          type: "object",
          properties: {
            retryLimit: {
              type: "integer",
              title: "重试次数",
              enum: [1, 2, 3],
              default: 1,
            },
          },
        }}
        value={{ retryLimit: 1 }}
        pending={false}
        onSubmit={onSubmit}
      />
    );

    fireEvent.change(screen.getByRole("combobox", { name: "重试次数" }), {
      target: { value: "3" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存配置" }));

    expect(onSubmit).toHaveBeenCalledWith({ retryLimit: 3 });
  });

  it("renders textarea widget hints as multiline controls", () => {
    render(
      <PluginConfigSchemaForm
        identity="publisher.textarea:1"
        schema={{
          type: "object",
          properties: {
            promptTemplate: {
              type: "string",
              title: "提示词模板",
              "x-aio-ui": { widget: "textarea" },
            },
          },
        }}
        value={{ promptTemplate: "第一行\n第二行" }}
        pending={false}
        onSubmit={vi.fn()}
      />
    );

    expect(screen.getByLabelText("提示词模板").tagName).toBe("TEXTAREA");
  });
});
