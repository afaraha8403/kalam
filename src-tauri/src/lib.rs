pub mod app_log;
mod audio;
mod config;
mod formatting;
mod history;
mod hotkey;
#[cfg(windows)]
mod hotkey_win;
mod injection;
mod notifications;
mod stt;
mod tray;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tauri::{Emitter, Manager};
use tokio::sync::{Mutex, oneshot};
use tauri_plugin_updater::UpdaterExt;

use crate::audio::vad::VADConfig;
use crate::audio::{AudioState, play_sound};
use crate::config::STTConfig;
use crate::config::{AppConfig, ConfigManager};
use crate::notifications::NotificationManager;
use crate::tray::TrayManager;
use crate::hotkey::{parse_rdev_hotkey, start_listener, HotkeyRegistration};

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
}

impl AppState {
    pub fn new(app_handle: tauri::AppHandle) -> anyhow::Result<Self> {
        let _ = history::migrate_from_json_if_needed();
        let config = Arc::new(Mutex::new(ConfigManager::new()?));
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
        })
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_shell::init());
    #[cfg(windows)]
    {
        builder = builder.device_event_filter(tauri::DeviceEventFilter::Always);
    }
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
            
            // Handle window visibility based on config
            if let Some(window) = app.get_webview_window("main") {
                if start_in_focus {
                    window.show()?;
                    window.set_focus()?;
                    log::info!("Window shown and focused on startup");
                } else {
                    // Play background start sound
                    if let Err(e) = play_sound(&app.handle(), "started-in-background") {
                        log::warn!("Failed to play background start sound: {}", e);
                    }
                    log::info!("App started in background (minimized to tray)");
                }
            }
            
            // Clone what we need before managing state
            let is_recording_flag = state.is_recording.clone();
            
            // Manage state
            app.manage(state);

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
                        if let Err(e) = state.notification_manager.warning("Microphone disconnected. Switched to system default.") {
                            log::warn!("Hotplug notification failed: {}", e);
                        }
                    }
                }
            });
            
            // Register global hotkeys via rdev (dictation + optional language toggle)
            let app_handle = app.handle().clone();
            let (hotkey_str, recording_mode, language_toggle_hotkey) = {
                let state = app.state::<AppState>();
                let config = state.config.blocking_lock();
                let cfg = config.get_all();
                (
                    cfg.hotkey.clone(),
                    cfg.recording_mode,
                    cfg.language_toggle_hotkey.clone(),
                )
            };

            let rt = tokio::runtime::Runtime::new().expect("hotkey runtime");
            let rt_handle = rt.handle().clone();
            std::thread::spawn(move || {
                rt.block_on(std::future::pending::<()>());
            });

            let mut registrations: Vec<HotkeyRegistration> = Vec::new();

            if let Ok(target_hotkey) = parse_rdev_hotkey(&hotkey_str) {
                let app_handle_press = app_handle.clone();
                let is_recording_press = is_recording_flag.clone();
                let rt_for_press = rt_handle.clone();
                let app_handle_release = app_handle.clone();
                let is_recording_release = is_recording_flag.clone();
                let mode_for_release = recording_mode.clone();
                let rt_for_release = rt_handle.clone();

                registrations.push(HotkeyRegistration {
                    target: target_hotkey,
                    active: Arc::new(AtomicBool::new(false)),
                    on_press: Arc::new(move || {
                        log::info!("Hotkey pressed - callback invoked");
                        let app_handle = app_handle_press.clone();
                        let is_recording = is_recording_press.clone();
                        let rt = rt_for_press.clone();
                        rt.spawn(async move {
                            let state = app_handle.state::<AppState>();
                            *state.press_start_time.lock().await = Some(std::time::Instant::now());
                            let config = state.config.lock().await;
                            let cfg = config.get_all();
                            drop(config);
                            match cfg.recording_mode {
                                crate::config::RecordingMode::Hold => {
                                    start_dictation(state, is_recording).await;
                                }
                                crate::config::RecordingMode::Toggle => {
                                    toggle_dictation(state, is_recording).await;
                                }
                            }
                        });
                    }),
                    on_release: Arc::new(move || {
                        if matches!(mode_for_release, crate::config::RecordingMode::Hold) {
                            let app_handle = app_handle_release.clone();
                            let is_recording = is_recording_release.clone();
                            let rt = rt_for_release.clone();
                            rt.spawn(async move {
                                let state = app_handle.state::<AppState>();
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
                                            tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
                                            emit_overlay_event(&app_for_overlay, OverlayEvent::Collapsed);
                                        });
                                    }
                                } else {
                                    stop_dictation(state, is_recording).await;
                                }
                            });
                        }
                    }),
                });
                log::info!("Dictation hotkey registered: {} (mode: {:?})", hotkey_str, recording_mode);
            } else {
                log::error!("Failed to parse hotkey: {}", hotkey_str);
            }

            if let Some(ref toggle_str) = language_toggle_hotkey {
                if !toggle_str.is_empty() {
                    if let Ok(toggle_hotkey) = parse_rdev_hotkey(toggle_str) {
                        let app_handle_toggle = app_handle.clone();
                        registrations.push(HotkeyRegistration {
                            target: toggle_hotkey,
                            active: Arc::new(AtomicBool::new(false)),
                            on_press: Arc::new(move || {
                                let app_handle = app_handle_toggle.clone();
                                rt_handle.spawn(async move {
                                    let state = app_handle.state::<AppState>();
                                    let mut config_mgr = state.config.lock().await;
                                    let mut cfg = config_mgr.get_all();
                                    if cfg.languages.len() >= 2 {
                                        cfg.languages.swap(0, 1);
                                        let label = language_display_name(&cfg.languages[0]);
                                        if config_mgr.save(cfg).is_ok() {
                                            drop(config_mgr);
                                            let msg = format!("Language set to: {}", label);
                                            let _ = state.notification_manager.info(&msg);
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

            if !registrations.is_empty() {
                start_listener(registrations);
            }

            log::info!("Kalam Voice initialized successfully");
            
            // Show overlay in collapsed state on startup
            if let Err(e) = show_overlay(&app.handle()) {
                log::warn!("Failed to show overlay on startup: {}", e);
            }
            emit_overlay_event(&app.handle(), OverlayEvent::Collapsed);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_platform,
            request_system_permission,
            open_system_permission_page,
            get_settings,
            save_settings,
            get_audio_devices,
            test_microphone_start,
            test_microphone_level,
            test_microphone_stop,
            get_history,
            clear_history,
            get_snippets,
            add_snippet,
            remove_snippet,
            check_api_key,
            get_model_status,
            download_model,
            get_app_log,
            get_app_log_empty,
            open_app_data_folder,
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
    let update = updater.check().await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
    if let Some(u) = update {
        let msg = format!("Update {} available. Restart the app to install.", u.version);
        app.state::<AppState>()
            .notification_manager
            .info(&msg)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }
    Ok(())
}

// Tauri command handlers

#[tauri::command]
fn get_platform() -> String {
    std::env::consts::OS.to_string()
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
            keys.as_ptr() as *const *const c_void,
            values.as_ptr() as *const *const c_void,
            1,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    let dict = unsafe { CFDictionary::wrap_under_create_rule(dict_ref) };
    let _trusted = unsafe { AXIsProcessTrustedWithOptions(dict.as_CFTypeRef() as *const c_void) };
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
            "microphone" => "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
            "accessibility" => "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
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
async fn get_settings(state: tauri::State<'_, AppState>) -> Result<AppConfig, String> {
    let config = state.config.lock().await;
    let cfg = config.get_all();
    log::debug!("Returning settings, api_key present: {}", cfg.stt_config.api_key.is_some());
    Ok(cfg)
}

#[tauri::command]
async fn save_settings(
    state: tauri::State<'_, AppState>,
    new_config: AppConfig,
) -> Result<(), String> {
    log::info!("=== SAVE_SETTINGS CALLED ===");
    log::info!("API key present: {}", new_config.stt_config.api_key.is_some());
    log::info!("API key length: {:?}", new_config.stt_config.api_key.as_ref().map(|s| s.len()));
    log::info!("Audio device: {:?}", new_config.audio_device);

    // Reject if language toggle hotkey is the same as the dictation hotkey
    if let Some(ref toggle) = new_config.language_toggle_hotkey {
        if !toggle.trim().is_empty() {
            if let (Ok(dictation), Ok(toggle_hk)) = (
                parse_rdev_hotkey(&new_config.hotkey),
                parse_rdev_hotkey(toggle),
            ) {
                if dictation == toggle_hk {
                    return Err("Language toggle hotkey cannot be the same as the dictation hotkey.".to_string());
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
    if let Err(e) = state.audio_capture.lock().await.set_device(effective_device_id) {
        log::error!("Failed to set audio device: {}", e);
        return Err(format!("Failed to set audio device: {}", e));
    }
    log::info!("Audio device set to: {}", if effective_device_id.is_empty() { "default" } else { effective_device_id });

    // Persist config with None for default (so we don't store "" or "null")
    let config_to_save = AppConfig {
        audio_device: device_id_to_apply.map(String::from),
        ..new_config
    };
    
    let mut config = state.config.lock().await;
    match config.save(config_to_save) {
        Ok(_) => {
            app_log::reconfigure(config.get_all().logging.clone());
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
    Ok(state.audio_capture.lock().await.get_current_recording_level())
}

#[tauri::command]
async fn test_microphone_stop(state: tauri::State<'_, AppState>) -> Result<TestMicrophoneResult, String> {
    log::info!("Test recording stop requested");
    match state.audio_capture.lock().await.stop_and_get_test_result().await {
        Ok((level, samples, sample_rate)) => {
            log::info!(
                "Test stopped, level: {}, samples: {}",
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
    history::get_history(limit, offset).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_history() -> Result<(), String> {
    history::clear().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_snippets(state: tauri::State<'_, AppState>) -> Result<Vec<config::Snippet>, String> {
    let config = state.config.lock().await;
    Ok(config.get_snippets())
}

#[tauri::command]
async fn get_model_status() -> Result<serde_json::Value, String> {
    crate::stt::get_model_status().await.map_err(|e| e.to_string())
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
async fn add_snippet(
    state: tauri::State<'_, AppState>,
    trigger: String,
    expansion: String,
) -> Result<(), String> {
    let mut config = state.config.lock().await;
    config.add_snippet(trigger, expansion).map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_snippet(state: tauri::State<'_, AppState>, trigger: String) -> Result<(), String> {
    let mut config = state.config.lock().await;
    config.remove_snippet(&trigger).map_err(|e| e.to_string())
}

#[tauri::command]
async fn check_api_key(provider: String, api_key: String) -> Result<bool, String> {
    log::info!("check_api_key called with provider: {}, api_key length: {}", provider, api_key.len());

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



const OVERLAY_LABEL: &str = "overlay";
const OVERLAY_WIDTH: i32 = 300;
const OVERLAY_HEIGHT: i32 = 80;
const OVERLAY_BOTTOM_MARGIN: i32 = 16;

#[derive(serde::Serialize, Clone)]
#[serde(tag = "kind")]
enum OverlayEvent {
    Collapsed,
    Listening,
    ShortPress,
    Recording { level: f32 },
    Processing,
    Success,
    Error { message: String },
}

fn emit_overlay_event(app: &tauri::AppHandle, event: OverlayEvent) {
    // Emit only to the overlay window so it always receives the event
    let _ = app.emit_to(OVERLAY_LABEL, "overlay-state", event);
}

/// Show the overlay window at bottom-center of primary monitor without stealing focus.
fn show_overlay(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let overlay = app
        .get_webview_window(OVERLAY_LABEL)
        .ok_or_else(|| anyhow::anyhow!("Overlay window not found"))?;
    let win = overlay.as_ref().window();

    if let Ok(Some(monitor)) = win.primary_monitor() {
        let wa = monitor.work_area();
        let x = wa.position.x + (wa.size.width as i32 - OVERLAY_WIDTH) / 2;
        let y = wa.position.y + (wa.size.height as i32 - OVERLAY_HEIGHT) - OVERLAY_BOTTOM_MARGIN;
        overlay.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))?;
    }

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
        emit_overlay_event(&state.app_handle, OverlayEvent::Listening);

        #[cfg(windows)]
        {
            let hwnd = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
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
        tauri::async_runtime::spawn(async move {
            while is_recording_level.load(Ordering::SeqCst) {
                let level = audio_capture.lock().await.get_current_recording_level();
                emit_overlay_event(&app_handle_level, OverlayEvent::Recording { level });
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
    },
    FromProvider {
        provider: Box<dyn crate::stt::provider::STTProvider>,
        audio_data: Vec<f32>,
        sample_rate: u32,
        vad_config: VADConfig,
        language: Option<String>,
    },
}

fn run_transcribe_job(job: TranscribeJob) -> anyhow::Result<crate::stt::TranscriptionResult> {
    let language = match &job {
        TranscribeJob::FromConfig { language, .. } => language.clone(),
        TranscribeJob::FromProvider { language, .. } => language.clone(),
    };
    let language = language.as_deref();
    match job {
        TranscribeJob::FromConfig {
            stt_config,
            audio_data,
            sample_rate,
            vad_config,
            ..
        } => {
            let provider = crate::stt::provider::create_provider_sync(&stt_config)?;
            log::info!("Starting transcription with {} (chunked + prompt chaining)", provider.name());
            crate::stt::transcribe_chunked(&*provider, &audio_data, sample_rate, &vad_config, language)
        }
        TranscribeJob::FromProvider {
            provider,
            audio_data,
            sample_rate,
            vad_config,
            ..
        } => crate::stt::transcribe_chunked(&*provider, &audio_data, sample_rate, &vad_config, language),
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
        
        let (audio_data, sample_rate) = match state.audio_capture.lock().await.stop_recording().await {
            Ok(result) => result,
            Err(e) => {
                log::error!("Failed to stop recording: {}", e);
                *audio_state = AudioState::Idle;
                emit_overlay_event(&state.app_handle, OverlayEvent::Error { message: "Recording failed".to_string() });
                let app_for_overlay = state.app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                    emit_overlay_event(&app_for_overlay, OverlayEvent::Collapsed);
                });
                return;
            }
        };
        
        drop(audio_state);
        log::info!("Audio recording stopped, {} samples at {}Hz, processing...", audio_data.len(), sample_rate);
        
        let config = state.config.lock().await.get_all();
        let audio_state_ref = state.audio_state.clone();
        let last_injected_len = state.last_injected_len.clone();
        let last_injected_text = state.last_injected_text.clone();
        let app_handle = state.app_handle.clone();
        let foreground_hwnd = state.foreground_for_injection.lock().await.take();

        tokio::spawn(async move {
            let stt_config = crate::config::privacy::effective_stt_config(&config);
            let vad_config = stt_config.vad_config();
            // Create Cloud/Groq provider inside the OS thread so reqwest::blocking::Client
            // is never created/dropped on a tokio worker (avoids runtime drop panic).
            let (tx, rx) = oneshot::channel();
            let is_sync_capable = match stt_config.mode {
                crate::config::STTMode::Cloud | crate::config::STTMode::Hybrid | crate::config::STTMode::Auto => true,
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
                        emit_overlay_event(&app_handle, OverlayEvent::Error { message: "Provider failed".to_string() });
                        let _ = crate::tray::TrayManager::set_tray_state(&app_handle, AudioState::Idle);
                        let app_for_overlay = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                            emit_overlay_event(&app_for_overlay, OverlayEvent::Collapsed);
                        });
                        return;
                    }
                };
                log::info!("Starting transcription with {} (chunked + prompt chaining)", provider.name());
                TranscribeJob::FromProvider {
                    provider,
                    audio_data,
                    sample_rate,
                    vad_config,
                    language,
                }
            };
            if matches!(&job, TranscribeJob::FromConfig { .. }) {
                log::info!("Starting transcription (Cloud/Groq on OS thread, chunked + prompt chaining)");
            }
            std::thread::spawn(move || {
                let result = run_transcribe_job(job);
                let _ = tx.send(result);
            });

            match tokio::time::timeout(
                std::time::Duration::from_secs(120),
                rx,
            ).await {
                Ok(Ok(Ok(result))) => {
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
                    let _ = app_handle.emit("dictation-result", &formatted);
                    #[cfg(windows)]
                    if let Some(hwnd) = foreground_hwnd {
                        let _ = unsafe {
                            windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd as isize)
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
                            if let Err(e) = injector.inject_with_config(&formatted, &config.formatting).await {
                                log::error!("Failed to inject text: {}", e);
                            } else {
                                last_injected_len.store(formatted.len(), Ordering::SeqCst);
                                *last_injected_text.lock().await = formatted;
                            }
                        }
                    }
                }
                Ok(Ok(Err(e))) => {
                    log::error!("Transcription failed: {}", e);
                    emit_overlay_event(&app_handle, OverlayEvent::Error { message: "Transcription failed".to_string() });
                }
                Ok(Err(_)) => {
                    log::error!("Transcription thread failed or panicked");
                    emit_overlay_event(&app_handle, OverlayEvent::Error { message: "Thread failed".to_string() });
                }
                Err(_) => {
                    log::warn!("Transcription timed out after 120s");
                    emit_overlay_event(&app_handle, OverlayEvent::Error { message: "Timed out".to_string() });
                }
            }

            let mut audio_state = audio_state_ref.lock().await;
            *audio_state = AudioState::Idle;
            emit_overlay_event(&app_handle, OverlayEvent::Success);
            let _ = crate::tray::TrayManager::set_tray_state(&app_handle, AudioState::Idle);

            // Emit Collapsed after a brief delay so success state is visible
            let app_for_overlay = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
                emit_overlay_event(&app_for_overlay, OverlayEvent::Collapsed);
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
