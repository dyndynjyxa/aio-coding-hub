// Usage: 生图页左栏哑组件：连接配置卡（输入框 blur 自动保存）+ 生成参数卡 + 存储管理卡。
// 所有状态与逻辑来自 useImageGenController。

import { useState } from "react";
import { formatBytes } from "../../utils/formatters";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { ConfirmDialog } from "../../ui/ConfirmDialog";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { CLEANUP_KEEP_COUNT, type ImageGenController } from "./useImageGenController";

// 官方支持的预设：auto + 三档官方尺寸（非官方尺寸会被上游 400 拒绝）。
const SIZE_OPTIONS = ["auto", "1024x1024", "1536x1024", "1024x1536"] as const;

export type ImageGenParamsPanelProps = {
  controller: ImageGenController;
  className?: string;
};

export function ImageGenParamsPanel({ controller, className }: ImageGenParamsPanelProps) {
  const {
    baseUrl,
    setBaseUrl,
    model,
    setModel,
    apiKeyDraft,
    setApiKeyDraft,
    apiKeyConfigured,
    requestUrlPreview,
    autoSaveConfig,
    params,
    updateParams,
    storage,
    changeStorageDir,
    cleanupStorage,
  } = controller;

  const compressionEnabled = params.outputFormat !== "png";
  // 清理二次确认：纯视图局部态。
  const [confirmCleanup, setConfirmCleanup] = useState(false);

  return (
    <div className={className}>
      <div className="space-y-6">
        <Card padding="sm">
          <h2 className="mb-3 text-sm font-semibold text-foreground">连接配置</h2>
          <div className="space-y-3">
            <FormField label="Base URL" hint="失焦自动保存">
              {(id) => (
                <Input
                  id={id}
                  mono
                  placeholder="https://api.example.com"
                  value={baseUrl}
                  onChange={(event) => setBaseUrl(event.target.value)}
                  onBlur={() => {
                    void autoSaveConfig();
                  }}
                />
              )}
            </FormField>
            <FormField label="API Key" hint={apiKeyConfigured ? "已配置" : "未配置"}>
              {(id) => (
                <Input
                  id={id}
                  type="password"
                  mono
                  placeholder={apiKeyConfigured ? "已配置（输入新值可替换）" : "请输入 API Key"}
                  value={apiKeyDraft}
                  onChange={(event) => setApiKeyDraft(event.target.value)}
                  onBlur={() => {
                    void autoSaveConfig();
                  }}
                />
              )}
            </FormField>
            <FormField label="模型">
              {(id) => (
                <Input
                  id={id}
                  mono
                  placeholder="gpt-image-2"
                  value={model}
                  onChange={(event) => setModel(event.target.value)}
                  onBlur={() => {
                    void autoSaveConfig();
                  }}
                />
              )}
            </FormField>
            {requestUrlPreview ? (
              <div className="break-all text-xs text-muted-foreground">
                请求 URL：{requestUrlPreview}
              </div>
            ) : null}
          </div>
        </Card>

        <Card padding="sm">
          <h2 className="mb-3 text-sm font-semibold text-foreground">生成参数</h2>
          <div className="space-y-3">
            <FormField label="尺寸">
              {(id) => (
                <Select
                  id={id}
                  value={params.size}
                  onChange={(event) => updateParams({ size: event.target.value })}
                >
                  {SIZE_OPTIONS.map((size) => (
                    <option key={size} value={size}>
                      {size}
                    </option>
                  ))}
                </Select>
              )}
            </FormField>
            <FormField label="质量">
              {(id) => (
                <Select
                  id={id}
                  value={params.quality}
                  onChange={(event) =>
                    updateParams({ quality: event.target.value as typeof params.quality })
                  }
                >
                  <option value="auto">auto</option>
                  <option value="low">low</option>
                  <option value="medium">medium</option>
                  <option value="high">high</option>
                </Select>
              )}
            </FormField>
            <FormField label="格式">
              {(id) => (
                <Select
                  id={id}
                  value={params.outputFormat}
                  onChange={(event) =>
                    updateParams({
                      outputFormat: event.target.value as typeof params.outputFormat,
                    })
                  }
                >
                  <option value="png">PNG</option>
                  <option value="jpeg">JPEG</option>
                  <option value="webp">WebP</option>
                </Select>
              )}
            </FormField>
            <FormField label="压缩率" hint={compressionEnabled ? "0-100" : "仅 JPEG/WebP 可用"}>
              {(id) => (
                <Input
                  id={id}
                  type="number"
                  min={0}
                  max={100}
                  disabled={!compressionEnabled}
                  value={params.outputCompression ?? ""}
                  onChange={(event) => {
                    const raw = event.target.value;
                    if (raw === "") {
                      updateParams({ outputCompression: null });
                      return;
                    }
                    const value = Math.min(100, Math.max(0, Number(raw)));
                    updateParams({ outputCompression: Number.isNaN(value) ? null : value });
                  }}
                />
              )}
            </FormField>
            <FormField label="审核">
              {(id) => (
                <Select
                  id={id}
                  value={params.moderation}
                  onChange={(event) =>
                    updateParams({ moderation: event.target.value as typeof params.moderation })
                  }
                >
                  <option value="auto">auto</option>
                  <option value="low">low</option>
                </Select>
              )}
            </FormField>
            <FormField label="数量" hint="1-10">
              {(id) => (
                <Input
                  id={id}
                  type="number"
                  min={1}
                  max={10}
                  value={params.n}
                  onChange={(event) => {
                    const value = Math.min(10, Math.max(1, Number(event.target.value)));
                    updateParams({ n: Number.isNaN(value) ? 1 : value });
                  }}
                />
              )}
            </FormField>
          </div>
        </Card>

        <Card padding="sm">
          <h2 className="mb-3 text-sm font-semibold text-foreground">存储</h2>
          <div className="space-y-3">
            <div className="space-y-1">
              <div className="text-xs text-muted-foreground">图片目录</div>
              <div
                className="truncate font-mono text-xs text-foreground"
                title={storage?.dir ?? undefined}
              >
                {storage?.dir ?? "—"}
              </div>
            </div>
            <div className="text-xs tabular-nums text-muted-foreground">
              占用 {formatBytes(storage?.totalBytes ?? null)} · {storage?.taskCount ?? 0} 条任务
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                size="sm"
                onClick={() => {
                  void changeStorageDir();
                }}
              >
                更改目录
              </Button>
              <Button size="sm" variant="danger" onClick={() => setConfirmCleanup(true)}>
                清理
              </Button>
            </div>
          </div>
        </Card>
      </div>
      <ConfirmDialog
        open={confirmCleanup}
        title="清理历史任务"
        description={`将保留最近 ${CLEANUP_KEEP_COUNT} 条任务，更早的任务记录与图片文件将被删除，不可恢复。`}
        onClose={() => setConfirmCleanup(false)}
        onConfirm={() => {
          setConfirmCleanup(false);
          void cleanupStorage();
        }}
        confirmLabel="清理"
        confirmingLabel="清理中…"
        confirming={false}
        confirmVariant="danger"
      />
    </div>
  );
}
