// Usage: 生图页右栏哑组件：消息流（用户/助手）+ 底部输入区。所有状态与逻辑来自 useImageGenController。

import { useRef } from "react";
import { Download, ImagePlus, X } from "lucide-react";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { EmptyState } from "../../ui/EmptyState";
import { Spinner } from "../../ui/Spinner";
import { Textarea } from "../../ui/Textarea";
import { ImageGenLightbox } from "./ImageGenLightbox";
import type { ImageGenUsage } from "../../services/image-gen/types";
import type {
  ImageGenAssistantMessage,
  ImageGenController,
  ImageGenGeneratedImage,
} from "./useImageGenController";

export type ImageGenConversationProps = {
  controller: ImageGenController;
  className?: string;
};

function formatUsage(usage: ImageGenUsage): string {
  const parts: string[] = [];
  if (usage.inputTokens != null) parts.push(`输入 ${usage.inputTokens}`);
  if (usage.outputTokens != null) parts.push(`输出 ${usage.outputTokens}`);
  if (usage.totalTokens != null) parts.push(`合计 ${usage.totalTokens}`);
  return `tokens：${parts.join(" · ")}`;
}

function AssistantMessageView({
  message,
  onRetry,
  onDownload,
  onUseAsReference,
  onPreview,
}: {
  message: ImageGenAssistantMessage;
  onRetry: (id: string) => void;
  onDownload: (image: ImageGenGeneratedImage) => void;
  onUseAsReference: (image: ImageGenGeneratedImage) => void;
  onPreview: (index: number) => void;
}) {
  if (message.status === "loading") {
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        <Spinner size="sm" />
        生成中…
      </div>
    );
  }

  if (message.status === "error") {
    return (
      <div className="space-y-2">
        <div className="break-words text-sm text-destructive">{message.error ?? "生成失败"}</div>
        <Button size="sm" onClick={() => onRetry(message.id)}>
          重试
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        {message.images.map((image, index) => (
          <div key={image.objectUrl} className="space-y-1.5">
            <button
              type="button"
              aria-label={`预览生成图片 ${index + 1}`}
              className="block w-full cursor-zoom-in"
              onClick={() => onPreview(index)}
            >
              <img
                src={image.objectUrl}
                alt={`生成图片 ${index + 1}`}
                className="w-full rounded-lg border border-line"
              />
            </button>
            <div className="flex flex-wrap gap-2">
              <Button size="sm" onClick={() => onDownload(image)}>
                <Download className="h-3.5 w-3.5" />
                下载
              </Button>
              <Button size="sm" onClick={() => onUseAsReference(image)}>
                设为参考图
              </Button>
            </div>
          </div>
        ))}
      </div>
      {message.usage ? (
        <div className="text-xs text-muted-foreground">{formatUsage(message.usage)}</div>
      ) : null}
    </div>
  );
}

export function ImageGenConversation({ controller, className }: ImageGenConversationProps) {
  const {
    messages,
    prompt,
    setPrompt,
    referenceImages,
    addReferenceFiles,
    removeReferenceImage,
    submit,
    retry,
    downloadImage,
    setAsReference,
    preview,
    openPreview,
    closePreview,
    stepPreview,
  } = controller;
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  return (
    <Card padding="sm" className={className}>
      <div className="flex flex-col gap-4">
        <div className="space-y-4">
          {messages.length === 0 ? (
            <EmptyState
              variant="dashed"
              title="还没有生成记录"
              description="在下方输入提示词开始生成图片"
            />
          ) : (
            messages.map((message) =>
              message.role === "user" ? (
                <div key={message.id} className="rounded-lg bg-secondary px-3 py-2">
                  <div className="break-words text-sm text-foreground">{message.prompt}</div>
                  {message.refThumbs.length > 0 ? (
                    <div className="mt-2 flex flex-wrap gap-2">
                      {message.refThumbs.map((thumb, index) => (
                        <img
                          key={thumb}
                          src={thumb}
                          alt={`参考图 ${index + 1}`}
                          className="h-12 w-12 rounded-md border border-line object-cover"
                        />
                      ))}
                    </div>
                  ) : null}
                </div>
              ) : (
                <div key={message.id} className="px-1">
                  <AssistantMessageView
                    message={message}
                    onRetry={(id) => {
                      void retry(id);
                    }}
                    onDownload={(image) => {
                      void downloadImage(image);
                    }}
                    onUseAsReference={(image) => {
                      void setAsReference(image);
                    }}
                    onPreview={(index) => {
                      openPreview(
                        message.images.map((image) => image.objectUrl),
                        index
                      );
                    }}
                  />
                </div>
              )
            )
          )}
        </div>

        <div className="space-y-2 border-t border-line pt-3">
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
    </Card>
  );
}
