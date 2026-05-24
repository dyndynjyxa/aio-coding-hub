import { createPortal } from "react-dom";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { FormField } from "../../ui/FormField";
import { Select } from "../../ui/Select";
import { Play, Server, Network, RefreshCw } from "lucide-react";

import type { ClaudeModelValidationDialogProps } from "./types";
import { ModelCombobox } from "./ModelCombobox";
import { HistoryListPanel } from "./HistoryListPanel";
import { DetailsPane } from "./DetailsPane";
import { useClaudeValidationState } from "./useClaudeValidationState";

export function ClaudeModelValidationDialog({
  open,
  onOpenChange,
  provider,
}: ClaudeModelValidationDialogProps) {
  const state = useClaudeValidationState(open, provider, onOpenChange);

  const {
    baseUrl,
    setBaseUrl,
    baseUrlPicking,
    templates,
    model,
    setModel,
    requestJson,
    setRequestJson,
    apiKeyPlaintext,
    validating,
    suiteSteps,
    suiteProgress,
    suiteIssuesOnly,
    setSuiteIssuesOnly,
    suiteActiveStepIndex,
    setSuiteActiveStepIndex,
    detailsTab,
    setDetailsTab,
    historyLoading,
    historyAvailable,
    selectedHistoryKey,
    setSelectedHistoryKey,
    historyClearing,
    confirmClearOpen,
    setConfirmClearOpen,
    suiteRounds,
    setSuiteRounds,
    crossProviderId,
    setCrossProviderId,
    hasCrossProviderTemplate,
    crossProviderOptions,
    title,
    historyGroups,
    selectedHistoryGroup,
    selectedHistoryLatest,
    activeResult,
    activeResultTemplateKey,
    currentSuiteSummary,
    historySuiteSummary,
    hasSuiteContext,
    detailsTabItems,
    suiteHeaderMetaText,
    handleOpenChange,
    refreshHistory,
    copyTextOrToast,
    runValidationSuite,
    clearProviderHistory,
  } = state;

  return (
    <Dialog
      open={open}
      onOpenChange={handleOpenChange}
      title={title}
      className="max-w-[95vw] sm:max-w-[95vw] md:max-w-[95vw] lg:max-w-[95vw] xl:max-w-[1600px] 2xl:max-w-[1800px] w-full"
    >
      {!provider ? (
        <div className="flex h-40 items-center justify-center text-sm text-muted-foreground">
          未选择服务商
        </div>
      ) : (
        <div className="space-y-6">
          {/* Provider Info Banner */}
          <div className="flex flex-wrap items-center justify-between rounded-2xl border border-border/60 dark:border-border/60 bg-white/50 dark:bg-card/30 px-5 py-4 text-sm shadow-sm backdrop-blur-md">
            <div className="flex flex-wrap items-center gap-6 text-secondary-foreground">
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br from-indigo-50 to-white dark:from-indigo-950/50 dark:to-secondary shadow-sm ring-1 ring-indigo-100 dark:ring-indigo-800/50">
                  <Server className="h-5 w-5 text-indigo-600 dark:text-indigo-400" />
                </div>
                <div>
                  <div className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
                    服务商
                  </div>
                  <div className="font-semibold text-foreground text-base">{provider.name}</div>
                </div>
              </div>
              <div className="hidden h-10 w-px bg-muted dark:bg-secondary sm:block" />
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br from-sky-50 to-white dark:from-sky-950/50 dark:to-secondary shadow-sm ring-1 ring-sky-100 dark:ring-sky-800/50">
                  <Network className="h-5 w-5 text-sky-600 dark:text-sky-400" />
                </div>
                <div>
                  <div className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
                    模式
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-foreground">
                      {provider.base_url_mode === "ping" ? "自动测速" : "顺序轮询"}
                    </span>
                    <span className="inline-flex items-center rounded-md bg-secondary/80 dark:bg-secondary px-2 py-0.5 text-xs font-medium text-muted-foreground dark:text-secondary-foreground ring-1 ring-inset ring-border">
                      {provider.base_urls.length} 个地址
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* Form Bar */}
          <div className="grid gap-5 rounded-2xl border border-border/60 dark:border-border/60 bg-secondary/40 dark:bg-secondary/40 p-5 sm:grid-cols-12 shadow-sm">
            <div className="sm:col-span-4">
              <FormField
                label="Endpoint"
                hint={provider.base_url_mode === "ping" && baseUrlPicking ? "测速中..." : null}
              >
                <Select
                  value={baseUrl}
                  onChange={(e) => setBaseUrl(e.currentTarget.value)}
                  disabled={validating}
                  mono
                  className="h-10 bg-white/80 dark:bg-card/80 text-xs shadow-sm"
                >
                  <option value="" disabled>
                    选择 Endpoint...
                  </option>
                  {provider.base_urls.map((url) => (
                    <option key={url} value={url}>
                      {url}
                    </option>
                  ))}
                </Select>
              </FormField>
            </div>

            <div className="sm:col-span-4">
              <FormField label="Model">
                <ModelCombobox value={model} onChange={setModel} disabled={validating} />
              </FormField>
            </div>

            <div className="flex items-end gap-2 sm:col-span-4">
              <FormField label="轮数" className="w-20 shrink-0">
                <input
                  type="number"
                  min={1}
                  max={99}
                  value={suiteRounds}
                  onChange={(e) => {
                    const v = parseInt(e.currentTarget.value, 10);
                    setSuiteRounds(Number.isFinite(v) && v >= 1 ? Math.min(v, 99) : 1);
                  }}
                  disabled={validating}
                  className="h-10 w-full rounded-md border border-border bg-white/80 dark:bg-card/80 px-3 text-xs font-mono text-center shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
                />
              </FormField>
              <Button
                onClick={() => void runValidationSuite()}
                variant="primary"
                size="md"
                disabled={validating}
                className="flex-1 h-10 shadow-sm"
              >
                {validating ? (
                  <>
                    <RefreshCw className="mr-2 h-3.5 w-3.5 animate-spin" />
                    {suiteProgress
                      ? suiteProgress.round > 1
                        ? `轮次 ${suiteProgress.round}/${suiteProgress.totalRounds} · 步骤 ${suiteProgress.current}/${suiteProgress.total}...`
                        : `执行中 (${suiteProgress.current}/${suiteProgress.total})...`
                      : "执行中..."}
                  </>
                ) : (
                  <>
                    <Play className="mr-2 h-3.5 w-3.5 fill-current" />
                    开始验证 ({templates.length})
                  </>
                )}
              </Button>
            </div>

            {hasCrossProviderTemplate && crossProviderOptions.length > 0 && (
              <div className="sm:col-span-12">
                <FormField
                  label="Cross-Provider Validation"
                  hint="用于 Step3 的跨供应商 Signature 验证"
                >
                  <Select
                    value={crossProviderId?.toString() ?? ""}
                    onChange={(e) => {
                      const val = e.currentTarget.value;
                      setCrossProviderId(val ? parseInt(val, 10) : null);
                    }}
                    disabled={validating}
                    className="h-10 bg-white/80 dark:bg-card/80 text-xs shadow-sm"
                  >
                    <option value="">选择官方供应商...</option>
                    {crossProviderOptions.map((p) => (
                      <option key={p.id} value={p.id.toString()}>
                        {p.name} ({p.base_urls[0] ?? "无 URL"})
                      </option>
                    ))}
                  </Select>
                </FormField>
              </div>
            )}
          </div>

          <div className="flex flex-col gap-6 lg:flex-row h-[70vh] min-h-[600px] max-h-[800px]">
            {/* Left Column: History List */}
            <HistoryListPanel
              provider={provider}
              historyAvailable={historyAvailable}
              historyLoading={historyLoading}
              historyGroups={historyGroups}
              selectedHistoryKey={selectedHistoryKey}
              historyClearing={historyClearing}
              onSelectGroup={(key) => {
                setSelectedHistoryKey(key);
                setDetailsTab("overview");
              }}
              onRefresh={() => void refreshHistory({ selectLatest: false })}
              onClear={() => setConfirmClearOpen(true)}
            />

            {/* Right Column: Details Pane */}
            <DetailsPane
              suiteSteps={suiteSteps}
              suiteProgress={suiteProgress}
              suiteIssuesOnly={suiteIssuesOnly}
              setSuiteIssuesOnly={setSuiteIssuesOnly}
              suiteActiveStepIndex={suiteActiveStepIndex}
              setSuiteActiveStepIndex={setSuiteActiveStepIndex}
              detailsTab={detailsTab}
              setDetailsTab={setDetailsTab}
              detailsTabItems={detailsTabItems}
              selectedHistoryGroup={selectedHistoryGroup}
              selectedHistoryLatest={selectedHistoryLatest}
              activeResult={activeResult}
              activeResultTemplateKey={activeResultTemplateKey}
              currentSuiteSummary={currentSuiteSummary}
              historySuiteSummary={historySuiteSummary}
              hasSuiteContext={hasSuiteContext}
              suiteHeaderMetaText={suiteHeaderMetaText}
              requestJson={requestJson}
              setRequestJson={setRequestJson}
              apiKeyPlaintext={apiKeyPlaintext}
              templates={templates}
              copyTextOrToast={copyTextOrToast}
            />
          </div>
        </div>
      )}

      {confirmClearOpen && typeof document !== "undefined"
        ? createPortal(
            <div className="fixed inset-0 z-[60] pointer-events-auto">
              <div
                className="absolute inset-0 bg-black/40"
                onClick={() => {
                  if (historyClearing) return;
                  setConfirmClearOpen(false);
                }}
              />
              <div className="absolute inset-0 flex items-center justify-center p-4">
                <div className="w-full max-w-md overflow-hidden rounded-2xl border border-border bg-white dark:bg-secondary shadow-card">
                  <div className="border-b border-border px-5 py-4">
                    <div className="text-sm font-semibold text-foreground">确认清空历史？</div>
                    <div className="mt-1 text-xs text-muted-foreground">
                      即将清空{" "}
                      <span className="font-medium text-foreground">
                        {provider?.name ?? "Provider"}
                      </span>{" "}
                      的验证历史，操作不可撤销。
                    </div>
                  </div>
                  <div className="flex items-center justify-end gap-2 px-5 py-4">
                    <Button
                      variant="secondary"
                      size="md"
                      disabled={historyClearing}
                      onClick={() => setConfirmClearOpen(false)}
                    >
                      取消
                    </Button>
                    <Button
                      variant="danger"
                      size="md"
                      disabled={historyClearing}
                      onClick={() => void clearProviderHistory()}
                    >
                      {historyClearing ? "清空中…" : "确认清空"}
                    </Button>
                  </div>
                </div>
              </div>
            </div>,
            document.body
          )
        : null}
    </Dialog>
  );
}
