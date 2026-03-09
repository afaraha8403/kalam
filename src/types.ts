export type LogLevel = 'Off' | 'Error' | 'Warn' | 'Info' | 'Debug'

export interface LoggingConfig {
  enabled: boolean
  level: LogLevel
  max_records: number
}

export type WaveformStyle = 'Line' | 'Symmetric' | 'Heartbeat' | 'Snake' | 'DoubleHelix' | 'Liquid' | 'Waves' | 'Glitch' | 'Bars' | 'CenterSplit'
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
  waveform_style?: WaveformStyle
  overlay_position?: OverlayPosition
  overlay_offset_x?: number
  overlay_offset_y?: number
  overlay_expand_direction?: ExpandDirection
  command_config: CommandConfig
  /** Update channel: stable or beta (pre-releases). */
  update_channel?: 'stable' | 'beta'
}

export interface STTConfig {
  mode: 'Cloud' | 'Local' | 'Hybrid' | 'Auto'
  provider: string
  api_key: string | null
  local_model: string | null
  vad_preset: 'Fast' | 'Balanced' | 'Accurate'
}

export interface FormattingConfig {
  voice_commands: boolean
  filler_word_removal: boolean
  auto_punctuation: boolean
  custom_rules: FormattingRule[]
  injection_method: 'Auto' | 'Keystrokes' | 'Clipboard'
  keystroke_delay_ms: number
  clipboard_threshold: number
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
  action: 'ForceLocal' | 'Block' | 'RequireConfirmation'
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
  mode: string
  language: string
  duration_ms: number | null
}

export interface AggregateStats {
  streak_days: number
  total_words: number
  time_saved_hours: number
  last_latency_ms: number | null
  today_avg_latency_ms: number | null
}

export interface DictionaryEntry {
  id: string
  term: string
  created_at: string
}

// Unified entry (notes, tasks, reminders, history) for SQLite + vector DB
export type EntryType = 'history' | 'note' | 'task' | 'reminder'

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
