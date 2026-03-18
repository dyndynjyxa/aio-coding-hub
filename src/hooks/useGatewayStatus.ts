import { openUrl } from "@tauri-apps/plugin-opener";
import { AIO_RELEASES_URL } from "../constants/urls";
import { useGatewayMeta } from "./useGatewayMeta";
import { useUpdateMeta } from "./useUpdateMeta";

/**
 * Derived gateway status for UI display.
 * Consolidates logic shared between Sidebar and MobileNav.
 */
export function useGatewayStatus() {
  const { gatewayAvailable, gateway, preferredPort } = useGatewayMeta();
  const updateMeta = useUpdateMeta();
  const hasUpdate = !!updateMeta.updateCandidate;
  const isPortable = updateMeta.about?.run_mode === "portable";

  const statusText =
    gatewayAvailable === "checking"
      ? "检查中"
      : gatewayAvailable === "unavailable"
        ? "不可用"
        : gateway == null
          ? "未知"
          : gateway.running
            ? "运行中"
            : "已停止";

  const statusTone =
    gatewayAvailable === "available" && gateway?.running
      ? "bg-emerald-50 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400"
      : "bg-slate-100 text-slate-600 dark:bg-slate-700 dark:text-slate-300";

  const portTone =
    gatewayAvailable === "available" && gateway?.running
      ? "bg-emerald-50 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-300"
      : "bg-slate-100 text-slate-600 dark:bg-slate-700 dark:text-slate-300";

  const portText = gatewayAvailable === "available" ? String(gateway?.port ?? preferredPort) : "—";

  const isGatewayRunning = gatewayAvailable === "available" && gateway?.running;

  return {
    statusText,
    statusTone,
    portTone,
    portText,
    isGatewayRunning,
    hasUpdate,
    isPortable,
    updateMeta,
  };
}

/** Open the AIO releases page via Tauri opener, fallback to window.open. */
export async function openReleasesUrl(): Promise<void> {
  try {
    await openUrl(AIO_RELEASES_URL);
  } catch {
    try {
      window.open(AIO_RELEASES_URL, "_blank", "noopener,noreferrer");
    } catch {
      /* best-effort */
    }
  }
}
