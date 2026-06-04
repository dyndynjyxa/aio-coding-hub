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

const DESKTOP_OPEN_URL_MAX_CHARS = 2048;
const DESKTOP_OPEN_PATH_MAX_CHARS = 4096;
const DESKTOP_OPEN_WITH_MAX_CHARS = 256;
const DESKTOP_OPEN_URL_SCHEMES = ["http", "https", "mailto", "tel"] as const;

function charLength(value: string) {
  return [...value].length;
}

function normalizeRequiredDesktopText(value: unknown, label: string, maxChars: number) {
  if (typeof value !== "string") {
    throw new Error(`SEC_INVALID_INPUT: ${label} must be a string`);
  }
  const normalized = value.trim();
  if (!normalized) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is required`);
  }
  if (charLength(normalized) > maxChars) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is too long (max ${maxChars} chars)`);
  }
  return normalized;
}

function normalizeOptionalDesktopProgram(value: unknown) {
  if (value == null) return null;
  if (typeof value !== "string") {
    throw new Error("SEC_INVALID_INPUT: with must be a string");
  }
  const normalized = value.trim();
  if (!normalized) return null;
  if (charLength(normalized) > DESKTOP_OPEN_WITH_MAX_CHARS) {
    throw new Error(
      `SEC_INVALID_INPUT: with is too long (max ${DESKTOP_OPEN_WITH_MAX_CHARS} chars)`
    );
  }
  return normalized;
}

function normalizeDesktopUrl(value: unknown) {
  const url = normalizeRequiredDesktopText(value, "url", DESKTOP_OPEN_URL_MAX_CHARS);
  let scheme: string;
  try {
    scheme = new URL(url).protocol.replace(/:$/, "").toLowerCase();
  } catch (error) {
    throw new Error(`SEC_INVALID_INPUT: invalid url: ${error}`);
  }
  if ((DESKTOP_OPEN_URL_SCHEMES as readonly string[]).includes(scheme)) {
    return url;
  }
  throw new Error(`SEC_INVALID_INPUT: unsupported url scheme=${scheme}`);
}

function normalizeDesktopPath(value: unknown) {
  return normalizeRequiredDesktopText(value, "path", DESKTOP_OPEN_PATH_MAX_CHARS);
}

function normalizeOpenUrlInput(input: string | DesktopOpenUrlOptions): DesktopOpenUrlOptions {
  if (typeof input === "string") {
    return { url: normalizeDesktopUrl(input), with: null };
  }
  return {
    url: normalizeDesktopUrl(input.url),
    with: normalizeOptionalDesktopProgram(input.with),
  };
}

function normalizeOpenPathInput(input: string | DesktopOpenPathOptions): DesktopOpenPathOptions {
  if (typeof input === "string") {
    return { path: normalizeDesktopPath(input), with: null };
  }
  return {
    path: normalizeDesktopPath(input.path),
    with: normalizeOptionalDesktopProgram(input.with),
  };
}

function normalizeRevealItemInput(
  input: string | DesktopRevealItemOptions
): DesktopRevealItemOptions {
  if (typeof input === "string") {
    return { path: normalizeDesktopPath(input) };
  }
  return { path: normalizeDesktopPath(input.path) };
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
