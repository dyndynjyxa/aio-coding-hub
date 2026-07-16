import { vi } from "vitest";
import { DEFAULT_IMAGE_GEN_PARAMS, type ImageGenController } from "../useImageGenController";

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
    params: { ...DEFAULT_IMAGE_GEN_PARAMS },
    updateParams: vi.fn(),
    messages: [],
    prompt: "",
    setPrompt: vi.fn(),
    referenceImages: [],
    generating: false,
    addReferenceFiles: vi.fn(async () => {}),
    removeReferenceImage: vi.fn(),
    submit: vi.fn(async () => {}),
    retry: vi.fn(async () => {}),
    setAsReference: vi.fn(async () => {}),
    downloadImage: vi.fn(async () => {}),
    preview: null,
    openPreview: vi.fn(),
    closePreview: vi.fn(),
    stepPreview: vi.fn(),
    ...overrides,
  };
}
