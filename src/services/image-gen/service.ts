// Usage: 生图 IPC 服务封装（image_gen_* 命令）。apiKey 明文永不经过前端与日志，由 Rust 从 DB 注入。

import { commands } from "../../generated/bindings";
import type {
  ImageGenConfigView,
  ImageGenFetchedImage,
  ImageGenHttpResponse,
  ImageGenMultipartFile,
  JsonValue,
} from "../../generated/bindings";
import { invokeGeneratedIpc } from "../generatedIpc";

export const IMAGE_GEN_ADAPTER_ID = "gpt-image";

export type {
  ImageGenConfigView,
  ImageGenFetchedImage,
  ImageGenHttpResponse,
  ImageGenMultipartFile,
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
