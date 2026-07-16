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
    expect(result.current.generating).toBe(false);
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

  it("revokes all created object urls on unmount", async () => {
    const { result, unmount } = await renderController();
    await act(async () => {
      await result.current.addReferenceFiles([makePngFile()]);
    });
    const url = result.current.referenceImages[0].objectUrl;
    unmount();
    expect(URL.revokeObjectURL).toHaveBeenCalledWith(url);
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
