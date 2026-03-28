mod app_info;
pub mod app_log;
pub mod audio;
mod commands;
mod config;
mod db;
#[cfg(feature = "dev-bridge")]
mod dev_bridge;
mod formatting;
mod history;
mod hotkey;
#[cfg(windows)]
mod hotkey_win;
mod injection;
mod models;
mod notifications;
#[cfg(windows)]
mod overlay_message_log_win;
pub mod stt;
mod system_reqs;
mod tray;

use chrono::TimeZone;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio_util::sync::CancellationToken;

/// App icon for window and taskbar (used in setup so dev and production show the same icon).
const WINDOW_ICON: tauri::image::Image<'static> = tauri::include_image!("icons/32x32.png");

use crate::audio::vad::VADConfig;
use crate::audio::{play_sound, AudioState};
use crate::config::STTConfig;
use crate::config::STTMode;
use crate::config::{AppConfig, ConfigManager, UpdateChannel};
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

/// Run counter for each "Listening" emit (cold vs warm: run 1 = first after process start, run 2+ = subsequent).
static LISTENING_EMIT_RUN: AtomicUsize = AtomicUsize::new(0);

/// When KALAM_LATENCY_DEBUG=1 or "true", append a timestamped label to ~/.kalam/latency-trace.log and flush.
/// Uses microsecond precision. Bypasses the log pipeline so timing isn't affected by log level or buffering.
/// Pub(crate) so hotkey_win can log OS_key_down for "before T0" latency testing.
pub(crate) fn latency_trace_write(label: &str) {
    if std::env::var("KALAM_LATENCY_DEBUG").as_deref() != Ok("1")
        && std::env::var("KALAM_LATENCY_DEBUG").as_deref() != Ok("true")
    {
        return;
    }
    let Ok(dir) = crate::config::get_kalam_dir() else {
        return;
    };
    let path = dir.join("latency-trace.log");
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    let line = format!("{}\t{}\n", ts, label);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        use std::io::Write;
        let _ = f.write_all(line.as_bytes());
        let _ = f.flush();
    }
}

/// Called from frontend (overlay) to record a trace point with the JS-side timestamp for correlation.
/// Writes: rust_micros\tlabel\tjs_micros so we can compute IPC/rendering deltas.
#[tauri::command]
fn trace_latency(event: String, js_timestamp: Option<f64>) {
    if std::env::var("KALAM_LATENCY_DEBUG").as_deref() != Ok("1")
        && std::env::var("KALAM_LATENCY_DEBUG").as_deref() != Ok("true")
    {
        return;
    }
    let Ok(dir) = crate::config::get_kalam_dir() else {
        return;
    };
    let path = dir.join("latency-trace.log");
    let rust_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    let line = match js_timestamp {
        Some(js_us) => format!("{}\t{}\t{}\n", rust_ts, event, js_us as u64),
        None => format!("{}\t{}\n", rust_ts, event),
    };
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        use std::io::Write;
        let _ = f.write_all(line.as_bytes());
        let _ = f.flush();
    }
}

/// Start logging Win32 messages for the overlay window (Windows only). Log file: ~/.kalam/overlay-messages.log
#[cfg(windows)]
#[tauri::command]
fn start_overlay_message_log(app: tauri::AppHandle) -> Result<(), String> {
    crate::overlay_message_log_win::start(&app)
}

#[cfg(not(windows))]
#[tauri::command]
fn start_overlay_message_log(_app: tauri::AppHandle) -> Result<(), String> {
    Err("Overlay message log is only available on Windows".to_string())
}

/// Stop the overlay message log.
#[cfg(windows)]
#[tauri::command]
fn stop_overlay_message_log() -> Result<(), String> {
    crate::overlay_message_log_win::stop()
}

#[cfg(not(windows))]
#[tauri::command]
fn stop_overlay_message_log() -> Result<(), String> {
    Err("Overlay message log is only available on Windows".to_string())
}

/// Start overlay message log and auto-stop after the given seconds. E.g. invoke with { seconds: 15 }, then hold the hotkey.
#[tauri::command]
async fn start_overlay_message_log_for_seconds(
    app: tauri::AppHandle,
    seconds: u64,
) -> Result<(), String> {
    #[cfg(windows)]
    crate::overlay_message_log_win::start(&app)?;
    #[cfg(not(windows))]
    let _ = app;
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(seconds)).await;
        #[cfg(windows)]
        let _ = crate::overlay_message_log_win::stop();
    });
    Ok(())
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
    /// On Windows: lowercase process filename (e.g. "notepad.exe") of the foreground window at recording start.
    pub foreground_process_name: Arc<Mutex<Option<String>>>,
    /// Full filesystem path to the foreground executable when available (used for friendly name + icon resolution).
    pub foreground_exe_path: Arc<Mutex<Option<String>>>,
    pub press_start_time: Arc<Mutex<Option<std::time::Instant>>>,
    pub local_model_manager: Arc<crate::stt::lifecycle::LocalModelManager>,
    /// Set when starting recording: Dictation (main hotkey) or Command (command hotkey). Read in stop_dictation to decide inject vs command pipeline.
    pub recording_type: Arc<Mutex<RecordingType>>,
    /// Whether the overlay OS window is expanded (300x120) or collapsed (80x80). Used by update_overlay_position.
    pub overlay_expanded: Arc<AtomicBool>,
    /// Cancellation token for the current transcription run. Replaced at start of each run; cancel_transcription cancels it.
    pub transcription_cancel: Arc<RwLock<CancellationToken>>,
    /// Hybrid/Auto forced Local due to sensitive app patterns; drives overlay amber UI for this dictation session.
    pub is_sensitive_app_active: Arc<AtomicBool>,
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
            foreground_process_name: Arc::new(Mutex::new(None)),
            foreground_exe_path: Arc::new(Mutex::new(None)),
            press_start_time: Arc::new(Mutex::new(None)),
            local_model_manager,
            recording_type: Arc::new(Mutex::new(RecordingType::Dictation)),
            overlay_expanded: Arc::new(AtomicBool::new(false)),
            transcription_cancel: Arc::new(RwLock::new(CancellationToken::new())),
            is_sensitive_app_active: Arc::new(AtomicBool::new(false)),
        })
    }
}

async fn cancel_dictation(state: tauri::State<'_, AppState>, is_recording: Arc<AtomicBool>) {
    log::info!("Cancelling dictation (short press or interrupted)");
    let mut audio_state = state.audio_state.lock().await;
    let should_cancel = matches!(*audio_state, AudioState::Starting | AudioState::Recording);
    let was_recording = matches!(*audio_state, AudioState::Recording);
    if should_cancel {
        *audio_state = AudioState::Idle;
    }
    drop(audio_state);

    if !should_cancel {
        return;
    }

    is_recording.store(false, Ordering::SeqCst);
    state.is_sensitive_app_active.store(false, Ordering::SeqCst);
    let _ = crate::tray::TrayManager::set_tray_state(&state.app_handle, AudioState::Idle);
    // Stop active stream only when recording was fully started. During Starting, start_dictation
    // performs cleanup if cancellation happened after stream startup but before final transition.
    if was_recording {
        let _ = state.audio_capture.lock().await.stop_recording().await;
    }
    emit_overlay_event(&state.app_handle, OverlayEvent::ShortPress);
    let app_for_overlay = state.app_handle.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
        reset_overlay_state(&app_for_overlay);
    });
}

