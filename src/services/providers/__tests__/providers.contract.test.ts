import { describe, expect, it } from "vitest";
import providersSource from "../providers.ts?raw";

describe("services/providers/providers contract", () => {
  it("derives provider ipc types from generated bindings instead of handwritten mirrors", () => {
    expect(providersSource).toContain("type ClaudeModels as GeneratedClaudeModels");
    expect(providersSource).toContain("type DailyResetMode as GeneratedDailyResetMode");
    expect(providersSource).toContain("type ProviderAuthMode as GeneratedProviderAuthMode");
    expect(providersSource).toContain(
      "type ProviderBaseUrlMode as GeneratedProviderBaseUrlMode"
    );
    expect(providersSource).toContain("type ProviderSummary as GeneratedProviderSummary");
    expect(providersSource).toContain("type ProviderUpsertInput as GeneratedProviderUpsertInput");
    expect(providersSource).toContain("type RemapGeneratedKeys");
    expect(providersSource).toContain("const providerUpsertFieldMap = {");
    expect(providersSource).not.toContain("export type ProviderSummary = {");
    expect(providersSource).not.toContain("export type ProviderUpsertInput = {");
  });
});
