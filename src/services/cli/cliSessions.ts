import {
  commands,
  type CliSessionsDisplayContentBlock,
  type CliSessionsDisplayMessage,
  type CliSessionsFolderLookupEntry as GeneratedCliSessionsFolderLookupEntry,
  type CliSessionsFolderLookupInput as GeneratedCliSessionsFolderLookupInput,
  type CliSessionsPaginatedMessages,
  type CliSessionsProjectSummary as GeneratedCliSessionsProjectSummary,
  type CliSessionsSessionSummary as GeneratedCliSessionsSessionSummary,
} from "../../generated/bindings";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";
import { narrowGeneratedStringUnion, type Override } from "../generatedTypeUtils";

const CLI_SESSION_SOURCE_VALUES = ["claude", "codex"] as const;

export const CLI_SESSIONS_DEFAULT_PAGE_SIZE = 50;
export const CLI_SESSIONS_MAX_PAGE_SIZE = 200;
export const CLI_SESSIONS_MAX_PATH_CHARS = 4096;
export const CLI_SESSIONS_MAX_TAIL_MESSAGES = 2000;
export const CLI_SESSIONS_MAX_DELETE_PATHS = 512;
export const CLI_SESSIONS_MAX_LOOKUP_ITEMS = 512;
export const CLI_SESSIONS_WSL_DISTRO_MAX_CHARS = 128;

export type CliSessionsSource = (typeof CLI_SESSION_SOURCE_VALUES)[number];

export type CliSessionsProjectSummary = Override<
  GeneratedCliSessionsProjectSummary,
  {
    source: CliSessionsSource;
  }
>;

export type CliSessionsSessionSummary = Override<
  GeneratedCliSessionsSessionSummary,
  {
    source: CliSessionsSource;
  }
>;

export type CliSessionsFolderLookupInput = Override<
  GeneratedCliSessionsFolderLookupInput,
  {
    source: CliSessionsSource;
  }
>;

export type CliSessionsFolderLookupEntry = Override<
  GeneratedCliSessionsFolderLookupEntry,
  {
    source: CliSessionsSource;
  }
>;

type CliSessionsMessagesCommandArgs = Parameters<typeof commands.cliSessionsMessagesGet>;
type CliSessionsSessionDeleteCommandArgs = Parameters<typeof commands.cliSessionsSessionDelete>;

export type CliSessionsMessagesInput = {
  source: CliSessionsSource;
  filePath: CliSessionsMessagesCommandArgs[1];
  page: CliSessionsMessagesCommandArgs[2];
  pageSize: CliSessionsMessagesCommandArgs[3];
  fromEnd: CliSessionsMessagesCommandArgs[4];
  wslDistro?: Exclude<CliSessionsMessagesCommandArgs[5], undefined>;
};

export type CliSessionsSessionDeleteInput = {
  source: CliSessionsSource;
  filePaths: CliSessionsSessionDeleteCommandArgs[1];
  wslDistro?: Exclude<CliSessionsSessionDeleteCommandArgs[2], undefined>;
};

function toCliSessionsSource(value: string, label: string): CliSessionsSource {
  return narrowGeneratedStringUnion(value, CLI_SESSION_SOURCE_VALUES, label);
}

function normalizeRequiredText(
  value: string,
  label: string,
  maxChars = CLI_SESSIONS_MAX_PATH_CHARS
) {
  const normalized = value.trim();
  if (!normalized) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is required`);
  }
  if ([...normalized].length > maxChars) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is too long (max ${maxChars} chars)`);
  }
  return normalized;
}

export function normalizeCliSessionsPage(page: number): number {
  if (!Number.isSafeInteger(page) || page < 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid page=${page}`);
  }
  return page;
}

export function normalizeCliSessionsPageSize(pageSize: number): number {
  if (!Number.isSafeInteger(pageSize) || pageSize < 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid pageSize=${pageSize}`);
  }
  if (pageSize === 0) return CLI_SESSIONS_DEFAULT_PAGE_SIZE;
  return Math.min(pageSize, CLI_SESSIONS_MAX_PAGE_SIZE);
}

export function validateCliSessionsMessageWindow(
  page: number,
  pageSize: number,
  fromEnd: boolean | null
) {
  if (fromEnd === false) return;

  const retainedMessages = (page + 1) * pageSize;
  if (retainedMessages > CLI_SESSIONS_MAX_TAIL_MESSAGES) {
    throw new Error(
      `SEC_INVALID_INPUT: message pagination window is too large (max ${CLI_SESSIONS_MAX_TAIL_MESSAGES} retained messages)`
    );
  }
}