fn create_registrations(
    app_handle: &tauri::AppHandle,
    is_recording_flag: Arc<AtomicBool>,
    hold_hotkey_str: &Option<String>,
    toggle_hotkey_str: Option<String>,
    language_toggle_hotkey: Option<String>,
    command_hotkey_str: Option<String>,
    recording_mode: Option<crate::config::RecordingMode>,
) -> Vec<HotkeyRegistration> {
    let mut registrations: Vec<HotkeyRegistration> = Vec::new();

    // None = legacy / default: register both hold and toggle when configured.
    let hold_ref: &Option<String> = match recording_mode {
        Some(crate::config::RecordingMode::Toggle) => &None,
        _ => hold_hotkey_str,
    };
    let toggle_effective: Option<String> = match recording_mode {
        Some(crate::config::RecordingMode::Hold) => None,
        _ => toggle_hotkey_str,
    };

    if let Some(hotkey_str) = hold_ref {
        if !hotkey_str.trim().is_empty() {
            if let Ok(target_hotkey) = parse_rdev_hotkey(hotkey_str) {
                let app_handle_press = app_handle.clone();
                let is_recording_press = is_recording_flag.clone();
                let app_handle_release = app_handle.clone();
                let is_recording_release = is_recording_flag.clone();
                // Record press time in sync callback so hold duration uses real key-down time, not when the async task runs.
                let hold_press_instant: Arc<std::sync::Mutex<Option<std::time::Instant>>> =
                    Arc::new(std::sync::Mutex::new(None));

                registrations.push(HotkeyRegistration {
                    target: target_hotkey,
                    active: Arc::new(AtomicBool::new(false)),
                    pending_activation: Arc::new(AtomicBool::new(false)),
                    is_dictation: true,
                    on_press: Arc::new({
                        let hold_press_instant_press = hold_press_instant.clone();
                        move || {
                            let _ = hold_press_instant_press.lock().map(|mut g| *g = Some(std::time::Instant::now()));
                            latency_trace_write("T0");
                            latency_trace_write("hold_callback_invoked");
                            log::info!("Hold hotkey pressed - callback invoked");
                            let app_handle = app_handle_press.clone();
                            let is_recording = is_recording_press.clone();
                            let hold_press_instant_async = hold_press_instant_press.clone();
                            tauri::async_runtime::spawn(async move {
                                latency_trace_write("T1");
                                latency_trace_write("hold_spawn_started");
                                let state = app_handle.state::<AppState>();
                                let press_instant = hold_press_instant_async.lock().ok().and_then(|mut g| g.take());
                                if let Some(instant) = press_instant {
                                    *state.press_start_time.lock().await = Some(instant);
                                }
                                let dictation_enabled = {
                                    let config = state.config.lock().await;
                                    let out = config.get_all().dictation_enabled;
                                    latency_trace_write("hold_config_acquired");
                                    out
                                };
                                if !dictation_enabled {
                                    return;
                                }
                                *state.recording_type.lock().await = RecordingType::Dictation;
                                start_dictation(state, is_recording).await;
                            });
                        }
                    }),
                    on_release: Arc::new(move || {
                        let app_handle = app_handle_release.clone();
                        let is_recording = is_recording_release.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app_handle.state::<AppState>();
                            let (dictation_enabled, min_hold_ms) = {
                                let config = state.config.lock().await;
                                let cfg = config.get_all();
                                (cfg.dictation_enabled, cfg.min_hold_ms)
                            };
                            if !dictation_enabled {
                                return;
                            }
                            let mut is_short_press = false;
                            if let Some(start_time) = state.press_start_time.lock().await.take() {
                                let elapsed_ms = start_time.elapsed().as_millis();
                                if elapsed_ms < min_hold_ms as u128 {
                                    is_short_press = true;
                                    log::info!("Hold release: elapsed {} ms < min_hold_ms {} → short press (cancel)", elapsed_ms, min_hold_ms);
                                    latency_trace_write(&format!("hold_release_short_elapsed_{}_min_{}", elapsed_ms, min_hold_ms));
                                } else {
                                    latency_trace_write(&format!("hold_release_long_elapsed_{}_min_{}", elapsed_ms, min_hold_ms));
                                }
                            } else {
                                log::warn!("Hold release: no press_start_time (release raced or missed) → treating as long press");
                                latency_trace_write("hold_release_no_start_time");
                            }
                            if is_short_press {
                                cancel_dictation(state, is_recording).await;
                            } else {
                                stop_dictation(state, is_recording).await;
                            }
                        });
                    }),
                    on_cancel: Some(Arc::new({
                        let app_handle_cancel = app_handle.clone();
                        let is_recording_cancel = is_recording_flag.clone();
                        move || {
                            let app_handle = app_handle_cancel.clone();
                            let is_recording = is_recording_cancel.clone();
                            tauri::async_runtime::spawn(async move {
                                let state = app_handle.state::<AppState>();
                                cancel_dictation(state, is_recording).await;
                            });
                        }
                    })),
                });
                log::info!("Hold dictation hotkey registered: {}", hotkey_str);
            } else {
                log::error!("Failed to parse hold hotkey: {}", hotkey_str);
            }
        }
    }

    if let Some(ref toggle_str) = toggle_effective {
        if !toggle_str.trim().is_empty() {
            if let Ok(toggle_hotkey) = parse_rdev_hotkey(toggle_str) {
                let app_handle_press = app_handle.clone();
                let is_recording_press = is_recording_flag.clone();

                registrations.push(HotkeyRegistration {
                    target: toggle_hotkey,
                    active: Arc::new(AtomicBool::new(false)),
                    pending_activation: Arc::new(AtomicBool::new(false)),
                    is_dictation: true,
                    on_press: Arc::new(move || {
                        latency_trace_write("T0");
                        latency_trace_write("toggle_callback_invoked");
                        log::info!("Toggle hotkey pressed - callback invoked");
                        let app_handle = app_handle_press.clone();
                        let is_recording = is_recording_press.clone();
                        tauri::async_runtime::spawn(async move {
                            latency_trace_write("T1");
                            latency_trace_write("toggle_spawn_started");
                            let state = app_handle.state::<AppState>();
                            let dictation_enabled = {
                                let config = state.config.lock().await;
                                let out = config.get_all().dictation_enabled;
                                latency_trace_write("toggle_config_acquired");
                                out
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
                    on_cancel: None,
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
                registrations.push(HotkeyRegistration {
                    target: toggle_hotkey,
                    active: Arc::new(AtomicBool::new(false)),
                    pending_activation: Arc::new(AtomicBool::new(false)),
                    is_dictation: false,
                    on_press: Arc::new(move || {
                        let app_handle = app_handle_toggle.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app_handle.state::<AppState>();
                            let mut config_mgr = state.config.lock().await;
                            let mut cfg = config_mgr.get_all();
                            if !cfg.dictation_enabled {
                                return;
                            }
                            if cfg.languages.len() >= 2 {
                                cfg.languages.swap(0, 1);
                                let label = language_display_name(&cfg.languages[0]);
                                // Clone so we can emit the same payload the UI expects from save_settings
                                // (StatusBar reads active language from config, not only sidebarDictationStore).
                                let updated = cfg.clone();
                                if config_mgr.save(cfg).is_ok() {
                                    drop(config_mgr);
                                    let _ = app_handle.emit("settings_updated", &updated);
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
                    on_cancel: None,
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
            let app_handle_release = app_handle.clone();
            let is_recording_release = is_recording_flag.clone();
            let cmd_press_instant: Arc<std::sync::Mutex<Option<std::time::Instant>>> =
                Arc::new(std::sync::Mutex::new(None));

            registrations.push(HotkeyRegistration {
                target: cmd_target,
                active: Arc::new(AtomicBool::new(false)),
                pending_activation: Arc::new(AtomicBool::new(false)),
                is_dictation: false,
                    on_press: Arc::new({
                        let cmd_press_instant_press = cmd_press_instant.clone();
                        move || {
                            let _ = cmd_press_instant_press.lock().map(|mut g| *g = Some(std::time::Instant::now()));
                            latency_trace_write("T0");
                            latency_trace_write("command_callback_invoked");
                            log::info!("Command hotkey pressed");
                            let app_handle = app_handle_press.clone();
                            let is_recording = is_recording_press.clone();
                            let cmd_press_instant_async = cmd_press_instant_press.clone();
                            tauri::async_runtime::spawn(async move {
                                latency_trace_write("T1");
                                latency_trace_write("command_spawn_started");
                                let state = app_handle.state::<AppState>();
                                let press_instant = cmd_press_instant_async.lock().ok().and_then(|mut g| g.take());
                                if let Some(instant) = press_instant {
                                    *state.press_start_time.lock().await = Some(instant);
                                }
                                let (dictation_enabled, cmd_enabled) = {
                                    let config = state.config.lock().await;
                                    let cfg = config.get_all();
                                    let out = (cfg.dictation_enabled, cfg.command_config.enabled);
                                    latency_trace_write("command_config_acquired");
                                    out
                                };
                                if !dictation_enabled || !cmd_enabled {
                                    return;
                                }
                                *state.recording_type.lock().await = RecordingType::Command;
                                start_dictation(state, is_recording).await;
                            });
                        }
                    }),
                on_release: Arc::new(move || {
                    let app_handle = app_handle_release.clone();
                    let is_recording = is_recording_release.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app_handle.state::<AppState>();
                        let (dictation_enabled, cmd_enabled, min_hold_ms) = {
                            let config = state.config.lock().await;
                            let cfg = config.get_all();
                            (cfg.dictation_enabled, cfg.command_config.enabled, cfg.min_hold_ms)
                        };
                        if !dictation_enabled || !cmd_enabled {
                            return;
                        }
                        let mut is_short_press = false;
                        if let Some(start_time) = state.press_start_time.lock().await.take() {
                            let elapsed_ms = start_time.elapsed().as_millis();
                            if elapsed_ms < min_hold_ms as u128 {
                                is_short_press = true;
                                log::info!("Command release: elapsed {} ms < min_hold_ms {} → short press (cancel)", elapsed_ms, min_hold_ms);
                                latency_trace_write(&format!("command_release_short_elapsed_{}_min_{}", elapsed_ms, min_hold_ms));
                            } else {
                                latency_trace_write(&format!("command_release_long_elapsed_{}_min_{}", elapsed_ms, min_hold_ms));
                            }
                        } else {
                            log::warn!("Command release: no press_start_time → treating as long press");
                            latency_trace_write("command_release_no_start_time");
                        }
                        if is_short_press {
                            cancel_dictation(state, is_recording).await;
                        } else {
                            stop_dictation(state, is_recording).await;
                        }
                    });
                }),
                on_cancel: Some(Arc::new({
                    let app_handle_cancel = app_handle.clone();
                    let is_recording_cancel = is_recording_flag.clone();
                    move || {
                        let app_handle = app_handle_cancel.clone();
                        let is_recording = is_recording_cancel.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app_handle.state::<AppState>();
                            cancel_dictation(state, is_recording).await;
                        });
                    }
                })),
            });
            log::info!("Command mode hotkey registered: {}", cmd_hotkey_str);
        } else {
            log::warn!("Failed to parse command hotkey: {}", cmd_hotkey_str);
        }
    }

    registrations
}

/// Sync OS login-item state with the `auto_start` config flag.
/// Skipped in debug builds so we do not register `target/debug/...` as a login item.
fn sync_autostart(app: &tauri::AppHandle, auto_start: bool) {
    if cfg!(debug_assertions) {
        log::info!(
            "Dev build: skipping autostart sync (auto_start={})",
            auto_start
        );
        return;
    }
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    let result = if auto_start {
        manager.enable()
    } else {
        manager.disable()
    };
    match result {
        Ok(_) => log::info!(
            "Autostart {}.",
            if auto_start { "enabled" } else { "disabled" }
        ),
        Err(e) => log::error!("Failed to sync autostart: {}", e),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Dev-only: HTTP bridge so browser (Vite dev) can read DB when Tauri app is running
    #[cfg(feature = "dev-bridge")]
    {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().expect("dev-bridge tokio runtime");
            rt.block_on(dev_bridge::run());
        });
    }

    let builder = {
        let b = tauri::Builder::default()
            .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(e) = window.show() {
                        log::warn!("Single-instance: failed to show main window: {}", e);
                    }
                    if let Err(e) = window.set_focus() {
                        log::warn!("Single-instance: failed to focus main window: {}", e);
                    }
                }
            }))
            .plugin(tauri_plugin_notification::init())
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_opener::init())
            .plugin(
                tauri_plugin_autostart::Builder::new()
                    .app_name("Kalam")
                    .build(),
            )
            .on_window_event(|window, event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    if window.label() == "main" {
                        window.hide().unwrap();
                        api.prevent_close();
                    }
                }
            });
        #[cfg(windows)]
        let b = b.device_event_filter(tauri::DeviceEventFilter::Always);
        b
    };
    builder
        .setup(|app| {
            // Initialize app state (first use of config; logger already set in main)
            let state = AppState::new(app.handle().clone())?;

            // Apply saved logging config to in-app log buffer
            app_log::reconfigure(state.config.blocking_lock().get_all().logging.clone());

            // Get config for startup behavior (one lock for related flags)
            let (start_in_focus, auto_start, sound_on_background_start) = {
                let config = state.config.blocking_lock();
                let c = config.get_all();
                (
                    c.start_in_focus,
                    c.auto_start,
                    c.notifications.sound_enabled,
                )
            };
            sync_autostart(app.handle(), auto_start);

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
                    if sound_on_background_start {
                        if let Err(e) = play_sound(app.handle(), "started-in-background") {
                            log::warn!("Failed to play background start sound: {}", e);
                        }
                    }
                    log::info!("App started in background (minimized to tray)");
                }
            }

            // Clone what we need before managing state
            let is_recording_flag = state.is_recording.clone();

            // Manage state
            app.manage(state);

            // Pre-release builds: default update channel to Beta until the user picks a channel in About.
            {
                let state = app.state::<AppState>();
                if !app.package_info().version.pre.is_empty() {
                    let mut mgr = state.config.blocking_lock();
                    let mut cfg = mgr.get_all();
                    if !cfg.update_channel_locked && cfg.update_channel == UpdateChannel::Stable {
                        cfg.update_channel = UpdateChannel::Beta;
                        if let Err(e) = mgr.save(cfg) {
                            log::warn!("Failed to set default beta update channel: {}", e);
                        } else {
                            log::info!("Update channel set to Beta (pre-release build)");
                        }
                    }
                }
            }

            let retention_startup = app
                .state::<AppState>()
                .config
                .blocking_lock()
                .get_all()
                .privacy
                .history_retention_days;
            if let Err(e) = history::prune_history_by_retention(retention_startup) {
                log::warn!("History retention prune on startup failed: {}", e);
            }

            // Apply initial overlay position
            if let Err(e) = update_overlay_position(app.handle()) {
                log::warn!("Failed to set initial overlay position: {}", e);
            }

            // Set overlay window (and webview where supported) background to transparent.
            // On macOS the webview/window can default to white; this avoids the white box.
            if let Some(overlay) = app.get_webview_window(OVERLAY_LABEL) {
                use tauri::window::Color;
                let transparent = Some(Color(0, 0, 0, 0));
                if let Err(e) = overlay.as_ref().window().set_background_color(transparent) {
                    log::debug!("Overlay set_background_color: {}", e);
                }
            }

            // Track cursor to keep overlay on the correct monitor
            let cursor_tracking_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    let _ = update_overlay_position(&cursor_tracking_handle);
                }
            });

            // When focus enters a sensitive app (Hybrid/Auto + patterns), expand the pill briefly with a lock hint.
            let sensitive_peek_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut last_idle_sensitive: Option<bool> = None;
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(450)).await;
                    let state = sensitive_peek_handle.state::<AppState>();
                    let audio = *state.audio_state.lock().await;
                    if !matches!(audio, crate::audio::AudioState::Idle) {
                        continue;
                    }
                    let cfg = match state.config.try_lock() {
                        Ok(g) => g.get_all(),
                        Err(_) => continue,
                    };
                    if !cfg.dictation_enabled {
                        continue;
                    }
                    let now = crate::config::privacy::foreground_matches_sensitive_app(&cfg);
                    let prev = last_idle_sensitive.unwrap_or(false);
                    if now && !prev {
                        emit_overlay_event(&sensitive_peek_handle, OverlayEvent::SensitiveAppPeek);
                    }
                    last_idle_sensitive = Some(now);
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
            std::thread::spawn(move || loop {
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
                    emit_overlay_event(
                        &hotplug_handle,
                        OverlayEvent::Error {
                            message: "Microphone disconnected. Switched to system default."
                                .to_string(),
                        },
                    );
                }
            });

            // Register global hotkeys via rdev (dictation + optional language toggle + optional command mode)
            let (
                hotkey_str,
                toggle_dictation_hotkey,
                language_toggle_hotkey,
                command_hotkey_str,
                recording_mode,
            ) = {
                let state = app.state::<AppState>();
                let config = state.config.blocking_lock();
                let cfg = config.get_all();
                let cmd_hk = cfg
                    .command_config
                    .enabled
                    .then(|| cfg.command_config.hotkey.clone())
                    .flatten()
                    .filter(|s| !s.trim().is_empty());
                (
                    cfg.hotkey.clone(),
                    cfg.toggle_dictation_hotkey.clone(),
                    cfg.language_toggle_hotkey.clone(),
                    cmd_hk,
                    cfg.recording_mode,
                )
            };

            let registrations = create_registrations(
                app.handle(),
                is_recording_flag.clone(),
                &hotkey_str,
                toggle_dictation_hotkey,
                language_toggle_hotkey,
                command_hotkey_str,
                recording_mode,
            );

            if !registrations.is_empty() {
                start_listener(registrations);
            }

            log::info!("Kalam initialized successfully");

            // Resize overlay to collapsed (80x80) so it does not block clicks, then show it
            if let Err(e) = resize_overlay(app.handle().clone(), false) {
                log::warn!("Failed to resize overlay on startup: {}", e);
            }
            if let Err(e) = show_overlay(app.handle()) {
                log::warn!("Failed to show overlay on startup: {}", e);
            }
            reset_overlay_state(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_platform,
            get_os_release_info,
            get_app_icon,
            resolve_target_app,
            set_hotkeys_paused,
            get_app_version,
            check_for_updates,
            download_and_install_update,
            request_system_permission,
            open_system_permission_page,
            get_permission_status,
            get_runtime_capabilities,
            get_settings,
            save_settings,
            skip_onboarding_with_defaults,
            get_audio_devices,
            test_microphone_start,
            test_microphone_level,
            test_microphone_stop,
            get_history,
            search_history,
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
            get_logging_state,
            save_log_to_file,
            save_logs_csv_to_file,
            get_app_data_path,
            open_app_data_folder,
            resize_overlay,
            get_overlay_initial_state,
            cancel_transcription,
            ui_toggle_dictation,
            trace_latency,
            start_overlay_message_log,
            stop_overlay_message_log,
            start_overlay_message_log_for_seconds,
            commands::export_logs_csv,
            commands::create_entry,
            commands::get_entries_by_type,
            commands::get_entries_with_reminder,
            commands::get_entries_for_reminders,
            commands::get_aggregate_stats,
            commands::get_dashboard_stats,
            commands::get_daily_stats,
            commands::get_tasks_due_on,
            commands::get_reminders_due_on,
            commands::get_entry,
            commands::update_entry,
            commands::delete_entry,
            commands::search_notes,
            commands::get_note_labels,
            commands::get_note_scope_counts,
            commands::empty_trash,
            commands::empty_task_trash,
            commands::save_attachment,
            commands::search_similar,
            commands::get_dictionary_entries,
            commands::add_dictionary_entry,
            commands::delete_dictionary_entry,
            commands::update_dictionary_entry,
            focus_main_window,
            reset_application,
            check_model_requirements,
            start_local_model,
            stop_local_model,
            stop_all_local_models,
            restart_local_model,
            delete_local_model,
            uninstall_sidecar,
            is_sidecar_installed_for_model,
            is_sidecar_available_for_model,
            commands::fetch_llm_models,
            commands::generate_structured_data,
            commands::test_llm_model,
            get_running_apps,
            pick_executable_file,
            get_installed_apps,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {
            // RunEvent::Reopen (macOS dock click to show window) is not in tauri 2.10 RunEvent.
            // Hide-on-close above fixes tray menu and taskbar restore on Windows.
        });
}

