import { describe, expect, it } from "vitest";
import {
  describePluginPermission,
  describePluginRuntime,
  pluginRiskLabel,
  pluginStatusLabel,
} from "../pluginProductCopy";

describe("pluginProductCopy", () => {
  it("translates plugin statuses into user-facing Chinese labels", () => {
    expect(pluginStatusLabel("enabled")).toBe("运行中");
    expect(pluginStatusLabel("disabled")).toBe("已关闭");
    expect(pluginStatusLabel("quarantined")).toBe("已隔离");
  });

  it("translates permission ids into user impact copy", () => {
    expect(describePluginPermission("request.body.read")).toEqual({
      label: "读取你发送给模型的内容",
      detail: "用于检查或分析请求正文。",
      risk: "high",
    });
    expect(describePluginPermission("request.body.write")).toEqual({
      label: "修改你发送给模型的内容",
      detail: "用于在发送前替换、追加或删除请求正文。",
      risk: "high",
    });
    expect(describePluginPermission("log.redact")).toEqual({
      label: "处理本地请求日志",
      detail: "用于在日志保存前隐藏敏感信息。",
      risk: "medium",
    });
  });

  it("describes runtimes without making implementation jargon primary", () => {
    expect(describePluginRuntime("native:privacyFilter")).toEqual({
      label: "内置隐私过滤引擎",
      detail: "由 AIO Coding Hub 提供，用于本地处理。",
    });
    expect(describePluginRuntime("declarativeRules")).toEqual({
      label: "规则插件",
      detail: "根据声明式规则处理请求、响应或日志。",
    });
  });

  it("maps risk levels to readable labels", () => {
    expect(pluginRiskLabel("low")).toBe("低风险");
    expect(pluginRiskLabel("medium")).toBe("中风险");
    expect(pluginRiskLabel("high")).toBe("高风险");
    expect(pluginRiskLabel("critical")).toBe("关键风险");
  });
});
