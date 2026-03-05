fn main() {
    // Build without Windows resources to avoid icon issues
    // For production builds, you need a valid icon.ico file
    tauri_build::try_build(tauri_build::Attributes::new()).expect("failed to run tauri-build");
}
