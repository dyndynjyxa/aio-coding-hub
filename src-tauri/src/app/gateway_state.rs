//! Usage: Gateway runtime state container and internal manager access helpers.

use crate::gateway::{manager::GatewayManager, runtime::GatewayRuntime};
use crate::shared::mutex_ext::MutexExt;
use std::sync::Mutex;
use tauri::Manager;

#[derive(Default)]
pub(crate) struct GatewayState {
    manager: Mutex<GatewayManager>,
}

fn with_gateway_manager<T, F>(state: &GatewayState, access: F) -> T
where
    F: FnOnce(&GatewayManager) -> T,
{
    let manager = state.manager.lock_or_recover();
    access(&manager)
}

fn with_gateway_manager_mut<T, F>(state: &GatewayState, access: F) -> T
where
    F: FnOnce(&mut GatewayManager) -> T,
{
    let mut manager = state.manager.lock_or_recover();
    access(&mut manager)
}

fn with_gateway_running<T, F>(state: &GatewayState, access: F) -> T
where
    F: FnOnce(Option<&GatewayRuntime>) -> T,
{
    with_gateway_manager(state, |manager| access(manager.running.as_ref()))
}

fn with_gateway_running_slot_mut<T, F>(state: &GatewayState, access: F) -> T
where
    F: FnOnce(&mut Option<GatewayRuntime>) -> T,
{
    with_gateway_manager_mut(state, |manager| access(&mut manager.running))
}

pub(super) fn with_app_running_gateway<R, T, F>(app: &tauri::AppHandle<R>, access: F) -> T
where
    R: tauri::Runtime,
    F: FnOnce(Option<&GatewayRuntime>) -> T,
{
    let state = app.state::<GatewayState>();
    with_gateway_running(state.inner(), access)
}

pub(super) fn with_app_running_gateway_slot_mut<R, T, F>(app: &tauri::AppHandle<R>, access: F) -> T
where
    R: tauri::Runtime,
    F: FnOnce(&mut Option<GatewayRuntime>) -> T,
{
    let state = app.state::<GatewayState>();
    with_gateway_running_slot_mut(state.inner(), access)
}

pub(super) fn take_app_running_gateway<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Option<crate::gateway::runtime::GatewayRuntimeHandles> {
    let state = app.state::<GatewayState>();
    with_gateway_manager_mut(state.inner(), GatewayManager::take_running)
}

pub(super) fn try_with_app_running_gateway<R, T, F>(
    app: &tauri::AppHandle<R>,
    access: F,
) -> Option<T>
where
    R: tauri::Runtime,
    F: FnOnce(Option<&GatewayRuntime>) -> T,
{
    app.try_state::<GatewayState>()
        .map(|state| with_gateway_running(state.inner(), access))
}
