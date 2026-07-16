import { vi } from "vitest";
import type { GptImageRequest } from "../../../services/image-gen/gptImageAdapter";
import {
  DEFAULT_IMAGE_GEN_PARAMS,
  type ImageGenController,
  type ImageGenTask,
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

export function makeTask(overrides: Partial<ImageGenTask> = {}): ImageGenTask {
  return {
    id: "t1",
    prompt: "一只猫",
    refThumbs: [],
    request: TEST_REQUEST,
    status: "done",
    images: [],
    createdAt: 1_700_000_000_000,
    startedAt: 1_700_000_000_000,
    elapsedMs: 186_000,
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
    savingConfig: false,
    requestUrlPreview: "",
    saveConfig: vi.fn(async () => {}),
    clearConfig: vi.fn(async () => {}),
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
    clearTasks: vi.fn(),
    reuseTask: vi.fn(),
    setAsReference: vi.fn(async () => {}),
    downloadImage: vi.fn(async () => {}),
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
