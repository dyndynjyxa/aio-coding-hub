// Usage: 生图页控制器。持有连接配置与生成参数；会话（消息流/参考图/prompt）经 imageGenSessionStore
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

export type ImageGenUserMessage = {
  id: string;
  role: "user";
  prompt: string;
  refThumbs: string[];
};

export type ImageGenAssistantMessage = {
  id: string;
  role: "assistant";
  status: "loading" | "done" | "error";
  images: ImageGenGeneratedImage[];
  usage?: ImageGenUsage;
  error?: string;
  request: GptImageRequest;
};

export type ImageGenMessage = ImageGenUserMessage | ImageGenAssistantMessage;

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
  const [savingConfig, setSavingConfig] = useState(false);

  // 生成参数（localStorage 记忆默认值）
  const [params, setParams] = useState<ImageGenParams>(() => readParamsFromStorage());
  useEffect(() => {
    writeParamsToStorage(params);
  }, [params]);

  // 会话：模块级 store（跨路由卸载保留），页面组件只读快照。
  const { messages, prompt, referenceImages } = useSyncExternalStore(
    subscribeImageGenSession,
    getImageGenSession
  );
  const setPrompt = useCallback((value: string) => {
    updateImageGenSession((prev) => ({ ...prev, prompt: value }));
  }, []);

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

  const saveConfig = useCallback(async () => {
    const normalizedBaseUrl = normalizeBaseUrl(baseUrl);
    const trimmedModel = model.trim();
    if (!normalizedBaseUrl) {
      toast.error("请填写 Base URL");
      return;
    }
    if (!trimmedModel) {
      toast.error("请填写模型 ID");
      return;
    }
    setSavingConfig(true);
    try {
      const trimmedKey = apiKeyDraft.trim();
      // 仅在用户输入了新值时传值；null = 保留现有 key。
      const view = await imageGenConfigSet(
        IMAGE_GEN_ADAPTER_ID,
        normalizedBaseUrl,
        trimmedModel,
        trimmedKey ? trimmedKey : null
      );
      setBaseUrl(view.baseUrl);
      setModel(view.model);
      setApiKeyConfigured(view.apiKeyConfigured);
      setApiKeyDraft("");
      toast.success("生图配置已保存");
    } catch {
      toast.error("保存生图配置失败：请查看控制台日志");
    } finally {
      setSavingConfig(false);
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
  const runGeneration = useCallback(async (assistantId: string, request: GptImageRequest) => {
    try {
      const result = await gptImageAdapter.generate(request);
      const images = result.images.map((image) => {
        const blob = base64ToBlob(image.b64, image.mime);
        return { blob, mime: image.mime, objectUrl: trackImageGenObjectUrl(blob) };
      });
      updateImageGenSession((prev) => ({
        ...prev,
        messages: prev.messages.map((message) =>
          message.id === assistantId && message.role === "assistant"
            ? {
                ...message,
                status: "done" as const,
                images,
                usage: result.usage,
                error: undefined,
              }
            : message
        ),
      }));
    } catch (err) {
      const error = formatUnknownError(err);
      updateImageGenSession((prev) => ({
        ...prev,
        messages: prev.messages.map((message) =>
          message.id === assistantId && message.role === "assistant"
            ? { ...message, status: "error" as const, error }
            : message
        ),
      }));
    }
  }, []);

  // 每次提交独立创建消息对并各自生成，互不阻塞。
  const submit = useCallback(async () => {
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
    const userMessage: ImageGenUserMessage = {
      id: nextId(),
      role: "user",
      prompt: trimmedPrompt,
      refThumbs: session.referenceImages.map((image) => image.objectUrl),
    };
    const assistantId = nextId();
    updateImageGenSession((prev) => ({
      messages: [
        ...prev.messages,
        userMessage,
        { id: assistantId, role: "assistant", status: "loading", images: [], request },
      ],
      referenceImages: [],
      prompt: "",
    }));
    await runGeneration(assistantId, request);
  }, [model, params, runGeneration]);

  // 重试使用消息内的参数快照，不读面板当前值；目标消息生成中时忽略。
  const retry = useCallback(
    async (assistantId: string) => {
      const message = getImageGenSession().messages.find((item) => item.id === assistantId);
      if (!message || message.role !== "assistant" || message.status === "loading") return;
      updateImageGenSession((prev) => ({
        ...prev,
        messages: prev.messages.map((item) =>
          item.id === assistantId && item.role === "assistant"
            ? { ...item, status: "loading" as const, error: undefined }
            : item
        ),
      }));
      await runGeneration(assistantId, message.request);
    },
    [runGeneration]
  );

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
    savingConfig,
    requestUrlPreview,
    saveConfig,
    // 生成参数
    params,
    updateParams,
    // 会话
    messages,
    prompt,
    setPrompt,
    referenceImages,
    addReferenceFiles,
    removeReferenceImage,
    submit,
    retry,
    setAsReference,
    downloadImage,
    // 点击预览
    preview,
    openPreview,
    closePreview,
    stepPreview,
  };
}

export type ImageGenController = ReturnType<typeof useImageGenController>;
