import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type ClaudeCliInfo = {
  found: boolean;
  executable_path: string | null;
  version: string | null;
  error: string | null;
  shell: string | null;
  resolved_via: string;
  config_dir: string;
  settings_path: string;
  mcp_timeout_ms: number | null;
  disable_error_reporting: boolean;
};

export type SimpleCliInfo = {
  found: boolean;
  executable_path: string | null;
  version: string | null;
  error: string | null;
  shell: string | null;
  resolved_via: string;
};

export type ClaudeEnvState = {
  config_dir: string;
  settings_path: string;
  mcp_timeout_ms: number | null;
  disable_error_reporting: boolean;
};

export type ClaudeSettingsState = {
  config_dir: string;
  settings_path: string;
  exists: boolean;

  model: string | null;
  output_style: string | null;
  language: string | null;
  always_thinking_enabled: boolean | null;

  show_turn_duration: boolean | null;
  spinner_tips_enabled: boolean | null;
  terminal_progress_bar_enabled: boolean | null;
  respect_gitignore: boolean | null;
  disable_git_participant: boolean;

  permissions_allow: string[];
  permissions_ask: string[];
  permissions_deny: string[];

  env_mcp_timeout_ms: number | null;
  env_mcp_tool_timeout_ms: number | null;
  env_experimental_agent_teams: boolean;
  env_claude_code_auto_compact_window: number | null;
  env_disable_background_tasks: boolean;
  env_disable_terminal_title: boolean;
  env_claude_bash_no_login: boolean;
  env_claude_code_attribution_header: boolean;
  env_claude_code_blocking_limit_override: number | null;
  env_claude_code_max_output_tokens: number | null;
  env_enable_experimental_mcp_cli: boolean;
  env_enable_tool_search: boolean;
  env_max_mcp_output_tokens: number | null;
  env_claude_code_disable_nonessential_traffic: boolean;
  env_claude_code_disable_1m_context: boolean;
  env_claude_code_proxy_resolves_hosts: boolean;
  env_claude_code_skip_prompt_history: boolean;
};

export type ClaudeSettingsPatch = Partial<{
  model: string;
  output_style: string;
  language: string;
  always_thinking_enabled: boolean;

  show_turn_duration: boolean;
  spinner_tips_enabled: boolean;
  terminal_progress_bar_enabled: boolean;
  respect_gitignore: boolean;
  disable_git_participant: boolean;

  permissions_allow: string[];
  permissions_ask: string[];
  permissions_deny: string[];

  env_mcp_timeout_ms: number;
  env_mcp_tool_timeout_ms: number;
  env_experimental_agent_teams: boolean;
  env_claude_code_auto_compact_window: number;
  env_disable_background_tasks: boolean;
  env_disable_terminal_title: boolean;
  env_claude_bash_no_login: boolean;
  env_claude_code_attribution_header: boolean;
  env_claude_code_blocking_limit_override: number;
  env_claude_code_max_output_tokens: number;
  env_enable_experimental_mcp_cli: boolean;
  env_enable_tool_search: boolean;
  env_max_mcp_output_tokens: number;
  env_claude_code_disable_nonessential_traffic: boolean;
  env_claude_code_disable_1m_context: boolean;
  env_claude_code_proxy_resolves_hosts: boolean;
  env_claude_code_skip_prompt_history: boolean;
}>;

export type CodexConfigState = {
  config_dir: string;
  config_path: string;
  user_home_default_dir: string;
  user_home_default_path: string;
  follow_codex_home_dir: string;
  follow_codex_home_path: string;
  can_open_config_dir: boolean;
  exists: boolean;

  model: string | null;
  approval_policy: string | null;
  sandbox_mode: string | null;
  model_reasoning_effort: string | null;
  plan_mode_reasoning_effort: string | null;
  web_search: string | null;
  personality: string | null;
  model_context_window: number | null;
  model_auto_compact_token_limit: number | null;
  service_tier: string | null;

  sandbox_workspace_write_network_access: boolean | null;

  features_unified_exec: boolean | null;
  features_shell_snapshot: boolean | null;
  features_apply_patch_freeform: boolean | null;
  features_shell_tool: boolean | null;
  features_exec_policy: boolean | null;
  features_remote_compaction: boolean | null;
  features_fast_mode: boolean | null;
  features_responses_websockets_v2: boolean | null;
  features_multi_agent: boolean | null;
};

