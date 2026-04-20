import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type CliSessionsSource = "claude" | "codex";

export type CliSessionsProjectSummary = {
  source: CliSessionsSource;
  id: string;
  display_path: string;
  short_name: string;
  session_count: number;
  last_modified: number | null;
  model_provider: string | null;
  wsl_distro: string | null;
};

export type CliSessionsSessionSummary = {
  source: CliSessionsSource;
  session_id: string;
  file_path: string;
  first_prompt: string | null;
  message_count: number;
  created_at: number | null;
  modified_at: number | null;
  git_branch: string | null;
  project_path: string | null;
  is_sidechain: boolean | null;
  cwd: string | null;
  model_provider: string | null;
  cli_version: string | null;
  wsl_distro: string | null;
};

export type CliSessionsFolderLookupInput = {
  source: CliSessionsSource;
  session_id: string;
};

export type CliSessionsFolderLookupEntry = {
  source: CliSessionsSource;
  session_id: string;
  folder_name: string;
  folder_path: string;
};

export type CliSessionsDisplayContentBlock =
  | { type: "text"; text: string }
  | { type: "thinking"; thinking: string }
  | { type: "tool_use"; id: string; name: string; input: string }
  | { type: "tool_result"; tool_use_id: string; content: string; is_error: boolean }
  | { type: "reasoning"; text: string }
  | { type: "function_call"; name: string; arguments: string; call_id: string }
  | { type: "function_call_output"; call_id: string; output: string };

export type CliSessionsDisplayMessage = {
  uuid: string | null;
  role: string;
  timestamp: string | null;
  model: string | null;
  content: CliSessionsDisplayContentBlock[];
};

export type CliSessionsPaginatedMessages = {
  messages: CliSessionsDisplayMessage[];
  total: number;
  page: number;
  page_size: number;
  has_more: boolean;
};

export async function cliSessionsProjectsList(source: CliSessionsSource, wslDistro?: string) {
  return invokeGeneratedIpc<CliSessionsProjectSummary[]>({
    title: "读取会话项目列表失败",
    cmd: "cli_sessions_projects_list",
    args: {
      source,
      wslDistro: wslDistro ?? null,
    },
    invoke: () =>
      commands.cliSessionsProjectsList(source, wslDistro ?? null) as Promise<
        GeneratedCommandResult<CliSessionsProjectSummary[]>
      >,
  });
}

export async function cliSessionsSessionsList(
  source: CliSessionsSource,
  projectId: string,
  wslDistro?: string
) {
  return invokeGeneratedIpc<CliSessionsSessionSummary[]>({
    title: "读取会话列表失败",
    cmd: "cli_sessions_sessions_list",
    args: {
      source,
      projectId,
      wslDistro: wslDistro ?? null,
    },
    invoke: () =>
      commands.cliSessionsSessionsList(source, projectId, wslDistro ?? null) as Promise<
        GeneratedCommandResult<CliSessionsSessionSummary[]>
      >,
  });
}

export async function cliSessionsMessagesGet(input: {
  source: CliSessionsSource;
  file_path: string;
  page: number;
  page_size: number;
  from_end: boolean;
  wsl_distro?: string;
}) {
  return invokeGeneratedIpc<CliSessionsPaginatedMessages>({
    title: "读取会话消息失败",
    cmd: "cli_sessions_messages_get",
    args: {
      source: input.source,
      filePath: input.file_path,
      page: input.page,
      pageSize: input.page_size,
      fromEnd: input.from_end,
      wslDistro: input.wsl_distro ?? null,
    },
    invoke: () =>
      commands.cliSessionsMessagesGet(
        input.source,
        input.file_path,
        input.page,
        input.page_size,
        input.from_end,
        input.wsl_distro ?? null
      ) as Promise<GeneratedCommandResult<CliSessionsPaginatedMessages>>,
  });
}

export async function cliSessionsSessionDelete(input: {
  source: CliSessionsSource;
  file_paths: string[];
  wsl_distro?: string;
}) {
  return invokeGeneratedIpc<string[]>({
    title: "删除会话失败",
    cmd: "cli_sessions_session_delete",
    args: {
      source: input.source,
      filePaths: input.file_paths,
      wslDistro: input.wsl_distro ?? null,
    },
    invoke: () =>
      commands.cliSessionsSessionDelete(
        input.source,
        input.file_paths,
        input.wsl_distro ?? null
      ) as Promise<GeneratedCommandResult<string[]>>,
  });
}

export async function cliSessionsFolderLookupByIds(
  items: CliSessionsFolderLookupInput[],
  wslDistro?: string
) {
  return invokeGeneratedIpc<CliSessionsFolderLookupEntry[]>({
    title: "读取会话文件夹信息失败",
    cmd: "cli_sessions_folder_lookup_by_ids",
    args: {
      items,
      wslDistro: wslDistro ?? null,
    },
    invoke: () =>
      commands.cliSessionsFolderLookupByIds(items as any, wslDistro ?? null) as Promise<
        GeneratedCommandResult<CliSessionsFolderLookupEntry[]>
      >,
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
