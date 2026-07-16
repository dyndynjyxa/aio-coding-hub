// Usage: GPT Image（OpenAI 兼容 /v1/images/*）适配器：URL 规整、请求构造、响应解析均为纯函数导出。
// 请求语义硬规则（经参考实现验证的兼容规则）：
// - model/prompt/size/output_format/moderation/quality 总是发送
// - n 仅 >1 时发送；output_compression 仅非 png 且非 null 时发送
// - 永不发送 background / input_fidelity / response_format / user

import type { ImageGenMultipartFile, JsonValue } from "../../generated/bindings";
import {
  IMAGE_GEN_ADAPTER_ID,
  imageGenFetchImage,
  imageGenPostJson,
  imageGenPostMultipart,
} from "./service";
import type { ImageGenAdapter, ImageGenCoreRequest, ImageGenResult, ImageGenUsage } from "./types";

export const GENERATIONS_PATH = "/v1/images/generations";
export const EDITS_PATH = "/v1/images/edits";
export const DEFAULT_IMAGE_GEN_MODEL = "gpt-image-2";

export type GptImageQuality = "auto" | "low" | "medium" | "high";
export type GptImageOutputFormat = "png" | "jpeg" | "webp";
export type GptImageModeration = "auto" | "low";

export type GptImageOptions = {
  model: string;
  quality: GptImageQuality;
  outputFormat: GptImageOutputFormat;
  outputCompression: number | null;
  moderation: GptImageModeration;
};

export type GptImageRequest = ImageGenCoreRequest & { options: GptImageOptions };

// ---------- URL 规整 ----------

