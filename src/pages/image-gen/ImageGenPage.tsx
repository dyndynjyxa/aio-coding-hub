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
      {/* lg 起双栏各自限高滚动：任务区独立滚动、输入栏钉底；<lg 塌单列回退页面级滚动。 */}
      <div className="min-h-0 flex-1 overflow-y-auto scrollbar-overlay lg:overflow-hidden">
        <div className="grid grid-cols-1 gap-6 lg:h-full lg:grid-cols-12">
          <ImageGenParamsPanel
            controller={controller}
            className="scrollbar-overlay lg:col-span-4 lg:min-h-0 lg:overflow-y-auto"
          />
          <ImageGenTaskPanel
            controller={controller}
            className="order-first lg:order-none lg:col-span-8 lg:min-h-0"
          />
        </div>
      </div>
      <ImageGenTaskDetail controller={controller} />
    </div>
  );
}
