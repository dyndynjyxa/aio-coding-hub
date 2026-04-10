import { listen } from "@tauri-apps/api/event";

import type { GatewayEventName } from "../../constants/gatewayEvents";
import { logToConsole } from "../consoleLog";

export type GatewayEventSubscription = {
  ready: Promise<void>;
  unsubscribe: () => void;
};

type Handler = (payload: unknown) => void;

type Entry = {
  handlers: Set<Handler>;
  init: Promise<void> | null;
  unlisten: (() => void) | null;
  disposed: boolean;
};

const entries = new Map<GatewayEventName, Entry>();

function dispatchHandlers(handlers: Set<Handler>, payload: unknown) {
  for (const handler of handlers) handler(payload);
}

function getOrCreateEntry(event: GatewayEventName): Entry {
  const existing = entries.get(event);
  if (existing) return existing;

  const created: Entry = {
    handlers: new Set(),
    init: null,
    unlisten: null,
    disposed: false,
  };
  entries.set(event, created);
  return created;
}

function disposeEntry(event: GatewayEventName, entry: Entry) {
  entry.disposed = true;
  if (entry.unlisten) entry.unlisten();
  entry.unlisten = null;
  entries.delete(event);
}

function ensureListening(event: GatewayEventName, entry: Entry): Promise<void> {
  if (entry.init) return entry.init;

  entry.init = listen(event, (evt) => {
    dispatchHandlers(entry.handlers, evt.payload);
  })
    .then((unlisten) => {
      entry.unlisten = unlisten;
      if (entry.disposed || entry.handlers.size === 0) disposeEntry(event, entry);
    })
    .catch((error) => {
      entry.init = null;
      entry.unlisten = null;
      logToConsole(
        "error",
        "网关事件监听初始化失败",
        { event, error: String(error) },
        "gateway:event_bus"
      );
      if (entry.disposed || entry.handlers.size === 0) disposeEntry(event, entry);
      throw error;
    });

  return entry.init;
}

export function subscribeGatewayEvent<TPayload>(
  event: GatewayEventName,
  handler: (payload: TPayload) => void
): GatewayEventSubscription {
  const entry = getOrCreateEntry(event);
  const wrapped: Handler = (payload) => {
    handler(payload as TPayload);
  };

  entry.handlers.add(wrapped);
  const ready = ensureListening(event, entry);

  return {
    ready,
    unsubscribe: () => {
      entry.handlers.delete(wrapped);
      if (entry.handlers.size !== 0) return;
      if (entry.unlisten) {
        disposeEntry(event, entry);
        return;
      }
      if (entry.init) entry.disposed = true;
      else disposeEntry(event, entry);
    },
  };
}
