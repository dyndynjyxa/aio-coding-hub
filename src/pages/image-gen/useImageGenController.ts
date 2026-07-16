// Usage: 生图页控制器。持有连接配置与生成参数；会话（任务列表/参考图/prompt）经 imageGenSessionStore
// 模块级保留（跨路由卸载），页面组件保持哑渲染。

import { useCallback, useEffect, useMemo, useState, useSyncExternalStore } from "react";
import { toast } from "sonner";
import {
  buildRequestUrlPreview,
  DEFAULT_IMAGE_GEN_MODEL,
  extFromMime,
  GENERATIONS_PATH,
  gptImageAdapter,
  normalizeBaseUrl,
  type GptImageModeration,
  type GptImageOutputFormat,
  type GptImageQuality,
  type GptImageRequest,
} from "../../services/image-gen/gptImageAdapter";
import {
  IMAGE_GEN_ADAPTER_ID,
  imageGenConfigGet,
  imageGenConfigSet,
  imageGenSaveImage,
} from "../../services/image-gen/service";
import type { ImageGenUsage } from "../../services/image-gen/types";
import {
  getImageGenSession,
  releaseImageGenObjectUrl,
  subscribeImageGenSession,
  trackImageGenObjectUrl,
  updateImageGenSession,
} from "./imageGenSessionStore";
import { saveDesktopFilePath } from "../../services/desktop/dialog";
import { formatUnknownError } from "../../utils/errors";

export const MAX_REFERENCE_IMAGES = 16;
export const MAX_REFERENCE_TOTAL_BYTES = 30 * 1024 * 1024;

const PARAMS_STORAGE_KEY = "aio-image-gen-params";

export type ImageGenParams = {
  size: string;
  quality: GptImageQuality;
  outputFormat: GptImageOutputFormat;
  outputCompression: number | null;
  moderation: GptImageModeration;
  n: number;
};

export const DEFAULT_IMAGE_GEN_PARAMS: ImageGenParams = {
  size: "auto",
  quality: "auto",
  outputFormat: "png",
  outputCompression: null,
  moderation: "auto",
  n: 1,
};

export type ImageGenGeneratedImage = {
  objectUrl: string;
  mime: string;
  blob: Blob;
};

export type ImageGenTaskStatus = "loading" | "done" | "error";

/** 一次生成 = 一条任务（原 user/assistant 消息对合并）。 */
export type ImageGenTask = {
  id: string;
  prompt: string;
  refThumbs: string[];
  request: GptImageRequest;
  status: ImageGenTaskStatus;
  images: ImageGenGeneratedImage[];
  usage?: ImageGenUsage;
  error?: string;
  createdAt: number;
  startedAt: number;
  elapsedMs?: number;
};

export type ImageGenStatusFilter = "all" | ImageGenTaskStatus;

export type ImageGenPreview = {
  urls: string[];
  index: number;
};

export type ImageGenReferenceImage = {
  id: string;
  mime: string;
  b64: string;
  sizeBytes: number;
  objectUrl: string;
};

// ---------- 纯函数（导出便于测试） ----------

export function readParamsFromStorage(): ImageGenParams {
  if (typeof window === "undefined") return DEFAULT_IMAGE_GEN_PARAMS;
  try {
    const raw = window.localStorage.getItem(PARAMS_STORAGE_KEY);
    if (!raw) return DEFAULT_IMAGE_GEN_PARAMS;
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return DEFAULT_IMAGE_GEN_PARAMS;
    return { ...DEFAULT_IMAGE_GEN_PARAMS, ...(parsed as Partial<ImageGenParams>) };
  } catch {
    return DEFAULT_IMAGE_GEN_PARAMS;
  }
}

export function writeParamsToStorage(params: ImageGenParams) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(PARAMS_STORAGE_KEY, JSON.stringify(params));
  } catch {
    // 忽略持久化失败（仅影响默认值记忆）。
  }
}

/** 搜索/筛选纯函数：query 忽略大小写子串匹配 prompt；store 为追加序，反转后新的在前。 */
export function filterTasks(
  tasks: ImageGenTask[],
  query: string,
  filter: ImageGenStatusFilter
): ImageGenTask[] {
  const q = query.trim().toLowerCase();
  return tasks
    .filter(
      (task) =>
        (filter === "all" || task.status === filter) &&
        (q === "" || task.prompt.toLowerCase().includes(q))
    )
    .reverse();
}

