use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use tauri::Manager;

pub fn play_sound(app_handle: &tauri::AppHandle, sound_name: &str) -> anyhow::Result<()> {
    let sound_path = app_handle
        .path()
        .resolve(
            format!("sounds/{}.wav", sound_name),
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|e| anyhow::anyhow!("Failed to resolve sound path: {}", e))?;

    // Read and play off the hot path so start_dictation isn't blocked by fs or audio init.
    let path = sound_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let sound_data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                log::error!("Failed to read sound file {:?}: {}", path, e);
                return;
            }
        };
        std::thread::spawn(move || {
            if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
                if let Ok(sink) = Sink::try_new(&stream_handle) {
                    if let Ok(source) = Decoder::new(Cursor::new(sound_data)) {
                        sink.append(source);
                        sink.sleep_until_end();
                    }
                }
            }
        });
    });

    Ok(())
}
