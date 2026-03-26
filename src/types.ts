export type LogLevel = 'Off' | 'Error' | 'Warn' | 'Info' | 'Debug'

export interface LoggingConfig {
  enabled: boolean
  level: LogLevel
  max_records: number
}

/** Shell theme: fixed palettes or match OS (`prefers-color-scheme`). */
export type ThemePreference = 'Light' | 'Dark' | 'Auto'

export type WaveformStyle =
  | 'SiriWave'
  | 'EchoRing'
  | 'RoundedBars'
  | 'BreathingAura'
  | 'Oscilloscope'
  | 'NeonPulse'
  | 'Aurora'
export type OverlayPosition = 'BottomCenter' | 'BottomLeft' | 'BottomRight' | 'TopCenter' | 'TopLeft' | 'TopRight' | 'CenterLeft' | 'CenterRight' | 'Center'
export type ExpandDirection = 'Up' | 'Down' | 'Center'

export type CommandModeProvider = 'groq' | 'openrouter' | 'gemini' | 'openai' | 'anthropic'

export interface CommandConfig {
  enabled: boolean
  hotkey: string | null
  provider: CommandModeProvider | null
  api_keys: Record<string, string>
  models: Record<string, string>
}

export interface AppConfig {
  hotkey: string | null
  /** Schema version for migrations. */
  config_version?: number
  toggle_dictation_hotkey: string | null
  recording_mode?: 'Hold' | 'Toggle' | null
  /** Master switch: when false, hotkeys and transcription are disabled. */
  dictation_enabled: boolean
  audio_device: string | null
  stt_config: STTConfig
  formatting: FormattingConfig
  privacy: PrivacyConfig
  notifications: NotificationConfig
  logging: LoggingConfig
  snippets: Snippet[]
  auto_start: boolean
  /** Ordered list of recognition languages. First is default; toggle hotkey swaps first and second. */
  languages: string[]
  language_toggle_hotkey: string | null
  start_in_focus: boolean
  min_hold_ms: number
  onboarding_complete?: boolean
  /** User email (required at onboarding). */
  user_email?: string | null
  /** Opt-in to marketing. Default false. */
  marketing_opt_in?: boolean
  /** Opt-in to product notifications and updates. Default false. */
  notifications_opt_in?: boolean
  /** Filled when the user completes the email step; from the OS (support / diagnostics). */
  onboarding_os_name?: string | null
  onboarding_os_version?: string | null
  waveform_style?: WaveformStyle
  overlay_position?: OverlayPosition
  overlay_offset_x?: number
  overlay_offset_y?: number
  overlay_expand_direction?: ExpandDirection
  command_config: CommandConfig
  /** Update channel: stable or beta (pre-releases). */
  update_channel?: 'stable' | 'beta',
  /** When true, left sidebar is collapsed to icon-only; persisted across restarts. */
  sidebar_collapsed?: boolean
  /** Light / Dark always; Auto follows system appearance. */
  theme_preference?: ThemePreference
}

export type AudioFilterPreset = 'Off' | 'Light' | 'Moderate' | 'Custom'

export interface AudioFilterConfig {
  enabled: boolean
  preset: AudioFilterPreset
  highpass_cutoff_hz: number
  noise_gate_threshold_db: number
  compressor_ratio: number
  compressor_threshold_db: number
  normalize_target_db: number
}

/** Mirrors Rust `TranscriptionTimeoutConfig` (serde snake_case). */
export interface TranscriptionTimeoutConfig {
  timeout_min_seconds_cloud: number
  timeout_min_seconds_local: number
  timeout_max_seconds: number
  timeout_multiplier: number
  timeout_buffer_seconds: number
}

export interface STTConfig {
  mode: 'Cloud' | 'Local' | 'Hybrid' | 'Auto'
  provider: string
  api_keys: Record<string, string>
  /** Legacy fallback; migrated into api_keys. */
  api_key?: string | null
  local_model: string | null
  vad_preset: 'Fast' | 'Balanced' | 'Accurate'
  audio_filter: AudioFilterConfig
  /** Dynamic STT timeouts; optional for older saved configs (Rust fills defaults on load). */
  transcription_timeout?: TranscriptionTimeoutConfig
}

