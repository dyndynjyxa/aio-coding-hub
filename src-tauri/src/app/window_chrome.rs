//! Usage: Platform-specific native window chrome tweaks.

pub(crate) fn apply_main_window_chrome(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    if let Err(err) = hide_windows_titlebar_icon(window) {
        tracing::warn!(error = %err, "failed to hide Windows titlebar icon");
    }

    #[cfg(not(windows))]
    let _ = window;
}

#[cfg(windows)]
fn hide_windows_titlebar_icon(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SendMessageW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, ICON_SMALL,
        ICON_SMALL2, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
        WM_SETICON, WS_EX_DLGMODALFRAME,
    };

    let hwnd = window.hwnd()?;
    let hwnd = hwnd.0 as windows_sys::Win32::Foundation::HWND;

    unsafe {
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, 0);
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL2 as usize, 0);

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_DLGMODALFRAME as isize);
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }

    Ok(())
}
