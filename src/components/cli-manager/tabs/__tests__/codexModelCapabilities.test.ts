import { describe, expect, it } from "vitest";
import {
  getModelMigrationEffort,
  highestKnownReasoningEffort,
  matchCodexModel,
  resolveReasoningOptions,
  shouldReconcileModelEffort,
  ultraConflictText,
} from "../codexModelCapabilities";
import type { CodexModelCatalogState } from "../../../../services/cli/cliManager";

function catalog(models: CodexModelCatalogState["models"]): CodexModelCatalogState {
  return {
    status: "ready",
    issue: null,
    snapshot: {
      config_path: "/tmp/.codex/config.toml",
      executable_path: "/tmp/codex",
      cli_version: "0.144.1",
    },
    models,
  };
}

function model(overrides: Partial<CodexModelCatalogState["models"][number]>) {
  return {
    id: "gpt-5.6-sol-id",
    model: "gpt-5.6-sol",
    display_name: "GPT-5.6 Sol",
    hidden: false,
    is_default: false,
    supported_reasoning_efforts: [
      { reasoning_effort: "low", description: null },
      { reasoning_effort: "medium", description: null },
      { reasoning_effort: "high", description: null },
      { reasoning_effort: "xhigh", description: null },
      { reasoning_effort: "max", description: "deep" },
      { reasoning_effort: "ultra", description: "delegation" },
    ],
    default_reasoning_effort: "medium",
    ...overrides,
  };
}

