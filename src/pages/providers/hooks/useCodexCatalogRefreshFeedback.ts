import { useEffect } from "react";
import { toast } from "sonner";
import { logToConsole } from "../../../services/consoleLog";
import { listenProviderCodexCatalogEvents } from "../../../services/providers/providerEvents";

export function useCodexCatalogRefreshFeedback() {
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;

    void listenProviderCodexCatalogEvents((payload) => {
      if (payload.status === "updated") {
        toast("模型映射已更新，重启 Codex 后生效");
        return;
      }
      toast("Codex 模型目录更新失败，请检查 Codex CLI、目录文件和权限后重试");
    })
      .then((cleanup) => {
        if (disposed) {
          cleanup();
        } else {
          unlisten = cleanup;
        }
      })
      .catch((error) => {
        logToConsole("warn", "监听 Codex 模型目录状态失败", { error: String(error) });
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
}
