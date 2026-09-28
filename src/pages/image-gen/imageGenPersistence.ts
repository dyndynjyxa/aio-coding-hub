// Usage: 生图历史持久化前端侧辅助（纯函数 + 缩略图生成 + 读回）。文件写入、DB 行与路径校验
// 全部在 Rust（image_gen_* 命令）；本模块负责 payload 构造、DB 行 → 任务映射与显示 URL 转换。

import { convertDesktopFileSrc as convertFileSrc } from "../../services/desktop/assetUrl";
import type {
  ImageGenTaskFilePayload,
  ImageGenTaskFileRow,
  ImageGenTaskPersistPayload,
  ImageGenTaskRow,
} from "../../generated/bindings";
import { extFromMime, type GptImageRequest } from "../../services/image-gen/gptImageAdapter";
import { IMAGE_GEN_ADAPTER_ID, imageGenReadImage } from "../../services/image-gen/service";
import type { ImageGenRefImage, ImageGenUsage } from "../../services/image-gen/types";
import type { ImageGenTask, ImageGenTaskImage, ImageGenTaskRefPath } from "./useImageGenController";

export const THUMBNAIL_MAX_DIM = 384;

// ---------- base64 与 Blob 互转 ----------

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

// ---------- 显示 URL ----------

/** 任务图片的原图显示 URL：memory 为 objectURL，disk 为 asset 协议 URL。 */
export function taskImageSrc(image: ImageGenTaskImage): string {
  return image.kind === "memory" ? image.objectUrl : image.src;
}

/** 任务图片的缩略图显示 URL：disk 无缩略图时回退原图。 */
export function taskImageThumbSrc(image: ImageGenTaskImage): string {
  return image.kind === "memory" ? image.objectUrl : image.thumbSrc;
}

// ---------- 缩略图（前端 canvas 生成，Rust 不引 image crate） ----------

/**
 * 生成 384px webp 缩略图 base64；任何环节失败（无 createImageBitmap / canvas 不可用 /
 * toBlob 返回 null）返回 null，调用方缺省缩略图但不阻断落盘。
 */
export async function generateThumbnailB64(blob: Blob): Promise<ImageGenTaskFilePayload | null> {
  try {
    const bitmap = await createImageBitmap(blob);
    const scale = Math.min(1, THUMBNAIL_MAX_DIM / Math.max(bitmap.width, bitmap.height));
    const width = Math.max(1, Math.round(bitmap.width * scale));
    const height = Math.max(1, Math.round(bitmap.height * scale));
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const context = canvas.getContext("2d");
    if (!context) return null;
    context.drawImage(bitmap, 0, 0, width, height);
    bitmap.close();
    const thumbBlob = await new Promise<Blob | null>((resolve) => {
      canvas.toBlob(resolve, "image/webp", 0.8);
    });
    if (!thumbBlob) return null;
    return { mime: "image/webp", dataB64: await blobToBase64(thumbBlob) };
  } catch {
    return null;
  }
}

// ---------- 请求快照（不落 b64） ----------

/** 序列化请求快照：referenceImages 剥离 b64，只存 {file, mime} 占位（文件名对应任务目录内布局）。 */
export function stripRequestSnapshot(request: GptImageRequest): string {
  return JSON.stringify({
    ...request,
    referenceImages: request.referenceImages.map((ref, index) => ({
      file: `ref-${index + 1}.${extFromMime(ref.mime)}`,
      mime: ref.mime,
    })),
  });
}

/** 解析请求快照（占位形态 referenceImages → b64 为空的 ImageGenRefImage）；结构非法时抛错。 */
export function parseRequestSnapshot(json: string): GptImageRequest {
  const parsed: unknown = JSON.parse(json);
  if (!parsed || typeof parsed !== "object") {
    throw new Error("请求快照不是对象");
  }
  const record = parsed as Record<string, unknown>;
  if (!record.options || typeof record.options !== "object") {
    throw new Error("请求快照缺少 options");
  }
  const refsRaw = Array.isArray(record.referenceImages) ? record.referenceImages : [];
  const referenceImages: ImageGenRefImage[] = refsRaw.map((ref) => {
    const mime =
      ref && typeof ref === "object" && typeof (ref as { mime?: unknown }).mime === "string"
        ? (ref as { mime: string }).mime
        : "image/png";
    return { mime, b64: "" };
  });
  return { ...(record as unknown as GptImageRequest), referenceImages };
}

// ---------- persist payload 构造 ----------

