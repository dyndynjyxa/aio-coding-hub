import { vi } from "vitest";
import { TAURI_ENDPOINT } from "../tauriEndpoint";

type TauriEvent<TPayload = unknown> = {
  event: string;
  payload: TPayload;
};

type TauriEventHandler<TPayload = unknown> = (event: TauriEvent<TPayload>) => void;

const listeners = new Map<string, Set<TauriEventHandler<any>>>();

export const emitTauriEvent = (event: string, payload: unknown) => {
  const handlers = listeners.get(event);
  if (!handlers) return;

  // Defensive copy: handlers may unregister while we're iterating.
  Array.from(handlers).forEach((handler) => handler({ event, payload }));
};

export const clearTauriEventListeners = () => {
  listeners.clear();
};

// Back-compat alias (older tests may refer to the reset name).
export const resetTauriEventListeners = clearTauriEventListeners;

async function parseTauriInvokeResponse(response: Response) {
  const text = await response.text();
  if (!text) return undefined;

  const contentType = response.headers.get("content-type") ?? "";
  const looksJson = contentType.includes("application/json") || contentType.includes("+json");
  if (looksJson) {
    try {
      return JSON.parse(text);
    } catch {
      // fallthrough
    }
  }

  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

export const tauriInvoke = vi.fn(async (command: string, payload?: Record<string, unknown>) => {
  const commandPath = String(command).replace(/^\/+/, "");
  const url = `${TAURI_ENDPOINT}/${commandPath}`;

  const response = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload ?? {}),
  });

  if (!response.ok) {
    const parsed = await parseTauriInvokeResponse(response);
    const message =
      typeof parsed === "string"
        ? parsed
        : parsed == null
          ? `Invoke failed for ${command}`
          : `Invoke failed for ${command}: ${JSON.stringify(parsed)}`;
    throw new Error(message);
  }

  return parseTauriInvokeResponse(response);
});

export const tauriUnlisten = vi.fn();

export const tauriListen = vi.fn(async (event: string, handler: TauriEventHandler<any>) => {
  const set = listeners.get(event) ?? new Set<TauriEventHandler<any>>();
  set.add(handler);
  listeners.set(event, set);

  return () => {
    tauriUnlisten();
    const current = listeners.get(event);
    current?.delete(handler);
    if (current && current.size === 0) listeners.delete(event);
  };
});

export const tauriEmit = vi.fn(async (event: string, payload?: unknown) => {
  emitTauriEvent(event, payload);
});

export const tauriOpenUrl = vi.fn();
export const tauriOpenPath = vi.fn();
export const tauriRevealItemInDir = vi.fn();
export const tauriDialogOpen = vi.fn();
export const tauriDialogSave = vi.fn();
export const tauriReadTextFile = vi.fn();
export const tauriWriteTextFile = vi.fn();

export const tauriIsPermissionGranted = vi.fn().mockResolvedValue(false);
export const tauriRequestPermission = vi.fn().mockResolvedValue("denied");
export const tauriSendNotification = vi.fn();

export class MockChannel<T> {
  private handler: (message: T) => void;
  constructor(handler: (message: T) => void) {
    this.handler = handler;
  }
  __emit(message: T) {
    this.handler(message);
  }
}

vi.mock("@tauri-apps/api/core", () => ({
  invoke: tauriInvoke,
  Channel: MockChannel,
}));

vi.mock("@tauri-apps/api/event", () => ({
  emit: tauriEmit,
  listen: tauriListen,
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: tauriOpenUrl,
  openPath: tauriOpenPath,
  revealItemInDir: tauriRevealItemInDir,
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: tauriDialogOpen,
  save: tauriDialogSave,
}));

vi.mock("@tauri-apps/plugin-fs", () => ({
  readTextFile: tauriReadTextFile,
  writeTextFile: tauriWriteTextFile,
}));

vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: tauriIsPermissionGranted,
  requestPermission: tauriRequestPermission,
  sendNotification: tauriSendNotification,
}));
