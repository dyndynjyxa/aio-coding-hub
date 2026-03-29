//! Usage: Auto-start side-effect helpers shared by settings-related flows.

fn next_auto_start_with_sync(
    previous_auto_start: bool,
    desired_auto_start: bool,
    force_sync: bool,
    sync: impl FnOnce(bool) -> Result<(), String>,
) -> Result<bool, String> {
    if !force_sync && previous_auto_start == desired_auto_start {
        return Ok(desired_auto_start);
    }

    sync(desired_auto_start)?;
    Ok(desired_auto_start)
}

#[cfg(desktop)]
fn sync_auto_start<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    enable_auto_start: bool,
) -> Result<(), String> {
    use tauri::Manager;
    use tauri_plugin_autostart::ManagerExt;

    if app
        .try_state::<tauri_plugin_autostart::AutoLaunchManager>()
        .is_none()
    {
        tracing::debug!("auto-start plugin not initialized, skipping sync");
        return Ok(());
    }

    if enable_auto_start {
        app.autolaunch()
            .enable()
            .map_err(|e| format!("failed to enable autostart: {e}"))
    } else {
        app.autolaunch()
            .disable()
            .map_err(|e| format!("failed to disable autostart: {e}"))
    }
}

#[cfg(not(desktop))]
fn sync_auto_start<R: tauri::Runtime>(
    _app: &tauri::AppHandle<R>,
    _enable_auto_start: bool,
) -> Result<(), String> {
    Ok(())
}

pub(crate) fn reconcile_auto_start<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    previous_auto_start: bool,
    desired_auto_start: bool,
    force_sync: bool,
) -> bool {
    match next_auto_start_with_sync(
        previous_auto_start,
        desired_auto_start,
        force_sync,
        |enable_auto_start| sync_auto_start(app, enable_auto_start),
    ) {
        Ok(next_auto_start) => next_auto_start,
        Err(err) => {
            tracing::warn!("auto-start sync failed: {}", err);
            previous_auto_start
        }
    }
}

pub(crate) fn restore_auto_start_best_effort<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    auto_start: bool,
) {
    if let Err(err) = sync_auto_start(app, auto_start) {
        tracing::warn!("auto-start rollback failed: {}", err);
    }
}

#[cfg(test)]
mod tests {
    use super::next_auto_start_with_sync;

    #[test]
    fn next_auto_start_skips_sync_when_value_unchanged_and_not_forced() {
        let mut sync_called = false;
        let next_auto_start = next_auto_start_with_sync(true, true, false, |_| {
            sync_called = true;
            Ok(())
        })
        .expect("next auto start");

        assert!(next_auto_start);
        assert!(!sync_called);
    }

    #[test]
    fn next_auto_start_forces_sync_when_requested() {
        let mut sync_calls = Vec::new();
        let next_auto_start = next_auto_start_with_sync(true, true, true, |enable_auto_start| {
            sync_calls.push(enable_auto_start);
            Ok(())
        })
        .expect("next auto start");

        assert!(next_auto_start);
        assert_eq!(sync_calls, vec![true]);
    }

    #[test]
    fn next_auto_start_propagates_sync_error() {
        let err = next_auto_start_with_sync(false, true, true, |_| {
            Err("failed to enable autostart".to_string())
        })
        .expect_err("sync should fail");

        assert!(err.contains("failed to enable autostart"));
    }
}
