import type {
  CodexModelCapability,
  CodexModelCatalogState,
  CodexReasoningEffortOption,
} from "../../../services/cli/cliManager";

export const KNOWN_REASONING_EFFORTS = [
  "minimal",
  "low",
  "medium",
  "high",
  "xhigh",
  "max",
  "ultra",
] as const;

const FALLBACK_REASONING_EFFORTS: readonly CodexReasoningEffortOption[] = [
  { reasoning_effort: "minimal", description: null },
  { reasoning_effort: "low", description: null },
  { reasoning_effort: "medium", description: null },
  { reasoning_effort: "high", description: null },
  { reasoning_effort: "xhigh", description: null },
];

const KNOWN_EFFORT_RANK = new Map(
  KNOWN_REASONING_EFFORTS.map((effort, index) => [effort, index] as const)
);

const REASONING_EFFORT_RISK_DESCRIPTIONS: Readonly<Record<"max" | "ultra", string>> = {
  max: "最大单任务推理深度，可能增加延迟和用量。",
  ultra: "会自动委派子智能体并行处理任务，增加并发和额外用量。",
};

function enrichReasoningEffortDescription(
  option: CodexReasoningEffortOption
): CodexReasoningEffortOption {
  const effort = option.reasoning_effort;
  if (effort !== "max" && effort !== "ultra") return option;

  return {
    ...option,
    description: REASONING_EFFORT_RISK_DESCRIPTIONS[effort],
  };
}

export type CodexModelMatch = {
  model: CodexModelCapability | null;
  reason: "matched_model" | "matched_id" | "default_model" | "catalog_miss";
};

export type ReasoningOptionView = CodexReasoningEffortOption & {
  label: string;
  isCurrentUnknown?: boolean;
};

export type ReasoningOptionsResolution = {
  match: CodexModelMatch;
  options: ReasoningOptionView[];
  source: "catalog" | "fallback" | "empty";
  statusText: string | null;
};

export function isCatalogSnapshotCurrent(
  catalog: CodexModelCatalogState | null | undefined,
  config: { config_path: string } | null | undefined,
  info: { executable_path: string | null; version: string | null } | null | undefined
): boolean {
  return Boolean(
    catalog &&
    config &&
    info &&
    catalog.snapshot.config_path === config.config_path &&
    catalog.snapshot.executable_path === info.executable_path &&
    catalog.snapshot.cli_version === info.version
  );
}

export function matchCodexModel(
  catalog: CodexModelCatalogState | null | undefined,
  modelText: string | null | undefined
): CodexModelMatch {
  const models = catalog?.models ?? [];
  const model = modelText?.trim() ?? "";
  if (model) {
    const byModel = models.find((entry) => entry.model === model);
    if (byModel) return { model: byModel, reason: "matched_model" };
    const byId = models.find((entry) => entry.id === model);
    if (byId) return { model: byId, reason: "matched_id" };
    return { model: null, reason: "catalog_miss" };
  }

  const defaultModel = models.find((entry) => entry.is_default) ?? null;
  return defaultModel
    ? { model: defaultModel, reason: "default_model" }
    : { model: null, reason: "catalog_miss" };
}

export function reasoningOptionLabel(value: string): string {
  switch (value) {
    case "minimal":
      return "最低 (minimal)";
    case "low":
      return "低 (low)";
    case "medium":
      return "中 (medium)";
    case "high":
      return "高 (high)";
    case "xhigh":
      return "极高 (xhigh)";
    case "max":
      return "最大深度 (max)";
    case "ultra":
      return "自动委派 (ultra)";
    default:
      return value;
  }
}

function dedupeOptions(
  options: readonly CodexReasoningEffortOption[]
): CodexReasoningEffortOption[] {
  const seen = new Set<string>();
  return options.filter((option) => {
    const value = option.reasoning_effort.trim();
    if (!value || seen.has(value)) return false;
    seen.add(value);
    return true;
  });
}

