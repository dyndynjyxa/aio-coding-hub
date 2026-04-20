mod app;
mod commands;
mod domain;
mod gateway;
mod infra;
mod shared;
pub mod test_support;

pub(crate) use app::{app_state, notice, resident};
pub(crate) use domain::{
    claude_model_validation, claude_model_validation_history, claude_plugins, cli_sessions, cost,
    cost_stats, mcp, prompts, provider_limit_usage, providers, skills, sort_modes, usage,
    usage_stats, workspace_switch, workspaces,
};
pub(crate) use gateway::session_manager;
pub(crate) use infra::{
    app_paths, base_url_probe, claude_settings, cli_manager, cli_proxy, cli_update, codex_config,
    codex_paths, data_management, db, env_conflicts, gemini_config, mcp_sync, model_price_aliases,
    model_prices, model_prices_sync, prompt_sync, provider_circuit_breakers, request_attempt_logs,
    request_logs, settings, wsl,
};
pub(crate) use shared::{blocking, circuit_breaker};

use app_state::{ensure_db_ready, DbInitState, GatewayState};
use commands::*;
use shared::mutex_ext::MutexExt;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

pub(crate) static EXIT_CLEANUP_SPAWNED: AtomicBool = AtomicBool::new(false);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Must run before Tauri initialises WebKitGTK to prevent EGL display
    // creation failure on Wayland (AppImage bundled-lib conflict, issue #93).
    crate::app::linux_webkit_compat::apply();

    let builder = tauri::Builder::default()
        .manage(DbInitState::default())
        .manage(GatewayState::default())
        .manage(resident::ResidentState::default())
        .manage(crate::app::heartbeat_watchdog::HeartbeatWatchdogState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init());

    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            resident::show_main_window(app);
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build());

    let app = builder
        .on_window_event(resident::on_window_event)
        .setup(|app| {
            crate::app::logging::init(app.handle());

            // Check for restart storm before installing the watchdog.
            // If the previous run wrote a restart marker less than RESTART_STORM_WINDOW ago,
            // show a native dialog and skip auto-recovery to break the cycle.
            if crate::app::heartbeat_watchdog::check_and_clear_restart_marker(app.handle()) {
                tracing::error!("startup: restart storm detected, auto-recovery disabled for this session");
                app.dialog().message(
                    "AIO Coding Hub 检测到 WebView 反复崩溃，已停止自动恢复。\n\n\
                     如果问题持续出现，请检查系统 WebView2 运行时是否正常。"
                ).title("WebView 恢复失败").blocking_show();
                // Still install the watchdog but it will not have a broken marker,
                // so normal page-level recovery will work if the WebView stabilizes.
            }

            crate::app::heartbeat_watchdog::install(app.handle());

            // Global panic hook: ensure any panic is written to disk logs for post-mortem diagnosis.
            // Note: payload is intentionally NOT logged to avoid leaking user data (consistent with blocking.rs).
            std::panic::set_hook(Box::new(|panic_info| {
                let location = panic_info
                    .location()
                    .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                    .unwrap_or_else(|| "unknown".to_string());
                tracing::error!(
                    location = %location,
                    "PANIC: application panicked at {location}. Check the log file for context leading up to this panic."
                );
            }));

            #[cfg(desktop)]
            {
                if let Err(err) = app
                    .handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())
                {
                    tracing::error!("updater initialization failed: {}", err);
                }

                if let Err(err) = resident::setup_tray(app.handle()) {
                    tracing::error!("system tray initialization failed: {}", err);
                }
            }

            #[cfg(debug_assertions)]
            {
                let enabled = std::env::var("AIO_CODING_HUB_DEV_DIAGNOSTICS")
                    .ok()
                    .map(|v| v.trim().to_ascii_lowercase())
                    .is_some_and(|v| v == "1" || v == "true" || v == "yes");
                if enabled {
                    let identifier = &app.config().identifier;
                    let product_name = app.config().product_name.as_deref().unwrap_or("<missing>");
                    tracing::info!(identifier = %identifier, "[dev] tauri identifier");
                    tracing::info!(product_name = %product_name, "[dev] productName");
                    if let Ok(dotdir_name) = std::env::var("AIO_CODING_HUB_DOTDIR_NAME") {
                        tracing::info!(dotdir_name = %dotdir_name, "[dev] AIO_CODING_HUB_DOTDIR_NAME");
                    }
                    if let Ok(dir) = app_paths::app_data_dir(app.handle()) {
                        tracing::info!(dir = %dir.display(), "[dev] app data dir");
                    }
                }
            }

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let db_state = app_handle.state::<DbInitState>();
                let db = match ensure_db_ready(app_handle.clone(), db_state.inner()).await {
                    Ok(db) => db,
                    Err(err) => {
                        tracing::error!("database initialization failed: {}", err);
                        return;
                    }
                };

                // M1: auto-start gateway on app launch (required for seamless CLI proxy experience).
                // Port conflicts are handled by the gateway's bind-first-available strategy.
                let settings = match blocking::run("startup_read_settings", {
                    let app_handle = app_handle.clone();
                    move || settings::read(&app_handle)
                })
                .await
                {
                    Ok(cfg) => cfg,
                    Err(err) => {
                        tracing::error!(
                            "startup settings read failed; skipping settings-dependent startup tasks: {}",
                            err
                        );
                        crate::app::cleanup::restore_cli_proxy_keep_state_best_effort(
                            &app_handle,
                            "startup_cli_proxy_restore_on_settings_read_failed",
                            "startup_settings_read_failed",
                            false,
                        )
                        .await;
                        resident::show_main_window(&app_handle);
                        return;
                    }
                };

                app_handle
                    .state::<resident::ResidentState>()
                    .set_tray_enabled(settings.tray_enabled);

                // Window starts hidden (visible:false in tauri.conf.json) to prevent flash.
                // Show it unless start_minimized is active.
                if settings.start_minimized {
                    resident::hide_main_window_on_startup(&app_handle);
                } else {
                    resident::show_main_window(&app_handle);
                }

                let preferred_port = settings.preferred_port;
                let enable_cli_proxy_startup_recovery = settings.enable_cli_proxy_startup_recovery;
                #[cfg(windows)]
                let gateway_listen_mode = settings.gateway_listen_mode;

                if enable_cli_proxy_startup_recovery {
                    match blocking::run("startup_cli_proxy_repair_incomplete_enable", {
                        let app_handle = app_handle.clone();
                        move || cli_proxy::startup_repair_incomplete_enable(&app_handle)
                    })
                    .await
                    {
                        Ok(results) => {
                            let mut repaired = Vec::new();
                            for result in results {
                                if result.ok {
                                    repaired.push(result.cli_key);
                                    continue;
                                }

                                tracing::warn!(
                                    cli_key = %result.cli_key,
                                    trace_id = %result.trace_id,
                                    error_code = %result.error_code.unwrap_or_default(),
                                    "startup recovery: cli_proxy enable state repair failed: {}",
                                    result.message
                                );
                            }

                            if !repaired.is_empty() {
                                tracing::info!(
                                    repaired = repaired.len(),
                                    cli_keys = ?repaired,
                                    "startup recovery: repaired cli_proxy enable state inconsistencies"
                                );
                            }
                        }
                        Err(err) => {
                            tracing::warn!("startup recovery: cli_proxy enable state repair task failed: {}", err);
                        }
                    }
                }

                let status = match blocking::run("startup_gateway_autostart", {
                    let app_handle = app_handle.clone();
                    let db = db.clone();
                    move || {
                        let state = app_handle.state::<GatewayState>();
                        let mut manager = state.0.lock_or_recover();
                        manager.start(&app_handle, db, Some(preferred_port))
                    }
                })
                .await
                {
                    Ok(status) => status,
                    Err(err) => {
                        tracing::error!("gateway auto-start failed: {}", err);
                        if enable_cli_proxy_startup_recovery {
                            crate::app::cleanup::restore_cli_proxy_keep_state_best_effort(
                                &app_handle,
                                "startup_cli_proxy_restore_keep_state",
                                "startup_recovery_gateway_failed",
                                true,
                            )
                            .await;
                        }
                        return;
                    }
                };

                crate::app::heartbeat_watchdog::gated_emit(&app_handle, crate::gateway::events::GATEWAY_STATUS_EVENT_NAME, status.clone());

                // Re-apply CLI proxy config to match the (potentially new) gateway address.
                // On clean exit, `restore_cli_proxy_keep_state` reverts config files to direct
                // mode while keeping `manifest.enabled = true`.  Without this sync the frontend
                // sees `applied_to_current_gateway = false` and shows a spurious "修复" button.
                if let Some(base_origin) = status.base_url.as_deref() {
                    let app_for_sync = app_handle.clone();
                    let base_origin = base_origin.to_string();
                    let _ = blocking::run("cli_proxy_sync_enabled_after_autostart", move || {
                        cli_proxy::sync_enabled(&app_for_sync, &base_origin, true)
                    })
                    .await;
                }

                // WSL auto-detect and auto-configure (Windows only, gated by wsl_auto_config)
                #[cfg(windows)]
                {
                    // Check for stale WSL manifests from a previous crash/unclean exit
                    let repair_app = app_handle.clone();
                    if let Err(err) =
                        blocking::run("startup_wsl_manifest_repair", move || {
                            infra::wsl::startup_repair_wsl_manifests(&repair_app)
                        })
                        .await
                    {
                        tracing::warn!("WSL manifest startup repair failed: {}", err);
                    }
                }

                #[cfg(windows)]
                if settings.wsl_auto_config {
                    let auto_cfg_app = app_handle.clone();
                    let auto_cfg_db = db.clone();
                    let auto_cfg_port = status.port;

                    tauri::async_runtime::spawn(async move {
                        if let Err(err) = commands::wsl::wsl_auto_configure_on_startup(
                            &auto_cfg_app,
                            auto_cfg_db,
                            gateway_listen_mode,
                            auto_cfg_port,
                        )
                        .await
                        {
                            tracing::warn!("WSL startup auto-configure failed: {}", err);
                        }
                    });
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ── settings ──
            settings_get,
            settings_set,
            settings_gateway_rectifier_set,
            settings_circuit_breaker_notice_set,
            settings_codex_session_id_completion_set,
            config_export,
            config_import,
            // ── app ──
            app_about_get,
            app_data_dir_get,
            app_exit,
            app_restart,
            app_heartbeat_pong,
            app_frontend_error_report,
            desktop_clipboard_write_text,
            desktop_dialog_open,
            desktop_dialog_save,
            desktop_notification_is_permission_granted,
            desktop_notification_notify,
            desktop_notification_request_permission,
            desktop_opener_open_path,
            desktop_opener_open_url,
            desktop_opener_reveal_item_in_dir,
            desktop_updater_check,
            desktop_updater_download_and_install,
            desktop_window_set_theme,
            // ── notice ──
            notice_send,
            // ── cli_manager ──
            cli_manager_claude_info_get,
            cli_manager_codex_info_get,
            cli_manager_codex_config_get,
            cli_manager_codex_config_set,
            cli_manager_codex_config_toml_get,
            cli_manager_codex_config_toml_validate,
            cli_manager_codex_config_toml_set,
            cli_manager_gemini_info_get,
            cli_manager_gemini_config_get,
            cli_manager_gemini_config_set,
            cli_manager_claude_env_set,
            cli_manager_claude_settings_get,
            cli_manager_claude_settings_set,
            cli_check_latest_version,
            cli_update,
            // ── gateway ──
            gateway_start,
            gateway_stop,
            gateway_status,
            gateway_check_port_available,
            gateway_sessions_list,
            gateway_circuit_status,
            gateway_circuit_reset_provider,
            gateway_circuit_reset_cli,
            gateway_upstream_proxy_validate,
            gateway_upstream_proxy_test,
            gateway_upstream_proxy_detect_ip,
            // ── wsl ──
            wsl_detect,
            wsl_host_address_get,
            wsl_config_status_get,
            wsl_configure_clients,
            // ── cli_sessions ──
            cli_sessions_projects_list,
            cli_sessions_sessions_list,
            cli_sessions_messages_get,
            cli_sessions_session_delete,
            // ── providers ──
            providers_list,
            provider_upsert,
            provider_duplicate,
            provider_set_enabled,
            provider_delete,
            providers_reorder,
            provider_claude_terminal_launch_command,
            provider_copy_api_key_to_clipboard,
            base_url_ping_ms,
            provider_oauth_start_flow,
            provider_oauth_refresh,
            provider_oauth_disconnect,
            provider_oauth_status,
            provider_oauth_fetch_limits,
            // ── claude_model_validation ──
            claude_provider_validate_model,
            claude_validation_history_list,
            claude_validation_history_clear_provider,
            // ── sort_modes ──
            sort_modes_list,
            sort_mode_create,
            sort_mode_rename,
            sort_mode_delete,
            sort_mode_active_list,
            sort_mode_active_set,
            sort_mode_providers_list,
            sort_mode_providers_set_order,
            sort_mode_provider_set_enabled,
            // ── model_prices ──
            model_prices_list,
            model_price_upsert,
            model_prices_sync_basellm,
            model_price_aliases_get,
            model_price_aliases_set,
            // ── prompts ──
            prompts_list,
            prompts_default_sync_from_files,
            prompt_upsert,
            prompt_set_enabled,
            prompt_delete,
            // ── mcp ──
            mcp_servers_list,
            mcp_server_upsert,
            mcp_server_set_enabled,
            mcp_server_delete,
            mcp_parse_json,
            mcp_import_servers,
            mcp_import_from_workspace_cli,
            // ── skills ──
            skill_repos_list,
            skill_repo_upsert,
            skill_repo_delete,
            skills_installed_list,
            skills_discover_available,
            skill_install,
            skill_install_to_local,
            skill_set_enabled,
            skill_uninstall,
            skill_return_to_local,
            skills_local_list,
            skill_local_delete,
            skill_import_local,
            skills_import_local_batch,
            skills_paths_get,
            skill_check_updates,
            skill_update,
            // ── request_logs ──
            request_logs_list,
            request_logs_list_all,
            request_logs_list_after_id,
            request_logs_list_after_id_all,
            request_log_get,
            request_log_get_by_trace_id,
            request_attempt_logs_by_trace_id,
            cli_sessions_folder_lookup_by_ids,
            // ── data_management ──
            db_disk_usage_get,
            request_logs_clear_all,
            app_data_reset,
            // ── usage ──
            usage_summary,
            usage_summary_v2,
            usage_leaderboard_provider,
            usage_leaderboard_day,
            usage_leaderboard_v2,
            usage_hourly_series,
            usage_provider_cache_rate_trend_v1,
            // ── cost ──
            cost_summary_v1,
            cost_trend_v1,
            cost_breakdown_provider_v1,
            cost_breakdown_model_v1,
            cost_scatter_cli_provider_model_v1,
            cost_top_requests_v1,
            cost_backfill_missing_v1,
            // ── env_conflicts ──
            env_conflicts_check,
            // ── cli_proxy ──
            cli_proxy_status_all,
            cli_proxy_set_enabled,
            cli_proxy_sync_enabled,
            cli_proxy_rebind_codex_home,
            // ── provider_limit_usage ──
            provider_limit_usage_v1,
            // ── workspaces ──
            workspaces_list,
            workspace_create,
            workspace_rename,
            workspace_delete,
            workspace_preview,
            workspace_apply,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, code, .. } = &event {
            // Note: `prevent_exit` is ignored for restart requests.
            // For app_restart we run cleanup explicitly before requesting restart.
            if *code != Some(tauri::RESTART_EXIT_CODE) {
                app_handle.state::<resident::ResidentState>().begin_exit();
                api.prevent_exit();

                if EXIT_CLEANUP_SPAWNED.swap(true, Ordering::SeqCst) {
                    return;
                }

                tracing::info!("exit requested, starting cleanup...");
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    crate::app::cleanup::cleanup_before_exit(&app_handle).await;
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    std::process::exit(0);
                });
            }
        }

        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen {
            has_visible_windows,
            ..
        } = event
        {
            if !has_visible_windows {
                resident::show_main_window(app_handle);
            }
        }
    });
}

