import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type {
  CodexConfigPatch,
  CodexConfigState,
  CodexModelCapability,
  CodexModelCatalogState,
  SimpleCliInfo,
} from "../../../../services/cli/cliManager";
import { useCodexModelMigration } from "../useCodexModelMigration";

type PersistCodexConfig = (patch: CodexConfigPatch) => Promise<CodexConfigState | null>;

const SNAPSHOT = {
  config_path: "/tmp/.codex/config.toml",
  executable_path: "/tmp/codex",
  cli_version: "1.0.0",
};

function makeInfo(): SimpleCliInfo {
  return {
    found: true,
    executable_path: SNAPSHOT.executable_path,
    version: SNAPSHOT.cli_version,
    error: null,
    shell: "/bin/zsh",
    resolved_via: "PATH",
  };
}

function makeConfig(overrides: Partial<CodexConfigState> = {}): CodexConfigState {
  return {
    config_dir: "/tmp/.codex",
    config_path: SNAPSHOT.config_path,
    user_home_default_dir: "/tmp/.codex",
    user_home_default_path: SNAPSHOT.config_path,
    follow_codex_home_dir: "/tmp/.codex",
    follow_codex_home_path: SNAPSHOT.config_path,
    can_open_config_dir: true,
    exists: true,
    model: "gpt-5.6-sol",
    approval_policy: "on-request",
    sandbox_mode: "workspace-write",
    model_reasoning_effort: "ultra",
    plan_mode_reasoning_effort: null,
    web_search: "cached",
    personality: null,
    model_context_window: 1_000_000,
    model_auto_compact_token_limit: 900_000,
    service_tier: null,
    sandbox_workspace_write_network_access: null,
    features_unified_exec: false,
    features_shell_snapshot: false,
    features_apply_patch_freeform: false,
    features_shell_tool: false,
    features_exec_policy: false,
    features_remote_compaction: false,
    features_fast_mode: false,
    features_responses_websockets_v2: false,
    features_multi_agent: null,
    ...overrides,
  };
}

function makeModel(overrides: Partial<CodexModelCapability> = {}): CodexModelCapability {
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
      { reasoning_effort: "max", description: null },
      { reasoning_effort: "ultra", description: null },
    ],
    default_reasoning_effort: "medium",
    ...overrides,
  };
}

function makeCatalog(
  status: CodexModelCatalogState["status"],
  models: CodexModelCapability[]
): CodexModelCatalogState {
  return {
    status,
    issue: status === "ready" ? null : "app_server_unavailable",
    snapshot: SNAPSHOT,
    models,
  };
}

function renderMigration(
  persistCodexConfig: PersistCodexConfig,
  props: {
    codexConfig?: CodexConfigState;
    codexModelCatalog: CodexModelCatalogState;
  } = { codexModelCatalog: makeCatalog("ready", [makeModel()]) }
) {
  return renderHook(
    ({ codexConfig, codexModelCatalog }) =>
      useCodexModelMigration({
        codexConfig: codexConfig ?? makeConfig(),
        codexInfo: makeInfo(),
        codexModelCatalog,
        persistCodexConfig,
      }),
    { initialProps: { codexConfig: props.codexConfig ?? makeConfig(), ...props } }
  );
}

