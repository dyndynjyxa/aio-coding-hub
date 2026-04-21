import {
  commands,
  type DesktopDialogOpenRequest,
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

function normalizeOpenDialogOptions(
  options: DesktopOpenDialogOptions
): DesktopDialogOpenRequest {
  return {
    title: options.title ?? null,
    filters: options.filters ?? null,
    defaultPath: options.defaultPath ?? null,
    multiple: options.multiple ?? null,
    directory: options.directory ?? null,
    recursive: options.recursive ?? null,
    canCreateDirectories: options.canCreateDirectories ?? null,
    pickerMode: options.pickerMode ?? null,
    fileAccessMode: options.fileAccessMode ?? null,
  };
}

function normalizeSaveDialogOptions(
  options: DesktopSaveDialogOptions
): DesktopDialogSaveRequest {
  return {
    title: options.title ?? null,
    filters: options.filters ?? null,
    defaultPath: options.defaultPath ?? null,
    canCreateDirectories: options.canCreateDirectories ?? null,
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
