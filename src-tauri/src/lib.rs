pub mod app_log;
mod audio;
mod commands;
mod config;
mod db;
mod formatting;
mod history;
mod hotkey;
#[cfg(windows)]
mod hotkey_win;
mod injection;
mod models;
mod notifications;
mod stt;
mod system_reqs;
mod tray;

use chrono::TimeZone;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::{oneshot, Mutex};

/// App icon for window and taskbar (used in setup so dev and production show the same icon).
const WINDOW_ICON: tauri::image::Image<'static> = tauri::include_image!("icons/32x32.png");

use crate::audio::vad::VADConfig;
use crate::audio::{play_sound, AudioState};
use crate::config::STTConfig;
use crate::config::{AppConfig, ConfigManager};
use crate::hotkey::{parse_rdev_hotkey, start_listener, HotkeyRegistration};
use crate::notifications::NotificationManager;
use crate::tray::TrayManager;

/// Parse ISO 8601 date-time or date-only string into Utc. Date-only gets 09:00:00 UTC.
fn parse_iso_datetime(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&chrono::Utc));
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let naive = d
            .and_hms_opt(9, 0, 0)
            .unwrap_or_else(|| d.and_hms_opt(0, 0, 0).unwrap());
        return Some(chrono::Utc.from_utc_datetime(&naive));
    }
    None
}

fn language_display_name(code: &str) -> String {
    let s: &'static str = match code {
        "auto" => "Auto-detect",
        "en" => "English",
        "es" => "Spanish",
        "fr" => "French",
        "de" => "German",
        "zh" => "Chinese",
        "yue" => "Cantonese",
        "ja" => "Japanese",
        "ko" => "Korean",
        "pt" => "Portuguese",
        "it" => "Italian",
        "ru" => "Russian",
        "ar" => "Arabic",
        "hi" => "Hindi",
        "nl" => "Dutch",
        "pl" => "Polish",
        "tr" => "Turkish",
        "sv" => "Swedish",
        "id" => "Indonesian",
        "th" => "Thai",
        _ => return code.to_string(),
    };
    s.to_string()
}

/// Which hotkey started the current (or last) recording: dictates whether we inject text or run command pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingType {
    Dictation,
    Command,
}

pub struct AppState {
    pub config: Arc<Mutex<ConfigManager>>,
    pub notification_manager: Arc<NotificationManager>,
    pub audio_state: Arc<Mutex<AudioState>>,
    pub app_handle: tauri::AppHandle,
    pub is_recording: Arc<AtomicBool>,
    pub audio_capture: Arc<Mutex<crate::audio::capture::AudioCapture>>,
    pub last_injected_len: Arc<AtomicUsize>,
    pub last_injected_text: Arc<Mutex<String>>,
    /// On Windows: HWND of foreground window at recording start; restore before injection so text goes to the right app.
    pub foreground_for_injection: Arc<Mutex<Option<usize>>>,
    pub press_start_time: Arc<Mutex<Option<std::time::Instant>>>,
    pub local_model_manager: Arc<crate::stt::lifecycle::LocalModelManager>,
    /// Set when starting recording: Dictation (main hotkey) or Command (command hotkey). Read in stop_dictation to decide inject vs command pipeline.
    pub recording_type: Arc<Mutex<RecordingType>>,
}

impl AppState {
    pub fn new(app_handle: tauri::AppHandle) -> anyhow::Result<Self> {
        let _ = history::migrate_from_json_if_needed();
        let config = Arc::new(Mutex::new(ConfigManager::new()?));
        if let Ok(true) = crate::db::migrate_snippets_from_config(
            config.blocking_lock().get_all().snippets.as_slice(),
        ) {
            let mut c = config.blocking_lock();
            let mut cfg = c.get_all().clone();
            cfg.snippets.clear();
            let _ = c.save(cfg);
        }
        let notification_manager = Arc::new(NotificationManager::new(app_handle.clone()));
        let audio_state = Arc::new(Mutex::new(AudioState::Idle));
        let is_recording = Arc::new(AtomicBool::new(false));
        let vad_config = config.blocking_lock().get_all().stt_config.vad_config();
        let mut audio_capture = crate::audio::capture::AudioCapture::new(vad_config)?;

        // Apply saved audio device preference
        let saved_device = {
            let cfg = config.blocking_lock();
            cfg.get_all().audio_device.clone()
        };

        if let Some(ref device_id) = saved_device {
            if let Err(e) = audio_capture.set_device(device_id) {
                log::warn!("Failed to set saved audio device '{}': {}", device_id, e);
            } else {
                log::info!("Restored audio device: {}", device_id);
            }
        }

        let audio_capture = Arc::new(Mutex::new(audio_capture));
        let last_injected_len = Arc::new(AtomicUsize::new(0));
        let last_injected_text = Arc::new(Mutex::new(String::new()));
        let local_model_manager = Arc::new(crate::stt::lifecycle::LocalModelManager::new(
            app_handle.clone(),
        ));

        Ok(Self {
            config,
            notification_manager,
            audio_state,
            app_handle,
            is_recording,
            audio_capture,
            last_injected_len,
            last_injected_text,
            foreground_for_injection: Arc::new(Mutex::new(None)),
            press_start_time: Arc::new(Mutex::new(None)),
            local_model_manager,
            recording_type: Arc::new(Mutex::new(RecordingType::Dictation)),
        })
    }
}