/** 校验追加参考图是否超限，超限时返回中文错误文案。 */
export function validateReferenceAddition(
  currentCount: number,
  currentBytes: number,
  addedCount: number,
  addedBytes: number
): string | null {
  if (currentCount + addedCount > MAX_REFERENCE_IMAGES) {
    return `参考图最多 ${MAX_REFERENCE_IMAGES} 张`;
  }
  if (currentBytes + addedBytes > MAX_REFERENCE_TOTAL_BYTES) {
    return "参考图合计不能超过 30MB";
  }
  return null;
}

/** 剪贴板数据的结构化子集（测试可用普通对象构造，jsdom 无真 DataTransfer）。 */
export type ClipboardImageSource = {
  items?: ArrayLike<DataTransferItem>;
  files?: ArrayLike<File>;
} | null;

/** 从剪贴板提取图片 File：优先 items（macOS 截图粘贴走这里），无命中再回退 files。 */
export function extractClipboardImageFiles(data: ClipboardImageSource): File[] {
  if (!data) return [];
  const fromItems = Array.from(data.items ?? [])
    .filter((item) => item.kind === "file" && item.type.startsWith("image/"))
    .map((item) => item.getAsFile())
    .filter((file): file is File => file !== null);
  if (fromItems.length > 0) return fromItems;
  return Array.from(data.files ?? []).filter((file) => file.type.startsWith("image/"));
}

export function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = String(reader.result ?? "");
      const commaIndex = result.indexOf(",");
      resolve(commaIndex >= 0 ? result.slice(commaIndex + 1) : result);
    };
    reader.onerror = () => reject(new Error("读取图片文件失败"));
    reader.readAsDataURL(blob);
  });
}

export function base64ToBlob(b64: string, mime: string): Blob {
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return new Blob([bytes], { type: mime });
}

let idCounter = 0;
function nextId(): string {
  idCounter += 1;
  return `imggen-${Date.now()}-${idCounter}`;
}

// ---------- 控制器 ----------

