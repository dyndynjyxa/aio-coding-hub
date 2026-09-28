// Usage: 生图页图片点击预览灯箱（哑组件）。Radix Dialog 原语自带 Esc 关闭、遮罩语义与焦点圈；←/→ 切换同组多图。

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { ChevronLeft, ChevronRight } from "lucide-react";
import type { ImageGenPreview } from "./useImageGenController";

export type ImageGenLightboxProps = {
  preview: ImageGenPreview | null;
  onClose: () => void;
  onStep: (delta: number) => void;
};

export function ImageGenLightbox({ preview, onClose, onStep }: ImageGenLightboxProps) {
  if (!preview) return null;
  const { urls, index } = preview;

  return (
    <DialogPrimitive.Root
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-50 bg-black/80" />
        <DialogPrimitive.Content
          aria-describedby={undefined}
          className="fixed inset-0 z-50 flex flex-col items-center justify-center gap-3 p-6 outline-none"
          onClick={onClose}
          onKeyDown={(event) => {
            if (event.key === "ArrowLeft") onStep(-1);
            if (event.key === "ArrowRight") onStep(1);
          }}
        >
          <DialogPrimitive.Title className="sr-only">图片预览</DialogPrimitive.Title>
          <img
            src={urls[index]}
            alt={`预览图片 ${index + 1}`}
            className="max-h-[85vh] max-w-full rounded-lg object-contain"
            onClick={(event) => event.stopPropagation()}
          />
          {urls.length > 1 ? (
            <div
              className="flex items-center gap-3 rounded-full bg-black/60 px-3 py-1.5"
              onClick={(event) => event.stopPropagation()}
            >
              <button
                type="button"
                aria-label="上一张"
                className="text-white/80 hover:text-white"
                onClick={() => onStep(-1)}
              >
                <ChevronLeft className="h-5 w-5" />
              </button>
              <span className="text-sm tabular-nums text-white/80">
                {index + 1} / {urls.length}
              </span>
              <button
                type="button"
                aria-label="下一张"
                className="text-white/80 hover:text-white"
                onClick={() => onStep(1)}
              >
                <ChevronRight className="h-5 w-5" />
              </button>
            </div>
          ) : null}
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
