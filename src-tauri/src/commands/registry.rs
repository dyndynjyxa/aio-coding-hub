//! Usage: Single-source Tauri command registry.
//!
//! Runtime invoke registration and Specta export both derive from this module.
//! Commands that cannot be exported through Specta must stay in one explicit
//! runtime-only exception list.

use super::*;

#[cfg(test)]
const HANDWRITTEN_RUNTIME_ONLY_COMMANDS: &[&str] = &["desktop_updater_download_and_install"];
#[cfg(test)]
const HANDWRITTEN_RUNTIME_ONLY_REASON: &str =
    "Requires a Tauri Channel callback, so this desktop updater path stays as the single handwritten desktop IPC exception.";

macro_rules! generated_command_registry {
    ($callback:ident) => {
        $callback! {
            // ── settings ──
            settings_get => crate::commands::settings::settings_get,
            settings_set => crate::commands::settings::settings_set,
            settings_gateway_rectifier_set => crate::commands::settings::settings_gateway_rectifier_set,
            settings_circuit_breaker_notice_set => crate::commands::settings::settings_circuit_breaker_notice_set,
            settings_codex_session_id_completion_set => crate::commands::settings::settings_codex_session_id_completion_set,
            config_export => crate::commands::config_migrate::config_export,
            config_import => crate::commands::config_migrate::config_import,
            // ── app ──
            app_about_get => crate::commands::app::app_about_get,
            app_data_dir_get => crate::commands::data_management::app_data_dir_get,
            app_exit => crate::commands::app::app_exit,
            app_restart => crate::commands::app::app_restart,
            app_heartbeat_pong => crate::commands::app::app_heartbeat_pong,
            app_startup_status_get => crate::commands::app::app_startup_status_get,
            app_startup_retry => crate::commands::app::app_startup_retry,
            app_frontend_error_report => crate::commands::app::app_frontend_error_report,
            desktop_clipboard_write_text => crate::commands::desktop::desktop_clipboard_write_text,
            desktop_dialog_open => crate::commands::desktop::desktop_dialog_open,
            desktop_dialog_save => crate::commands::desktop::desktop_dialog_save,
            desktop_notification_is_permission_granted => crate::commands::desktop::desktop_notification_is_permission_granted,
            desktop_notification_notify => crate::commands::desktop::desktop_notification_notify,
            desktop_notification_play_sound => crate::commands::desktop::desktop_notification_play_sound,
            desktop_notification_request_permission => crate::commands::desktop::desktop_notification_request_permission,
            desktop_opener_open_path => crate::commands::desktop::desktop_opener_open_path,
            desktop_opener_open_url => crate::commands::desktop::desktop_opener_open_url,
            desktop_opener_reveal_item_in_dir => crate::commands::desktop::desktop_opener_reveal_item_in_dir,
            desktop_updater_check => crate::commands::desktop::desktop_updater_check,
            desktop_window_set_theme => crate::commands::desktop::desktop_window_set_theme,
            // ── notice ──
            notice_send => crate::commands::notice::notice_send,
            // ── cli_manager ──
            cli_manager_claude_info_get => crate::commands::cli_manager::cli_manager_claude_info_get,
            cli_manager_codex_info_get => crate::commands::cli_manager::cli_manager_codex_info_get,
            cli_manager_codex_config_get => crate::commands::cli_manager::cli_manager_codex_config_get,
            cli_manager_codex_config_set => crate::commands::cli_manager::cli_manager_codex_config_set,
            cli_manager_codex_config_toml_get => crate::commands::cli_manager::cli_manager_codex_config_toml_get,
            cli_manager_codex_config_toml_validate => crate::commands::cli_manager::cli_manager_codex_config_toml_validate,
            cli_manager_codex_config_toml_set => crate::commands::cli_manager::cli_manager_codex_config_toml_set,
            cli_manager_gemini_info_get => crate::commands::cli_manager::cli_manager_gemini_info_get,
            cli_manager_gemini_config_get => crate::commands::cli_manager::cli_manager_gemini_config_get,
            cli_manager_gemini_config_set => crate::commands::cli_manager::cli_manager_gemini_config_set,
            cli_manager_claude_env_set => crate::commands::cli_manager::cli_manager_claude_env_set,
            cli_manager_claude_settings_get => crate::commands::cli_manager::cli_manager_claude_settings_get,
            cli_manager_claude_settings_set => crate::commands::cli_manager::cli_manager_claude_settings_set,
            cli_manager_claude_hooks_get => crate::commands::cli_manager::cli_manager_claude_hooks_get,
            cli_manager_claude_hooks_set => crate::commands::cli_manager::cli_manager_claude_hooks_set,
            cli_check_latest_version => crate::commands::cli_update::cli_check_latest_version,
            cli_update => crate::commands::cli_update::cli_update,
            // ── gateway ──
            gateway_start => crate::commands::gateway::gateway_start,
            gateway_stop => crate::commands::gateway::gateway_stop,
            gateway_status => crate::commands::gateway::gateway_status,
            gateway_check_port_available => crate::commands::gateway::gateway_check_port_available,
            gateway_sessions_list => crate::commands::gateway::gateway_sessions_list,
            gateway_circuit_status => crate::commands::gateway::gateway_circuit_status,
            gateway_circuit_reset_provider => crate::commands::gateway::gateway_circuit_reset_provider,
            gateway_circuit_reset_cli => crate::commands::gateway::gateway_circuit_reset_cli,
            gateway_upstream_proxy_validate => crate::commands::gateway::gateway_upstream_proxy_validate,
            gateway_upstream_proxy_test => crate::commands::gateway::gateway_upstream_proxy_test,
            gateway_upstream_proxy_detect_ip => crate::commands::gateway::gateway_upstream_proxy_detect_ip,
            // ── wsl ──
            wsl_detect => crate::commands::wsl::wsl_detect,
            wsl_host_address_get => crate::commands::wsl::wsl_host_address_get,
            wsl_config_status_get => crate::commands::wsl::wsl_config_status_get,
            wsl_configure_clients => crate::commands::wsl::wsl_configure_clients,
            // ── cli_sessions ──
            cli_sessions_projects_list => crate::commands::cli_sessions::cli_sessions_projects_list,
            cli_sessions_sessions_list => crate::commands::cli_sessions::cli_sessions_sessions_list,
            cli_sessions_messages_get => crate::commands::cli_sessions::cli_sessions_messages_get,
            cli_sessions_session_delete => crate::commands::cli_sessions::cli_sessions_session_delete,
            // ── providers ──
            providers_list => crate::commands::providers::providers_list,
            provider_upsert => crate::commands::providers::provider_upsert,
            provider_duplicate => crate::commands::providers::provider_duplicate,
            provider_set_enabled => crate::commands::providers::provider_set_enabled,
            provider_delete => crate::commands::providers::provider_delete,
            providers_reorder => crate::commands::providers::providers_reorder,
            provider_claude_terminal_launch_command => crate::commands::providers::provider_claude_terminal_launch_command,
            provider_copy_api_key_to_clipboard => crate::commands::providers::provider_copy_api_key_to_clipboard,
            base_url_ping_ms => crate::commands::providers::base_url_ping_ms,
            provider_test_availability => crate::commands::provider_availability::provider_test_availability,
            provider_oauth_start_flow => crate::commands::providers::provider_oauth_start_flow,
            provider_oauth_refresh => crate::commands::providers::provider_oauth_refresh,
            provider_oauth_disconnect => crate::commands::providers::provider_oauth_disconnect,
            provider_oauth_status => crate::commands::providers::provider_oauth_status,
            provider_oauth_fetch_limits => crate::commands::providers::provider_oauth_fetch_limits,
            // ── claude_model_validation ──
            claude_provider_validate_model => crate::commands::claude_model_validation::claude_provider_validate_model,
            claude_validation_history_list => crate::commands::claude_model_validation::claude_validation_history_list,
            claude_validation_history_clear_provider => crate::commands::claude_model_validation::claude_validation_history_clear_provider,
            // ── sort_modes ──
            sort_modes_list => crate::commands::sort_modes::sort_modes_list,
            sort_mode_create => crate::commands::sort_modes::sort_mode_create,
            sort_mode_rename => crate::commands::sort_modes::sort_mode_rename,
            sort_mode_delete => crate::commands::sort_modes::sort_mode_delete,
            sort_mode_active_list => crate::commands::sort_modes::sort_mode_active_list,
            sort_mode_active_set => crate::commands::sort_modes::sort_mode_active_set,
            sort_mode_providers_list => crate::commands::sort_modes::sort_mode_providers_list,
            sort_mode_providers_set_order => crate::commands::sort_modes::sort_mode_providers_set_order,
            sort_mode_provider_set_enabled => crate::commands::sort_modes::sort_mode_provider_set_enabled,
            // ── model_prices ──
            model_prices_list => crate::commands::model_prices::model_prices_list,
            model_price_upsert => crate::commands::model_prices::model_price_upsert,
            model_prices_sync_basellm => crate::commands::model_prices::model_prices_sync_basellm,
            model_price_aliases_get => crate::commands::model_prices::model_price_aliases_get,
            model_price_aliases_set => crate::commands::model_prices::model_price_aliases_set,
            // ── prompts ──
            prompts_list => crate::commands::prompts::prompts_list,
            prompts_default_sync_from_files => crate::commands::prompts::prompts_default_sync_from_files,
            prompt_upsert => crate::commands::prompts::prompt_upsert,
            prompt_set_enabled => crate::commands::prompts::prompt_set_enabled,
            prompt_delete => crate::commands::prompts::prompt_delete,
            // ── mcp ──
            mcp_servers_list => crate::commands::mcp::mcp_servers_list,
            mcp_server_upsert => crate::commands::mcp::mcp_server_upsert,
            mcp_server_set_enabled => crate::commands::mcp::mcp_server_set_enabled,
            mcp_server_delete => crate::commands::mcp::mcp_server_delete,
            mcp_parse_json => crate::commands::mcp::mcp_parse_json,
            mcp_import_servers => crate::commands::mcp::mcp_import_servers,
            mcp_import_from_workspace_cli => crate::commands::mcp::mcp_import_from_workspace_cli,
            // ── skills ──
            skill_repos_list => crate::commands::skills::skill_repos_list,
            skill_repo_upsert => crate::commands::skills::skill_repo_upsert,
            skill_repo_delete => crate::commands::skills::skill_repo_delete,
            skills_installed_list => crate::commands::skills::skills_installed_list,
            skills_discover_available => crate::commands::skills::skills_discover_available,
            skill_install => crate::commands::skills::skill_install,
            skill_install_to_local => crate::commands::skills::skill_install_to_local,
            skill_set_enabled => crate::commands::skills::skill_set_enabled,
            skill_uninstall => crate::commands::skills::skill_uninstall,
            skill_return_to_local => crate::commands::skills::skill_return_to_local,
            skills_local_list => crate::commands::skills::skills_local_list,
            skill_local_delete => crate::commands::skills::skill_local_delete,
            skill_import_local => crate::commands::skills::skill_import_local,
            skills_import_local_batch => crate::commands::skills::skills_import_local_batch,
            skills_paths_get => crate::commands::skills::skills_paths_get,
            skill_check_updates => crate::commands::skills::skill_check_updates,
            skill_update => crate::commands::skills::skill_update,
            // ── request_logs ──
            request_logs_list => crate::commands::request_logs::request_logs_list,
            request_logs_list_all => crate::commands::request_logs::request_logs_list_all,
            request_logs_list_after_id => crate::commands::request_logs::request_logs_list_after_id,
            request_logs_list_after_id_all => crate::commands::request_logs::request_logs_list_after_id_all,
            request_log_get => crate::commands::request_logs::request_log_get,
            request_log_get_by_trace_id => crate::commands::request_logs::request_log_get_by_trace_id,
            request_attempt_logs_by_trace_id => crate::commands::request_logs::request_attempt_logs_by_trace_id,
            cli_sessions_folder_lookup_by_ids => crate::commands::cli_sessions::cli_sessions_folder_lookup_by_ids,
            // ── data_management ──
            db_disk_usage_get => crate::commands::data_management::db_disk_usage_get,
            request_logs_clear_all => crate::commands::data_management::request_logs_clear_all,
            app_data_reset => crate::commands::data_management::app_data_reset,
            // ── usage ──
            usage_summary => crate::commands::usage::usage_summary,
            usage_summary_v2 => crate::commands::usage::usage_summary_v2,
            usage_leaderboard_provider => crate::commands::usage::usage_leaderboard_provider,
            usage_leaderboard_day => crate::commands::usage::usage_leaderboard_day,
            usage_leaderboard_v2 => crate::commands::usage::usage_leaderboard_v2,
            usage_hourly_series => crate::commands::usage::usage_hourly_series,
            usage_day_detail_v1 => crate::commands::usage::usage_day_detail_v1,
            usage_folder_options_v1 => crate::commands::usage::usage_folder_options_v1,
            usage_provider_cache_rate_trend_v1 => crate::commands::usage::usage_provider_cache_rate_trend_v1,
            // ── cost ──
            cost_summary_v1 => crate::commands::cost::cost_summary_v1,
            cost_trend_v1 => crate::commands::cost::cost_trend_v1,
            cost_breakdown_provider_v1 => crate::commands::cost::cost_breakdown_provider_v1,
            cost_breakdown_model_v1 => crate::commands::cost::cost_breakdown_model_v1,
            cost_scatter_cli_provider_model_v1 => crate::commands::cost::cost_scatter_cli_provider_model_v1,
            cost_top_requests_v1 => crate::commands::cost::cost_top_requests_v1,
            cost_backfill_missing_v1 => crate::commands::cost::cost_backfill_missing_v1,
            // ── env_conflicts ──
            env_conflicts_check => crate::commands::env_conflicts::env_conflicts_check,
            // ── cli_proxy ──
            cli_proxy_status_all => crate::commands::cli_proxy::cli_proxy_status_all,
            cli_proxy_set_enabled => crate::commands::cli_proxy::cli_proxy_set_enabled,
            cli_proxy_sync_enabled => crate::commands::cli_proxy::cli_proxy_sync_enabled,
            cli_proxy_rebind_codex_home => crate::commands::cli_proxy::cli_proxy_rebind_codex_home,
            // ── provider_limit_usage ──
            provider_limit_usage_v1 => crate::commands::provider_limit_usage::provider_limit_usage_v1,
            // ── workspaces ──
            workspaces_list => crate::commands::workspaces::workspaces_list,
            workspace_create => crate::commands::workspaces::workspace_create,
            workspace_rename => crate::commands::workspaces::workspace_rename,
            workspace_delete => crate::commands::workspaces::workspace_delete,
            workspace_preview => crate::commands::workspaces::workspace_preview,
            workspace_apply => crate::commands::workspaces::workspace_apply,
        }
    };
}

