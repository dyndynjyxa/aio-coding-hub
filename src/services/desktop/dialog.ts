import {
  commands,
  type DesktopDialogFileAccessMode,
  type DesktopDialogFilter,
  type DesktopDialogOpenRequest,
  type DesktopDialogPickerMode,
  type DesktopDialogSaveRequest,
} from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

type DesktopDialogInput<TContract> = {
  [K in keyof TContract]?: TContract[K] | undefined;
};

export type DesktopOpenDialogOptions = DesktopDialogInput<DesktopDialogOpenRequest>;
export type DesktopSaveDialogOptions = DesktopDialogInput<DesktopDialogSaveRequest>;
export type DesktopDialogSelection = string | string[] | null;
export type DesktopSingleOpenDialogOptions = Omit<DesktopOpenDialogOptions, "multiple"> & {
  multiple?: false | null;
};

const DESKTOP_DIALOG_TITLE_MAX_CHARS = 256;
const DESKTOP_DIALOG_DEFAULT_PATH_MAX_CHARS = 4096;
const DESKTOP_DIALOG_FILTER_NAME_MAX_CHARS = 128;
const DESKTOP_DIALOG_FILTER_EXTENSION_MAX_CHARS = 64;
const DESKTOP_DIALOG_PICKER_MODE_VALUES = ["document", "media", "image", "video"] as const;
const DESKTOP_DIALOG_FILE_ACCESS_MODE_VALUES = ["copy", "scoped"] as const;

function charLength(value: string) {
  return [...value].length;
}

function normalizeOptionalDialogText(
  value: unknown,
  label: string,
  maxChars: number
): string | null {
  if (value == null) return null;
  if (typeof value !== "string") {
    throw new Error(`SEC_INVALID_INPUT: ${label} must be a string`);
  }
  const normalized = value.trim();
  if (!normalized) return null;
  if (charLength(normalized) > maxChars) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is too long (max ${maxChars} chars)`);
  }
  return normalized;
}

function normalizeRequiredDialogText(value: unknown, label: string, maxChars: number): string {
  const normalized = normalizeOptionalDialogText(value, label, maxChars);
  if (!normalized) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is required`);
  }
  return normalized;
}

function normalizeOptionalDialogBoolean(value: unknown, label: string): boolean | null {
  if (value == null) return null;
  if (typeof value === "boolean") return value;
  throw new Error(`SEC_INVALID_INPUT: ${label} must be a boolean`);
}

function normalizeDialogFilterExtensions(value: unknown): string[] {
  if (!Array.isArray(value)) {
    throw new Error("SEC_INVALID_INPUT: filter extensions must be an array");
  }

  const extensions = value
    .map((item) => {
      const normalized = normalizeOptionalDialogText(
        item,
        "filter extension",
        DESKTOP_DIALOG_FILTER_EXTENSION_MAX_CHARS
      );
      return normalized?.replace(/^\.+/, "") ?? null;
    })
    .filter((item): item is string => Boolean(item));

  if (extensions.length === 0) {
    throw new Error("SEC_INVALID_INPUT: filter extensions are required");
  }

  return extensions;
}

function normalizeDialogFilters(value: unknown): DesktopDialogFilter[] | null {
  if (value == null) return null;
  if (!Array.isArray(value)) {
    throw new Error("SEC_INVALID_INPUT: filters must be an array");
  }

  return value.map((filter) => {
    if (filter == null || typeof filter !== "object") {
      throw new Error("SEC_INVALID_INPUT: filter must be an object");
    }
    const candidate = filter as Partial<DesktopDialogFilter>;
    return {
      name: normalizeRequiredDialogText(
        candidate.name,
        "filter name",
        DESKTOP_DIALOG_FILTER_NAME_MAX_CHARS
      ),
      extensions: normalizeDialogFilterExtensions(candidate.extensions),
    };
  });
}

function normalizeDialogPickerMode(value: unknown): DesktopDialogPickerMode | null {
  if (value == null) return null;
  if (
    typeof value === "string" &&
    (DESKTOP_DIALOG_PICKER_MODE_VALUES as readonly string[]).includes(value)
  ) {
    return value as DesktopDialogPickerMode;
  }
  throw new Error(`SEC_INVALID_INPUT: invalid pickerMode=${String(value)}`);
}

