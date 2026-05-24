import {
  commands,
  type ClaudeCliInfo as GeneratedClaudeCliInfo,
  type ClaudeEnvState as GeneratedClaudeEnvState,
  type ClaudeHookGroup as GeneratedClaudeHookGroup,
  type ClaudeHooksSetInput as GeneratedClaudeHooksSetInput,
  type ClaudeHooksState as GeneratedClaudeHooksState,
  type ClaudeSettingsPatch as GeneratedClaudeSettingsPatch,
  type ClaudeSettingsState as GeneratedClaudeSettingsState,
  type CodexConfigPatch as GeneratedCodexConfigPatch,
  type CodexConfigState as GeneratedCodexConfigState,
  type CodexConfigTomlState as GeneratedCodexConfigTomlState,
  type CodexConfigTomlValidationError as GeneratedCodexConfigTomlValidationError,
  type CodexConfigTomlValidationResult as GeneratedCodexConfigTomlValidationResult,
  type GeminiConfigPatch as GeneratedGeminiConfigPatch,
  type GeminiConfigState as GeneratedGeminiConfigState,
  type SimpleCliInfo as GeneratedSimpleCliInfo,
} from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type ClaudeCliInfo = GeneratedClaudeCliInfo;
export type SimpleCliInfo = GeneratedSimpleCliInfo;
export type ClaudeEnvState = GeneratedClaudeEnvState;
export type ClaudeSettingsState = GeneratedClaudeSettingsState;
export type ClaudeSettingsPatch = Partial<GeneratedClaudeSettingsPatch>;
export type ClaudeHooksState = GeneratedClaudeHooksState;
export type ClaudeHookGroup = GeneratedClaudeHookGroup;
export type ClaudeHooksSetInput = GeneratedClaudeHooksSetInput;
export type CodexConfigState = GeneratedCodexConfigState;
export type CodexConfigPatch = Partial<GeneratedCodexConfigPatch>;
export type CodexConfigTomlState = GeneratedCodexConfigTomlState;
export type CodexConfigTomlValidationError = GeneratedCodexConfigTomlValidationError;
export type CodexConfigTomlValidationResult = GeneratedCodexConfigTomlValidationResult;
export type GeminiConfigState = GeneratedGeminiConfigState;
export type GeminiConfigPatch = Partial<GeneratedGeminiConfigPatch>;
export type ClaudeEnvSetInput = {
  mcpTimeoutMs: number | null;
  disableErrorReporting: boolean;
};

const DEFAULT_CODEX_CONFIG_PATCH = {
  model: null,
  approval_policy: null,
  sandbox_mode: null,
  model_reasoning_effort: null,
  plan_mode_reasoning_effort: null,
  web_search: null,
  personality: null,
  model_context_window: null,
  model_auto_compact_token_limit: null,
  service_tier: null,
  sandbox_workspace_write_network_access: null,
  features_unified_exec: null,
  features_shell_snapshot: null,
  features_apply_patch_freeform: null,
  features_shell_tool: null,
  features_exec_policy: null,
  features_remote_compaction: null,
  features_fast_mode: null,
  features_responses_websockets_v2: null,
  features_multi_agent: null,
} satisfies GeneratedCodexConfigPatch;

const DEFAULT_GEMINI_CONFIG_PATCH = {
  modelName: null,
  modelMaxSessionTurns: null,
  modelCompressionThreshold: null,
  defaultApprovalMode: null,
  enableAutoUpdate: null,
  enableNotifications: null,
  vimMode: null,
  retryFetchErrors: null,
  maxAttempts: null,
  uiTheme: null,
  uiHideBanner: null,
  uiHideTips: null,
  uiShowLineNumbers: null,
  uiInlineThinkingMode: null,
  usageStatisticsEnabled: null,
  sessionRetentionEnabled: null,
  sessionRetentionMaxAge: null,
  planModelRouting: null,
  securityAuthSelectedType: null,
} satisfies GeneratedGeminiConfigPatch;

