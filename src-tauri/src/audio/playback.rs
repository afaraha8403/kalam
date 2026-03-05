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

    let sound_data = std::fs::read(&sound_path)
        .map_err(|e| anyhow::anyhow!("Failed to read sound file {:?}: {}", sound_path, e))?;

    std::thread::spawn(move || match OutputStream::try_default() {
        Ok((_stream, stream_handle)) => match Sink::try_new(&stream_handle) {
            Ok(sink) => match Decoder::new(Cursor::new(sound_data)) {
                Ok(source) => {
                    sink.append(source);
                    sink.sleep_until_end();
                }
                Err(e) => log::error!("Failed to decode sound data: {}", e),
            },
            Err(e) => log::error!("Failed to create audio sink: {}", e),
        },
        Err(e) => log::error!("Failed to get default output stream: {}", e),
    });

    Ok(())
}
