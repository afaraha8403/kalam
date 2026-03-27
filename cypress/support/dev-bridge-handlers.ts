/**
 * In-browser E2E: the app calls the dev bridge at POST /api/invoke. Cypress intercepts
 * that URL and returns JSON shaped like real Tauri command results.
 */

export type MockProfile = 'onboarding' | 'main'
type MockPlatform = 'windows' | 'macos' | 'linux'

const BASE_ONBOARDING: Record<string, unknown> = {
  onboarding_complete: false,
  config_version: 1,
  hotkey: 'Ctrl+Win',
  toggle_dictation_hotkey: null,
  dictation_enabled: true,
  audio_device: '',
  stt_config: {
    mode: 'Cloud',
    provider: 'groq',
    api_keys: {},
    local_model: 'sensevoice',
    vad_preset: 'Balanced',
    audio_filter: {
      enabled: true,
      preset: 'Light',
      highpass_cutoff_hz: 80,
      noise_gate_threshold_db: -45,
      compressor_ratio: 3,
      compressor_threshold_db: -18,
      normalize_target_db: -6,
    },
  },
  formatting: {
    voice_commands: true,
    filler_word_removal: true,
    auto_punctuation: true,
    custom_rules: [],
    injection_method: 'Auto',
    keystroke_delay_ms: 10,
    clipboard_threshold: 200,
    force_clipboard_apps: [],
  },
  privacy: {
    history_retention_days: 90,
    telemetry_enabled: false,
    sensitive_app_detection: false,
    sensitive_app_patterns: [],
  },
  notifications: {
    show_completion: true,
    show_errors: true,
    show_updates: true,
    sound_enabled: false,
  },
  logging: { enabled: false, level: 'Info', max_records: 2000 },
  snippets: [],
  auto_start: false,
  languages: ['en'],
  language_toggle_hotkey: null,
  start_in_focus: true,
  min_hold_ms: 300,
  command_config: {
    enabled: false,
    hotkey: null,
    provider: null,
    api_keys: {},
    models: {},
  },
  update_channel: 'stable',
  sidebar_collapsed: false,
  theme_preference: 'Auto',
  user_email: null,
  onboarding_os_name: null,
  onboarding_os_version: null,
  notifications_opt_in: true,
  marketing_opt_in: false,
  waveform_style: 'Aurora',
  overlay_position: 'BottomCenter',
  overlay_offset_x: 0,
  overlay_offset_y: 0,
  overlay_expand_direction: 'Up',
}

const BASE_MAIN: Record<string, unknown> = {
  ...BASE_ONBOARDING,
  onboarding_complete: true,
}

let mockConfig: Record<string, unknown> = structuredClone(BASE_MAIN)
let mockPlatform: MockPlatform = 'windows'

type MockPermissionStatus = {
  platform: string
  microphone: { state: string; actionable: boolean; message: string }
  accessibility: { state: string; actionable: boolean; message: string }
  input_monitoring: { state: string; actionable: boolean; message: string }
}

type MockRuntimeCapabilities = {
  can_capture_audio: boolean
  can_text_inject: boolean
  can_global_hotkey: boolean
  capture_audio_state: string
  text_inject_state: string
  global_hotkey_state: string
  next_steps: string[]
  permission_status: MockPermissionStatus
}

function defaultPermissionStatus(platform: MockPlatform): MockPermissionStatus {
  if (platform === 'macos') {
    return {
      platform: 'macos',
      microphone: {
        state: 'unknown',
        actionable: true,
        message: 'macOS shows the microphone prompt when recording starts. Run a mic test to confirm access.',
      },
      accessibility: {
        state: 'needs_action',
        actionable: true,
        message: 'Enable Accessibility so Kalam can insert text into other apps.',
      },
      input_monitoring: {
        state: 'unknown',
        actionable: true,
        message: 'Input Monitoring is usually prompted when global shortcuts are captured.',
      },
    }
  }
  if (platform === 'linux') {
    return {
      platform: 'linux',
      microphone: {
        state: 'unknown',
        actionable: true,
        message: 'Linux permissions vary by distribution and audio stack.',
      },
      accessibility: {
        state: 'unknown',
        actionable: true,
        message: 'Linux accessibility/injection support varies by desktop environment.',
      },
      input_monitoring: {
        state: 'unknown',
        actionable: true,
        message: 'Global hotkey support varies by desktop environment and compositor.',
      },
    }
  }
  return {
    platform: 'windows',
    microphone: {
      state: 'unknown',
      actionable: true,
      message: 'If recording fails, check Windows Privacy > Microphone access for this app.',
    },
    accessibility: {
      state: 'granted',
      actionable: false,
      message: 'No separate accessibility toggle is usually required on Windows.',
    },
    input_monitoring: {
      state: 'granted',
      actionable: false,
      message: 'No separate Input Monitoring permission is required on Windows.',
    },
  }
}

