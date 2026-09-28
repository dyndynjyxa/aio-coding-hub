import { describe, expect, it } from "vitest";
import { DEFAULT_FORM_VALUES } from "../providerEditorUtils";
import { buildProviderEditorUpsertInput } from "../providerEditorSubmitModel";
import type { ProviderEditorPayloadContext } from "../providerEditorActionContext";

function makeContext(
  overrides: Partial<ProviderEditorPayloadContext> = {}
): ProviderEditorPayloadContext {
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
    customHeaders: [],
    supportsWebsockets: false,
    apiKeyConfigured: false,
    isCodexGatewaySource: false,
    sourceProviderId: null,
    selectedCx2ccSourceProvider: null,
    modelPolicyStatus: "ready",
    modelPolicy: {
      version: 1,
      mode: "all",
      modelPatterns: [],
      mappings: [],
    },
    formValues: {
      ...DEFAULT_FORM_VALUES,
      name: "Provider A",
      api_key: "sk-test",
    },
    ...overrides,
  };
}

describe("pages/providers/providerEditorSubmitModel", () => {
  it.each([
    ["codex", "api_key", true],
    ["codex", "oauth", true],
    ["claude", "api_key", false],
    ["claude", "cx2cc", false],
  ] as const)(
    "only submits WS capability for native Codex: %s / %s",
    (cliKey, authMode, expected) => {
      const result = buildProviderEditorUpsertInput(
        makeContext({
          cliKey,
          authMode,
          supportsWebsockets: true,
          isCodexGatewaySource: true,
        })
      );
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.payload.supportsWebsockets).toBe(expected);
    }
  );

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

    expect(result.value.payload.baseUrls).toEqual([]);
    expect(result.value.payload.apiKey).toBeNull();
    expect(result.value.payload.authMode).toBe("oauth");
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

    expect(result.value.payload.costMultiplier).toBe(0);
    expect(result.value.payload.bridgeType).toBe("cx2cc");
    expect(result.value.payload.sourceProviderId).toBeNull();
    expect(result.value.payload.authMode).toBe("api_key");
  });

  it("preserves legacy ownership when editing without opting into the generic policy", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        mode: "edit",
        editingProviderId: 9,
        modelPolicyStatus: "legacy",
        modelPolicy: null,
      })
    );

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.payload.modelPolicy).toBeNull();
  });

  it("blocks saving an invalid policy until it is reset", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        modelPolicyStatus: "invalid",
        modelPolicy: null,
      })
    );

    expect(result).toEqual({
      ok: false,
      error: {
        kind: "message",
        message: "模型策略无效，请先重置并保存",
      },
    });
  });

  it("blocks an empty selected policy", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        modelPolicy: { version: 1, mode: "selected", modelPatterns: [], mappings: [] },
      })
    );

    expect(result).toEqual({
      ok: false,
      error: {
        kind: "message",
        message: "仅这些可用模式至少需要一个模型或映射",
      },
    });
  });
});

it("normalizes custom headers and rejects bridge ownership and invalid values", () => {
  const valid = buildProviderEditorUpsertInput(
    makeContext({ customHeaders: [{ name: "X-Tenant", value: " a " }] })
  );
  expect(valid.ok && valid.value.payload.customHeaders).toEqual([{ name: "x-tenant", value: "a" }]);
  for (const overrides of [
    { customHeaders: [{ name: "authorization", value: "secret" }] },
    { customHeaders: [{ name: "x-tenant", value: "secret\n" }] },
    { authMode: "cx2cc" as const, customHeaders: [{ name: "x-tenant", value: "secret" }] },
  ]) {
    const result = buildProviderEditorUpsertInput(makeContext(overrides));
    expect(result.ok).toBe(false);
    expect(JSON.stringify(result)).not.toContain("secret");
  }
});