export function useImageGenController() {
  // 连接配置
  const [baseUrl, setBaseUrl] = useState("");
  const [model, setModel] = useState(DEFAULT_IMAGE_GEN_MODEL);
  const [apiKeyDraft, setApiKeyDraft] = useState("");
  const [apiKeyConfigured, setApiKeyConfigured] = useState(false);

  // 生成参数（localStorage 记忆默认值）
  const [params, setParams] = useState<ImageGenParams>(() => readParamsFromStorage());
  useEffect(() => {
    writeParamsToStorage(params);
  }, [params]);

  // 会话：模块级 store（跨路由卸载保留），页面组件只读快照。
  const { tasks, prompt, referenceImages } = useSyncExternalStore(
    subscribeImageGenSession,
    getImageGenSession
  );
  const setPrompt = useCallback((value: string) => {
    updateImageGenSession((prev) => ({ ...prev, prompt: value }));
  }, []);

  // 搜索/筛选：组件态即可（不进 store）。
  const [searchQuery, setSearchQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<ImageGenStatusFilter>("all");

  // 任务详情弹窗：持 id，派生 task（任务被删时自动关闭渲染）。
  const [detailTaskId, setDetailTaskId] = useState<string | null>(null);
  const openDetail = useCallback((taskId: string) => setDetailTaskId(taskId), []);
  const closeDetail = useCallback(() => setDetailTaskId(null), []);
  const detailTask = useMemo(
    () => (detailTaskId ? (tasks.find((task) => task.id === detailTaskId) ?? null) : null),
    [detailTaskId, tasks]
  );

  // 点击预览：同组 objectURL + 当前下标；null = 关闭。
  const [preview, setPreview] = useState<ImageGenPreview | null>(null);
  const openPreview = useCallback((urls: string[], index: number) => {
    setPreview({ urls, index });
  }, []);
  const closePreview = useCallback(() => setPreview(null), []);
  const stepPreview = useCallback((delta: number) => {
    setPreview((prev) => {
      if (!prev) return prev;
      const index = (prev.index + delta + prev.urls.length) % prev.urls.length;
      return { ...prev, index };
    });
  }, []);

  // 配置加载：失败静默（invokeGeneratedIpc 已记日志），页面保持默认可编辑。
  useEffect(() => {
    let cancelled = false;
    void imageGenConfigGet(IMAGE_GEN_ADAPTER_ID)
      .then((view) => {
        if (cancelled) return;
        setBaseUrl(view.baseUrl);
        if (view.model) setModel(view.model);
        setApiKeyConfigured(view.apiKeyConfigured);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  const requestUrlPreview = useMemo(
    () => buildRequestUrlPreview(baseUrl, GENERATIONS_PATH),
    [baseUrl]
  );

  // 连接配置自动保存：三个输入框 blur 触发；baseUrl 为空时静默跳过；成功静默回填规整值，
  // 失败 toast。blur 天然去抖；连续 blur 的并发写以最后一次完成为准（简单 async，无需队列）。
  const autoSaveConfig = useCallback(async () => {
    const normalizedBaseUrl = normalizeBaseUrl(baseUrl);
    if (!normalizedBaseUrl) return;
    try {
      const trimmedKey = apiKeyDraft.trim();
      // 仅在用户输入了新值时传值；null = 保留现有 key。
      const view = await imageGenConfigSet(
        IMAGE_GEN_ADAPTER_ID,
        normalizedBaseUrl,
        model.trim() || DEFAULT_IMAGE_GEN_MODEL,
        trimmedKey ? trimmedKey : null
      );
      setBaseUrl(view.baseUrl);
      setModel(view.model);
      setApiKeyConfigured(view.apiKeyConfigured);
      setApiKeyDraft("");
    } catch {
      toast.error("保存生图配置失败：请查看控制台日志");
    }
  }, [apiKeyDraft, baseUrl, model]);

  const updateParams = useCallback((patch: Partial<ImageGenParams>) => {
    setParams((prev) => ({ ...prev, ...patch }));
  }, []);

  const addReferenceFiles = useCallback(async (files: FileList | File[]) => {
    const list = Array.from(files);
    if (list.length === 0) return;
    const current = getImageGenSession().referenceImages;
    const currentBytes = current.reduce((sum, image) => sum + image.sizeBytes, 0);
    const addedBytes = list.reduce((sum, file) => sum + file.size, 0);
    const error = validateReferenceAddition(current.length, currentBytes, list.length, addedBytes);
    if (error) {
      toast.error(error);
      return;
    }
    try {
      const added: ImageGenReferenceImage[] = [];
      for (const file of list) {
        const b64 = await blobToBase64(file);
        added.push({
          id: nextId(),
          mime: file.type || "image/png",
          b64,
          sizeBytes: file.size,
          objectUrl: trackImageGenObjectUrl(file),
        });
      }
      updateImageGenSession((prev) => ({
        ...prev,
        referenceImages: [...prev.referenceImages, ...added],
      }));
    } catch (err) {
      toast.error(formatUnknownError(err));
    }
  }, []);

  // Ctrl+V 粘贴剪贴板图片 → 参考图。addReferenceFiles 为空依赖 useCallback（引用稳定），
  // 监听器只在页面挂载期间存在一份；无图片时不拦截，普通文本粘贴不受影响。
  useEffect(() => {
    const onPaste = (event: ClipboardEvent) => {
      const files = extractClipboardImageFiles(event.clipboardData);
      if (files.length === 0) return;
      event.preventDefault();
      void addReferenceFiles(files);
    };
    document.addEventListener("paste", onPaste);
    return () => document.removeEventListener("paste", onPaste);
  }, [addReferenceFiles]);

  const removeReferenceImage = useCallback((id: string) => {
    updateImageGenSession((prev) => {
      const target = prev.referenceImages.find((image) => image.id === id);
      if (target) releaseImageGenObjectUrl(target.objectUrl);
      return { ...prev, referenceImages: prev.referenceImages.filter((image) => image.id !== id) };
    });
  }, []);

  // 完成回调写模块 store：页面卸载后任务继续完成，回来即见结果。
  const runGeneration = useCallback(async (taskId: string, request: GptImageRequest) => {
    try {
      const result = await gptImageAdapter.generate(request);
      // 任务在生成中被删除：丢弃结果。此处到下方 updater 全程同步，
      // objectURL 尚未创建即返回，天然无泄漏。
      if (!getImageGenSession().tasks.some((task) => task.id === taskId)) return;
      const images = result.images.map((image) => {
        const blob = base64ToBlob(image.b64, image.mime);
        return { blob, mime: image.mime, objectUrl: trackImageGenObjectUrl(blob) };
      });
      updateImageGenSession((prev) => ({
        ...prev,
        tasks: prev.tasks.map((task) => {
          if (task.id !== taskId) return task;
          // 重试覆盖旧结果时释放被替换图片的 objectURL。
          for (const old of task.images) releaseImageGenObjectUrl(old.objectUrl);
          return {
            ...task,
            status: "done" as const,
            images,
            usage: result.usage,
            error: undefined,
            elapsedMs: Date.now() - task.startedAt,
          };
        }),
      }));
    } catch (err) {
      const error = formatUnknownError(err);
      updateImageGenSession((prev) => ({
        ...prev,
        tasks: prev.tasks.map((task) =>
          task.id === taskId
            ? {
                ...task,
                status: "error" as const,
                error,
                elapsedMs: Date.now() - task.startedAt,
              }
            : task
        ),
      }));
    }
  }, []);

  // 每次提交独立创建任务并各自生成，互不阻塞。
  const submit = useCallback(async () => {
    // 配置守卫：Base URL 或 API Key（已存 / 草稿）缺失时提示，不创建任务。
    if (!normalizeBaseUrl(baseUrl) || (!apiKeyConfigured && !apiKeyDraft.trim())) {
      toast.error("请先在左侧完成连接配置（Base URL 与 API Key）");
      return;
    }
    const session = getImageGenSession();
    const trimmedPrompt = session.prompt.trim();
    if (!trimmedPrompt) return;
    const request: GptImageRequest = {
      prompt: trimmedPrompt,
      referenceImages: session.referenceImages.map(({ mime, b64 }) => ({ mime, b64 })),
      n: params.n,
      size: params.size,
      options: {
        model: model.trim() || DEFAULT_IMAGE_GEN_MODEL,
        quality: params.quality,
        outputFormat: params.outputFormat,
        outputCompression: params.outputCompression,
        moderation: params.moderation,
      },
    };
    const now = Date.now();
    const taskId = nextId();
    updateImageGenSession((prev) => ({
      tasks: [
        ...prev.tasks,
        {
          id: taskId,
          prompt: trimmedPrompt,
          refThumbs: session.referenceImages.map((image) => image.objectUrl),
          request,
          status: "loading" as const,
          images: [],
          createdAt: now,
          startedAt: now,
        },
      ],
      referenceImages: [],
      prompt: "",
    }));
    await runGeneration(taskId, request);
  }, [apiKeyConfigured, apiKeyDraft, baseUrl, model, params, runGeneration]);

  // 重试使用任务内的参数快照，不读面板当前值；目标任务生成中时忽略。
  // startedAt 重置计时，createdAt 保持创建时间不变。
  const retry = useCallback(
    async (taskId: string) => {
      const target = getImageGenSession().tasks.find((task) => task.id === taskId);
      if (!target || target.status === "loading") return;
      updateImageGenSession((prev) => ({
        ...prev,
        tasks: prev.tasks.map((task) =>
          task.id === taskId
            ? { ...task, status: "loading" as const, error: undefined, startedAt: Date.now() }
            : task
        ),
      }));
      await runGeneration(taskId, target.request);
    },
    [runGeneration]
  );

  // 删除任务：释放全部 objectURL（refThumbs + 生成图）后移出 store；loading 中也允许删除
  // （即"取消"），在途完成回调会因任务不存在而丢弃结果（见 runGeneration）。
  const deleteTask = useCallback((taskId: string) => {
    const target = getImageGenSession().tasks.find((task) => task.id === taskId);
    if (!target) return;
    // 预览正持有被删任务的图片 URL 时同步关闭（与 clearTasks 对齐）。
    setPreview((prev) =>
      prev && target.images.some((image) => prev.urls.includes(image.objectUrl)) ? null : prev
    );
    updateImageGenSession((prev) => {
      for (const url of target.refThumbs) releaseImageGenObjectUrl(url);
      for (const image of target.images) releaseImageGenObjectUrl(image.objectUrl);
      return { ...prev, tasks: prev.tasks.filter((task) => task.id !== taskId) };
    });
  }, []);

  // 清空全部任务（含生成图片）：释放所有任务 objectURL；在途生成完成后因任务不存在而丢弃结果。
  // 详情弹窗因 detailTask 派生自动关闭；预览可能持有已 revoke 的 URL，一并关闭。
  const clearTasks = useCallback(() => {
    updateImageGenSession((prev) => {
      for (const task of prev.tasks) {
        for (const url of task.refThumbs) releaseImageGenObjectUrl(url);
        for (const image of task.images) releaseImageGenObjectUrl(image.objectUrl);
      }
      return { ...prev, tasks: [] };
    });
    setPreview(null);
    toast.success("已清空任务");
  }, []);

  // 复用配置：从任务的 request 快照回填 prompt/参数/模型/参考图（替换当前输入区参考图）。
  const reuseTask = useCallback((taskId: string) => {
    const target = getImageGenSession().tasks.find((task) => task.id === taskId);
    if (!target) return;
    const { request } = target;
    setParams({
      size: request.size,
      n: request.n,
      quality: request.options.quality,
      outputFormat: request.options.outputFormat,
      outputCompression: request.options.outputCompression,
      moderation: request.options.moderation,
    });
    setModel(request.options.model);
    const rebuilt: ImageGenReferenceImage[] = request.referenceImages.map((ref) => {
      const blob = base64ToBlob(ref.b64, ref.mime);
      return {
        id: nextId(),
        mime: ref.mime,
        b64: ref.b64,
        sizeBytes: blob.size,
        objectUrl: trackImageGenObjectUrl(blob),
      };
    });
    updateImageGenSession((prev) => {
      for (const image of prev.referenceImages) releaseImageGenObjectUrl(image.objectUrl);
      return { ...prev, prompt: request.prompt, referenceImages: rebuilt };
    });
    toast.success("已复用配置");
  }, []);

  const setAsReference = useCallback(async (image: ImageGenGeneratedImage) => {
    const current = getImageGenSession().referenceImages;
    const currentBytes = current.reduce((sum, item) => sum + item.sizeBytes, 0);
    const error = validateReferenceAddition(current.length, currentBytes, 1, image.blob.size);
    if (error) {
      toast.error(error);
      return;
    }
    try {
      const b64 = await blobToBase64(image.blob);
      updateImageGenSession((prev) => ({
        ...prev,
        referenceImages: [
          ...prev.referenceImages,
          {
            id: nextId(),
            mime: image.mime,
            b64,
            sizeBytes: image.blob.size,
            objectUrl: trackImageGenObjectUrl(image.blob),
          },
        ],
      }));
      toast.success("已设为参考图");
    } catch (err) {
      toast.error(formatUnknownError(err));
    }
  }, []);

  const downloadImage = useCallback(async (image: ImageGenGeneratedImage) => {
    try {
      const path = await saveDesktopFilePath({
        title: "保存图片",
        defaultPath: `image-${Date.now()}.${extFromMime(image.mime)}`,
      });
      if (!path) return;
      const b64 = await blobToBase64(image.blob);
      await imageGenSaveImage(path, b64);
      toast.success("图片已保存");
    } catch {
      toast.error("保存图片失败：请查看控制台日志");
    }
  }, []);

  return {
    // 连接配置
    baseUrl,
    setBaseUrl,
    model,
    setModel,
    apiKeyDraft,
    setApiKeyDraft,
    apiKeyConfigured,
    requestUrlPreview,
    autoSaveConfig,
    // 生成参数
    params,
    updateParams,
    // 会话
    tasks,
    prompt,
    setPrompt,
    referenceImages,
    addReferenceFiles,
    removeReferenceImage,
    submit,
    retry,
    deleteTask,
    clearTasks,
    reuseTask,
    setAsReference,
    downloadImage,
    // 搜索/筛选
    searchQuery,
    setSearchQuery,
    statusFilter,
    setStatusFilter,
    // 任务详情
    detailTask,
    openDetail,
    closeDetail,
    // 点击预览
    preview,
    openPreview,
    closePreview,
    stepPreview,
  };
}

export type ImageGenController = ReturnType<typeof useImageGenController>;