fn create_registrations(
    app_handle: &tauri::AppHandle,
    is_recording_flag: Arc<AtomicBool>,
    rt_handle: tokio::runtime::Handle,
    hold_hotkey_str: &Option<String>,
    toggle_hotkey_str: Option<String>,
    language_toggle_hotkey: Option<String>,
    command_hotkey_str: Option<String>,
) -> Vec<HotkeyRegistration> {
    let mut registrations: Vec<HotkeyRegistration> = Vec::new();

    if let Some(hotkey_str) = hold_hotkey_str {
        if !hotkey_str.trim().is_empty() {
            if let Ok(target_hotkey) = parse_rdev_hotkey(hotkey_str) {
                let app_handle_press = app_handle.clone();
                let is_recording_press = is_recording_flag.clone();
                let rt_for_press = rt_handle.clone();
                let app_handle_release = app_handle.clone();
                let is_recording_release = is_recording_flag.clone();
                let rt_for_release = rt_handle.clone();

                registrations.push(HotkeyRegistration {
                    target: target_hotkey,
                    active: Arc::new(AtomicBool::new(false)),
                    on_press: Arc::new(move || {
                        log::info!("Hold hotkey pressed - callback invoked");
                        let app_handle = app_handle_press.clone();
                        let is_recording = is_recording_press.clone();
                        let rt = rt_for_press.clone();
                        rt.spawn(async move {
                            let state = app_handle.state::<AppState>();
                            let dictation_enabled = {
                                let config = state.config.lock().await;
                                config.get_all().dictation_enabled
                            };
                            if !dictation_enabled {
                                return;
                            }
                            *state.recording_type.lock().await = RecordingType::Dictation;
                            *state.press_start_time.lock().await = Some(std::time::Instant::now());
                            start_dictation(state, is_recording).await;
                        });
                    }),
                    on_release: Arc::new(move || {
                        let app_handle = app_handle_release.clone();
                        let is_recording = is_recording_release.clone();
                        let rt = rt_for_release.clone();
                        rt.spawn(async move {
                            let state = app_handle.state::<AppState>();
                            let dictation_enabled = {
                                let config = state.config.lock().await;
                                config.get_all().dictation_enabled
                            };
                            if !dictation_enabled {
                                return;
                            }
                            let min_hold_ms = {
                                let config = state.config.lock().await;
                                config.get_all().min_hold_ms
                            };
                            let mut is_short_press = false;
                            if let Some(start_time) = state.press_start_time.lock().await.take() {
                                if start_time.elapsed().as_millis() < min_hold_ms as u128 {
                                    is_short_press = true;
                                }
                            }
                            if is_short_press {
                                log::info!("Short press detected, cancelling dictation");
                                let mut audio_state = state.audio_state.lock().await;
                                if matches!(*audio_state, AudioState::Recording) {
                                    *audio_state = AudioState::Idle;
                                    is_recording.store(false, Ordering::SeqCst);
                                    let _ = state.audio_capture.lock().await.stop_recording().await;
                                    emit_overlay_event(&state.app_handle, OverlayEvent::ShortPress);
                                    let app_for_overlay = state.app_handle.clone();
                                    tauri::async_runtime::spawn(async move {
                                        tokio::time::sleep(tokio::time::Duration::from_millis(
                                            1500,
                                        ))
                                        .await;
                                        reset_overlay_state(&app_for_overlay);
                                    });
                                }
                            } else {
                                stop_dictation(state, is_recording).await;
                            }
                        });
                    }),
                });
                log::info!("Hold dictation hotkey registered: {}", hotkey_str);
            } else {
                log::error!("Failed to parse hold hotkey: {}", hotkey_str);
            }
        }
    }

    if let Some(ref toggle_str) = toggle_hotkey_str {
        if !toggle_str.trim().is_empty() {
            if let Ok(toggle_hotkey) = parse_rdev_hotkey(toggle_str) {
                let app_handle_press = app_handle.clone();
                let is_recording_press = is_recording_flag.clone();
                let rt_for_press = rt_handle.clone();

                registrations.push(HotkeyRegistration {
                    target: toggle_hotkey,
                    active: Arc::new(AtomicBool::new(false)),
                    on_press: Arc::new(move || {
                        log::info!("Toggle hotkey pressed - callback invoked");
                        let app_handle = app_handle_press.clone();
                        let is_recording = is_recording_press.clone();
                        let rt = rt_for_press.clone();
                        rt.spawn(async move {
                            let state = app_handle.state::<AppState>();
                            let dictation_enabled = {
                                let config = state.config.lock().await;
                                config.get_all().dictation_enabled
                            };
                            if !dictation_enabled {
                                return;
                            }
                            *state.recording_type.lock().await = RecordingType::Dictation;
                            *state.press_start_time.lock().await = Some(std::time::Instant::now());
                            toggle_dictation(state, is_recording).await;
                        });
                    }),
                    on_release: Arc::new(|| {}),
                });
                log::info!("Toggle dictation hotkey registered: {}", toggle_str);
            } else {
                log::warn!("Failed to parse toggle dictation hotkey: {}", toggle_str);
            }
        }
    }

    if let Some(ref toggle_str) = language_toggle_hotkey {
        if !toggle_str.is_empty() {
            if let Ok(toggle_hotkey) = parse_rdev_hotkey(toggle_str) {
                let app_handle_toggle = app_handle.clone();
                let rt_for_toggle = rt_handle.clone();
                registrations.push(HotkeyRegistration {
                    target: toggle_hotkey,
                    active: Arc::new(AtomicBool::new(false)),
                    on_press: Arc::new(move || {
                        let app_handle = app_handle_toggle.clone();
                        rt_for_toggle.spawn(async move {
                            let state = app_handle.state::<AppState>();
                            let mut config_mgr = state.config.lock().await;
                            let mut cfg = config_mgr.get_all();
                            if !cfg.dictation_enabled {
                                return;
                            }
                            if cfg.languages.len() >= 2 {
                                cfg.languages.swap(0, 1);
                                let label = language_display_name(&cfg.languages[0]);
                                if config_mgr.save(cfg).is_ok() {
                                    drop(config_mgr);
                                    let msg = "Language: ".to_string();
                                    emit_overlay_event(
                                        &app_handle,
                                        OverlayEvent::Status {
                                            message: msg,
                                            highlight: Some(label),
                                        },
                                    );
                                }
                            }
                        });
                    }),
                    on_release: Arc::new(|| {}),
                });
                log::info!("Language toggle hotkey registered: {}", toggle_str);
            } else {
                log::warn!("Failed to parse language toggle hotkey: {}", toggle_str);
            }
        }
    }

    if let Some(ref cmd_hotkey_str) = command_hotkey_str {
        if let Ok(cmd_target) = parse_rdev_hotkey(cmd_hotkey_str) {
            let app_handle_press = app_handle.clone();
            let is_recording_press = is_recording_flag.clone();
            let rt_for_press = rt_handle.clone();
            let app_handle_release = app_handle.clone();
            let is_recording_release = is_recording_flag.clone();
            let rt_for_release = rt_handle.clone();

            registrations.push(HotkeyRegistration {
                target: cmd_target,
                active: Arc::new(AtomicBool::new(false)),
                on_press: Arc::new(move || {
                    log::info!("Command hotkey pressed");
                    let app_handle = app_handle_press.clone();
                    let is_recording = is_recording_press.clone();
                    let rt = rt_for_press.clone();
                    rt.spawn(async move {
                        let state = app_handle.state::<AppState>();
                        let (dictation_enabled, cmd_enabled) = {
                            let config = state.config.lock().await;
                            let cfg = config.get_all();
                            (cfg.dictation_enabled, cfg.command_config.enabled)
                        };
                        if !dictation_enabled || !cmd_enabled {
                            return;
                        }
                        *state.recording_type.lock().await = RecordingType::Command;
                        *state.press_start_time.lock().await = Some(std::time::Instant::now());
                        start_dictation(state, is_recording).await;
                    });
                }),
                on_release: Arc::new(move || {
                    let app_handle = app_handle_release.clone();
                    let is_recording = is_recording_release.clone();
                    let rt = rt_for_release.clone();
                    rt.spawn(async move {
                        let state = app_handle.state::<AppState>();
                        let (dictation_enabled, cmd_enabled) = {
                            let config = state.config.lock().await;
                            let cfg = config.get_all();
                            (cfg.dictation_enabled, cfg.command_config.enabled)
                        };
                        if !dictation_enabled || !cmd_enabled {
                            return;
                        }
                        let min_hold_ms = {
                            let config = state.config.lock().await;
                            config.get_all().min_hold_ms
                        };
                        let mut is_short_press = false;
                        if let Some(start_time) = state.press_start_time.lock().await.take() {
                            if start_time.elapsed().as_millis() < min_hold_ms as u128 {
                                is_short_press = true;
                            }
                        }
                        if is_short_press {
                            log::info!("Command short press detected, cancelling");
                            let mut audio_state = state.audio_state.lock().await;
                            if matches!(*audio_state, AudioState::Recording) {
                                *audio_state = AudioState::Idle;
                                is_recording.store(false, Ordering::SeqCst);
                                let _ = state.audio_capture.lock().await.stop_recording().await;
                                emit_overlay_event(&state.app_handle, OverlayEvent::ShortPress);
                                let app_for_overlay = state.app_handle.clone();
                                tauri::async_runtime::spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(1500))
                                        .await;
                                    reset_overlay_state(&app_for_overlay);
                                });
                            }
                        } else {
                            stop_dictation(state, is_recording).await;
                        }
                    });
                }),
            });
            log::info!("Command mode hotkey registered: {}", cmd_hotkey_str);
        } else {
            log::warn!("Failed to parse command hotkey: {}", cmd_hotkey_str);
        }
    }

    registrations
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = {
        let b = tauri::Builder::default()
            .plugin(tauri_plugin_notification::init())
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_shell::init());
        #[cfg(windows)]
        let b = b.device_event_filter(tauri::DeviceEventFilter::Always);
        b
    };
    builder.setup(|app| {
            // Initialize app state (first use of config; logger already set in main)
            let state = AppState::new(app.handle().clone())?;

            // Apply saved logging config to in-app log buffer
            app_log::reconfigure(state.config.blocking_lock().get_all().logging.clone());

            // Get config for startup behavior
            let start_in_focus = {
                let config = state.config.blocking_lock();
                config.get_all().start_in_focus
            };

            // Set app icon on main window (window title bar + taskbar on Windows)
            if let Some(window) = app.get_webview_window("main") {
                if let Err(e) = window.set_icon(WINDOW_ICON.clone()) {
                    log::warn!("Failed to set main window icon: {}", e);
                }
            }

            // Handle window visibility based on config
            if let Some(window) = app.get_webview_window("main") {
                if start_in_focus {
                    window.show()?;
                    window.set_focus()?;
                    log::info!("Window shown and focused on startup");
                } else {
                    // Play background start sound
                    if let Err(e) = play_sound(app.handle(), "started-in-background") {
                        log::warn!("Failed to play background start sound: {}", e);
                    }
                    log::info!("App started in background (minimized to tray)");
                }
            }

            // Clone what we need before managing state
            let is_recording_flag = state.is_recording.clone();

            // Manage state
            app.manage(state);

            // Apply initial overlay position
            if let Err(e) = update_overlay_position(app.handle()) {
                log::warn!("Failed to set initial overlay position: {}", e);
            }

            // Track cursor to keep overlay on the correct monitor
            let cursor_tracking_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    let _ = update_overlay_position(&cursor_tracking_handle);
                }
            });

            // Setup system tray
            TrayManager::setup(app)?;

            // Background update check: after delay, check for updates and notify if available
            let update_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                if let Err(e) = run_update_check(&update_handle).await {
                    log::debug!("Update check: {}", e);
                }
            });

            // Background monitor: if active mic disconnects, fall back to default and notify
            let hotplug_handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    let state = hotplug_handle.state::<AppState>();
                    let device_id = state.config.blocking_lock().get_all().audio_device.clone();
                    let fell_back = {
                        let mut capture = state.audio_capture.blocking_lock();
                        capture.try_fallback_if_disconnected(device_id.as_deref())
                    };
                    if fell_back {
                        let mut config_mgr = state.config.blocking_lock();
                        let mut cfg = config_mgr.get_all();
                        cfg.audio_device = Some("default".to_string());
                        let _ = config_mgr.save(cfg);
                        drop(config_mgr);
                        emit_overlay_event(&hotplug_handle, OverlayEvent::Error { message: "Microphone disconnected. Switched to system default.".to_string() });
                    }
                }
            });

            // Register global hotkeys via rdev (dictation + optional language toggle + optional command mode)
            let (hotkey_str, toggle_dictation_hotkey, language_toggle_hotkey, command_hotkey_str) = {
                let state = app.state::<AppState>();
                let config = state.config.blocking_lock();
                let cfg = config.get_all();
                let cmd_hk = cfg.command_config.enabled
                    .then(|| cfg.command_config.hotkey.clone())
                    .flatten()
                    .filter(|s| !s.trim().is_empty());
                (
                    cfg.hotkey.clone(),
                    cfg.toggle_dictation_hotkey.clone(),
                    cfg.language_toggle_hotkey.clone(),
                    cmd_hk,
                )
            };

            let rt = tokio::runtime::Runtime::new().expect("hotkey runtime");
            let rt_handle = rt.handle().clone();
            std::thread::spawn(move || {
                rt.block_on(std::future::pending::<()>());
            });

            let registrations = create_registrations(
                app.handle(),
                is_recording_flag.clone(),
                rt_handle.clone(),
                &hotkey_str,
                toggle_dictation_hotkey,
                language_toggle_hotkey,
                command_hotkey_str,
            );

            if !registrations.is_empty() {
                start_listener(registrations);
            }

            log::info!("Kalam initialized successfully");

            // Show overlay in collapsed state on startup
            if let Err(e) = show_overlay(app.handle()) {
                log::warn!("Failed to show overlay on startup: {}", e);
            }
            reset_overlay_state(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_platform,
            set_hotkeys_paused,
            get_app_version,
            check_for_updates,
            request_system_permission,
            open_system_permission_page,
            get_settings,
            save_settings,
            skip_onboarding_with_defaults,
            get_audio_devices,
            test_microphone_start,
            test_microphone_level,
            test_microphone_stop,
            get_history,
            get_db_status,
            clear_history,
            commands::get_snippets,
            commands::add_snippet,
            commands::remove_snippet,
            check_api_key,
            get_model_status,
            download_model,
            get_app_log,
            get_app_log_empty,
            open_app_data_folder,
            commands::export_logs_csv,
            commands::create_entry,
            commands::get_entries_by_type,
            commands::get_entries_with_reminder,
            commands::get_entries_for_reminders,
            commands::get_aggregate_stats,
            commands::get_daily_stats,
            commands::get_tasks_due_on,
            commands::get_reminders_due_on,
            commands::get_entry,
            commands::update_entry,
            commands::delete_entry,
            commands::search_notes,
            commands::get_note_labels,
            commands::empty_trash,
            commands::save_attachment,
            commands::search_similar,
            commands::get_dictionary_entries,
            commands::add_dictionary_entry,
            commands::delete_dictionary_entry,
            reset_application,
            check_model_requirements,
            start_local_model,
            stop_local_model,
            restart_local_model,
            delete_local_model,
            commands::fetch_llm_models,
            commands::generate_structured_data,
            commands::test_llm_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Check for app updates; if available and user has show_updates enabled, show a notification.
pub async fn run_update_check(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let show = app
        .state::<AppState>()
        .config
        .lock()
        .await
        .get_all()
        .notifications
        .show_updates;
    if !show {
        return Ok(());
    }
    let updater = app.updater().map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let update = updater
        .check()
        .await
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    if let Some(u) = update {
        let msg = format!(
            "Update {} available. Restart the app to install.",
            u.version
        );
        app.state::<AppState>()
            .notification_manager
            .info(&msg)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }
    Ok(())
}

/// User-initiated check from tray: always run check and show a notification (up to date, update available, or error).
pub async fn run_update_check_user_initiated(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let nm = &app.state::<AppState>().notification_manager;
    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => {
            let _ = nm.error("Could not check for updates.");
            return Err(anyhow::anyhow!("{:?}", e));
        }
    };
    let update = match updater.check().await {
        Ok(u) => u,
        Err(e) => {
            let _ = nm.error("Could not check for updates.");
            return Err(anyhow::anyhow!("{:?}", e));
        }
    };
    if let Some(u) = update {
        let msg = format!(
            "Update {} available. Restart the app to install.",
            u.version
        );
        nm.info(&msg).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    } else {
        let _ = nm.info("Kalam is up to date.");
    }
    Ok(())
}

