//! Usage: Isolate native audio playback in the macOS system player process.

use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::shared::error::{AppError, AppResult};

use super::DING_MP3_BYTES;

const PLAYBACK_TIMEOUT: Duration = Duration::from_secs(10);
const PLAYER_POLL_INTERVAL: Duration = Duration::from_millis(25);

pub(super) fn play_embedded_sound_blocking() -> AppResult<()> {
    // cpal 0.16 device enumeration can segfault in optimized macOS builds.
    // Keep native audio outside the app process; Rust error handling cannot catch SIGSEGV.
    play_with_player(
        Command::new("/usr/bin/afplay"),
        &std::env::temp_dir(),
        PLAYBACK_TIMEOUT,
    )
}

fn play_with_player(mut command: Command, temp_dir: &Path, timeout: Duration) -> AppResult<()> {
    let mut sound = tempfile::Builder::new()
        .prefix("aio-notification-")
        .suffix(".mp3")
        .tempfile_in(temp_dir)
        .map_err(|error| AppError::new("NOTIFICATION_SOUND_TEMPFILE_FAILED", error.to_string()))?;
    sound
        .write_all(DING_MP3_BYTES)
        .map_err(|error| AppError::new("NOTIFICATION_SOUND_WRITE_FAILED", error.to_string()))?;

    let result = command
        .arg(sound.path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| AppError::new("NOTIFICATION_SOUND_PLAYER_SPAWN_FAILED", error.to_string()))
        .and_then(|mut child| wait_for_player(&mut child, timeout));

    if let Err(error) = sound.close() {
        tracing::warn!(error = %error, "notification sound temporary file cleanup failed");
    }
    result
}

fn wait_for_player(child: &mut Child, timeout: Duration) -> AppResult<()> {
    let started = Instant::now();
    let error = loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                return Err(AppError::new(
                    "NOTIFICATION_SOUND_PLAYER_FAILED",
                    status.to_string(),
                ));
            }
            Ok(None) if started.elapsed() >= timeout => {
                break AppError::new(
                    "NOTIFICATION_SOUND_PLAYER_TIMEOUT",
                    format!("player exceeded {}ms", timeout.as_millis()),
                );
            }
            Ok(None) => thread::sleep(PLAYER_POLL_INTERVAL),
            Err(error) => {
                break AppError::new("NOTIFICATION_SOUND_PLAYER_WAIT_FAILED", error.to_string());
            }
        }
    };

    // Child::drop does not terminate or reap the process.
    if let Err(error) = child.kill() {
        tracing::warn!(error = %error, "notification sound player termination failed");
    }
    if let Err(error) = child.wait() {
        tracing::warn!(error = %error, "notification sound player reap failed");
    }
    Err(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_empty(directory: &Path) {
        assert_eq!(std::fs::read_dir(directory).unwrap().count(), 0);
    }

    #[test]
    fn player_receives_exact_asset_and_temporary_file_is_removed() {
        let root = tempfile::tempdir().unwrap();
        let sounds = root.path().join("sound files with spaces");
        std::fs::create_dir(&sounds).unwrap();
        let expected = root.path().join("expected.mp3");
        std::fs::write(&expected, DING_MP3_BYTES).unwrap();
        let mut command = Command::new("/usr/bin/cmp");
        command.arg(&expected);

        play_with_player(command, &sounds, PLAYBACK_TIMEOUT).unwrap();

        assert_empty(&sounds);
    }

    #[test]
    fn spawn_failure_removes_temporary_file() {
        let root = tempfile::tempdir().unwrap();
        let error = play_with_player(
            Command::new(root.path().join("missing-player")),
            root.path(),
            PLAYBACK_TIMEOUT,
        )
        .unwrap_err();

        assert_eq!(error.code(), "NOTIFICATION_SOUND_PLAYER_SPAWN_FAILED");
        assert_empty(root.path());
    }

    #[test]
    fn unsuccessful_player_removes_temporary_file() {
        let root = tempfile::tempdir().unwrap();
        let error = play_with_player(
            Command::new("/usr/bin/false"),
            root.path(),
            PLAYBACK_TIMEOUT,
        )
        .unwrap_err();

        assert_eq!(error.code(), "NOTIFICATION_SOUND_PLAYER_FAILED");
        assert_empty(root.path());
    }

    #[test]
    fn signaled_player_does_not_terminate_parent_and_next_playback_succeeds() {
        let root = tempfile::tempdir().unwrap();
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "kill -TERM $$", "test-player"]);

        let error = play_with_player(command, root.path(), PLAYBACK_TIMEOUT).unwrap_err();

        assert_eq!(error.code(), "NOTIFICATION_SOUND_PLAYER_FAILED");
        assert!(error.to_string().contains("signal"));
        assert_empty(root.path());
        play_with_player(Command::new("/usr/bin/true"), root.path(), PLAYBACK_TIMEOUT).unwrap();
        assert_empty(root.path());
    }

    #[test]
    fn timed_out_player_is_killed_and_reaped() {
        let mut child = Command::new("/bin/sleep")
            .arg("60")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let started = Instant::now();

        let error = wait_for_player(&mut child, Duration::from_millis(50)).unwrap_err();

        assert_eq!(error.code(), "NOTIFICATION_SOUND_PLAYER_TIMEOUT");
        assert!(started.elapsed() < Duration::from_secs(5));
        let status = child.try_wait().unwrap().expect("player must be reaped");
        assert!(!status.success());
    }

    #[test]
    fn timeout_removes_temporary_file() {
        let root = tempfile::tempdir().unwrap();
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "exec /bin/sleep 60", "test-player"]);

        let error = play_with_player(command, root.path(), Duration::from_millis(50)).unwrap_err();

        assert_eq!(error.code(), "NOTIFICATION_SOUND_PLAYER_TIMEOUT");
        assert_empty(root.path());
    }

    #[test]
    fn temporary_file_failure_returns_an_error() {
        let root = tempfile::tempdir().unwrap();
        let error = play_with_player(
            Command::new("/usr/bin/true"),
            &root.path().join("missing-directory"),
            PLAYBACK_TIMEOUT,
        )
        .unwrap_err();

        assert_eq!(error.code(), "NOTIFICATION_SOUND_TEMPFILE_FAILED");
        assert_empty(root.path());
    }

    #[test]
    #[ignore = "plays an audible notification through the macOS system audio device"]
    fn macos_system_player_smoke() {
        play_embedded_sound_blocking().unwrap();
    }
}