const DEFAULT_CLAUDE_SETTINGS_PATCH = {
  model: null,
  output_style: null,
  language: null,
  always_thinking_enabled: null,
  show_turn_duration: null,
  spinner_tips_enabled: null,
  terminal_progress_bar_enabled: null,
  respect_gitignore: null,
  disable_git_participant: null,
  permissions_allow: null,
  permissions_ask: null,
  permissions_deny: null,
  env_mcp_timeout_ms: null,
  env_mcp_tool_timeout_ms: null,
  env_experimental_agent_teams: null,
  env_claude_code_auto_compact_window: null,
  env_disable_background_tasks: null,
  env_disable_terminal_title: null,
  env_claude_bash_no_login: null,
  env_claude_code_attribution_header: null,
  env_claude_code_blocking_limit_override: null,
  env_claude_code_max_output_tokens: null,
  env_enable_experimental_mcp_cli: null,
  env_enable_tool_search: null,
  env_max_mcp_output_tokens: null,
  env_claude_code_disable_nonessential_traffic: null,
  env_claude_code_disable_1m_context: null,
  env_claude_code_proxy_resolves_hosts: null,
  env_claude_code_skip_prompt_history: null,
} satisfies GeneratedClaudeSettingsPatch;

function withGeneratedPatchDefaults<TPatch extends object>(
  defaults: TPatch,
  patch: Partial<TPatch>
): TPatch {
  return {
    ...defaults,
    ...patch,
  };
}

function toCodexConfigPatch(patch: CodexConfigPatch): GeneratedCodexConfigPatch {
  return withGeneratedPatchDefaults(DEFAULT_CODEX_CONFIG_PATCH, patch);
}

function toGeminiConfigPatch(patch: GeminiConfigPatch): GeneratedGeminiConfigPatch {
  return withGeneratedPatchDefaults(DEFAULT_GEMINI_CONFIG_PATCH, patch);
}

function toClaudeSettingsPatch(patch: ClaudeSettingsPatch): GeneratedClaudeSettingsPatch {
  return withGeneratedPatchDefaults(DEFAULT_CLAUDE_SETTINGS_PATCH, patch);
}

export async function cliManagerClaudeInfoGet() {
  return invokeGeneratedIpc<ClaudeCliInfo>({
    title: "获取 Claude CLI 信息失败",
    cmd: "cli_manager_claude_info_get",
    invoke: () =>
      commands.cliManagerClaudeInfoGet() as Promise<GeneratedCommandResult<ClaudeCliInfo>>,
  });
}

export async function cliManagerCodexInfoGet() {
  return invokeGeneratedIpc<SimpleCliInfo>({
    title: "获取 Codex CLI 信息失败",
    cmd: "cli_manager_codex_info_get",
    invoke: () =>
      commands.cliManagerCodexInfoGet() as Promise<GeneratedCommandResult<SimpleCliInfo>>,
  });
}

export async function cliManagerCodexConfigGet() {
  return invokeGeneratedIpc<CodexConfigState>({
    title: "读取 Codex 配置失败",
    cmd: "cli_manager_codex_config_get",
    invoke: () =>
      commands.cliManagerCodexConfigGet() as Promise<GeneratedCommandResult<CodexConfigState>>,
  });
}

export async function cliManagerCodexConfigSet(patch: CodexConfigPatch) {
  const normalizedPatch = toCodexConfigPatch(patch);
  return invokeGeneratedIpc<CodexConfigState>({
    title: "保存 Codex 配置失败",
    cmd: "cli_manager_codex_config_set",
    args: { patch: normalizedPatch },
    invoke: () =>
      commands.cliManagerCodexConfigSet(normalizedPatch) as Promise<
        GeneratedCommandResult<CodexConfigState>
      >,
  });
}

export async function cliManagerCodexConfigTomlGet() {
  return invokeGeneratedIpc<CodexConfigTomlState>({
    title: "读取 Codex TOML 配置失败",
    cmd: "cli_manager_codex_config_toml_get",
    invoke: () =>
      commands.cliManagerCodexConfigTomlGet() as Promise<
        GeneratedCommandResult<CodexConfigTomlState>
      >,
  });
}

export async function cliManagerCodexConfigTomlValidate(toml: string) {
  return invokeGeneratedIpc<CodexConfigTomlValidationResult>({
    title: "校验 Codex TOML 配置失败",
    cmd: "cli_manager_codex_config_toml_validate",
    args: { toml },
    invoke: () =>
      commands.cliManagerCodexConfigTomlValidate(toml) as Promise<
        GeneratedCommandResult<CodexConfigTomlValidationResult>
      >,
  });
}

