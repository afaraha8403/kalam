#![allow(dead_code)]

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager};

/// Icon for system tray (32x32; 64x64 not present in repo).
const TRAY_ICON: tauri::image::Image<'static> = tauri::include_image!("icons/32x32.png");

pub const TRAY_ID: &str = "main";

pub struct TrayManager;

impl TrayManager {
    pub fn setup(app: &mut App) -> anyhow::Result<()> {
        // Create menu items
        let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
        let history_i = MenuItem::with_id(app, "history", "History", true, None::<&str>)?;
        let dictionary_i = MenuItem::with_id(app, "dictionary", "Dictionary", true, None::<&str>)?;
        let snippets_i = MenuItem::with_id(app, "snippets", "Snippets", true, None::<&str>)?;
        let separator = PredefinedMenuItem::separator(app)?;
        let check_updates = MenuItem::with_id(
            app,
            "check_updates",
            "Check for Updates",
            true,
            None::<&str>,
        )?;
        let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

        let menu = Menu::with_items(
            app,
            &[
                &settings_i,
                &history_i,
                &dictionary_i,
                &snippets_i,
                &separator,
                &check_updates,
                &quit_i,
            ],
        )?;

        let app_handle = app.handle().clone();
        TrayIconBuilder::with_id(TRAY_ID)
            .icon(TRAY_ICON.clone())
            .tooltip("Kalam - Ready")
            .menu(&menu)
            .show_menu_on_left_click(false)
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
                "dictionary" => {
                    if let Err(e) = show_window(app, "dictionary") {
                        log::error!("Failed to show dictionary: {}", e);
                    }
                }
                "snippets" => {
                    if let Err(e) = show_window(app, "snippets") {
                        log::error!("Failed to show snippets: {}", e);
                    }
                }
                "check_updates" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = crate::run_update_check_user_initiated(&app_handle).await {
                            log::warn!("Update check failed: {}", e);
                        }
                    });
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            })
            .on_tray_icon_event(move |_tray, event| {
                if let TrayIconEvent::Click { button, .. } = event {
                    if button == tauri::tray::MouseButton::Left {
                        if let Err(e) = show_window(&app_handle, "main") {
                            log::error!("Failed to show main window: {}", e);
                        }
                    }
                }
            })
            .build(app)?;

        Ok(())
    }

    /// Update tray tooltip (and optionally icon) based on audio state.
    pub fn set_tray_state(app: &AppHandle, state: crate::audio::AudioState) -> anyhow::Result<()> {
        let tray = app
            .tray_by_id(TRAY_ID)
            .ok_or_else(|| anyhow::anyhow!("Tray not found"))?;

        let (tooltip, _icon_path): (&str, Option<&str>) = match state {
            crate::audio::AudioState::Idle => ("Kalam - Ready", None),
            crate::audio::AudioState::Starting => ("Kalam - Starting...", None),
            crate::audio::AudioState::Recording => ("Kalam - Recording...", None),
            crate::audio::AudioState::Processing => ("Kalam - Processing...", None),
        };

        tray.set_tooltip(Some(tooltip))?;
        // When icons/tray-recording.png and icons/tray-processing.png exist, set_icon here
        Ok(())
    }
}

fn show_window(app: &AppHandle, page: &str) -> anyhow::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.set_focus()?;
        // Tell frontend to navigate when opening from tray menu (Settings, History, Dictionary, Snippets)
        if page != "main" {
            let _ = app.emit("tray-navigate", page);
        }
    }
    Ok(())
}