export type CodexConfigPatch = Partial<{
  model: string;
  approval_policy: string;
  sandbox_mode: string;
  model_reasoning_effort: string;
  plan_mode_reasoning_effort: string;
  web_search: string;
  personality: string;
  model_context_window: number | null;
  model_auto_compact_token_limit: number | null;
  service_tier: string;

  sandbox_workspace_write_network_access: boolean;

  features_unified_exec: boolean;
  features_shell_snapshot: boolean;
  features_apply_patch_freeform: boolean;
  features_shell_tool: boolean;
  features_exec_policy: boolean;
  features_remote_compaction: boolean;
  features_fast_mode: boolean;
  features_responses_websockets_v2: boolean;
  features_multi_agent: boolean;
}>;

export type CodexConfigTomlState = {
  config_path: string;
  exists: boolean;
  toml: string;
};

export type CodexConfigTomlValidationError = {
  message: string;
  line: number | null;
  column: number | null;
};

export type CodexConfigTomlValidationResult = {
  ok: boolean;
  error: CodexConfigTomlValidationError | null;
};

export type GeminiConfigState = {
  configDir: string;
  configPath: string;
  exists: boolean;
  modelName: string | null;
  modelMaxSessionTurns: number | null;
  modelCompressionThreshold: number | null;
  defaultApprovalMode: string | null;
  enableAutoUpdate: boolean | null;
  enableNotifications: boolean | null;
  vimMode: boolean | null;
  retryFetchErrors: boolean | null;
  maxAttempts: number | null;
  uiTheme: string | null;
  uiHideBanner: boolean | null;
  uiHideTips: boolean | null;
  uiShowLineNumbers: boolean | null;
  uiInlineThinkingMode: string | null;
  usageStatisticsEnabled: boolean | null;
  sessionRetentionEnabled: boolean | null;
  sessionRetentionMaxAge: string | null;
  planModelRouting: boolean | null;
  securityAuthSelectedType: string | null;
};

export type GeminiConfigPatch = Partial<{
  modelName: string;
  modelMaxSessionTurns: number;
  modelCompressionThreshold: number;
  defaultApprovalMode: string;
  enableAutoUpdate: boolean;
  enableNotifications: boolean;
  vimMode: boolean;
  retryFetchErrors: boolean;
  maxAttempts: number;
  uiTheme: string;
  uiHideBanner: boolean;
  uiHideTips: boolean;
  uiShowLineNumbers: boolean;
  uiInlineThinkingMode: string;
  usageStatisticsEnabled: boolean;
  sessionRetentionEnabled: boolean;
  sessionRetentionMaxAge: string;
  planModelRouting: boolean;
  securityAuthSelectedType: string;
}>;

function toCodexConfigPatch(patch: CodexConfigPatch) {
  return {
    model: patch.model ?? null,
    approval_policy: patch.approval_policy ?? null,
    sandbox_mode: patch.sandbox_mode ?? null,
    model_reasoning_effort: patch.model_reasoning_effort ?? null,
    plan_mode_reasoning_effort: patch.plan_mode_reasoning_effort ?? null,
    web_search: patch.web_search ?? null,
    personality: patch.personality ?? null,
    model_context_window: patch.model_context_window ?? null,
    model_auto_compact_token_limit: patch.model_auto_compact_token_limit ?? null,
    service_tier: patch.service_tier ?? null,
    sandbox_workspace_write_network_access: patch.sandbox_workspace_write_network_access ?? null,
    features_unified_exec: patch.features_unified_exec ?? null,
    features_shell_snapshot: patch.features_shell_snapshot ?? null,
    features_apply_patch_freeform: patch.features_apply_patch_freeform ?? null,
    features_shell_tool: patch.features_shell_tool ?? null,
    features_exec_policy: patch.features_exec_policy ?? null,
    features_remote_compaction: patch.features_remote_compaction ?? null,
    features_fast_mode: patch.features_fast_mode ?? null,
    features_responses_websockets_v2: patch.features_responses_websockets_v2 ?? null,
    features_multi_agent: patch.features_multi_agent ?? null,
  };
}

