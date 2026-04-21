import { describe, expect, it } from "vitest";
import { DEFAULT_FORM_VALUES } from "../providerEditorUtils";
import { buildProviderEditorUpsertInput } from "../providerEditorSubmitModel";
import type { ProviderEditorPayloadContext } from "../providerEditorActionContext";

function makeContext(overrides: Partial<ProviderEditorPayloadContext> = {}): ProviderEditorPayloadContext {
  return {
    mode: "create",
    cliKey: "claude",
    editingProviderId: null,
    authMode: "api_key",
    baseUrlMode: "order",
    baseUrlRows: [{ id: "1", url: "https://example.com/v1", ping: { status: "idle" } }],
    tags: [],
    claudeModels: {},
    streamIdleTimeoutSeconds: "",
    apiKeyConfigured: false,
    isCodexGatewaySource: false,
    sourceProviderId: null,
    selectedCx2ccSourceProvider: null,
    formValues: {
      ...DEFAULT_FORM_VALUES,
      name: "Provider A",
      api_key: "sk-test",
    },
    ...overrides,
  };
}

describe("pages/providers/providerEditorSubmitModel", () => {
  it("requires an api key when editing an api-key provider without a saved secret", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        mode: "edit",
        editingProviderId: 8,
        apiKeyConfigured: false,
        formValues: {
          ...DEFAULT_FORM_VALUES,
          name: "Provider A",
          api_key: "",
        },
      })
    );

    expect(result).toEqual({
      ok: false,
      error: {
        kind: "message",
        message: "请输入 API Key",
      },
    });
  });

  it("clears base urls and api key for oauth providers", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        authMode: "oauth",
        formValues: {
          ...DEFAULT_FORM_VALUES,
          name: "OAuth Provider",
          api_key: "",
          auth_mode: "oauth",
        },
      })
    );

    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.payload.base_urls).toEqual([]);
    expect(result.value.payload.api_key).toBeNull();
    expect(result.value.payload.auth_mode).toBe("oauth");
  });

  it("forces cx2cc gateway sources to use zero cost and no source provider id", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        authMode: "cx2cc",
        isCodexGatewaySource: true,
        formValues: {
          ...DEFAULT_FORM_VALUES,
          name: "CX2CC Provider",
          api_key: "",
        },
      })
    );

    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.payload.cost_multiplier).toBe(0);
    expect(result.value.payload.bridge_type).toBe("cx2cc");
    expect(result.value.payload.source_provider_id).toBeNull();
    expect(result.value.payload.auth_mode).toBe("api_key");
  });
});