export interface FormattingConfig {
  voice_commands: boolean
  filler_word_removal: boolean
  auto_punctuation: boolean
  custom_rules: FormattingRule[]
  injection_method: 'Auto' | 'Keystrokes' | 'Clipboard'
  keystroke_delay_ms: number
  clipboard_threshold: number
  /** Clipboard-injection retry count (Rust `injection` module). */
  retry_attempts?: number
  retry_delay_ms?: number
  force_clipboard_apps: string[]
}

export interface FormattingRule {
  pattern: string
  replacement: string
  enabled: boolean
}

export interface PrivacyConfig {
  history_retention_days: number
  telemetry_enabled: boolean
  sensitive_app_detection: boolean
  sensitive_app_patterns: SensitiveAppPattern[]
}

export interface SensitiveAppPattern {
  pattern: string
  pattern_type: 'ProcessName' | 'WindowTitle' | 'BundleId'
  /** Only `ForceLocal` is supported; legacy configs may still deserialize extra values as ForceLocal in Rust. */
  action: 'ForceLocal'
}

export interface NotificationConfig {
  show_completion: boolean
  show_errors: boolean
  show_updates: boolean
  sound_enabled: boolean
}

export interface Snippet {
  trigger: string
  expansion: string
}

export interface HistoryEntry {
  id: string
  text: string
  created_at: string
  /** `dictation` or `command` (from backend session_mode). */
  mode: string
  language: string
  duration_ms: number | null
  /** Foreground app process name when captured (e.g. notepad.exe). */
  target_app?: string | null
  /** Friendly app title when resolved (e.g. "Notepad"). */
  target_app_name?: string | null
  /** Effective STT path when captured (omitted on legacy rows). */
  stt_mode?: 'Cloud' | 'Local' | 'Hybrid' | 'Auto' | string | null
  /** Stored at save time; omit on legacy rows. */
  word_count?: number | null
  /** Wall-clock STT latency for this session (ms). */
  stt_latency_ms?: number | null
  /** STT provider id when captured (e.g. groq); omitted on older rows. */
  stt_provider?: string | null
}

export interface AggregateStats {
  streak_days: number
  total_words: number
  time_saved_hours: number
  last_latency_ms: number | null
  today_avg_latency_ms: number | null
}

export interface WordsByDay {
  date: string
  words: number
}

export interface AppCountRow {
  app: string
  count: number
}

export interface FlowPoint {
  created_at: string
  target_app?: string | null
}

export interface ActivityDay {
  date: string
  count: number
}

/** Payload from `get_dashboard_stats` (Overview charts). */
export interface DashboardStats {
  words_dictated_7d: WordsByDay[]
  top_apps_7d: AppCountRow[]
  total_time_dictating_7d_ms: number
  dictation_flow_7d: FlowPoint[]
  session_lengths_7d_ms: number[]
  /** Mean words per timed dictation (same rows as `session_lengths_7d_ms`), last 7 days. */
  avg_words_per_dictation_7d: number | null
  activity_heatmap_14d: ActivityDay[]
  streak_days: number
  total_words: number
}

export interface DictionaryEntry {
  id: string
  term: string
  created_at: string
}

// Unified entry (notes, tasks, history) for SQLite + vector DB. Reminders are `reminder_at` on notes/tasks.
export type EntryType = 'history' | 'note' | 'task'

export interface Subtask {
  title: string
  is_completed: boolean
}

export interface Entry {
  id: string
  entry_type: EntryType
  created_at: string
  updated_at: string
  sync_status: string
  title: string | null
  content: string
  attachments: string[]
  tags: string[]
  color: string | null
  is_pinned: boolean
  priority: number | null
  due_date: string | null
  subtasks: Subtask[] | null
  is_completed: boolean | null
  reminder_at: string | null
  rrule: string | null
  archived_at: string | null
  deleted_at: string | null
  target_app?: string | null
  target_app_name?: string | null
  duration_ms?: number | null
  word_count?: number | null
  stt_latency_ms?: number | null
  stt_mode?: string | null
  dictation_language?: string | null
  session_mode?: string | null
  stt_provider?: string | null
  /** Manual order: pinned / unpinned groups for notes; Tasks list order for `entry_type === 'task'`. */
  note_order?: number
}

export interface AppLogRow {
  id: string
  level: string
  message: string
  module: string
  timestamp: string
}

export interface AudioDevice {
  id: string
  name: string
  is_default: boolean
}