export function normalizeCliSessionsProjectId(projectId: string): string {
  return normalizeRequiredText(projectId, "projectId");
}

export function normalizeCliSessionsFilePath(filePath: string): string {
  return normalizeRequiredText(filePath, "filePath");
}

export function normalizeCliSessionsFolderLookupItems(
  items: readonly CliSessionsFolderLookupInput[]
): CliSessionsFolderLookupInput[] {
  if (items.length > CLI_SESSIONS_MAX_LOOKUP_ITEMS) {
    throw new Error(
      `SEC_INVALID_INPUT: folder lookup items must contain at most ${CLI_SESSIONS_MAX_LOOKUP_ITEMS} entries`
    );
  }

  return items
    .map((item) => ({ source: item.source, session_id: item.session_id.trim() }))
    .filter((item) => item.session_id.length > 0)
    .map((item) => ({
      source: item.source,
      session_id: normalizeRequiredText(item.session_id, "sessionId"),
    }));
}

export function normalizeCliSessionsDeleteFilePaths(filePaths: readonly string[]): string[] {
  if (filePaths.length === 0) {
    throw new Error("SEC_INVALID_INPUT: filePaths is required");
  }
  if (filePaths.length > CLI_SESSIONS_MAX_DELETE_PATHS) {
    throw new Error(
      `SEC_INVALID_INPUT: filePaths must contain at most ${CLI_SESSIONS_MAX_DELETE_PATHS} entries`
    );
  }

  const normalized = filePaths
    .map((filePath) => filePath.trim())
    .filter((filePath) => filePath.length > 0)
    .map((filePath) => normalizeRequiredText(filePath, "filePath"));
  if (normalized.length === 0) {
    throw new Error("SEC_INVALID_INPUT: filePaths is required");
  }
  return normalized;
}

export function normalizeCliSessionsWslDistro(value?: string | null): string | null {
  if (value == null) return null;
  const normalized = value.trim();
  if (!normalized) return null;
  if (/[\u0000-\u001f\u007f]/u.test(normalized)) {
    throw new Error("SEC_INVALID_INPUT: WSL distro name contains control characters");
  }
  if ([...normalized].length > CLI_SESSIONS_WSL_DISTRO_MAX_CHARS) {
    throw new Error(
      `SEC_INVALID_INPUT: WSL distro name is too long (max ${CLI_SESSIONS_WSL_DISTRO_MAX_CHARS} chars)`
    );
  }
  return normalized;
}

function toCliSessionsProjectSummary(
  value: GeneratedCliSessionsProjectSummary
): CliSessionsProjectSummary {
  return {
    ...value,
    source: toCliSessionsSource(value.source, "cli_sessions_projects_list.source"),
  };
}

function toCliSessionsSessionSummary(
  value: GeneratedCliSessionsSessionSummary
): CliSessionsSessionSummary {
  return {
    ...value,
    source: toCliSessionsSource(value.source, "cli_sessions_sessions_list.source"),
  };
}

function toCliSessionsFolderLookupEntry(
  value: GeneratedCliSessionsFolderLookupEntry
): CliSessionsFolderLookupEntry {
  return {
    ...value,
    source: toCliSessionsSource(value.source, "cli_sessions_folder_lookup_by_ids.source"),
  };
}

export async function cliSessionsProjectsList(source: CliSessionsSource, wslDistro?: string) {
  const normalizedWslDistro = normalizeCliSessionsWslDistro(wslDistro);

  return invokeGeneratedIpc<CliSessionsProjectSummary[]>({
    title: "读取会话项目列表失败",
    cmd: "cli_sessions_projects_list",
    args: {
      source,
      wslDistro: normalizedWslDistro,
    },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.cliSessionsProjectsList(source, normalizedWslDistro),
        (rows) => rows.map(toCliSessionsProjectSummary)
      ),
  });
}

