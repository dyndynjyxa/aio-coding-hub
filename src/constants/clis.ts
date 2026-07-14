// Usage: Shared CLI definitions and derived UI filter helpers.

export const CLI_KEYS = ["claude", "codex", "gemini", "grok"] as const;

export type CliKey = (typeof CLI_KEYS)[number];

export type CliCapability =
  | "gateway"
  | "provider"
  | "logs"
  | "usage"
  | "pricing"
  | "cliProxy"
  | "cliManager"
  | "mcp"
  | "skills"
  | "prompts"
  | "workspaces"
  | "wsl"
  | "managedUpdate"
  | "providerPluginTarget";

export type CliCapabilities = Record<CliCapability, boolean>;

export type CliItem = {
  key: CliKey;
  name: string;
  desc: string;
  capabilities: CliCapabilities;
};

const NO_CAPABILITIES: CliCapabilities = {
  gateway: false,
  provider: false,
  logs: false,
  usage: false,
  pricing: false,
  cliProxy: false,
  cliManager: false,
  mcp: false,
  skills: false,
  prompts: false,
  workspaces: false,
  wsl: false,
  managedUpdate: false,
  providerPluginTarget: false,
};

function capabilities(...enabled: CliCapability[]): CliCapabilities {
  const result = { ...NO_CAPABILITIES };
  for (const capability of enabled) result[capability] = true;
  return result;
}

const LEGACY_CLI_CAPABILITIES = capabilities(
  "gateway",
  "provider",
  "logs",
  "usage",
  "pricing",
  "cliProxy",
  "cliManager",
  "mcp",
  "skills",
  "prompts",
  "workspaces",
  "wsl",
  "managedUpdate",
  "providerPluginTarget"
);

const GROK_CAPABILITIES = capabilities(
  "gateway",
  "provider",
  "logs",
  "usage",
  "pricing",
  "cliProxy",
  "cliManager",
  "mcp",
  "skills",
  "prompts",
  "workspaces"
);

export const CLI_REGISTRY: readonly CliItem[] = [
  {
    key: "claude",
    name: "Claude",
    desc: "Claude CLI",
    capabilities: LEGACY_CLI_CAPABILITIES,
  },
  {
    key: "codex",
    name: "Codex",
    desc: "OpenAI Codex CLI",
    capabilities: LEGACY_CLI_CAPABILITIES,
  },
  {
    key: "gemini",
    name: "Gemini",
    desc: "Google Gemini CLI",
    capabilities: LEGACY_CLI_CAPABILITIES,
  },
  {
    key: "grok",
    name: "Grok",
    desc: "xAI Grok CLI",
    capabilities: GROK_CAPABILITIES,
  },
];

export const CLIS = CLI_REGISTRY;

export function clisWith(capability: CliCapability) {
  return CLI_REGISTRY.filter((cli) => cli.capabilities[capability]);
}

export function cliKeysWith(capability: CliCapability): CliKey[] {
  return clisWith(capability).map((cli) => cli.key);
}

export function createCliRecord<T>(factory: (cliKey: CliKey) => T): Record<CliKey, T> {
  return Object.fromEntries(CLI_KEYS.map((cliKey) => [cliKey, factory(cliKey)])) as Record<
    CliKey,
    T
  >;
}

export type CliFilterKey = "all" | CliKey;

export type CliFilterItem = {
  key: CliFilterKey;
  label: string;
};

export const CLI_FILTER_ITEMS: CliFilterItem[] = [
  { key: "all", label: "全部" },
  ...CLIS.map((cli) => ({ key: cli.key, label: cli.name })),
];

const CLI_SHORT_LABELS: Record<CliKey, string> = {
  claude: "Claude",
  codex: "Codex",
  gemini: "Gemini",
  grok: "Grok",
};

export const CLI_SHORT_ITEMS: Array<{ key: CliKey; label: string }> = CLIS.map((cli) => ({
  key: cli.key,
  label: CLI_SHORT_LABELS[cli.key],
}));

export const CLI_FILTER_SHORT_ITEMS: CliFilterItem[] = [
  { key: "all", label: "全部" },
  ...CLI_SHORT_ITEMS,
];

export function cliFilterItemsWith(capability: CliCapability): CliFilterItem[] {
  return [
    { key: "all", label: "全部" },
    ...clisWith(capability).map((cli) => ({ key: cli.key, label: cli.name })),
  ];
}

export function cliShortItemsWith(capability: CliCapability) {
  const allowed = new Set(cliKeysWith(capability));
  return CLI_SHORT_ITEMS.filter((item) => allowed.has(item.key));
}

export function isCliKey(value: unknown): value is CliKey {
  if (typeof value !== "string") return false;
  return CLIS.some((cli) => cli.key === value);
}

export function cliLongLabel(cliKey: string) {
  return CLIS.find((cli) => cli.key === cliKey)?.name ?? cliKey;
}

export function cliFromKeyOrDefault(cliKey: unknown) {
  if (typeof cliKey !== "string") return CLIS[0];
  return CLIS.find((cli) => cli.key === cliKey) ?? CLIS[0];
}

type LegacyEnabledFlagCliKey = Extract<CliKey, "claude" | "codex" | "gemini">;
type CliEnabledFlagKey = `enabled_${LegacyEnabledFlagCliKey}`;

export type CliEnabledFlags = Record<CliEnabledFlagKey, boolean>;

export function enabledFlagForCli<T extends CliEnabledFlags>(
  row: T,
  cliKey: LegacyEnabledFlagCliKey
) {
  const key = `enabled_${cliKey}` as CliEnabledFlagKey;
  return row[key];
}

export function cliShortLabel(cliKey: string) {
  if (isCliKey(cliKey)) {
    return CLI_SHORT_LABELS[cliKey];
  }
  return cliKey;
}

const CLI_BADGE_BASE =
  "bg-slate-100 dark:bg-slate-700 text-slate-600 dark:text-slate-400 border border-slate-200/90 dark:border-slate-600/70";

const CLI_BADGE_HOVER =
  "group-hover:bg-white dark:group-hover:bg-slate-800 group-hover:border-slate-200 dark:group-hover:border-slate-700";

export function cliBadgeTone(cliKey: string) {
  if (isCliKey(cliKey)) return `${CLI_BADGE_BASE} ${CLI_BADGE_HOVER}`;
  return CLI_BADGE_BASE;
}

export function cliBadgeToneStatic(_cliKey: string) {
  return CLI_BADGE_BASE;
}
