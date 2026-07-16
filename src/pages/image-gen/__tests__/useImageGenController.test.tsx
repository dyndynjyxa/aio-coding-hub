import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { gptImageAdapter } from "../../../services/image-gen/gptImageAdapter";
import {
  imageGenConfigGet,
  imageGenConfigSet,
  imageGenSaveImage,
} from "../../../services/image-gen/service";
import { saveDesktopFilePath } from "../../../services/desktop/dialog";
import type { ImageGenResult } from "../../../services/image-gen/types";
import { getImageGenSession, resetImageGenSessionForTests } from "../imageGenSessionStore";
import {
  base64ToBlob,
  DEFAULT_IMAGE_GEN_PARAMS,
  readParamsFromStorage,
  useImageGenController,
  validateReferenceAddition,
  type ImageGenAssistantMessage,
} from "../useImageGenController";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

vi.mock("../../../services/image-gen/service", async () => {
  const actual = await vi.importActual<typeof import("../../../services/image-gen/service")>(
    "../../../services/image-gen/service"
  );
  return {
    ...actual,
    imageGenConfigGet: vi.fn(),
    imageGenConfigSet: vi.fn(),
    imageGenSaveImage: vi.fn(),
  };
});

vi.mock("../../../services/image-gen/gptImageAdapter", async () => {
  const actual = await vi.importActual<
    typeof import("../../../services/image-gen/gptImageAdapter")
  >("../../../services/image-gen/gptImageAdapter");
  return {
    ...actual,
    gptImageAdapter: { ...actual.gptImageAdapter, generate: vi.fn() },
  };
});

vi.mock("../../../services/desktop/dialog", () => ({
  saveDesktopFilePath: vi.fn(),
}));

const EMPTY_CONFIG = {
  adapterId: "gpt-image",
  baseUrl: "",
  model: "",
  apiKeyConfigured: false,
};

function makePngFile(name = "ref.png", sizeBytes?: number): File {
  const file = new File(["fake-image-bytes"], name, { type: "image/png" });
  if (sizeBytes != null) {
    Object.defineProperty(file, "size", { value: sizeBytes });
  }
  return file;
}

async function renderController() {
  const rendered = renderHook(() => useImageGenController());
  await waitFor(() => {
    expect(imageGenConfigGet).toHaveBeenCalled();
  });
  return rendered;
}

