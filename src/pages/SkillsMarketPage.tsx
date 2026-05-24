// Usage: Discover and install skills from repos. Backend commands: `skills_discover_available`, `skill_install_to_local`, `skill_repos_*`, `skills_installed_list`, `skills_local_list`.

import { ChevronDown, ChevronRight, ExternalLink } from "lucide-react";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Dialog } from "../ui/Dialog";
import { EmptyState } from "../ui/EmptyState";
import { Spinner } from "../ui/Spinner";
import { Switch } from "../ui/Switch";
import { TabList } from "../ui/TabList";
import { useSkillsMarketPageDataModel } from "./skills-market/useSkillsMarketPageDataModel";

export function SkillsMarketPage() {
  const model = useSkillsMarketPageDataModel();
  const {
    navigate,
    orderedCliTabs,
    effectiveCli,
    setActiveCli,
    currentCli,
    repos,
    enabledRepoCount,
    activeWorkspaceId,
    loading,
    discovering,
    query,
    setQuery,
    repoFilter,
    setRepoFilter,
    onlyActionable,
    setOnlyActionable,
    expandedRepos,
    installingRepoKey,
    installingSources,
    installBusy,
    repoDialogOpen,
    setRepoDialogOpen,
    newRepoUrl,
    setNewRepoUrl,
    newRepoBranch,
    setNewRepoBranch,
    repoSaving,
    repoToggleId,
    repoDeleteTarget,
    setRepoDeleteTarget,
    repoDeleting,
    repoOptions,
    available,
    groupedAvailable,
    refreshAvailable,
    addRepo,
    toggleRepoEnabled,
    confirmDeleteRepo,
    installSingleSkill,
    installWholeRepo,
    toggleRepoExpanded,
    getStatus,
    statusLabel,
    statusTone,
    repositoryWebUrl,
    sourceHint,
    sourceKey,
    formatUnixSeconds,
  } = model;

  return (
    <div className="flex h-full flex-col gap-4 overflow-hidden">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-3">
          <h1 className="text-2xl font-semibold tracking-tight">Skill 市场</h1>
          <Button onClick={() => navigate("/skills")} variant="secondary">
            返回 Skill
          </Button>
          <Button onClick={() => setRepoDialogOpen(true)} variant="secondary">
            管理仓库
          </Button>
          <Button
            onClick={() => void refreshAvailable(true)}
            variant="primary"
            disabled={discovering}
          >
            {discovering ? "刷新中…" : "刷新发现"}
          </Button>
        </div>

        <TabList
          ariaLabel="CLI 选择"
          items={orderedCliTabs.map((cli) => ({ key: cli.key, label: cli.name }))}
          value={effectiveCli}
          onChange={setActiveCli}
        />
      </div>

      <Card
        padding="md"
        className="border-border/80 bg-[radial-gradient(circle_at_top_left,_rgba(14,165,233,0.08),_transparent_45%),linear-gradient(135deg,rgba(255,255,255,0.98),rgba(248,250,252,0.96))] dark:border-border dark:bg-card"
      >
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="max-w-3xl">
            <div className="text-sm font-semibold text-foreground">
              按仓库浏览，默认直接安装到当前 CLI
            </div>
            <div className="mt-1 text-xs leading-6 text-muted-foreground">
              现在市场页默认把技能装进 <span className="font-medium">{currentCli.name}</span>{" "}
              的本机目录，不会先进入通用技能区。需要统一管理时，再去 Skill 页面导入到通用技能。
            </div>
            {!activeWorkspaceId ? (
              <div className="mt-2 text-xs text-amber-700 dark:text-amber-400">
                当前还没有激活的 workspace，安装前请先去 Workspaces 页面设置当前工作区。
              </div>
            ) : null}
          </div>

          <div className="flex flex-wrap items-center gap-2 text-xs">
            <span className="rounded-full bg-white/80 px-3 py-1.5 text-secondary-foreground shadow-sm dark:bg-secondary dark:text-secondary-foreground">
              已启用仓库 {enabledRepoCount} / {repos.length}
            </span>
            <span className="rounded-full bg-white/80 px-3 py-1.5 text-secondary-foreground shadow-sm dark:bg-secondary dark:text-secondary-foreground">
              当前 CLI {currentCli.name}
            </span>
            <span className="rounded-full bg-white/80 px-3 py-1.5 text-secondary-foreground shadow-sm dark:bg-secondary dark:text-secondary-foreground">
              发现技能 {available.length}
            </span>
          </div>
        </div>
      </Card>

      <Card
        className="min-h-0 flex flex-1 flex-col overflow-hidden"
        padding="md"
        data-testid="skills-market-list-card"
      >
        <div className="flex flex-wrap items-center gap-2">
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="搜索技能、仓库、目录"
            className="w-full rounded-xl border border-border bg-white px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-accent/30 dark:border-border dark:bg-secondary dark:text-foreground sm:w-[320px]"
          />

          <select
            value={repoFilter}
            onChange={(e) => setRepoFilter(e.target.value)}
            className="h-10 rounded-xl border border-border bg-white px-3 text-sm outline-none focus:ring-2 focus:ring-accent/30 dark:border-border dark:bg-secondary dark:text-foreground"
          >
            <option value="all">全部仓库</option>
            {repoOptions.map((option) => (
              <option key={option.key} value={option.key}>
                {option.label}
              </option>
            ))}
          </select>

          <div className="flex items-center gap-2 rounded-xl border border-border bg-white px-3 py-2 dark:border-border dark:bg-secondary">
            <span className="text-xs text-muted-foreground">仅显示可安装</span>
            <Switch checked={onlyActionable} onCheckedChange={setOnlyActionable} />
          </div>

          {query ? (
            <Button size="sm" variant="ghost" onClick={() => setQuery("")}>
              清空
            </Button>
          ) : null}
        </div>

        <div
          className="mt-4 min-h-0 flex-1 overflow-y-auto pr-1 scrollbar-overlay"
          data-testid="skills-market-scroll-region"
        >
          {loading ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Spinner size="sm" />
              加载中…
            </div>
          ) : discovering ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Spinner size="sm" />
              扫描中…
            </div>
          ) : enabledRepoCount === 0 ? (
            <EmptyState
              title="暂无启用的仓库"
              description="先添加并启用仓库，再点击右上角“刷新发现”。"
            />
          ) : groupedAvailable.length === 0 ? (
            <EmptyState
              title="没有匹配的仓库或技能"
              description="可以试试清空搜索、切换仓库，或者关闭“仅显示可安装”。"
            />
          ) : (
            <div className="space-y-3">
              {groupedAvailable.map((group) => {
                const expanded = expandedRepos.has(group.key);
                const repoUrl = repositoryWebUrl(group.gitUrl);

                return (
                  <section
                    key={group.key}
                    className="rounded-2xl border border-border/80 bg-[linear-gradient(180deg,rgba(248,250,252,0.96),rgba(255,255,255,0.92))] p-4 dark:border-border dark:bg-card"
                  >
                    <div className="flex flex-wrap items-start justify-between gap-4">
                      <div className="min-w-0 flex-1">
                        <button
                          type="button"
                          onClick={() => toggleRepoExpanded(group.key)}
                          className="flex min-w-0 items-start gap-3 text-left"
                        >
                          <span className="mt-0.5 rounded-full border border-border bg-white p-1 text-muted-foreground dark:border-border dark:bg-secondary dark:text-secondary-foreground">
                            {expanded ? (
                              <ChevronDown className="h-4 w-4" />
                            ) : (
                              <ChevronRight className="h-4 w-4" />
                            )}
                          </span>
                          <div className="min-w-0">
                            <div className="flex flex-wrap items-center gap-2">
                              <span className="truncate text-base font-semibold text-foreground">
                                {group.repoPrefix}
                              </span>
                              <span className="rounded-full bg-card px-2.5 py-1 text-[11px] font-medium text-white dark:bg-secondary dark:text-foreground">
                                {group.skills.length} 个技能
                              </span>
                              {group.installableCount > 0 ? (
                                <span className="rounded-full bg-sky-50 px-2.5 py-1 text-[11px] font-medium text-sky-700 dark:bg-sky-900/30 dark:text-sky-300">
                                  可安装 {group.installableCount}
                                </span>
                              ) : null}
                              {group.localCount > 0 ? (
                                <span className="rounded-full bg-sky-50 px-2.5 py-1 text-[11px] font-medium text-sky-700 dark:bg-sky-900/30 dark:text-sky-300">
                                  本机 {group.localCount}
                                </span>
                              ) : null}
                              {group.enabledCount > 0 ? (
                                <span className="rounded-full bg-emerald-50 px-2.5 py-1 text-[11px] font-medium text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400">
                                  通用已启用 {group.enabledCount}
                                </span>
                              ) : null}
                            </div>
                            <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                              <span className="font-mono">{group.repoPath}</span>
                              <span>branch: {group.branch}</span>
                            </div>
                          </div>
                        </button>
                      </div>

                      <div className="flex flex-wrap items-center justify-end gap-2">
                        {repoUrl ? (
                          <a
                            href={repoUrl}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="inline-flex h-9 w-9 items-center justify-center rounded-full border border-border bg-white text-muted-foreground transition hover:text-foreground dark:border-border dark:bg-secondary dark:text-secondary-foreground dark:hover:text-foreground"
                            title={group.repoPath}
                          >
                            <ExternalLink className="h-4 w-4" />
                          </a>
                        ) : null}
                        <Button
                          size="sm"
                          variant="secondary"
                          onClick={() => toggleRepoExpanded(group.key)}
                        >
                          {expanded ? "收起" : "展开"}
                        </Button>
                        <Button
                          size="sm"
                          variant="primary"
                          disabled={group.installableCount === 0 || installBusy}
                          onClick={() => void installWholeRepo(group)}
                        >
                          {installingRepoKey === group.key ? "安装中…" : `安装本仓库全部技能`}
                        </Button>
                      </div>
                    </div>

                    {!expanded ? (
                      <div className="mt-3 flex flex-wrap gap-2">
                        {group.skills.slice(0, 4).map((skill) => (
                          <span
                            key={sourceKey(skill)}
                            className="rounded-full border border-border bg-white px-3 py-1 text-xs text-muted-foreground dark:border-border dark:bg-secondary dark:text-secondary-foreground"
                          >
                            {skill.name}
                          </span>
                        ))}
                        {group.skills.length > 4 ? (
                          <span className="rounded-full border border-dashed border-border px-3 py-1 text-xs text-muted-foreground dark:border-border dark:text-muted-foreground">
                            还有 {group.skills.length - 4} 个
                          </span>
                        ) : null}
                      </div>
                    ) : (
                      <div className="mt-4 space-y-2">
                        {group.skills.map((skill) => {
                          const key = sourceKey(skill);
                          const status = getStatus(skill);
                          const installing = installingSources.has(key);

                          return (
                            <div
                              key={key}
                              className="rounded-xl border border-border bg-white/90 px-3 py-3 dark:border-border dark:bg-secondary"
                            >
                              <div className="flex flex-wrap items-start gap-3">
                                <div className="min-w-0 flex-1">
                                  <div className="flex flex-wrap items-center gap-2">
                                    <span className="min-w-0 truncate text-sm font-semibold text-foreground">
                                      {skill.name}
                                    </span>
                                    <span
                                      className={`rounded-full px-2 py-0.5 text-[11px] font-medium ${statusTone(status)}`}
                                    >
                                      {statusLabel(status)}
                                    </span>
                                    {repoUrl ? (
                                      <a
                                        href={repoUrl}
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        className="text-muted-foreground hover:text-muted-foreground dark:text-muted-foreground dark:hover:text-muted-foreground"
                                        title={sourceHint(skill)}
                                      >
                                        <ExternalLink className="h-3.5 w-3.5" />
                                      </a>
                                    ) : null}
                                  </div>
                                  {skill.description ? (
                                    <div className="mt-1.5 text-xs text-muted-foreground">
                                      {skill.description}
                                    </div>
                                  ) : null}
                                  <div className="mt-2 truncate font-mono text-[11px] text-muted-foreground">
                                    {skill.source_subdir}
                                  </div>
                                </div>

                                <div className="ms-auto flex shrink-0 flex-wrap items-center justify-end gap-2">
                                  {status === "not_installed" ? (
                                    <Button
                                      size="sm"
                                      variant="primary"
                                      disabled={installBusy}
                                      onClick={() => void installSingleSkill(skill)}
                                    >
                                      {installing ? "安装中…" : `安装到 ${currentCli.name}`}
                                    </Button>
                                  ) : status === "needs_enable" ? (
                                    <Button
                                      size="sm"
                                      variant="secondary"
                                      onClick={() => navigate("/skills")}
                                    >
                                      去通用技能
                                    </Button>
                                  ) : status === "local_installed" ? (
                                    <Button
                                      size="sm"
                                      variant="secondary"
                                      onClick={() => navigate("/skills")}
                                    >
                                      查看本机已安装
                                    </Button>
                                  ) : (
                                    <Button size="sm" variant="secondary" disabled>
                                      已在通用技能
                                    </Button>
                                  )}
                                </div>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </section>
                );
              })}
            </div>
          )}
        </div>
      </Card>

      <Dialog
        open={repoDialogOpen}
        title="Skill 仓库"
        description="启用后的仓库会参与发现。刷新发现只会更新 ~/.aio-coding-hub/skill-repos 下的缓存副本，不会动你的原始仓库。"
        onOpenChange={setRepoDialogOpen}
      >
        <div className="space-y-4">
          <div className="rounded-xl border border-border bg-secondary p-3 dark:border-border dark:bg-secondary">
            <div className="text-sm font-semibold">添加仓库</div>
            <div className="mt-2 grid gap-3 sm:grid-cols-3">
              <div className="sm:col-span-2">
                <div className="text-xs font-medium text-muted-foreground">Git URL</div>
                <input
                  value={newRepoUrl}
                  onChange={(e) => setNewRepoUrl(e.target.value)}
                  placeholder="https://github.com/owner/repo"
                  className="mt-1 w-full rounded-lg border border-border bg-white px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-accent/30 dark:border-border dark:bg-secondary dark:text-foreground"
                />
              </div>
              <div>
                <div className="text-xs font-medium text-muted-foreground">Branch</div>
                <input
                  value={newRepoBranch}
                  onChange={(e) => setNewRepoBranch(e.target.value)}
                  placeholder="auto / main / master"
                  className="mt-1 w-full rounded-lg border border-border bg-white px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-accent/30 dark:border-border dark:bg-secondary dark:text-foreground"
                />
                <div className="mt-1 text-[11px] text-muted-foreground">
                  推荐使用 <span className="font-mono">auto</span>。
                </div>
              </div>
            </div>
            <div className="mt-3 flex justify-end">
              <Button onClick={() => void addRepo()} variant="primary" disabled={repoSaving}>
                {repoSaving ? "添加中…" : "添加仓库"}
              </Button>
            </div>
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between gap-3">
              <div className="text-sm font-semibold">仓库列表</div>
              <span className="text-xs text-muted-foreground">{repos.length} 个</span>
            </div>

            {repos.length === 0 ? (
              <EmptyState
                title="暂无仓库"
                description="添加后点击页面右上角“刷新发现”即可扫描技能。"
              />
            ) : (
              repos.map((repo) => {
                const repoUrl = repositoryWebUrl(repo.git_url);
                return (
                  <div
                    key={repo.id}
                    className="rounded-xl border border-border bg-white p-3 dark:border-border dark:bg-secondary"
                  >
                    <div className="flex items-center gap-2">
                      <span className="min-w-0 truncate text-sm font-medium">{repo.git_url}</span>
                      {repoUrl ? (
                        <a
                          href={repoUrl}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="shrink-0 text-muted-foreground hover:text-muted-foreground dark:text-muted-foreground dark:hover:text-muted-foreground"
                          title={repo.git_url}
                        >
                          <ExternalLink className="h-3.5 w-3.5" />
                        </a>
                      ) : null}
                      <div className="ms-auto flex items-center gap-2">
                        <span className="text-xs text-muted-foreground">启用</span>
                        <Switch
                          checked={repo.enabled}
                          disabled={repoToggleId === repo.id || repoDeleting}
                          onCheckedChange={(next) => void toggleRepoEnabled(repo, next)}
                        />
                        <Button
                          size="sm"
                          variant="secondary"
                          disabled={repoDeleting}
                          onClick={() => setRepoDeleteTarget(repo)}
                        >
                          删除
                        </Button>
                      </div>
                    </div>
                    <div className="mt-1.5 flex items-center gap-3 text-xs text-muted-foreground">
                      <span>
                        branch: <span className="font-mono">{repo.branch}</span>
                      </span>
                      <span>更新 {formatUnixSeconds(repo.updated_at)}</span>
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>
      </Dialog>

      <Dialog
        open={repoDeleteTarget != null}
        title="删除仓库"
        description="这只会移除本地记录，不会删除你的 Git 仓库。"
        onOpenChange={(open) => {
          if (!open) setRepoDeleteTarget(null);
        }}
      >
        <div className="space-y-3">
          <div className="text-sm text-secondary-foreground">确认删除以下仓库？</div>
          <div className="rounded-xl border border-border bg-secondary p-3 text-xs text-muted-foreground dark:border-border dark:bg-secondary dark:text-muted-foreground">
            <div className="break-all font-mono">{repoDeleteTarget?.git_url}</div>
            <div className="mt-1">
              branch: <span className="font-mono">{repoDeleteTarget?.branch}</span>
            </div>
          </div>
          <div className="flex items-center justify-end gap-2">
            <Button
              variant="secondary"
              onClick={() => setRepoDeleteTarget(null)}
              disabled={repoDeleting}
            >
              取消
            </Button>
            <Button
              variant="primary"
              onClick={() => void confirmDeleteRepo()}
              disabled={repoDeleting}
            >
              {repoDeleting ? "删除中…" : "确认删除"}
            </Button>
          </div>
        </div>
      </Dialog>
    </div>
  );
}
