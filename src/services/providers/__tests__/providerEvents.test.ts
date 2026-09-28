import { beforeEach, describe, expect, it, vi } from "vitest";
import { appEventNames } from "../../../constants/appEvents";
import { emitTauriEvent, tauriListen, tauriUnlisten } from "../../../test/mocks/tauri";

const logToConsoleMock = vi.hoisted(() => vi.fn());

vi.mock("../../consoleLog", () => ({
  logToConsole: logToConsoleMock,
}));

import { listenProviderCodexCatalogEvents, parseCodexCatalogEventPayload } from "../providerEvents";

describe("services/providers/providerEvents", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("parses only supported catalog statuses", () => {
    expect(parseCodexCatalogEventPayload({ status: "updated" })).toEqual({ status: "updated" });
    expect(parseCodexCatalogEventPayload({ status: "failed" })).toEqual({ status: "failed" });
    expect(parseCodexCatalogEventPayload({ status: "stale" })).toBeNull();
    expect(parseCodexCatalogEventPayload(null)).toBeNull();
  });

  it("delivers valid events and returns listener cleanup", async () => {
    const onEvent = vi.fn();
    const unlisten = await listenProviderCodexCatalogEvents(onEvent);

    expect(tauriListen).toHaveBeenCalledWith(
      appEventNames.providerCodexCatalog,
      expect.any(Function)
    );
    emitTauriEvent(appEventNames.providerCodexCatalog, { status: "updated" });
    expect(onEvent).toHaveBeenCalledWith({ status: "updated" });

    unlisten();
    expect(tauriUnlisten).toHaveBeenCalledTimes(1);
  });

  it("drops and logs invalid payloads", async () => {
    const onEvent = vi.fn();
    const unlisten = await listenProviderCodexCatalogEvents(onEvent);

    emitTauriEvent(appEventNames.providerCodexCatalog, { status: "stale" });
    unlisten();

    expect(onEvent).not.toHaveBeenCalled();
    expect(logToConsoleMock).toHaveBeenCalledWith(
      "warn",
      "忽略无效的 Codex 模型目录事件",
      { payload_type: "object" },
      appEventNames.providerCodexCatalog
    );
  });
});
