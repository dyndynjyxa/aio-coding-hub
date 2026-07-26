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
  imageGenReadImage,
  imageGenSaveImage,
  imageGenStorageCleanup,
  imageGenStorageGet,
  imageGenStorageSetDir,
  imageGenTaskDelete,
  imageGenTaskPersist,
  imageGenTasksClear,
  imageGenTasksList,
  type ImageGenTaskPersistPayload,
  type ImageGenTaskRow,
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
      imageGenTaskPersist: vi.fn(),
      imageGenTasksList: vi.fn(),
      imageGenTaskDelete: vi.fn(),
      imageGenTasksClear: vi.fn(),
      imageGenReadImage: vi.fn(),
      imageGenStorageGet: vi.fn(),
      imageGenStorageSetDir: vi.fn(),
      imageGenStorageCleanup: vi.fn(),
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

  // ---------- 历史持久化（二期） ----------

  const PERSIST_PAYLOAD: ImageGenTaskPersistPayload = {
    id: "t1",
    adapterId: IMAGE_GEN_ADAPTER_ID,
    prompt: "一只猫",
    requestJson: "{}",
    status: "done",
    error: null,
    usageJson: null,
    createdAt: 1_700_000_000_000,
    elapsedMs: 900,
    images: [{ mime: "image/png", dataB64: "SUPER-SECRET-IMAGE-BYTES" }],
    thumbs: [{ mime: "image/webp", dataB64: "SUPER-SECRET-THUMB-BYTES" }],
    refImages: [{ mime: "image/png", dataB64: "SUPER-SECRET-REF-BYTES" }],
  };

  const TASK_ROW: ImageGenTaskRow = {
    id: "t1",
    adapterId: IMAGE_GEN_ADAPTER_ID,
    prompt: "一只猫",
    requestJson: "{}",
    status: "done",
    error: null,
    usageJson: null,
    images: [
      { path: "/store/t1/image-1.png", thumbPath: "/store/t1/thumb-1.webp", mime: "image/png" },
    ],
    refImages: [],
    dir: "/store/t1",
    createdAt: 1_700_000_000_000,
    elapsedMs: 900,
  };

  it("imageGenTaskPersist returns the stored row and never logs image bytes", async () => {
    vi.mocked(commands.imageGenTaskPersist).mockResolvedValue({ status: "error", error: "满了" });
    await expect(imageGenTaskPersist(PERSIST_PAYLOAD)).rejects.toThrow("满了");
    expect(commands.imageGenTaskPersist).toHaveBeenCalledWith(PERSIST_PAYLOAD);
    // 失败日志只含计数，不含任何 base64 字节。
    const logged = JSON.stringify(vi.mocked(logToConsole).mock.calls);
    expect(logged).not.toContain("SUPER-SECRET");
    expect(logged).toContain('"imageCount":1');

    vi.mocked(commands.imageGenTaskPersist).mockResolvedValue({ status: "ok", data: TASK_ROW });
    await expect(imageGenTaskPersist(PERSIST_PAYLOAD)).resolves.toEqual(TASK_ROW);
  });

  it("imageGenTasksList forwards the cursor and returns rows", async () => {
    vi.mocked(commands.imageGenTasksList).mockResolvedValue({ status: "ok", data: [TASK_ROW] });
    await expect(imageGenTasksList(null, 50)).resolves.toEqual([TASK_ROW]);
    expect(commands.imageGenTasksList).toHaveBeenCalledWith(null, 50);

    vi.mocked(commands.imageGenTasksList).mockResolvedValue({ status: "error", error: "db" });
    await expect(imageGenTasksList(123, 10)).rejects.toThrow("db");
  });

  it("imageGenTaskDelete tolerates the null result and throws on error", async () => {
    vi.mocked(commands.imageGenTaskDelete).mockResolvedValue({ status: "ok", data: null });
    await expect(imageGenTaskDelete("t1")).resolves.toBeNull();
    expect(commands.imageGenTaskDelete).toHaveBeenCalledWith("t1");

    vi.mocked(commands.imageGenTaskDelete).mockResolvedValue({ status: "error", error: "io" });
    await expect(imageGenTaskDelete("t1")).rejects.toThrow("io");
  });

  it("imageGenTasksClear returns the removed count (including zero)", async () => {
    vi.mocked(commands.imageGenTasksClear).mockResolvedValue({ status: "ok", data: 0 });
    await expect(imageGenTasksClear()).resolves.toBe(0);

    vi.mocked(commands.imageGenTasksClear).mockResolvedValue({ status: "error", error: "io" });
    await expect(imageGenTasksClear()).rejects.toThrow("io");
  });

  it("imageGenReadImage returns bytes and throws on out-of-scope paths", async () => {
    const fetched = { mime: "image/png", dataB64: "AA" };
    vi.mocked(commands.imageGenReadImage).mockResolvedValue({ status: "ok", data: fetched });
    await expect(imageGenReadImage("/store/t1/image-1.png")).resolves.toEqual(fetched);
    expect(commands.imageGenReadImage).toHaveBeenCalledWith("/store/t1/image-1.png");

    vi.mocked(commands.imageGenReadImage).mockResolvedValue({
      status: "error",
      error: "SEC_PATH: outside storage dir",
    });
    await expect(imageGenReadImage("/etc/passwd")).rejects.toThrow("SEC_PATH");
  });

  it("imageGenStorage get/setDir/cleanup round-trip the storage view and counts", async () => {
    const view = { dir: "/store", totalBytes: 42, taskCount: 2 };
    vi.mocked(commands.imageGenStorageGet).mockResolvedValue({ status: "ok", data: view });
    await expect(imageGenStorageGet()).resolves.toEqual(view);

    vi.mocked(commands.imageGenStorageSetDir).mockResolvedValue({ status: "ok", data: view });
    await expect(imageGenStorageSetDir("/store")).resolves.toEqual(view);
    expect(commands.imageGenStorageSetDir).toHaveBeenCalledWith("/store");

    vi.mocked(commands.imageGenStorageSetDir).mockResolvedValue({
      status: "error",
      error: "不可写",
    });
    await expect(imageGenStorageSetDir("/readonly")).rejects.toThrow("不可写");

    vi.mocked(commands.imageGenStorageCleanup).mockResolvedValue({ status: "ok", data: 7 });
    await expect(imageGenStorageCleanup(50)).resolves.toBe(7);
    expect(commands.imageGenStorageCleanup).toHaveBeenCalledWith(50);
  });
});
