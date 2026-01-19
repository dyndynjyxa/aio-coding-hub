mod app;
mod commands;
mod domain;
mod gateway;
mod infra;
mod shared;

pub(crate) use app::{app_state, notice, resident};
pub(crate) use domain::{
    claude_model_validation, claude_model_validation_history, cost, cost_stats, mcp, prompts,
    providers, skills, sort_modes, usage, usage_stats,
};
pub(crate) use gateway::session_manager;
pub(crate) use infra::{
    app_paths, base_url_probe, cli_manager, cli_proxy, data_management, db, mcp_sync,
    model_price_aliases, model_prices, model_prices_sync, prompt_sync, provider_circuit_breakers,
    request_attempt_logs, request_logs, settings, wsl,
};
pub(crate) use shared::{blocking, circuit_breaker};

use app_state::{ensure_db_ready, DbInitState, GatewayState};
use commands::*;
use tauri::Emitter;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .manage(DbInitState::default())
        .manage(GatewayState::default())
        .manage(resident::ResidentState::default())
        .plugin(tauri_plugin_opener::init());

    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            resident::show_main_window(app);
        }));

    let app = builder
        .on_window_event(resident::on_window_event)
        .setup(|app| {
            #[cfg(desktop)]
            {
                if let Err(err) = app
                    .handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())
                {
                    eprintln!("updater init error: {err}");
                }

                if let Err(err) = resident::setup_tray(app.handle()) {
                    eprintln!("tray init error: {err}");
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
                    eprintln!("[dev] tauri identifier: {identifier}");
                    eprintln!("[dev] productName: {product_name}");
                    if let Ok(dotdir_name) = std::env::var("AIO_CODING_HUB_DOTDIR_NAME") {
                        eprintln!("[dev] AIO_CODING_HUB_DOTDIR_NAME: {}", dotdir_name);
                    }
                    if let Ok(dir) = app_paths::app_data_dir(app.handle()) {
                        eprintln!("[dev] app data dir: {}", dir.display());
                    }
                }
            }

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let db_state = app_handle.state::<DbInitState>();
                if let Err(err) = ensure_db_ready(app_handle.clone(), db_state.inner()).await {
                    eprintln!("db init error: {err}");
                    return;
                }

                // M1: auto-start gateway on app launch (required for seamless CLI proxy experience).
                // Port conflicts are handled by the gateway's bind-first-available strategy.
                let settings = match blocking::run("startup_read_settings", {
                    let app_handle = app_handle.clone();
                    move || Ok(settings::read(&app_handle).unwrap_or_default())
                })
                .await
                {
                    Ok(cfg) => cfg,
                    Err(err) => {
                        eprintln!("settings read error: {err}");
                        settings::AppSettings::default()
                    }
                };

                app_handle
                    .state::<resident::ResidentState>()
                    .set_tray_enabled(settings.tray_enabled);

                let status = match blocking::run("startup_gateway_autostart", {
                    let app_handle = app_handle.clone();
                    move || {
                        let state = app_handle.state::<GatewayState>();
                        let mut manager = state.0.lock().unwrap_or_else(|e| e.into_inner());
                        manager.start(&app_handle, Some(settings.preferred_port))
                    }
                })
                .await
                {
                    Ok(status) => status,
                    Err(err) => {
                        eprintln!("gateway auto-start error: {err}");
                        return;
                    }
                };

                let _ = app_handle.emit("gateway:status", status.clone());
                if let Some(base_origin) = status.base_url.as_deref() {
                    // Best-effort: if any CLI proxy is enabled, keep its config aligned with the actual gateway port.
                    let base_origin = base_origin.to_string();
                    let _ = blocking::run("startup_cli_proxy_sync_enabled", {
                        let app_handle = app_handle.clone();
                        move || cli_proxy::sync_enabled(&app_handle, &base_origin)
                    })
                    .await;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            settings_get,
            app_about_get,
            notice_send,
            settings_set,
            settings_gateway_rectifier_set,
            settings_circuit_breaker_notice_set,
            settings_codex_session_id_completion_set,
            cli_manager_claude_info_get,
            cli_manager_codex_info_get,
            cli_manager_gemini_info_get,
            cli_manager_claude_env_set,
            gateway_start,
            gateway_stop,
            gateway_status,
            gateway_check_port_available,
            wsl_detect,
            wsl_host_address_get,
            wsl_config_status_get,
            wsl_configure_clients,
            gateway_sessions_list,
            providers_list,
            provider_upsert,
            provider_set_enabled,
            provider_delete,
            providers_reorder,
            base_url_ping_ms,
            claude_provider_validate_model,
            claude_provider_get_api_key_plaintext,
            claude_validation_history_list,
            claude_validation_history_clear_provider,
            sort_modes_list,
            sort_mode_create,
            sort_mode_rename,
            sort_mode_delete,
            sort_mode_active_list,
            sort_mode_active_set,
            sort_mode_providers_list,
            sort_mode_providers_set_order,
            model_prices_list,
            model_price_upsert,
            model_prices_sync_basellm,
            model_price_aliases_get,
            model_price_aliases_set,
            prompts_list,
            prompts_default_sync_from_files,
            prompt_upsert,
            prompt_set_enabled,
            prompt_delete,
            mcp_servers_list,
            mcp_server_upsert,
            mcp_server_set_enabled,
            mcp_server_delete,
            mcp_parse_json,
            mcp_import_servers,
            skill_repos_list,
            skill_repo_upsert,
            skill_repo_delete,
            skills_installed_list,
            skills_discover_available,
            skill_install,
            skill_set_enabled,
            skill_uninstall,
            skills_local_list,
            skill_import_local,
            skills_paths_get,
            request_logs_list,
            request_logs_list_all,
            request_logs_list_after_id,
            request_logs_list_after_id_all,
            request_log_get,
            request_log_get_by_trace_id,
            request_attempt_logs_by_trace_id,
            app_data_dir_get,
            db_disk_usage_get,
            request_logs_clear_all,
            app_data_reset,
            app_exit,
            app_restart,
            gateway_circuit_status,
            gateway_circuit_reset_provider,
            gateway_circuit_reset_cli,
            usage_summary,
            usage_summary_v2,
            usage_leaderboard_provider,
            usage_leaderboard_day,
            usage_leaderboard_v2,
            usage_hourly_series,
            cost_summary_v1,
            cost_trend_v1,
            cost_breakdown_provider_v1,
            cost_breakdown_model_v1,
            cost_scatter_cli_provider_model_v1,
            cost_top_requests_v1,
            cost_backfill_missing_v1,
            cli_proxy_status_all,
            cli_proxy_set_enabled,
            cli_proxy_sync_enabled
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app_handle, _event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen {
            has_visible_windows,
            ..
        } = _event
        {
            if !has_visible_windows {
                resident::show_main_window(_app_handle);
            }
        }
    });
}