function toGeminiConfigPatch(patch: GeminiConfigPatch) {
  return {
    modelName: patch.modelName ?? null,
    modelMaxSessionTurns: patch.modelMaxSessionTurns ?? null,
    modelCompressionThreshold: patch.modelCompressionThreshold ?? null,
    defaultApprovalMode: patch.defaultApprovalMode ?? null,
    enableAutoUpdate: patch.enableAutoUpdate ?? null,
    enableNotifications: patch.enableNotifications ?? null,
    vimMode: patch.vimMode ?? null,
    retryFetchErrors: patch.retryFetchErrors ?? null,
    maxAttempts: patch.maxAttempts ?? null,
    uiTheme: patch.uiTheme ?? null,
    uiHideBanner: patch.uiHideBanner ?? null,
    uiHideTips: patch.uiHideTips ?? null,
    uiShowLineNumbers: patch.uiShowLineNumbers ?? null,
    uiInlineThinkingMode: patch.uiInlineThinkingMode ?? null,
    usageStatisticsEnabled: patch.usageStatisticsEnabled ?? null,
    sessionRetentionEnabled: patch.sessionRetentionEnabled ?? null,
    sessionRetentionMaxAge: patch.sessionRetentionMaxAge ?? null,
    planModelRouting: patch.planModelRouting ?? null,
    securityAuthSelectedType: patch.securityAuthSelectedType ?? null,
  };
}

function toClaudeSettingsPatch(patch: ClaudeSettingsPatch) {
  return {
    model: patch.model ?? null,
    output_style: patch.output_style ?? null,
    language: patch.language ?? null,
    always_thinking_enabled: patch.always_thinking_enabled ?? null,
    show_turn_duration: patch.show_turn_duration ?? null,
    spinner_tips_enabled: patch.spinner_tips_enabled ?? null,
    terminal_progress_bar_enabled: patch.terminal_progress_bar_enabled ?? null,
    respect_gitignore: patch.respect_gitignore ?? null,
    disable_git_participant: patch.disable_git_participant ?? null,
    permissions_allow: patch.permissions_allow ?? null,
    permissions_ask: patch.permissions_ask ?? null,
    permissions_deny: patch.permissions_deny ?? null,
    env_mcp_timeout_ms: patch.env_mcp_timeout_ms ?? null,
    env_mcp_tool_timeout_ms: patch.env_mcp_tool_timeout_ms ?? null,
    env_experimental_agent_teams: patch.env_experimental_agent_teams ?? null,
    env_claude_code_auto_compact_window: patch.env_claude_code_auto_compact_window ?? null,
    env_disable_background_tasks: patch.env_disable_background_tasks ?? null,
    env_disable_terminal_title: patch.env_disable_terminal_title ?? null,
    env_claude_bash_no_login: patch.env_claude_bash_no_login ?? null,
    env_claude_code_attribution_header: patch.env_claude_code_attribution_header ?? null,
    env_claude_code_blocking_limit_override:
      patch.env_claude_code_blocking_limit_override ?? null,
    env_claude_code_max_output_tokens: patch.env_claude_code_max_output_tokens ?? null,
    env_enable_experimental_mcp_cli: patch.env_enable_experimental_mcp_cli ?? null,
    env_enable_tool_search: patch.env_enable_tool_search ?? null,
    env_max_mcp_output_tokens: patch.env_max_mcp_output_tokens ?? null,
    env_claude_code_disable_nonessential_traffic:
      patch.env_claude_code_disable_nonessential_traffic ?? null,
    env_claude_code_disable_1m_context: patch.env_claude_code_disable_1m_context ?? null,
    env_claude_code_proxy_resolves_hosts: patch.env_claude_code_proxy_resolves_hosts ?? null,
    env_claude_code_skip_prompt_history: patch.env_claude_code_skip_prompt_history ?? null,
  };
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
      commands.cliManagerGeminiConfigGet() as Promise<
        GeneratedCommandResult<GeminiConfigState>
      >,
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

export async function cliManagerClaudeEnvSet(input: {
  mcp_timeout_ms: number | null;
  disable_error_reporting: boolean;
}) {
  return invokeGeneratedIpc<ClaudeEnvState>({
    title: "保存 Claude 环境变量失败",
    cmd: "cli_manager_claude_env_set",
    args: {
      mcpTimeoutMs: input.mcp_timeout_ms,
      disableErrorReporting: input.disable_error_reporting,
    },
    invoke: () =>
      commands.cliManagerClaudeEnvSet(
        input.mcp_timeout_ms,
        input.disable_error_reporting
      ) as Promise<GeneratedCommandResult<ClaudeEnvState>>,
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
