import { invokeService } from "../invokeServiceCommand";

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

export async function cliManagerClaudeInfoGet() {
  return invokeService<ClaudeCliInfo>("获取 Claude CLI 信息失败", "cli_manager_claude_info_get");
}

export async function cliManagerCodexInfoGet() {
  return invokeService<SimpleCliInfo>("获取 Codex CLI 信息失败", "cli_manager_codex_info_get");
}

export async function cliManagerCodexConfigGet() {
  return invokeService<CodexConfigState>("读取 Codex 配置失败", "cli_manager_codex_config_get");
}

export async function cliManagerCodexConfigSet(patch: CodexConfigPatch) {
  return invokeService<CodexConfigState>("保存 Codex 配置失败", "cli_manager_codex_config_set", {
    patch,
  });
}

export async function cliManagerCodexConfigTomlGet() {
  return invokeService<CodexConfigTomlState>(
    "读取 Codex TOML 配置失败",
    "cli_manager_codex_config_toml_get"
  );
}

export async function cliManagerCodexConfigTomlValidate(toml: string) {
  return invokeService<CodexConfigTomlValidationResult>(
    "校验 Codex TOML 配置失败",
    "cli_manager_codex_config_toml_validate",
    {
      toml,
    }
  );
}

export async function cliManagerCodexConfigTomlSet(toml: string) {
  return invokeService<CodexConfigState>(
    "保存 Codex TOML 配置失败",
    "cli_manager_codex_config_toml_set",
    { toml }
  );
}

export async function cliManagerGeminiInfoGet() {
  return invokeService<SimpleCliInfo>("获取 Gemini CLI 信息失败", "cli_manager_gemini_info_get");
}

export async function cliManagerGeminiConfigGet() {
  return invokeService<GeminiConfigState>("读取 Gemini 配置失败", "cli_manager_gemini_config_get");
}

export async function cliManagerGeminiConfigSet(patch: GeminiConfigPatch) {
  return invokeService<GeminiConfigState>("保存 Gemini 配置失败", "cli_manager_gemini_config_set", {
    patch,
  });
}

export async function cliManagerClaudeEnvSet(input: {
  mcp_timeout_ms: number | null;
  disable_error_reporting: boolean;
}) {
  return invokeService<ClaudeEnvState>("保存 Claude 环境变量失败", "cli_manager_claude_env_set", {
    mcpTimeoutMs: input.mcp_timeout_ms,
    disableErrorReporting: input.disable_error_reporting,
  });
}

export async function cliManagerClaudeSettingsGet() {
  return invokeService<ClaudeSettingsState>(
    "读取 Claude 设置失败",
    "cli_manager_claude_settings_get"
  );
}

export async function cliManagerClaudeSettingsSet(patch: ClaudeSettingsPatch) {
  return invokeService<ClaudeSettingsState>(
    "保存 Claude 设置失败",
    "cli_manager_claude_settings_set",
    { patch }
  );
}
