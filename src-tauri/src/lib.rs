mod audio;
mod config;
mod formatting;
mod history;
mod hotkey;
mod injection;
mod notifications;
mod stt;
mod tray;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;
use tokio::sync::Mutex;

use crate::audio::{AudioState, play_sound};
use crate::config::{AppConfig, ConfigManager};
use crate::notifications::NotificationManager;
use crate::tray::TrayManager;
use crate::hotkey::{parse_rdev_hotkey, start_listener};

// Simplified AppState for compilation
pub struct AppState {
    pub config: Arc<Mutex<ConfigManager>>,
    pub notification_manager: Arc<NotificationManager>,
    pub audio_state: Arc<Mutex<AudioState>>,
    pub app_handle: tauri::AppHandle,
    pub is_recording: Arc<AtomicBool>,
    pub audio_capture: Arc<Mutex<crate::audio::capture::AudioCapture>>,
}

impl AppState {
    pub fn new(app_handle: tauri::AppHandle) -> anyhow::Result<Self> {
        let config = Arc::new(Mutex::new(ConfigManager::new()?));
        let notification_manager = Arc::new(NotificationManager::new(app_handle.clone()));
        let audio_state = Arc::new(Mutex::new(AudioState::Idle));
        let is_recording = Arc::new(AtomicBool::new(false));
        let mut audio_capture = crate::audio::capture::AudioCapture::new()?;
        
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

        Ok(Self {
            config,
            notification_manager,
            audio_state,
            app_handle,
            is_recording,
            audio_capture,
        })
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize app state
            let state = AppState::new(app.handle().clone())?;
            
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
            
            // Register global hotkey via rdev
            let app_handle = app.handle().clone();
            let (hotkey_str, recording_mode) = {
                let state = app.state::<AppState>();
                let config = state.config.blocking_lock();
                let cfg = config.get_all();
                (cfg.hotkey.clone(), cfg.recording_mode)
            };
            
            if let Ok(target_hotkey) = parse_rdev_hotkey(&hotkey_str) {
                let app_handle_press = app_handle.clone();
                let is_recording_press = is_recording_flag.clone();
                
                let app_handle_release = app_handle.clone();
                let is_recording_release = is_recording_flag.clone();
                let mode_for_release = recording_mode.clone();

                start_listener(
                    target_hotkey,
                    move || {
                        let app_handle = app_handle_press.clone();
                        let is_recording = is_recording_press.clone();
                        
                        let rt = tokio::runtime::Handle::current();
                        rt.spawn(async move {
                            let state = app_handle.state::<AppState>();
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
                    },
                    move || {
                        if matches!(mode_for_release, crate::config::RecordingMode::Hold) {
                            let app_handle = app_handle_release.clone();
                            let is_recording = is_recording_release.clone();
                            
                            let rt = tokio::runtime::Handle::current();
                            rt.spawn(async move {
                                let state = app_handle.state::<AppState>();
                                stop_dictation(state, is_recording).await;
                            });
                        }
                    }
                );
                
                log::info!("Global hotkey registered via rdev: {} (mode: {:?})", hotkey_str, recording_mode);
            } else {
                log::error!("Failed to parse hotkey: {}", hotkey_str);
            }

            log::info!("Kalam Voice initialized successfully");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            get_audio_devices,
            test_microphone,
            get_history,
            clear_history,
            get_snippets,
            add_snippet,
            remove_snippet,
            check_api_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Tauri command handlers
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
    newConfig: AppConfig,
) -> Result<(), String> {
    log::info!("=== SAVE_SETTINGS CALLED ===");
    log::info!("API key present: {}", newConfig.stt_config.api_key.is_some());
    log::info!("API key length: {:?}", newConfig.stt_config.api_key.as_ref().map(|s| s.len()));
    log::info!("Audio device: {:?}", newConfig.audio_device);
    
    // Apply audio device change if specified
    if let Some(ref device_id) = newConfig.audio_device {
        if let Err(e) = state.audio_capture.lock().await.set_device(device_id) {
            log::error!("Failed to set audio device: {}", e);
            return Err(format!("Failed to set audio device: {}", e));
        }
        log::info!("Audio device set to: {}", device_id);
    }
    
    let mut config = state.config.lock().await;
    match config.save(newConfig) {
        Ok(_) => {
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
async fn test_microphone(state: tauri::State<'_, AppState>) -> Result<f32, String> {
    log::info!("Testing microphone...");
    match state.audio_capture.lock().await.test_microphone().await {
        Ok(level) => {
            log::info!("Microphone test completed, level: {}", level);
            Ok(level)
        }
        Err(e) => {
            log::error!("Microphone test failed: {}", e);
            Err(format!("Microphone test failed: {}", e))
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
async fn check_api_key(provider: String, apiKey: String) -> Result<bool, String> {
    log::info!("check_api_key called with provider: {}, apiKey length: {}", provider, apiKey.len());

    if apiKey.is_empty() {
        log::warn!("API key is empty");
        return Ok(false);
    }

    match stt::validate_api_key(&provider, &apiKey).await {
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



async fn start_dictation(state: tauri::State<'_, AppState>, is_recording: Arc<AtomicBool>) {
    // Check if already recording
    if is_recording.load(Ordering::SeqCst) {
        log::debug!("Already recording, ignoring start request");
        return;
    }
    
    let mut audio_state = state.audio_state.lock().await;
    
    if matches!(*audio_state, AudioState::Idle) {
        log::info!("Starting dictation...");
        *audio_state = AudioState::Recording;
        is_recording.store(true, Ordering::SeqCst);
        
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
        drop(audio_state);
        
        log::info!("Audio recording started");
    } else {
        log::debug!("Cannot start dictation, current state: {:?}", *audio_state);
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
        
        // Play end sound
        let app_handle = state.app_handle.clone();
        
        if let Err(e) = play_sound(&app_handle, "dictation-ended") {
            log::warn!("Failed to play dictation end sound: {}", e);
        }
        
        let audio_data = match state.audio_capture.lock().await.stop_recording().await {
            Ok(data) => data,
            Err(e) => {
                log::error!("Failed to stop recording: {}", e);
                *audio_state = AudioState::Idle;
                return;
            }
        };
        
        drop(audio_state);
        log::info!("Audio recording stopped, processing...");
        
        let config = state.config.lock().await.get_all();
        let audio_state_ref = state.audio_state.clone();
        
        tokio::spawn(async move {
            if let Some(api_key) = config.stt_config.api_key {
                log::info!("Starting transcription with Groq");
                let transcribe_task = tokio::task::spawn_blocking(move || {
                    let provider = crate::stt::groq::GroqProvider::new(api_key)?;
                    use crate::stt::provider::STTProvider;
                    provider.transcribe_blocking(&audio_data)
                });
                
                match transcribe_task.await {
                    Ok(Ok(result)) => {
                        log::info!("Transcription successful: {}", result.text);
                        if let Ok(injector) = crate::injection::TextInjector::new() {
                            if let Err(e) = injector.inject(&result.text).await {
                                log::error!("Failed to inject text: {}", e);
                            }
                        }
                    }
                    Ok(Err(e)) => log::error!("Transcription failed: {}", e),
                    Err(e) => log::error!("Transcription task failed: {}", e),
                }
            } else {
                log::warn!("No API key found for STT provider");
            }
            
            let mut audio_state = audio_state_ref.lock().await;
            *audio_state = AudioState::Idle;
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