export async function cliSessionsSessionsList(
  source: CliSessionsSource,
  projectId: string,
  wslDistro?: string
) {
  const normalizedProjectId = normalizeCliSessionsProjectId(projectId);
  const normalizedWslDistro = normalizeCliSessionsWslDistro(wslDistro);

  return invokeGeneratedIpc<CliSessionsSessionSummary[]>({
    title: "读取会话列表失败",
    cmd: "cli_sessions_sessions_list",
    args: {
      source,
      projectId: normalizedProjectId,
      wslDistro: normalizedWslDistro,
    },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.cliSessionsSessionsList(source, normalizedProjectId, normalizedWslDistro),
        (rows) => rows.map(toCliSessionsSessionSummary)
      ),
  });
}

export async function cliSessionsMessagesGet(input: CliSessionsMessagesInput) {
  const filePath = normalizeCliSessionsFilePath(input.filePath);
  const page = normalizeCliSessionsPage(input.page);
  const pageSize = normalizeCliSessionsPageSize(input.pageSize);
  validateCliSessionsMessageWindow(page, pageSize, input.fromEnd);
  const normalizedWslDistro = normalizeCliSessionsWslDistro(input.wslDistro);

  return invokeGeneratedIpc<CliSessionsPaginatedMessages>({
    title: "读取会话消息失败",
    cmd: "cli_sessions_messages_get",
    args: {
      source: input.source,
      filePath,
      page,
      pageSize,
      fromEnd: input.fromEnd,
      wslDistro: normalizedWslDistro,
    },
    invoke: () =>
      commands.cliSessionsMessagesGet(
        input.source,
        filePath,
        page,
        pageSize,
        input.fromEnd,
        normalizedWslDistro
      ),
  });
}

export async function cliSessionsSessionDelete(input: CliSessionsSessionDeleteInput) {
  const filePaths = normalizeCliSessionsDeleteFilePaths(input.filePaths);
  const normalizedWslDistro = normalizeCliSessionsWslDistro(input.wslDistro);

  return invokeGeneratedIpc<string[]>({
    title: "删除会话失败",
    cmd: "cli_sessions_session_delete",
    args: {
      source: input.source,
      filePaths,
      wslDistro: normalizedWslDistro,
    },
    invoke: () => commands.cliSessionsSessionDelete(input.source, filePaths, normalizedWslDistro),
  });
}

export async function cliSessionsFolderLookupByIds(
  items: CliSessionsFolderLookupInput[],
  wslDistro?: string
) {
  const normalizedItems = normalizeCliSessionsFolderLookupItems(items);
  if (normalizedItems.length === 0) return [];
  const normalizedWslDistro = normalizeCliSessionsWslDistro(wslDistro);

  return invokeGeneratedIpc<CliSessionsFolderLookupEntry[]>({
    title: "读取会话文件夹信息失败",
    cmd: "cli_sessions_folder_lookup_by_ids",
    args: {
      items: normalizedItems,
      wslDistro: normalizedWslDistro,
    },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.cliSessionsFolderLookupByIds(normalizedItems, normalizedWslDistro),
        (rows) => rows.map(toCliSessionsFolderLookupEntry)
      ),
  });
}

/**
 * Escapes a shell argument for safe command execution across platforms.
 *
 * - Windows: Uses double quotes and escapes internal double quotes by doubling them
 * - Unix/Linux/macOS: Uses single quotes and escapes internal single quotes with '\''
 *
 * This prevents shell injection attacks when building commands with user-provided input.
 *
 * @param arg - The argument string to escape
 * @returns The escaped argument safe for shell execution
 *
 * @example
 * // Windows: escapeShellArg('hello "world"') => '"hello ""world"""'
 * // Unix: escapeShellArg("it's fine") => '\'it'\''s fine\''
 */
export function escapeShellArg(arg: string): string {
  // Detect platform using navigator (browser environment)
  const isWindows = typeof navigator !== "undefined" && /Win/.test(navigator.userAgent);

  // Handle empty string
  if (arg === "") {
    return isWindows ? '""' : "''";
  }

  // Windows: Use double quotes, escape internal double quotes by doubling them
  if (isWindows) {
    return `"${arg.replace(/"/g, '""')}"`;
  }

  // Unix-like systems: Use single quotes, escape single quotes with '\''
  // The pattern '\'' ends the current quote, adds an escaped single quote, and starts a new quote
  return `'${arg.replace(/'/g, "'\\''")}'`;
}

export type {
  CliSessionsDisplayContentBlock,
  CliSessionsDisplayMessage,
  CliSessionsPaginatedMessages,
};