function defaultRuntimeCapabilities(platform: MockPlatform): MockRuntimeCapabilities {
  const permission_status = defaultPermissionStatus(platform)
  return {
    can_capture_audio: true,
    can_text_inject: platform !== 'macos',
    can_global_hotkey: platform !== 'macos',
    capture_audio_state: permission_status.microphone.state,
    text_inject_state: permission_status.accessibility.state,
    global_hotkey_state: permission_status.input_monitoring.state,
    next_steps: [],
    permission_status,
  }
}

let mockPermissionStatus: MockPermissionStatus = defaultPermissionStatus(mockPlatform)
let mockRuntimeCapabilities: MockRuntimeCapabilities = defaultRuntimeCapabilities(mockPlatform)

export function resetMockConfig(profile: MockProfile = 'main') {
  mockConfig = structuredClone(profile === 'onboarding' ? BASE_ONBOARDING : BASE_MAIN)
  mockPlatform = 'windows'
  mockPermissionStatus = defaultPermissionStatus(mockPlatform)
  mockRuntimeCapabilities = defaultRuntimeCapabilities(mockPlatform)
}

export function setMockPlatform(platform: MockPlatform) {
  mockPlatform = platform
  mockPermissionStatus = defaultPermissionStatus(platform)
  mockRuntimeCapabilities = defaultRuntimeCapabilities(platform)
}

export function setMockRuntimeCapabilities(next: Partial<MockRuntimeCapabilities>) {
  mockRuntimeCapabilities = {
    ...mockRuntimeCapabilities,
    ...next,
  }
  if (next.permission_status) {
    mockPermissionStatus = next.permission_status
  } else {
    mockPermissionStatus = mockRuntimeCapabilities.permission_status
  }
}

function modelStatusEntry() {
  return {
    installed: false,
    size_mb: 0,
    status: 'NotInstalled',
    error: null,
    label: 'SenseVoice',
    quality: '',
    languages: '',
    rss_bytes: null as number | null,
  }
}

function hardwareOk() {
  return { can_run: true, reason: null }
}

/** Parse { cmd, args } from Cypress intercept body. */
export function handleDevBridgeInvoke(body: { cmd?: string; args?: Record<string, unknown> }): unknown {
  const cmd = body.cmd ?? ''
  const args = body.args ?? {}

  switch (cmd) {
    case 'get_settings':
      return mockConfig
    case 'save_settings': {
      const next = args.newConfig as Record<string, unknown> | undefined
      if (next) {
        mockConfig = structuredClone(next)
      }
      return null
    }
    case 'skip_onboarding_with_defaults': {
      mockConfig = structuredClone({ ...mockConfig, onboarding_complete: true })
      return null
    }
    case 'get_platform':
      return mockPlatform
    case 'get_permission_status':
      return mockPermissionStatus
    case 'get_runtime_capabilities':
      return mockRuntimeCapabilities
    case 'get_os_release_info':
      return { name: 'Windows', version: '10.0 (e2e mock)' }
    case 'get_audio_devices':
      return []
    case 'get_db_status':
      return { ok: true }
    case 'get_aggregate_stats':
      return { last_latency_ms: null }
    case 'get_dashboard_stats':
      return {
        words_dictated_7d: [],
        top_apps_7d: [],
        total_time_dictating_7d_ms: 0,
        dictation_flow_7d: [],
        session_lengths_7d_ms: [],
        avg_words_per_dictation_7d: null,
        activity_heatmap_14d: [],
        streak_days: 0,
        total_words: 0,
      }
    case 'get_history':
      return []
    case 'search_history':
      return []
    case 'clear_history':
      return null
    case 'get_tasks_due_on':
    case 'get_reminders_due_on':
      return []
    case 'check_model_requirements':
      return hardwareOk()
    case 'is_sidecar_available_for_model':
      return true
    case 'get_model_status':
      return {
        sensevoice: modelStatusEntry(),
        whisper_base: { ...modelStatusEntry(), label: 'Whisper Base' },
      }
    case 'is_sidecar_installed_for_model':
      return false
    case 'stop_all_local_models':
    case 'start_local_model':
    case 'stop_local_model':
    case 'restart_local_model':
    case 'open_system_permission_page':
    case 'request_system_permission':
      return null
    case 'get_app_log_empty':
      return true
    case 'get_app_data_path':
      return 'C:\\temp\\kalam-e2e'
    case 'get_note_scope_counts':
      return { active: 0, archived: 0, trash: 0 }
    case 'get_dictionary_entries':
      return []
    case 'update_dictionary_entry':
      return null
    case 'focus_main_window':
      return null
    case 'check_api_key':
      return true
    case 'test_microphone_start':
      return null
    case 'test_microphone_level':
      return 0
    case 'test_microphone_stop':
      return { level: 0, samples: [0.01, -0.01, 0.02], sample_rate: 16000 }
    default:
      // eslint-disable-next-line no-console
      console.warn('[cypress dev-bridge mock] unhandled cmd:', cmd)
      return null
  }
}