pub(crate) fn register_runtime_commands(
    builder: tauri::Builder<tauri::Wry>,
) -> tauri::Builder<tauri::Wry> {
    macro_rules! build_runtime_handler {
        ($($name:ident => $path:path),+ $(,)?) => {
            builder.invoke_handler(tauri::generate_handler![
                $($name,)*
                desktop_updater_download_and_install,
            ])
        };
    }

    generated_command_registry!(build_runtime_handler)
}

pub(crate) fn export_typescript_bindings(output_path: &str) -> Result<(), String> {
    macro_rules! collect_exported_commands {
        ($($name:ident => $path:path),+ $(,)?) => {
            tauri_specta::Builder::<tauri::Wry>::new().commands(
                tauri_specta::collect_commands![$($name,)*]
            )
        };
    }

    let builder = generated_command_registry!(collect_exported_commands);

    builder
        .export(
            specta_typescript::Typescript::default()
                .header(
                    "/* eslint-disable */\n// @ts-nocheck\n// NOTE: Generated IPC contract for settings, config migration, desktop, app management, gateway, request-log, CLI update, CLI proxy, provider, WSL, sort-mode, provider-limit, usage, cost, model-price, prompt, workspace, skills, MCP, CLI manager, CLI sessions, Claude validation, notice, and env-conflict command families.",
                )
                .bigint(specta_typescript::BigIntExportBehavior::Number),
            output_path,
        )
        .map_err(|error| format!("failed to export specta TypeScript bindings: {error}"))
}