/** 从内存态任务构造持久化 payload；缩略图逐张生成，首个失败即停（后端按下标配对，只允许前缀缺省）。 */
export async function buildPersistPayload(task: ImageGenTask): Promise<ImageGenTaskPersistPayload> {
  const images: ImageGenTaskFilePayload[] = [];
  for (const image of task.images) {
    if (image.kind !== "memory") continue; // disk 图已在磁盘，无需重传
    images.push({ mime: image.mime, dataB64: await blobToBase64(image.blob) });
  }
  const thumbs: ImageGenTaskFilePayload[] = [];
  for (const image of task.images) {
    if (image.kind !== "memory") break;
    const thumb = await generateThumbnailB64(image.blob);
    if (!thumb) break;
    thumbs.push(thumb);
  }
  return {
    id: task.id,
    adapterId: IMAGE_GEN_ADAPTER_ID,
    prompt: task.prompt,
    requestJson: stripRequestSnapshot(task.request),
    status: task.status === "error" ? "error" : "done",
    error: task.error ?? null,
    usageJson: task.usage ? JSON.stringify(task.usage) : null,
    createdAt: task.createdAt,
    elapsedMs: task.elapsedMs ?? null,
    images,
    thumbs,
    refImages: task.request.referenceImages
      .filter((ref) => ref.b64)
      .map((ref) => ({ mime: ref.mime, dataB64: ref.b64 })),
  };
}

// ---------- DB 行 → 任务 ----------

export function taskImageFromFileRow(file: ImageGenTaskFileRow): ImageGenTaskImage {
  const src = convertFileSrc(file.path);
  return {
    kind: "disk",
    src,
    thumbSrc: file.thumbPath ? convertFileSrc(file.thumbPath) : src,
    path: file.path,
    mime: file.mime,
  };
}

/** DB 行映射为 disk 形态任务；请求快照解析失败的行降级跳过（console 容忍，不阻断其余行）。 */
export function taskFromRow(row: ImageGenTaskRow): ImageGenTask | null {
  let request: GptImageRequest;
  try {
    request = parseRequestSnapshot(row.requestJson);
  } catch (err) {
    console.warn("[image-gen] 跳过无法解析的历史任务行", row.id, err);
    return null;
  }
  let usage: ImageGenUsage | undefined;
  if (row.usageJson) {
    try {
      usage = JSON.parse(row.usageJson) as ImageGenUsage;
    } catch {
      usage = undefined;
    }
  }
  return {
    id: row.id,
    prompt: row.prompt,
    request,
    status: row.status === "error" ? "error" : "done",
    error: row.error ?? undefined,
    usage,
    images: row.images.map(taskImageFromFileRow),
    refThumbs: row.refImages.map((file) => convertFileSrc(file.path)),
    refPaths: row.refImages.map((file) => ({ path: file.path, mime: file.mime })),
    createdAt: row.createdAt,
    startedAt: row.createdAt,
    elapsedMs: row.elapsedMs ?? undefined,
    persisted: true,
  };
}

// ---------- store 合并/清理 ----------

/** 按 id 去重合并（已在 store 的任务优先），并按 createdAt 升序（展示层反转为新在前）。 */
export function mergeTasksByCreatedAt(
  current: ImageGenTask[],
  incoming: ImageGenTask[]
): ImageGenTask[] {
  const ids = new Set(current.map((task) => task.id));
  return [...current, ...incoming.filter((task) => !ids.has(task.id))].sort(
    (a, b) => a.createdAt - b.createdAt
  );
}

/** cleanup 后同步 store：persisted 任务按 createdAt 保留最近 keepCount 条，memory 任务不受影响。 */
export function pruneTasksForCleanup(tasks: ImageGenTask[], keepCount: number): ImageGenTask[] {
  const persisted = tasks
    .filter((task) => task.persisted)
    .sort((a, b) => b.createdAt - a.createdAt);
  const keep = new Set(persisted.slice(0, keepCount).map((task) => task.id));
  return tasks.filter((task) => !task.persisted || keep.has(task.id));
}

// ---------- 落盘参考图读回 ----------

/** 逐个读回落盘参考图字节（任一缺失即抛错，调用方 toast「图片文件缺失」并中止操作）。 */
export async function readBackReferenceImages(
  refPaths: ImageGenTaskRefPath[]
): Promise<ImageGenRefImage[]> {
  return Promise.all(
    refPaths.map(async (ref) => {
      const image = await imageGenReadImage(ref.path);
      return { mime: image.mime, b64: image.dataB64 };
    })
  );
}
