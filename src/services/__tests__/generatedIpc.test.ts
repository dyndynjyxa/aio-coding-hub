import { describe, expect, it, vi } from "vitest";
import { invokeGeneratedIpc } from "../generatedIpc";
import { logToConsole } from "../consoleLog";

vi.mock("../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../consoleLog")>("../consoleLog");
  return {
    ...actual,
    logToConsole: vi.fn(),
  };
});

describe("services/generatedIpc", () => {
  it("throws string error results and logs command details", async () => {
    await expect(
      invokeGeneratedIpc({
        title: "读取设置失败",
        cmd: "settings_get",
        args: { source: "test" },
        invoke: async () => ({ status: "error", error: "boom" }),
      })
    ).rejects.toThrow("boom");

    expect(logToConsole).toHaveBeenCalledWith("error", "读取设置失败", {
      cmd: "settings_get",
      args: { source: "test" },
      error: "boom",
    });
  });

  it("formats structured error results before logging", async () => {
    await expect(
      invokeGeneratedIpc({
        title: "更新设置失败",
        cmd: "settings_set",
        args: { update: { preferredPort: 37123 } },
        invoke: async () => ({ status: "error", error: { code: "E_BAD_SETTINGS" } }),
      })
    ).rejects.toThrow('{"code":"E_BAD_SETTINGS"}');

    expect(logToConsole).toHaveBeenCalledWith("error", "更新设置失败", {
      cmd: "settings_set",
      args: { update: { preferredPort: 37123 } },
      error: '{"code":"E_BAD_SETTINGS"}',
    });
  });

  it("redacts sensitive log args before forwarding to console logs", async () => {
    await expect(
      invokeGeneratedIpc({
        title: "保存供应商失败",
        cmd: "provider_upsert",
        args: {
          input: {
            apiKey: "sk-secret",
            nested: {
              refreshToken: "rt-secret",
              safe: "ok",
            },
          },
        },
        invoke: async () => ({ status: "error", error: "boom" }),
      })
    ).rejects.toThrow("boom");

    expect(logToConsole).toHaveBeenCalledWith("error", "保存供应商失败", {
      cmd: "provider_upsert",
      args: {
        input: {
          apiKey: "[REDACTED]",
          nested: {
            refreshToken: "[REDACTED]",
            safe: "ok",
          },
        },
      },
      error: "boom",
    });
  });

  it("supports null-result fallback for commands that legitimately return null", async () => {
    await expect(
      invokeGeneratedIpc<null, string>({
        title: "读取更新结果失败",
        cmd: "desktop_updater_check",
        invoke: async () => ({ status: "ok", data: null }),
        nullResultBehavior: "return_fallback",
        fallback: "no-update",
      })
    ).resolves.toBe("no-update");

    expect(logToConsole).not.toHaveBeenCalledWith(
      "error",
      "读取更新结果失败",
      expect.anything()
    );
  });

  it("passes through raw generated command payloads without Result envelope", async () => {
    await expect(
      invokeGeneratedIpc({
        title: "读取应用信息失败",
        cmd: "app_about_get",
        invoke: async () => ({
          os: "macos",
          arch: "arm64",
          profile: "release",
        }),
      })
    ).resolves.toEqual({
      os: "macos",
      arch: "arm64",
      profile: "release",
    });
  });

  it("supports fallback when a raw command returns null", async () => {
    await expect(
      invokeGeneratedIpc<string | null, string>({
        title: "读取应用目录失败",
        cmd: "app_data_dir_get",
        invoke: async () => null,
        nullResultBehavior: "return_fallback",
        fallback: "fallback-dir",
      })
    ).resolves.toBe("fallback-dir");
  });
});
