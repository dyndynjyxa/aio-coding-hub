// Usage:
// - Rendered by `src/pages/SettingsPage.tsx` from the "数据与同步" section.
// - Configure model price alias rules used by backend request log cost calculation.

import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { Switch } from "../../ui/Switch";
import { cn } from "../../utils/cn";
import type { CliKey } from "../../services/providers";
import {
  modelPriceAliasesGet,
  modelPriceAliasesSet,
  modelPricesList,
  type ModelPriceAliasMatchType,
  type ModelPriceAliasRule,
  type ModelPriceAliases,
} from "../../services/modelPrices";

const CLI_ITEMS: Array<{ key: CliKey; label: string }> = [
  { key: "claude", label: "Claude" },
  { key: "codex", label: "Codex" },
  { key: "gemini", label: "Gemini" },
];

const MATCH_TYPE_ITEMS: Array<{ key: ModelPriceAliasMatchType; label: string }> = [
  { key: "exact", label: "精确 (exact)" },
  { key: "wildcard", label: "通配符 (wildcard: 单个 *)" },
  { key: "prefix", label: "前缀 (prefix)" },
];

const EMPTY_ALIASES: ModelPriceAliases = { version: 1, rules: [] };

function newRule(seed?: Partial<ModelPriceAliasRule>): ModelPriceAliasRule {
  return {
    cli_key: seed?.cli_key ?? "gemini",
    match_type: seed?.match_type ?? "prefix",
    pattern: seed?.pattern ?? "",
    target_model: seed?.target_model ?? "",
    enabled: seed?.enabled ?? true,
  };
}

function normalizeAliases(input: ModelPriceAliases | null | undefined): ModelPriceAliases {
  if (!input || typeof input !== "object") return { ...EMPTY_ALIASES };
  const version = Number.isFinite(input.version) ? input.version : 1;
  const rules = Array.isArray(input.rules) ? input.rules : [];
  return { version, rules };
}

function modelsDatalistId(cliKey: CliKey) {
  return `model-price-aliases-models-${cliKey}`;
}

