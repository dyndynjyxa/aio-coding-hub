// Usage: 生图页面。左栏连接/生成参数配置，右栏任务网格 + 详情弹窗；网络请求经 Rust image_gen_* 命令代理（CSP 约束）。

import { PageHeader } from "../../ui/PageHeader";
import { ImageGenParamsPanel } from "./ImageGenParamsPanel";
import { ImageGenTaskDetail } from "./ImageGenTaskDetail";
import { ImageGenTaskPanel } from "./ImageGenTaskPanel";
import { useImageGenController } from "./useImageGenController";

export function ImageGenPage() {
  const controller = useImageGenController();

  return (
    <div className="flex h-full flex-col gap-6 overflow-hidden">
      <PageHeader title="生图" subtitle="基于 OpenAI 兼容图像接口生成与编辑图片" />
      <div className="min-h-0 flex-1 overflow-y-auto scrollbar-overlay">
        <div className="grid grid-cols-1 gap-6 lg:grid-cols-12 lg:items-start">
          <ImageGenParamsPanel controller={controller} className="lg:col-span-4" />
          <ImageGenTaskPanel
            controller={controller}
            className="order-first lg:order-none lg:col-span-8"
          />
        </div>
      </div>
      <ImageGenTaskDetail controller={controller} />
    </div>
  );
}
