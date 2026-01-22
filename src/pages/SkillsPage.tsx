// Usage: Manage installed/local skills. Backend commands: `skills_installed_list`, `skills_local_list`, `skill_set_enabled`, `skill_uninstall`, `skill_import_local`.

import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import { ExternalLink } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { CLIS, cliFromKeyOrDefault, enabledFlagForCli, isCliKey } from "../constants/clis";
import { logToConsole } from "../services/consoleLog";
import type { CliKey } from "../services/providers";
import {
  skillImportLocal,
  skillSetEnabled,
  skillUninstall,
  skillsInstalledList,
  skillsLocalList,
  type InstalledSkillSummary,
  type LocalSkillSummary,
} from "../services/skills";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Dialog } from "../ui/Dialog";
import { PageHeader } from "../ui/PageHeader";
import { Switch } from "../ui/Switch";
import { TabList } from "../ui/TabList";
import { cn } from "../utils/cn";
import { formatActionFailureToast } from "../utils/errors";

function formatUnixSeconds(ts: number) {
  try {
    return new Date(ts * 1000).toLocaleString();
  } catch {
    return String(ts);
  }
}

function enabledForCli(skill: InstalledSkillSummary, cliKey: CliKey) {
  return enabledFlagForCli(skill, cliKey);
}

function enabledLabel(skill: InstalledSkillSummary) {
  const enabled: string[] = [];
  if (skill.enabled_claude) enabled.push("Claude");
  if (skill.enabled_codex) enabled.push("Codex");
  if (skill.enabled_gemini) enabled.push("Gemini");
  return enabled.length ? enabled.join(" / ") : "未启用";
}

function sourceHint(
  skill: Pick<InstalledSkillSummary, "source_git_url" | "source_branch" | "source_subdir">
) {
  return `${skill.source_git_url}#${skill.source_branch}:${skill.source_subdir}`;
}

function readCliFromStorage(): CliKey {
  try {
    const raw = localStorage.getItem("skills.activeCli");
    if (isCliKey(raw)) return raw;
  } catch {}
  return "claude";
}

function writeCliToStorage(cli: CliKey) {
  try {
    localStorage.setItem("skills.activeCli", cli);
  } catch {}
}

const CLI_TABS: Array<{ key: CliKey; label: string }> = CLIS.map((cli) => ({
  key: cli.key,
  label: cli.name,
}));

async function openPathOrReveal(path: string) {
  try {
    await openPath(path);
    return;
  } catch (err) {
    logToConsole("warn", "openPath 失败，尝试 revealItemInDir", {
      error: String(err),
      path,
    });
  }
  await revealItemInDir(path);
}

