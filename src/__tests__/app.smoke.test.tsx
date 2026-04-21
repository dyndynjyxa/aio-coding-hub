import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClientProvider } from "@tanstack/react-query";
import { createTestQueryClient } from "../test/utils/reactQuery";
import App from "../App";

const { mockLogToConsole } = vi.hoisted(() => ({
  mockLogToConsole: vi.fn(),
}));

vi.mock("../services/consoleLog", async () => {
  const actual =
    await vi.importActual<typeof import("../services/consoleLog")>("../services/consoleLog");
  return {
    ...actual,
    logToConsole: mockLogToConsole,
  };
});

vi.mock("../services/gateway/gatewayEvents", () => ({
  listenGatewayEvents: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("../services/notification/noticeEvents", () => ({
  listenNoticeEvents: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("../services/settings/settings", async () => {
  const actual = await vi.importActual<typeof import("../services/settings/settings")>(
    "../services/settings/settings"
  );
  return {
    ...actual,
    settingsGet: vi.fn().mockResolvedValue(null),
  };
});

import { listenGatewayEvents } from "../services/gateway/gatewayEvents";
import { listenNoticeEvents } from "../services/notification/noticeEvents";
import { settingsGet } from "../services/settings/settings";

const DEFAULT_HASH = "#/";

function renderApp() {
  const client = createTestQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <App />
    </QueryClientProvider>
  );
}

async function renderRouteAndFindHeading(hash: string, headingName: string, timeout = 15000) {
  window.location.hash = hash;
  renderApp();
  return screen.findByRole("heading", { level: 1, name: headingName }, { timeout });
}

describe("App (smoke)", () => {
  beforeEach(() => {
    mockLogToConsole.mockReset();
    vi.mocked(listenGatewayEvents).mockResolvedValue(() => {});
    vi.mocked(listenNoticeEvents).mockResolvedValue(() => {});
    vi.mocked(settingsGet).mockResolvedValue(null as any);
  });

  afterEach(() => {
    cleanup();
    window.location.hash = DEFAULT_HASH;
  });

  it("renders home route by default", async () => {
    expect(await renderRouteAndFindHeading("#/", "首页")).toBeInTheDocument();
  });

  it("renders settings route via hash", async () => {
    expect(await renderRouteAndFindHeading("#/settings", "设置")).toBeInTheDocument();
  });

  it("redirects unknown hash routes back to home", async () => {
    expect(await renderRouteAndFindHeading("#/definitely-missing", "首页")).toBeInTheDocument();
  });

  it("logs warning when event listeners initialization fails", async () => {
    vi.mocked(listenGatewayEvents).mockRejectedValueOnce(new Error("gateway init failed"));
    vi.mocked(listenNoticeEvents).mockRejectedValueOnce(new Error("notice init failed"));

    window.location.hash = "#/settings";
    renderApp();

    expect(
      await screen.findByRole("heading", { level: 1, name: "设置" }, { timeout: 15000 })
    ).toBeInTheDocument();

    await vi.waitFor(() => {
      expect(mockLogToConsole).toHaveBeenCalledWith(
        "warn",
        "网关事件监听初始化失败",
        expect.objectContaining({
          stage: "listenGatewayEvents",
          error: expect.stringContaining("gateway init failed"),
        })
      );
    });

    expect(mockLogToConsole).toHaveBeenCalledWith(
      "warn",
      "通知事件监听初始化失败",
      expect.objectContaining({
        stage: "listenNoticeEvents",
        error: expect.stringContaining("notice init failed"),
      })
    );
  });
});