function normalizeDialogFileAccessMode(value: unknown): DesktopDialogFileAccessMode | null {
  if (value == null) return null;
  if (
    typeof value === "string" &&
    (DESKTOP_DIALOG_FILE_ACCESS_MODE_VALUES as readonly string[]).includes(value)
  ) {
    return value as DesktopDialogFileAccessMode;
  }
  throw new Error(`SEC_INVALID_INPUT: invalid fileAccessMode=${String(value)}`);
}

function normalizeOpenDialogOptions(options: DesktopOpenDialogOptions): DesktopDialogOpenRequest {
  return {
    title: normalizeOptionalDialogText(options.title, "title", DESKTOP_DIALOG_TITLE_MAX_CHARS),
    filters: normalizeDialogFilters(options.filters),
    defaultPath: normalizeOptionalDialogText(
      options.defaultPath,
      "defaultPath",
      DESKTOP_DIALOG_DEFAULT_PATH_MAX_CHARS
    ),
    multiple: normalizeOptionalDialogBoolean(options.multiple, "multiple"),
    directory: normalizeOptionalDialogBoolean(options.directory, "directory"),
    recursive: normalizeOptionalDialogBoolean(options.recursive, "recursive"),
    canCreateDirectories: normalizeOptionalDialogBoolean(
      options.canCreateDirectories,
      "canCreateDirectories"
    ),
    pickerMode: normalizeDialogPickerMode(options.pickerMode),
    fileAccessMode: normalizeDialogFileAccessMode(options.fileAccessMode),
  };
}

function normalizeSaveDialogOptions(options: DesktopSaveDialogOptions): DesktopDialogSaveRequest {
  return {
    title: normalizeOptionalDialogText(options.title, "title", DESKTOP_DIALOG_TITLE_MAX_CHARS),
    filters: normalizeDialogFilters(options.filters),
    defaultPath: normalizeOptionalDialogText(
      options.defaultPath,
      "defaultPath",
      DESKTOP_DIALOG_DEFAULT_PATH_MAX_CHARS
    ),
    canCreateDirectories: normalizeOptionalDialogBoolean(
      options.canCreateDirectories,
      "canCreateDirectories"
    ),
  };
}

function normalizeDialogSelection(
  selection: string[] | null,
  options: { multiple?: boolean | null }
): DesktopDialogSelection {
  if (!selection?.length) {
    return null;
  }

  if (options.multiple === true) {
    return selection;
  }

  return selection[0] ?? null;
}

export async function openDesktopDialog(
  options: DesktopOpenDialogOptions
): Promise<DesktopDialogSelection> {
  const payload = normalizeOpenDialogOptions(options);

  const selection = await invokeGeneratedIpc<string[] | null, null>({
    title: "打开文件选择器失败",
    cmd: "desktop_dialog_open",
    args: { options: payload },
    invoke: () =>
      commands.desktopDialogOpen(payload) as Promise<GeneratedCommandResult<string[] | null>>,
    nullResultBehavior: "return_fallback",
    fallback: null,
  });

  return normalizeDialogSelection(selection, payload);
}

export async function saveDesktopDialog(options: DesktopSaveDialogOptions): Promise<string | null> {
  const payload = normalizeSaveDialogOptions(options);

  return invokeGeneratedIpc<string | null, null>({
    title: "打开保存对话框失败",
    cmd: "desktop_dialog_save",
    args: { options: payload },
    invoke: () =>
      commands.desktopDialogSave(payload) as Promise<GeneratedCommandResult<string | null>>,
    nullResultBehavior: "return_fallback",
    fallback: null,
  });
}

export function pickDesktopSinglePath(selection: DesktopDialogSelection): string | null {
  if (!selection) {
    return null;
  }

  return Array.isArray(selection) ? (selection[0] ?? null) : selection;
}

export async function openDesktopSinglePath(options: DesktopSingleOpenDialogOptions) {
  const selection = await openDesktopDialog(options);
  return pickDesktopSinglePath(selection);
}

export async function saveDesktopFilePath(options: DesktopSaveDialogOptions) {
  const selection = await saveDesktopDialog(options);
  return pickDesktopSinglePath(selection);
}
