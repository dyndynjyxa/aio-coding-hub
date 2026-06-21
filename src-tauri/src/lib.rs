mod app;
mod commands;
mod domain;
mod gateway;
mod infra;
mod shared;
pub mod test_support;

pub(crate) use app::{app_state, gateway_control, gateway_runtime_access, notice, resident};
pub(crate) use domain::{
    claude_model_validation, claude_model_validation_history, claude_plugins, cli_sessions, cost,
    cost_stats, mcp, plugins, prompts, provider_limit_usage, providers, skills, sort_modes, usage,
    usage_stats, workspace_switch, workspaces,
};
pub(crate) use gateway::session_manager;
pub(crate) use infra::{
    app_paths, base_url_probe, claude_hooks, claude_settings, cli_manager, cli_proxy, cli_update,
    codex_config, codex_paths, data_management, db, env_conflicts, gemini_config, mcp_sync,
    model_price_aliases, model_prices, model_prices_sync, prompt_sync, provider_circuit_breakers,
    request_attempt_logs, request_logs, settings, wsl,
};
pub(crate) use shared::{blocking, circuit_breaker};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Must run before Tauri initialises WebKitGTK to prevent EGL display
    // creation failure on Wayland (AppImage bundled-lib conflict, issue #93).
    crate::app::linux_webkit_compat::apply();

    let app = commands::registry::register_runtime_commands(
        crate::app::plugin_registry::create_builder(),
    )
    .on_window_event(resident::on_window_event)
    .setup(crate::app::bootstrap::setup)
    .build(tauri::generate_context!())
    .expect("error while building tauri application");

    app.run(crate::app::lifecycle::handle_run_event);
}

/// 导出前端使用的 TypeScript IPC 绑定。
pub fn export_typescript_bindings(output_path: &str) -> Result<(), String> {
    commands::registry::export_typescript_bindings(output_path)
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
