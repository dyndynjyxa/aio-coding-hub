// Usage: 生图任务详情弹窗（哑组件，由页面挂载）。左侧大图（n>1 缩略图切换、点击开灯箱），
// 右侧输入内容/参数快照/创建时间与耗时/tokens 与预估费用；操作：复用配置（输入区非空时二次确认）/
// 下载/设为参考图/删除（二次确认）。

import { useState } from "react";
import { toast } from "sonner";
import { copyText } from "../../services/clipboard";
import { estimateGptImageCostUsd } from "../../services/image-gen/gptImageAdapter";
import type { ImageGenUsage } from "../../services/image-gen/types";
import { cn } from "../../utils/cn";
import { formatDurationMs, formatUsdCompact } from "../../utils/formatters";
import { Button } from "../../ui/Button";
import { ConfirmDialog } from "../../ui/ConfirmDialog";
import { Dialog } from "../../ui/Dialog";
import { Spinner } from "../../ui/Spinner";
import { ImageGenElapsed, ImageGenImage } from "./ImageGenTaskPanel";
import { taskImageSrc, taskImageThumbSrc } from "./imageGenPersistence";
import type { ImageGenController, ImageGenTask } from "./useImageGenController";

export type ImageGenTaskDetailProps = {
  controller: ImageGenController;
};

function formatUsageLine(usage: ImageGenUsage): string {
  const parts: string[] = [];
  if (usage.inputTokens != null) {
    const details = usage.inputTokensDetails;
    const suffix =
      details && (details.textTokens != null || details.imageTokens != null)
        ? `（文本 ${details.textTokens ?? 0} / 图像 ${details.imageTokens ?? 0}）`
        : "";
    parts.push(`输入 ${usage.inputTokens}${suffix}`);
  }
  if (usage.outputTokens != null) parts.push(`输出 ${usage.outputTokens}`);
  if (usage.totalTokens != null) parts.push(`合计 ${usage.totalTokens}`);
  return `tokens：${parts.join(" · ")}`;
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
      {children}
    </h3>
  );
}

export function ImageGenTaskDetail({ controller }: ImageGenTaskDetailProps) {
  const { detailTask } = controller;
  // 详情打开期间任务被删除（其他入口删除）时不再渲染。
  if (!detailTask) return null;
  // key 按任务重置内部局部态（当前图下标、删除确认）。
  return <TaskDetailContent key={detailTask.id} task={detailTask} controller={controller} />;
}