describe("codexModelCapabilities", () => {
  it("matches model before id and uses the default only for an empty input", () => {
    const entries = [
      model({ id: "gpt-5.6-sol-id", model: "gpt-5.6-sol", is_default: true }),
      model({ id: "gpt-5.6-terra-id", model: "other" }),
    ];
    expect(matchCodexModel(catalog(entries), "gpt-5.6-sol").reason).toBe("matched_model");
    expect(matchCodexModel(catalog(entries), "gpt-5.6-terra-id").reason).toBe("matched_id");
    expect(matchCodexModel(catalog(entries), "").reason).toBe("default_model");
  });

  it("replaces max and ultra descriptions with stable risk guidance", () => {
    const result = resolveReasoningOptions(catalog([model({})]), "gpt-5.6-sol", "medium");
    expect(result.source).toBe("catalog");
    expect(result.options.map((option) => option.reasoning_effort)).toEqual([
      "",
      "low",
      "medium",
      "high",
      "xhigh",
      "max",
      "ultra",
    ]);
    expect(result.options.find((option) => option.reasoning_effort === "max")?.description).toBe(
      "最大单任务推理深度，可能增加延迟和用量。"
    );
    expect(result.options.find((option) => option.reasoning_effort === "ultra")?.description).toBe(
      "会自动委派子智能体并行处理任务，增加并发和额外用量。"
    );
  });

  it("does not add ultra to a model that only declares max", () => {
    const result = resolveReasoningOptions(
      catalog([
        model({
          model: "gpt-5.6-luna",
          supported_reasoning_efforts: [
            { reasoning_effort: "low", description: null },
            { reasoning_effort: "max", description: null },
          ],
        }),
      ]),
      "gpt-5.6-luna",
      "max"
    );
    expect(result.options.map((option) => option.reasoning_effort)).toEqual(["", "low", "max"]);
    expect(result.options.find((option) => option.reasoning_effort === "max")?.description).toBe(
      "最大单任务推理深度，可能增加延迟和用量。"
    );
  });

  it("keeps missing current values and uses fallback without inventing max or ultra", () => {
    const result = resolveReasoningOptions(null, "custom-model", "future-reasoning");
    expect(result.source).toBe("fallback");
    expect(result.options.map((option) => option.reasoning_effort)).toEqual([
      "",
      "minimal",
      "low",
      "medium",
      "high",
      "xhigh",
      "future-reasoning",
    ]);
    expect(result.options[result.options.length - 1]?.isCurrentUnknown).toBe(true);
  });

  it.each([
    {
      caseName: "degraded catalog",
      modelCatalog: { ...catalog([]), status: "degraded" as const, issue: "timeout" as const },
      modelText: "custom-model",
      currentEffort: "max",
      expectedDescription: "最大单任务推理深度，可能增加延迟和用量。",
    },
    {
      caseName: "unavailable catalog",
      modelCatalog: {
        ...catalog([]),
        status: "unavailable" as const,
        issue: "cli_not_found" as const,
      },
      modelText: "custom-model",
      currentEffort: "ultra",
      expectedDescription: "会自动委派子智能体并行处理任务，增加并发和额外用量。",
    },
    {
      caseName: "catalog miss",
      modelCatalog: catalog([model({})]),
      modelText: "missing-model",
      currentEffort: "ultra",
      expectedDescription: "会自动委派子智能体并行处理任务，增加并发和额外用量。",
    },
    {
      caseName: "empty supported efforts",
      modelCatalog: catalog([model({ supported_reasoning_efforts: [] })]),
      modelText: "gpt-5.6-sol",
      currentEffort: "max",
      expectedDescription: "最大单任务推理深度，可能增加延迟和用量。",
    },
    {
      caseName: "missing supported efforts",
      modelCatalog: catalog([model({ supported_reasoning_efforts: null })]),
      modelText: "gpt-5.6-sol",
      currentEffort: "ultra",
      expectedDescription: "会自动委派子智能体并行处理任务，增加并发和额外用量。",
    },
  ])("enriches an undeclared current effort for $caseName", (testCase) => {
    const result = resolveReasoningOptions(
      testCase.modelCatalog,
      testCase.modelText,
      testCase.currentEffort
    );
    const currentOption = result.options.find(
      (option) => option.reasoning_effort === testCase.currentEffort
    );

    expect(currentOption?.isCurrentUnknown).toBe(true);
    expect(currentOption?.description).toBe(testCase.expectedDescription);
  });

  it("distinguishes missing and empty supported effort lists", () => {
    const missing = resolveReasoningOptions(
      catalog([model({ supported_reasoning_efforts: null })]),
      "gpt-5.6-sol",
      "max"
    );
    const empty = resolveReasoningOptions(
      catalog([model({ supported_reasoning_efforts: [] })]),
      "gpt-5.6-sol",
      "max"
    );
    expect(missing.source).toBe("fallback");
    expect(empty.source).toBe("empty");
    expect(empty.options.map((option) => option.reasoning_effort)).toEqual(["", "max"]);
  });

  it("downgrades only confirmed max and ultra values using semantic rank", () => {
    const target = model({
      supported_reasoning_efforts: [
        { reasoning_effort: "high", description: null },
        { reasoning_effort: "xhigh", description: null },
        { reasoning_effort: "max", description: null },
      ],
    });
    expect(getModelMigrationEffort(target, "ultra")).toBe("max");
    expect(getModelMigrationEffort(target, "max")).toBeNull();
    expect(getModelMigrationEffort(target, "future-reasoning")).toBeNull();
    expect(highestKnownReasoningEffort(target.supported_reasoning_efforts ?? [])).toBe("max");
    expect(
      getModelMigrationEffort(
        model({
          supported_reasoning_efforts: [
            { reasoning_effort: "provider-special", description: null },
          ],
        }),
        "ultra"
      )
    ).toBeNull();
  });

  it("keeps a one-time reconciliation pending when only unknown efforts are declared", () => {
    const target = model({
      supported_reasoning_efforts: [{ reasoning_effort: "provider-future", description: null }],
    });
    expect(shouldReconcileModelEffort(target, "ultra", true)).toBe(true);
    expect(shouldReconcileModelEffort(target, "max", true)).toBe(true);
    expect(shouldReconcileModelEffort(target, "medium", true)).toBe(false);
    expect(
      shouldReconcileModelEffort(model({ supported_reasoning_efforts: [] }), "max", true)
    ).toBe(false);
    expect(shouldReconcileModelEffort(null, "ultra", false)).toBe(true);
  });

  it("only treats explicit false multi_agent as an ultra conflict", () => {
    expect(ultraConflictText("ultra", false)).toContain("multi_agent");
    expect(ultraConflictText("ultra", null)).toBeNull();
    expect(ultraConflictText("max", false)).toBeNull();
  });
});
