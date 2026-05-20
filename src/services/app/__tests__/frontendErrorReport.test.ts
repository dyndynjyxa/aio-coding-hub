import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  appFrontendErrorReport,
  FRONTEND_ERROR_HREF_MAX_CHARS,
  FRONTEND_ERROR_MESSAGE_MAX_CHARS,
  normalizeFrontendErrorReportInput,
} from "../frontendErrorReport";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      appFrontendErrorReport: vi.fn(),
    },
  };
});

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return {
    ...actual,
    logToConsole: vi.fn(),
  };
});

describe("services/app/frontendErrorReport", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("normalizes report payloads before generated IPC", async () => {
    vi.mocked(commands.appFrontendErrorReport).mockResolvedValueOnce({
      status: "ok",
      data: true,
    });

    await expect(
      appFrontendErrorReport({
        source: " render " as never,
        message: " boom ",
        stack: " stack ",
        detailsJson: " ",
        href: " http://localhost/#/ ",
        userAgent: " test-agent ",
      })
    ).resolves.toBe(true);

    expect(commands.appFrontendErrorReport).toHaveBeenCalledWith({
      source: "render",
      message: "boom",
      stack: "stack",
      detailsJson: null,
      href: "http://localhost/#/",
      userAgent: "test-agent",
    });
  });

  it("truncates oversized report fields", () => {
    const normalized = normalizeFrontendErrorReportInput({
      source: "error",
      message: "m".repeat(FRONTEND_ERROR_MESSAGE_MAX_CHARS + 10),
      stack: null,
      detailsJson: null,
      href: "h".repeat(FRONTEND_ERROR_HREF_MAX_CHARS + 10),
      userAgent: null,
    });

    expect([...normalized.message]).toHaveLength(FRONTEND_ERROR_MESSAGE_MAX_CHARS);
    expect([...(normalized.href ?? "")]).toHaveLength(FRONTEND_ERROR_HREF_MAX_CHARS);
  });

  it("rejects invalid report payloads before generated IPC", async () => {
    await expect(
      appFrontendErrorReport({
        source: "unknown",
        message: "boom",
        stack: null,
        detailsJson: null,
        href: null,
        userAgent: null,
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");

    await expect(
      appFrontendErrorReport({
        source: "error",
        message: " ",
        stack: null,
        detailsJson: null,
        href: null,
        userAgent: null,
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");

    expect(commands.appFrontendErrorReport).not.toHaveBeenCalled();
  });

  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.appFrontendErrorReport).mockRejectedValueOnce(
      new Error("frontend report boom")
    );

    await expect(
      appFrontendErrorReport({
        source: "error",
        message: "boom",
        stack: null,
        detailsJson: null,
        href: null,
        userAgent: null,
      })
    ).rejects.toThrow("frontend report boom");

    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "上报前端异常失败",
      expect.objectContaining({
        cmd: "app_frontend_error_report",
        error: expect.stringContaining("frontend report boom"),
      })
    );
  });
});
