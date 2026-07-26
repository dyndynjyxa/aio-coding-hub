import { act, fireEvent, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { gptImageAdapter } from "../../../services/image-gen/gptImageAdapter";
import {
  imageGenConfigGet,
  imageGenConfigSet,
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
} from "../../../services/image-gen/service";
import { openDesktopSinglePath, saveDesktopFilePath } from "../../../services/desktop/dialog";
import type { ImageGenResult } from "../../../services/image-gen/types";
import { getImageGenSession, resetImageGenSessionForTests } from "../imageGenSessionStore";
import {
  base64ToBlob,
  DEFAULT_IMAGE_GEN_PARAMS,
  extractClipboardImageFiles,
  filterTasks,
  HISTORY_PAGE_SIZE,
  readParamsFromStorage,
  useImageGenController,
  validateReferenceAddition,
  type ImageGenTask,
  type ImageGenTaskImage,
} from "../useImageGenController";
import { makeTask } from "./testUtils";

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
    imageGenTaskPersist: vi.fn(),
    imageGenTasksList: vi.fn(),
    imageGenTaskDelete: vi.fn(),
    imageGenTasksClear: vi.fn(),
    imageGenReadImage: vi.fn(),
    imageGenStorageGet: vi.fn(),
    imageGenStorageSetDir: vi.fn(),
    imageGenStorageCleanup: vi.fn(),
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
  openDesktopSinglePath: vi.fn(),
}));

const EMPTY_CONFIG = {
  adapterId: "gpt-image",
  baseUrl: "",
  model: "",
  apiKeyConfigured: false,
};

const CONFIGURED_CONFIG = {
  adapterId: "gpt-image",
  baseUrl: "https://api.example.com/v1",
  model: "gpt-image-2",
  apiKeyConfigured: true,
};

const STORAGE_VIEW = {
  dir: "/Users/tester/.aio-coding-hub/image-gen",
  totalBytes: 2048,
  taskCount: 3,
};

function makePngFile(name = "ref.png", sizeBytes?: number): File {
  const file = new File(["fake-image-bytes"], name, { type: "image/png" });
  if (sizeBytes != null) {
    Object.defineProperty(file, "size", { value: sizeBytes });
  }
  return file;
}

function makeGeneratedImage(): ImageGenTaskImage {
  return {
    kind: "memory",
    objectUrl: "blob:external",
    mime: "image/png",
    blob: new Blob(["x"], { type: "image/png" }),
  };
}

/** 收窄为 memory 形态（断言辅助）。 */
function memoryImage(image: ImageGenTaskImage) {
  if (image.kind !== "memory") throw new Error(`expected memory image, got ${image.kind}`);
  return image;
}

/** 收窄为 disk 形态（断言辅助）。 */
function diskImage(image: ImageGenTaskImage) {
  if (image.kind !== "disk") throw new Error(`expected disk image, got ${image.kind}`);
  return image;
}

/** 从 persist payload 构造 Rust 会返回的行视图（路径按任务目录布局）。 */
function rowFromPayload(payload: ImageGenTaskPersistPayload): ImageGenTaskRow {
  const dir = `/store/${payload.id}`;
  return {
    id: payload.id,
    adapterId: payload.adapterId ?? "gpt-image",
    prompt: payload.prompt,
    requestJson: payload.requestJson,
    status: payload.status,
    error: payload.error,
    usageJson: payload.usageJson,
    images: payload.images.map((image, index) => ({
      path: `${dir}/image-${index + 1}.png`,
      thumbPath: payload.thumbs[index] ? `${dir}/thumb-${index + 1}.webp` : null,
      mime: image.mime,
    })),
    refImages: payload.refImages.map((ref, index) => ({
      path: `${dir}/ref-${index + 1}.png`,
      thumbPath: null,
      mime: ref.mime,
    })),
    dir,
    createdAt: payload.createdAt,
    elapsedMs: payload.elapsedMs,
  };
}

const ROW_REQUEST_JSON = JSON.stringify({
  prompt: "历史任务",
  referenceImages: [],
  n: 1,
  size: "auto",
  options: {
    model: "gpt-image-2",
    quality: "auto",
    outputFormat: "png",
    outputCompression: null,
    moderation: "auto",
  },
});

function makeRow(overrides: Partial<ImageGenTaskRow> = {}): ImageGenTaskRow {
  return {
    id: "row-1",
    adapterId: "gpt-image",
    prompt: "历史任务",
    requestJson: ROW_REQUEST_JSON,
    status: "done",
    error: null,
    usageJson: null,
    images: [
      {
        path: "/store/row-1/image-1.png",
        thumbPath: "/store/row-1/thumb-1.webp",
        mime: "image/png",
      },
    ],
    refImages: [],
    dir: "/store/row-1",
    createdAt: 1_700_000_000_000,
    elapsedMs: 1200,
    ...overrides,
  };
}

/** 含落盘参考图的行：快照占位无 b64，refImages 提供读回路径。 */
function makeRowWithRef(): ImageGenTaskRow {
  return makeRow({
    requestJson: JSON.stringify({
      prompt: "历史任务",
      referenceImages: [{ file: "ref-1.png", mime: "image/png" }],
      n: 1,
      size: "auto",
      options: {
        model: "gpt-image-2",
        quality: "auto",
        outputFormat: "png",
        outputCompression: null,
        moderation: "auto",
      },
    }),
    refImages: [{ path: "/store/row-1/ref-1.png", thumbPath: null, mime: "image/png" }],
  });
}