export async function cliManagerCodexConfigTomlSet(toml: string) {
  return invokeGeneratedIpc<CodexConfigState>({
    title: "保存 Codex TOML 配置失败",
    cmd: "cli_manager_codex_config_toml_set",
    args: { toml },
    invoke: () =>
      commands.cliManagerCodexConfigTomlSet(toml) as Promise<
        GeneratedCommandResult<CodexConfigState>
      >,
  });
}

export async function cliManagerGeminiInfoGet() {
  return invokeGeneratedIpc<SimpleCliInfo>({
    title: "获取 Gemini CLI 信息失败",
    cmd: "cli_manager_gemini_info_get",
    invoke: () =>
      commands.cliManagerGeminiInfoGet() as Promise<GeneratedCommandResult<SimpleCliInfo>>,
  });
}

export async function cliManagerGeminiConfigGet() {
  return invokeGeneratedIpc<GeminiConfigState>({
    title: "读取 Gemini 配置失败",
    cmd: "cli_manager_gemini_config_get",
    invoke: () =>
      commands.cliManagerGeminiConfigGet() as Promise<GeneratedCommandResult<GeminiConfigState>>,
  });
}

export async function cliManagerGeminiConfigSet(patch: GeminiConfigPatch) {
  const normalizedPatch = toGeminiConfigPatch(patch);
  return invokeGeneratedIpc<GeminiConfigState>({
    title: "保存 Gemini 配置失败",
    cmd: "cli_manager_gemini_config_set",
    args: { patch: normalizedPatch },
    invoke: () =>
      commands.cliManagerGeminiConfigSet(normalizedPatch) as Promise<
        GeneratedCommandResult<GeminiConfigState>
      >,
  });
}

export async function cliManagerClaudeEnvSet(input: ClaudeEnvSetInput) {
  return invokeGeneratedIpc<ClaudeEnvState>({
    title: "保存 Claude 环境变量失败",
    cmd: "cli_manager_claude_env_set",
    args: {
      mcpTimeoutMs: input.mcpTimeoutMs,
      disableErrorReporting: input.disableErrorReporting,
    },
    invoke: () =>
      commands.cliManagerClaudeEnvSet(input.mcpTimeoutMs, input.disableErrorReporting) as Promise<
        GeneratedCommandResult<ClaudeEnvState>
      >,
  });
}

export async function cliManagerClaudeSettingsGet() {
  return invokeGeneratedIpc<ClaudeSettingsState>({
    title: "读取 Claude 设置失败",
    cmd: "cli_manager_claude_settings_get",
    invoke: () =>
      commands.cliManagerClaudeSettingsGet() as Promise<
        GeneratedCommandResult<ClaudeSettingsState>
      >,
  });
}

export async function cliManagerClaudeSettingsSet(patch: ClaudeSettingsPatch) {
  const normalizedPatch = toClaudeSettingsPatch(patch);
  return invokeGeneratedIpc<ClaudeSettingsState>({
    title: "保存 Claude 设置失败",
    cmd: "cli_manager_claude_settings_set",
    args: { patch: normalizedPatch },
    invoke: () =>
      commands.cliManagerClaudeSettingsSet(normalizedPatch) as Promise<
        GeneratedCommandResult<ClaudeSettingsState>
      >,
  });
}

export async function cliManagerClaudeHooksGet() {
  return invokeGeneratedIpc<ClaudeHooksState>({
    title: "读取 Claude Hooks 失败",
    cmd: "cli_manager_claude_hooks_get",
    invoke: () =>
      commands.cliManagerClaudeHooksGet() as Promise<GeneratedCommandResult<ClaudeHooksState>>,
  });
}

export async function cliManagerClaudeHooksSet(input: ClaudeHooksSetInput) {
  return invokeGeneratedIpc<ClaudeHooksState>({
    title: "保存 Claude Hooks 失败",
    cmd: "cli_manager_claude_hooks_set",
    args: { input },
    invoke: () =>
      commands.cliManagerClaudeHooksSet(input) as Promise<GeneratedCommandResult<ClaudeHooksState>>,
  });
}
