import { useMemo, useSyncExternalStore } from "react";
import { toast } from "sonner";
import { queryClient } from "../query/queryClient";
import { updaterKeys } from "../query/keys";
import { useAppAboutQuery } from "../query/appAbout";
import { useUpdaterCheckQuery } from "../query/updater";
import { logToConsole } from "../services/consoleLog";
import {
  updaterCheck,
  updaterDownloadAndInstall,
  type UpdaterCheckUpdate,
  type UpdaterDownloadEvent,
} from "../services/updater";
import type { AppAboutInfo } from "../services/appAbout";

const STORAGE_KEY_LAST_CHECKED_AT_MS = "updater.lastCheckedAtMs";

export type UpdateMeta = {
  about: AppAboutInfo | null;
  updateCandidate: UpdaterCheckUpdate | null;
  checkingUpdate: boolean;
  dialogOpen: boolean;

  installingUpdate: boolean;
  installError: string | null;
  installTotalBytes: number | null;
  installDownloadedBytes: number;
};

type Listener = () => void;

type UpdateUiState = Pick<
  UpdateMeta,
  | "dialogOpen"
  | "installingUpdate"
  | "installError"
  | "installTotalBytes"
  | "installDownloadedBytes"
>;

let uiSnapshot: UpdateUiState = {
  dialogOpen: false,
  installingUpdate: false,
  installError: null,
  installTotalBytes: null,
  installDownloadedBytes: 0,
};

const listeners = new Set<Listener>();

let started = false;
let starting: Promise<void> | null = null;
let checkingPromise: Promise<UpdaterCheckUpdate | null> | null = null;
let installingPromise: Promise<boolean | null> | null = null;

function emit() {
  for (const listener of listeners) listener();
}

function setUiSnapshot(patch: Partial<UpdateUiState>) {
  uiSnapshot = { ...uiSnapshot, ...patch };
  emit();
}

function writeLastCheckedAtMs(ms: number) {
  try {
    localStorage.setItem(STORAGE_KEY_LAST_CHECKED_AT_MS, String(ms));
  } catch {}
}

async function ensureStarted() {
  if (started) return;
  if (starting) return starting;

  starting = (async () => {
    started = true;
    starting = null;
  })();

  return starting;
}

export async function updateCheckNow(options: {
  silent: boolean;
  openDialogIfUpdate: boolean;
}): Promise<UpdaterCheckUpdate | null> {
  await ensureStarted();

  if (checkingPromise) return checkingPromise;

  checkingPromise = (async () => {
    try {
      const update = await queryClient.fetchQuery({
        queryKey: updaterKeys.check(),
        queryFn: () => updaterCheck(),
        staleTime: 0,
      });

      writeLastCheckedAtMs(Date.now());

      if (update && options.openDialogIfUpdate) {
        setUiSnapshot({
          dialogOpen: true,
          installError: null,
          installDownloadedBytes: 0,
          installTotalBytes: null,
          installingUpdate: false,
        });
      }

      if (!update && !options.silent) {
        toast("已是最新版本");
      }

      return update;
    } catch (err) {
      const message = String(err);
      logToConsole("error", "检查更新失败", { error: message });
      writeLastCheckedAtMs(Date.now());
      if (!options.silent) toast(`检查更新失败：${message}`);
      return null;
    } finally {
      checkingPromise = null;
    }
  })();

  return checkingPromise;
}

function onUpdaterDownloadEvent(evt: UpdaterDownloadEvent) {
  if (evt.event === "started") {
    const total = evt.data?.contentLength;
    setUiSnapshot({ installTotalBytes: typeof total === "number" ? total : null });
    return;
  }
  if (evt.event === "progress") {
    const chunk = evt.data?.chunkLength;
    if (typeof chunk === "number" && Number.isFinite(chunk) && chunk > 0) {
      setUiSnapshot({ installDownloadedBytes: uiSnapshot.installDownloadedBytes + chunk });
    }
  }
}

export async function updateDownloadAndInstall(): Promise<boolean | null> {
  await ensureStarted();

  const updateCandidate =
    queryClient.getQueryData<UpdaterCheckUpdate | null>(updaterKeys.check()) ?? null;
  if (!updateCandidate) return null;

  if (uiSnapshot.installingUpdate) return installingPromise ?? true;

  setUiSnapshot({
    installError: null,
    installDownloadedBytes: 0,
    installTotalBytes: null,
    installingUpdate: true,
  });

  installingPromise = (async () => {
    try {
      const ok = await updaterDownloadAndInstall({
        rid: updateCandidate.rid,
        onEvent: onUpdaterDownloadEvent,
      });
      return ok;
    } catch (err) {
      const message = String(err);
      setUiSnapshot({ installError: message });
      logToConsole("error", "安装更新失败", { error: message });
      toast("安装更新失败：请稍后重试");
      return false;
    } finally {
      setUiSnapshot({ installingUpdate: false });
      installingPromise = null;
    }
  })();

  return installingPromise;
}

export function updateDialogSetOpen(open: boolean) {
  if (!open && uiSnapshot.installingUpdate) return;

  setUiSnapshot({ dialogOpen: open });
  if (!open) {
    setUiSnapshot({
      installError: null,
      installDownloadedBytes: 0,
      installTotalBytes: null,
      installingUpdate: false,
    });
  }
}

export function useUpdateMeta(): UpdateMeta {
  const aboutQuery = useAppAboutQuery();
  const updaterCheckQuery = useUpdaterCheckQuery();

  const ui = useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      void ensureStarted();
      return () => listeners.delete(listener);
    },
    () => uiSnapshot,
    () => uiSnapshot
  );

  return useMemo(
    () => ({
      about: aboutQuery.data ?? null,
      updateCandidate: updaterCheckQuery.data ?? null,
      checkingUpdate: updaterCheckQuery.isFetching,
      dialogOpen: ui.dialogOpen,

      installingUpdate: ui.installingUpdate,
      installError: ui.installError,
      installTotalBytes: ui.installTotalBytes,
      installDownloadedBytes: ui.installDownloadedBytes,
    }),
    [
      aboutQuery.data,
      ui.dialogOpen,
      ui.installDownloadedBytes,
      ui.installError,
      ui.installTotalBytes,
      ui.installingUpdate,
      updaterCheckQuery.data,
      updaterCheckQuery.isFetching,
    ]
  );
}
