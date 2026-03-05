#![allow(dead_code)]

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager};

pub struct TrayManager;

impl TrayManager {
    pub fn setup(app: &mut App) -> anyhow::Result<()> {
        // Create menu items
        let settings_i = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
        let history_i = MenuItem::with_id(app, "history", "History...", true, None::<&str>)?;
        let snippets_i = MenuItem::with_id(app, "snippets", "Snippets...", true, None::<&str>)?;
        let separator = MenuItem::with_id(app, "separator", "---", false, None::<&str>)?;
        let check_updates = MenuItem::with_id(
            app,
            "check_updates",
            "Check for Updates",
            true,
            None::<&str>,
        )?;
        let about_i = MenuItem::with_id(app, "about", "About", true, None::<&str>)?;
        let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

        let menu = Menu::with_items(
            app,
            &[
                &settings_i,
                &history_i,
                &snippets_i,
                &separator,
                &check_updates,
                &about_i,
                &quit_i,
            ],
        )?;

        TrayIconBuilder::new()
            .icon(app.default_window_icon().unwrap().clone())
            .menu(&menu)
            .on_menu_event(|app, event| match event.id.as_ref() {
                "settings" => {
                    if let Err(e) = show_window(app, "settings") {
                        log::error!("Failed to show settings: {}", e);
                    }
                }
                "history" => {
                    if let Err(e) = show_window(app, "history") {
                        log::error!("Failed to show history: {}", e);
                    }
                }
                "snippets" => {
                    if let Err(e) = show_window(app, "snippets") {
                        log::error!("Failed to show snippets: {}", e);
                    }
                }
                "check_updates" => {
                    // Trigger update check
                }
                "about" => {
                    // Show about dialog
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            })
            .on_tray_icon_event(|_tray, event| {
                if let TrayIconEvent::Click { button, .. } = event {
                    if button == tauri::tray::MouseButton::Left {
                        // Start recording on left click
                    }
                }
            })
            .build(app)?;

        Ok(())
    }

    pub fn set_recording_state(app: &AppHandle, recording: bool) -> anyhow::Result<()> {
        // Update tray icon based on recording state
        // In production, switch between different icon files
        let _ = recording;
        let _ = app;
        Ok(())
    }
}

fn show_window(app: &AppHandle, _page: &str) -> anyhow::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.set_focus()?;
        // Navigate to specific page in Svelte router
    }
    Ok(())
}
