import { describe, expect, it } from "vitest";
import type { ProviderModelPolicyV1 } from "../../../services/providers/providers";
import { defaultProbeModel, probeModelCandidates } from "../providerProbeDefaults";

function policy(partial: Partial<ProviderModelPolicyV1>): ProviderModelPolicyV1 {
  return { version: 1, mode: "all", modelPatterns: [], mappings: [], ...partial };
}

describe("providerProbeDefaults", () => {
  it("lists configured concrete models and defaults to the first one", () => {
    const value = policy({
      mode: "selected",
      modelPatterns: ["deepseek-v4-flash", "grok-4.6"],
      mappings: [{ source: "gpt-5.4", target: "hy3-free" }],
    });

    expect(probeModelCandidates(value)).toEqual(["deepseek-v4-flash", "grok-4.6", "gpt-5.4"]);
    expect(defaultProbeModel(value)).toBe("deepseek-v4-flash");
  });

  it("drops wildcards and duplicates", () => {
    const value = policy({
      modelPatterns: ["gpt-*", " grok-4.6 ", "grok-4.6", ""],
      mappings: [{ source: "grok-4.6", target: "x" }],
    });

    expect(probeModelCandidates(value)).toEqual(["grok-4.6"]);
  });

  it("never probes an excluded blocklist and tolerates a missing policy", () => {
    const excluded = policy({ mode: "excluded", modelPatterns: ["gpt-4o-mini"] });

    expect(probeModelCandidates(excluded)).toEqual([]);
    expect(defaultProbeModel(excluded)).toBe("");
    expect(probeModelCandidates(null)).toEqual([]);
    expect(defaultProbeModel(null)).toBe("");
  });
});