export function SkillsPage() {
  const navigate = useNavigate();
  const [activeCli, setActiveCli] = useState<CliKey>(() => readCliFromStorage());
  const currentCli = useMemo(() => cliFromKeyOrDefault(activeCli), [activeCli]);

  const [installed, setInstalled] = useState<InstalledSkillSummary[]>([]);
  const [localSkills, setLocalSkills] = useState<LocalSkillSummary[]>([]);

  const [loading, setLoading] = useState(false);
  const [localLoading, setLocalLoading] = useState(false);
  const [togglingSkillId, setTogglingSkillId] = useState<number | null>(null);
  const [uninstallingSkillId, setUninstallingSkillId] = useState<number | null>(null);

  const [uninstallTarget, setUninstallTarget] = useState<InstalledSkillSummary | null>(null);

  const [importTarget, setImportTarget] = useState<LocalSkillSummary | null>(null);
  const [importingLocal, setImportingLocal] = useState(false);

  useEffect(() => {
    writeCliToStorage(activeCli);
  }, [activeCli]);

  async function refreshInstalled() {
    setLoading(true);
    try {
      const rows = await skillsInstalledList();
      if (!rows) {
        setInstalled([]);
        return;
      }
      setInstalled(rows);
    } catch (err) {
      logToConsole("error", "加载 Skills 数据失败", { error: String(err) });
      toast("加载失败：请查看控制台日志");
    } finally {
      setLoading(false);
    }
  }

  async function refreshLocal(cliKey: CliKey) {
    setLocalLoading(true);
    try {
      const rows = await skillsLocalList(cliKey);
      if (!rows) {
        setLocalSkills([]);
        return;
      }
      setLocalSkills(rows);
    } catch (err) {
      logToConsole("error", "扫描本机 Skill 失败", {
        error: String(err),
        cli: cliKey,
      });
      toast("扫描本机 Skill 失败：请查看控制台日志");
    } finally {
      setLocalLoading(false);
    }
  }

  useEffect(() => {
    void refreshInstalled();
  }, []);

  useEffect(() => {
    void refreshLocal(activeCli);
  }, [activeCli]);

  async function toggleSkillEnabled(skill: InstalledSkillSummary, enabled: boolean) {
    if (togglingSkillId != null) return;
    setTogglingSkillId(skill.id);
    try {
      const next = await skillSetEnabled({
        skill_id: skill.id,
        cli_key: activeCli,
        enabled,
      });
      if (!next) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }
      setInstalled((prev) => prev.map((row) => (row.id === next.id ? next : row)));
      toast(enabled ? "已启用" : "已禁用");
    } catch (err) {
      const formatted = formatActionFailureToast("切换启用", err);
      logToConsole("error", "切换 Skill 启用状态失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
        cli: activeCli,
        skill_id: skill.id,
        enabled,
      });
      toast(formatted.toast);
    } finally {
      setTogglingSkillId(null);
    }
  }

  async function confirmUninstallSkill() {
    if (!uninstallTarget) return;
    if (uninstallingSkillId != null) return;
    setUninstallingSkillId(uninstallTarget.id);
    try {
      const ok = await skillUninstall(uninstallTarget.id);
      if (!ok) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }
      setInstalled((prev) => prev.filter((row) => row.id !== uninstallTarget.id));
      toast("已卸载");
      logToConsole("info", "卸载 Skill", uninstallTarget);
      setUninstallTarget(null);
    } catch (err) {
      const formatted = formatActionFailureToast("卸载", err);
      logToConsole("error", "卸载 Skill 失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
        skill: uninstallTarget,
      });
      toast(formatted.toast);
    } finally {
      setUninstallingSkillId(null);
    }
  }

  async function confirmImportLocalSkill() {
    if (!importTarget) return;
    if (importingLocal) return;
    setImportingLocal(true);
    try {
      const next = await skillImportLocal({
        cli_key: activeCli,
        dir_name: importTarget.dir_name,
      });
      if (!next) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      toast("已导入到技能库");
      logToConsole("info", "导入本机 Skill", { cli: activeCli, imported: next });
      setImportTarget(null);
      await refreshInstalled();
      await refreshLocal(activeCli);
    } catch (err) {
      const formatted = formatActionFailureToast("导入", err);
      logToConsole("error", "导入本机 Skill 失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
        cli: activeCli,
        skill: importTarget,
      });
      toast(formatted.toast);
    } finally {
      setImportingLocal(false);
    }
  }

  async function openLocalSkillDir(skill: LocalSkillSummary) {
    try {
      await openPathOrReveal(skill.path);
    } catch (err) {
      logToConsole("error", "打开本机 Skill 目录失败", {
        error: String(err),
        cli: activeCli,
        path: skill.path,
      });
      toast("打开目录失败：请查看控制台日志");
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="Skill"
        actions={
          <>
            <Button onClick={() => navigate("/skills/market")} variant="primary">
              Skill 市场
            </Button>
            <TabList
              ariaLabel="CLI 选择"
              items={CLI_TABS}
              value={activeCli}
              onChange={setActiveCli}
            />
          </>
        }
      />

      <div className="grid gap-4 lg:grid-cols-2">
        <Card className="min-h-[240px]" padding="md">
          <div className="flex items-start justify-between gap-3">
            <div className="text-sm font-semibold">通用技能</div>
            <span className="rounded-full bg-slate-100 px-2 py-1 text-xs font-medium text-slate-700">
              {installed.length}
            </span>
          </div>

          <div className="mt-4 space-y-2">
            {loading ? (
              <div className="text-sm text-slate-600">加载中…</div>
            ) : installed.length === 0 ? (
              <div className="rounded-xl border border-dashed border-slate-200 bg-slate-50 p-4 text-sm text-slate-600">
                暂无已安装 Skill。
              </div>
            ) : (
              installed.map((skill) => (
                <div key={skill.id} className="rounded-xl border border-slate-200 bg-white p-3">
                  <div className="flex items-center gap-2">
                    <span className="min-w-0 truncate text-sm font-semibold">{skill.name}</span>
                    <a
                      href={`${skill.source_git_url}${skill.source_branch ? `#` + skill.source_branch : ""}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="shrink-0 text-slate-400 hover:text-slate-600"
                      title={sourceHint(skill)}
                    >
                      <ExternalLink className="h-3.5 w-3.5" />
                    </a>
                    <div className="ms-auto flex items-center gap-2">
                      <span className="text-xs text-slate-600">启用</span>
                      <Switch
                        checked={enabledForCli(skill, activeCli)}
                        disabled={
                          togglingSkillId === skill.id || uninstallingSkillId === skill.id
                        }
                        onCheckedChange={(next) => void toggleSkillEnabled(skill, next)}
                      />
                      <Button
                        size="sm"
                        variant="secondary"
                        disabled={uninstallingSkillId === skill.id}
                        onClick={() => setUninstallTarget(skill)}
                      >
                        卸载
                      </Button>
                    </div>
                  </div>
                  {skill.description ? (
                    <div className="mt-1.5 text-xs text-slate-500">{skill.description}</div>
                  ) : null}
                  <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-slate-500">
                    <span
                      className={cn(
                        "rounded-full px-2 py-1 font-medium",
                        enabledForCli(skill, activeCli)
                          ? "bg-emerald-50 text-emerald-700"
                          : "bg-slate-100 text-slate-600"
                      )}
                    >
                      {enabledLabel(skill)}
                    </span>
                    <span>更新 {formatUnixSeconds(skill.updated_at)}</span>
                  </div>
                </div>
              ))
            )}
          </div>
        </Card>

        <Card className="min-h-[240px]" padding="md">
          <div className="flex items-start justify-between gap-3">
            <div className="text-sm font-semibold">本机已安装</div>
            <span className="rounded-full bg-slate-100 px-2 py-1 text-xs font-medium text-slate-700">
              {localLoading ? "扫描中…" : `${localSkills.length}`}
            </span>
          </div>

          <div className="mt-4 space-y-2">
            {localLoading ? (
              <div className="text-sm text-slate-600">扫描中…</div>
            ) : localSkills.length === 0 ? (
              <div className="rounded-xl border border-dashed border-slate-200 bg-slate-50 p-4 text-sm text-slate-600">
                未发现本机 Skill。
              </div>
            ) : (
              localSkills.map((skill) => (
                <div
                  key={skill.path}
                  className="rounded-xl border border-slate-200 bg-slate-50 p-3"
                >
                  <div className="flex items-center gap-2">
                    <span className="min-w-0 truncate text-sm font-semibold">
                      {skill.name || skill.dir_name}
                    </span>
                    <div className="ms-auto flex items-center gap-2">
                      <Button size="sm" variant="primary" onClick={() => setImportTarget(skill)}>
                        导入技能库
                      </Button>
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={() => void openLocalSkillDir(skill)}
                      >
                        打开目录
                      </Button>
                    </div>
                  </div>
                  {skill.description ? (
                    <div className="mt-1.5 text-xs text-slate-500">{skill.description}</div>
                  ) : null}
                  <div className="mt-2 truncate font-mono text-xs text-slate-500">
                    {skill.path}
                  </div>
                </div>
              ))
            )}
          </div>
        </Card>
      </div>

      <Dialog
        open={importTarget != null}
        title="导入到技能库"
        description="导入后该 Skill 会被 AIO 记录并管理，可同步到其他 CLI，并支持卸载。"
        onOpenChange={(open) => {
          if (!open) setImportTarget(null);
        }}
      >
        <div className="space-y-3">
          <div className="rounded-xl border border-slate-200 bg-slate-50 p-3 text-xs text-slate-600">
            <div className="font-medium text-slate-800">
              {importTarget?.name || importTarget?.dir_name}
            </div>
            <div className="mt-1 break-all font-mono">{importTarget?.path}</div>
          </div>

          <div className="text-sm text-slate-700">
            导入后你可以在 AIO 中对该 Skill 执行启用/禁用/卸载。
          </div>
          <div className="text-sm text-slate-700">
            注意：导入后该目录会被视为由 AIO 管理；后续卸载会删除 “{currentCli.name}” 的对应目录。
          </div>

          <div className="flex items-center justify-end gap-2">
            <Button
              variant="secondary"
              onClick={() => setImportTarget(null)}
              disabled={importingLocal}
            >
              取消
            </Button>
            <Button
              variant="primary"
              onClick={() => void confirmImportLocalSkill()}
              disabled={importingLocal}
            >
              {importingLocal ? "导入中…" : "确认导入"}
            </Button>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={uninstallTarget != null}
        title="卸载 Skill"
        description="将从 AIO 技能库与三端 CLI skills 目录移除（仅删除由本工具管理并带 marker 的目录）。"
        onOpenChange={(open) => {
          if (!open) setUninstallTarget(null);
        }}
      >
        <div className="space-y-3">
          <div className="text-sm text-slate-700">确认卸载以下 Skill？</div>
          <div className="rounded-xl border border-slate-200 bg-slate-50 p-3 text-xs text-slate-600">
            <div className="font-medium text-slate-800">{uninstallTarget?.name}</div>
            <div className="mt-1 break-all font-mono">
              {uninstallTarget ? sourceHint(uninstallTarget) : null}
            </div>
          </div>
          <div className="flex items-center justify-end gap-2">
            <Button
              variant="secondary"
              onClick={() => setUninstallTarget(null)}
              disabled={uninstallingSkillId != null}
            >
              取消
            </Button>
            <Button
              variant="primary"
              onClick={() => void confirmUninstallSkill()}
              disabled={uninstallingSkillId != null}
            >
              {uninstallingSkillId != null ? "卸载中…" : "确认卸载"}
            </Button>
          </div>
        </div>
      </Dialog>
    </div>
  );
}