export function ModelPriceAliasesDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [aliases, setAliases] = useState<ModelPriceAliases>({ ...EMPTY_ALIASES });

  const [modelsByCli, setModelsByCli] = useState<Record<CliKey, string[]>>({
    claude: [],
    codex: [],
    gemini: [],
  });

  const modelCountsByCli = useMemo(() => {
    return {
      claude: modelsByCli.claude.length,
      codex: modelsByCli.codex.length,
      gemini: modelsByCli.gemini.length,
    };
  }, [modelsByCli]);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [aliasesRes, claude, codex, gemini] = await Promise.all([
        modelPriceAliasesGet(),
        modelPricesList("claude"),
        modelPricesList("codex"),
        modelPricesList("gemini"),
      ]);

      if (!aliasesRes) {
        toast("仅在 Tauri Desktop 环境可用");
        setAliases({ ...EMPTY_ALIASES });
        return;
      }

      setAliases(normalizeAliases(aliasesRes));

      setModelsByCli({
        claude: (claude ?? []).map((row) => row.model),
        codex: (codex ?? []).map((row) => row.model),
        gemini: (gemini ?? []).map((row) => row.model),
      });
    } catch (err) {
      toast("加载定价匹配规则失败：请查看控制台日志");
      // eslint-disable-next-line no-console
      console.error("[ModelPriceAliasesDialog] load error", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    void (async () => {
      await load();
      if (cancelled) return;
    })();
    return () => {
      cancelled = true;
    };
  }, [open, load]);

  const enabledRuleCount = useMemo(() => {
    return (aliases.rules ?? []).filter((r) => r?.enabled).length;
  }, [aliases.rules]);

  function updateRule(index: number, patch: Partial<ModelPriceAliasRule>) {
    setAliases((prev) => {
      const rules = (prev.rules ?? []).slice();
      const cur = rules[index] ?? newRule();
      rules[index] = { ...cur, ...patch };
      return { ...prev, rules };
    });
  }

  function deleteRule(index: number) {
    setAliases((prev) => {
      const rules = (prev.rules ?? []).slice();
      rules.splice(index, 1);
      return { ...prev, rules };
    });
  }

  async function save() {
    if (saving) return;
    setSaving(true);
    try {
      const saved = await modelPriceAliasesSet(aliases);
      if (!saved) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }
      setAliases(normalizeAliases(saved));
      toast("已保存定价匹配规则");
      onOpenChange(false);
    } catch (err) {
      toast("保存失败：请检查规则内容（例如 wildcard 只能包含一个 *）");
      // eslint-disable-next-line no-console
      console.error("[ModelPriceAliasesDialog] save error", err);
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (saving) return;
        onOpenChange(next);
      }}
      title="定价匹配"
      description="用于解决 requested_model 与已同步 model_prices 名称不一致导致的 cost 缺失。仅在精确查价失败时触发。"
      className="max-w-4xl"
    >
      <div className="space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-2 text-xs text-slate-600">
            <span className="inline-flex items-center rounded-md bg-slate-100 px-2 py-1 font-medium">
              启用 {enabledRuleCount} 条
            </span>
            <span className="text-slate-400">|</span>
            <span>
              模型数：Claude {modelCountsByCli.claude} · Codex {modelCountsByCli.codex} · Gemini{" "}
              {modelCountsByCli.gemini}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="secondary"
              size="sm"
              disabled={loading || saving}
              onClick={() => setAliases((prev) => ({ ...prev, rules: [...prev.rules, newRule()] }))}
            >
              新增规则
            </Button>
            <Button variant="secondary" size="sm" disabled={loading || saving} onClick={load}>
              刷新
            </Button>
          </div>
        </div>

        <datalist id={modelsDatalistId("claude")}>
          {modelsByCli.claude.map((m) => (
            <option key={`claude:${m}`} value={m} />
          ))}
        </datalist>
        <datalist id={modelsDatalistId("codex")}>
          {modelsByCli.codex.map((m) => (
            <option key={`codex:${m}`} value={m} />
          ))}
        </datalist>
        <datalist id={modelsDatalistId("gemini")}>
          {modelsByCli.gemini.map((m) => (
            <option key={`gemini:${m}`} value={m} />
          ))}
        </datalist>

        {loading ? (
          <div className="flex items-center justify-center rounded-xl border border-slate-200 bg-slate-50/50 p-8">
            <div className="flex items-center gap-3 text-sm text-slate-500">
              <svg
                className="h-5 w-5 animate-spin text-slate-400"
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
              >
                <circle
                  className="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  strokeWidth="4"
                />
                <path
                  className="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                />
              </svg>
              <span>加载规则中…</span>
            </div>
          </div>
        ) : aliases.rules.length === 0 ? (
          <div className="rounded-xl border border-dashed border-slate-300 bg-slate-50/50 p-8 text-center">
            <div className="mx-auto mb-2 h-10 w-10 rounded-full bg-slate-100 p-2.5 text-slate-400">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
              </svg>
            </div>
            <div className="text-sm font-medium text-slate-700">暂无规则</div>
            <div className="mt-1 text-xs text-slate-500">
              示例：Gemini 配置 <code className="rounded bg-slate-200 px-1 py-0.5 font-mono text-[11px]">prefix gemini-3-flash</code> → <code className="rounded bg-slate-200 px-1 py-0.5 font-mono text-[11px]">gemini-3-flash-preview</code>
            </div>
          </div>
        ) : (
          <div className="space-y-3">
            {aliases.rules.map((rule, idx) => {
              const cliKey: CliKey = (rule?.cli_key as CliKey) ?? "gemini";
              const matchType: ModelPriceAliasMatchType = rule?.match_type ?? "prefix";
              const disabled = !rule?.enabled;
              return (
                <div
                  key={`rule-${idx}`}
                  className={cn(
                    "group rounded-xl border border-slate-200 bg-white p-4 shadow-sm",
                    "transition-all duration-200 ease-in-out",
                    disabled
                      ? "opacity-60 grayscale-[30%]"
                      : "hover:border-slate-300 hover:shadow-md"
                  )}
                >
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <div className="flex items-center gap-3">
                      <div className="text-xs font-semibold text-slate-800">规则 #{idx + 1}</div>
                      <div className="flex items-center gap-2">
                        <span className="text-xs text-slate-600">启用</span>
                        <Switch
                          size="sm"
                          checked={!!rule.enabled}
                          onCheckedChange={(checked) => updateRule(idx, { enabled: checked })}
                        />
                      </div>
                    </div>
                    <Button
                      variant="danger"
                      size="sm"
                      onClick={() => {
                        const ok = window.confirm("确认删除该规则？");
                        if (!ok) return;
                        deleteRule(idx);
                      }}
                    >
                      删除
                    </Button>
                  </div>

                  <div className="mt-4 grid items-start gap-4 lg:grid-cols-12">
                    <div className="lg:col-span-2">
                      <label className="mb-1.5 block text-xs font-medium text-slate-700">CLI</label>
                      <Select
                        value={cliKey}
                        onChange={(e) =>
                          updateRule(idx, { cli_key: e.currentTarget.value as CliKey })
                        }
                        disabled={saving}
                      >
                        {CLI_ITEMS.map((it) => (
                          <option key={it.key} value={it.key}>
                            {it.label}
                          </option>
                        ))}
                      </Select>
                    </div>

                    <div className="lg:col-span-2">
                      <label className="mb-1.5 block text-xs font-medium text-slate-700">匹配类型</label>
                      <Select
                        value={matchType}
                        onChange={(e) =>
                          updateRule(idx, {
                            match_type: e.currentTarget.value as ModelPriceAliasMatchType,
                          })
                        }
                        disabled={saving}
                      >
                        {MATCH_TYPE_ITEMS.map((it) => (
                          <option key={it.key} value={it.key}>
                            {it.label}
                          </option>
                        ))}
                      </Select>
                    </div>

                    <div className="lg:col-span-4">
                      <label className="mb-1.5 block text-xs font-medium text-slate-700">Pattern</label>
                      <Input
                        mono
                        value={rule.pattern ?? ""}
                        onChange={(e) => updateRule(idx, { pattern: e.currentTarget.value })}
                        placeholder={
                          matchType === "exact"
                            ? "例如：gemini-3-flash"
                            : matchType === "wildcard"
                              ? "例如：gemini-3-*-preview"
                              : "例如：claude-opus-4-5"
                        }
                        disabled={saving}
                      />
                      <p className="mt-1.5 text-[11px] leading-relaxed text-slate-500">
                        {matchType === "wildcard"
                          ? "wildcard：仅支持单个 *"
                          : matchType === "prefix"
                            ? "prefix：以 pattern 开头即命中"
                            : "exact：完全相等才命中"}
                      </p>
                    </div>

                    <div className="lg:col-span-4">
                      <label className="mb-1.5 block text-xs font-medium text-slate-700">目标模型</label>
                      <Input
                        mono
                        list={modelsDatalistId(cliKey)}
                        value={rule.target_model ?? ""}
                        onChange={(e) => updateRule(idx, { target_model: e.currentTarget.value })}
                        placeholder="输入或从建议中选择…"
                        disabled={saving}
                      />
                      <p className="mt-1.5 text-[11px] leading-relaxed text-slate-500">
                        下拉列表选择具体模型
                      </p>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}

        <div className="flex items-center justify-end gap-3 border-t border-slate-200 pt-4">
          <Button variant="secondary" onClick={() => onOpenChange(false)} disabled={saving}>
            取消
          </Button>
          <Button variant="primary" onClick={save} disabled={loading || saving}>
            {saving ? (
              <span className="flex items-center gap-2">
                <svg
                  className="h-4 w-4 animate-spin"
                  xmlns="http://www.w3.org/2000/svg"
                  fill="none"
                  viewBox="0 0 24 24"
                >
                  <circle
                    className="opacity-25"
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    strokeWidth="4"
                  />
                  <path
                    className="opacity-75"
                    fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                  />
                </svg>
                保存中…
              </span>
            ) : (
              "保存"
            )}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