// Tauri command handlers

#[tauri::command]
fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

#[tauri::command]
fn set_hotkeys_paused(paused: bool) {
    crate::hotkey::set_hotkeys_paused(paused);
}

#[tauri::command]
fn get_app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Check for updates; returns new version if available, None if up to date, Err on failure.
#[tauri::command]
async fn check_for_updates(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let updater = app.updater().map_err(|e| format!("{:?}", e))?;
    let update = updater.check().await.map_err(|e| format!("{:?}", e))?;
    Ok(update.as_ref().map(|u| u.version.clone()))
}

/// Request a system permission using OS-native methods when available.
/// - macOS accessibility: Shows the system "Allow app to control this computer?" dialog.
/// - Other cases: Opens the relevant settings page (same as open_system_permission_page).
#[tauri::command]
fn request_system_permission(permission: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if permission == "accessibility" {
            return request_macos_accessibility();
        }
        // Microphone: macOS shows the prompt on first capture; no separate request API from Rust. Open Settings.
    }

    #[cfg(any(windows, target_os = "linux"))]
    {
        let _ = permission;
    }

    open_system_permission_page(permission)
}

#[cfg(target_os = "macos")]
fn request_macos_accessibility() -> Result<(), String> {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;
    use core_foundation_sys::base::kCFAllocatorDefault;
    use core_foundation_sys::dictionary::{
        kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks, CFDictionaryCreate,
    };
    use std::ffi::c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: *const c_void) -> i32;
    }

    let key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
    let value = CFBoolean::true_value();
    let keys = [key.as_CFTypeRef()];
    let values = [value.as_CFTypeRef()];
    let dict_ref = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    let dict: CFDictionary<CFString, CFBoolean> =
        unsafe { CFDictionary::wrap_under_create_rule(dict_ref) };
    let _trusted = unsafe { AXIsProcessTrustedWithOptions(dict.as_CFTypeRef()) };
    Ok(())
}

#[tauri::command]
fn open_system_permission_page(permission: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        let uri = match permission {
            "microphone" => "ms-settings:privacy-microphone",
            "accessibility" => "ms-settings:easeofaccess-keyboard",
            _ => return Err(format!("Unknown permission: {}", permission)),
        };
        std::process::Command::new("cmd")
            .args(["/c", "start", "", uri])
            .spawn()
            .map_err(|e| format!("Failed to open settings: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        let uri = match permission {
            "microphone" => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
            }
            "accessibility" => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
            }
            _ => return Err(format!("Unknown permission: {}", permission)),
        };
        std::process::Command::new("open")
            .arg(uri)
            .spawn()
            .map_err(|e| format!("Failed to open settings: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        // No universal URI; frontend shows instructions only
        let _ = permission;
        Ok(())
    }
}

