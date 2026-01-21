// Usage: Shared CLI definitions and derived UI filter helpers.

import type { CliKey } from "../services/providers";

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
  if (cliKey === "claude") return "Claude";
  if (cliKey === "codex") return "Codex";
  if (cliKey === "gemini") return "Gemini";
  return cliKey;
}

export function cliBadgeTone(cliKey: string) {
  if (cliKey === "claude")
    return "bg-slate-100 text-slate-600 group-hover:bg-white group-hover:border-slate-200 border border-transparent";
  if (cliKey === "codex")
    return "bg-slate-100 text-slate-600 group-hover:bg-white group-hover:border-slate-200 border border-transparent";
  if (cliKey === "gemini")
    return "bg-slate-100 text-slate-600 group-hover:bg-white group-hover:border-slate-200 border border-transparent";
  return "bg-slate-100 text-slate-600 border border-transparent";
}
