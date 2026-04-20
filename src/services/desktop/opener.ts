import {
  commands,
  type DesktopOpenPathRequest,
  type DesktopOpenUrlRequest,
  type DesktopRevealItemRequest,
} from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type DesktopOpenUrlOptions = DesktopOpenUrlRequest;
export type DesktopOpenPathOptions = DesktopOpenPathRequest;
export type DesktopRevealItemOptions = DesktopRevealItemRequest;

function normalizeOpenUrlInput(input: string | DesktopOpenUrlOptions): DesktopOpenUrlOptions {
  if (typeof input === "string") {
    return { url: input, with: null };
  }
  return {
    url: input.url,
    with: input.with ?? null,
  };
}

function normalizeOpenPathInput(input: string | DesktopOpenPathOptions): DesktopOpenPathOptions {
  if (typeof input === "string") {
    return { path: input, with: null };
  }
  return {
    path: input.path,
    with: input.with ?? null,
  };
}

function normalizeRevealItemInput(
  input: string | DesktopRevealItemOptions
): DesktopRevealItemOptions {
  if (typeof input === "string") {
    return { path: input };
  }
  return { path: input.path };
}

export async function openDesktopUrl(input: string | DesktopOpenUrlOptions) {
  const payload = normalizeOpenUrlInput(input);
  return invokeGeneratedIpc<boolean>({
    title: "打开链接失败",
    cmd: "desktop_opener_open_url",
    args: { input: payload },
    invoke: () =>
      commands.desktopOpenerOpenUrl(payload) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function openDesktopPath(input: string | DesktopOpenPathOptions) {
  const payload = normalizeOpenPathInput(input);
  return invokeGeneratedIpc<boolean>({
    title: "打开目录失败",
    cmd: "desktop_opener_open_path",
    args: { input: payload },
    invoke: () =>
      commands.desktopOpenerOpenPath(payload) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function revealDesktopItem(input: string | DesktopRevealItemOptions) {
  const payload = normalizeRevealItemInput(input);
  return invokeGeneratedIpc<boolean>({
    title: "定位目录失败",
    cmd: "desktop_opener_reveal_item_in_dir",
    args: { input: payload },
    invoke: () =>
      commands.desktopOpenerRevealItemInDir(payload) as Promise<GeneratedCommandResult<boolean>>,
  });
}
