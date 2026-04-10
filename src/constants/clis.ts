// Usage: Shared CLI definitions and derived UI filter helpers.

import type { CliKey } from "../services/providers/providers";

export type CliItem = {
  key: CliKey;
  name: string;
  desc: string;
};

export const CLIS: CliItem[] = [
  { key: "claude", name: "Claude Code", desc: "Claude CLI / Claude Code" },
  { key: "codex", name: "Codex", desc: "OpenAI Codex CLI" },
  { key: "gemini", name: "Gemini", desc: "Google Gemini CLI" },
];

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
  claude: "Claude Code",
  codex: "Codex",
  gemini: "Gemini",
};

export const CLI_SHORT_ITEMS: Array<{ key: CliKey; label: string }> = CLIS.map((cli) => ({
  key: cli.key,
  label: CLI_SHORT_LABELS[cli.key],
}));

export const CLI_FILTER_SHORT_ITEMS: CliFilterItem[] = [
  { key: "all", label: "全部" },
  ...CLI_SHORT_ITEMS,
];

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

type CliEnabledFlagKey = `enabled_${CliKey}`;

export type CliEnabledFlags = Record<CliEnabledFlagKey, boolean>;

export function enabledFlagForCli<T extends CliEnabledFlags>(row: T, cliKey: CliKey) {
  const key = `enabled_${cliKey}` as CliEnabledFlagKey;
  return row[key];
}

export function cliShortLabel(cliKey: string) {
  if (cliKey === "claude" || cliKey === "codex" || cliKey === "gemini") {
    return CLI_SHORT_LABELS[cliKey];
  }
  return cliKey;
}

const CLI_BADGE_BASE =
  "bg-slate-100 dark:bg-slate-700 text-slate-600 dark:text-slate-400 border border-transparent";

const CLI_BADGE_HOVER =
  "group-hover:bg-white dark:group-hover:bg-slate-800 group-hover:border-slate-200 dark:group-hover:border-slate-700";

export function cliBadgeTone(cliKey: string) {
  if (cliKey === "claude" || cliKey === "codex" || cliKey === "gemini")
    return `${CLI_BADGE_BASE} ${CLI_BADGE_HOVER}`;
  return CLI_BADGE_BASE;
}

export function cliBadgeToneStatic(_cliKey: string) {
  return CLI_BADGE_BASE;
}
