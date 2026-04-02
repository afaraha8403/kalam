/**
 * Runtime dictation phase for the main-window status bar.
 * The overlay lives in a separate Tauri webview; Rust emits `overlay-state-broadcast`
 * so the shell can mirror overlay lifecycle without shared JS memory.
 */
import { writable } from 'svelte/store'

export type DictationPhase =
  | 'idle'
  | 'listening'
  | 'recording'
  | 'processing'
  | 'error'
  | 'status'
  | 'cancelling'

export interface DictationRuntimeSnapshot {
  phase: DictationPhase
  /** Mic level 0–1 while recording (from overlay payload). */
  audioLevel: number
  isCommand: boolean
  /** Voice-activated editing hotkey session (Phase 5). */
  isVoiceEdit: boolean
  processingElapsedSec: number
  processingExpectedSec: number
  processingAttempt: number
  processingMessage: string | null
  errorMessage: string | null
  statusMessage: string | null
  statusHighlight: string | null
  /** Wall-clock duration while in recording phase (client-side). */
  recordingDurationSec: number
}

const baseSnapshot = (): DictationRuntimeSnapshot => ({
  phase: 'idle',
  audioLevel: 0,
  isCommand: false,
  isVoiceEdit: false,
  processingElapsedSec: 0,
  processingExpectedSec: 120,
  processingAttempt: 1,
  processingMessage: null,
  errorMessage: null,
  statusMessage: null,
  statusHighlight: null,
  recordingDurationSec: 0,
})

export const dictationRuntimeStore = writable<DictationRuntimeSnapshot>(baseSnapshot())

let recordingInterval: ReturnType<typeof setInterval> | null = null
let recordingStartedAt: number | null = null

function clearRecordingWallClock() {
  if (recordingInterval) {
    clearInterval(recordingInterval)
    recordingInterval = null
  }
  recordingStartedAt = null
}

function startRecordingWallClock() {
  clearRecordingWallClock()
  recordingStartedAt = Date.now()
  dictationRuntimeStore.update((s) => ({ ...s, recordingDurationSec: 0 }))
  recordingInterval = setInterval(() => {
    if (recordingStartedAt == null) return
    const sec = Math.floor((Date.now() - recordingStartedAt) / 1000)
    dictationRuntimeStore.update((s) => ({ ...s, recordingDurationSec: sec }))
  }, 400)
}

/** Payload matches Rust `OverlayEvent` JSON (`#[serde(tag = "kind")]`). */
export function applyOverlayBroadcast(payload: unknown): void {
  if (!payload || typeof payload !== 'object') return
  const p = payload as Record<string, unknown>
  const kind = p.kind
  if (typeof kind !== 'string') return

  switch (kind) {
    case 'Hidden':
    case 'Dormant':
    case 'Collapsed':
    case 'Success':
    case 'ShortPress':
    case 'SensitiveAppPeek':
      clearRecordingWallClock()
      dictationRuntimeStore.set(baseSnapshot())
      return
    case 'Listening':
      clearRecordingWallClock()
      dictationRuntimeStore.set({ ...baseSnapshot(), phase: 'listening' })
      return
    case 'Recording': {
      const level = typeof p.level === 'number' ? Math.min(1, Math.max(0, p.level)) : 0
      const isCommand = p.is_command === true
      const isVoiceEdit = p.is_voice_edit === true
      if (recordingStartedAt == null) startRecordingWallClock()
      dictationRuntimeStore.update((s) => ({
        ...s,
        phase: 'recording',
        audioLevel: level,
        isCommand: isCommand,
        isVoiceEdit,
      }))
      return
    }
    case 'Processing':
      clearRecordingWallClock()
      dictationRuntimeStore.update((s) => ({
        ...baseSnapshot(),
        phase: 'processing',
        isVoiceEdit: p.is_voice_edit === true,
        processingElapsedSec: typeof p.elapsed_secs === 'number' ? p.elapsed_secs : 0,
        processingExpectedSec: typeof p.expected_secs === 'number' ? p.expected_secs : 120,
        processingAttempt: typeof p.attempt === 'number' ? p.attempt : 1,
        processingMessage:
          typeof p.message === 'string' ? p.message : p.message == null ? null : null,
      }))
      return
    case 'Error':
      clearRecordingWallClock()
      dictationRuntimeStore.set({
        ...baseSnapshot(),
        phase: 'error',
        errorMessage: typeof p.message === 'string' ? p.message : 'Error',
      })
      return
    case 'Status':
      clearRecordingWallClock()
      dictationRuntimeStore.set({
        ...baseSnapshot(),
        phase: 'status',
        statusMessage: typeof p.message === 'string' ? p.message : '',
        statusHighlight:
          typeof p.highlight === 'string' ? p.highlight : p.highlight == null ? null : null,
      })
      return
    case 'Cancelling':
      clearRecordingWallClock()
      dictationRuntimeStore.set({ ...baseSnapshot(), phase: 'cancelling' })
      return
    default:
      return
  }
}
