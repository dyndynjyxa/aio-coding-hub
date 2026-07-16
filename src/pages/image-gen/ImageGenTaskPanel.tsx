// Usage: 生图页右栏哑组件：工具行（搜索/状态筛选）+ 任务卡片网格 + 底部输入区。
// 所有状态与逻辑来自 useImageGenController；点卡片打开任务详情弹窗（ImageGenTaskDetail，由页面挂载）。

import { useRef, useState } from "react";
import { CircleAlert, ImagePlus, X } from "lucide-react";
import { cn } from "../../utils/cn";
import { formatCountdownSeconds } from "../../utils/formatters";
import { useNowMs } from "../../hooks/useNowMs";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { EmptyState } from "../../ui/EmptyState";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { Spinner } from "../../ui/Spinner";
import { Textarea } from "../../ui/Textarea";
import { ConfirmDialog } from "../../ui/ConfirmDialog";
import { ImageGenLightbox } from "./ImageGenLightbox";
import {
  filterTasks,
  type ImageGenController,
  type ImageGenStatusFilter,
  type ImageGenTask,
} from "./useImageGenController";

export type ImageGenTaskPanelProps = {
  controller: ImageGenController;
  className?: string;
};

const STATUS_FILTER_OPTIONS: { value: ImageGenStatusFilter; label: string }[] = [
  { value: "all", label: "全部" },
  { value: "loading", label: "生成中" },
  { value: "done", label: "成功" },
  { value: "error", label: "失败" },
];

/** 逐秒计时（mm:ss，超 1h 为 h:mm:ss）。复用共享 interval bucket，卸载自动清理。 */
export function ImageGenElapsed({ startedAt }: { startedAt: number }) {
  const nowMs = useNowMs(true, 1000);
  return (
    <span className="text-xs tabular-nums text-muted-foreground">
      {formatCountdownSeconds((nowMs - startedAt) / 1000)}
    </span>
  );
}

function TaskChips({ task }: { task: ImageGenTask }) {
  return (
    <div className="flex flex-wrap gap-1.5">
      <span className="rounded bg-secondary px-1.5 py-0.5 text-xs text-muted-foreground">
        {task.request.size}
      </span>
      <span className="rounded bg-secondary px-1.5 py-0.5 text-xs text-muted-foreground">
        {task.request.options.quality}
      </span>
    </div>
  );
}

function ImageGenTaskCard({
  task,
  onOpenDetail,
  onRetry,
  onDelete,
}: {
  task: ImageGenTask;
  onOpenDetail: () => void;
  onRetry: () => void;
  onDelete: () => void;
}) {
  return (
    <article className="rounded-lg border border-line p-3">
      <button
        type="button"
        aria-label={`查看任务详情：${task.prompt.slice(0, 30)}`}
        className="flex w-full items-start gap-3 text-left"
        onClick={onOpenDetail}
      >
        {task.status === "done" && task.images.length > 0 ? (
          <div className="relative h-24 w-24 shrink-0">
            <img
              src={task.images[0].objectUrl}
              alt={`任务缩略图：${task.prompt.slice(0, 30)}`}
              className="h-24 w-24 rounded object-cover"
            />
            {task.images.length > 1 ? (
              <span className="absolute right-1 top-1 rounded bg-black/60 px-1 text-xs tabular-nums text-white">
                +{task.images.length - 1}
              </span>
            ) : null}
          </div>
        ) : (
          <div className="flex h-24 w-24 shrink-0 flex-col items-center justify-center gap-1 rounded bg-muted">
            {task.status === "loading" ? (
              <>
                <Spinner size="sm" />
                <ImageGenElapsed startedAt={task.startedAt} />
              </>
            ) : (
              <CircleAlert className="h-6 w-6 text-destructive" />
            )}
          </div>
        )}
        <div className="min-w-0 flex-1 space-y-1.5">
          <div className="line-clamp-2 break-words text-sm text-foreground">{task.prompt}</div>
          <TaskChips task={task} />
          {task.status === "error" ? (
            <div className="truncate text-xs text-destructive">{task.error ?? "生成失败"}</div>
          ) : null}
          {task.status === "done" && task.elapsedMs != null ? (
            <div className="text-xs tabular-nums text-muted-foreground">
              耗时 {formatCountdownSeconds(task.elapsedMs / 1000)}
            </div>
          ) : null}
        </div>
      </button>
      {task.status === "error" ? (
        <div className="mt-2 flex gap-2">
          <Button size="sm" onClick={onRetry}>
            重试
          </Button>
          <Button size="sm" variant="danger" onClick={onDelete}>
            删除
          </Button>
        </div>
      ) : null}
    </article>
  );
}

