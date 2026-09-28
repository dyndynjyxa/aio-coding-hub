import { useCallback, useEffect, useRef, useState } from "react";
import type {
  CodexConfigPatch,
  CodexConfigState,
  CodexModelCatalogState,
  SimpleCliInfo,
} from "../../../services/cli/cliManager";
import {
  getModelMigrationEffort,
  isCatalogSnapshotCurrent,
  matchCodexModel,
  shouldReconcileModelEffort,
} from "./codexModelCapabilities";

type PersistCodexConfig = (patch: CodexConfigPatch) => Promise<CodexConfigState | null>;

type PendingReconciliation = {
  sourceModel: string;
  targetModel: string;
  sourceEffort: string;
};

export function useCodexModelMigration({
  codexConfig,
  codexInfo,
  codexModelCatalog,
  persistCodexConfig,
}: {
  codexConfig: CodexConfigState | null;
  codexInfo: SimpleCliInfo | null;
  codexModelCatalog: CodexModelCatalogState | null;
  persistCodexConfig: PersistCodexConfig;
}) {
  const [pending, setPending] = useState<PendingReconciliation | null>(null);
  const [statusText, setStatusText] = useState<string | null>(null);
  const operationRef = useRef(0);
  const reconciliationKeyRef = useRef<string | null>(null);
  const persistRef = useRef(persistCodexConfig);
  persistRef.current = persistCodexConfig;

  const savedModel = codexConfig?.model?.trim() ?? "";
  const savedEffort = codexConfig?.model_reasoning_effort?.trim() ?? "";
  const catalogIsCurrent = isCatalogSnapshotCurrent(codexModelCatalog, codexConfig, codexInfo);
  const catalog = catalogIsCurrent ? codexModelCatalog : null;

  const cancelPending = useCallback(() => {
    operationRef.current += 1;
    reconciliationKeyRef.current = null;
    setPending(null);
    setStatusText(null);
  }, []);

  const persistModel = useCallback(
    async (modelText: string, currentEffort: string): Promise<CodexConfigState | null> => {
      const targetModel = modelText.trim();
      if (targetModel === savedModel) return null;

      const operation = ++operationRef.current;
      reconciliationKeyRef.current = null;
      setPending(null);
      setStatusText(null);

      const match = matchCodexModel(catalog, targetModel);
      const downgrade =
        catalog?.status === "ready" ? getModelMigrationEffort(match.model, currentEffort) : null;
      const patch: CodexConfigPatch = {
        model: targetModel,
        model_context_window: null,
        model_auto_compact_token_limit: null,
        ...(downgrade ? { model_reasoning_effort: downgrade } : {}),
      };
      const updated = await persistRef.current(patch);
      if (operation !== operationRef.current) return updated;

      if (!updated) {
        setStatusText("模型保存失败，未清除覆盖或调整推理强度。");
        return null;
      }

      const confirmedModel = updated.model?.trim() ?? "";
      const confirmedEffort = updated.model_reasoning_effort?.trim() ?? "";
      const needsReconciliation = shouldReconcileModelEffort(
        match.model,
        confirmedEffort,
        catalog?.status === "ready" && match.reason !== "catalog_miss"
      );
      if (needsReconciliation) {
        setPending({
          sourceModel: savedModel,
          targetModel: confirmedModel,
          sourceEffort: confirmedEffort,
        });
        setStatusText("模型已保存，能力目录恢复后会再次检查推理强度。");
      } else if (downgrade) {
        setStatusText(`当前推理强度已调整为 ${downgrade}：目标模型不支持原设置。`);
      } else {
        setStatusText("已切换模型，已清除上下文覆盖。");
      }
      return updated;
    },
    [catalog, savedModel]
  );

  useEffect(() => {
    if (!pending || !catalog || catalog.status !== "ready") return;
    const modelMatchesPending =
      savedModel === pending.sourceModel || savedModel === pending.targetModel;
    if (!modelMatchesPending || savedEffort !== pending.sourceEffort) {
      operationRef.current += 1;
      reconciliationKeyRef.current = null;
      setPending(null);
      setStatusText(null);
      return;
    }

    const match = matchCodexModel(catalog, pending.targetModel);
    const capabilityConfirmed = catalog.status === "ready" && match.reason !== "catalog_miss";
    const key = `${pending.targetModel}\u0000${pending.sourceEffort}`;
    if (reconciliationKeyRef.current === key) return;
    reconciliationKeyRef.current = key;

    if (match.model?.supported_reasoning_efforts?.length === 0) {
      setPending(null);
      setStatusText("能力目录已恢复，但未提供可确认的降级档位，已保留当前推理强度。");
      return;
    }

    if (shouldReconcileModelEffort(match.model, pending.sourceEffort, capabilityConfirmed)) {
      setPending(null);
      setStatusText(
        match.model
          ? "能力目录已恢复，但未提供可确认的降级档位，已保留当前推理强度。"
          : "能力目录已恢复，但当前模型未命中，已保留当前推理强度。"
      );
      return;
    }
    const downgrade = getModelMigrationEffort(match.model, pending.sourceEffort);

    if (!downgrade) {
      setPending(null);
      setStatusText(
        match.model ? "能力目录已确认，当前推理强度无需调整。" : "当前模型仍未在能力目录中。"
      );
      return;
    }

    const operation = ++operationRef.current;
    void persistRef.current({ model_reasoning_effort: downgrade }).then((updated) => {
      if (operation !== operationRef.current) return;
      setPending(null);
      setStatusText(
        updated
          ? `能力目录恢复后，推理强度已调整为 ${downgrade}。`
          : "推理强度自动调整失败，已保留当前配置。"
      );
    });
  }, [catalog, pending, savedEffort, savedModel]);

  return {
    catalog,
    statusText,
    hasPendingReconciliation: pending != null,
    onModelInputChange: cancelPending,
    onEffortInputChange: cancelPending,
    persistModel,
  };
}