function withCurrentUnknown(
  options: readonly CodexReasoningEffortOption[],
  currentEffort: string
): ReasoningOptionView[] {
  const normalized = currentEffort.trim();
  const resolved: ReasoningOptionView[] = [
    { reasoning_effort: "", description: null, label: "默认" },
    ...dedupeOptions(options).map((option) => ({
      ...enrichReasoningEffortDescription(option),
      label: reasoningOptionLabel(option.reasoning_effort),
    })),
  ];
  if (normalized && !resolved.some((option) => option.reasoning_effort === normalized)) {
    const currentOption = enrichReasoningEffortDescription({
      reasoning_effort: normalized,
      description: null,
    });
    resolved.push({
      ...currentOption,
      label: `${normalized}（当前配置，未声明）`,
      isCurrentUnknown: true,
    });
  }
  return resolved;
}

export function resolveReasoningOptions(
  catalog: CodexModelCatalogState | null | undefined,
  modelText: string | null | undefined,
  currentEffort: string
): ReasoningOptionsResolution {
  const match = matchCodexModel(catalog, modelText);
  const capability = match.model;
  if (catalog?.status === "ready" && capability) {
    if (capability.supported_reasoning_efforts === null) {
      return {
        match,
        options: withCurrentUnknown(FALLBACK_REASONING_EFFORTS, currentEffort),
        source: "fallback",
        statusText: "当前模型未提供推理能力字段，选项仅供编辑。",
      };
    }
    if (capability.supported_reasoning_efforts.length === 0) {
      return {
        match,
        options: withCurrentUnknown([], currentEffort),
        source: "empty",
        statusText: "当前模型未声明普通推理强度选项。",
      };
    }
    return {
      match,
      options: withCurrentUnknown(capability.supported_reasoning_efforts, currentEffort),
      source: "catalog",
      statusText: null,
    };
  }

  const statusText =
    catalog?.status === "ready"
      ? "当前模型未在能力目录中，推理强度未确认。"
      : catalog
        ? "未读取到模型能力，当前推理选项仅供编辑。"
        : "模型能力目录不可用，当前推理选项仅供编辑。";
  return {
    match,
    options: withCurrentUnknown(FALLBACK_REASONING_EFFORTS, currentEffort),
    source: "fallback",
    statusText,
  };
}

export function highestKnownReasoningEffort(
  options: readonly CodexReasoningEffortOption[]
): string | null {
  return (
    dedupeOptions(options)
      .map((option) => option.reasoning_effort)
      .filter((value): value is (typeof KNOWN_REASONING_EFFORTS)[number] =>
        KNOWN_EFFORT_RANK.has(value as (typeof KNOWN_REASONING_EFFORTS)[number])
      )
      .sort(
        (left, right) => (KNOWN_EFFORT_RANK.get(right) ?? -1) - (KNOWN_EFFORT_RANK.get(left) ?? -1)
      )[0] ?? null
  );
}

export function getModelMigrationEffort(
  target: CodexModelCapability | null,
  currentEffort: string
): string | null {
  const current = currentEffort.trim();
  if (current !== "max" && current !== "ultra") return null;
  const supported = target?.supported_reasoning_efforts;
  if (!supported || supported.some((option) => option.reasoning_effort === current)) return null;
  return highestKnownReasoningEffort(supported);
}

export function shouldReconcileModelEffort(
  target: CodexModelCapability | null,
  currentEffort: string,
  capabilityConfirmed: boolean
): boolean {
  const current = currentEffort.trim();
  if (current !== "max" && current !== "ultra") return false;
  if (!capabilityConfirmed || !target || target.supported_reasoning_efforts === null) return true;

  const supported = target.supported_reasoning_efforts;
  if (supported.length === 0 || supported.some((option) => option.reasoning_effort === current)) {
    return false;
  }
  return highestKnownReasoningEffort(supported) === null;
}

export function ultraConflictText(
  effort: string,
  featuresMultiAgent: boolean | null
): string | null {
  return effort.trim() === "ultra" && featuresMultiAgent === false
    ? "当前已关闭 multi_agent，ultra 可能无法按预期执行。"
    : null;
}