async function renderController() {
  const rendered = renderHook(() => useImageGenController());
  await waitFor(() => {
    expect(imageGenConfigGet).toHaveBeenCalled();
  });
  // 冲刷配置/存储/历史加载 promise，保证提交守卫读到已回填的连接配置。
  await act(async () => {});
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
    // 默认返回已配置视图：提交守卫（Base URL + API Key）在多数用例中应放行。
    vi.mocked(imageGenConfigGet).mockResolvedValue(CONFIGURED_CONFIG);
    // 历史/存储默认空态；persist 默认悬挂（任务保持 memory 形态，落盘流单独测试）。
    vi.mocked(imageGenTasksList).mockResolvedValue([]);
    vi.mocked(imageGenStorageGet).mockResolvedValue(STORAGE_VIEW);
    vi.mocked(imageGenTaskPersist).mockImplementation(() => new Promise<ImageGenTaskRow>(() => {}));
    vi.mocked(imageGenTaskDelete).mockResolvedValue(null);
    vi.mocked(imageGenTasksClear).mockResolvedValue(0);
    vi.mocked(imageGenStorageCleanup).mockResolvedValue(0);
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

  it("submits a text-to-image request as a single task that transitions to done", async () => {
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
    expect(result.current.tasks).toHaveLength(1);
    const task = result.current.tasks[0];
    expect(task).toMatchObject({ prompt: "一只猫", status: "done", persisted: false });
    expect(task.images).toHaveLength(1);
    expect(task.usage).toEqual({ totalTokens: 42 });
    expect(task.createdAt).toBe(task.startedAt);
    expect(task.elapsedMs).toBeGreaterThanOrEqual(0);
    expect(result.current.prompt).toBe("");
  });

  it("does nothing for an empty prompt", async () => {
    const { result } = await renderController();
    await act(async () => {
      await result.current.submit();
    });
    expect(gptImageAdapter.generate).not.toHaveBeenCalled();
    expect(result.current.tasks).toHaveLength(0);
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

    const failed = result.current.tasks[0];
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
    const retried = result.current.tasks[0];
    expect(retried.status).toBe("done");
    expect(retried.error).toBeUndefined();
  });

  it("retry resets startedAt while keeping createdAt", async () => {
    vi.mocked(gptImageAdapter.generate).mockRejectedValueOnce(new Error("boom"));
    const { result } = await renderController();
    act(() => {
      result.current.setPrompt("失败");
    });
    await act(async () => {
      await result.current.submit();
    });
    const failed = result.current.tasks[0];
    const { createdAt, startedAt } = failed;

    const nowSpy = vi.spyOn(Date, "now").mockReturnValue(startedAt + 5000);
    vi.mocked(gptImageAdapter.generate).mockResolvedValueOnce({
      images: [{ mime: "image/png", b64: btoa("ok") }],
    });
    await act(async () => {
      await result.current.retry(failed.id);
    });
    nowSpy.mockRestore();

    const retried = result.current.tasks[0];
    expect(retried.createdAt).toBe(createdAt);
    expect(retried.startedAt).toBe(startedAt + 5000);
    // Date.now 被固定：完成时刻 - startedAt = 0。
    expect(retried.elapsedMs).toBe(0);
    expect(retried.status).toBe("done");
  });

  it("deletes a done memory task locally and releases its object urls (refThumbs + images)", async () => {
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("img") }],
    });
    const { result } = await renderController();
    await act(async () => {
      await result.current.addReferenceFiles([makePngFile()]);
    });
    const refUrl = result.current.referenceImages[0].objectUrl;
    act(() => {
      result.current.setPrompt("待删除");
    });
    await act(async () => {
      await result.current.submit();
    });
    const task = result.current.tasks[0];
    const generatedUrl = memoryImage(task.images[0]).objectUrl;
    expect(task.refThumbs).toEqual([refUrl]);

    act(() => {
      result.current.deleteTask(task.id);
    });
    expect(result.current.tasks).toHaveLength(0);
    expect(URL.revokeObjectURL).toHaveBeenCalledWith(refUrl);
    expect(URL.revokeObjectURL).toHaveBeenCalledWith(generatedUrl);
    // memory 任务不走后端删除命令。
    expect(imageGenTaskDelete).not.toHaveBeenCalled();
  });

  it("drops the in-flight result after the loading task is deleted (no resurrection, no leak)", async () => {
    let resolveGen!: (value: ImageGenResult) => void;
    vi.mocked(gptImageAdapter.generate).mockImplementation(
      () =>
        new Promise<ImageGenResult>((resolve) => {
          resolveGen = resolve;
        })
    );
    const { result } = await renderController();
    act(() => {
      result.current.setPrompt("删我");
    });
    await act(async () => {
      void result.current.submit();
    });
    const taskId = result.current.tasks[0].id;

    act(() => {
      result.current.deleteTask(taskId);
    });
    expect(result.current.tasks).toHaveLength(0);
    const urlsCreatedBefore = vi.mocked(URL.createObjectURL).mock.calls.length;

    await act(async () => {
      resolveGen({ images: [{ mime: "image/png", b64: btoa("late") }] });
    });
    // 任务不复活，且迟到结果未创建任何 objectURL（无泄漏）。
    expect(getImageGenSession().tasks.some((task) => task.id === taskId)).toBe(false);
    expect(vi.mocked(URL.createObjectURL).mock.calls.length).toBe(urlsCreatedBefore);
  });

  it("reuses a task config: prompt, params, model and rebuilt reference images", async () => {
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("img") }],
    });
    const { result } = await renderController();

    await act(async () => {
      await result.current.addReferenceFiles([makePngFile()]);
    });
    const refB64 = result.current.referenceImages[0].b64;
    act(() => {
      result.current.updateParams({ n: 2, size: "1024x1024", quality: "high" });
      result.current.setModel("gpt-image-2-2026-04-21");
      result.current.setPrompt("原始提示词");
    });
    await act(async () => {
      await result.current.submit();
    });
    const task = result.current.tasks[0];

    // 改动面板与输入，证明复用回填的是快照值。
    act(() => {
      result.current.updateParams({ n: 9, size: "auto", quality: "low" });
      result.current.setModel("other-model");
      result.current.setPrompt("新草稿");
    });

    await act(async () => {
      await result.current.reuseTask(task.id);
    });

    expect(result.current.prompt).toBe("原始提示词");
    expect(result.current.params).toMatchObject({ n: 2, size: "1024x1024", quality: "high" });
    expect(result.current.model).toBe("gpt-image-2-2026-04-21");
    expect(result.current.referenceImages).toHaveLength(1);
    expect(result.current.referenceImages[0].b64).toBe(refB64);
    expect(toast.success).toHaveBeenCalledWith("已复用配置");
  });

  it("reuseTask ignores unknown task ids", async () => {
    const { result } = await renderController();
    await act(async () => {
      await result.current.reuseTask("missing");
    });
    expect(toast.success).not.toHaveBeenCalled();
  });

  it("clears all tasks through the backend and releases every object url", async () => {
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("img") }],
    });
    const { result } = await renderController();
    act(() => {
      result.current.setPrompt("第一张");
    });
    await act(async () => {
      await result.current.submit();
    });
    act(() => {
      result.current.setPrompt("第二张");
    });
    await act(async () => {
      await result.current.submit();
    });
    const urls = result.current.tasks.flatMap((task) =>
      task.images.map((image) => memoryImage(image).objectUrl)
    );
    expect(urls).toHaveLength(2);

    await act(async () => {
      await result.current.clearTasks();
    });
    expect(imageGenTasksClear).toHaveBeenCalled();
    expect(result.current.tasks).toHaveLength(0);
    for (const url of urls) {
      expect(URL.revokeObjectURL).toHaveBeenCalledWith(url);
    }
    expect(result.current.preview).toBeNull();
    expect(toast.success).toHaveBeenCalledWith("已清空任务");
  });

  it("keeps tasks and toasts when clearing fails in the backend", async () => {
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("img") }],
    });
    vi.mocked(imageGenTasksClear).mockRejectedValue(new Error("io"));
    const { result } = await renderController();
    act(() => {
      result.current.setPrompt("清不掉");
    });
    await act(async () => {
      await result.current.submit();
    });

    await act(async () => {
      await result.current.clearTasks();
    });
    expect(toast.error).toHaveBeenCalledWith("清空任务失败：请查看控制台日志");
    expect(result.current.tasks).toHaveLength(1);
    expect(toast.success).not.toHaveBeenCalledWith("已清空任务");
  });

  it("opens and closes the task detail, deriving the task from the store", async () => {
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("img") }],
    });
    const { result } = await renderController();
    act(() => {
      result.current.setPrompt("详情");
    });
    await act(async () => {
      await result.current.submit();
    });
    const task = result.current.tasks[0];

    act(() => {
      result.current.openDetail(task.id);
    });
    expect(result.current.detailTask?.id).toBe(task.id);

    // 详情打开期间任务被删除 → detailTask 变 null（弹窗不渲染）。
    act(() => {
      result.current.deleteTask(task.id);
    });
    expect(result.current.detailTask).toBeNull();

    act(() => {
      result.current.closeDetail();
    });
    expect(result.current.detailTask).toBeNull();
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
    const task = result.current.tasks[0];

    await act(async () => {
      await result.current.setAsReference(task.images[0]);
    });
    expect(result.current.referenceImages).toHaveLength(1);
    expect(result.current.referenceImages[0].mime).toBe("image/png");
    expect(toast.success).toHaveBeenCalledWith("已设为参考图");
  });

  it("blocks setAsReference at the 16-image limit", async () => {
    const { result } = await renderController();
    const files = Array.from({ length: 16 }, (_, index) => makePngFile(`f${index}.png`));
    await act(async () => {
      await result.current.addReferenceFiles(files);
    });
    expect(result.current.referenceImages).toHaveLength(16);
    const before = result.current.referenceImages;

    await act(async () => {
      await result.current.setAsReference(makeGeneratedImage());
    });
    expect(toast.error).toHaveBeenCalledWith("参考图最多 16 张");
    expect(result.current.referenceImages).toBe(before);
    expect(toast.success).not.toHaveBeenCalledWith("已设为参考图");
  });

  it("blocks setAsReference above the 30MB total budget", async () => {
    const { result } = await renderController();
    await act(async () => {
      await result.current.addReferenceFiles([makePngFile("big.png", 30 * 1024 * 1024)]);
    });
    expect(result.current.referenceImages).toHaveLength(1);

    await act(async () => {
      await result.current.setAsReference(makeGeneratedImage());
    });
    expect(toast.error).toHaveBeenCalledWith("参考图合计不能超过 30MB");
    expect(result.current.referenceImages).toHaveLength(1);
  });

  it("updates error and elapsed across consecutive failed retries without leaking urls", async () => {
    vi.mocked(gptImageAdapter.generate).mockRejectedValueOnce(new Error("第一次失败"));
    const { result } = await renderController();
    act(() => {
      result.current.setPrompt("连续失败");
    });
    await act(async () => {
      await result.current.submit();
    });
    const first = result.current.tasks[0];
    expect(first.status).toBe("error");
    expect(first.error).toContain("第一次失败");
    const urlsAfterSubmit = vi.mocked(URL.createObjectURL).mock.calls.length;

    // 第二次失败：固定时钟证明 startedAt/elapsedMs 随重试更新。
    const base = first.startedAt;
    vi.mocked(gptImageAdapter.generate).mockRejectedValueOnce(new Error("第二次失败"));
    const nowSpy = vi
      .spyOn(Date, "now")
      .mockReturnValueOnce(base + 5000)
      .mockReturnValue(base + 6234);
    await act(async () => {
      await result.current.retry(first.id);
    });
    nowSpy.mockRestore();
    const second = result.current.tasks[0];
    expect(second.status).toBe("error");
    expect(second.error).toContain("第二次失败");
    expect(second.startedAt).toBe(base + 5000);
    expect(second.elapsedMs).toBe(1234);

    // 第三次失败：错误继续更新，三次调用共享同一份请求快照。
    vi.mocked(gptImageAdapter.generate).mockRejectedValueOnce(new Error("第三次失败"));
    await act(async () => {
      await result.current.retry(second.id);
    });
    const third = result.current.tasks[0];
    expect(third.error).toContain("第三次失败");
    const calls = vi.mocked(gptImageAdapter.generate).mock.calls;
    expect(calls).toHaveLength(3);
    expect(calls[1][0]).toBe(calls[0][0]);
    expect(calls[2][0]).toBe(calls[0][0]);
    // 失败路径从不创建生成图 objectURL（无泄漏）。
    expect(vi.mocked(URL.createObjectURL).mock.calls.length).toBe(urlsAfterSubmit);
  });

  it("closes the preview when the deleted task owns the previewed image", async () => {
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("img") }],
    });
    const { result } = await renderController();
    act(() => {
      result.current.setPrompt("预览中删除");
    });
    await act(async () => {
      await result.current.submit();
    });
    const task = result.current.tasks[0];
    act(() => {
      result.current.openPreview(
        task.images.map((image) => memoryImage(image).objectUrl),
        0
      );
    });
    expect(result.current.preview).not.toBeNull();

    act(() => {
      result.current.deleteTask(task.id);
    });
    expect(result.current.preview).toBeNull();
  });

  it("keeps the preview open when deleting an unrelated task", async () => {
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("img") }],
    });
    const { result } = await renderController();
    act(() => {
      result.current.setPrompt("第一张");
    });
    await act(async () => {
      await result.current.submit();
    });
    act(() => {
      result.current.setPrompt("第二张");
    });
    await act(async () => {
      await result.current.submit();
    });
    const [taskA, taskB] = result.current.tasks;
    const previewUrl = memoryImage(taskB.images[0]).objectUrl;
    act(() => {
      result.current.openPreview([previewUrl], 0);
    });

    act(() => {
      result.current.deleteTask(taskA.id);
    });
    expect(result.current.preview).toEqual({ urls: [previewUrl], index: 0 });
  });

  it("downloads a generated memory image through the save dialog", async () => {
    vi.mocked(saveDesktopFilePath).mockResolvedValue("/tmp/out.png");
    vi.mocked(imageGenSaveImage).mockResolvedValue(true);
    const { result } = await renderController();

    await act(async () => {
      await result.current.downloadImage({
        kind: "memory",
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
        kind: "memory",
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
        kind: "memory",
        objectUrl: "blob:x",
        mime: "image/png",
        blob: base64ToBlob(btoa("gen"), "image/png"),
      });
    });
    expect(toast.error).toHaveBeenCalledWith("保存图片失败：请查看控制台日志");
  });

  it("auto-saves on blur with a normalized base url and a new api key, silently on success", async () => {
    vi.mocked(imageGenConfigGet).mockResolvedValue(EMPTY_CONFIG);
    vi.mocked(imageGenConfigSet).mockResolvedValue(CONFIGURED_CONFIG);
    const { result } = await renderController();

    act(() => {
      result.current.setBaseUrl("api.example.com");
      result.current.setApiKeyDraft("sk-new");
    });
    await act(async () => {
      await result.current.autoSaveConfig();
    });

    expect(imageGenConfigSet).toHaveBeenCalledWith(
      "gpt-image",
      "https://api.example.com/v1",
      "gpt-image-2",
      "sk-new"
    );
    // 成功回填规整后的视图并清空草稿，且静默无 toast。
    expect(result.current.baseUrl).toBe("https://api.example.com/v1");
    expect(result.current.apiKeyConfigured).toBe(true);
    expect(result.current.apiKeyDraft).toBe("");
    expect(toast.success).not.toHaveBeenCalled();
    expect(toast.error).not.toHaveBeenCalled();
  });

  it("skips auto-save silently when the base url is empty", async () => {
    vi.mocked(imageGenConfigGet).mockResolvedValue(EMPTY_CONFIG);
    const { result } = await renderController();
    await act(async () => {
      await result.current.autoSaveConfig();
    });
    expect(imageGenConfigSet).not.toHaveBeenCalled();
    expect(toast.error).not.toHaveBeenCalled();
  });

  it("auto-save preserves the stored key with null and falls back to the default model", async () => {
    vi.mocked(imageGenConfigSet).mockResolvedValue(CONFIGURED_CONFIG);
    const { result } = await renderController();
    act(() => {
      result.current.setModel("   ");
    });
    await act(async () => {
      await result.current.autoSaveConfig();
    });
    expect(imageGenConfigSet).toHaveBeenCalledWith(
      "gpt-image",
      "https://api.example.com/v1",
      "gpt-image-2",
      null
    );
  });

  it("toasts when auto-save fails", async () => {
    vi.mocked(imageGenConfigSet).mockRejectedValue(new Error("db"));
    const { result } = await renderController();
    act(() => {
      result.current.setBaseUrl("https://api.example.com/v1");
    });
    await act(async () => {
      await result.current.autoSaveConfig();
    });
    expect(toast.error).toHaveBeenCalledWith("保存生图配置失败：请查看控制台日志");
  });

  it("blocks submit with a toast when the connection config is missing", async () => {
    vi.mocked(imageGenConfigGet).mockResolvedValue(EMPTY_CONFIG);
    const { result } = await renderController();
    act(() => {
      result.current.setPrompt("一只猫");
    });
    await act(async () => {
      await result.current.submit();
    });
    expect(toast.error).toHaveBeenCalledWith("请先在左侧完成连接配置（Base URL 与 API Key）");
    expect(gptImageAdapter.generate).not.toHaveBeenCalled();
    expect(result.current.tasks).toHaveLength(0);
  });

  it("blocks submit when the base url exists but no api key is configured or drafted", async () => {
    vi.mocked(imageGenConfigGet).mockResolvedValue({
      ...EMPTY_CONFIG,
      baseUrl: "https://api.example.com/v1",
    });
    const { result } = await renderController();
    act(() => {
      result.current.setPrompt("一只猫");
    });
    await act(async () => {
      await result.current.submit();
    });
    expect(toast.error).toHaveBeenCalledWith("请先在左侧完成连接配置（Base URL 与 API Key）");
    expect(gptImageAdapter.generate).not.toHaveBeenCalled();
    expect(result.current.tasks).toHaveLength(0);
  });

  it("allows submit with an unsaved api key draft", async () => {
    vi.mocked(imageGenConfigGet).mockResolvedValue({
      ...EMPTY_CONFIG,
      baseUrl: "https://api.example.com/v1",
    });
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("img") }],
    });
    const { result } = await renderController();
    act(() => {
      result.current.setApiKeyDraft("sk-draft");
      result.current.setPrompt("一只猫");
    });
    await act(async () => {
      await result.current.submit();
    });
    expect(gptImageAdapter.generate).toHaveBeenCalled();
    expect(result.current.tasks).toHaveLength(1);
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

  it("filterTasks matches prompt case-insensitively, filters by status and returns newest first", () => {
    const tasks: ImageGenTask[] = [
      makeTask({ id: "t1", prompt: "A Cat", status: "done" }),
      makeTask({ id: "t2", prompt: "a dog", status: "error" }),
      makeTask({ id: "t3", prompt: "Cat and dog", status: "loading" }),
    ];

    // 空 query + all：全量，新的在前（store 追加序反转）。
    expect(filterTasks(tasks, "", "all").map((task) => task.id)).toEqual(["t3", "t2", "t1"]);
    // 大小写不敏感子串匹配。
    expect(filterTasks(tasks, "CAT", "all").map((task) => task.id)).toEqual(["t3", "t1"]);
    // 状态过滤。
    expect(filterTasks(tasks, "", "error").map((task) => task.id)).toEqual(["t2"]);
    // 组合：query + 状态。
    expect(filterTasks(tasks, "cat", "loading").map((task) => task.id)).toEqual(["t3"]);
    expect(filterTasks(tasks, "无命中", "all")).toEqual([]);
    // 纯函数：不改写入参顺序。
    expect(tasks.map((task) => task.id)).toEqual(["t1", "t2", "t3"]);
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
    const generatedUrl = memoryImage(result.current.tasks[0].images[0]).objectUrl;

    unmount();
    // 卸载不再全量 revoke（URL 生命周期为应用会话级）。
    expect(URL.revokeObjectURL).not.toHaveBeenCalled();

    const { result: restored } = await renderController();
    expect(restored.current.tasks).toHaveLength(1);
    expect(restored.current.tasks[0].status).toBe("done");
    expect(memoryImage(restored.current.tasks[0].images[0]).objectUrl).toBe(generatedUrl);
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

    // 两条 loading 任务共存。
    expect(result.current.tasks).toHaveLength(2);
    const firstId = result.current.tasks[0].id;
    const secondId = result.current.tasks[1].id;
    expect(result.current.tasks[0].status).toBe("loading");
    expect(result.current.tasks[1].status).toBe("loading");
    expect(deferreds).toHaveLength(2);

    // 先完成第二个：第一条仍 loading，互不阻塞。
    await act(async () => {
      deferreds[1]({
        images: [{ mime: "image/png", b64: btoa("two") }],
        usage: { totalTokens: 2 },
      });
    });
    let first = result.current.tasks.find((task) => task.id === firstId)!;
    let second = result.current.tasks.find((task) => task.id === secondId)!;
    expect(first.status).toBe("loading");
    expect(second.status).toBe("done");
    expect(second.usage).toEqual({ totalTokens: 2 });

    await act(async () => {
      deferreds[0]({
        images: [{ mime: "image/png", b64: btoa("one") }],
        usage: { totalTokens: 1 },
      });
    });
    first = result.current.tasks.find((task) => task.id === firstId)!;
    second = result.current.tasks.find((task) => task.id === secondId)!;
    expect(first.status).toBe("done");
    expect(first.usage).toEqual({ totalTokens: 1 });
    expect(second.usage).toEqual({ totalTokens: 2 });
    expect(memoryImage(first.images[0]).objectUrl).not.toBe(
      memoryImage(second.images[0]).objectUrl
    );
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
    const task = getImageGenSession().tasks[0];
    expect(task.status).toBe("done");
    expect(task.images).toHaveLength(1);
  });

  it("ignores retry while the target task is still loading", async () => {
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
    const taskId = result.current.tasks[0].id;

    await act(async () => {
      await result.current.retry(taskId);
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

  it("adds a pasted clipboard image as a reference image (items path, macOS 截图)", async () => {
    const file = makePngFile("paste.png");
    const { result } = await renderController();
    let notPrevented = true;
    act(() => {
      // jsdom 无真 DataTransfer，clipboardData 用普通对象构造即可。
      notPrevented = fireEvent.paste(document, {
        clipboardData: {
          items: [{ kind: "file", type: "image/png", getAsFile: () => file }],
          files: [],
        },
      });
    });
    expect(notPrevented).toBe(false); // 含图片时 preventDefault
    await waitFor(() => {
      expect(result.current.referenceImages).toHaveLength(1);
    });
    expect(result.current.referenceImages[0].mime).toBe("image/png");
  });

  it("falls back to clipboard files when items carry no image", async () => {
    const file = makePngFile("paste-file.png");
    const { result } = await renderController();
    act(() => {
      fireEvent.paste(document, { clipboardData: { items: [], files: [file] } });
    });
    await waitFor(() => {
      expect(result.current.referenceImages).toHaveLength(1);
    });
  });

  it("ignores text-only paste without intercepting the event", async () => {
    const { result } = await renderController();
    let notPrevented = false;
    act(() => {
      notPrevented = fireEvent.paste(document, {
        clipboardData: {
          items: [{ kind: "string", type: "text/plain", getAsFile: () => null }],
          files: [],
        },
      });
    });
    expect(notPrevented).toBe(true); // 无图片不拦截，文本粘贴不受影响
    expect(result.current.referenceImages).toHaveLength(0);
  });

  it("removes the paste listener on unmount", async () => {
    const file = makePngFile("late.png");
    const { unmount } = await renderController();
    unmount();
    let notPrevented = false;
    act(() => {
      notPrevented = fireEvent.paste(document, {
        clipboardData: {
          items: [{ kind: "file", type: "image/png", getAsFile: () => file }],
          files: [],
        },
      });
    });
    // 监听器已摘除：事件未被拦截，模块 store 不变。
    expect(notPrevented).toBe(true);
    expect(getImageGenSession().referenceImages).toHaveLength(0);
  });

  it("extractClipboardImageFiles handles null and non-image data", () => {
    expect(extractClipboardImageFiles(null)).toEqual([]);
    expect(extractClipboardImageFiles({})).toEqual([]);
    expect(
      extractClipboardImageFiles({
        files: [new File(["x"], "a.txt", { type: "text/plain" })],
      })
    ).toEqual([]);
  });

  it("stepPreview is a no-op when no preview is open", async () => {
    const { result } = await renderController();
    act(() => {
      result.current.stepPreview(1);
    });
    expect(result.current.preview).toBeNull();
  });

  // ---------- 持久化：落盘流 ----------

  it("persists a finished task and switches it to the disk form, releasing object urls", async () => {
    vi.mocked(imageGenTaskPersist).mockImplementation(async (payload) => rowFromPayload(payload));
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("img") }],
      usage: { totalTokens: 42 },
    });
    const { result } = await renderController();

    act(() => {
      result.current.setPrompt("落盘");
    });
    await act(async () => {
      await result.current.submit();
    });
    await waitFor(() => {
      expect(result.current.tasks[0].persisted).toBe(true);
    });

    const task = result.current.tasks[0];
    const image = diskImage(task.images[0]);
    expect(image.src).toBe(`asset://localhost//store/${task.id}/image-1.png`);
    // jsdom 无 createImageBitmap：缩略图生成失败仍落盘，thumbSrc 回退原图。
    expect(image.thumbSrc).toBe(image.src);
    expect(task.usage).toEqual({ totalTokens: 42 });
    // 旧 objectURL 已释放（本用例唯一创建的 URL 即生成图）。
    const generatedUrl = vi.mocked(URL.createObjectURL).mock.results[0].value as string;
    expect(URL.revokeObjectURL).toHaveBeenCalledWith(generatedUrl);

    const payload = vi.mocked(imageGenTaskPersist).mock.calls[0][0];
    expect(payload.status).toBe("done");
    expect(payload.images).toHaveLength(1);
    expect(payload.thumbs).toEqual([]);
    expect(payload.usageJson).toBe(JSON.stringify({ totalTokens: 42 }));
  });

  it("strips reference image bytes from the request snapshot and persists them as files", async () => {
    vi.mocked(imageGenTaskPersist).mockImplementation(async (payload) => rowFromPayload(payload));
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("img") }],
    });
    const { result } = await renderController();

    await act(async () => {
      await result.current.addReferenceFiles([makePngFile()]);
    });
    const refB64 = result.current.referenceImages[0].b64;
    const refUrl = result.current.referenceImages[0].objectUrl;
    act(() => {
      result.current.setPrompt("带参考图落盘");
    });
    await act(async () => {
      await result.current.submit();
    });
    await waitFor(() => {
      expect(result.current.tasks[0].persisted).toBe(true);
    });

    const payload = vi.mocked(imageGenTaskPersist).mock.calls[0][0];
    // 快照剥离 b64，只存 {file, mime} 占位；字节走 refImages 落盘。
    expect(payload.requestJson).not.toContain(refB64);
    expect(JSON.parse(payload.requestJson).referenceImages).toEqual([
      { file: "ref-1.png", mime: "image/png" },
    ]);
    expect(payload.refImages).toEqual([{ mime: "image/png", dataB64: refB64 }]);

    const task = result.current.tasks[0];
    expect(task.refThumbs[0].startsWith("asset://")).toBe(true);
    expect(task.refPaths).toEqual([{ path: `/store/${task.id}/ref-1.png`, mime: "image/png" }]);
    expect(URL.revokeObjectURL).toHaveBeenCalledWith(refUrl);
  });

  it("keeps the memory form and toasts when persisting fails", async () => {
    vi.mocked(imageGenTaskPersist).mockRejectedValue(new Error("disk full"));
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("img") }],
    });
    const { result } = await renderController();

    act(() => {
      result.current.setPrompt("落盘失败");
    });
    await act(async () => {
      await result.current.submit();
    });
    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith("已生成但保存到本地失败，本条记录仅本次会话可见");
    });

    const task = result.current.tasks[0];
    expect(task.persisted).toBe(false);
    expect(task.images[0].kind).toBe("memory");
    expect(URL.revokeObjectURL).not.toHaveBeenCalled();
  });

  it("persists failed tasks with their error and empty images", async () => {
    vi.mocked(imageGenTaskPersist).mockImplementation(async (payload) => rowFromPayload(payload));
    vi.mocked(gptImageAdapter.generate).mockRejectedValue(new Error("HTTP 500: boom"));
    const { result } = await renderController();

    act(() => {
      result.current.setPrompt("失败也落盘");
    });
    await act(async () => {
      await result.current.submit();
    });
    await waitFor(() => {
      expect(result.current.tasks[0].persisted).toBe(true);
    });

    const payload = vi.mocked(imageGenTaskPersist).mock.calls[0][0];
    expect(payload.status).toBe("error");
    expect(payload.error).toContain("HTTP 500: boom");
    expect(payload.images).toEqual([]);
    const task = result.current.tasks[0];
    expect(task.status).toBe("error");
    expect(task.error).toContain("HTTP 500: boom");
  });

  it("cleans up the persisted row when the task was deleted mid-persist", async () => {
    let resolvePersist!: () => void;
    vi.mocked(imageGenTaskPersist).mockImplementation(
      (payload) =>
        new Promise<ImageGenTaskRow>((resolve) => {
          resolvePersist = () => resolve(rowFromPayload(payload));
        })
    );
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("img") }],
    });
    const { result } = await renderController();
    act(() => {
      result.current.setPrompt("落盘中删除");
    });
    await act(async () => {
      await result.current.submit();
    });
    await waitFor(() => {
      expect(imageGenTaskPersist).toHaveBeenCalled();
    });
    const taskId = result.current.tasks[0].id;

    // 落盘尚未完成即删除（memory 形态 → 本地直删）。
    act(() => {
      result.current.deleteTask(taskId);
    });
    expect(result.current.tasks).toHaveLength(0);

    await act(async () => {
      resolvePersist();
    });
    // 刚写入的行被回收，任务不复活。
    expect(imageGenTaskDelete).toHaveBeenCalledWith(taskId);
    expect(result.current.tasks).toHaveLength(0);
  });

  // ---------- 持久化：hydration 与分页 ----------

  it("hydrates the task grid from the database on mount (including failed tasks)", async () => {
    vi.mocked(imageGenTasksList).mockResolvedValue([
      makeRow(),
      makeRow({
        id: "row-2",
        status: "error",
        error: "HTTP 500: boom",
        images: [],
        createdAt: 1_700_000_000_500,
      }),
    ]);
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(2);
    });

    expect(imageGenTasksList).toHaveBeenCalledWith(null, HISTORY_PAGE_SIZE);
    // store 升序（展示层反转）；两行均 persisted。
    const [done, failed] = result.current.tasks;
    expect(done).toMatchObject({
      id: "row-1",
      status: "done",
      persisted: true,
      prompt: "历史任务",
    });
    expect(diskImage(done.images[0])).toMatchObject({
      path: "/store/row-1/image-1.png",
      src: "asset://localhost//store/row-1/image-1.png",
      thumbSrc: "asset://localhost//store/row-1/thumb-1.webp",
    });
    expect(failed).toMatchObject({ id: "row-2", status: "error", error: "HTTP 500: boom" });
    expect(result.current.hasMore).toBe(false);
  });

  it("does not re-hydrate on remount once the store is hydrated", async () => {
    const { unmount } = await renderController();
    expect(imageGenTasksList).toHaveBeenCalledTimes(1);
    unmount();
    await renderController();
    expect(imageGenTasksList).toHaveBeenCalledTimes(1);
  });

  it("skips rows whose request snapshot cannot be parsed", async () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    vi.mocked(imageGenTasksList).mockResolvedValue([
      makeRow({ id: "bad", requestJson: "not-json{" }),
      makeRow({ id: "good", createdAt: 1_700_000_000_100 }),
    ]);
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });
    expect(result.current.tasks[0].id).toBe("good");
    expect(warnSpy).toHaveBeenCalled();
  });

  it("loads the next page with the oldest createdAt as the cursor", async () => {
    const base = 1_700_000_000_000;
    const firstPage = Array.from({ length: HISTORY_PAGE_SIZE }, (_, index) =>
      makeRow({ id: `r${index}`, createdAt: base - index })
    );
    vi.mocked(imageGenTasksList)
      .mockResolvedValueOnce(firstPage)
      .mockResolvedValueOnce([makeRow({ id: "older", createdAt: base - 10_000 })]);
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(HISTORY_PAGE_SIZE);
    });
    expect(result.current.hasMore).toBe(true);

    await act(async () => {
      await result.current.loadMoreTasks();
    });
    expect(imageGenTasksList).toHaveBeenLastCalledWith(
      base - (HISTORY_PAGE_SIZE - 1),
      HISTORY_PAGE_SIZE
    );
    expect(result.current.tasks).toHaveLength(HISTORY_PAGE_SIZE + 1);
    // 返回条数不足一页：没有更多。
    expect(result.current.hasMore).toBe(false);
  });

  it("toasts when loading more history fails", async () => {
    vi.mocked(imageGenTasksList).mockResolvedValueOnce([]).mockRejectedValueOnce(new Error("db"));
    const { result } = await renderController();
    await act(async () => {
      await result.current.loadMoreTasks();
    });
    expect(toast.error).toHaveBeenCalledWith("加载更多失败：请查看控制台日志");
  });

  // ---------- 持久化：disk 任务的四操作（read_image 读回） ----------

  it("retries a disk task by reading reference images back from disk", async () => {
    vi.mocked(imageGenTasksList).mockResolvedValue([makeRowWithRef()]);
    vi.mocked(imageGenReadImage).mockResolvedValue({ mime: "image/png", dataB64: btoa("ref") });
    vi.mocked(gptImageAdapter.generate).mockResolvedValue({
      images: [{ mime: "image/png", b64: btoa("new") }],
    });
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });

    await act(async () => {
      await result.current.retry("row-1");
    });
    expect(imageGenReadImage).toHaveBeenCalledWith("/store/row-1/ref-1.png");
    const request = vi.mocked(gptImageAdapter.generate).mock.calls[0][0];
    expect(request.referenceImages).toEqual([{ mime: "image/png", b64: btoa("ref") }]);
    expect(result.current.tasks[0].status).toBe("done");
  });

  it("does not overwrite the persisted done row when a disk-task retry fails", async () => {
    // 独立 id：与其他用例的悬挂 persist（异步 FileReader）隔离，断言只看本任务。
    vi.mocked(imageGenTasksList).mockResolvedValue([makeRow({ id: "row-keep" })]);
    vi.mocked(gptImageAdapter.generate).mockRejectedValue(new Error("HTTP 500: boom"));
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });

    await act(async () => {
      await result.current.retry("row-keep");
    });
    // 失败态仅本次会话可见；不 upsert 空 images 的 error 行覆盖上一次成功结果。
    const callsForTask = vi
      .mocked(imageGenTaskPersist)
      .mock.calls.filter(([payload]) => payload.id === "row-keep");
    expect(callsForTask).toHaveLength(0);
    const task = result.current.tasks[0];
    expect(task.status).toBe("error");
    expect(task.error).toContain("HTTP 500: boom");
    expect(task.images[0].kind).toBe("disk");
    expect(task.persisted).toBe(true);
  });

  it("aborts a disk-task retry with a toast when the reference file is missing", async () => {
    vi.mocked(imageGenTasksList).mockResolvedValue([makeRowWithRef()]);
    vi.mocked(imageGenReadImage).mockRejectedValue(new Error("SEC_PATH: not found"));
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });

    await act(async () => {
      await result.current.retry("row-1");
    });
    expect(toast.error).toHaveBeenCalledWith("图片文件缺失");
    expect(gptImageAdapter.generate).not.toHaveBeenCalled();
    expect(result.current.tasks[0].status).toBe("done");
  });

  it("reuses a disk task config by reading reference images back from disk", async () => {
    vi.mocked(imageGenTasksList).mockResolvedValue([makeRowWithRef()]);
    vi.mocked(imageGenReadImage).mockResolvedValue({ mime: "image/png", dataB64: btoa("ref") });
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });

    await act(async () => {
      await result.current.reuseTask("row-1");
    });
    expect(result.current.prompt).toBe("历史任务");
    expect(result.current.referenceImages).toHaveLength(1);
    expect(result.current.referenceImages[0].b64).toBe(btoa("ref"));
    expect(toast.success).toHaveBeenCalledWith("已复用配置");
  });

  it("aborts disk-task reuse without touching the input area when the file is missing", async () => {
    vi.mocked(imageGenTasksList).mockResolvedValue([makeRowWithRef()]);
    vi.mocked(imageGenReadImage).mockRejectedValue(new Error("missing"));
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });

    await act(async () => {
      await result.current.reuseTask("row-1");
    });
    expect(toast.error).toHaveBeenCalledWith("图片文件缺失");
    expect(result.current.prompt).toBe("");
    expect(result.current.referenceImages).toHaveLength(0);
    expect(toast.success).not.toHaveBeenCalled();
  });

  it("sets a disk image as reference by reading it back from disk", async () => {
    vi.mocked(imageGenTasksList).mockResolvedValue([makeRow()]);
    vi.mocked(imageGenReadImage).mockResolvedValue({ mime: "image/png", dataB64: btoa("img") });
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });

    await act(async () => {
      await result.current.setAsReference(result.current.tasks[0].images[0]);
    });
    expect(imageGenReadImage).toHaveBeenCalledWith("/store/row-1/image-1.png");
    expect(result.current.referenceImages).toHaveLength(1);
    expect(result.current.referenceImages[0].b64).toBe(btoa("img"));
    expect(toast.success).toHaveBeenCalledWith("已设为参考图");
  });

  it("aborts setAsReference with a toast when the disk image file is missing", async () => {
    vi.mocked(imageGenTasksList).mockResolvedValue([makeRow()]);
    vi.mocked(imageGenReadImage).mockRejectedValue(new Error("missing"));
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });

    await act(async () => {
      await result.current.setAsReference(result.current.tasks[0].images[0]);
    });
    expect(toast.error).toHaveBeenCalledWith("图片文件缺失");
    expect(result.current.referenceImages).toHaveLength(0);
  });

  it("downloads a disk image by reading its bytes back from disk", async () => {
    vi.mocked(imageGenTasksList).mockResolvedValue([makeRow()]);
    vi.mocked(saveDesktopFilePath).mockResolvedValue("/tmp/out.png");
    vi.mocked(imageGenReadImage).mockResolvedValue({ mime: "image/png", dataB64: btoa("img") });
    vi.mocked(imageGenSaveImage).mockResolvedValue(true);
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });

    await act(async () => {
      await result.current.downloadImage(result.current.tasks[0].images[0]);
    });
    expect(imageGenReadImage).toHaveBeenCalledWith("/store/row-1/image-1.png");
    expect(imageGenSaveImage).toHaveBeenCalledWith("/tmp/out.png", btoa("img"));
    expect(toast.success).toHaveBeenCalledWith("图片已保存");
  });

  it("aborts a disk-image download with a toast when the file is missing", async () => {
    vi.mocked(imageGenTasksList).mockResolvedValue([makeRow()]);
    vi.mocked(saveDesktopFilePath).mockResolvedValue("/tmp/out.png");
    vi.mocked(imageGenReadImage).mockRejectedValue(new Error("missing"));
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });

    await act(async () => {
      await result.current.downloadImage(result.current.tasks[0].images[0]);
    });
    expect(toast.error).toHaveBeenCalledWith("图片文件缺失");
    expect(imageGenSaveImage).not.toHaveBeenCalled();
  });

  // ---------- 持久化：删除语义 ----------

  it("deletes a persisted task through the backend before touching the store", async () => {
    vi.mocked(imageGenTasksList).mockResolvedValue([makeRow()]);
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });

    act(() => {
      result.current.deleteTask("row-1");
    });
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(0);
    });
    expect(imageGenTaskDelete).toHaveBeenCalledWith("row-1");
  });

  it("keeps the persisted task and toasts when backend deletion fails", async () => {
    vi.mocked(imageGenTasksList).mockResolvedValue([makeRow()]);
    vi.mocked(imageGenTaskDelete).mockRejectedValue(new Error("io"));
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.tasks).toHaveLength(1);
    });

    await act(async () => {
      result.current.deleteTask("row-1");
    });
    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith("删除任务失败：请查看控制台日志");
    });
    expect(result.current.tasks).toHaveLength(1);
  });

  // ---------- 存储管理 ----------

  it("loads storage stats on mount", async () => {
    const { result } = await renderController();
    await waitFor(() => {
      expect(result.current.storage).toEqual(STORAGE_VIEW);
    });
  });

  it("keeps a null storage view when loading stats fails", async () => {
    vi.mocked(imageGenStorageGet).mockRejectedValue(new Error("db"));
    const { result } = await renderController();
    expect(result.current.storage).toBeNull();
  });

  it("changes the storage directory through the directory picker", async () => {
    vi.mocked(openDesktopSinglePath).mockResolvedValue("/new/dir");
    vi.mocked(imageGenStorageSetDir).mockResolvedValue({
      dir: "/new/dir",
      totalBytes: 0,
      taskCount: 0,
    });
    const { result } = await renderController();
    await act(async () => {
      await result.current.changeStorageDir();
    });
    expect(openDesktopSinglePath).toHaveBeenCalledWith(
      expect.objectContaining({ directory: true })
    );
    expect(imageGenStorageSetDir).toHaveBeenCalledWith("/new/dir");
    expect(result.current.storage?.dir).toBe("/new/dir");
    expect(toast.success).toHaveBeenCalledWith("存储目录已更新");
  });

  it("aborts silently when the directory picker is cancelled", async () => {
    vi.mocked(openDesktopSinglePath).mockResolvedValue(null);
    const { result } = await renderController();
    await act(async () => {
      await result.current.changeStorageDir();
    });
    expect(imageGenStorageSetDir).not.toHaveBeenCalled();
    expect(toast.error).not.toHaveBeenCalled();
  });

  it("toasts when changing the storage directory fails", async () => {
    vi.mocked(openDesktopSinglePath).mockResolvedValue("/new/dir");
    vi.mocked(imageGenStorageSetDir).mockRejectedValue(new Error("not writable"));
    const { result } = await renderController();
    await act(async () => {
      await result.current.changeStorageDir();
    });
    expect(toast.error).toHaveBeenCalledWith("更改存储目录失败：请查看控制台日志");
    expect(result.current.storage).toEqual(STORAGE_VIEW);
  });

  it("cleans up old history and reports the removed count", async () => {
    vi.mocked(imageGenStorageCleanup).mockResolvedValue(3);
    const { result } = await renderController();
    await act(async () => {
      await result.current.cleanupStorage();
    });
    expect(imageGenStorageCleanup).toHaveBeenCalledWith(50);
    expect(toast.success).toHaveBeenCalledWith("已清理 3 条历史任务");
    // 清理后刷新统计（挂载一次 + 清理一次）。
    expect(imageGenStorageGet).toHaveBeenCalledTimes(2);
  });

  it("toasts when cleanup fails", async () => {
    vi.mocked(imageGenStorageCleanup).mockRejectedValue(new Error("io"));
    const { result } = await renderController();
    await act(async () => {
      await result.current.cleanupStorage();
    });
    expect(toast.error).toHaveBeenCalledWith("清理失败：请查看控制台日志");
  });
});
