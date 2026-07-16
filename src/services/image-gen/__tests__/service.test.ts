import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  IMAGE_GEN_ADAPTER_ID,
  imageGenConfigGet,
  imageGenConfigSet,
  imageGenFetchImage,
  imageGenPostJson,
  imageGenPostMultipart,
  imageGenSaveImage,
} from "../service";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      imageGenConfigGet: vi.fn(),
      imageGenConfigSet: vi.fn(),
      imageGenPostJson: vi.fn(),
      imageGenPostMultipart: vi.fn(),
      imageGenFetchImage: vi.fn(),
      imageGenSaveImage: vi.fn(),
    },
  };
});

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return { ...actual, logToConsole: vi.fn() };
});

const CONFIG_VIEW = {
  adapterId: IMAGE_GEN_ADAPTER_ID,
  baseUrl: "https://api.example.com/v1",
  model: "gpt-image-2",
  apiKeyConfigured: true,
};

describe("services/image-gen/service", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("imageGenConfigGet returns the config view", async () => {
    vi.mocked(commands.imageGenConfigGet).mockResolvedValue({ status: "ok", data: CONFIG_VIEW });
    await expect(imageGenConfigGet(IMAGE_GEN_ADAPTER_ID)).resolves.toEqual(CONFIG_VIEW);
    expect(commands.imageGenConfigGet).toHaveBeenCalledWith(IMAGE_GEN_ADAPTER_ID);
  });

  it("imageGenConfigGet throws and logs on error results", async () => {
    vi.mocked(commands.imageGenConfigGet).mockResolvedValue({ status: "error", error: "boom" });
    await expect(imageGenConfigGet(IMAGE_GEN_ADAPTER_ID)).rejects.toThrow("boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取生图配置失败",
      expect.objectContaining({ cmd: "image_gen_config_get" })
    );
  });

  it("imageGenConfigSet forwards the api key but never logs its value", async () => {
    vi.mocked(commands.imageGenConfigSet).mockResolvedValue({ status: "error", error: "denied" });
    await expect(
      imageGenConfigSet(
        IMAGE_GEN_ADAPTER_ID,
        "https://api.example.com/v1",
        "gpt-image-2",
        "sk-secret"
      )
    ).rejects.toThrow("denied");
    expect(commands.imageGenConfigSet).toHaveBeenCalledWith(
      IMAGE_GEN_ADAPTER_ID,
      "https://api.example.com/v1",
      "gpt-image-2",
      "sk-secret"
    );
    const logArgs = vi.mocked(logToConsole).mock.calls[0][2] as {
      args: { apiKey: string };
    };
    expect(logArgs.args.apiKey).toBe("[REDACTED]");
    expect(JSON.stringify(vi.mocked(logToConsole).mock.calls)).not.toContain("sk-secret");
  });

  it("imageGenConfigSet never logs the api key value on the success path either", async () => {
    vi.mocked(commands.imageGenConfigSet).mockResolvedValue({ status: "ok", data: CONFIG_VIEW });
    await expect(
      imageGenConfigSet(
        IMAGE_GEN_ADAPTER_ID,
        "https://api.example.com/v1",
        "gpt-image-2",
        "sk-secret"
      )
    ).resolves.toEqual(CONFIG_VIEW);
    expect(JSON.stringify(vi.mocked(logToConsole).mock.calls)).not.toContain("sk-secret");
  });

  it("imageGenConfigSet passes null through to preserve the stored key", async () => {
    vi.mocked(commands.imageGenConfigSet).mockResolvedValue({ status: "ok", data: CONFIG_VIEW });
    await imageGenConfigSet(
      IMAGE_GEN_ADAPTER_ID,
      "https://api.example.com/v1",
      "gpt-image-2",
      null
    );
    expect(commands.imageGenConfigSet).toHaveBeenCalledWith(
      IMAGE_GEN_ADAPTER_ID,
      "https://api.example.com/v1",
      "gpt-image-2",
      null
    );
  });

  it("imageGenPostJson returns the http response and defaults timeout to null", async () => {
    const response = { status: 200, bodyText: "{}" };
    vi.mocked(commands.imageGenPostJson).mockResolvedValue({ status: "ok", data: response });
    await expect(
      imageGenPostJson(IMAGE_GEN_ADAPTER_ID, "/v1/images/generations", { model: "gpt-image-2" })
    ).resolves.toEqual(response);
    expect(commands.imageGenPostJson).toHaveBeenCalledWith(
      IMAGE_GEN_ADAPTER_ID,
      "/v1/images/generations",
      { model: "gpt-image-2" },
      null
    );
  });

  it("imageGenPostJson throws on error results", async () => {
    vi.mocked(commands.imageGenPostJson).mockResolvedValue({ status: "error", error: "网络错误" });
    await expect(
      imageGenPostJson(IMAGE_GEN_ADAPTER_ID, "/v1/images/generations", {})
    ).rejects.toThrow("网络错误");
  });

  it("imageGenPostMultipart forwards fields and files", async () => {
    const response = { status: 200, bodyText: "{}" };
    vi.mocked(commands.imageGenPostMultipart).mockResolvedValue({ status: "ok", data: response });
    const fields: [string, string][] = [["prompt", "hi"]];
    const files = [{ field: "image[]", filename: "input-1.png", mime: "image/png", dataB64: "AA" }];
    await expect(
      imageGenPostMultipart(IMAGE_GEN_ADAPTER_ID, "/v1/images/edits", fields, files)
    ).resolves.toEqual(response);
    expect(commands.imageGenPostMultipart).toHaveBeenCalledWith(
      IMAGE_GEN_ADAPTER_ID,
      "/v1/images/edits",
      fields,
      files,
      null
    );
  });

  it("imageGenFetchImage returns the fetched image", async () => {
    const fetched = { mime: "image/png", dataB64: "AA" };
    vi.mocked(commands.imageGenFetchImage).mockResolvedValue({ status: "ok", data: fetched });
    await expect(imageGenFetchImage("https://cdn.example.com/a.png")).resolves.toEqual(fetched);
    expect(commands.imageGenFetchImage).toHaveBeenCalledWith("https://cdn.example.com/a.png", null);
  });

  it("imageGenSaveImage saves and throws on failure", async () => {
    vi.mocked(commands.imageGenSaveImage).mockResolvedValue({ status: "ok", data: true });
    await expect(imageGenSaveImage("/tmp/a.png", "AA")).resolves.toBe(true);
    expect(commands.imageGenSaveImage).toHaveBeenCalledWith("/tmp/a.png", "AA");

    vi.mocked(commands.imageGenSaveImage).mockResolvedValue({ status: "error", error: "写盘失败" });
    await expect(imageGenSaveImage("/tmp/a.png", "AA")).rejects.toThrow("写盘失败");
  });
});
