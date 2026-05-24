import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import { appAboutGet } from "../appAbout";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      appAboutGet: vi.fn(),
    },
  };
});

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return { ...actual, logToConsole: vi.fn() };
});

describe("services/app/appAbout", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns about info when available", async () => {
    vi.mocked(commands.appAboutGet).mockResolvedValue({
      os: " mac ",
      arch: "arm64",
      profile: "debug",
      app_version: " 0.0.0 ",
      bundle_type: " appimage ",
      run_mode: "portable",
    } as any);

    const result = await appAboutGet();
    expect(result).toEqual(
      expect.objectContaining({
        os: "mac",
        arch: "arm64",
        app_version: "0.0.0",
        bundle_type: "appimage",
        run_mode: "portable",
      })
    );
  });

  it("rejects invalid about info payloads before caching app lifecycle branches", async () => {
    vi.mocked(commands.appAboutGet).mockResolvedValueOnce({
      os: "mac",
      arch: "arm64",
      profile: "release",
      app_version: "0.1.0",
      bundle_type: null,
      run_mode: "desktop",
    } as any);

    await expect(appAboutGet()).rejects.toThrow("IPC_INVALID_RESULT");

    vi.mocked(commands.appAboutGet).mockResolvedValueOnce({
      os: "",
      arch: "arm64",
      profile: "release",
      app_version: "0.1.0",
      bundle_type: null,
      run_mode: "portable",
    } as any);

    await expect(appAboutGet()).rejects.toThrow("IPC_INVALID_RESULT");
  });

  it("throws when invoke throws", async () => {
    vi.mocked(commands.appAboutGet).mockRejectedValue(new Error("boom"));

    await expect(appAboutGet()).rejects.toThrow("boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取应用信息失败",
      expect.objectContaining({
        cmd: "app_about_get",
        error: expect.stringContaining("boom"),
      })
    );
  });

  it("throws when invoke returns null", async () => {
    vi.mocked(commands.appAboutGet).mockResolvedValue(null as any);

    await expect(appAboutGet()).rejects.toThrow("IPC_NULL_RESULT: app_about_get");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取应用信息失败",
      expect.objectContaining({
        cmd: "app_about_get",
        error: expect.stringContaining("IPC_NULL_RESULT: app_about_get"),
      })
    );
  });
});
