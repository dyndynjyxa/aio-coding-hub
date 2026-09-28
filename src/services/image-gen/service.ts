// Usage: 生图 IPC 服务封装（image_gen_* 命令）。apiKey 明文永不经过前端与日志，由 Rust 从 DB 注入。

import { commands } from "../../generated/bindings";
import type {
  ImageGenConfigView,
  ImageGenFetchedImage,
  ImageGenHttpResponse,
  ImageGenMultipartFile,
  ImageGenStorageView,
  ImageGenTaskPersistPayload,
  ImageGenTaskRow,
  JsonValue,
} from "../../generated/bindings";
import { invokeGeneratedIpc } from "../generatedIpc";

export const IMAGE_GEN_ADAPTER_ID = "gpt-image";

export type {
  ImageGenConfigView,
  ImageGenFetchedImage,
  ImageGenHttpResponse,
  ImageGenMultipartFile,
  ImageGenStorageView,
  ImageGenTaskPersistPayload,
  ImageGenTaskRow,
};

export async function imageGenConfigGet(adapterId: string): Promise<ImageGenConfigView> {
  return invokeGeneratedIpc<ImageGenConfigView>({
    title: "读取生图配置失败",
    cmd: "image_gen_config_get",
    args: { adapterId },
    invoke: () => commands.imageGenConfigGet(adapterId),
  });
}

export async function imageGenConfigSet(
  adapterId: string,
  baseUrl: string,
  model: string,
  apiKey: string | null
): Promise<ImageGenConfigView> {
  return invokeGeneratedIpc<ImageGenConfigView>({
    title: "保存生图配置失败",
    cmd: "image_gen_config_set",
    // apiKey 不进日志：仅记录是否携带新值。
    args: { adapterId, baseUrl, model, apiKey: apiKey == null ? null : "[REDACTED]" },
    invoke: () => commands.imageGenConfigSet(adapterId, baseUrl, model, apiKey),
  });
}

export async function imageGenPostJson(
  adapterId: string,
  path: string,
  body: JsonValue,
  timeoutSecs: number | null = null
): Promise<ImageGenHttpResponse> {
  return invokeGeneratedIpc<ImageGenHttpResponse>({
    title: "生图请求失败",
    cmd: "image_gen_post_json",
    // body 含 prompt 与潜在大 payload，不进日志。
    args: { adapterId, path },
    invoke: () => commands.imageGenPostJson(adapterId, path, body, timeoutSecs),
  });
}

export async function imageGenPostMultipart(
  adapterId: string,
  path: string,
  fields: [string, string][],
  files: ImageGenMultipartFile[],
  timeoutSecs: number | null = null
): Promise<ImageGenHttpResponse> {
  return invokeGeneratedIpc<ImageGenHttpResponse>({
    title: "生图编辑请求失败",
    cmd: "image_gen_post_multipart",
    // files 含 base64 图片数据，不进日志。
    args: { adapterId, path, fileCount: files.length },
    invoke: () => commands.imageGenPostMultipart(adapterId, path, fields, files, timeoutSecs),
  });
}

export async function imageGenFetchImage(
  url: string,
  timeoutSecs: number | null = null
): Promise<ImageGenFetchedImage> {
  return invokeGeneratedIpc<ImageGenFetchedImage>({
    title: "下载生成图片失败",
    cmd: "image_gen_fetch_image",
    args: { url },
    invoke: () => commands.imageGenFetchImage(url, timeoutSecs),
  });
}

export async function imageGenSaveImage(path: string, dataB64: string): Promise<boolean> {
  return invokeGeneratedIpc<boolean>({
    title: "保存图片失败",
    cmd: "image_gen_save_image",
    // dataB64 体量大，不进日志。
    args: { path },
    invoke: () => commands.imageGenSaveImage(path, dataB64),
  });
}

// ---------- 历史持久化（二期） ----------

export async function imageGenTaskPersist(
  payload: ImageGenTaskPersistPayload
): Promise<ImageGenTaskRow> {
  return invokeGeneratedIpc<ImageGenTaskRow>({
    title: "保存生成记录失败",
    cmd: "image_gen_task_persist",
    // 图片/缩略图/参考图均为 base64 大 payload，不进日志：仅记录计数。
    args: {
      id: payload.id,
      status: payload.status,
      imageCount: payload.images.length,
      thumbCount: payload.thumbs.length,
      refImageCount: payload.refImages.length,
    },
    invoke: () => commands.imageGenTaskPersist(payload),
  });
}

export async function imageGenTasksList(
  beforeCreatedAt: number | null,
  limit: number
): Promise<ImageGenTaskRow[]> {
  return invokeGeneratedIpc<ImageGenTaskRow[]>({
    title: "读取生成记录失败",
    cmd: "image_gen_tasks_list",
    args: { beforeCreatedAt, limit },
    invoke: () => commands.imageGenTasksList(beforeCreatedAt, limit),
  });
}

export async function imageGenTaskDelete(id: string): Promise<null> {
  return invokeGeneratedIpc<null, null>({
    title: "删除生成记录失败",
    cmd: "image_gen_task_delete",
    args: { id },
    invoke: () => commands.imageGenTaskDelete(id),
    nullResultBehavior: "return_fallback",
    fallback: null,
  });
}

export async function imageGenTasksClear(): Promise<number> {
  return invokeGeneratedIpc<number>({
    title: "清空生成记录失败",
    cmd: "image_gen_tasks_clear",
    invoke: () => commands.imageGenTasksClear(),
  });
}

export async function imageGenReadImage(path: string): Promise<ImageGenFetchedImage> {
  return invokeGeneratedIpc<ImageGenFetchedImage>({
    title: "读取本地图片失败",
    cmd: "image_gen_read_image",
    args: { path },
    invoke: () => commands.imageGenReadImage(path),
  });
}

export async function imageGenStorageGet(): Promise<ImageGenStorageView> {
  return invokeGeneratedIpc<ImageGenStorageView>({
    title: "读取存储信息失败",
    cmd: "image_gen_storage_get",
    invoke: () => commands.imageGenStorageGet(),
  });
}

export async function imageGenStorageSetDir(dir: string): Promise<ImageGenStorageView> {
  return invokeGeneratedIpc<ImageGenStorageView>({
    title: "更改存储目录失败",
    cmd: "image_gen_storage_set_dir",
    args: { dir },
    invoke: () => commands.imageGenStorageSetDir(dir),
  });
}

export async function imageGenStorageCleanup(keepCount: number): Promise<number> {
  return invokeGeneratedIpc<number>({
    title: "清理生成记录失败",
    cmd: "image_gen_storage_cleanup",
    args: { keepCount },
    invoke: () => commands.imageGenStorageCleanup(keepCount),
  });
}
