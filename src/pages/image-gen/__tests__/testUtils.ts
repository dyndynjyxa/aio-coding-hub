import { vi } from "vitest";
import type { GptImageRequest } from "../../../services/image-gen/gptImageAdapter";
import {
  DEFAULT_IMAGE_GEN_PARAMS,
  type ImageGenController,
  type ImageGenTask,
  type ImageGenTaskImage,
} from "../useImageGenController";

export const TEST_REQUEST: GptImageRequest = {
  prompt: "一只猫",
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
};

export function makeMemoryImage(objectUrl = "blob:generated-1"): ImageGenTaskImage {
  return {
    kind: "memory",
    objectUrl,
    mime: "image/png",
    blob: new Blob(["x"], { type: "image/png" }),
  };
}

export function makeDiskImage(path = "/store/t1/image-1.png"): ImageGenTaskImage {
  return {
    kind: "disk",
    src: `asset://localhost/${path}`,
    thumbSrc: `asset://localhost/${path.replace("image-", "thumb-")}`,
    path,
    mime: "image/png",
  };
}

export function makeTask(overrides: Partial<ImageGenTask> = {}): ImageGenTask {
  return {
    id: "t1",
    prompt: "一只猫",
    refThumbs: [],
    refPaths: [],
    request: TEST_REQUEST,
    status: "done",
    images: [],
    createdAt: 1_700_000_000_000,
    startedAt: 1_700_000_000_000,
    elapsedMs: 186_000,
    persisted: false,
    ...overrides,
  };
}

export function makeController(overrides: Partial<ImageGenController> = {}): ImageGenController {
  return {
    baseUrl: "",
    setBaseUrl: vi.fn(),
    model: "gpt-image-2",
    setModel: vi.fn(),
    apiKeyDraft: "",
    setApiKeyDraft: vi.fn(),
    apiKeyConfigured: false,
    requestUrlPreview: "",
    autoSaveConfig: vi.fn(async () => {}),
    params: { ...DEFAULT_IMAGE_GEN_PARAMS },
    updateParams: vi.fn(),
    tasks: [],
    prompt: "",
    setPrompt: vi.fn(),
    referenceImages: [],
    addReferenceFiles: vi.fn(async () => {}),
    removeReferenceImage: vi.fn(),
    submit: vi.fn(async () => {}),
    retry: vi.fn(async () => {}),
    deleteTask: vi.fn(),
    clearTasks: vi.fn(async () => {}),
    reuseTask: vi.fn(async () => {}),
    setAsReference: vi.fn(async () => {}),
    downloadImage: vi.fn(async () => {}),
    hasMore: false,
    loadMoreTasks: vi.fn(async () => {}),
    storage: null,
    changeStorageDir: vi.fn(async () => {}),
    cleanupStorage: vi.fn(async () => {}),
    searchQuery: "",
    setSearchQuery: vi.fn(),
    statusFilter: "all",
    setStatusFilter: vi.fn(),
    detailTask: null,
    openDetail: vi.fn(),
    closeDetail: vi.fn(),
    preview: null,
    openPreview: vi.fn(),
    closePreview: vi.fn(),
    stepPreview: vi.fn(),
    ...overrides,
  };
}
