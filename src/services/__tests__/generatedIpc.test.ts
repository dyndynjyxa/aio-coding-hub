import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";
import { logToConsole } from "../consoleLog";

vi.mock("../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../consoleLog")>("../consoleLog");
  return {
    ...actual,
    logToConsole: vi.fn(),
  };
});

describe("services/generatedIpc", () => {
  beforeEach(() => {
    vi.mocked(logToConsole).mockClear();
  });

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

  it("bounds large log args while preserving sensitive-key redaction", async () => {
    const items = Array.from({ length: 80 }, (_, index) => ({
      label: `item-${index}`,
      token: `secret-${index}`,
      text: "x".repeat(3_000),
    }));
    const wide = Object.fromEntries(
      Array.from({ length: 80 }, (_, index) => [`k${String(index).padStart(2, "0")}`, index])
    );

    await expect(
      invokeGeneratedIpc({
        title: "保存失败",
        cmd: "large_payload",
        args: {
          input: {
            apiKey: "sk-secret",
            items,
            wide,
          },
        },
        invoke: async () => ({ status: "error", error: "boom" }),
      })
    ).rejects.toThrow("boom");

    const calls = vi.mocked(logToConsole).mock.calls;
    const loggedDetails = calls[calls.length - 1]?.[2] as {
      args: {
        input: {
          apiKey: string;
          items: Array<Record<string, unknown> | string>;
          wide: Record<string, unknown>;
        };
      };
    };

    expect(loggedDetails.args.input.apiKey).toBe("[REDACTED]");
    expect(loggedDetails.args.input.items).toHaveLength(51);
    expect(loggedDetails.args.input.items[0]).toMatchObject({
      label: "item-0",
      token: "[REDACTED]",
      text: expect.stringContaining("[Truncated"),
    });
    expect(loggedDetails.args.input.items[loggedDetails.args.input.items.length - 1]).toBe(
      "[Truncated 30 items]"
    );
    expect(loggedDetails.args.input.items).not.toContainEqual(
      expect.objectContaining({ label: "item-79" })
    );
    expect(loggedDetails.args.input.wide.k00).toBe(0);
    expect(loggedDetails.args.input.wide.k79).toBeUndefined();
    expect(loggedDetails.args.input.wide.__truncated__).toBe("30 keys truncated");
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

    expect(logToConsole).not.toHaveBeenCalledWith("error", "读取更新结果失败", expect.anything());
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

  it("keeps ok envelopes with null data intact while mapping generated responses", () => {
    expect(
      mapGeneratedCommandResponse<string[], number[]>({ status: "ok", data: null }, (items) =>
        items.map((item) => Number(item))
      )
    ).toEqual({
      status: "ok",
      data: null,
    });
  });
});