const GITHUB_RELEASES_API: &str = "https://api.github.com/repos/afaraha8403/kalam/releases";
const GITHUB_STABLE_LATEST_URL: &str =
    "https://github.com/afaraha8403/kalam/releases/latest/download/latest.json";

/// Resolve the update endpoint URL for the given channel. Beta uses GitHub API to find latest prerelease.
async fn get_update_endpoint_for_channel(channel: UpdateChannel) -> anyhow::Result<String> {
    match channel {
        UpdateChannel::Stable => Ok(GITHUB_STABLE_LATEST_URL.to_string()),
        UpdateChannel::Beta => {
            let client = reqwest::Client::builder()
                .user_agent("Kalam-Updater/1.0")
                .build()?;
            let releases: Vec<serde_json::Value> = client
                .get(GITHUB_RELEASES_API)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            let tag_name = releases
                .iter()
                .find(|r| r["prerelease"].as_bool() == Some(true))
                .and_then(|r| r["tag_name"].as_str())
                .map(String::from)
                .ok_or_else(|| anyhow::anyhow!("no prerelease found"))?;
            Ok(format!(
                "https://github.com/afaraha8403/kalam/releases/download/{}/latest.json",
                tag_name
            ))
        }
    }
}

/// Check for updates using the channel from app config (stable or beta).
async fn check_update_with_channel(
    app: &tauri::AppHandle,
) -> anyhow::Result<Option<tauri_plugin_updater::Update>> {
    let channel = app
        .state::<AppState>()
        .config
        .lock()
        .await
        .get_all()
        .update_channel
        .clone();
    let endpoint = get_update_endpoint_for_channel(channel).await?;
    let url = url::Url::parse(&endpoint).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let updater = app
        .updater_builder()
        .endpoints(vec![url])
        .map_err(|e| anyhow::anyhow!("{:?}", e))?
        .build()
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    updater
        .check()
        .await
        .map_err(|e| anyhow::anyhow!("{:?}", e))
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
    let update = check_update_with_channel(app).await?;
    if let Some(u) = update {
        // Install is from Settings → About (Update now / Update on next start), not tray-only restart.
        let msg = format!(
            "Update {} available. Open Settings → About to download and install.",
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
    let show_errors = app
        .state::<AppState>()
        .config
        .lock()
        .await
        .get_all()
        .notifications
        .show_errors;
    let nm = &app.state::<AppState>().notification_manager;
    let update = match check_update_with_channel(app).await {
        Ok(u) => u,
        Err(e) => {
            if show_errors {
                let _ = nm.error("Could not check for updates.");
            }
            return Err(e);
        }
    };
    if let Some(u) = update {
        let msg = format!(
            "Update {} available. Open Settings → About to download and install.",
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

/// Human-readable OS name and version for onboarding / support context (sysinfo).
#[derive(serde::Serialize)]
pub struct OsReleaseInfo {
    pub name: String,
    pub version: String,
}

pub(crate) fn read_os_release_info() -> OsReleaseInfo {
    use sysinfo::System;
    let name = System::name()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| std::env::consts::OS.to_string());
    let version = System::long_os_version()
        .filter(|s| !s.is_empty())
        .or_else(System::os_version)
        .unwrap_or_default();
    OsReleaseInfo { name, version }
}

#[tauri::command]
fn get_os_release_info() -> OsReleaseInfo {
    read_os_release_info()
}

/// Base64-encoded PNG for `applications.icon_png`, or None if not cached.
#[tauri::command]
fn get_app_icon(process_name: String) -> Result<Option<String>, String> {
    use base64::Engine;
    let conn = crate::db::open_db().map_err(|e| e.to_string())?;
    let normalized = crate::app_info::normalize_process_name(&process_name);
    let icon: Option<Vec<u8>> = conn
        .query_row(
            "SELECT icon_png FROM applications WHERE process_name = ?1",
            [&normalized],
            |row| row.get(0),
        )
        .ok()
        .flatten();
    Ok(icon.map(|b| base64::engine::general_purpose::STANDARD.encode(b)))
}

/// Resolve display name and icon for a process (fills `applications` if missing).
#[tauri::command]
fn resolve_target_app(process_name: String) -> Result<(String, Option<String>), String> {
    use base64::Engine;
    let conn = crate::db::open_db().map_err(|e| e.to_string())?;
    let (name, icon) = crate::db::get_or_resolve_application(&conn, &process_name, None)
        .map_err(|e| e.to_string())?;
    let icon_b64 = icon.map(|b| base64::engine::general_purpose::STANDARD.encode(b));
    Ok((name, icon_b64))
}

/// Information about a running or installed application for the "sensitive apps" picker.
#[derive(serde::Serialize, Clone)]
pub struct AppListEntry {
    pub process_name: String,
    pub display_name: String,
    pub icon_base64: Option<String>,
    pub exe_path: Option<String>,
}

/// List currently running processes with their friendly names and icons.
/// Excludes system processes and the current app to keep the list useful.
#[tauri::command]
fn get_running_apps() -> Result<Vec<AppListEntry>, String> {
    use base64::Engine;
    use std::collections::HashSet;
    use sysinfo::{ProcessesToUpdate, System};

    let mut sys = System::new_all();
    sys.refresh_processes(ProcessesToUpdate::All);

    let conn = crate::db::open_db().map_err(|e| e.to_string())?;
    let current_pid = std::process::id();

    // Collect unique process names, skipping system processes and this app
    let mut seen: HashSet<String> = HashSet::new();
    let mut results: Vec<AppListEntry> = Vec::new();

    for (pid, process) in sys.processes() {
        // Skip current app
        if pid.as_u32() == current_pid {
            continue;
        }

        let exe_path = process.exe().map(|p| p.to_string_lossy().to_string());
        let process_name = process.name().to_string_lossy().to_string().to_lowercase();

        // Skip if we've seen this normalized name
        let normalized = crate::app_info::normalize_process_name(&process_name);
        if !seen.insert(normalized.clone()) {
            continue;
        }

        // Skip common system processes
        let system_procs: HashSet<&str> = [
            "svchost.exe",
            "csrss.exe",
            "smss.exe",
            "services.exe",
            "lsass.exe",
            "wininit.exe",
            "winlogon.exe",
            "explorer.exe",
            "taskhostw.exe",
            "kernel",
            "registry",
            "system interrupts",
            "system",
            "launchd",
            "kernel_task",
            "windowserver",
            "dock",
            "finder",
            "systemd",
            "kthreadd",
            "init",
            "bash",
            "sh",
            "zsh",
        ]
        .iter()
        .cloned()
        .collect();
        if system_procs.contains(normalized.as_str()) {
            continue;
        }

        // Get display name and icon from DB/cache
        let (display_name, icon_opt) =
            crate::db::get_or_resolve_application(&conn, &process_name, exe_path.as_deref())
                .unwrap_or_else(|_| (crate::app_info::capitalize_process_name(&normalized), None));

        let icon_base64 = icon_opt.map(|b| base64::engine::general_purpose::STANDARD.encode(b));

        results.push(AppListEntry {
            process_name: normalized,
            display_name,
            icon_base64,
            exe_path,
        });
    }

    // Sort by display name for better UX
    results.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });

    // Limit to reasonable number for UI performance
    results.truncate(100);

    Ok(results)
}

/// Open a file picker to select an executable file. Returns the selected path or None if cancelled.
#[tauri::command]
async fn pick_executable_file() -> Result<Option<String>, String> {
    let picked = rfd::AsyncFileDialog::new()
        .set_title("Select an application executable")
        .add_filter("Executables", &["exe", "app", "AppImage", "bin"])
        .add_filter("All files", &["*"])
        .pick_file()
        .await;

    Ok(picked.map(|f| f.path().to_string_lossy().to_string()))
}

/// List installed applications (platform-specific).
/// Windows: common installation directories
/// macOS: /Applications folder
/// Linux: .desktop entries
#[tauri::command]
fn get_installed_apps() -> Result<Vec<AppListEntry>, String> {
    use base64::Engine;
    use std::collections::HashSet;

    let _conn = crate::db::open_db().map_err(|e| e.to_string())?;
    let mut seen: HashSet<String> = HashSet::new();
    let mut results: Vec<AppListEntry> = Vec::new();

    #[cfg(windows)]
    {
        // Common Windows installation paths
        let program_files = std::env::var("ProgramFiles").ok();
        let program_files_x86 = std::env::var("ProgramFiles(x86)").ok();
        let local_app_data = std::env::var("LocalAppData").ok();

        let mut scan_dirs: Vec<std::path::PathBuf> = Vec::new();
        if let Some(pf) = program_files {
            scan_dirs.push(std::path::PathBuf::from(pf));
        }
        if let Some(pf86) = program_files_x86 {
            scan_dirs.push(std::path::PathBuf::from(pf86));
        }
        if let Some(la) = local_app_data {
            scan_dirs.push(std::path::PathBuf::from(la));
        }

        for dir in scan_dirs {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        // Look for .exe files in subdirectories
                        if let Ok(sub_entries) = std::fs::read_dir(&path) {
                            for sub in sub_entries.flatten() {
                                let sub_path = sub.path();
                                if sub_path
                                    .extension()
                                    .map(|e| e.eq_ignore_ascii_case("exe"))
                                    .unwrap_or(false)
                                {
                                    let exe_path = sub_path.to_string_lossy().to_string();
                                    let process_name =
                                        crate::app_info::normalize_process_name(&exe_path);
                                    if !seen.insert(process_name.clone()) {
                                        continue;
                                    }

                                    if let Some(info) = crate::app_info::resolve(&exe_path) {
                                        let icon_base64 = info.icon_png.map(|b| {
                                            base64::engine::general_purpose::STANDARD.encode(b)
                                        });
                                        results.push(AppListEntry {
                                            process_name: info.process_name,
                                            display_name: info.display_name,
                                            icon_base64,
                                            exe_path: Some(exe_path),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let apps_dir = std::path::PathBuf::from("/Applications");
        if let Ok(entries) = std::fs::read_dir(&apps_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "app").unwrap_or(false) {
                    let exe_path = path.to_string_lossy().to_string();
                    let process_name = crate::app_info::normalize_process_name(&exe_path);
                    if !seen.insert(process_name.clone()) {
                        continue;
                    }

                    if let Some(info) = crate::app_info::resolve(&exe_path) {
                        let icon_base64 = info
                            .icon_png
                            .map(|b| base64::engine::general_purpose::STANDARD.encode(b));
                        results.push(AppListEntry {
                            process_name: info.process_name,
                            display_name: info.display_name,
                            icon_base64,
                            exe_path: Some(exe_path),
                        });
                    }
                }
            }
        }

        // Also check user Applications folder
        if let Ok(home) = std::env::var("HOME") {
            let user_apps = std::path::PathBuf::from(home).join("Applications");
            if let Ok(entries) = std::fs::read_dir(&user_apps) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "app").unwrap_or(false) {
                        let exe_path = path.to_string_lossy().to_string();
                        let process_name = crate::app_info::normalize_process_name(&exe_path);
                        if !seen.insert(process_name.clone()) {
                            continue;
                        }

                        if let Some(info) = crate::app_info::resolve(&exe_path) {
                            let icon_base64 = info
                                .icon_png
                                .map(|b| base64::engine::general_purpose::STANDARD.encode(b));
                            results.push(AppListEntry {
                                process_name: info.process_name,
                                display_name: info.display_name,
                                icon_base64,
                                exe_path: Some(exe_path),
                            });
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Scan desktop entry directories
        let desktop_dirs = [
            std::path::PathBuf::from("/usr/share/applications"),
            std::path::PathBuf::from("/usr/local/share/applications"),
            std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".local/share/applications"))
                .unwrap_or_default(),
        ];

        for dir in desktop_dirs {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            // Parse Exec line to get the binary name
                            let exec_line = content
                                .lines()
                                .find(|l| l.starts_with("Exec="))
                                .map(|l| l.trim_start_matches("Exec="));

                            if let Some(exec) = exec_line {
                                let exe_name = exec.split_whitespace().next().unwrap_or(exec);
                                let process_name =
                                    crate::app_info::normalize_process_name(exe_name);
                                if !seen.insert(process_name.clone()) {
                                    continue;
                                }

                                // Get Name from desktop entry
                                let name_line = content
                                    .lines()
                                    .find(|l| l.starts_with("Name="))
                                    .map(|l| l.trim_start_matches("Name=").to_string())
                                    .unwrap_or_else(|| {
                                        crate::app_info::capitalize_process_name(&process_name)
                                    });

                                // Try to get icon
                                let icon_name = content
                                    .lines()
                                    .find(|l| l.starts_with("Icon="))
                                    .map(|l| l.trim_start_matches("Icon="));

                                let icon_base64 = icon_name
                                    .and_then(|icon| {
                                        if std::path::Path::new(icon).is_absolute()
                                            && std::path::Path::new(icon).exists()
                                        {
                                            std::fs::read(icon).ok()
                                        } else {
                                            // Try to look up in icon theme cache
                                            use freedesktop_icon_lookup::Cache;
                                            let mut cache = Cache::new().ok()?;
                                            cache.load_default().ok()?;
                                            let _ = cache.load("Adwaita");
                                            let _ = cache.load("hicolor");
                                            let icon_path = cache.lookup(icon, None::<&str>)?;
                                            std::fs::read(icon_path).ok()
                                        }
                                    })
                                    .map(|b| base64::engine::general_purpose::STANDARD.encode(b));

                                results.push(AppListEntry {
                                    process_name,
                                    display_name: name_line,
                                    icon_base64,
                                    exe_path: Some(exe_name.to_string()),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort by display name
    results.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });

    Ok(results)
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
    let update = check_update_with_channel(&app)
        .await
        .map_err(|e| format!("{:?}", e))?;
    Ok(update.as_ref().map(|u| u.version.clone()))
}

/// Download and install the available update (uses channel from settings).
/// When `restart` is true, restarts the app immediately after install so the new version runs.
/// When `restart` is false, finishes without restarting—the update applies on the next app launch.
/// Returns Ok(()) on success. Err if no update or download/install failed.
#[tauri::command]
async fn download_and_install_update(app: tauri::AppHandle, restart: bool) -> Result<(), String> {
    let update = check_update_with_channel(&app)
        .await
        .map_err(|e| format!("{:?}", e))?
        .ok_or_else(|| "No update available.".to_string())?;
    let app_emit = app.clone();
    let mut downloaded: u64 = 0;
    update
        .download_and_install(
            move |chunk_len, total| {
                downloaded += chunk_len as u64;
                let percent = total.map(|t| (downloaded as f64 / t as f64) * 100.0);
                let _ = app_emit.emit("update-download-progress", (downloaded, total, percent));
            },
            || {},
        )
        .await
        .map_err(|e| format!("{:?}", e))?;
    if restart {
        app.restart();
    }
    Ok(())
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

#[derive(Debug, Clone, serde::Serialize)]
struct PermissionStatusItem {
    /// granted | needs_action | unknown
    state: String,
    actionable: bool,
    message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct PermissionStatusPayload {
    platform: String,
    microphone: PermissionStatusItem,
    accessibility: PermissionStatusItem,
    input_monitoring: PermissionStatusItem,
}

#[derive(Debug, Clone, serde::Serialize)]
struct RuntimeCapabilitiesPayload {
    /// Whether audio capture appears available right now.
    can_capture_audio: bool,
    /// Whether text injection into other apps is expected to work.
    can_text_inject: bool,
    /// Whether global hotkeys are expected to work.
    can_global_hotkey: bool,
    /// granted | needs_action | unknown
    capture_audio_state: String,
    /// granted | needs_action | unknown
    text_inject_state: String,
    /// granted | needs_action | unknown
    global_hotkey_state: String,
    /// Actionable guidance to unblock the user.
    next_steps: Vec<String>,
    permission_status: PermissionStatusPayload,
}

fn has_any_audio_input_device() -> bool {
    use cpal::traits::HostTrait;
    let host = cpal::default_host();
    if let Ok(mut devices) = host.input_devices() {
        if devices.next().is_some() {
            return true;
        }
    }
    host.default_input_device().is_some()
}

#[cfg(target_os = "macos")]
fn is_macos_accessibility_trusted() -> bool {
    use std::ffi::c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> i32;
    }

    let _ = std::mem::size_of::<*const c_void>();
    unsafe { AXIsProcessTrusted() != 0 }
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)] // Called by macOS-only status checks; non-mac builds keep a stub for shared call sites.
fn is_macos_accessibility_trusted() -> bool {
    false
}

#[tauri::command]
fn get_permission_status() -> PermissionStatusPayload {
    #[cfg(target_os = "macos")]
    {
        let has_mic_device = has_any_audio_input_device();
        let accessibility_trusted = is_macos_accessibility_trusted();
        return PermissionStatusPayload {
            platform: "macos".to_string(),
            microphone: PermissionStatusItem {
                state: if has_mic_device {
                    "unknown".to_string()
                } else {
                    "needs_action".to_string()
                },
                actionable: true,
                message: if has_mic_device {
                    "macOS shows the microphone prompt when recording starts. Run a mic test to confirm access."
                        .to_string()
                } else {
                    "No input device detected. Connect or enable a microphone, then retry.".to_string()
                },
            },
            accessibility: PermissionStatusItem {
                state: if accessibility_trusted {
                    "granted".to_string()
                } else {
                    "needs_action".to_string()
                },
                actionable: !accessibility_trusted,
                message: if accessibility_trusted {
                    "Accessibility is enabled.".to_string()
                } else {
                    "Enable Accessibility so Kalam can insert text into other apps.".to_string()
                },
            },
            input_monitoring: PermissionStatusItem {
                state: "unknown".to_string(),
                actionable: true,
                message: "Input Monitoring is usually prompted when global shortcuts are captured. Test your hotkey in another app to confirm."
                    .to_string(),
            },
        };
    }

    #[cfg(windows)]
    {
        let has_mic_device = has_any_audio_input_device();
        PermissionStatusPayload {
            platform: "windows".to_string(),
            microphone: PermissionStatusItem {
                state: if has_mic_device {
                    "unknown".to_string()
                } else {
                    "needs_action".to_string()
                },
                actionable: true,
                message: if has_mic_device {
                    "If recording fails, check Windows Privacy > Microphone access for this app."
                        .to_string()
                } else {
                    "No input device detected. Connect or enable a microphone, then retry."
                        .to_string()
                },
            },
            accessibility: PermissionStatusItem {
                state: "granted".to_string(),
                actionable: false,
                message: "No separate accessibility toggle is usually required on Windows."
                    .to_string(),
            },
            input_monitoring: PermissionStatusItem {
                state: "granted".to_string(),
                actionable: false,
                message: "No separate Input Monitoring permission is required on Windows."
                    .to_string(),
            },
        }
    }

    #[cfg(target_os = "linux")]
    {
        let has_mic_device = has_any_audio_input_device();
        PermissionStatusPayload {
            platform: "linux".to_string(),
            microphone: PermissionStatusItem {
                state: if has_mic_device {
                    "unknown".to_string()
                } else {
                    "needs_action".to_string()
                },
                actionable: true,
                message: if has_mic_device {
                    "Linux permissions vary by distribution and audio stack. Run mic test to confirm capture."
                        .to_string()
                } else {
                    "No input device detected. Check PipeWire/PulseAudio and device settings."
                        .to_string()
                },
            },
            accessibility: PermissionStatusItem {
                state: "unknown".to_string(),
                actionable: true,
                message: "Linux accessibility/injection support varies by desktop environment."
                    .to_string(),
            },
            input_monitoring: PermissionStatusItem {
                state: "unknown".to_string(),
                actionable: true,
                message: "Global hotkey support varies by desktop environment and compositor."
                    .to_string(),
            },
        }
    }
}

#[tauri::command]
fn get_runtime_capabilities() -> RuntimeCapabilitiesPayload {
    let status = get_permission_status();

    let can_capture_audio = status.microphone.state != "needs_action";
    let can_text_inject = if cfg!(target_os = "macos") {
        status.accessibility.state == "granted"
    } else {
        true
    };
    // We cannot reliably determine this across all OSes without active runtime probes;
    // keep unknown as advisory and let onboarding/settings provide explicit test actions.
    let can_global_hotkey = if cfg!(target_os = "macos") {
        status.input_monitoring.state == "granted"
    } else {
        true
    };

    let mut next_steps: Vec<String> = Vec::new();
    if status.microphone.actionable {
        next_steps.push(
            "Run the microphone test first; if it fails, open microphone settings and retry."
                .to_string(),
        );
    }
    if status.accessibility.actionable {
        next_steps.push(
            "Enable accessibility/text-control permission so Kalam can insert text into other apps."
                .to_string(),
        );
    }
    if status.input_monitoring.actionable {
        next_steps.push(
            "Test the global shortcut in another app; if it fails, enable Input Monitoring for Kalam."
                .to_string(),
        );
    }

    RuntimeCapabilitiesPayload {
        can_capture_audio,
        can_text_inject,
        can_global_hotkey,
        capture_audio_state: status.microphone.state.clone(),
        text_inject_state: status.accessibility.state.clone(),
        global_hotkey_state: status.input_monitoring.state.clone(),
        next_steps,
        permission_status: status,
    }
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
    let selected_api_key_present = cfg
        .stt_config
        .api_keys
        .get(&cfg.stt_config.provider)
        .map(|s| !s.is_empty())
        .unwrap_or(false)
        || cfg
            .stt_config
            .api_key
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false);
    log::debug!(
        "Returning settings, selected provider api key present: {}",
        selected_api_key_present
    );
    Ok(cfg)
}

#[tauri::command]
async fn save_settings(
    state: tauri::State<'_, AppState>,
    new_config: AppConfig,
) -> Result<(), String> {
    // Reconfigure logger immediately so this save operation is captured in the in-app log
    app_log::reconfigure(new_config.logging.clone());

    log::info!("=== SAVE_SETTINGS CALLED ===");
    let selected_api_key_len = new_config
        .stt_config
        .api_keys
        .get(&new_config.stt_config.provider)
        .map(|s| s.len())
        .or_else(|| new_config.stt_config.api_key.as_ref().map(|s| s.len()));
    log::info!(
        "Selected provider API key present: {}",
        selected_api_key_len.is_some()
    );
    log::info!(
        "Selected provider API key length: {:?}",
        selected_api_key_len
    );
    log::info!("Audio device: {:?}", new_config.audio_device);

    // Reject if toggle dictation hotkey is the same as the hold to dictate hotkey
    if let Some(ref hold) = new_config.hotkey {
        if !hold.trim().is_empty() {
            if let Ok(hold_hk) = parse_rdev_hotkey(hold) {
                if let Some(ref toggle_dict) = new_config.toggle_dictation_hotkey {
                    if !toggle_dict.trim().is_empty() {
                        if let Ok(toggle_hk) = parse_rdev_hotkey(toggle_dict) {
                            if hold_hk == toggle_hk {
                                return Err("Toggle dictation hotkey cannot be the same as the hold to dictate hotkey.".to_string());
                            }
                        }
                    }
                }
            }
        }
    }

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

            sync_autostart(&state.app_handle, config_to_save.auto_start);

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
            let cmd_hk = config_to_save
                .command_config
                .enabled
                .then(|| config_to_save.command_config.hotkey.clone())
                .flatten()
                .filter(|s| !s.trim().is_empty());

            let registrations = create_registrations(
                &state.app_handle,
                state.is_recording.clone(),
                &config_to_save.hotkey,
                config_to_save.toggle_dictation_hotkey.clone(),
                config_to_save.language_toggle_hotkey.clone(),
                cmd_hk,
                config_to_save.recording_mode,
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

/// Returns the effective logging config the backend is actually using, so the
/// UI can confirm it matches what the user expects.
#[tauri::command]
fn get_logging_state() -> Result<serde_json::Value, String> {
    let config = app_log::current_config();
    serde_json::to_value(&config).map_err(|e| e.to_string())
}

/// Show a native "Save as" dialog and write the in-memory log buffer to the chosen file.
#[tauri::command]
async fn save_log_to_file() -> Result<(), String> {
    let content = app_log::get_snapshot();
    if content.trim().is_empty() {
        return Err(
            "No log entries to download. Enable logging and use the app to capture entries."
                .to_string(),
        );
    }
    let default_name = format!(
        "kalam-log-{}.log",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    let path = tauri::async_runtime::spawn_blocking(move || {
        rfd::FileDialog::new()
            .add_filter("Log", &["log"])
            .set_file_name(&default_name)
            .save_file()
    })
    .await
    .map_err(|e| e.to_string())?
    .ok_or("Save cancelled")?;
    tauri::async_runtime::spawn_blocking(move || std::fs::write(path, content))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Show a native "Save as" dialog and write all logs from the database (CSV) to the chosen file.
#[tauri::command]
async fn save_logs_csv_to_file() -> Result<(), String> {
    let (csv, filename) = app_log::export_logs_csv().map_err(|e| e.to_string())?;
    let lines = csv.trim().split('\n').filter(|s| !s.is_empty()).count();
    if lines <= 1 {
        return Err("No log entries in database.".to_string());
    }
    let path = tauri::async_runtime::spawn_blocking(move || {
        rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name(&filename)
            .save_file()
    })
    .await
    .map_err(|e| e.to_string())?
    .ok_or("Save cancelled")?;
    tauri::async_runtime::spawn_blocking(move || std::fs::write(path, csv))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
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

    // Match save_settings: push config to the shell/overlay and re-register hotkeys so reset
    // is live without an app restart (previously only app_reset ran, leaving stale registrations).
    let _ = state.app_handle.emit("settings_updated", &default_config);
    let _ = update_overlay_position(&state.app_handle);
    reset_overlay_state(&state.app_handle);

    let cmd_hk = default_config
        .command_config
        .enabled
        .then(|| default_config.command_config.hotkey.clone())
        .flatten()
        .filter(|s| !s.trim().is_empty());
    let registrations = create_registrations(
        &state.app_handle,
        state.is_recording.clone(),
        &default_config.hotkey,
        default_config.toggle_dictation_hotkey.clone(),
        default_config.language_toggle_hotkey.clone(),
        cmd_hk,
        default_config.recording_mode,
    );
    crate::hotkey::update_registrations(registrations);

    let _ = state.app_handle.emit("app_reset", ());
    Ok(())
}

#[tauri::command]
fn get_app_data_path() -> Result<String, String> {
    crate::config::get_kalam_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
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
        Ok((_level, mut samples, sample_rate)) => {
            let cfg = state.config.lock().await.get_all();
            let stt = crate::config::privacy::effective_stt_config(&cfg);
            crate::audio::filter::apply_filter_chain(&mut samples, &stt.audio_filter, sample_rate);
            // Match dictation path: level reflects post-filter signal for the meter readout.
            let level = if samples.is_empty() {
                0.0
            } else {
                let sum: f32 = samples.iter().map(|s| s * s).sum();
                let rms = (sum / samples.len() as f32).sqrt();
                (rms * 10.0).min(1.0)
            };
            log::info!(
                "Test stopped, level: {}, samples: {} (after optional filter)",
                level,
                samples.len()
            );
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

#[tauri::command]
async fn search_history(query: String) -> Result<Vec<history::HistoryEntry>, String> {
    history::search(query.trim())
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
async fn clear_history(app: tauri::AppHandle) -> Result<(), String> {
    history::clear().await.map_err(|e| e.to_string())?;
    let _ = app.emit("history-cleared", ());
    Ok(())
}

/// Bring the main window forward so the user can retry dictation after an overlay error.
#[tauri::command]
fn focus_main_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
    Ok(())
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
    fn model_metadata(id: &str) -> (String, String, String) {
        match id {
            "sensevoice" => (
                "SenseVoice Small".into(),
                "Fast".into(),
                "50+ languages".into(),
            ),
            "whisper_base" => (
                "Whisper Base".into(),
                "Good quality".into(),
                "99+ languages".into(),
            ),
            _ => (id.to_string(), "—".into(), "—".into()),
        }
    }
    let rss_by_id = state.local_model_manager.sidecar_rss_by_model_id().await;
    let mut out = serde_json::Map::new();
    for m in crate::stt::models::known_models() {
        let status = state.local_model_manager.get_status(m.id).await;
        let (status_label, error_message) = status_parts(status);
        let (label, quality, languages) = model_metadata(m.id);
        let rss_bytes = rss_by_id.get(m.id).copied();
        out.insert(
            m.id.to_string(),
            json!({
                "installed": crate::stt::models::is_installed(m.id),
                "size_mb": m.size_mb,
                "status": status_label,
                "error": error_message,
                "download_progress": serde_json::Value::Null,
                "label": label,
                "quality": quality,
                "languages": languages,
                "rss_bytes": rss_bytes
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
async fn stop_all_local_models(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.local_model_manager.stop_all_models().await;
    Ok(())
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
async fn uninstall_sidecar(
    state: tauri::State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    let sidecar_id = stt::sidecars::model_id_to_sidecar_id(&model_id)
        .ok_or_else(|| format!("No engine for model {}", model_id))?;
    state
        .local_model_manager
        .stop_model(&model_id)
        .await
        .map_err(|e| e.to_string())?;
    stt::sidecars::uninstall_sidecar(sidecar_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn is_sidecar_installed_for_model(model_id: String) -> Result<bool, String> {
    Ok(stt::sidecars::sidecar_installed_for_model(&model_id))
}

#[tauri::command]
fn is_sidecar_available_for_model(model_id: String) -> Result<bool, String> {
    Ok(stt::sidecars::sidecar_available_for_model(&model_id))
}

#[tauri::command]
async fn download_model(
    state: tauri::State<'_, AppState>,
    model_type: String,
) -> Result<(), String> {
    // Install the local engine (sidecar) before model weights so first-time setup can start the model once.
    if let Some(sidecar_id) = crate::stt::sidecars::model_id_to_sidecar_id(&model_type) {
        if crate::stt::sidecars::sidecar_download_info(sidecar_id).is_some()
            && !crate::stt::sidecars::sidecar_is_installed(sidecar_id)
        {
            crate::stt::sidecars::download_sidecar_with_progress(sidecar_id, &state.app_handle)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
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
/// Extra space so pill box-shadow can spill outside the window (not clipped)
const OVERLAY_SHADOW_MARGIN: i32 = 24;
/// Expanded pill size (matches .blip.expanded in Overlay.svelte) + shadow margin
const OVERLAY_EXPANDED_WIDTH: i32 = 250 + OVERLAY_SHADOW_MARGIN * 2;
const OVERLAY_EXPANDED_HEIGHT: i32 = 48 + OVERLAY_SHADOW_MARGIN * 2;
/// Collapsed pill size: fits .blip.collapsed (48×10) + 1px border + small margin so edges aren't clipped + shadow margin
const OVERLAY_COLLAPSED_WIDTH: i32 = 52 + OVERLAY_SHADOW_MARGIN * 2;
const OVERLAY_COLLAPSED_HEIGHT: i32 = 14 + OVERLAY_SHADOW_MARGIN * 2;
const OVERLAY_BOTTOM_MARGIN: i32 = 24;

#[derive(serde::Serialize, Clone)]
#[serde(tag = "kind")]
enum OverlayEvent {
    Hidden,
    Collapsed,
    Listening {
        sensitive_app: bool,
    },
    ShortPress,
    Recording {
        level: f32,
        is_command: bool,
        sensitive_app: bool,
    },
    /// Processing with progress: elapsed/expected for timeout hint, attempt for retry badge.
    Processing {
        elapsed_secs: u32,
        expected_secs: u32,
        attempt: u32,
        message: Option<String>,
    },
    #[allow(dead_code)] // Reserved for future "transcription succeeded" UI
    Success,
    Error {
        message: String,
    },
    Status {
        message: String,
        highlight: Option<String>,
    },
    /// User clicked cancel; show briefly then return to idle.
    Cancelling,
    /// Focus moved to a sensitive app (Hybrid/Auto): brief expanded pill with lock; no dictation yet.
    SensitiveAppPeek,
}

#[tauri::command]
fn resize_overlay(app: tauri::AppHandle, expanded: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    state
        .overlay_expanded
        .store(expanded, std::sync::atomic::Ordering::Relaxed);

    if let Some(overlay) = app.get_webview_window(OVERLAY_LABEL) {
        let width = if expanded {
            OVERLAY_EXPANDED_WIDTH as f64
        } else {
            OVERLAY_COLLAPSED_WIDTH as f64
        };
        let height = if expanded {
            OVERLAY_EXPANDED_HEIGHT as f64
        } else {
            OVERLAY_COLLAPSED_HEIGHT as f64
        };

        // Resize first, then update position: when a window resizes, its top-left stays fixed
        // and it grows down/right. The pill is centered via flexbox, so it would appear to move.
        // By resizing then repositioning, we compensate so the pill stays visually fixed.
        let size = tauri::LogicalSize::new(width, height);
        let _ = overlay.set_size(tauri::Size::Logical(size));
        let _ = update_overlay_position(&app);
        let _ = overlay.set_always_on_top(true);
    }
    Ok(())
}

fn emit_overlay_event(app: &tauri::AppHandle, event: OverlayEvent) {
    let _ = app.emit_to(OVERLAY_LABEL, "overlay-state", event.clone());
    // Main shell status bar runs in a different webview; mirror lifecycle for runtime-first UI.
    let _ = app.emit("overlay-state-broadcast", event);

    // WebView2's Chromium renderer throttles ExecuteScript delivery for unfocused windows.
    // Nudge the renderer by forcing a repaint on the overlay HWND. This sends a synchronous
    // WM_PAINT that the renderer must process, flushing any pending scripts along the way.
    #[cfg(windows)]
    nudge_overlay_renderer(app);
    latency_trace_write("T5_after_nudge");
}

/// Force the overlay's WebView2 renderer to process pending ExecuteScript calls.
/// WebView2 throttles JS execution in unfocused windows. Sending WM_NULL forces the
/// window's message pump to iterate, and the WebView2 subclass proc picks up queued work.
/// As a stronger nudge, we also briefly activate the window (SetForegroundWindow) and
/// immediately restore the previous foreground — this wakes the Chromium renderer without
/// a user-visible focus flash.
#[cfg(windows)]
fn nudge_overlay_renderer(app: &tauri::AppHandle) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Some(overlay) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };
    let win = overlay.as_ref().window();
    let Ok(handle) = win.window_handle() else {
        return;
    };
    if let RawWindowHandle::Win32(win32) = handle.as_raw() {
        let hwnd = win32.hwnd.get() as windows_sys::Win32::Foundation::HWND;
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::*;

            let prev = GetForegroundWindow();

            // Briefly make the overlay foreground so its renderer exits throttled state.
            SetForegroundWindow(hwnd);

            // Restore original foreground immediately so the user sees no focus change.
            if prev != 0 {
                SetForegroundWindow(prev);
            }

            // WM_NULL nudges the message loop without side effects.
            PostMessageW(hwnd, 0, 0, 0);
        }
    }
}

/// Returns the initial overlay state (Collapsed or Hidden) for the overlay to fetch when it mounts.
/// Used because the startup emit can happen before the overlay webview has registered its listener.
#[tauri::command]
async fn cancel_transcription(state: tauri::State<'_, AppState>) -> Result<(), String> {
    log::info!("User requested transcription cancellation");
    let guard = state.transcription_cancel.write().await;
    guard.cancel();
    drop(guard);
    emit_overlay_event(&state.app_handle, OverlayEvent::Cancelling);
    Ok(())
}

/// Toggle dictation from the main-window status bar (same behavior as the toggle hotkey).
#[tauri::command]
async fn ui_toggle_dictation(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let is_recording = state.is_recording.clone();
    toggle_dictation(state, is_recording).await;
    Ok(())
}

#[tauri::command]
fn get_overlay_initial_state(state: tauri::State<AppState>) -> OverlayEvent {
    let dictation_enabled = if let Ok(config) = state.config.try_lock() {
        config.get_all().dictation_enabled
    } else {
        true
    };
    if dictation_enabled {
        OverlayEvent::Collapsed
    } else {
        OverlayEvent::Hidden
    }
}

fn reset_overlay_state(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let dictation_enabled = if let Ok(config) = state.config.try_lock() {
        config.get_all().dictation_enabled
    } else {
        true // default fallback
    };
    // Pre-collapse the window from Rust so JS doesn't have to roundtrip.
    let _ = resize_overlay(app.clone(), false);
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
    let (position, offset_x, offset_y, expand_direction) = {
        // Use try_lock to avoid blocking the async runtime
        if let Ok(cfg) = state.config.try_lock() {
            let all = cfg.get_all();
            (
                all.overlay_position.clone(),
                all.overlay_offset_x,
                all.overlay_offset_y,
                all.overlay_expand_direction.clone(),
            )
        } else {
            // Fallback if we can't lock immediately
            (
                crate::config::OverlayPosition::default(),
                0,
                0,
                crate::config::ExpandDirection::default(),
            )
        }
    };

    let expanded = state.overlay_expanded.load(Ordering::Relaxed);
    let (logical_width, logical_height) = if expanded {
        (
            OVERLAY_EXPANDED_WIDTH as f64,
            OVERLAY_EXPANDED_HEIGHT as f64,
        )
    } else {
        (
            OVERLAY_COLLAPSED_WIDTH as f64,
            OVERLAY_COLLAPSED_HEIGHT as f64,
        )
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

        let physical_margin = (OVERLAY_BOTTOM_MARGIN as f64 * scale_factor).round() as i32;
        let physical_offset_x = (offset_x as f64 * scale_factor).round() as i32;
        let physical_offset_y = (offset_y as f64 * scale_factor).round() as i32;
        let full_width = (OVERLAY_EXPANDED_WIDTH as f64 * scale_factor).round() as i32;
        let full_height = (OVERLAY_EXPANDED_HEIGHT as f64 * scale_factor).round() as i32;
        let physical_width = (logical_width * scale_factor).round() as i32;
        let physical_height = (logical_height * scale_factor).round() as i32;

        // Compute top-left of the *full* (300x120) window so we can anchor the pill when resizing.
        let mut x_full = wa.position.x;
        let mut y_full = wa.position.y;
        use crate::config::OverlayPosition::*;
        match position {
            BottomCenter => {
                x_full += (wa.size.width as i32 - full_width) / 2;
                y_full += wa.size.height as i32 - full_height - physical_margin;
            }
            BottomLeft => {
                x_full += physical_margin;
                y_full += wa.size.height as i32 - full_height - physical_margin;
            }
            BottomRight => {
                x_full += wa.size.width as i32 - full_width - physical_margin;
                y_full += wa.size.height as i32 - full_height - physical_margin;
            }
            TopCenter => {
                x_full += (wa.size.width as i32 - full_width) / 2;
                y_full += physical_margin;
            }
            TopLeft => {
                x_full += physical_margin;
                y_full += physical_margin;
            }
            TopRight => {
                x_full += wa.size.width as i32 - full_width - physical_margin;
                y_full += physical_margin;
            }
            CenterLeft => {
                x_full += physical_margin;
                y_full += (wa.size.height as i32 - full_height) / 2;
            }
            CenterRight => {
                x_full += wa.size.width as i32 - full_width - physical_margin;
                y_full += (wa.size.height as i32 - full_height) / 2;
            }
            Center => {
                x_full += (wa.size.width as i32 - full_width) / 2;
                y_full += (wa.size.height as i32 - full_height) / 2;
            }
        }
        x_full += physical_offset_x;
        y_full += physical_offset_y;

        // Content anchor: point on screen that stays fixed when window resizes (depends on expand_direction).
        use crate::config::ExpandDirection;
        let (anchor_cx, anchor_cy) = match expand_direction {
            ExpandDirection::Up => (x_full + full_width / 2, y_full + full_height),
            ExpandDirection::Down => (x_full + full_width / 2, y_full),
            ExpandDirection::Center => (x_full + full_width / 2, y_full + full_height / 2),
        };
        let x = anchor_cx - physical_width / 2;
        let y = match expand_direction {
            ExpandDirection::Up => anchor_cy - physical_height,
            ExpandDirection::Down => anchor_cy,
            ExpandDirection::Center => anchor_cy - physical_height / 2,
        };

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
        }
        // Re-assert always on top every time (e.g. after resize) so overlay stays above taskbar
        let _ = overlay.set_always_on_top(true);
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

    // Re-apply transparent background before show (helps macOS avoid white flash).
    let _ = win.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));

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

/// On Windows: resolve lowercase filename and full image path for the foreground HWND.
/// Returns None if the process cannot be queried.
#[cfg(windows)]
fn get_foreground_exe_info(hwnd: usize) -> Option<(String, String)> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd as isize, &mut pid) };
    if pid == 0 {
        return None;
    }

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle == 0 {
        return None;
    }

    let mut buf = [0u16; 1024];
    let mut size = buf.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) };
    unsafe { CloseHandle(handle) };

    if ok == 0 {
        return None;
    }

    let path = String::from_utf16_lossy(&buf[..size as usize]);
    let path = path.trim_end_matches('\0').to_string();
    let short = std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())?;
    Some((short, path))
}

/// True when Hybrid/Auto mode forces Local STT due to sensitive app patterns (overlay amber state).
fn sensitive_app_forces_local(config: &AppConfig) -> bool {
    let effective = crate::config::privacy::effective_stt_config(config);
    matches!(effective.mode, STTMode::Local)
        && matches!(config.stt_config.mode, STTMode::Hybrid | STTMode::Auto)
}

async fn start_dictation(state: tauri::State<'_, AppState>, is_recording: Arc<AtomicBool>) {
    if is_recording.load(Ordering::SeqCst) {
        log::debug!("Already recording, ignoring start request");
        return;
    }
    {
        let mut audio_state = state.audio_state.lock().await;
        if !matches!(*audio_state, AudioState::Idle) {
            log::debug!("Cannot start dictation, current state: {:?}", *audio_state);
            return;
        }
        *audio_state = AudioState::Starting;
    }
    latency_trace_write("start_dictation_entered");
    let _ = crate::tray::TrayManager::set_tray_state(&state.app_handle, AudioState::Starting);

    log::info!("Starting dictation...");

    // Pre-resize the overlay to expanded BEFORE emitting Listening so the window is already
    // large enough when JS processes the event. Eliminates the IPC roundtrip (JS → Rust resize)
    // that was the main source of perceived delay.
    latency_trace_write("T2_before_resize");
    let _ = resize_overlay(state.app_handle.clone(), true);
    latency_trace_write("T2_after_resize");

    #[cfg(windows)]
    {
        let hwnd = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
        if hwnd != 0 {
            *state.foreground_for_injection.lock().await = Some(hwnd as usize);
            // Resolve process name + full path in background so it doesn't block pill/sound (like beta.5).
            let process_name_state = state.foreground_process_name.clone();
            let exe_path_state = state.foreground_exe_path.clone();
            let hwnd_usize = hwnd as usize;
            tauri::async_runtime::spawn(async move {
                let name = tauri::async_runtime::spawn_blocking(move || {
                    get_foreground_exe_info(hwnd_usize)
                })
                .await
                .unwrap_or(None);
                if let Ok(mut guard) = process_name_state.try_lock() {
                    *guard = name.as_ref().map(|(n, _)| n.clone());
                }
                if let Ok(mut guard) = exe_path_state.try_lock() {
                    *guard = name.as_ref().map(|(_, p)| p.clone());
                }
            });
        }
    }
    #[cfg(not(windows))]
    {
        if let Some((process_name, _title)) = crate::config::privacy::get_foreground_app() {
            if !process_name.is_empty() {
                *state.foreground_process_name.lock().await = Some(process_name);
            }
        }
        if let Ok(window) = active_win_pos_rs::get_active_window() {
            let pid = window.process_id;
            use sysinfo::{Pid, ProcessesToUpdate, System};
            let mut sys = System::new();
            sys.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid as u32)]));
            if let Some(p) = sys.process(Pid::from_u32(pid as u32)) {
                if let Some(exe) = p.exe() {
                    *state.foreground_exe_path.lock().await =
                        Some(exe.to_string_lossy().to_string());
                }
            }
        }
    }

    let cfg_for_sensitive = state.config.lock().await.get_all();
    let sensitive_app = sensitive_app_forces_local(&cfg_for_sensitive);
    state
        .is_sensitive_app_active
        .store(sensitive_app, Ordering::SeqCst);

    let run = LISTENING_EMIT_RUN.fetch_add(1, Ordering::SeqCst) + 1;
    latency_trace_write(&format!("listening_emit_run_{}", run));
    latency_trace_write("T4_before_emit");
    emit_overlay_event(&state.app_handle, OverlayEvent::Listening { sensitive_app });
    latency_trace_write("after_emit_listening");
    let app_handle = state.app_handle.clone();
    latency_trace_write("T3_before_play_sound");
    let sound_enabled = state
        .config
        .lock()
        .await
        .get_all()
        .notifications
        .sound_enabled;
    if sound_enabled {
        if let Err(e) = play_sound(&app_handle, "dictation-started") {
            log::warn!("Failed to play dictation start sound: {}", e);
        }
    }
    latency_trace_write("T3_after_play_sound");
    latency_trace_write("after_play_sound");

    latency_trace_write("before_update_overlay_position");
    let _ = update_overlay_position(&state.app_handle);
    latency_trace_write("after_update_overlay_position");

    // Start actual audio recording
    if let Err(e) = state.audio_capture.lock().await.start_recording().await {
        log::error!("Failed to start recording: {}", e);
        let mut audio_state = state.audio_state.lock().await;
        *audio_state = AudioState::Idle;
        is_recording.store(false, Ordering::SeqCst);
        state.is_sensitive_app_active.store(false, Ordering::SeqCst);
        return;
    }

    // If a cancellation happened during startup, clean up and exit.
    {
        let mut audio_state = state.audio_state.lock().await;
        if !matches!(*audio_state, AudioState::Starting) {
            let _ = state.audio_capture.lock().await.stop_recording().await;
            is_recording.store(false, Ordering::SeqCst);
            state.is_sensitive_app_active.store(false, Ordering::SeqCst);
            return;
        }
        *audio_state = AudioState::Recording;
    }
    is_recording.store(true, Ordering::SeqCst);
    let _ = crate::tray::TrayManager::set_tray_state(&state.app_handle, AudioState::Recording);

    // Emit audio level to overlay while recording
    let app_handle_level = state.app_handle.clone();
    let audio_capture = state.audio_capture.clone();
    let is_recording_level = is_recording.clone();
    let is_sensitive_app_active = state.is_sensitive_app_active.clone();
    let is_command = *state.recording_type.lock().await == RecordingType::Command;
    tauri::async_runtime::spawn(async move {
        while is_recording_level.load(Ordering::SeqCst) {
            let level = audio_capture.lock().await.get_current_recording_level();
            emit_overlay_event(
                &app_handle_level,
                OverlayEvent::Recording {
                    level,
                    is_command,
                    sensitive_app: is_sensitive_app_active.load(Ordering::SeqCst),
                },
            );
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    });

    log::info!("Audio recording started");
}

/// Input for the blocking transcription thread. Provider is always created on the OS thread
/// so reqwest::blocking::Client is never created/dropped on a tokio worker.
/// audio_data is Arc so the job can be cloned for retries without copying the buffer.
#[derive(Clone)]
enum TranscribeJob {
    FromConfig {
        stt_config: STTConfig,
        app_handle: Option<tauri::AppHandle>,
        audio_data: Arc<Vec<f32>>,
        sample_rate: u32,
        vad_config: VADConfig,
        language: Option<String>,
        vocabulary: Option<String>,
    },
}

/// Build a DuckDuckGo search URL (`q` query param) so the OS default browser opens a normal web search.
fn command_mode_web_search_url(query: &str) -> Result<String, String> {
    let mut u = url::Url::parse("https://duckduckgo.com/")
        .map_err(|e| format!("Invalid search URL: {}", e))?;
    u.query_pairs_mut().append_pair("q", query);
    Ok(u.to_string())
}

/// Open the given web search in the user's default browser (tauri-plugin-opener).
fn open_command_mode_web_search(app_handle: &tauri::AppHandle, query: &str) -> Result<(), String> {
    let url = command_mode_web_search_url(query)?;
    app_handle
        .opener()
        .open_url(url, None::<&str>)
        .map_err(|e| format!("Could not open browser: {}", e))?;
    Ok(())
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

    // Same idea as "new note …": fixed prefix, then the rest is the payload (here: search terms).
    const ONLINE_SEARCH_PREFIX: &str = "online search";
    if lower.starts_with(ONLINE_SEARCH_PREFIX) {
        let query = trimmed[ONLINE_SEARCH_PREFIX.len()..].trim();
        if query.is_empty() {
            if config.notifications.show_errors {
                let state = app_handle.state::<AppState>();
                let _ = state
                    .notification_manager
                    .warning("Say \"online search\" followed by what you want to look up.");
            }
            return Err("Say \"online search\" followed by what you want to look up.".to_string());
        }
        open_command_mode_web_search(app_handle, query)?;
        let preview = if query.chars().count() > 50 {
            format!("{}...", query.chars().take(47).collect::<String>())
        } else {
            query.to_string()
        };
        emit_overlay_event(
            app_handle,
            OverlayEvent::Status {
                message: format!("Search opened: {}", preview),
                highlight: None,
            },
        );
        return Ok(());
    }

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
            // Voice "new reminder" creates a note; user sets `reminder_at` in the note editor (no standalone reminder type).
            ("note", trimmed["new reminder".len()..].trim().to_string())
        } else {
            if config.notifications.show_errors {
                let state = app_handle.state::<AppState>();
                let _ = state.notification_manager.warning(
                    "Say \"new note\", \"new task\", \"new reminder\", or \"online search\" followed by your content.",
                );
            }
            return Err("Could not detect command. Say 'new note', 'new task', 'new reminder', or 'online search' followed by your content.".to_string());
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
- entry_type: exactly one of "note", "task" (if the user wants a time-based reminder with no task to complete, use "note" with reminder_at)
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
                            if et_lower == "note" || et_lower == "task" {
                                resolved_entry_type = et_lower;
                            } else if et_lower == "reminder" {
                                // LLM may still emit "reminder"; store as a note with reminder_at / rrule.
                                resolved_entry_type = "note".to_string();
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
        target_app: None,
        target_app_name: None,
        duration_ms: None,
        word_count: None,
        stt_latency_ms: None,
        stt_mode: None,
        dictation_language: None,
        session_mode: None,
        stt_provider: None,
        note_order: 0,
    };

    let conn = crate::db::open_db().map_err(|e| e.to_string())?;
    crate::db::insert_entry(&conn, &entry).map_err(|e| e.to_string())?;
    crate::db::insert_embedding_stub(&conn, &entry.id).map_err(|e| e.to_string())?;

    let _state = app_handle.state::<AppState>();
    let label = match entry_type {
        "task" => "Task",
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

/// Ensures long recordings get enough wall time vs adaptive stats from short clips.
fn apply_audio_transcription_floor(
    mut timeout_secs: u64,
    recording_duration_ms: Option<u32>,
    is_cloud: bool,
    max_secs: u64,
) -> u64 {
    if let Some(ms) = recording_duration_ms.filter(|&m| m > 0) {
        let audio_secs = (u64::from(ms) + 999) / 1000;
        let audio_floor = (if is_cloud {
            // Cloud APIs process audio near-real-time; cap the per-clip floor so long
            // recordings don't balloon the timeout (Groq handles 40s audio in ~2s normally).
            let clip_floor = audio_secs.min(20);
            12u64.saturating_add(clip_floor / 2)
        } else {
            25u64.saturating_add(audio_secs.saturating_mul(4))
        })
        .min(max_secs);
        if audio_floor > timeout_secs {
            log::info!(
                "Transcription timeout: {}ms audio raises floor from {}s to {}s (cloud={})",
                ms,
                timeout_secs,
                audio_floor,
                is_cloud
            );
        }
        timeout_secs = timeout_secs.max(audio_floor);
    }
    timeout_secs.min(max_secs)
}

/// Compute transcription timeout from historical latency (today's average) with tiered logic.
/// Cloud vs local use different minimums and cold-start defaults.
/// When `recording_duration_ms` is set, raises the ceiling so long clips are not cut off by
/// stats from short utterances (adaptive min can be far below real-time API needs).
fn calculate_transcription_timeout(
    config: &AppConfig,
    recording_duration_ms: Option<u32>,
) -> std::time::Duration {
    use crate::config::privacy::effective_stt_config;
    use crate::config::STTMode;
    use std::time::Duration;

    let stt = effective_stt_config(config);
    let tc = &stt.transcription_timeout;
    let is_cloud = matches!(stt.mode, STTMode::Cloud | STTMode::Hybrid | STTMode::Auto);

    let (min_secs, default_secs) = if is_cloud {
        (tc.timeout_min_seconds_cloud, 45u64)
    } else {
        (tc.timeout_min_seconds_local, 75u64)
    };
    let max_secs = tc.timeout_max_seconds;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let Ok(conn) = db::open_db() else {
        log::debug!("Timeout: no DB, using default {}s", default_secs);
        let secs = apply_audio_transcription_floor(
            default_secs.min(max_secs),
            recording_duration_ms,
            is_cloud,
            max_secs,
        );
        return Duration::from_secs(secs);
    };
    let Ok(Some(row)) = db::get_daily_stats(&conn, Some(&today)) else {
        log::debug!(
            "Timeout: no stats for today, using default {}s",
            default_secs
        );
        let secs = apply_audio_transcription_floor(
            default_secs.min(max_secs),
            recording_duration_ms,
            is_cloud,
            max_secs,
        );
        return Duration::from_secs(secs);
    };

    // Need at least 3 samples today to adapt; otherwise use default.
    if row.transcriptions_count < 3 {
        log::debug!(
            "Timeout: only {} samples today, using default {}s",
            row.transcriptions_count,
            default_secs
        );
        let secs = apply_audio_transcription_floor(
            default_secs.min(max_secs),
            recording_duration_ms,
            is_cloud,
            max_secs,
        );
        return Duration::from_secs(secs);
    }
    let Some(avg_ms) = row.latency_avg_ms else {
        let secs = apply_audio_transcription_floor(
            default_secs.min(max_secs),
            recording_duration_ms,
            is_cloud,
            max_secs,
        );
        return Duration::from_secs(secs);
    };
    let avg_secs = (avg_ms as f64) / 1000.0;
    let computed = (avg_secs * tc.timeout_multiplier) + (tc.timeout_buffer_seconds as f64);
    let timeout_secs = (computed as u64).clamp(min_secs, max_secs);
    let timeout_secs =
        apply_audio_transcription_floor(timeout_secs, recording_duration_ms, is_cloud, max_secs);
    log::info!(
        "Transcription timeout: avg {}ms -> {}s (clamped to [{}, {}])",
        avg_ms,
        timeout_secs,
        min_secs,
        max_secs
    );
    Duration::from_secs(timeout_secs)
}

fn run_transcribe_job(job: TranscribeJob) -> anyhow::Result<crate::stt::TranscriptionResult> {
    let TranscribeJob::FromConfig {
        stt_config,
        app_handle,
        audio_data,
        sample_rate,
        vad_config,
        language,
        vocabulary,
    } = job;
    let language = language.as_deref();
    let vocabulary = vocabulary.as_deref();
    let provider = crate::stt::provider::create_provider_sync(&stt_config, app_handle.as_ref())?;
    log::info!(
        "Starting transcription with {} (chunked + prompt chaining)",
        provider.name()
    );
    crate::stt::transcribe_chunked(
        &*provider,
        audio_data.as_slice(),
        sample_rate,
        &vad_config,
        language,
        vocabulary,
    )
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
        state.is_sensitive_app_active.store(false, Ordering::SeqCst);
        emit_overlay_event(
            &state.app_handle,
            OverlayEvent::Processing {
                elapsed_secs: 0,
                expected_secs: 120,
                attempt: 1,
                message: None,
            },
        );
        let _ = crate::tray::TrayManager::set_tray_state(&state.app_handle, AudioState::Processing);

        // Play end sound when the user enabled UI sounds
        let app_handle = state.app_handle.clone();
        let sound_enabled = state
            .config
            .lock()
            .await
            .get_all()
            .notifications
            .sound_enabled;
        if sound_enabled {
            if let Err(e) = play_sound(&app_handle, "dictation-ended") {
                log::warn!("Failed to play dictation end sound: {}", e);
            }
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
        #[allow(unused_variables)]
        let foreground_process = state.foreground_process_name.lock().await.take();
        let foreground_exe_path = state.foreground_exe_path.lock().await.take();
        let recording_type = *state.recording_type.lock().await;
        let transcription_cancel = state.transcription_cancel.clone();

        tokio::spawn(async move {
            let stt_config = crate::config::privacy::effective_stt_config(&config);
            let vad_config = stt_config.vad_config();
            // Create provider inside the OS thread so reqwest::blocking::Client is never
            // created/dropped on a tokio worker (avoids "Cannot drop a runtime" panic).
            let app_handle_for_job = match stt_config.mode {
                crate::config::STTMode::Local => Some(app_handle.clone()),
                crate::config::STTMode::Cloud
                | crate::config::STTMode::Hybrid
                | crate::config::STTMode::Auto => None,
            };
            // One-shot light DSP before STT; same buffer is reused across transcription retries.
            let mut audio_data = audio_data;
            crate::audio::filter::apply_filter_chain(
                &mut audio_data,
                &stt_config.audio_filter,
                sample_rate,
            );
            let audio_data = Arc::new(audio_data);
            let recording_duration_ms: Option<u32> = if sample_rate > 0 {
                let ms = audio_data.len() as u64 * 1000 / sample_rate as u64;
                u32::try_from(ms.min(u64::from(u32::MAX)))
                    .ok()
                    .filter(|&n| n > 0)
            } else {
                None
            };
            let job = TranscribeJob::FromConfig {
                stt_config: stt_config.clone(),
                app_handle: app_handle_for_job.clone(),
                audio_data: audio_data.clone(),
                sample_rate,
                vad_config,
                language: config.languages.first().cloned(),
                vocabulary: vocabulary.clone(),
            };
            let max_wall_timeout = std::time::Duration::from_secs(
                stt_config.transcription_timeout.timeout_max_seconds.max(1),
            );
            let base_timeout = calculate_transcription_timeout(&config, recording_duration_ms);
            let mut this_attempt_timeout = base_timeout;
            let expected_secs = base_timeout.as_secs().max(1) as u32;
            log::info!(
                "Starting transcription (provider will be created on OS thread, chunked + prompt chaining), timeout {:?}",
                base_timeout
            );

            // Cancel token for this run; cancel_transcription command will cancel it.
            let cancel_token = CancellationToken::new();
            {
                let mut guard = transcription_cancel.write().await;
                *guard = cancel_token.clone();
            }
            let progress_app = app_handle.clone();
            let progress_token = cancel_token.clone();
            let current_attempt = Arc::new(AtomicUsize::new(1));
            let progress_attempt = current_attempt.clone();
            let progress_handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                let mut elapsed: u32 = 0;
                loop {
                    interval.tick().await;
                    if progress_token.is_cancelled() {
                        break;
                    }
                    elapsed = elapsed.saturating_add(1);
                    let attempt = progress_attempt.load(Ordering::SeqCst) as u32;
                    let long_threshold = (expected_secs / 2).max(8);
                    let message = if attempt > 1 {
                        Some(format!("Retry {}/3", attempt))
                    } else if elapsed >= long_threshold {
                        Some("Slow response...".to_string())
                    } else if elapsed >= 3 {
                        Some("Transcribing".to_string())
                    } else {
                        None
                    };
                    emit_overlay_event(
                        &progress_app,
                        OverlayEvent::Processing {
                            elapsed_secs: elapsed,
                            expected_secs,
                            attempt,
                            message,
                        },
                    );
                }
            });

            const MAX_ATTEMPTS: u32 = 3;
            let start = std::time::Instant::now();
            let mut last_error: Option<anyhow::Error> = None;
            let mut result = None;
            for attempt in 1..=MAX_ATTEMPTS {
                current_attempt.store(attempt as usize, Ordering::SeqCst);
                if cancel_token.is_cancelled() {
                    log::info!("Transcription cancelled by user");
                    progress_handle.abort();
                    last_error = None;
                    result = None;
                    break;
                }
                if attempt > 1 {
                    emit_overlay_event(
                        &app_handle,
                        OverlayEvent::Processing {
                            elapsed_secs: start.elapsed().as_secs() as u32,
                            expected_secs,
                            attempt,
                            message: Some(format!("Retry {}/3", attempt)),
                        },
                    );
                }
                let (tx, rx) =
                    oneshot::channel::<anyhow::Result<crate::stt::TranscriptionResult>>();
                let job_attempt = job.clone();
                std::thread::spawn(move || {
                    let r = run_transcribe_job(job_attempt);
                    let _ = tx.send(r);
                });

                let wait_fut = tokio::time::timeout(this_attempt_timeout, rx);
                tokio::select! {
                    biased;
                    _ = cancel_token.cancelled() => {
                        log::info!("Transcription cancelled by user (during wait)");
                        progress_handle.abort();
                        last_error = None;
                        result = None;
                        break;
                    }
                    res = wait_fut => match res {
                        Ok(Ok(Ok(transcription_result))) => {
                            progress_handle.abort();
                            result = Some(transcription_result);
                            last_error = None;
                            break;
                        }
                        Ok(Ok(Err(e))) => {
                            last_error = Some(e);
                            let te = last_error
                                .as_ref()
                                .and_then(|e| e.downcast_ref::<crate::stt::TranscriptionError>());
                            let retriable = te.is_some_and(|t| t.is_retriable());
                            if retriable && attempt < MAX_ATTEMPTS {
                                if let Some(t) = te {
                                    log::info!("Transcription error (retriable): {}", t);
                                }
                                let backoff_secs = 1u64 << (attempt - 1);
                                log::info!(
                                    "Transcription attempt {} failed (retriable), backing off {}s",
                                    attempt, backoff_secs
                                );
                                tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                            } else {
                                progress_handle.abort();
                                break;
                            }
                        }
                        Ok(Err(_)) => {
                            progress_handle.abort();
                            last_error = Some(anyhow::anyhow!("Transcription thread panicked"));
                            break;
                        }
                        Err(_) => {
                            log::warn!(
                                "Transcription attempt {} hit wall-clock deadline after {:?}",
                                attempt,
                                this_attempt_timeout
                            );
                            if attempt < MAX_ATTEMPTS {
                                let backoff_secs = 1u64 << (attempt - 1);
                                let scaled_secs = this_attempt_timeout
                                    .as_secs()
                                    .saturating_mul(3)
                                    .saturating_add(1)
                                    / 2;
                                this_attempt_timeout = std::time::Duration::from_secs(
                                    scaled_secs.max(this_attempt_timeout.as_secs()),
                                )
                                .min(max_wall_timeout);
                                log::info!(
                                    "Retrying transcription after timeout: backoff {}s, next deadline {:?}",
                                    backoff_secs,
                                    this_attempt_timeout
                                );
                                tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs))
                                    .await;
                            } else {
                                progress_handle.abort();
                                last_error = Some(anyhow::anyhow!("Transcription timed out"));
                                break;
                            }
                        }
                    }
                }
                if result.is_some() {
                    break;
                }
            }

            match result {
                Some(transcription_result) => {
                    let latency_ms = start.elapsed().as_millis() as u32;
                    if transcription_result.text.trim().is_empty() {
                        log::info!("Transcription empty (no speech), skipping save and injection");
                    } else {
                        let stt_mode_label = match stt_config.mode {
                            crate::config::STTMode::Cloud => "Cloud",
                            crate::config::STTMode::Local => "Local",
                            crate::config::STTMode::Hybrid => "Hybrid",
                            crate::config::STTMode::Auto => "Auto",
                        }
                        .to_string();
                        let dictation_language = config
                            .languages
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "auto".to_string());

                        #[derive(Clone, serde::Serialize)]
                        struct TranscriptionSavedPayload {
                            latency_ms: u32,
                            words_count: u32,
                        }

                        if recording_type == RecordingType::Command {
                            let cmd_text = transcription_result.text.trim().to_string();
                            let words_count = cmd_text.split_whitespace().count() as u32;
                            let meta = crate::history::HistorySaveMeta {
                                word_count: words_count,
                                stt_latency_ms: latency_ms,
                                stt_mode: stt_mode_label.clone(),
                                stt_provider: stt_config.provider.trim().to_string(),
                                dictation_language: dictation_language.clone(),
                                session_mode: "command".to_string(),
                            };
                            if let Err(e) = history::save_transcription(
                                &cmd_text,
                                foreground_process.clone(),
                                foreground_exe_path.clone(),
                                recording_duration_ms,
                                meta,
                                config.privacy.history_retention_days,
                            )
                            .await
                            {
                                log::error!("Failed to save command transcription to DB: {}", e);
                            }
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
                            let _ = app_handle.emit(
                                "transcription-saved",
                                TranscriptionSavedPayload {
                                    latency_ms,
                                    words_count,
                                },
                            );
                            if let Err(e) =
                                run_command_pipeline(&cmd_text, &config, &app_handle).await
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
                                &transcription_result.text,
                                &config.formatting,
                                &config.snippets,
                                len,
                                Some(prev_ref),
                            );
                            log::info!(
                                "Transcription completed: raw_len={} formatted_len={} actions={} injection={:?}",
                                transcription_result.text.len(),
                                formatted.len(),
                                actions.len(),
                                config.formatting.injection_method
                            );
                            let words_count = formatted.split_whitespace().count() as u32;
                            let meta = crate::history::HistorySaveMeta {
                                word_count: words_count,
                                stt_latency_ms: latency_ms,
                                stt_mode: stt_mode_label,
                                stt_provider: stt_config.provider.trim().to_string(),
                                dictation_language,
                                session_mode: "dictation".to_string(),
                            };
                            if let Err(e) = history::save_transcription(
                                &formatted,
                                foreground_process.clone(),
                                foreground_exe_path.clone(),
                                recording_duration_ms,
                                meta,
                                config.privacy.history_retention_days,
                            )
                            .await
                            {
                                log::error!("Failed to save transcription to DB: {}", e);
                            }
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
                                // Win11 XAML apps (e.g. Notepad) need extra time to fully settle
                                // focus before synthetic input is accepted reliably.
                                std::thread::sleep(std::time::Duration::from_millis(150));
                            }
                            // On Windows, override injection method to Clipboard for apps known to
                            // corrupt rapid keystroke bursts via TSF (e.g. Win11 Notepad).
                            #[allow(unused_mut)]
                            let mut effective_formatting = config.formatting.clone();
                            #[cfg(windows)]
                            {
                                if let Some(ref pname) = foreground_process {
                                    let name_lower = pname.to_lowercase();
                                    let force_clipboard = config
                                        .formatting
                                        .force_clipboard_apps
                                        .iter()
                                        .any(|app| name_lower.contains(&app.to_lowercase()));
                                    if force_clipboard {
                                        log::info!(
                                            "Forcing clipboard injection for process: {}",
                                            pname
                                        );
                                        effective_formatting.injection_method =
                                            crate::config::InjectionMethod::Clipboard;
                                    }
                                }
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
                                        .inject_with_config(&formatted, &effective_formatting)
                                        .await
                                    {
                                        log::error!("Failed to inject text: {}", e);
                                    } else {
                                        // Log enough to verify the right text was injected,
                                        // without logging full PII (first 5 words only).
                                        let preview: String = formatted
                                            .split_whitespace()
                                            .take(5)
                                            .collect::<Vec<_>>()
                                            .join(" ");
                                        log::info!(
                                            "Injection success: {} chars, preview: {:?}…",
                                            formatted.len(),
                                            preview
                                        );
                                        last_injected_len.store(formatted.len(), Ordering::SeqCst);
                                        *last_injected_text.lock().await = formatted;
                                        if config.notifications.show_completion {
                                            let nm = &app_handle
                                                .state::<AppState>()
                                                .notification_manager;
                                            let _ = nm.success("Dictation complete");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                None => {
                    if let Some(ref e) = last_error {
                        log::error!("Transcription failed: {}", e);
                        let message = if e.to_string().contains("timed out") {
                            format!(
                                "Timed out after {}s (last attempt deadline)",
                                this_attempt_timeout.as_secs()
                            )
                        } else {
                            e.to_string()
                        };
                        emit_overlay_event(&app_handle, OverlayEvent::Error { message });
                    }
                    // When last_error is None, user cancelled — no error overlay (Cancelling already shown).
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
        AudioState::Starting => {
            log::info!("Dictation startup in progress, ignoring toggle...");
        }
        AudioState::Recording => {
            stop_dictation(state, is_recording).await;
        }
        AudioState::Processing => {
            log::info!("Dictation already processing, ignoring...");
        }
    }
}