describe("pages/image-gen/useImageGenController", () => {
  beforeEach(() => {
    // 模块 store 跨测试泄漏，必须先重置（会 revoke 上个测试登记的 URL）。
    resetImageGenSessionForTests();
    vi.clearAllMocks();
    window.localStorage.clear();
    let urlCounter = 0;
    URL.createObjectURL = vi.fn(() => {
      urlCounter += 1;
      return `blob:mock-${urlCounter}`;
    });
    URL.revokeObjectURL = vi.fn();
    vi.mocked(imageGenConfigGet).mockResolvedValue(EMPTY_CONFIG);
  });

  it("hydrates connection config from the backend", async () => {
    vi.mocked(imageGenConfigGet).mockResolvedValue({
      adapterId: "gpt-image",
      baseUrl: "https://api.example.com/v1",
      model: "gpt-image-2-2026-04-21",
      apiKeyConfigured: true,
    });
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.baseUrl).toBe("https://api.example.com/v1");
    });
    expect(result.current.model).toBe("gpt-image-2-2026-04-21");
    expect(result.current.apiKeyConfigured).toBe(true);
    expect(result.current.requestUrlPreview).toBe("https://api.example.com/v1/images/generations");
  });

  it("keeps defaults editable when config load fails", async () => {
    vi.mocked(imageGenConfigGet).mockRejectedValue(new Error("db down"));
    const { result } = await renderController();
    expect(result.current.baseUrl).toBe("");
    expect(result.current.model).toBe("gpt-image-2");
  });

  it("submits a text-to-image request and appends a done assistant message", async () => {
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("img") }],
      usage: { totalTokens: 42 },
    });
    const { result } = await renderController();

    act(() => {
      result.current.setPrompt("一只猫");
    });
    await act(async () => {
      await result.current.submit();
    });

    expect(gptImageAdapter.generate).toHaveBeenCalledWith(
      expect.objectContaining({
        prompt: "一只猫",
        referenceImages: [],
        n: 1,
        size: "auto",
        options: expect.objectContaining({ model: "gpt-image-2", quality: "auto" }),
      })
    );
    expect(result.current.messages).toHaveLength(2);
    expect(result.current.messages[0]).toMatchObject({ role: "user", prompt: "一只猫" });
    const assistant = result.current.messages[1] as ImageGenAssistantMessage;
    expect(assistant.status).toBe("done");
    expect(assistant.images).toHaveLength(1);
    expect(assistant.usage).toEqual({ totalTokens: 42 });
    expect(result.current.prompt).toBe("");
  });

  it("does nothing for an empty prompt", async () => {
    const { result } = await renderController();
    await act(async () => {
      await result.current.submit();
    });
    expect(gptImageAdapter.generate).not.toHaveBeenCalled();
    expect(result.current.messages).toHaveLength(0);
  });

  it("shows a readable error and retries with the request snapshot", async () => {
    vi.mocked(gptImageAdapter.generate).mockRejectedValueOnce(new Error("HTTP 500: boom"));
    const { result } = await renderController();

    act(() => {
      result.current.setPrompt("失败一次");
    });
    await act(async () => {
      await result.current.submit();
    });

    const failed = result.current.messages[1] as ImageGenAssistantMessage;
    expect(failed.status).toBe("error");
    expect(failed.error).toContain("HTTP 500: boom");

    // 改动面板当前值，证明重试不读它们。
    act(() => {
      result.current.updateParams({ n: 7, size: "1024x1024" });
      result.current.setPrompt("面板新值");
    });

    vi.mocked(gptImageAdapter.generate).mockResolvedValueOnce({
      images: [{ mime: "image/png", b64: btoa("ok") }],
    });
    await act(async () => {
      await result.current.retry(failed.id);
    });

    const calls = vi.mocked(gptImageAdapter.generate).mock.calls;
    expect(calls).toHaveLength(2);
    expect(calls[1][0]).toBe(calls[0][0]);
    const retried = result.current.messages[1] as ImageGenAssistantMessage;
    expect(retried.status).toBe("done");
    expect(retried.error).toBeUndefined();
  });

  it("rejects more than 16 reference images", async () => {
    const { result } = await renderController();
    const files = Array.from({ length: 17 }, (_, index) => makePngFile(`f${index}.png`));
    await act(async () => {
      await result.current.addReferenceFiles(files);
    });
    expect(toast.error).toHaveBeenCalledWith("参考图最多 16 张");
    expect(result.current.referenceImages).toHaveLength(0);
  });

  it("rejects reference images above the 30MB total budget", async () => {
    const { result } = await renderController();
    await act(async () => {
      await result.current.addReferenceFiles([makePngFile("big.png", 31 * 1024 * 1024)]);
    });
    expect(toast.error).toHaveBeenCalledWith("参考图合计不能超过 30MB");
    expect(result.current.referenceImages).toHaveLength(0);
  });

  it("adds valid reference images and routes them into the request", async () => {
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("edit") }],
    });
    const { result } = await renderController();

    await act(async () => {
      await result.current.addReferenceFiles([makePngFile()]);
    });
    expect(result.current.referenceImages).toHaveLength(1);
    expect(result.current.referenceImages[0].b64.length).toBeGreaterThan(0);

    act(() => {
      result.current.setPrompt("改成夜景");
    });
    await act(async () => {
      await result.current.submit();
    });

    const request = vi.mocked(gptImageAdapter.generate).mock.calls[0][0];
    expect(request.referenceImages).toHaveLength(1);
    expect(request.referenceImages[0].mime).toBe("image/png");
    // 提交后参考图清空。
    expect(result.current.referenceImages).toHaveLength(0);
  });

  it("removes a reference image and revokes its object url", async () => {
    const { result } = await renderController();
    await act(async () => {
      await result.current.addReferenceFiles([makePngFile()]);
    });
    const target = result.current.referenceImages[0];
    act(() => {
      result.current.removeReferenceImage(target.id);
    });
    expect(result.current.referenceImages).toHaveLength(0);
    expect(URL.revokeObjectURL).toHaveBeenCalledWith(target.objectUrl);
  });

  it("uses a generated image as the next reference image", async () => {
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("gen") }],
    });
    const { result } = await renderController();
    act(() => {
      result.current.setPrompt("先生成");
    });
    await act(async () => {
      await result.current.submit();
    });
    const assistant = result.current.messages[1] as ImageGenAssistantMessage;

    await act(async () => {
      await result.current.setAsReference(assistant.images[0]);
    });
    expect(result.current.referenceImages).toHaveLength(1);
    expect(result.current.referenceImages[0].mime).toBe("image/png");
    expect(toast.success).toHaveBeenCalledWith("已设为参考图");
  });

  it("downloads a generated image through the save dialog", async () => {
    vi.mocked(saveDesktopFilePath).mockResolvedValue("/tmp/out.png");
    vi.mocked(imageGenSaveImage).mockResolvedValue(true);
    const { result } = await renderController();

    await act(async () => {
      await result.current.downloadImage({
        objectUrl: "blob:x",
        mime: "image/png",
        blob: base64ToBlob(btoa("gen"), "image/png"),
      });
    });

    expect(saveDesktopFilePath).toHaveBeenCalledWith(
      expect.objectContaining({ title: "保存图片" })
    );
    expect(imageGenSaveImage).toHaveBeenCalledWith("/tmp/out.png", btoa("gen"));
    expect(toast.success).toHaveBeenCalledWith("图片已保存");
  });

  it("aborts download when the save dialog is cancelled", async () => {
    vi.mocked(saveDesktopFilePath).mockResolvedValue(null);
    const { result } = await renderController();
    await act(async () => {
      await result.current.downloadImage({
        objectUrl: "blob:x",
        mime: "image/png",
        blob: base64ToBlob(btoa("gen"), "image/png"),
      });
    });
    expect(imageGenSaveImage).not.toHaveBeenCalled();
  });

  it("toasts when download fails", async () => {
    vi.mocked(saveDesktopFilePath).mockResolvedValue("/tmp/out.png");
    vi.mocked(imageGenSaveImage).mockRejectedValue(new Error("写盘失败"));
    const { result } = await renderController();
    await act(async () => {
      await result.current.downloadImage({
        objectUrl: "blob:x",
        mime: "image/png",
        blob: base64ToBlob(btoa("gen"), "image/png"),
      });
    });
    expect(toast.error).toHaveBeenCalledWith("保存图片失败：请查看控制台日志");
  });

  it("validates required fields before saving config", async () => {
    const { result } = await renderController();
    await act(async () => {
      await result.current.saveConfig();
    });
    expect(toast.error).toHaveBeenCalledWith("请填写 Base URL");
    expect(imageGenConfigSet).not.toHaveBeenCalled();

    act(() => {
      result.current.setBaseUrl("api.example.com");
      result.current.setModel("  ");
    });
    await act(async () => {
      await result.current.saveConfig();
    });
    expect(toast.error).toHaveBeenCalledWith("请填写模型 ID");
    expect(imageGenConfigSet).not.toHaveBeenCalled();
  });

  it("saves config with a normalized base url and a new api key", async () => {
    vi.mocked(imageGenConfigSet).mockResolvedValue({
      adapterId: "gpt-image",
      baseUrl: "https://api.example.com/v1",
      model: "gpt-image-2",
      apiKeyConfigured: true,
    });
    const { result } = await renderController();

    act(() => {
      result.current.setBaseUrl("api.example.com");
      result.current.setApiKeyDraft("sk-new");
    });
    await act(async () => {
      await result.current.saveConfig();
    });

    expect(imageGenConfigSet).toHaveBeenCalledWith(
      "gpt-image",
      "https://api.example.com/v1",
      "gpt-image-2",
      "sk-new"
    );
    expect(result.current.apiKeyConfigured).toBe(true);
    expect(result.current.apiKeyDraft).toBe("");
    expect(toast.success).toHaveBeenCalledWith("生图配置已保存");
  });

  it("preserves the stored key by passing null when the draft is empty", async () => {
    vi.mocked(imageGenConfigSet).mockResolvedValue({
      adapterId: "gpt-image",
      baseUrl: "https://api.example.com/v1",
      model: "gpt-image-2",
      apiKeyConfigured: true,
    });
    const { result } = await renderController();
    act(() => {
      result.current.setBaseUrl("https://api.example.com/v1");
    });
    await act(async () => {
      await result.current.saveConfig();
    });
    expect(imageGenConfigSet).toHaveBeenCalledWith(
      "gpt-image",
      "https://api.example.com/v1",
      "gpt-image-2",
      null
    );
  });

  it("toasts when saving config fails", async () => {
    vi.mocked(imageGenConfigSet).mockRejectedValue(new Error("db"));
    const { result } = await renderController();
    act(() => {
      result.current.setBaseUrl("https://api.example.com/v1");
    });
    await act(async () => {
      await result.current.saveConfig();
    });
    expect(toast.error).toHaveBeenCalledWith("保存生图配置失败：请查看控制台日志");
    expect(result.current.savingConfig).toBe(false);
  });

  it("persists generation params to localStorage and restores them", async () => {
    const { result, unmount } = await renderController();
    act(() => {
      result.current.updateParams({ n: 4, outputFormat: "jpeg", outputCompression: 80 });
    });
    await waitFor(() => {
      expect(readParamsFromStorage()).toMatchObject({ n: 4, outputFormat: "jpeg" });
    });
    unmount();

    const { result: restored } = await renderController();
    expect(restored.current.params).toMatchObject({
      n: 4,
      outputFormat: "jpeg",
      outputCompression: 80,
    });
  });

  it("falls back to defaults for corrupted localStorage payloads", () => {
    window.localStorage.setItem("aio-image-gen-params", "not-json{");
    expect(readParamsFromStorage()).toEqual(DEFAULT_IMAGE_GEN_PARAMS);
    window.localStorage.setItem("aio-image-gen-params", '"just-a-string"');
    expect(readParamsFromStorage()).toEqual(DEFAULT_IMAGE_GEN_PARAMS);
  });

  it("validateReferenceAddition enforces count and byte budgets", () => {
    expect(validateReferenceAddition(0, 0, 16, 1024)).toBeNull();
    expect(validateReferenceAddition(16, 0, 1, 1)).toBe("参考图最多 16 张");
    expect(validateReferenceAddition(0, 0, 1, 30 * 1024 * 1024 + 1)).toBe(
      "参考图合计不能超过 30MB"
    );
  });

  it("keeps the session across unmount/remount without revoking urls (regression: 路由懒加载卸载)", async () => {
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("img") }],
    });
    const { result, unmount } = await renderController();

    act(() => {
      result.current.setPrompt("一只猫");
    });
    await act(async () => {
      await result.current.submit();
    });
    await act(async () => {
      await result.current.addReferenceFiles([makePngFile()]);
    });
    act(() => {
      result.current.setPrompt("草稿");
    });
    const refUrl = result.current.referenceImages[0].objectUrl;
    const generatedUrl = (result.current.messages[1] as ImageGenAssistantMessage).images[0]
      .objectUrl;

    unmount();
    // 卸载不再全量 revoke（URL 生命周期为应用会话级）。
    expect(URL.revokeObjectURL).not.toHaveBeenCalled();

    const { result: restored } = await renderController();
    expect(restored.current.messages).toHaveLength(2);
    const assistant = restored.current.messages[1] as ImageGenAssistantMessage;
    expect(assistant.status).toBe("done");
    expect(assistant.images[0].objectUrl).toBe(generatedUrl);
    expect(restored.current.prompt).toBe("草稿");
    expect(restored.current.referenceImages).toHaveLength(1);
    expect(restored.current.referenceImages[0].objectUrl).toBe(refUrl);
  });

  it("runs two submissions concurrently without cross-talk", async () => {
    const deferreds: Array<(value: ImageGenResult) => void> = [];
    vi.mocked(gptImageAdapter.generate).mockImplementation(
      () =>
        new Promise<ImageGenResult>((resolve) => {
          deferreds.push(resolve);
        })
    );
    const { result } = await renderController();

    act(() => {
      result.current.setPrompt("第一张");
    });
    await act(async () => {
      void result.current.submit();
    });
    act(() => {
      result.current.setPrompt("第二张");
    });
    await act(async () => {
      void result.current.submit();
    });

    // 两条 loading assistant 消息共存。
    expect(result.current.messages).toHaveLength(4);
    const firstId = result.current.messages[1].id;
    const secondId = result.current.messages[3].id;
    expect((result.current.messages[1] as ImageGenAssistantMessage).status).toBe("loading");
    expect((result.current.messages[3] as ImageGenAssistantMessage).status).toBe("loading");
    expect(deferreds).toHaveLength(2);

    // 先完成第二个：第一条仍 loading，互不阻塞。
    await act(async () => {
      deferreds[1]({
        images: [{ mime: "image/png", b64: btoa("two") }],
        usage: { totalTokens: 2 },
      });
    });
    let first = result.current.messages.find((m) => m.id === firstId) as ImageGenAssistantMessage;
    let second = result.current.messages.find((m) => m.id === secondId) as ImageGenAssistantMessage;
    expect(first.status).toBe("loading");
    expect(second.status).toBe("done");
    expect(second.usage).toEqual({ totalTokens: 2 });

    await act(async () => {
      deferreds[0]({
        images: [{ mime: "image/png", b64: btoa("one") }],
        usage: { totalTokens: 1 },
      });
    });
    first = result.current.messages.find((m) => m.id === firstId) as ImageGenAssistantMessage;
    second = result.current.messages.find((m) => m.id === secondId) as ImageGenAssistantMessage;
    expect(first.status).toBe("done");
    expect(first.usage).toEqual({ totalTokens: 1 });
    expect(second.usage).toEqual({ totalTokens: 2 });
    expect(first.images[0].objectUrl).not.toBe(second.images[0].objectUrl);
  });

  it("finishes an in-flight generation after unmount and writes the result to the store", async () => {
    let resolveGen!: (value: ImageGenResult) => void;
    vi.mocked(gptImageAdapter.generate).mockImplementation(
      () =>
        new Promise<ImageGenResult>((resolve) => {
          resolveGen = resolve;
        })
    );
    const { result, unmount } = await renderController();
    act(() => {
      result.current.setPrompt("后台完成");
    });
    await act(async () => {
      void result.current.submit();
    });
    unmount();

    await act(async () => {
      resolveGen({ images: [{ mime: "image/png", b64: btoa("bg") }] });
    });
    const assistant = getImageGenSession().messages[1] as ImageGenAssistantMessage;
    expect(assistant.status).toBe("done");
    expect(assistant.images).toHaveLength(1);
  });

  it("ignores retry while the target message is still loading", async () => {
    vi.mocked(gptImageAdapter.generate).mockImplementation(
      () => new Promise<ImageGenResult>(() => {})
    );
    const { result } = await renderController();
    act(() => {
      result.current.setPrompt("生成中");
    });
    await act(async () => {
      void result.current.submit();
    });
    const assistantId = result.current.messages[1].id;

    await act(async () => {
      await result.current.retry(assistantId);
    });
    expect(gptImageAdapter.generate).toHaveBeenCalledTimes(1);
  });

  it("opens, steps (wrapping) and closes the preview", async () => {
    const { result } = await renderController();

    act(() => {
      result.current.openPreview(["blob:a", "blob:b", "blob:c"], 2);
    });
    expect(result.current.preview).toEqual({ urls: ["blob:a", "blob:b", "blob:c"], index: 2 });

    // 向后越界回绕到第一张，向前越界回绕到最后一张。
    act(() => {
      result.current.stepPreview(1);
    });
    expect(result.current.preview?.index).toBe(0);
    act(() => {
      result.current.stepPreview(-1);
    });
    expect(result.current.preview?.index).toBe(2);

    act(() => {
      result.current.closePreview();
    });
    expect(result.current.preview).toBeNull();
  });

  it("stepPreview is a no-op when no preview is open", async () => {
    const { result } = await renderController();
    act(() => {
      result.current.stepPreview(1);
    });
    expect(result.current.preview).toBeNull();
  });
});
