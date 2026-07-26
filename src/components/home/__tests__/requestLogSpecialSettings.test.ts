import { describe, expect, it } from "vitest";
import {
  formatClaudeModelMappingText,
  hasClaudeModelMappingSpecialSetting,
  hasPriorityServiceTierSpecialSetting,
  resolveClaudeModelMappingFromSpecialSettings,
} from "../requestLogSpecialSettings";

describe("components/home/requestLogSpecialSettings", () => {
  it("resolves Claude model mapping special settings with final provider preference", () => {
    const settings = JSON.stringify([
      { type: "noop" },
      {
        type: "claude_model_mapping",
        requestedModel: " claude-sonnet ",
        effectiveModel: " gpt-5.4 ",
        mappingKind: " sonnet ",
        providerId: 1,
        providerName: " Provider A ",
        applied: true,
      },
      {
        type: "claude_model_mapping",
        requestedModel: " claude-opus ",
        effectiveModel: " gpt-5.5 ",
        mappingKind: " opus ",
        providerId: 2,
        providerName: " Provider B ",
        applied: true,
      },
    ]);

    expect(resolveClaudeModelMappingFromSpecialSettings(settings, 1)).toEqual({
      requestedModel: "claude-sonnet",
      effectiveModel: "gpt-5.4",
      mappingKind: "sonnet",
      providerId: 1,
      providerName: "Provider A",
      applied: true,
    });
    expect(resolveClaudeModelMappingFromSpecialSettings(settings, 99)?.providerId).toBe(2);
    expect(resolveClaudeModelMappingFromSpecialSettings("not-json")).toBeNull();
    expect(
      resolveClaudeModelMappingFromSpecialSettings(JSON.stringify({ type: "noop" }))
    ).toBeNull();
    expect(
      resolveClaudeModelMappingFromSpecialSettings(
        JSON.stringify([
          {
            type: "claude_model_mapping",
            requestedModel: "same",
            effectiveModel: "same",
            mappingKind: "sonnet",
            providerId: 1,
            providerName: "Provider A",
            applied: true,
          },
        ])
      )
    ).toBeNull();

    expect(hasClaudeModelMappingSpecialSetting(settings)).toBe(true);
    expect(hasClaudeModelMappingSpecialSetting(JSON.stringify([{ type: "noop" }]))).toBe(false);
    expect(hasClaudeModelMappingSpecialSetting("bad-json")).toBe(false);
  });

  it("formats model mapping text and priority service tier settings", () => {
    expect(
      formatClaudeModelMappingText(" fallback-model ", {
        requestedModel: " claude-sonnet ",
        effectiveModel: " gpt-5.4 ",
        mappingKind: "sonnet",
        providerId: 1,
        providerName: "Provider A",
        applied: true,
      })
    ).toBe("claude-sonnet → gpt-5.4");
    expect(formatClaudeModelMappingText(" fallback-model ", null)).toBe("fallback-model");
    expect(formatClaudeModelMappingText("   ", null)).toBe("未知");

    expect(hasPriorityServiceTierSpecialSetting(null)).toBe(false);
    expect(hasPriorityServiceTierSpecialSetting("bad-json")).toBe(false);
    expect(
      hasPriorityServiceTierSpecialSetting(JSON.stringify({ type: "codex_service_tier_result" }))
    ).toBe(false);
    expect(hasPriorityServiceTierSpecialSetting(JSON.stringify([{ type: "noop" }]))).toBe(false);
    expect(
      hasPriorityServiceTierSpecialSetting(
        JSON.stringify([{ type: "codex_service_tier_result", actualServiceTier: "priority" }])
      )
    ).toBe(true);
    expect(
      hasPriorityServiceTierSpecialSetting(
        JSON.stringify([
          {
            type: "codex_service_tier_result",
            billingSourcePreference: "auto",
            effectivePriority: true,
          },
        ])
      )
    ).toBe(true);
    expect(
      hasPriorityServiceTierSpecialSetting(
        JSON.stringify([{ type: "codex_service_tier_result", effectivePriority: false }])
      )
    ).toBe(false);
  });
});
