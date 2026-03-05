export interface AppConfig {
  hotkey: string
  recording_mode: 'Hold' | 'Toggle'
  audio_device: string | null
  stt_config: STTConfig
  formatting: FormattingConfig
  privacy: PrivacyConfig
  notifications: NotificationConfig
  snippets: Snippet[]
  auto_start: boolean
  language: string
  start_in_focus: boolean
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

export interface AudioDevice {
  id: string
  name: string
  is_default: boolean
}
