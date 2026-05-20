//! Usage: Native playback for the bundled notification sound.

use std::io::Cursor;
use std::thread;

const DING_MP3_BYTES: &[u8] = include_bytes!("../../../public/ding.mp3");

fn play_embedded_sound_blocking() -> Result<(), String> {
    let stream = rodio::OutputStreamBuilder::open_default_stream()
        .map_err(|error| format!("NOTIFICATION_SOUND_OUTPUT_UNAVAILABLE: {error}"))?;
    let sink = rodio::Sink::connect_new(stream.mixer());
    let source = rodio::Decoder::try_from(Cursor::new(DING_MP3_BYTES))
        .map_err(|error| format!("NOTIFICATION_SOUND_DECODE_FAILED: {error}"))?;

    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}

pub(crate) fn play_notification_sound() -> Result<(), String> {
    thread::Builder::new()
        .name("notification-sound".to_string())
        .spawn(|| {
            if let Err(error) = play_embedded_sound_blocking() {
                tracing::warn!(error = %error, "native notification sound playback failed");
            }
        })
        .map(|_| ())
        .map_err(|error| format!("NOTIFICATION_SOUND_THREAD_SPAWN_FAILED: {error}"))
}

#[cfg(test)]
mod tests {
    use super::DING_MP3_BYTES;

    #[test]
    fn embeds_non_empty_notification_sound_asset() {
        assert!(DING_MP3_BYTES.len() > 1_024);
        let has_id3_tag = DING_MP3_BYTES.starts_with(b"ID3");
        let has_frame_sync = DING_MP3_BYTES[0] == 0xff && (DING_MP3_BYTES[1] & 0xe0) == 0xe0;
        assert!(
            has_id3_tag || has_frame_sync,
            "notification sound asset should look like an MP3"
        );
    }
}
