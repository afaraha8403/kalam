// Release builds on Windows: use GUI subsystem so no console window appears.
// Debug builds keep console for development logging.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use kalam_voice::{app_log, run};

fn main() {
    app_log::init(Default::default());
    run();
}