#[tauri::command]
async fn skip_onboarding_with_defaults(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let default_config = config::settings::AppConfig {
        onboarding_complete: true,
        ..Default::default()
    };

    if let Err(e) = state.audio_capture.lock().await.set_device("") {
        log::error!("Failed to set default audio device: {}", e);
        return Err(format!("Failed to set audio device: {}", e));
    }

    let mut config = state.config.lock().await;
    match config.save(default_config.clone()) {
        Ok(_) => {
            app_log::reconfigure(config.get_all().logging.clone());
            drop(config);
            let _ = state.app_handle.emit("settings_updated", default_config);
            let _ = update_overlay_position(&state.app_handle);
            Ok(())
        }
        Err(e) => {
            log::error!("skip_onboarding_with_defaults failed: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn get_settings(state: tauri::State<'_, AppState>) -> Result<AppConfig, String> {
    let config = state.config.lock().await;
    let cfg = config.get_all();
    log::debug!(
        "Returning settings, api_key present: {}",
        cfg.stt_config.api_key.is_some()
    );
    Ok(cfg)
}

#[tauri::command]
async fn save_settings(
    state: tauri::State<'_, AppState>,
    new_config: AppConfig,
) -> Result<(), String> {
    log::info!("=== SAVE_SETTINGS CALLED ===");
    log::info!(
        "API key present: {}",
        new_config.stt_config.api_key.is_some()
    );
    log::info!(
        "API key length: {:?}",
        new_config.stt_config.api_key.as_ref().map(|s| s.len())
    );
    log::info!("Audio device: {:?}", new_config.audio_device);

    // Reject if language toggle hotkey is the same as the dictation hotkeys
    if let Some(ref toggle) = new_config.language_toggle_hotkey {
        if !toggle.trim().is_empty() {
            if let Ok(toggle_hk) = parse_rdev_hotkey(toggle) {
                if let Some(ref hold) = new_config.hotkey {
                    if let Ok(hold_hk) = parse_rdev_hotkey(hold) {
                        if hold_hk == toggle_hk {
                            return Err("Language toggle hotkey cannot be the same as the hold dictation hotkey.".to_string());
                        }
                    }
                }
                if let Some(ref toggle_dict) = new_config.toggle_dictation_hotkey {
                    if let Ok(toggle_dict_hk) = parse_rdev_hotkey(toggle_dict) {
                        if toggle_dict_hk == toggle_hk {
                            return Err("Language toggle hotkey cannot be the same as the toggle dictation hotkey.".to_string());
                        }
                    }
                }
            }
        }
    }

    // Reject if command hotkey is the same as the dictation hotkeys when command mode is enabled
    if new_config.command_config.enabled {
        if let Some(ref cmd_hk) = new_config.command_config.hotkey {
            if !cmd_hk.trim().is_empty() {
                if let Ok(cmd_target) = parse_rdev_hotkey(cmd_hk) {
                    if let Some(ref hold) = new_config.hotkey {
                        if let Ok(hold_hk) = parse_rdev_hotkey(hold) {
                            if hold_hk == cmd_target {
                                return Err("Command mode hotkey cannot be the same as the hold dictation hotkey.".to_string());
                            }
                        }
                    }
                    if let Some(ref toggle_dict) = new_config.toggle_dictation_hotkey {
                        if let Ok(toggle_dict_hk) = parse_rdev_hotkey(toggle_dict) {
                            if toggle_dict_hk == cmd_target {
                                return Err("Command mode hotkey cannot be the same as the toggle dictation hotkey.".to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Normalize: empty or "null" string means default device
    let device_id_to_apply = new_config.audio_device.as_ref().and_then(|s| {
        if s.is_empty() || s == "null" {
            None
        } else {
            Some(s.as_str())
        }
    });
    let effective_device_id = device_id_to_apply.unwrap_or("");
    if let Err(e) = state
        .audio_capture
        .lock()
        .await
        .set_device(effective_device_id)
    {
        log::error!("Failed to set audio device: {}", e);
        return Err(format!("Failed to set audio device: {}", e));
    }
    log::info!(
        "Audio device set to: {}",
        if effective_device_id.is_empty() {
            "default"
        } else {
            effective_device_id
        }
    );

    // Persist config with None for default (so we don't store "" or "null")
    let config_to_save = AppConfig {
        audio_device: device_id_to_apply.map(String::from),
        ..new_config
    };

    let mut config = state.config.lock().await;
    match config.save(config_to_save.clone()) {
        Ok(_) => {
            app_log::reconfigure(config.get_all().logging.clone());
            let _ = state.app_handle.emit("settings_updated", &config_to_save);

            // Drop the lock before calling update_overlay_position
            // to avoid blocking the async runtime
            drop(config);
            let _ = update_overlay_position(&state.app_handle);

            reset_overlay_state(&state.app_handle);

            if !config_to_save.dictation_enabled {
                // If dictation was turned off while recording, abort it immediately
                if state.is_recording.load(Ordering::SeqCst) {
                    log::info!("Dictation disabled while recording, aborting dictation...");
                    state.is_recording.store(false, Ordering::SeqCst);
                    let mut audio_state = state.audio_state.lock().await;
                    *audio_state = AudioState::Idle;
                    let _ = state.audio_capture.lock().await.stop_recording().await;
                    let _ = crate::tray::TrayManager::set_tray_state(
                        &state.app_handle,
                        AudioState::Idle,
                    );
                }
            }

            // Update hotkey registrations
            let rt_handle = tokio::runtime::Handle::current();
            let cmd_hk = config_to_save
                .command_config
                .enabled
                .then(|| config_to_save.command_config.hotkey.clone())
                .flatten()
                .filter(|s| !s.trim().is_empty());

            let registrations = create_registrations(
                &state.app_handle,
                state.is_recording.clone(),
                rt_handle,
                &config_to_save.hotkey,
                config_to_save.toggle_dictation_hotkey.clone(),
                config_to_save.language_toggle_hotkey.clone(),
                cmd_hk,
            );
            crate::hotkey::update_registrations(registrations);

            log::info!("=== SAVE_SETTINGS SUCCESS ===");
            Ok(())
        }
        Err(e) => {
            log::error!("=== SAVE_SETTINGS FAILED: {} ===", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn get_audio_devices() -> Result<Vec<audio::AudioDevice>, String> {
    audio::list_devices().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_app_log() -> Result<String, String> {
    Ok(app_log::get_snapshot())
}

#[tauri::command]
fn get_app_log_empty() -> Result<bool, String> {
    Ok(app_log::is_empty())
}

#[tauri::command]
async fn reset_application(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let kalam_dir = crate::config::get_kalam_dir().map_err(|e| e.to_string())?;
    let config_path = kalam_dir.join("config.json");
    let _ = std::fs::remove_file(&config_path);

    if let Err(e) = history::delete_all_persisted_data() {
        log::warn!("reset_application: failed to delete history data: {}", e);
    }

    if let Err(e) = state.audio_capture.lock().await.set_device("") {
        log::warn!(
            "reset_application: failed to set default audio device: {}",
            e
        );
    }

    let default_config = AppConfig::default();
    let mut config = state.config.lock().await;
    config
        .save(default_config.clone())
        .map_err(|e| e.to_string())?;
    app_log::reconfigure(config.get_all().logging.clone());
    drop(config);
    let _ = state.app_handle.emit("app_reset", ());
    let _ = update_overlay_position(&state.app_handle);
    Ok(())
}

#[tauri::command]
fn open_app_data_folder() -> Result<(), String> {
    let path = crate::config::get_kalam_dir().map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(serde::Serialize)]
pub struct TestMicrophoneResult {
    pub level: f32,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

#[tauri::command]
async fn test_microphone_start(state: tauri::State<'_, AppState>) -> Result<(), String> {
    log::info!("Test recording started");
    state
        .audio_capture
        .lock()
        .await
        .start_recording()
        .await
        .map_err(|e| format!("Failed to start: {}", e))
}

#[tauri::command]
async fn test_microphone_level(state: tauri::State<'_, AppState>) -> Result<f32, String> {
    Ok(state
        .audio_capture
        .lock()
        .await
        .get_current_recording_level())
}

#[tauri::command]
async fn test_microphone_stop(
    state: tauri::State<'_, AppState>,
) -> Result<TestMicrophoneResult, String> {
    log::info!("Test recording stop requested");
    match state
        .audio_capture
        .lock()
        .await
        .stop_and_get_test_result()
        .await
    {
        Ok((level, samples, sample_rate)) => {
            log::info!("Test stopped, level: {}, samples: {}", level, samples.len());
            Ok(TestMicrophoneResult {
                level,
                samples,
                sample_rate,
            })
        }
        Err(e) => {
            log::error!("Test stop failed: {}", e);
            Err(format!("{}", e))
        }
    }
}

#[tauri::command]
async fn get_history(
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<history::HistoryEntry>, String> {
    history::get_history(limit, offset)
        .await
        .map_err(|e| e.to_string())
}

/// Returns database connectivity status for the status bar.
#[tauri::command]
fn get_db_status() -> DbStatus {
    let ok = crate::db::open_db().is_ok();
    DbStatus { ok }
}

#[derive(serde::Serialize)]
struct DbStatus {
    ok: bool,
}

#[tauri::command]
async fn clear_history() -> Result<(), String> {
    history::clear().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_model_status(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    use serde_json::json;
    fn status_parts(status: crate::stt::lifecycle::ModelStatus) -> (String, Option<String>) {
        match status {
            crate::stt::lifecycle::ModelStatus::NotInstalled => ("NotInstalled".to_string(), None),
            crate::stt::lifecycle::ModelStatus::Stopped => ("Stopped".to_string(), None),
            crate::stt::lifecycle::ModelStatus::Starting => ("Starting".to_string(), None),
            crate::stt::lifecycle::ModelStatus::Running => ("Running".to_string(), None),
            crate::stt::lifecycle::ModelStatus::Error(msg) => ("Error".to_string(), Some(msg)),
        }
    }
    let mut out = serde_json::Map::new();
    for m in crate::stt::models::known_models() {
        let status = state.local_model_manager.get_status(m.id).await;
        let (status_label, error_message) = status_parts(status);
        out.insert(
            m.id.to_string(),
            json!({
                "installed": crate::stt::models::is_installed(m.id),
                "size_mb": m.size_mb,
                "status": status_label,
                "error": error_message,
                "download_progress": serde_json::Value::Null
            }),
        );
    }
    Ok(serde_json::Value::Object(out))
}

#[tauri::command]
async fn start_local_model(
    state: tauri::State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    state
        .local_model_manager
        .start_model(&model_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_local_model(
    state: tauri::State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    state
        .local_model_manager
        .stop_model(&model_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn restart_local_model(
    state: tauri::State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    state
        .local_model_manager
        .restart_model(&model_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_local_model(
    state: tauri::State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    state
        .local_model_manager
        .delete_model(&model_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn download_model(
    state: tauri::State<'_, AppState>,
    model_type: String,
) -> Result<(), String> {
    crate::stt::models::download_model_with_progress(&model_type, &state.app_handle)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn check_api_key(provider: String, api_key: String) -> Result<bool, String> {
    log::info!(
        "check_api_key called with provider: {}, api_key length: {}",
        provider,
        api_key.len()
    );

    if api_key.is_empty() {
        log::warn!("API key is empty");
        return Ok(false);
    }

    match stt::validate_api_key(&provider, &api_key).await {
        Ok(result) => {
            log::info!("API validation result: {}", result);
            Ok(result)
        }
        Err(e) => {
            log::error!("API validation error: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
fn check_model_requirements(model_id: String) -> Result<system_reqs::HardwareCheckResult, String> {
    Ok(system_reqs::check_model_requirements(&model_id))
}

const OVERLAY_LABEL: &str = "overlay";
const OVERLAY_WIDTH: i32 = 300;
const OVERLAY_HEIGHT: i32 = 120;
const OVERLAY_BOTTOM_MARGIN: i32 = 24;

#[derive(serde::Serialize, Clone)]
#[serde(tag = "kind")]
enum OverlayEvent {
    Hidden,
    Collapsed,
    Listening,
    ShortPress,
    Recording {
        level: f32,
        is_command: bool,
    },
    Processing,
    #[allow(dead_code)] // Reserved for future "transcription succeeded" UI
    Success,
    Error {
        message: String,
    },
    Status {
        message: String,
        highlight: Option<String>,
    },
}

fn emit_overlay_event(app: &tauri::AppHandle, event: OverlayEvent) {
    // Emit only to the overlay window so it always receives the event
    let _ = app.emit_to(OVERLAY_LABEL, "overlay-state", event);
}

fn reset_overlay_state(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let dictation_enabled = if let Ok(config) = state.config.try_lock() {
        config.get_all().dictation_enabled
    } else {
        true // default fallback
    };
    if dictation_enabled {
        emit_overlay_event(app, OverlayEvent::Collapsed);
    } else {
        emit_overlay_event(app, OverlayEvent::Hidden);
    }
}

fn update_overlay_position(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let overlay = app
        .get_webview_window(OVERLAY_LABEL)
        .ok_or_else(|| anyhow::anyhow!("Overlay window not found"))?;
    let win = overlay.as_ref().window();

    let state = app.state::<AppState>();
    let (position, offset_x, offset_y) = {
        // Use try_lock to avoid blocking the async runtime
        if let Ok(cfg) = state.config.try_lock() {
            let all = cfg.get_all();
            (
                all.overlay_position.clone(),
                all.overlay_offset_x,
                all.overlay_offset_y,
            )
        } else {
            // Fallback if we can't lock immediately
            (crate::config::OverlayPosition::default(), 0, 0)
        }
    };

    // Get absolute cursor position using the OS API, then find the matching monitor
    let cursor_screen_pos: Option<(i32, i32)> = {
        #[cfg(windows)]
        {
            let mut pt = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
            let ok = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt) };
            if ok != 0 {
                Some((pt.x, pt.y))
            } else {
                None
            }
        }
        #[cfg(not(windows))]
        {
            None
        }
    };

    let target_monitor = cursor_screen_pos
        .and_then(|(cx, cy)| {
            win.available_monitors().ok()?.into_iter().find(|m| {
                let pos = m.position();
                let size = m.size();
                cx >= pos.x
                    && cx < pos.x + size.width as i32
                    && cy >= pos.y
                    && cy < pos.y + size.height as i32
            })
        })
        .or_else(|| win.primary_monitor().ok().flatten());

    if let Some(monitor) = target_monitor {
        let wa = monitor.work_area();
        let scale_factor = monitor.scale_factor();

        // We must calculate the physical size based on our known logical size.
        // If we use win.outer_size() while the window is hidden, it may return 0,
        // causing the window to be placed completely off-screen or behind the taskbar.
        let physical_width = (OVERLAY_WIDTH as f64 * scale_factor).round() as i32;
        let physical_height = (OVERLAY_HEIGHT as f64 * scale_factor).round() as i32;
        let physical_margin = (OVERLAY_BOTTOM_MARGIN as f64 * scale_factor).round() as i32;
        let physical_offset_x = (offset_x as f64 * scale_factor).round() as i32;
        let physical_offset_y = (offset_y as f64 * scale_factor).round() as i32;

        let mut x = wa.position.x;
        let mut y = wa.position.y;

        use crate::config::OverlayPosition::*;
        match position {
            BottomCenter => {
                x += (wa.size.width as i32 - physical_width) / 2;
                y += wa.size.height as i32 - physical_height - physical_margin;
            }
            BottomLeft => {
                x += physical_margin;
                y += wa.size.height as i32 - physical_height - physical_margin;
            }
            BottomRight => {
                x += wa.size.width as i32 - physical_width - physical_margin;
                y += wa.size.height as i32 - physical_height - physical_margin;
            }
            TopCenter => {
                x += (wa.size.width as i32 - physical_width) / 2;
                y += physical_margin;
            }
            TopLeft => {
                x += physical_margin;
                y += physical_margin;
            }
            TopRight => {
                x += wa.size.width as i32 - physical_width - physical_margin;
                y += physical_margin;
            }
            CenterLeft => {
                x += physical_margin;
                y += (wa.size.height as i32 - physical_height) / 2;
            }
            CenterRight => {
                x += wa.size.width as i32 - physical_width - physical_margin;
                y += (wa.size.height as i32 - physical_height) / 2;
            }
            Center => {
                x += (wa.size.width as i32 - physical_width) / 2;
                y += (wa.size.height as i32 - physical_height) / 2;
            }
        }

        x += physical_offset_x;
        y += physical_offset_y;

        let new_pos = tauri::PhysicalPosition { x, y };
        let mut should_update = true;

        if let Ok(current_pos) = overlay.outer_position() {
            // Allow a small 1px variance due to rounding
            if (current_pos.x - new_pos.x).abs() <= 1 && (current_pos.y - new_pos.y).abs() <= 1 {
                should_update = false;
            }
        }

        if should_update {
            let _ = overlay.set_position(tauri::Position::Physical(new_pos));
            let _ = overlay.set_always_on_top(true); // Re-assert to ensure it stays above taskbar
        }
    }
    Ok(())
}

/// Show the overlay window at the configured position without stealing focus.
fn show_overlay(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let _ = update_overlay_position(app);
    let overlay = app
        .get_webview_window(OVERLAY_LABEL)
        .ok_or_else(|| anyhow::anyhow!("Overlay window not found"))?;
    #[allow(unused_variables)]
    let win = overlay.as_ref().window();

    #[cfg(windows)]
    let prev_hwnd = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };

    #[cfg(windows)]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle::Win32};
        if let Ok(handle) = win.window_handle() {
            if let Win32(win32) = handle.as_raw() {
                use windows_sys::Win32::Foundation::HWND;
                let hwnd: HWND = win32.hwnd.get();
                const SW_SHOWNOACTIVATE: i32 = 4;
                let _ = unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(hwnd, SW_SHOWNOACTIVATE)
                };
            }
        }
    }

    overlay.show()?;

    #[cfg(windows)]
    {
        if prev_hwnd != 0 {
            unsafe { windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(prev_hwnd) };
        }
    }
    Ok(())
}

async fn start_dictation(state: tauri::State<'_, AppState>, is_recording: Arc<AtomicBool>) {
    if is_recording.load(Ordering::SeqCst) {
        log::debug!("Already recording, ignoring start request");
        return;
    }

    let mut audio_state = state.audio_state.lock().await;

    if matches!(*audio_state, AudioState::Idle) {
        log::info!("Starting dictation...");

        // Ensure overlay is positioned on the monitor where the cursor is right now
        let _ = update_overlay_position(&state.app_handle);

        emit_overlay_event(&state.app_handle, OverlayEvent::Listening);

        #[cfg(windows)]
        {
            let hwnd =
                unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
            if hwnd != 0 {
                *state.foreground_for_injection.lock().await = Some(hwnd as usize);
            }
        }
        *audio_state = AudioState::Recording;
        is_recording.store(true, Ordering::SeqCst);
        let _ = crate::tray::TrayManager::set_tray_state(&state.app_handle, AudioState::Recording);

        // Play start sound
        let app_handle = state.app_handle.clone();

        if let Err(e) = play_sound(&app_handle, "dictation-started") {
            log::warn!("Failed to play dictation start sound: {}", e);
        }

        // Start actual audio recording
        if let Err(e) = state.audio_capture.lock().await.start_recording().await {
            log::error!("Failed to start recording: {}", e);
            *audio_state = AudioState::Idle;
            is_recording.store(false, Ordering::SeqCst);
            return;
        }

        // Emit audio level to overlay while recording
        let app_handle_level = state.app_handle.clone();
        let audio_capture = state.audio_capture.clone();
        let is_recording_level = is_recording.clone();
        let is_command = *state.recording_type.lock().await == RecordingType::Command;
        tauri::async_runtime::spawn(async move {
            while is_recording_level.load(Ordering::SeqCst) {
                let level = audio_capture.lock().await.get_current_recording_level();
                emit_overlay_event(
                    &app_handle_level,
                    OverlayEvent::Recording { level, is_command },
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        });

        drop(audio_state);

        log::info!("Audio recording started");
    } else {
        log::debug!("Cannot start dictation, current state: {:?}", *audio_state);
    }
}

/// Input for the blocking transcription thread. Either create provider from config (Cloud/Groq)
/// inside the thread, or use an already-created provider (Local).
enum TranscribeJob {
    FromConfig {
        stt_config: STTConfig,
        audio_data: Vec<f32>,
        sample_rate: u32,
        vad_config: VADConfig,
        language: Option<String>,
        vocabulary: Option<String>,
    },
    FromProvider {
        provider: Box<dyn crate::stt::provider::STTProvider>,
        audio_data: Vec<f32>,
        sample_rate: u32,
        vad_config: VADConfig,
        language: Option<String>,
        vocabulary: Option<String>,
    },
}

/// Command mode: parse "new note/task/reminder" from transcribed text, or use LLM to infer type and extract fields.
async fn run_command_pipeline(
    text: &str,
    config: &AppConfig,
    app_handle: &tauri::AppHandle,
) -> Result<(), String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("No speech detected.".to_string());
    }
    let lower = trimmed.to_lowercase();

    let cmd = &config.command_config;
    let provider = cmd.provider.as_deref().unwrap_or("");
    let use_llm = cmd.enabled
        && !provider.is_empty()
        && cmd
            .api_keys
            .get(provider)
            .map(|s| !s.is_empty())
            .unwrap_or(false)
        && cmd
            .models
            .get(provider)
            .map(|s| !s.is_empty())
            .unwrap_or(false);

    // When LLM is enabled: infer entry_type and all fields from full transcription. No "new note/task/reminder" required.
    // When LLM is disabled: require literal prefix and use the rest as payload.
    let (entry_type, payload) = if use_llm {
        // We'll get entry_type from the LLM response; use a placeholder until then.
        ("", trimmed.to_string())
    } else {
        let (t, p) = if lower.starts_with("new note") {
            ("note", trimmed["new note".len()..].trim().to_string())
        } else if lower.starts_with("new task") {
            ("task", trimmed["new task".len()..].trim().to_string())
        } else if lower.starts_with("new reminder") {
            (
                "reminder",
                trimmed["new reminder".len()..].trim().to_string(),
            )
        } else {
            let state = app_handle.state::<AppState>();
            let _ = state.notification_manager.warning(
                "Say \"new note\", \"new task\", or \"new reminder\" followed by your content.",
            );
            return Err("Could not detect command. Say 'new note', 'new task', or 'new reminder' followed by your content.".to_string());
        };
        if p.is_empty() {
            return Err("Command content is empty.".to_string());
        }
        (t, p)
    };

    let now = chrono::Utc::now();
    let mut resolved_entry_type = entry_type.to_string();
    let mut title: Option<String> = None;
    let mut content = if use_llm {
        String::new()
    } else {
        payload.clone()
    };
    let mut due_date: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut reminder_at: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut rrule: Option<String> = None;
    let mut tags: Vec<String> = vec![];
    let mut subtasks: Option<Vec<crate::models::Subtask>> = None;

    if use_llm {
        let api_key = cmd.api_keys.get(provider).map(|s| s.as_str()).unwrap_or("");
        let model = cmd.models.get(provider).map(|s| s.as_str()).unwrap_or("");
        let today = now.format("%Y-%m-%d").to_string();
        let system_prompt = format!(
            r#"You are a command parser for a voice-controlled productivity app. The user spoke one phrase. Your job is to classify it as a TASK, REMINDER, or NOTE and extract structured fields.

CLASSIFICATION RULES (follow strictly):
- TASK: something the user needs to DO or COMPLETE. Action-oriented. Examples: "buy groceries", "finish the report by Friday", "call the dentist", "pick up the kids", "send the invoice", "clean the garage". If it describes work, an errand, a to-do item, or anything that can be checked off as done, it is a TASK. A task MAY have a due date but does not require one.
- REMINDER: a time-based ALERT to jog the user's memory at a specific moment. The emphasis is on WHEN to be notified, not on completing work. Examples: "remind me about the meeting at 3pm", "remind me to take my medication every morning", "don't forget Mom's birthday on March 15th". Key signals: "remind me", "don't forget", "alert me", or any phrasing where the core intent is to receive a notification at a particular time. A reminder MUST have a reminder_at time.
- NOTE: information to record for reference — thoughts, ideas, observations, or anything that is neither actionable nor time-triggered. Examples: "the WiFi password is abc123", "meeting notes from today's standup", "recipe for banana bread".

WHEN IN DOUBT: if the user describes something to accomplish (even without a deadline), choose TASK. Only choose REMINDER when the user explicitly wants to be notified/alerted at a time.

Return ONLY a valid JSON object with these keys (omit any key that is not present or not inferable):
- entry_type: exactly one of "note", "task", "reminder"
- title: short, clear summary (for tasks: the action; for reminders: what to be reminded about; for notes: a brief heading)
- content: additional details or description (optional; for reminders usually omit unless the user gave extra context)
- due_date: ISO 8601 date-time deadline. Applies to tasks (when it's due) and notes (if the user mentioned a date relevant to the note). Not used for reminders.
- reminder_at: ISO 8601 date-time for when to send a notification. Required for reminders. Also set on tasks or notes if the user wants to be reminded about them (e.g. "buy milk by Friday, remind me Thursday" → task with due_date Friday + reminder_at Thursday; "note about the meeting, remind me at 2pm" → note with reminder_at).
- rrule: RFC 5545 recurrence rule if repetition is mentioned (e.g. "FREQ=DAILY", "FREQ=WEEKLY;BYDAY=MO"). Can apply to reminders or recurring tasks.
- tags: array of short category/label strings. Infer relevant tags from the content even if the user didn't explicitly say them. For example: "buy groceries" → ["shopping", "errands"]; "finish the quarterly report" → ["work", "reports"]; "dentist appointment Thursday" → ["health", "appointments"]; "recipe for banana bread" → ["recipes", "cooking"]. Use 1-3 lowercase tags that help the user organize and filter entries later. Omit if nothing meaningful can be inferred.
- subtasks: array of strings (only for tasks, if the user listed multiple steps or sub-items)

Today's date is {}. Use it to resolve relative dates like "tomorrow", "next Monday", "in two hours", "at 3pm". Always include a time component (default to 09:00 UTC if only a date is mentioned). Return only the JSON object, no markdown or explanation."#,
            today
        );
        match crate::commands::generate_structured_data(
            provider.to_string(),
            api_key.to_string(),
            model.to_string(),
            system_prompt,
            payload.clone(),
        )
        .await
        {
            Ok(json_str) => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(obj) = v.as_object() {
                        if let Some(et) = obj.get("entry_type").and_then(|x| x.as_str()) {
                            let et_lower = et.to_lowercase();
                            if et_lower == "note" || et_lower == "task" || et_lower == "reminder" {
                                resolved_entry_type = et_lower;
                            }
                        }
                        if resolved_entry_type.is_empty() {
                            log::warn!("LLM did not return valid entry_type, defaulting to note");
                            resolved_entry_type = "note".to_string();
                        }
                        if let Some(t) = obj.get("title").and_then(|x| x.as_str()) {
                            title = Some(t.to_string());
                        }
                        if let Some(c) = obj.get("content").and_then(|x| x.as_str()) {
                            content = c.to_string();
                        }
                        if let Some(d) = obj.get("due_date").and_then(|x| x.as_str()) {
                            if let Some(dt) = parse_iso_datetime(d) {
                                due_date = Some(dt);
                            }
                        }
                        if let Some(r) = obj.get("reminder_at").and_then(|x| x.as_str()) {
                            if let Some(dt) = parse_iso_datetime(r) {
                                reminder_at = Some(dt);
                            }
                        }
                        if let Some(r) = obj.get("rrule").and_then(|x| x.as_str()) {
                            rrule = Some(r.to_string());
                        }
                        if let Some(arr) = obj.get("tags").and_then(|x| x.as_array()) {
                            tags = arr
                                .iter()
                                .filter_map(|x| x.as_str().map(String::from))
                                .collect();
                        }
                        if let Some(arr) = obj.get("subtasks").and_then(|x| x.as_array()) {
                            subtasks = Some(
                                arr.iter()
                                    .filter_map(|x| {
                                        x.as_str().map(|s| crate::models::Subtask {
                                            title: s.to_string(),
                                            is_completed: false,
                                        })
                                    })
                                    .collect(),
                            );
                        }
                    }
                } else {
                    log::warn!(
                        "LLM returned invalid JSON, using raw text as note: {}",
                        json_str.chars().take(100).collect::<String>()
                    );
                    resolved_entry_type = "note".to_string();
                    content = payload.clone();
                }
            }
            Err(e) => {
                log::warn!("LLM parsing failed, using raw payload as note: {}", e);
                resolved_entry_type = "note".to_string();
                content = payload.clone();
            }
        }
    }

    let entry_type = resolved_entry_type.as_str();

    if title.is_none() && !content.is_empty() {
        let first = content.lines().next().unwrap_or(&content).to_string();
        title = Some(first.clone());
        if content.lines().count() > 1 {
            content = content
                .lines()
                .skip(1)
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string();
        } else {
            content = String::new();
        }
    }
    if title.as_deref() == Some("") {
        title = None;
    }

    if entry_type == "reminder" && title.is_none() && content.is_empty() {
        title = Some(payload.clone());
    }

    let entry = crate::models::Entry {
        id: uuid::Uuid::new_v4().to_string(),
        entry_type: entry_type.to_string(),
        created_at: now,
        updated_at: now,
        sync_status: "pending".to_string(),
        title: title.clone(),
        content: content.clone(),
        attachments: vec![],
        tags: tags.clone(),
        color: None,
        is_pinned: false,
        priority: None,
        due_date,
        subtasks,
        is_completed: if entry_type == "task" {
            Some(false)
        } else {
            None
        },
        reminder_at,
        rrule,
        archived_at: None,
        deleted_at: None,
    };

    let conn = crate::db::open_db().map_err(|e| e.to_string())?;
    crate::db::insert_entry(&conn, &entry).map_err(|e| e.to_string())?;
    crate::db::insert_embedding_stub(&conn, &entry.id).map_err(|e| e.to_string())?;

    let _state = app_handle.state::<AppState>();
    let label = match entry_type {
        "task" => "Task",
        "reminder" => "Reminder",
        _ => "Note",
    };
    let summary = title.as_deref().unwrap_or(content.as_str());
    let msg = format!(
        "{} created: {}",
        label,
        if summary.len() > 50 {
            format!("{}...", &summary[..47])
        } else {
            summary.to_string()
        }
    );
    emit_overlay_event(
        app_handle,
        OverlayEvent::Status {
            message: msg,
            highlight: None,
        },
    );
    let _ = app_handle.emit("command-entry-created", ());
    Ok(())
}

fn run_transcribe_job(job: TranscribeJob) -> anyhow::Result<crate::stt::TranscriptionResult> {
    let (language, vocabulary) = match &job {
        TranscribeJob::FromConfig {
            language,
            vocabulary,
            ..
        } => (language.clone(), vocabulary.clone()),
        TranscribeJob::FromProvider {
            language,
            vocabulary,
            ..
        } => (language.clone(), vocabulary.clone()),
    };
    let language = language.as_deref();
    let vocabulary = vocabulary.as_deref();
    match job {
        TranscribeJob::FromConfig {
            stt_config,
            audio_data,
            sample_rate,
            vad_config,
            ..
        } => {
            let provider = crate::stt::provider::create_provider_sync(&stt_config)?;
            log::info!(
                "Starting transcription with {} (chunked + prompt chaining)",
                provider.name()
            );
            crate::stt::transcribe_chunked(
                &*provider,
                &audio_data,
                sample_rate,
                &vad_config,
                language,
                vocabulary,
            )
        }
        TranscribeJob::FromProvider {
            provider,
            audio_data,
            sample_rate,
            vad_config,
            ..
        } => crate::stt::transcribe_chunked(
            &*provider,
            &audio_data,
            sample_rate,
            &vad_config,
            language,
            vocabulary,
        ),
    }
}

async fn stop_dictation(state: tauri::State<'_, AppState>, is_recording: Arc<AtomicBool>) {
    // Check if actually recording
    if !is_recording.load(Ordering::SeqCst) {
        log::debug!("Not recording, ignoring stop request");
        return;
    }

    let mut audio_state = state.audio_state.lock().await;

    if matches!(*audio_state, AudioState::Recording) {
        log::info!("Stopping dictation...");
        *audio_state = AudioState::Processing;
        is_recording.store(false, Ordering::SeqCst);
        emit_overlay_event(&state.app_handle, OverlayEvent::Processing);
        let _ = crate::tray::TrayManager::set_tray_state(&state.app_handle, AudioState::Processing);

        // Play end sound
        let app_handle = state.app_handle.clone();

        if let Err(e) = play_sound(&app_handle, "dictation-ended") {
            log::warn!("Failed to play dictation end sound: {}", e);
        }

        let (audio_data, sample_rate) =
            match state.audio_capture.lock().await.stop_recording().await {
                Ok(result) => result,
                Err(e) => {
                    log::error!("Failed to stop recording: {}", e);
                    *audio_state = AudioState::Idle;
                    emit_overlay_event(
                        &state.app_handle,
                        OverlayEvent::Error {
                            message: "Recording failed".to_string(),
                        },
                    );
                    let app_for_overlay = state.app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                        reset_overlay_state(&app_for_overlay);
                    });
                    return;
                }
            };

        drop(audio_state);
        log::info!(
            "Audio recording stopped, {} samples at {}Hz, processing...",
            audio_data.len(),
            sample_rate
        );

        let mut config = state.config.lock().await.get_all();
        let vocabulary = if let Ok(conn) = crate::db::open_db() {
            if let Ok(entries) = crate::db::get_entries_by_type(&conn, "snippet", None, 500, 0) {
                config.snippets = entries
                    .into_iter()
                    .map(|e| config::Snippet {
                        trigger: e.title.unwrap_or_default(),
                        expansion: e.content,
                    })
                    .collect();
            }
            crate::db::get_dictionary_entries(&conn)
                .ok()
                .and_then(|entries| {
                    let s = entries
                        .iter()
                        .map(|e| e.term.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    if s.is_empty() {
                        None
                    } else if s.len() > 800 {
                        Some(s.chars().take(800).collect::<String>())
                    } else {
                        Some(s)
                    }
                })
        } else {
            None
        };
        if let Some(ref v) = vocabulary {
            log::info!(
                "Dictionary vocabulary loaded for transcription ({} chars, preview: {:?})",
                v.len(),
                v.chars().take(80).collect::<String>()
            );
        }
        let config = config;
        let audio_state_ref = state.audio_state.clone();
        let last_injected_len = state.last_injected_len.clone();
        let last_injected_text = state.last_injected_text.clone();
        let app_handle = state.app_handle.clone();
        #[allow(unused_variables)]
        let foreground_hwnd = state.foreground_for_injection.lock().await.take();
        let recording_type = *state.recording_type.lock().await;

        tokio::spawn(async move {
            let stt_config = crate::config::privacy::effective_stt_config(&config);
            let vad_config = stt_config.vad_config();
            // Create Cloud/Groq provider inside the OS thread so reqwest::blocking::Client
            // is never created/dropped on a tokio worker (avoids runtime drop panic).
            let (tx, rx) = oneshot::channel();
            let is_sync_capable = match stt_config.mode {
                crate::config::STTMode::Cloud
                | crate::config::STTMode::Hybrid
                | crate::config::STTMode::Auto => true,
                crate::config::STTMode::Local => false,
            };

            let language = config.languages.first().cloned();
            let job = if is_sync_capable {
                TranscribeJob::FromConfig {
                    stt_config: stt_config.clone(),
                    audio_data,
                    sample_rate,
                    vad_config,
                    language: language.clone(),
                    vocabulary: vocabulary.clone(),
                }
            } else {
                let provider = match crate::stt::provider::STTProviderFactory::create(
                    &stt_config,
                    Some(app_handle.clone()),
                )
                .await
                {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("STT provider creation failed: {}", e);
                        let mut audio_state = audio_state_ref.lock().await;
                        *audio_state = AudioState::Idle;
                        emit_overlay_event(
                            &app_handle,
                            OverlayEvent::Error {
                                message: "Provider failed".to_string(),
                            },
                        );
                        let _ =
                            crate::tray::TrayManager::set_tray_state(&app_handle, AudioState::Idle);
                        let app_for_overlay = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                            reset_overlay_state(&app_for_overlay);
                        });
                        return;
                    }
                };
                log::info!(
                    "Starting transcription with {} (chunked + prompt chaining)",
                    provider.name()
                );
                TranscribeJob::FromProvider {
                    provider,
                    audio_data,
                    sample_rate,
                    vad_config,
                    language,
                    vocabulary: vocabulary.clone(),
                }
            };
            if matches!(&job, TranscribeJob::FromConfig { .. }) {
                log::info!(
                    "Starting transcription (Cloud/Groq on OS thread, chunked + prompt chaining)"
                );
            }
            let start = std::time::Instant::now();
            std::thread::spawn(move || {
                let result = run_transcribe_job(job);
                let _ = tx.send(result);
            });

            match tokio::time::timeout(std::time::Duration::from_secs(120), rx).await {
                Ok(Ok(Ok(result))) => {
                    let latency_ms = start.elapsed().as_millis() as u32;
                    if recording_type == RecordingType::Command {
                        if let Err(e) =
                            run_command_pipeline(&result.text, &config, &app_handle).await
                        {
                            log::error!("Command pipeline failed: {}", e);
                            emit_overlay_event(
                                &app_handle,
                                OverlayEvent::Error { message: e.clone() },
                            );
                        }
                    } else {
                        let len = last_injected_len.load(Ordering::SeqCst);
                        let prev_text = last_injected_text.lock().await.clone();
                        let prev_ref = prev_text.as_str();
                        let (formatted, actions) = crate::formatting::format_text(
                            &result.text,
                            &config.formatting,
                            &config.snippets,
                            len,
                            Some(prev_ref),
                        );
                        log::info!("Transcription completed, length: {}", formatted.len());
                        if let Err(e) = history::save_transcription(&formatted).await {
                            log::error!("Failed to save transcription to DB: {}", e);
                        }
                        let words_count = formatted.split_whitespace().count() as u32;
                        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        if let Ok(conn) = db::open_db() {
                            if let Err(e) = db::record_transcription_stats(
                                &conn,
                                &date,
                                words_count,
                                latency_ms,
                            ) {
                                log::error!("Failed to record daily stats: {}", e);
                            }
                        }
                        #[derive(Clone, serde::Serialize)]
                        struct TranscriptionSavedPayload {
                            latency_ms: u32,
                            words_count: u32,
                        }
                        let _ = app_handle.emit(
                            "transcription-saved",
                            TranscriptionSavedPayload {
                                latency_ms,
                                words_count,
                            },
                        );
                        let _ = app_handle.emit("dictation-result", &formatted);
                        #[cfg(windows)]
                        if let Some(hwnd) = foreground_hwnd {
                            let _ = unsafe {
                                windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(
                                    hwnd as isize,
                                )
                            };
                            std::thread::sleep(std::time::Duration::from_millis(50));
                        }
                        if let Ok(injector) = crate::injection::TextInjector::new() {
                            for action in &actions {
                                match action {
                                    crate::formatting::VoiceAction::Undo => {
                                        if let Err(e) = injector.inject_undo() {
                                            log::error!("Failed to inject undo: {}", e);
                                        }
                                    }
                                    crate::formatting::VoiceAction::DeleteLastChars(n) => {
                                        if let Err(e) = injector.inject_backspaces(*n) {
                                            log::error!("Failed to inject backspaces: {}", e);
                                        }
                                    }
                                }
                            }
                            if !formatted.is_empty() {
                                if let Err(e) = injector
                                    .inject_with_config(&formatted, &config.formatting)
                                    .await
                                {
                                    log::error!("Failed to inject text: {}", e);
                                } else {
                                    last_injected_len.store(formatted.len(), Ordering::SeqCst);
                                    *last_injected_text.lock().await = formatted;
                                }
                            }
                        }
                    }
                }
                Ok(Ok(Err(e))) => {
                    log::error!("Transcription failed: {}", e);
                    emit_overlay_event(
                        &app_handle,
                        OverlayEvent::Error {
                            message: "Transcription failed".to_string(),
                        },
                    );
                }
                Ok(Err(_)) => {
                    log::error!("Transcription thread failed or panicked");
                    emit_overlay_event(
                        &app_handle,
                        OverlayEvent::Error {
                            message: "Thread failed".to_string(),
                        },
                    );
                }
                Err(_) => {
                    log::warn!("Transcription timed out after 120s");
                    emit_overlay_event(
                        &app_handle,
                        OverlayEvent::Error {
                            message: "Timed out".to_string(),
                        },
                    );
                }
            }

            let mut audio_state = audio_state_ref.lock().await;
            *audio_state = AudioState::Idle;
            let _ = crate::tray::TrayManager::set_tray_state(&app_handle, AudioState::Idle);

            // No Success UI: go straight to Collapsed after brief delay
            let app_for_overlay = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                reset_overlay_state(&app_for_overlay);
            });
        });
    } else {
        log::debug!("Cannot stop dictation, current state: {:?}", *audio_state);
    }
}

async fn toggle_dictation(state: tauri::State<'_, AppState>, is_recording: Arc<AtomicBool>) {
    let current_state = *state.audio_state.lock().await;

    match current_state {
        AudioState::Idle => {
            start_dictation(state, is_recording).await;
        }
        AudioState::Recording => {
            stop_dictation(state, is_recording).await;
        }
        AudioState::Processing => {
            log::info!("Dictation already processing, ignoring...");
        }
    }
}
