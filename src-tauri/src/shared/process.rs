//! Process lifecycle helpers shared by short-lived CLI integrations.

#[cfg(unix)]
pub(crate) fn configure_unix_process_group(command: &mut std::process::Command) {
    use std::io;
    use std::os::unix::process::CommandExt;

    unsafe {
        command.pre_exec(|| {
            if setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(unix)]
pub(crate) fn terminate_unix_process_group(process_id: u32) {
    let Ok(process_id) = i32::try_from(process_id) else {
        return;
    };
    unsafe {
        const SIGTERM: i32 = 15;
        const SIGKILL: i32 = 9;
        let _ = kill(-process_id, SIGTERM);
        std::thread::sleep(std::time::Duration::from_millis(30));
        let _ = kill(-process_id, SIGKILL);
    }
}

#[cfg(unix)]
extern "C" {
    fn setsid() -> i32;
    fn kill(pid: i32, signal: i32) -> i32;
}

#[cfg(windows)]
pub(crate) fn terminate_windows_process_tree(process_id: u32) {
    use std::os::windows::process::CommandExt;
    use std::path::Path;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let taskkill_path = std::env::var_os("SystemRoot")
        .map(|root| Path::new(&root).join("System32").join("taskkill.exe"))
        .filter(|path| path.is_file());
    let mut command = match taskkill_path {
        Some(path) => Command::new(path),
        None => Command::new("taskkill"),
    };
    command
        .args(["/PID", &process_id.to_string(), "/T", "/F"])
        .creation_flags(CREATE_NO_WINDOW);
    let _ = command.status();
}