/** 自动补 https://、去尾部斜杠；仅当 URL path 为空时补 /v1（带路径的中转不误伤）。 */
export function normalizeBaseUrl(input: string): string {
  let url = input.trim();
  if (!url) return "";
  if (!/^https?:\/\//i.test(url)) url = `https://${url}`;
  url = url.replace(/\/+$/, "");
  try {
    const parsed = new URL(url);
    if (parsed.pathname === "" || parsed.pathname === "/") {
      url = `${url}/v1`;
    }
  } catch {
    // 非法 URL 原样返回，由后端校验兜底。
  }
  return url;
}

/** 与 Rust 侧拼接逻辑一致：base 以 /v1 结尾时对 path 的 /v1 前缀去重。 */
export function buildRequestUrlPreview(baseUrl: string, path: string): string {
  const normalized = normalizeBaseUrl(baseUrl);
  if (!normalized) return "";
  const suffix = normalized.endsWith("/v1") && path.startsWith("/v1") ? path.slice(3) : path;
  return `${normalized}${suffix}`;
}

// ---------- 请求构造 ----------

export function buildGenerationsBody(req: GptImageRequest): JsonValue {
  const { options } = req;
  const body: { [key: string]: JsonValue } = {
    model: options.model,
    prompt: req.prompt,
    size: req.size,
    output_format: options.outputFormat,
    moderation: options.moderation,
    quality: options.quality,
  };
  if (req.n > 1) body.n = req.n;
  if (options.outputFormat !== "png" && options.outputCompression != null) {
    body.output_compression = options.outputCompression;
  }
  return body;
}

const MIME_EXT: Record<string, string> = {
  "image/png": "png",
  "image/jpeg": "jpeg",
  "image/webp": "webp",
};

export function extFromMime(mime: string): string {
  return MIME_EXT[mime] ?? (mime.split("/")[1] || "png");
}

export function buildEditsParts(req: GptImageRequest): {
  fields: [string, string][];
  files: ImageGenMultipartFile[];
} {
  const { options } = req;
  const fields: [string, string][] = [
    ["model", options.model],
    ["prompt", req.prompt],
    ["size", req.size],
    ["output_format", options.outputFormat],
    ["moderation", options.moderation],
    ["quality", options.quality],
  ];
  if (req.n > 1) fields.push(["n", String(req.n)]);
  if (options.outputFormat !== "png" && options.outputCompression != null) {
    fields.push(["output_compression", String(options.outputCompression)]);
  }
  const files = req.referenceImages.map((image, index) => ({
    field: "image[]",
    filename: `input-${index + 1}.${extFromMime(image.mime)}`,
    mime: image.mime,
    dataB64: image.b64,
  }));
  return { fields, files };
}

// ---------- 响应解析 ----------

/** 错误消息提取链：error.message → detail（字符串/数组）→ error（字符串）→ message → HTTP {status}。 */
export function extractApiErrorMessage(bodyText: string, status: number): string {
  const fallback = `HTTP ${status}`;
  let parsed: unknown = null;
  try {
    parsed = JSON.parse(bodyText);
  } catch {
    parsed = null;
  }
  if (parsed && typeof parsed === "object") {
    const record = parsed as Record<string, unknown>;
    const error = record.error;
    if (error && typeof error === "object") {
      const message = (error as Record<string, unknown>).message;
      if (typeof message === "string" && message.trim()) return message;
    }
    const detail = record.detail;
    if (typeof detail === "string" && detail.trim()) return detail;
    if (Array.isArray(detail) && detail.length > 0) {
      return detail
        .map((item) => (typeof item === "string" ? item : JSON.stringify(item)))
        .join("; ");
    }
    if (typeof error === "string" && error.trim()) return error;
    const message = record.message;
    if (typeof message === "string" && message.trim()) return message;
  }
  const text = bodyText.trim();
  if (text) return `${fallback}: ${text.slice(0, 200)}`;
  return fallback;
}

export function outputMimeFor(format: GptImageOutputFormat): string {
  return `image/${format}`;
}

export function parseUsage(value: unknown): ImageGenUsage | undefined {
  if (!value || typeof value !== "object") return undefined;
  const record = value as Record<string, unknown>;
  const usage: ImageGenUsage = {};
  if (typeof record.input_tokens === "number") usage.inputTokens = record.input_tokens;
  if (typeof record.output_tokens === "number") usage.outputTokens = record.output_tokens;
  if (typeof record.total_tokens === "number") usage.totalTokens = record.total_tokens;
  const detailsRaw = record.input_tokens_details;
  if (detailsRaw && typeof detailsRaw === "object") {
    const raw = detailsRaw as Record<string, unknown>;
    const details: { textTokens?: number; imageTokens?: number } = {};
    if (typeof raw.text_tokens === "number") details.textTokens = raw.text_tokens;
    if (typeof raw.image_tokens === "number") details.imageTokens = raw.image_tokens;
    if (Object.keys(details).length > 0) usage.inputTokensDetails = details;
  }
  return Object.keys(usage).length > 0 ? usage : undefined;
}

// gpt-image-2 官方费率（$/1M token）：文本输入 $5、图像输入 $8、图像输出 $30。
const COST_TEXT_INPUT_USD_PER_M = 5;
const COST_IMAGE_INPUT_USD_PER_M = 8;
const COST_OUTPUT_USD_PER_M = 30;

/**
 * 预估费用（美元）：有输入明细按 文本×$5 + 图像×$8 + 输出×$30 计；
 * 无明细退化为 input×$5 + 输出×$30；usage 缺失或无可计费字段返回 null。
 */
export function estimateGptImageCostUsd(usage: ImageGenUsage | undefined): number | null {
  if (!usage) return null;
  const output = usage.outputTokens ?? 0;
  const details = usage.inputTokensDetails;
  if (details && (details.textTokens != null || details.imageTokens != null)) {
    return (
      ((details.textTokens ?? 0) * COST_TEXT_INPUT_USD_PER_M +
        (details.imageTokens ?? 0) * COST_IMAGE_INPUT_USD_PER_M +
        output * COST_OUTPUT_USD_PER_M) /
      1_000_000
    );
  }
  if (usage.inputTokens == null && usage.outputTokens == null) return null;
  return (
    ((usage.inputTokens ?? 0) * COST_TEXT_INPUT_USD_PER_M + output * COST_OUTPUT_USD_PER_M) /
    1_000_000
  );
}

export type FetchImageByUrl = (url: string) => Promise<{ mime: string; b64: string }>;

/** 优先取 data[].b64_json；条目仅有 url 时经 fetchImageByUrl 兜底下载。 */
export async function parseImagesResponse(
  bodyText: string,
  outputMime: string,
  fetchImageByUrl: FetchImageByUrl
): Promise<ImageGenResult> {
  let parsed: unknown = null;
  try {
    parsed = JSON.parse(bodyText);
  } catch {
    throw new Error("响应不是有效的 JSON，无法解析生成结果");
  }
  const record = (parsed && typeof parsed === "object" ? parsed : {}) as Record<string, unknown>;
  const data = record.data;
  if (!Array.isArray(data) || data.length === 0) {
    throw new Error("响应中没有图片数据（data 为空）");
  }
  const images: { mime: string; b64: string }[] = [];
  for (const item of data) {
    if (!item || typeof item !== "object") continue;
    const entry = item as Record<string, unknown>;
    if (typeof entry.b64_json === "string" && entry.b64_json) {
      images.push({ mime: outputMime, b64: entry.b64_json });
    } else if (typeof entry.url === "string" && entry.url) {
      images.push(await fetchImageByUrl(entry.url));
    }
  }
  if (images.length === 0) {
    throw new Error("响应中没有可解析的图片（缺少 b64_json / url）");
  }
  return { images, usage: parseUsage(record.usage) };
}

// ---------- 适配器 ----------

async function fetchImageViaRust(url: string): Promise<{ mime: string; b64: string }> {
  const fetched = await imageGenFetchImage(url);
  return { mime: fetched.mime, b64: fetched.dataB64 };
}

export const gptImageAdapter: ImageGenAdapter<GptImageOptions> = {
  id: IMAGE_GEN_ADAPTER_ID,
  label: "GPT Image",
  async generate(req: GptImageRequest): Promise<ImageGenResult> {
    let response;
    if (req.referenceImages.length > 0) {
      const { fields, files } = buildEditsParts(req);
      response = await imageGenPostMultipart(IMAGE_GEN_ADAPTER_ID, EDITS_PATH, fields, files);
    } else {
      response = await imageGenPostJson(
        IMAGE_GEN_ADAPTER_ID,
        GENERATIONS_PATH,
        buildGenerationsBody(req)
      );
    }
    if (response.status < 200 || response.status >= 300) {
      throw new Error(extractApiErrorMessage(response.bodyText, response.status));
    }
    return parseImagesResponse(
      response.bodyText,
      outputMimeFor(req.options.outputFormat),
      fetchImageViaRust
    );
  },
};