export function ImageGenTaskPanel({ controller, className }: ImageGenTaskPanelProps) {
  const {
    tasks,
    searchQuery,
    setSearchQuery,
    statusFilter,
    setStatusFilter,
    openDetail,
    prompt,
    setPrompt,
    referenceImages,
    addReferenceFiles,
    removeReferenceImage,
    submit,
    retry,
    deleteTask,
    clearTasks,
    preview,
    closePreview,
    stepPreview,
  } = controller;
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  // 拖拽悬停高亮 / 清空任务二次确认：纯视图局部态。
  const [isDragOver, setIsDragOver] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);

  const visibleTasks = filterTasks(tasks, searchQuery, statusFilter);

  return (
    <Card padding="sm" className={className}>
      <div className="flex flex-col gap-4">
        {tasks.length > 0 || searchQuery ? (
          <div className="flex items-center gap-2">
            <Input
              placeholder="搜索提示词…"
              aria-label="搜索提示词"
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
            />
            <Select
              aria-label="状态筛选"
              className="w-28 shrink-0"
              value={statusFilter}
              onChange={(event) => setStatusFilter(event.target.value as ImageGenStatusFilter)}
            >
              {STATUS_FILTER_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </Select>
            <Button
              size="sm"
              variant="danger"
              className="shrink-0"
              disabled={tasks.length === 0}
              onClick={() => setConfirmClear(true)}
            >
              清空任务
            </Button>
          </div>
        ) : null}

        {tasks.length === 0 ? (
          <EmptyState
            variant="dashed"
            title="还没有生成记录"
            description="在下方输入提示词开始生成图片"
          />
        ) : visibleTasks.length === 0 ? (
          <EmptyState variant="dashed" title="没有匹配的任务" />
        ) : (
          <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
            {visibleTasks.map((task) => (
              <ImageGenTaskCard
                key={task.id}
                task={task}
                onOpenDetail={() => openDetail(task.id)}
                onRetry={() => {
                  void retry(task.id);
                }}
                onDelete={() => deleteTask(task.id)}
              />
            ))}
          </div>
        )}

        <div
          data-testid="image-gen-drop-zone"
          className={cn(
            "space-y-2 border-t border-line pt-3",
            isDragOver && "rounded-lg ring-1 ring-primary"
          )}
          onDragOver={(event) => {
            event.preventDefault();
            if (event.dataTransfer.types.includes("Files")) setIsDragOver(true);
          }}
          onDragLeave={() => setIsDragOver(false)}
          onDrop={(event) => {
            event.preventDefault();
            setIsDragOver(false);
            const files = Array.from(event.dataTransfer.files).filter((file) =>
              file.type.startsWith("image/")
            );
            if (files.length > 0) void addReferenceFiles(files);
          }}
        >
          {referenceImages.length > 0 ? (
            <div className="flex flex-wrap gap-2">
              {referenceImages.map((image, index) => (
                <div key={image.id} className="relative">
                  <img
                    src={image.objectUrl}
                    alt={`参考图 ${index + 1}`}
                    className="h-14 w-14 rounded-md border border-line object-cover"
                  />
                  <button
                    type="button"
                    aria-label={`移除参考图 ${index + 1}`}
                    className="absolute -right-1.5 -top-1.5 rounded-full border border-line bg-surface-panel p-0.5 text-muted-foreground hover:text-foreground"
                    onClick={() => removeReferenceImage(image.id)}
                  >
                    <X className="h-3 w-3" />
                  </button>
                </div>
              ))}
            </div>
          ) : null}
          <Textarea
            rows={3}
            placeholder="描述你想生成的图片…"
            aria-label="提示词"
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
          />
          <div className="flex items-center justify-between gap-2">
            <input
              ref={fileInputRef}
              type="file"
              accept="image/*"
              multiple
              className="hidden"
              aria-label="上传参考图"
              onChange={(event) => {
                if (event.target.files) {
                  void addReferenceFiles(event.target.files);
                }
                event.target.value = "";
              }}
            />
            <Button size="sm" onClick={() => fileInputRef.current?.click()}>
              <ImagePlus className="h-3.5 w-3.5" />
              参考图
            </Button>
            <Button
              variant="primary"
              disabled={!prompt.trim()}
              onClick={() => {
                void submit();
              }}
            >
              生成
            </Button>
          </div>
        </div>
      </div>
      <ImageGenLightbox preview={preview} onClose={closePreview} onStep={stepPreview} />
      <ConfirmDialog
        open={confirmClear}
        title="清空任务"
        description="将删除全部任务及生成图片（会话仅保存在内存中，不可恢复）。"
        onClose={() => setConfirmClear(false)}
        onConfirm={() => {
          setConfirmClear(false);
          clearTasks();
        }}
        confirmLabel="清空"
        confirmingLabel="清空中…"
        confirming={false}
        confirmVariant="danger"
      />
    </Card>
  );
}