#[cfg(test)]
fn generated_command_names() -> &'static [&'static str] {
    macro_rules! collect_names {
        ($($name:ident => $path:path),+ $(,)?) => {
            &[$(stringify!($name),)*]
        };
    }

    generated_command_registry!(collect_names)
}

#[cfg(test)]
mod tests {
    use super::{
        generated_command_names, HANDWRITTEN_RUNTIME_ONLY_COMMANDS, HANDWRITTEN_RUNTIME_ONLY_REASON,
    };

    #[test]
    fn keeps_updater_install_as_the_only_runtime_only_command() {
        assert_eq!(
            HANDWRITTEN_RUNTIME_ONLY_COMMANDS,
            &["desktop_updater_download_and_install"]
        );
        assert!(
            HANDWRITTEN_RUNTIME_ONLY_REASON.contains("Channel callback"),
            "handwritten updater exception reason should stay explicit"
        );
        assert!(
            !generated_command_names().contains(&HANDWRITTEN_RUNTIME_ONLY_COMMANDS[0]),
            "runtime-only command must stay outside Specta-generated bindings"
        );
    }

    #[test]
    fn includes_model_price_upsert_in_generated_command_registry() {
        assert!(
            generated_command_names().contains(&"model_price_upsert"),
            "model_price_upsert should stay in the shared generated command registry"
        );
    }
}
