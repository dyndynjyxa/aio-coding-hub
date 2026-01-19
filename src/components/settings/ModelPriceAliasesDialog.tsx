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
      title="定价匹配（别名）"
      description="用于解决 requested_model 与已同步 model_prices 名称不一致导致的 cost 缺失。仅在精确查价失败时触发。"
      className="max-w-4xl"
    >
      <div className="space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="text-xs text-slate-600">
            已启用 {enabledRuleCount} 条；可用模型数：Claude {modelCountsByCli.claude} / Codex{" "}
            {modelCountsByCli.codex} / Gemini {modelCountsByCli.gemini}
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
          <div className="rounded-lg border border-slate-200 bg-slate-50 p-4 text-sm text-slate-600">
            加载中…
          </div>
        ) : aliases.rules.length === 0 ? (
          <div className="rounded-lg border border-slate-200 bg-slate-50 p-4 text-sm text-slate-600">
            暂无规则。示例：Gemini 可配置 `prefix gemini-3-flash` → `gemini-3-flash-preview`。
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
                    "rounded-xl border border-slate-200 bg-white p-4 shadow-sm",
                    disabled ? "opacity-70" : null
                  )}
                >
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <div className="flex items-center gap-3">
                      <div className="text-xs font-semibold text-slate-900">规则 #{idx + 1}</div>
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

                  <div className="mt-3 grid gap-3 lg:grid-cols-12">
                    <div className="lg:col-span-2">
                      <div className="mb-1 text-xs font-medium text-slate-600">CLI</div>
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

                    <div className="lg:col-span-3">
                      <div className="mb-1 text-xs font-medium text-slate-600">匹配类型</div>
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

                    <div className="lg:col-span-3">
                      <div className="mb-1 text-xs font-medium text-slate-600">
                        Pattern（用于匹配 requested_model）
                      </div>
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
                      <div className="mt-1 text-[11px] text-slate-500">
                        {matchType === "wildcard"
                          ? "wildcard：仅支持单个 *"
                          : matchType === "prefix"
                            ? "prefix：requested_model 以 pattern 开头即命中"
                            : "exact：完全相等才命中"}
                      </div>
                    </div>

                    <div className="lg:col-span-4">
                      <div className="mb-1 text-xs font-medium text-slate-600">
                        目标模型（从 model_prices 中取价）
                      </div>
                      <Input
                        mono
                        list={modelsDatalistId(cliKey)}
                        value={rule.target_model ?? ""}
                        onChange={(e) => updateRule(idx, { target_model: e.currentTarget.value })}
                        placeholder="输入或从建议中选择…"
                        disabled={saving}
                      />
                      <div className="mt-1 text-[11px] text-slate-500">
                        建议从下拉列表选择，避免拼写不一致导致仍然无法计费。
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}

        <div className="flex items-center justify-end gap-2 border-t border-slate-200 pt-3">
          <Button variant="secondary" onClick={() => onOpenChange(false)} disabled={saving}>
            取消
          </Button>
          <Button variant="primary" onClick={save} disabled={loading || saving}>
            {saving ? "保存中…" : "保存"}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