/// 导出前端使用的 TypeScript IPC 绑定。
pub fn export_typescript_bindings(output_path: &str) -> Result<(), String> {
    let builder =
        tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
            commands::settings::settings_get,
            commands::settings::settings_set,
            commands::settings::settings_gateway_rectifier_set,
            commands::settings::settings_circuit_breaker_notice_set,
            commands::settings::settings_codex_session_id_completion_set,
            commands::config_migrate::config_export,
            commands::config_migrate::config_import,
            commands::cli_update::cli_check_latest_version,
            commands::cli_update::cli_update,
            commands::desktop::desktop_clipboard_write_text,
            commands::desktop::desktop_dialog_open,
            commands::desktop::desktop_dialog_save,
            commands::desktop::desktop_window_set_theme,
            commands::desktop::desktop_notification_is_permission_granted,
            commands::desktop::desktop_notification_request_permission,
            commands::desktop::desktop_notification_notify,
            commands::desktop::desktop_opener_open_path,
            commands::desktop::desktop_opener_open_url,
            commands::desktop::desktop_opener_reveal_item_in_dir,
            commands::desktop::desktop_updater_check,
            commands::app::app_about_get,
            commands::app::app_exit,
            commands::app::app_restart,
            commands::app::app_heartbeat_pong,
            commands::app::app_frontend_error_report,
            commands::data_management::app_data_dir_get,
            commands::data_management::db_disk_usage_get,
            commands::data_management::request_logs_clear_all,
            commands::data_management::app_data_reset,
            commands::gateway::gateway_status,
            commands::gateway::gateway_check_port_available,
            commands::gateway::gateway_sessions_list,
            commands::gateway::gateway_circuit_status,
            commands::gateway::gateway_circuit_reset_provider,
            commands::gateway::gateway_circuit_reset_cli,
            commands::gateway::gateway_start,
            commands::gateway::gateway_stop,
            commands::gateway::gateway_upstream_proxy_validate,
            commands::gateway::gateway_upstream_proxy_test,
            commands::gateway::gateway_upstream_proxy_detect_ip,
            commands::cli_proxy::cli_proxy_status_all,
            commands::cli_proxy::cli_proxy_set_enabled,
            commands::cli_proxy::cli_proxy_sync_enabled,
            commands::cli_proxy::cli_proxy_rebind_codex_home,
            commands::wsl::wsl_detect,
            commands::wsl::wsl_host_address_get,
            commands::wsl::wsl_config_status_get,
            commands::wsl::wsl_configure_clients,
            commands::providers::providers_list,
            commands::providers::provider_upsert,
            commands::providers::provider_duplicate,
            commands::providers::provider_set_enabled,
            commands::providers::provider_delete,
            commands::providers::providers_reorder,
            commands::providers::provider_claude_terminal_launch_command,
            commands::providers::provider_copy_api_key_to_clipboard,
            commands::providers::base_url_ping_ms,
            commands::providers::provider_oauth_start_flow,
            commands::providers::provider_oauth_refresh,
            commands::providers::provider_oauth_disconnect,
            commands::providers::provider_oauth_status,
            commands::providers::provider_oauth_fetch_limits,
            commands::sort_modes::sort_modes_list,
            commands::sort_modes::sort_mode_create,
            commands::sort_modes::sort_mode_rename,
            commands::sort_modes::sort_mode_delete,
            commands::sort_modes::sort_mode_active_list,
            commands::sort_modes::sort_mode_active_set,
            commands::sort_modes::sort_mode_providers_list,
            commands::sort_modes::sort_mode_providers_set_order,
            commands::sort_modes::sort_mode_provider_set_enabled,
            commands::provider_limit_usage::provider_limit_usage_v1,
            commands::request_logs::request_logs_list,
            commands::request_logs::request_logs_list_all,
            commands::request_logs::request_logs_list_after_id,
            commands::request_logs::request_logs_list_after_id_all,
            commands::request_logs::request_log_get,
            commands::request_logs::request_log_get_by_trace_id,
            commands::request_logs::request_attempt_logs_by_trace_id,
            commands::model_prices::model_prices_list,
            commands::model_prices::model_prices_sync_basellm,
            commands::model_prices::model_price_aliases_get,
            commands::model_prices::model_price_aliases_set,
            commands::usage::usage_summary,
            commands::usage::usage_summary_v2,
            commands::usage::usage_leaderboard_provider,
            commands::usage::usage_leaderboard_day,
            commands::usage::usage_leaderboard_v2,
            commands::usage::usage_hourly_series,
            commands::usage::usage_provider_cache_rate_trend_v1,
            commands::cost::cost_summary_v1,
            commands::cost::cost_trend_v1,
            commands::cost::cost_breakdown_provider_v1,
            commands::cost::cost_breakdown_model_v1,
            commands::cost::cost_scatter_cli_provider_model_v1,
            commands::cost::cost_top_requests_v1,
            commands::cost::cost_backfill_missing_v1,
            commands::prompts::prompts_list,
            commands::prompts::prompts_default_sync_from_files,
            commands::prompts::prompt_upsert,
            commands::prompts::prompt_set_enabled,
            commands::prompts::prompt_delete,
            commands::workspaces::workspaces_list,
            commands::workspaces::workspace_create,
            commands::workspaces::workspace_rename,
            commands::workspaces::workspace_delete,
            commands::workspaces::workspace_preview,
            commands::workspaces::workspace_apply,
            commands::skills::skill_repos_list,
            commands::skills::skill_repo_upsert,
            commands::skills::skill_repo_delete,
            commands::skills::skills_installed_list,
            commands::skills::skills_discover_available,
            commands::skills::skill_install,
            commands::skills::skill_install_to_local,
            commands::skills::skill_set_enabled,
            commands::skills::skill_uninstall,
            commands::skills::skill_return_to_local,
            commands::skills::skills_local_list,
            commands::skills::skill_local_delete,
            commands::skills::skill_import_local,
            commands::skills::skills_import_local_batch,
            commands::skills::skills_paths_get,
            commands::skills::skill_check_updates,
            commands::skills::skill_update,
            commands::mcp::mcp_servers_list,
            commands::mcp::mcp_server_upsert,
            commands::mcp::mcp_server_set_enabled,
            commands::mcp::mcp_server_delete,
            commands::mcp::mcp_parse_json,
            commands::mcp::mcp_import_servers,
            commands::mcp::mcp_import_from_workspace_cli,
            commands::cli_manager::cli_manager_claude_info_get,
            commands::cli_manager::cli_manager_codex_info_get,
            commands::cli_manager::cli_manager_codex_config_get,
            commands::cli_manager::cli_manager_codex_config_set,
            commands::cli_manager::cli_manager_codex_config_toml_get,
            commands::cli_manager::cli_manager_codex_config_toml_validate,
            commands::cli_manager::cli_manager_codex_config_toml_set,
            commands::cli_manager::cli_manager_gemini_info_get,
            commands::cli_manager::cli_manager_gemini_config_get,
            commands::cli_manager::cli_manager_gemini_config_set,
            commands::cli_manager::cli_manager_claude_env_set,
            commands::cli_manager::cli_manager_claude_settings_get,
            commands::cli_manager::cli_manager_claude_settings_set,
            commands::cli_sessions::cli_sessions_projects_list,
            commands::cli_sessions::cli_sessions_sessions_list,
            commands::cli_sessions::cli_sessions_messages_get,
            commands::cli_sessions::cli_sessions_session_delete,
            commands::cli_sessions::cli_sessions_folder_lookup_by_ids,
            commands::claude_model_validation::claude_provider_validate_model,
            commands::claude_model_validation::claude_validation_history_list,
            commands::claude_model_validation::claude_validation_history_clear_provider,
            commands::env_conflicts::env_conflicts_check,
            commands::notice::notice_send
        ]);

    builder
        .export(
            specta_typescript::Typescript::default()
                .header(
                    "/* eslint-disable */
// @ts-nocheck
// NOTE: Generated IPC contract for settings, config migration, desktop, app management, gateway, request-log, CLI update, CLI proxy, provider, WSL, sort-mode, provider-limit, usage, cost, model-price, prompt, workspace, skills, MCP, CLI manager, CLI sessions, Claude validation, notice, and env-conflict command families.",
                )
                .bigint(specta_typescript::BigIntExportBehavior::Number),
            output_path,
        )
        .map_err(|error| format!("failed to export specta TypeScript bindings: {error}"))
}

/// Specta type export smoke test.
///
/// 仅用于手动重新导出前端 bindings：
/// `cargo test export_bindings -- --ignored`
#[cfg(test)]
#[test]
#[ignore = "run manually: cargo test export_bindings -- --ignored"]
fn export_bindings() {
    export_typescript_bindings("../src/generated/bindings.ts")
        .expect("failed to export specta TypeScript bindings");
}
