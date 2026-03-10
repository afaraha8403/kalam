fn main() {
    // Embed Windows .exe icon (Explorer, taskbar) from icons/icon.ico.
    // Window title/taskbar at runtime are set in lib.rs via set_icon().
    let attrs = tauri_build::Attributes::new().windows_attributes(
        tauri_build::WindowsAttributes::new().window_icon_path("icons/icon.ico"),
    );
    tauri_build::try_build(attrs).expect("failed to run tauri-build");
}
