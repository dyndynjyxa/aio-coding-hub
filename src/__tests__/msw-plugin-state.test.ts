import { describe, expect, it } from "vitest";
import { installOfficialPluginState } from "../test/msw/state";

describe("MSW plugin state", () => {
  it("models official Privacy Filter as schema-driven config", () => {
    const result = installOfficialPluginState("official.privacy-filter");

    expect(result.manifest.configSchema).toMatchObject({
      type: "object",
      properties: {
        redactBeforeUpstream: {
          type: "boolean",
          title: "发送给模型前处理",
          "x-aio-ui": { widget: "switch" },
        },
        redactLogs: {
          type: "boolean",
          title: "保存日志前处理",
          "x-aio-ui": { widget: "switch" },
        },
        sensitiveTypes: {
          type: "array",
          title: "策略大类",
          description: expect.stringContaining("200+ Gitleaks"),
          "x-aio-ui": { widget: "checkboxGroup" },
        },
      },
    });
    expect(result.config).toMatchObject({
      redactBeforeUpstream: true,
      redactLogs: true,
      profile: "balanced",
    });
    expect(result.config).toHaveProperty("sensitiveTypes");
  });
});
