import { describe, expect, it } from "vitest";
import { validateProviderModelPolicy } from "../providerModelPolicy";

describe("providerModelPolicy", () => {
  it("accepts large policies and long Unicode model names", () => {
    expect(
      validateProviderModelPolicy({
        version: 1,
        mode: "selected",
        modelPatterns: ["模型".repeat(201), ...Array.from({ length: 500 }, (_, i) => `model-${i}`)],
        mappings: [],
      })
    ).toBeNull();
  });

  it("requires mapping targets and accepts a mapping as selected-model support", () => {
    expect(
      validateProviderModelPolicy({
        version: 1,
        mode: "selected",
        modelPatterns: [],
        mappings: [{ source: "gpt-5.6-luna", target: "deepseek-v4-flash" }],
      })
    ).toBeNull();
    expect(
      validateProviderModelPolicy({
        version: 1,
        mode: "all",
        modelPatterns: [],
        mappings: [{ source: "gpt-5.6-luna", target: "" }],
      })
    ).toBe("上游模型不能为空");
  });
});