function TaskDetailContent({
  task,
  controller,
}: {
  task: ImageGenTask;
  controller: ImageGenController;
}) {
  const {
    closeDetail,
    reuseTask,
    deleteTask,
    downloadImage,
    setAsReference,
    openPreview,
    prompt,
    referenceImages,
  } = controller;
  const [imageIndex, setImageIndex] = useState(0);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [confirmReuse, setConfirmReuse] = useState(false);
  // 输入区有内容（提示词草稿或参考图）时，复用配置需二次确认防覆盖。
  const inputDirty = prompt.trim() !== "" || referenceImages.length > 0;
  // 任务在弹窗打开期间完成会追加图片：下标越界时收敛到最后一张。
  const safeIndex = Math.min(imageIndex, Math.max(0, task.images.length - 1));
  const currentImage = task.images[safeIndex] ?? null;
  const { request } = task;
  const cost = estimateGptImageCostUsd(task.usage);

  const paramRows: [string, string][] = [
    ["模型", request.options.model],
    ["尺寸", request.size],
    ["质量", request.options.quality],
    ["格式", request.options.outputFormat],
    ...(request.options.outputFormat !== "png"
      ? ([
          [
            "压缩率",
            request.options.outputCompression != null
              ? String(request.options.outputCompression)
              : "—",
          ],
        ] as [string, string][])
      : []),
    ["审核", request.options.moderation],
    ["数量", String(request.n)],
  ];

  return (
    <Dialog
      open
      title="任务详情"
      className="max-w-3xl"
      onOpenChange={(open) => {
        if (!open) closeDetail();
      }}
    >
      <div className="grid gap-4 md:grid-cols-2">
        <div className="space-y-2">
          {task.status === "loading" ? (
            <div className="flex h-64 flex-col items-center justify-center gap-2 rounded-lg bg-muted">
              <Spinner size="sm" />
              <ImageGenElapsed startedAt={task.startedAt} />
            </div>
          ) : task.status === "error" ? (
            <div className="break-words rounded-lg bg-muted p-3 text-sm text-destructive">
              {task.error ?? "生成失败"}
            </div>
          ) : currentImage ? (
            <>
              <button
                type="button"
                aria-label="预览大图"
                className="block w-full cursor-zoom-in"
                onClick={() =>
                  openPreview(
                    task.images.map((image) => taskImageSrc(image)),
                    safeIndex
                  )
                }
              >
                <ImageGenImage
                  key={taskImageSrc(currentImage)}
                  src={taskImageSrc(currentImage)}
                  alt={`生成图片 ${safeIndex + 1}`}
                  className="max-h-[55vh] w-full rounded-lg border border-line object-contain"
                />
              </button>
              {task.images.length > 1 ? (
                <div className="flex flex-wrap gap-2">
                  {task.images.map((image, index) => (
                    <button
                      key={taskImageSrc(image)}
                      type="button"
                      aria-label={`切换到第 ${index + 1} 张`}
                      className={cn(
                        "rounded-md border",
                        index === safeIndex ? "border-primary" : "border-line"
                      )}
                      onClick={() => setImageIndex(index)}
                    >
                      <img
                        src={taskImageThumbSrc(image)}
                        alt={`缩略图 ${index + 1}`}
                        className="h-14 w-14 rounded-md object-cover"
                      />
                    </button>
                  ))}
                </div>
              ) : null}
            </>
          ) : null}
        </div>

        <div className="space-y-3">
          <section className="space-y-1">
            <div className="flex items-center justify-between gap-2">
              <SectionTitle>输入内容</SectionTitle>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => {
                  void copyText(task.prompt).then(() => toast.success("已复制提示词"));
                }}
              >
                复制
              </Button>
            </div>
            <div className="break-words text-sm text-foreground">{task.prompt}</div>
          </section>

          {task.refThumbs.length > 0 ? (
            <section className="space-y-1">
              <SectionTitle>参考图</SectionTitle>
              <div className="flex flex-wrap gap-2">
                {task.refThumbs.map((thumb, index) => (
                  <img
                    key={thumb}
                    src={thumb}
                    alt={`参考图 ${index + 1}`}
                    className="h-12 w-12 rounded-md border border-line object-cover"
                  />
                ))}
              </div>
            </section>
          ) : null}

          <section className="space-y-1">
            <SectionTitle>参数配置</SectionTitle>
            <dl className="grid grid-cols-2 gap-x-4 gap-y-1">
              {paramRows.map(([label, value]) => (
                <div key={label} className="flex items-baseline justify-between gap-2">
                  <dt className="text-xs text-muted-foreground">{label}</dt>
                  <dd className="truncate text-xs text-foreground">{value}</dd>
                </div>
              ))}
            </dl>
          </section>

          <div className="text-xs text-muted-foreground">
            创建于 {new Date(task.createdAt).toLocaleString()}
            {task.elapsedMs != null ? ` · 耗时 ${formatDurationMs(task.elapsedMs)}` : ""}
          </div>

          {task.usage ? (
            <div className="space-y-0.5 text-xs text-muted-foreground">
              <div>{formatUsageLine(task.usage)}</div>
              {cost != null ? <div>预估 {formatUsdCompact(cost)}</div> : null}
            </div>
          ) : null}

          <div className="flex flex-wrap gap-2 pt-1">
            <Button
              size="sm"
              onClick={() => {
                if (inputDirty) {
                  setConfirmReuse(true);
                  return;
                }
                void reuseTask(task.id);
                closeDetail();
              }}
            >
              复用配置
            </Button>
            <Button
              size="sm"
              disabled={!currentImage}
              onClick={() => {
                if (currentImage) void downloadImage(currentImage);
              }}
            >
              下载
            </Button>
            <Button
              size="sm"
              disabled={!currentImage}
              onClick={() => {
                if (currentImage) void setAsReference(currentImage);
              }}
            >
              设为参考图
            </Button>
            <Button size="sm" variant="danger" onClick={() => setConfirmDelete(true)}>
              删除任务
            </Button>
          </div>
        </div>
      </div>

      <ConfirmDialog
        open={confirmReuse}
        title="覆盖当前输入？"
        description="复用配置将替换输入区当前的提示词与参考图。"
        onClose={() => setConfirmReuse(false)}
        onConfirm={() => {
          setConfirmReuse(false);
          void reuseTask(task.id);
          closeDetail();
        }}
        confirmLabel="覆盖"
        confirmingLabel="覆盖中…"
        confirming={false}
      />

      <ConfirmDialog
        open={confirmDelete}
        title="删除任务"
        description="将同时删除本地图片文件，删除后不可恢复。"
        onClose={() => setConfirmDelete(false)}
        onConfirm={() => {
          setConfirmDelete(false);
          deleteTask(task.id);
          closeDetail();
        }}
        confirmLabel="删除"
        confirmingLabel="删除中…"
        confirming={false}
        confirmVariant="danger"
      />
    </Dialog>
  );
}
