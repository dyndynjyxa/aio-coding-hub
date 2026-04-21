import { toast } from "sonner";
import { copyText } from "../../services/clipboard";
import { logToConsole } from "../../services/consoleLog";
import { providerCopyApiKeyToClipboard } from "../../services/providers/providers";
import type { CopyApiKeyActionContext } from "./providerEditorActionContext";

export async function copyApiKey(ctx: CopyApiKeyActionContext) {
  const draftValue = ctx.apiKeyValue.trim();
  if (draftValue) {
    try {
      await copyText(draftValue);
      toast("已复制草稿 API Key");
    } catch {
      toast("复制 API Key 失败");
    }
    return;
  }

  if (ctx.mode !== "edit" || !ctx.editProvider || !ctx.apiKeyConfigured) {
    toast("暂无可复制的 API Key");
    return;
  }

  if (ctx.copyingApiKey) return;

  ctx.setCopyingApiKey(true);
  try {
    const copied = await providerCopyApiKeyToClipboard(ctx.editProvider.id);
    if (!copied) {
      toast("复制 API Key 失败");
      return;
    }
    toast("已复制已保存的 API Key");
  } catch (err) {
    logToConsole("error", "复制已保存 API Key 失败", {
      provider_id: ctx.editProvider.id,
      cli_key: ctx.editProvider.cli_key,
      error: String(err),
    });
    toast(`复制 API Key 失败：${String(err)}`);
  } finally {
    ctx.setCopyingApiKey(false);
  }
}