describe("useCodexModelMigration", () => {
  it("does not persist a model value that only changes surrounding whitespace", async () => {
    const persistCodexConfig = vi.fn<PersistCodexConfig>();
    const { result } = renderMigration(persistCodexConfig);

    await act(async () => {
      await expect(result.current.persistModel("  gpt-5.6-sol  ", "ultra")).resolves.toBeNull();
    });

    expect(persistCodexConfig).not.toHaveBeenCalled();
  });

  it("clears token overrides and downgrades a confirmed incompatible effort atomically", async () => {
    const updated = makeConfig({ model: "gpt-5.6-luna", model_reasoning_effort: "max" });
    const persistCodexConfig = vi.fn<PersistCodexConfig>().mockResolvedValue(updated);
    const catalog = makeCatalog("ready", [
      makeModel(),
      makeModel({
        id: "gpt-5.6-luna-id",
        model: "gpt-5.6-luna",
        display_name: "GPT-5.6 Luna",
        supported_reasoning_efforts: [
          { reasoning_effort: "low", description: null },
          { reasoning_effort: "max", description: null },
        ],
      }),
    ]);
    const { result } = renderMigration(persistCodexConfig, { codexModelCatalog: catalog });

    await act(async () => {
      await result.current.persistModel("gpt-5.6-luna", "ultra");
    });

    expect(persistCodexConfig).toHaveBeenCalledWith({
      model: "gpt-5.6-luna",
      model_context_window: null,
      model_auto_compact_token_limit: null,
      model_reasoning_effort: "max",
    });
    expect(result.current.statusText).toContain("max");
  });

  it("does not start reconciliation after a failed model save", async () => {
    const persistCodexConfig = vi.fn<PersistCodexConfig>().mockResolvedValue(null);
    const { result } = renderMigration(persistCodexConfig, {
      codexModelCatalog: makeCatalog("degraded", []),
    });

    await act(async () => {
      await result.current.persistModel("custom-model", "ultra");
    });

    expect(persistCodexConfig).toHaveBeenCalledTimes(1);
    expect(result.current.hasPendingReconciliation).toBe(false);
    expect(result.current.statusText).toBe("模型保存失败，未清除覆盖或调整推理强度。");
  });

  it("reconciles once after the catalog recovers, even while query data is catching up", async () => {
    const savedBeforeSwitch = makeConfig();
    const savedAfterSwitch = makeConfig({ model: "gpt-5.6-luna" });
    const savedAfterReconciliation = makeConfig({
      model: "gpt-5.6-luna",
      model_reasoning_effort: "max",
      model_context_window: null,
      model_auto_compact_token_limit: null,
    });
    const persistCodexConfig = vi
      .fn<PersistCodexConfig>()
      .mockResolvedValueOnce(savedAfterSwitch)
      .mockResolvedValueOnce(savedAfterReconciliation);
    const degradedCatalog = makeCatalog("degraded", []);
    const readyCatalog = makeCatalog("ready", [
      makeModel(),
      makeModel({
        id: "gpt-5.6-luna-id",
        model: "gpt-5.6-luna",
        display_name: "GPT-5.6 Luna",
        supported_reasoning_efforts: [
          { reasoning_effort: "low", description: null },
          { reasoning_effort: "max", description: null },
        ],
      }),
    ]);
    const { result, rerender } = renderHook(
      ({ codexConfig, codexModelCatalog }) =>
        useCodexModelMigration({
          codexConfig,
          codexInfo: makeInfo(),
          codexModelCatalog,
          persistCodexConfig,
        }),
      { initialProps: { codexConfig: savedBeforeSwitch, codexModelCatalog: degradedCatalog } }
    );

    await act(async () => {
      await result.current.persistModel("gpt-5.6-luna", "ultra");
    });
    expect(result.current.hasPendingReconciliation).toBe(true);

    rerender({ codexConfig: savedAfterSwitch, codexModelCatalog: readyCatalog });
    await waitFor(() => expect(persistCodexConfig).toHaveBeenCalledTimes(2));
    expect(persistCodexConfig).toHaveBeenLastCalledWith({ model_reasoning_effort: "max" });
    await waitFor(() => expect(result.current.hasPendingReconciliation).toBe(false));

    rerender({ codexConfig: savedAfterReconciliation, codexModelCatalog: readyCatalog });
    expect(persistCodexConfig).toHaveBeenCalledTimes(2);
  });

  it("clears pending reconciliation when the user changes effort before recovery", async () => {
    const savedAfterSwitch = makeConfig({ model: "gpt-5.6-luna" });
    const persistCodexConfig = vi.fn<PersistCodexConfig>().mockResolvedValue(savedAfterSwitch);
    const { result, rerender } = renderHook(
      ({ codexConfig, codexModelCatalog }) =>
        useCodexModelMigration({
          codexConfig,
          codexInfo: makeInfo(),
          codexModelCatalog,
          persistCodexConfig,
        }),
      {
        initialProps: {
          codexConfig: makeConfig(),
          codexModelCatalog: makeCatalog("degraded", []),
        },
      }
    );

    await act(async () => {
      await result.current.persistModel("gpt-5.6-luna", "ultra");
      result.current.onEffortInputChange();
    });
    expect(result.current.hasPendingReconciliation).toBe(false);

    rerender({
      codexConfig: savedAfterSwitch,
      codexModelCatalog: makeCatalog("ready", [
        makeModel({
          id: "gpt-5.6-luna-id",
          model: "gpt-5.6-luna",
          supported_reasoning_efforts: [
            { reasoning_effort: "low", description: null },
            { reasoning_effort: "max", description: null },
          ],
        }),
      ]),
    });

    await waitFor(() => expect(persistCodexConfig).toHaveBeenCalledTimes(1));
  });

  it("ends the one-time check when the recovered catalog has no known downgrade", async () => {
    const savedAfterSwitch = makeConfig({ model: "custom-model" });
    const persistCodexConfig = vi.fn<PersistCodexConfig>().mockResolvedValue(savedAfterSwitch);
    const { result, rerender } = renderHook(
      ({ codexConfig, codexModelCatalog }) =>
        useCodexModelMigration({
          codexConfig,
          codexInfo: makeInfo(),
          codexModelCatalog,
          persistCodexConfig,
        }),
      {
        initialProps: {
          codexConfig: makeConfig(),
          codexModelCatalog: makeCatalog("degraded", []),
        },
      }
    );

    await act(async () => {
      await result.current.persistModel("custom-model", "ultra");
    });
    expect(result.current.hasPendingReconciliation).toBe(true);

    rerender({
      codexConfig: savedAfterSwitch,
      codexModelCatalog: makeCatalog("ready", [
        makeModel({
          id: "custom-model-id",
          model: "custom-model",
          supported_reasoning_efforts: [{ reasoning_effort: "provider-future", description: null }],
        }),
      ]),
    });

    await waitFor(() => expect(result.current.hasPendingReconciliation).toBe(false));
    expect(result.current.statusText).toContain("未提供可确认的降级档位");
    expect(persistCodexConfig).toHaveBeenCalledTimes(1);
  });

  it("reports an explicit empty effort catalog without retrying reconciliation", async () => {
    const savedAfterSwitch = makeConfig({ model: "custom-model" });
    const persistCodexConfig = vi.fn<PersistCodexConfig>().mockResolvedValue(savedAfterSwitch);
    const { result, rerender } = renderHook(
      ({ codexConfig, codexModelCatalog }) =>
        useCodexModelMigration({
          codexConfig,
          codexInfo: makeInfo(),
          codexModelCatalog,
          persistCodexConfig,
        }),
      {
        initialProps: {
          codexConfig: makeConfig(),
          codexModelCatalog: makeCatalog("degraded", []),
        },
      }
    );

    await act(async () => {
      await result.current.persistModel("custom-model", "ultra");
    });
    expect(result.current.hasPendingReconciliation).toBe(true);

    const readyCatalog = makeCatalog("ready", [
      makeModel({
        id: "custom-model-id",
        model: "custom-model",
        supported_reasoning_efforts: [],
      }),
    ]);
    rerender({ codexConfig: savedAfterSwitch, codexModelCatalog: readyCatalog });

    await waitFor(() => expect(result.current.hasPendingReconciliation).toBe(false));
    expect(result.current.statusText).toBe(
      "能力目录已恢复，但未提供可确认的降级档位，已保留当前推理强度。"
    );
    expect(persistCodexConfig).toHaveBeenCalledTimes(1);

    rerender({ codexConfig: savedAfterSwitch, codexModelCatalog: readyCatalog });
    expect(persistCodexConfig).toHaveBeenCalledTimes(1);
  });

  it("ignores a late model save after the user invalidates the operation", async () => {
    let resolveSave!: (value: CodexConfigState | null) => void;
    const savePromise = new Promise<CodexConfigState | null>((resolve) => {
      resolveSave = resolve;
    });
    const persistCodexConfig = vi.fn<PersistCodexConfig>().mockReturnValue(savePromise);
    const { result } = renderMigration(persistCodexConfig, {
      codexModelCatalog: makeCatalog("degraded", []),
    });

    let migrationPromise!: Promise<CodexConfigState | null>;
    await act(async () => {
      migrationPromise = result.current.persistModel("custom-model", "ultra");
      result.current.onModelInputChange();
      resolveSave(makeConfig({ model: "custom-model" }));
      await migrationPromise;
    });

    expect(result.current.hasPendingReconciliation).toBe(false);
    expect(result.current.statusText).toBeNull();
  });
});
