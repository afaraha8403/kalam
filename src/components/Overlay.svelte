<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { invoke } from '$lib/backend'
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { listen } from '@tauri-apps/api/event'
  import type {
    AppConfig,
    AutoActivateSwitchedPayload,
    DictationMode,
    OverlayActivePreference,
    WaveformStyle,
    ExpandDirection,
  } from '../types'
  import { formatHotkeyForDisplay } from '$lib/platformHotkey'
  import { defaultAccentForModeId, effectiveModeAccent } from '$lib/modeAccent'

  invoke('trace_latency', { event: 'T6', jsTimestamp: Date.now() * 1000 }).catch(() => {})

  const KINDS = [
    'Hidden',
    'Dormant',
    'Listening',
    'ShortPress',
    'Recording',
    'Processing',
    'Success',
    'Error',
    'Status',
    'Cancelling',
    'SensitiveAppPeek',
  ] as const
  type OverlayEvent =
    | { kind: 'Hidden' }
    | {
        kind: 'Dormant'
        mode_name?: string
        /** CSS color from config (active mode accent). */
        accent_color?: string
        sensitive_app?: boolean
        last_transcription_preview?: string | null
      }
    | { kind: 'Listening'; sensitive_app?: boolean }
    | { kind: 'ShortPress' }
    | {
        kind: 'Recording'
        level: number
        is_command: boolean
        is_voice_edit?: boolean
        voice_edit_has_selection?: boolean
        sensitive_app?: boolean
        context_active?: boolean
        context_label?: string | null
        context_sources?: string[]
      }
    | {
        kind: 'Processing'
        elapsed_secs?: number
        expected_secs?: number
        attempt?: number
        message?: string | null
        is_voice_edit?: boolean
        context_active?: boolean
        context_label?: string | null
        context_sources?: string[]
      }
    | { kind: 'Success' }
    | { kind: 'Error'; message: string }
    | { kind: 'Status'; message: string; highlight?: string }
    | { kind: 'Cancelling' }
    | { kind: 'SensitiveAppPeek' }

  let state: OverlayEvent = { kind: 'Hidden' }
  let firstAppearance = true
  let waveformStyle: WaveformStyle = 'Aurora'
  let expandDirection: ExpandDirection = 'Up'
  let hotkeyStr = ''
  let overlayPlatform = 'windows'
  let activeModeName = 'Default'
  let activeModeAccent = defaultAccentForModeId('default')

  function syncActiveModeFromConfig(cfg: AppConfig) {
    const id = cfg.active_mode_id ?? 'default'
    const m = cfg.modes.find((x) => x.id === id)
    activeModeName = m?.name ?? 'Default'
    activeModeAccent = m ? effectiveModeAccent(m) : defaultAccentForModeId(id)
  }

  $: hotkeyDisplayStr =
    hotkeyStr.trim() !== '' ? formatHotkeyForDisplay(hotkeyStr, overlayPlatform) : ''

  let currentTime = ''
  function updateClock() {
    const now = new Date()
    currentTime = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  }
  updateClock()
  const clockInterval = setInterval(updateClock, 30_000)

  function isValidPayload(p: unknown): p is OverlayEvent {
    if (!p || typeof p !== 'object') return false
    const k = (p as { kind?: string }).kind
    if (typeof k !== 'string' || !KINDS.includes(k as typeof KINDS[number])) return false
    if (k === 'Dormant') {
      const d = p as { mode_name?: unknown; accent_color?: unknown; sensitive_app?: unknown; last_transcription_preview?: unknown }
      if (d.mode_name !== undefined && typeof d.mode_name !== 'string') return false
      if (d.accent_color !== undefined && typeof d.accent_color !== 'string') return false
      if (d.sensitive_app !== undefined && typeof d.sensitive_app !== 'boolean') return false
      if (
        d.last_transcription_preview !== undefined &&
        d.last_transcription_preview !== null &&
        typeof d.last_transcription_preview !== 'string'
      )
        return false
    }
    if (k === 'Listening') {
      const li = p as { sensitive_app?: unknown }
      if (li.sensitive_app !== undefined && typeof li.sensitive_app !== 'boolean') return false
    }
    if (k === 'Recording') {
      const rec = p as { level?: unknown, is_command?: unknown, is_voice_edit?: unknown, voice_edit_has_selection?: unknown, sensitive_app?: unknown, context_active?: unknown, context_label?: unknown, context_sources?: unknown }
      if (rec.level !== undefined && typeof rec.level !== 'number') return false
      if (rec.is_command !== undefined && typeof rec.is_command !== 'boolean') return false
      if (rec.is_voice_edit !== undefined && typeof rec.is_voice_edit !== 'boolean') return false
      if (rec.voice_edit_has_selection !== undefined && typeof rec.voice_edit_has_selection !== 'boolean') return false
      if (rec.sensitive_app !== undefined && typeof rec.sensitive_app !== 'boolean') return false
      if (rec.context_active !== undefined && typeof rec.context_active !== 'boolean') return false
      if (rec.context_label !== undefined && rec.context_label !== null && typeof rec.context_label !== 'string') return false
      if (rec.context_sources !== undefined && !Array.isArray(rec.context_sources)) return false
    }
    if (k === 'Processing') {
      const proc = p as { elapsed_secs?: unknown, expected_secs?: unknown, attempt?: unknown, message?: unknown, is_voice_edit?: unknown, context_active?: unknown, context_label?: unknown, context_sources?: unknown }
      if (proc.elapsed_secs !== undefined && typeof proc.elapsed_secs !== 'number') return false
      if (proc.expected_secs !== undefined && typeof proc.expected_secs !== 'number') return false
      if (proc.attempt !== undefined && typeof proc.attempt !== 'number') return false
      if (proc.message !== undefined && proc.message !== null && typeof proc.message !== 'string') return false
      if (proc.is_voice_edit !== undefined && typeof proc.is_voice_edit !== 'boolean') return false
      if (proc.context_active !== undefined && typeof proc.context_active !== 'boolean') return false
      if (proc.context_label !== undefined && proc.context_label !== null && typeof proc.context_label !== 'string') return false
      if (proc.context_sources !== undefined && !Array.isArray(proc.context_sources)) return false
    }
    return true
  }

  async function cancelTranscription() {
    try {
      await invoke('cancel_transcription')
    } catch (e) {
      console.error('Cancel transcription failed:', e)
    }
  }

  function makeLocalDormant(): OverlayEvent {
    return {
      kind: 'Dormant',
      mode_name: activeModeName,
      accent_color: activeModeAccent,
      sensitive_app: false,
      last_transcription_preview: null,
    }
  }

  /** Dormant pill dot: `overlay-state` payload or last `settings_updated` sync. */
  $: dormantDotBackground =
    state.kind === 'Dormant' ? (state.accent_color?.trim() || activeModeAccent) : ''

  $: if (firstAppearance && state.kind !== 'Hidden') {
    setTimeout(() => { firstAppearance = false }, 700)
  }

  async function goToDormantIdle() {
    // Save current tier before going dormant (for hover restore)
    if (overlayLayoutTier !== 'Dormant') {
      lastTier = overlayLayoutTier
    }
    // Resize HWND first so `%`-based pill width matches the tier before CSS transitions run (avoids mid-animation jumps).
    await invoke('resize_overlay_to', { size: 'Dormant' }).catch(console.error)
    overlayLayoutTier = 'Dormant'
    state = makeLocalDormant()
  }

  function dismissOverlayMessage() {
    if (statusTimeout) clearTimeout(statusTimeout)
    statusTimeout = null
    void goToDormantIdle()
  }

  async function retryAfterError() {
    dismissOverlayMessage()
    try {
      await invoke('ui_toggle_dictation')
    } catch (e) {
      console.error('retryAfterError failed:', e)
    }
  }

  $: processingElapsed = state.kind === 'Processing' ? (state.elapsed_secs ?? 0) : 0
  $: processingExpected = state.kind === 'Processing' ? (state.expected_secs ?? 120) : 120
  $: processingAttempt = state.kind === 'Processing' ? (state.attempt ?? 1) : 1
  $: processingMessage = state.kind === 'Processing' ? (state.message ?? null) : null
  $: processingPastHalfExpected =
    state.kind === 'Processing' &&
    processingElapsed >= Math.max(8, Math.floor(processingExpected / 2))

  /** Show elapsed counter after 2s of processing. */
  $: showElapsedCounter = state.kind === 'Processing' && processingElapsed >= 2

  /** Friendly processing label that escalates with time. */
  $: processingLabel = (() => {
    if (processingAttempt > 1) return `Trying again… (${processingAttempt})`
    if (processingPastHalfExpected) return 'Still working…'
    return processingMessage ?? 'Transcribing…'
  })()

  $: rawLevel = state.kind === 'Recording' ? Number(state.level) || 0 : 0

  // Persist session-type flags so they survive the Recording -> Processing transition.
  // Reset only when going back to idle states (Dormant/Hidden/Listening).
  let _lastIsCommand = false
  let _lastIsVoiceEdit = false
  let _lastVoiceEditHasSel = false
  $: if (state.kind === 'Recording') {
    _lastIsCommand = Boolean(state.is_command)
    _lastIsVoiceEdit = Boolean(state.is_voice_edit)
    _lastVoiceEditHasSel = Boolean(state.voice_edit_has_selection)
  } else if (state.kind === 'Dormant' || state.kind === 'Hidden' || state.kind === 'Listening') {
    _lastIsCommand = false
    _lastIsVoiceEdit = false
    _lastVoiceEditHasSel = false
  }
  $: isCommand = state.kind === 'Recording' ? Boolean(state.is_command) : _lastIsCommand
  $: isVoiceEdit = state.kind === 'Recording' ? Boolean(state.is_voice_edit) : _lastIsVoiceEdit
  $: voiceEditHasSelection = state.kind === 'Recording' ? Boolean(state.voice_edit_has_selection) : _lastVoiceEditHasSel
  $: isProcessingVoiceEdit = state.kind === 'Processing' ? Boolean(state.is_voice_edit) || _lastIsVoiceEdit : false

  $: isSensitiveApp =
    state.kind === 'SensitiveAppPeek' ||
    (state.kind === 'Listening' && Boolean(state.sensitive_app)) ||
    (state.kind === 'Recording' && Boolean(state.sensitive_app))

  type OverlayLayoutTier = 'Dormant' | 'Mini' | 'Full'
  let overlayLayoutTier: OverlayLayoutTier = 'Dormant'
  let lastTier: 'Mini' | 'Full' = 'Mini' // Remember last tier for hover restore
  let overlayActivePreference: OverlayActivePreference = 'Mini'
  let overlayAlwaysVisible = false
  let modesList: DictationMode[] = []
  let polishEnabledOverlay = false
  let contextPreviews: {
    appLabel: string | null
    clipboardPreview: string | null
    selectionPreview: string | null
  } | null = null

  let modeMenuOpen = false
  let contextMenuOpen = false
  let contextMenuX = 0
  let contextMenuY = 0
  let _prevMenuOpen = false

  /** Rust: resize HWND for menu clearance + hit-test; DOM: undo overflow clipping on html/body/#app. */
  $: {
    const open = modeMenuOpen || contextMenuOpen
    invoke('set_overlay_menu_open', { open }).catch(() => {})
    // WHY: When a menu closes, Rust shrinks the HWND from canvas back to tight-fit. That resize
    // fires a synthetic mouseleave while the cursor is still over the pill. Arm the retract guard
    // so the bogus leave is suppressed and the pill stays expanded until the pointer truly exits.
    if (_prevMenuOpen && !open) {
      armIdleRetractGuardAfterChromeResize()
    }
    _prevMenuOpen = open
    if (typeof document !== 'undefined') {
      document.documentElement.classList.toggle('kalam-ov-menu', open)
      document.body.classList.toggle('kalam-ov-menu', open)
      document.getElementById('app')?.classList.toggle('kalam-ov-menu', open)
    }
  }

  $: idleModeLabel =
    state.kind === 'Dormant' && typeof state.mode_name === 'string' && state.mode_name.trim() !== ''
      ? state.mode_name
      : activeModeName

  $: copyLastTitle =
    state.kind === 'Dormant' && state.last_transcription_preview
      ? `Copy: ${state.last_transcription_preview.slice(0, 42)}${state.last_transcription_preview.length > 42 ? '…' : ''}`
      : 'Copy last transcription'

  let isHovered = false
  /**
   * After "collapse to pill" while idle, block hover-to-expand until the pointer actually leaves the pill.
   * A short time gate is not enough: `resize_overlay_to` IPC can take >500ms, and while the cursor stays over
   * the pill, `mouseenter`/layout can then run and restore `lastTier` (Full) — so collapse looks broken.
   */
  let blockDormantHoverExpandUntilLeave = false

  /** After expand/collapse resizes the window, ignore `mouseleave` retract until the pointer is confirmed outside the pill (resize often fires a bogus leave). */
  let suppressIdleRetractUntilOutside = false
  let pillRootEl: HTMLElement | null = null
  let boundPointerMoveForRetract: ((e: PointerEvent) => void) | null = null

  function disarmIdleRetractGuard() {
    suppressIdleRetractUntilOutside = false
    if (boundPointerMoveForRetract != null && typeof document !== 'undefined') {
      document.removeEventListener('pointermove', boundPointerMoveForRetract, true)
      boundPointerMoveForRetract = null
    }
  }

  function armIdleRetractGuardAfterChromeResize() {
    suppressIdleRetractUntilOutside = true
    if (typeof document === 'undefined' || boundPointerMoveForRetract != null) return
    boundPointerMoveForRetract = (e: PointerEvent) => {
      if (!suppressIdleRetractUntilOutside) return
      // WHY: While a menu is open the pointer legitimately leaves the pill rect to reach dropdown items.
      if (modeMenuOpen || contextMenuOpen) return
      const el = pillRootEl
      if (!el) return
      const r = el.getBoundingClientRect()
      const { clientX: x, clientY: y } = e
      if (x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) return
      disarmIdleRetractGuard()
      void retractDormantChromeToDotIfIdle()
    }
    document.addEventListener('pointermove', boundPointerMoveForRetract, true)
  }

  async function retractDormantChromeToDotIfIdle() {
    if (state.kind !== 'Dormant' || overlayLayoutTier === 'Dormant') return
    // WHY: Never collapse while a dropdown/context menu is open — the user is actively interacting.
    if (modeMenuOpen || contextMenuOpen) return
    // Drop cached previews so we never show a previous app's context after retract.
    contextPreviews = null
    lastTier = overlayLayoutTier
    await invoke('resize_overlay_to', { size: 'Dormant' }).catch(() => {})
    overlayLayoutTier = 'Dormant'
  }

  async function onBlipMouseEnter(e: MouseEvent) {
    isHovered = true
    if (blockDormantHoverExpandUntilLeave) return
    // If dormant, restore to last tier (Mini or Full)
    if (state.kind === 'Dormant' && overlayLayoutTier === 'Dormant') {
      const tier = lastTier
      // WHY: Arm the retract guard BEFORE the resize IPC. The resize repositions the HWND which
      // fires a synthetic mouseleave while the cursor is still visually over the pill. Without
      // the guard, onBlipMouseLeave sets isHovered=false and retractDormantChromeToDotIfIdle
      // collapses the pill right after it expanded — requiring multiple hover attempts.
      armIdleRetractGuardAfterChromeResize()
      await invoke('resize_overlay_to', { size: tier }).catch(() => {})
      overlayLayoutTier = tier
      // Full panel reads context from a prior invoke; re-fetch so it matches current foreground.
      if (tier === 'Full') await loadContextPreviewsOnly()
    }
  }

  async function onBlipMouseLeave(e: MouseEvent) {
    const cur = e.currentTarget
    const rel = e.relatedTarget
    // Only clear the compact lock when leaving the pill for real (not moving between children inside it).
    if (
      !(
        blockDormantHoverExpandUntilLeave &&
        cur instanceof HTMLElement &&
        rel instanceof Node &&
        cur.contains(rel)
      )
    ) {
      blockDormantHoverExpandUntilLeave = false
    }
    // WHY: Resize/reposition fires synthetic mouseleave while the cursor is still “on” "on" the pill. If we set isHovered=false immediately, CSS starts collapsing (300ms transition). If the guard is active, return early WITHOUT changing isHovered, so CSS stays expanded. This prevents the "mushed" mid-transition appearance when collapse interrupts expand.
    if (suppressIdleRetractUntilOutside) return
    // WHY: Pointer may leave the .pill box while hovering over an absolutely-positioned dropdown
    // (rendered above/below the pill). Don't retract — the user is still interacting.
    if (modeMenuOpen || contextMenuOpen) return
    isHovered = false
    await retractDormantChromeToDotIfIdle()
  }

  /** Close context menu when state leaves dormant (mode menu stays open during active states). */
  $: if (state.kind !== 'Dormant' && state.kind !== 'Recording' && state.kind !== 'Processing') {
    modeMenuOpen = false
    contextMenuOpen = false
  }

  let statusTimeout: ReturnType<typeof setTimeout> | null = null

  let autoActivateToast = ''
  let autoActivateToastTimer: ReturnType<typeof setTimeout> | null = null

  function showAutoActivateToast(p: AutoActivateSwitchedPayload) {
    const app = p.triggered_by_app?.trim() ?? ''
    if (p.is_restore) {
      autoActivateToast = `Restored: ${p.mode_name}`
    } else if (app) {
      autoActivateToast = `${p.mode_name} (${app})`
    } else {
      autoActivateToast = `${p.mode_name}`
    }
    if (autoActivateToastTimer) clearTimeout(autoActivateToastTimer)
    autoActivateToastTimer = setTimeout(() => {
      autoActivateToast = ''
      autoActivateToastTimer = null
    }, 1800)
  }

  async function syncOverlayWindowAfterEvent(kind: string) {
    if (kind === 'Hidden') return
    if (kind === 'Dormant') {
      // Save current tier before going dormant
      if (overlayLayoutTier !== 'Dormant') {
        lastTier = overlayLayoutTier
      }
      await invoke('resize_overlay_to', { size: 'Dormant' }).catch(() => {})
      overlayLayoutTier = 'Dormant'
      return
    }
    if (
      kind === 'SensitiveAppPeek' ||
      kind === 'ShortPress' ||
      kind === 'Status' ||
      kind === 'Success' ||
      kind === 'Error' ||
      kind === 'Cancelling'
    ) {
      // These transient states use current tier
      const size = overlayLayoutTier === 'Dormant' ? lastTier : overlayLayoutTier
      await invoke('resize_overlay_to', { size }).catch(() => {})
      return
    }
    if (kind === 'Listening') {
      const newTier = overlayActivePreference === 'Full' ? 'Full' : 'Mini'
      lastTier = newTier
      await invoke('resize_overlay_to', { size: newTier }).catch(() => {})
      overlayLayoutTier = newTier
      if (newTier === 'Full') await loadContextPreviewsOnly()
      return
    }
    if (kind === 'Recording' || kind === 'Processing') {
      // Use active preference or maintain current tier
      const newTier = overlayLayoutTier === 'Dormant' ? lastTier : overlayLayoutTier
      await invoke('resize_overlay_to', { size: newTier }).catch(() => {})
      if (overlayLayoutTier === 'Dormant') {
        overlayLayoutTier = newTier
      }
      if (newTier === 'Full') await loadContextPreviewsOnly()
    }
  }

  async function loadContextPreviewsOnly() {
    try {
      const p = (await invoke('get_context_previews')) as {
        appLabel?: string | null
        clipboardPreview?: string | null
        selectionPreview?: string | null
      }
      contextPreviews = {
        appLabel: p.appLabel ?? null,
        clipboardPreview: p.clipboardPreview ?? null,
        selectionPreview: p.selectionPreview ?? null,
      }
    } catch {
      contextPreviews = { appLabel: null, clipboardPreview: null, selectionPreview: null }
    }
  }

  /** Context summary line for Full panel — shows source names, not content. */
  $: fullContextSummary = (() => {
    if (!contextPreviews) return ''
    const parts: string[] = []
    if (contextPreviews.appLabel) parts.push(contextPreviews.appLabel)
    if (contextPreviews.clipboardPreview) parts.push('Clipboard')
    if (contextPreviews.selectionPreview) parts.push('Selection')
    return parts.join(' · ')
  })()

  async function expandToFullChrome() {
    blockDormantHoverExpandUntilLeave = false
    lastTier = 'Full'
    contextPreviews = null
    await invoke('resize_overlay_to', { size: 'Full' }).catch(() => {})
    overlayLayoutTier = 'Full'
    await loadContextPreviewsOnly()
    armIdleRetractGuardAfterChromeResize()
  }

  /** True between primary-button pointerdown and the synthetic click — avoids double collapse + helps when WebView eats `click` for drag. */
  let collapseChromeFromPointer = false

  function onCollapseChromePointerDown(e: PointerEvent) {
    if (e.button !== 0) return
    e.stopPropagation()
    collapseChromeFromPointer = true
    void collapseToDormant()
  }

  function onCollapseChromeClick(e: MouseEvent) {
    e.stopPropagation()
    // detail > 0 = real mouse click; keyboard activation uses detail 0 — never drop that path if a pointerdown already ran.
    if (collapseChromeFromPointer && e.detail > 0) {
      collapseChromeFromPointer = false
      return
    }
    collapseChromeFromPointer = false
    void collapseToDormant()
  }

  /**
   * Shrink chrome: Recording/Processing full → Mini.
   * Idle maximized (Full) → Mini strip (compact pill with mode/copy/expand) — not tier Dormant (dot-only), which reads as “gone”.
   * Dot-only idle is still from pointer leaving the strip (mouseleave), not this control.
   */
  async function collapseToDormant() {
    contextPreviews = null
    modeMenuOpen = false
    contextMenuOpen = false
    if (state.kind === 'Recording' || state.kind === 'Processing') {
      lastTier = 'Mini'
      await invoke('resize_overlay_to', { size: 'Mini' }).catch(() => {})
      overlayLayoutTier = 'Mini'
      armIdleRetractGuardAfterChromeResize()
    } else if (overlayLayoutTier === 'Full') {
      lastTier = 'Mini'
      await invoke('resize_overlay_to', { size: 'Mini' }).catch(() => {})
      overlayLayoutTier = 'Mini'
      state = makeLocalDormant()
      blockDormantHoverExpandUntilLeave = true
      armIdleRetractGuardAfterChromeResize()
    }
  }

  let copyFeedback = ''
  let copyFeedbackTimer: ReturnType<typeof setTimeout> | null = null

  async function copyLastTranscription() {
    try {
      const copied = (await invoke('copy_last_transcription')) as boolean
      copyFeedback = copied ? 'Copied' : 'No transcription yet'
    } catch (e) {
      console.error('copy_last_transcription failed:', e)
      copyFeedback = 'Couldn\u2019t copy'
    }
    if (copyFeedbackTimer) clearTimeout(copyFeedbackTimer)
    copyFeedbackTimer = setTimeout(() => { copyFeedback = ''; copyFeedbackTimer = null }, 1200)
    contextMenuOpen = false
    modeMenuOpen = false
  }

  async function togglePolishFromOverlay() {
    try {
      const on = (await invoke('toggle_polish_from_overlay')) as boolean
      polishEnabledOverlay = on
    } catch (e) {
      console.error('toggle_polish_from_overlay failed:', e)
    }
  }

  async function setActiveModeFromOverlay(modeId: string) {
    try {
      await invoke('set_active_mode', { modeId })
    } catch (e) {
      console.error('set_active_mode failed:', e)
    }
    modeMenuOpen = false
    contextMenuOpen = false
  }

  function onRootContextMenu(e: MouseEvent) {
    if (state.kind !== 'Dormant') return
    e.preventDefault()
    contextMenuX = e.clientX
    contextMenuY = e.clientY
    contextMenuOpen = true
    modeMenuOpen = false
  }

  function closeContextMenus() {
    contextMenuOpen = false
    modeMenuOpen = false
  }

  /**
   * Toggle the mode dropdown. When opening, pre-expand the HWND to canvas size so the dropdown
   * isn't clipped by the WebView boundary, then set the flag to render the dropdown.
   */
  async function toggleModeMenu() {
    if (modeMenuOpen) {
      modeMenuOpen = false
      return
    }
    // WHY: Expand HWND to canvas BEFORE rendering the dropdown so it has room immediately.
    // The reactive $: block also calls set_overlay_menu_open, but the dropdown would render
    // in the same tick before the IPC completes — causing a clipped frame.
    await invoke('set_overlay_menu_open', { open: true }).catch(() => {})
    modeMenuOpen = true
  }

  /** Map backend error strings to plain-language messages. */
  function friendlyError(raw: string): string {
    const lower = raw.toLowerCase()
    if (lower.includes('rate limit')) return 'Too many requests — try again in a moment'
    if (lower.includes('timeout') || lower.includes('timed out')) return 'Request timed out — check your connection'
    if (lower.includes('api key') || lower.includes('unauthorized') || lower.includes('401')) return 'API key issue — check Settings'
    if (lower.includes('network') || lower.includes('connection') || lower.includes('fetch')) return 'Connection issue — check your network'
    if (lower.includes('model') && lower.includes('not found')) return 'Model not available — check Settings'
    if (raw.length > 60) return raw.slice(0, 57) + '…'
    return raw
  }

  // ── Waveform engine (unchanged logic, kept intact) ──
  const WAVE_POINTS = 100
  let levelHistory: number[] = []
  let currentLevel = 0
  let snakeOffset = 0
  let animationFrameId: number | null = null
  let waveCanvas: HTMLCanvasElement | null = null
  let cssVizEl: HTMLDivElement | null = null

  function paddedLevels(): number[] {
    const pad = Math.max(0, WAVE_POINTS - levelHistory.length)
    const out: number[] = []
    for (let i = 0; i < pad; i++) out.push(0)
    out.push(...levelHistory)
    return out.slice(-WAVE_POINTS)
  }

  /** Accent color per session type — desaturated palette, not neon. */
  function accentColor(): string {
    if (isCommand) return 'var(--ov-command)'
    if (isSensitiveApp) return 'var(--ov-sensitive)'
    if (isVoiceEdit) return 'var(--ov-voice-edit)'
    return 'var(--ov-accent)'
  }

  function drawWaveCanvas() {
    const canvas = waveCanvas
    if (!canvas) return
    const rect = canvas.getBoundingClientRect()
    const w = Math.max(1, rect.width)
    const h = Math.max(1, rect.height)
    const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1
    const bw = Math.floor(w * dpr)
    const bh = Math.floor(h * dpr)
    if (canvas.width !== bw || canvas.height !== bh) {
      canvas.width = bw
      canvas.height = bh
    }
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    ctx.clearRect(0, 0, w, h)

    const padded = paddedLevels()
    const cmd = isCommand
    const ve = isVoiceEdit && !cmd
    const stroke = cmd ? '#d4637a' : isSensitiveApp ? '#c8912e' : ve ? '#9578d9' : '#5a9ec4'
    const strokeSoft = cmd
      ? 'rgba(212, 99, 122,'
      : isSensitiveApp
        ? 'rgba(200, 145, 46,'
        : ve
          ? 'rgba(149, 120, 217,'
          : 'rgba(90, 158, 196,'

    if (waveformStyle === 'Oscilloscope') {
      ctx.beginPath()
      for (let i = 0; i < WAVE_POINTS; i++) {
        const l = padded[i] ?? 0
        const x = (i / (WAVE_POINTS - 1)) * w
        const y = h * 0.5 - l * (h * 0.5 - 2) * 0.92
        if (i === 0) ctx.moveTo(x, y)
        else ctx.lineTo(x, y)
      }
      ctx.shadowBlur = 8
      ctx.shadowColor = stroke
      ctx.strokeStyle = stroke
      ctx.lineWidth = 1.75
      ctx.lineJoin = 'round'
      ctx.lineCap = 'round'
      ctx.stroke()
      ctx.shadowBlur = 0
      return
    }

    if (waveformStyle === 'Aurora') {
      ctx.globalCompositeOperation = 'screen'
      ctx.filter = 'blur(8px)'

      const colors = cmd
        ? [
            'rgba(212, 99, 122, 0.85)',
            'rgba(190, 70, 95, 0.75)',
            'rgba(200, 145, 46, 0.7)',
            'rgba(240, 235, 230, 0.35)',
          ]
        : isSensitiveApp
          ? [
              'rgba(200, 145, 46, 0.85)',
              'rgba(210, 170, 70, 0.75)',
              'rgba(190, 150, 40, 0.7)',
              'rgba(240, 235, 230, 0.35)',
            ]
          : ve
            ? [
                'rgba(149, 120, 217, 0.85)',
                'rgba(125, 90, 210, 0.8)',
                'rgba(175, 160, 220, 0.7)',
                'rgba(240, 235, 230, 0.35)',
              ]
            : [
                'rgba(90, 158, 196, 0.85)',
                'rgba(50, 150, 120, 0.75)',
                'rgba(125, 90, 210, 0.7)',
                'rgba(240, 235, 230, 0.35)',
              ]

      for (let layer = 0; layer < 4; layer++) {
        const phase = snakeOffset * (0.5 + layer * 0.2) + layer * 2.0
        const ampMul = 0.5 + layer * 0.15
        ctx.beginPath()
        for (let i = 0; i < WAVE_POINTS; i++) {
          const l = padded[i] ?? 0
          const x = (i / (WAVE_POINTS - 1)) * w
          const wave1 = Math.sin(i * 0.05 + phase) * 0.5
          const wave2 = Math.sin(i * 0.1 - phase * 0.8) * 0.3
          const wave = wave1 + wave2
          const yOffset = (wave * h * 0.2) + (Math.sin(i * 0.08 + phase * 1.5) * l * h * ampMul)
          const y = h * 0.5 + yOffset
          if (i === 0) ctx.moveTo(x, y)
          else ctx.lineTo(x, y)
        }
        ctx.strokeStyle = colors[layer]
        ctx.lineWidth = layer === 3 ? h * 0.2 : h * 0.6
        ctx.lineCap = 'round'
        ctx.lineJoin = 'round'
        ctx.stroke()
      }
      ctx.filter = 'none'
      ctx.globalCompositeOperation = 'source-over'
      return
    }

    // SiriWave
    for (let layer = 0; layer < 3; layer++) {
      const phase = snakeOffset * (1.1 + layer * 0.35) + layer * 1.7
      const ampMul = 0.28 + layer * 0.12
      const alpha = 0.35 + layer * 0.22
      ctx.beginPath()
      for (let i = 0; i < WAVE_POINTS; i++) {
        const l = padded[i] ?? 0
        const x = (i / (WAVE_POINTS - 1)) * w
        const wave = Math.sin(i * 0.18 + phase) * (h * 0.08)
        const y = h * 0.5 + Math.sin(i * 0.12 + phase * 0.8) * l * h * ampMul + wave * l
        if (i === 0) ctx.moveTo(x, y)
        else ctx.lineTo(x, y)
      }
      ctx.strokeStyle = `${strokeSoft} ${alpha})`
      ctx.lineWidth = 1.4 + layer * 0.35
      ctx.lineJoin = 'round'
      ctx.lineCap = 'round'
      ctx.stroke()
    }
  }

  function updateCssViz() {
    const el = cssVizEl
    if (!el) return
    const padded = paddedLevels()

    if (waveformStyle === 'EchoRing') {
      const v0 = padded[WAVE_POINTS - 1] ?? 0
      const v1 = padded[WAVE_POINTS - 8] ?? 0
      const v2 = padded[WAVE_POINTS - 16] ?? 0
      el.style.setProperty('--echo-core', String(0.8 + v0 * 0.6))
      el.style.setProperty('--echo-scale1', String(0.5 + v0 * 0.8))
      el.style.setProperty('--echo-op1', String(v0 * 0.8))
      el.style.setProperty('--echo-scale2', String(0.6 + v1 * 0.8))
      el.style.setProperty('--echo-op2', String(v1 * 0.6))
      el.style.setProperty('--echo-scale3', String(0.7 + v2 * 0.8))
      el.style.setProperty('--echo-op3', String(v2 * 0.4))
      return
    }

    if (waveformStyle === 'RoundedBars') {
      for (let i = 0; i <= 5; i++) {
        const historyIdx = WAVE_POINTS - 1 - (5 - i) * 4
        let sum = 0
        for (let k = 0; k < 3; k++) sum += padded[historyIdx - k] ?? 0
        const v = sum / 3
        const scale = 0.15 + v * 0.85
        if (i === 5) {
          el.style.setProperty('--b5', String(scale))
        } else {
          el.style.setProperty(`--b${i}`, String(scale))
          el.style.setProperty(`--b${10 - i}`, String(scale))
        }
      }
      return
    }

    if (waveformStyle === 'BreathingAura') {
      const v = currentLevel
      el.style.setProperty('--aura-scale', String(0.5 + v * 0.9))
      el.style.setProperty('--aura-glow', String(4 + v * 26))
      return
    }

    if (waveformStyle === 'NeonPulse') {
      const v = currentLevel
      const jitter = Math.sin(snakeOffset * 2) * 0.05 * v
      const nw = Math.max(0, v + jitter)
      el.style.setProperty('--neon-w', String(nw))
      el.style.setProperty('--neon-h', String(v))
      el.style.setProperty('--neon-g', String(v))
      return
    }
  }

  function animateWave() {
    if (state.kind !== 'Recording') {
      levelHistory = []
      snakeOffset = 0
      currentLevel = 0
      animationFrameId = null
      return
    }

    const r = Math.min(1, Math.max(0, rawLevel))
    const gain = Math.pow(r, 0.8) * 1.8
    const targetLevel = Math.min(1, gain)

    if (targetLevel > currentLevel) {
      currentLevel += (targetLevel - currentLevel) * 0.45
    } else {
      currentLevel += (targetLevel - currentLevel) * 0.15
    }

    if (levelHistory.length >= WAVE_POINTS) levelHistory.shift()
    levelHistory.push(currentLevel)
    snakeOffset += 0.15

    if (waveformStyle === 'SiriWave' || waveformStyle === 'Oscilloscope' || waveformStyle === 'Aurora') {
      drawWaveCanvas()
    } else {
      updateCssViz()
    }

    animationFrameId = requestAnimationFrame(animateWave)
  }

  $: if (state.kind === 'Recording' && !animationFrameId) {
    animationFrameId = requestAnimationFrame(animateWave)
  } else if (state.kind !== 'Recording' && animationFrameId) {
    cancelAnimationFrame(animationFrameId)
    animationFrameId = null
    levelHistory = []
    snakeOffset = 0
    currentLevel = 0
  }

  // WebView2 keep-alive worker to prevent JS throttling in unfocused windows
  let keepAliveWorker: Worker | null = null
  try {
    const blob = new Blob(
      [`setInterval(()=>postMessage(""),100)`],
      { type: 'application/javascript' }
    )
    keepAliveWorker = new Worker(URL.createObjectURL(blob))
    keepAliveWorker.onmessage = () => {}
  } catch { /* best-effort */ }

  onMount(() => {
    let unlisten: (() => void) | null = null
    let unlistenSettings: (() => void) | null = null
    let unlistenAutoActivate: (() => void) | null = null
    let unlistenCursorLeft: (() => void) | null = null
    let pendingSuccessTimer: ReturnType<typeof setTimeout> | null = null
    let retryHoldUntil: number | null = null
    let lastSeenProcessingAttempt = 1

    Promise.all([
      invoke('get_settings'),
      invoke('get_platform').catch(() => 'windows'),
    ])
      .then(([config, os]) => {
        overlayPlatform = typeof os === 'string' ? os : 'windows'
        const cfg = config as AppConfig
        if (cfg.waveform_style) waveformStyle = cfg.waveform_style
        if (cfg.overlay_expand_direction) expandDirection = cfg.overlay_expand_direction
        if (cfg.hotkey) hotkeyStr = cfg.hotkey
        else if (cfg.toggle_dictation_hotkey) hotkeyStr = cfg.toggle_dictation_hotkey
        syncActiveModeFromConfig(cfg)
        modesList = cfg.modes ?? []
        overlayActivePreference = cfg.overlay_active_preference ?? 'Mini'
        overlayAlwaysVisible = cfg.overlay_always_visible ?? false
        polishEnabledOverlay = cfg.polish_enabled ?? false
      })
      .catch(console.error)

    listen<AppConfig>('settings_updated', (e) => {
      if (e.payload?.waveform_style) waveformStyle = e.payload.waveform_style
      if (e.payload?.overlay_expand_direction) expandDirection = e.payload.overlay_expand_direction
      if (e.payload?.hotkey) hotkeyStr = e.payload.hotkey
      else if (e.payload?.toggle_dictation_hotkey) hotkeyStr = e.payload.toggle_dictation_hotkey
      if (e.payload?.modes) modesList = e.payload.modes
      if (e.payload?.overlay_active_preference != null) overlayActivePreference = e.payload.overlay_active_preference
      if (e.payload?.overlay_always_visible != null) overlayAlwaysVisible = e.payload.overlay_always_visible
      if (e.payload?.polish_enabled != null) polishEnabledOverlay = e.payload.polish_enabled
      if (e.payload?.modes && e.payload?.active_mode_id != null) syncActiveModeFromConfig(e.payload)
    }).then((fn) => { unlistenSettings = fn })

    listen<AutoActivateSwitchedPayload>('auto-activate-switched', (e) => {
      if (e.payload) showAutoActivateToast(e.payload)
    }).then((fn) => { unlistenAutoActivate = fn })

    // WHY: Rust hit-test loop emits cursor-left-pill when the pointer exits the pill rect and the
    // WebView becomes click-through. DOM mouseleave never fires in that case, so without this
    // listener the pill stays visually expanded even though it can no longer receive mouse events.
    getCurrentWebviewWindow().listen('cursor-left-pill', () => {
      if (modeMenuOpen || contextMenuOpen) return
      // WHY: During resize transitions the retract guard is armed to absorb bogus mouseleave.
      // Rust's position_suppressed window usually prevents this event from firing during
      // transitions, but respect the JS-side guard as a belt-and-suspenders safeguard.
      if (suppressIdleRetractUntilOutside) return
      isHovered = false
      void retractDormantChromeToDotIfIdle()
    }).then((fn) => { unlistenCursorLeft = fn })

    const onDocPointerDown = (ev: MouseEvent) => {
      const t = ev.target as Node
      if (t instanceof Element && t.closest?.('[data-overlay-menu-root]')) return
      closeContextMenus()
    }
    const onKeyDown = (ev: KeyboardEvent) => {
      if (ev.key === 'Escape') {
        closeContextMenus()
      }
    }
    document.addEventListener('pointerdown', onDocPointerDown, true)
    document.addEventListener('keydown', onKeyDown, true)

    getCurrentWebviewWindow().listen<OverlayEvent>('overlay-state', (e) => {
      const jsTs = Date.now() * 1000
      invoke('trace_latency', { event: 'T7', jsTimestamp: jsTs }).catch(() => {})
      const p = e?.payload
      if (!isValidPayload(p)) return

      if (p.kind === 'Listening') {
        invoke('trace_latency', { event: 'T7_listening', jsTimestamp: jsTs }).catch(() => {})
      }

      const traceAfterState = () => {
        tick().then(() => {
          invoke('trace_latency', { event: 'T8', jsTimestamp: Date.now() * 1000 }).catch(() => {})
          requestAnimationFrame(() => {
            invoke('trace_latency', { event: 'T9', jsTimestamp: Date.now() * 1000 }).catch(() => {})
          })
        })
      }

      if (p.kind === 'Processing') {
        if (pendingSuccessTimer) { clearTimeout(pendingSuccessTimer); pendingSuccessTimer = null }
        const att = p.attempt ?? 1
        if (att > 1 && att > lastSeenProcessingAttempt) retryHoldUntil = Date.now() + 1500
        lastSeenProcessingAttempt = att
        state = p
        traceAfterState()
        if (statusTimeout) clearTimeout(statusTimeout)
        void syncOverlayWindowAfterEvent('Processing')
        return
      }

      if (p.kind === 'Success') {
        const now = Date.now()
        const holdEnd = retryHoldUntil
        const wasRetrying = state.kind === 'Processing' && (state.attempt ?? 1) > 1
        if (holdEnd !== null && now < holdEnd && wasRetrying) {
          if (pendingSuccessTimer) clearTimeout(pendingSuccessTimer)
          pendingSuccessTimer = setTimeout(() => {
            pendingSuccessTimer = null
            retryHoldUntil = null
            lastSeenProcessingAttempt = 1
            state = { kind: 'Success' }
            traceAfterState()
            void syncOverlayWindowAfterEvent('Success')
            if (statusTimeout) clearTimeout(statusTimeout)
            statusTimeout = setTimeout(() => { void goToDormantIdle() }, 2000)
          }, holdEnd - now)
          return
        }
        retryHoldUntil = null
        lastSeenProcessingAttempt = 1
        if (pendingSuccessTimer) { clearTimeout(pendingSuccessTimer); pendingSuccessTimer = null }
        state = p
        traceAfterState()
        void syncOverlayWindowAfterEvent('Success')
        if (statusTimeout) clearTimeout(statusTimeout)
        statusTimeout = setTimeout(() => { void goToDormantIdle() }, 2000)
        return
      }

      if (pendingSuccessTimer) { clearTimeout(pendingSuccessTimer); pendingSuccessTimer = null }
      retryHoldUntil = null
      lastSeenProcessingAttempt = 1
      state = p
      traceAfterState()
      void syncOverlayWindowAfterEvent(p.kind)
      if (statusTimeout) clearTimeout(statusTimeout)
      if (p.kind === 'Status' || p.kind === 'Cancelling') {
        statusTimeout = setTimeout(() => { void goToDormantIdle() }, 2500)
      } else if (p.kind === 'SensitiveAppPeek') {
        statusTimeout = setTimeout(() => { void goToDormantIdle() }, 2600)
      }
      // Errors: NO auto-dismiss. User must click Retry or Dismiss.
    }).then((fn) => { unlisten = fn })

    invoke<OverlayEvent>('get_overlay_initial_state')
      .then((initial) => {
        if (isValidPayload(initial)) {
          state = initial
          void syncOverlayWindowAfterEvent(initial.kind)
        }
      })
      .catch(console.error)

    return () => {
      unlisten?.()
      unlistenSettings?.()
      unlistenAutoActivate?.()
      unlistenCursorLeft?.()
      document.removeEventListener('pointerdown', onDocPointerDown, true)
      document.removeEventListener('keydown', onKeyDown, true)
      if (statusTimeout) clearTimeout(statusTimeout)
      if (pendingSuccessTimer) clearTimeout(pendingSuccessTimer)
      if (autoActivateToastTimer) clearTimeout(autoActivateToastTimer)
      if (copyFeedbackTimer) clearTimeout(copyFeedbackTimer)
      clearInterval(clockInterval)
      disarmIdleRetractGuard()
      keepAliveWorker?.terminate()
      invoke('set_overlay_menu_open', { open: false }).catch(() => {})
      document.documentElement.classList.remove('kalam-ov-menu')
      document.body.classList.remove('kalam-ov-menu')
      document.getElementById('app')?.classList.remove('kalam-ov-menu')
    }
  })
</script>

{#if state.kind !== 'Hidden'}
<div
  class="ov-root"
  class:expand-up={expandDirection === 'Up'}
  class:expand-down={expandDirection === 'Down'}
  class:expand-center={expandDirection === 'Center'}
  class:has-menu-open={modeMenuOpen || contextMenuOpen}
  role="status"
  aria-label="Kalam dictation status"
  on:mouseenter={onBlipMouseEnter}
>
  {#if autoActivateToast}
    <div class="toast" role="status" aria-live="polite">{autoActivateToast}</div>
  {/if}
  {#if copyFeedback}
    <div class="toast toast-bottom" role="status" aria-live="polite">{copyFeedback}</div>
  {/if}
  <div
    bind:this={pillRootEl}
    class="pill"
    class:hello={firstAppearance}
    class:dormant-pill={state.kind === 'Dormant'}
    class:menu-open={modeMenuOpen || contextMenuOpen}
    class:tier-dormant={overlayLayoutTier === 'Dormant'}
    class:tier-mini={overlayLayoutTier === 'Mini'}
    class:tier-full={overlayLayoutTier === 'Full'}
    class:recording={state.kind === 'Recording'}
    class:processing={state.kind === 'Processing'}
    class:processing-long={state.kind === 'Processing' && processingPastHalfExpected}
    class:success={state.kind === 'Success'}
    class:error={state.kind === 'Error'}
    class:sensitive-app={isSensitiveApp}
    class:always-visible={overlayAlwaysVisible}
    class:is-command={isCommand}
    class:is-voice-edit={isVoiceEdit && !isCommand}
    on:mouseleave={onBlipMouseLeave}
    on:contextmenu={onRootContextMenu}
  >
    <!-- ═══════════════ DORMANT ═══════════════ -->
    {#if state.kind === 'Dormant'}
      {#if overlayLayoutTier === 'Full'}
        <div class="full-panel dormant-full">
          <div class="full-header">
            <div class="full-mode-identity">
              <span class="full-mode-swatch" style={dormantDotBackground ? `background-color: ${dormantDotBackground}` : undefined} aria-hidden="true" />
              <span class="full-mode-title" data-tauri-drag-region>{idleModeLabel}</span>
            </div>
            <button
              type="button"
              class="polish-chip"
              aria-pressed={polishEnabledOverlay}
              data-tauri-drag-region="false"
              data-overlay-menu-root
              on:click|stopPropagation={() => togglePolishFromOverlay()}
            >
              Clean up {polishEnabledOverlay ? 'on' : 'off'}
            </button>
          </div>
          <p class="full-sub" data-tauri-drag-region>Press <kbd class="hotkey-kbd">{hotkeyDisplayStr}</kbd> to dictate</p>
          {#if fullContextSummary}
            <div class="ctx-summary">{fullContextSummary}</div>
          {/if}
          <div class="full-footer" data-tauri-drag-region="false" data-overlay-menu-root>
            <div class="mode-wrap">
              <button type="button" class="action-chip" aria-expanded={modeMenuOpen} on:click|stopPropagation={() => toggleModeMenu()}
                >{idleModeLabel} <span class="chevron" aria-hidden="true">{modeMenuOpen ? '▴' : '▾'}</span></button
              >
              {#if modeMenuOpen}
                <div class="mode-dropdown" role="menu">
                  {#each modesList as m (m.id)}
                    <button type="button" role="menuitem" on:click|stopPropagation={() => setActiveModeFromOverlay(m.id)}>{m.name}</button>
                  {/each}
                </div>
              {/if}
            </div>
            <button type="button" class="icon-btn" title={copyLastTitle} on:click|stopPropagation={() => copyLastTranscription()}>
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true"><rect x="9" y="9" width="13" height="13" rx="2" stroke="currentColor" stroke-width="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" stroke="currentColor" stroke-width="2"/></svg>
            </button>
            <!-- Shrink maximized idle panel → compact strip (Mini), not dot-only (Dormant) -->
            <button
              type="button"
              class="icon-btn"
              title="Shrink to compact pill"
              data-tauri-drag-region="false"
              on:pointerdown|stopPropagation={onCollapseChromePointerDown}
              on:click|stopPropagation={onCollapseChromeClick}
            >
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M4 14h6v6M20 10h-6V4M14 10l7-7M10 14l-7 7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </button>
          </div>
        </div>
      {:else}
        <!-- Dormant compact: collapsed = single mode-color swatch (draggable); expanded row uses the same swatch next to the mode name — no second status dot. -->
        <div class="dormant-bar" class:expanded={overlayLayoutTier !== 'Dormant'}>
          {#if overlayLayoutTier === 'Dormant'}
            <span
              class="dormant-mode-swatch dormant-idle-mark"
              aria-hidden="true"
              data-tauri-drag-region
              style={dormantDotBackground ? `background-color: ${dormantDotBackground}` : undefined}
            />
          {/if}
          <!-- Reveal row: left = mode (opens menu) + hotkey; right = copy + expand. Wrapper animates width; menu needs overflow visible when open (see .pill.menu-open). -->
          <div class="dormant-expand-inner" class:menu-open={modeMenuOpen}>
            <div class="dormant-left">
              <div class="mode-wrap dormant-mode-wrap" data-overlay-menu-root>
                <button
                  type="button"
                  class="dormant-mode-trigger"
                  aria-expanded={modeMenuOpen}
                  aria-haspopup="menu"
                  title="Switch mode"
                  data-tauri-drag-region="false"
                  on:click|stopPropagation={() => toggleModeMenu()}
                >
                  <span
                    class="dormant-mode-swatch"
                    aria-hidden="true"
                    style={dormantDotBackground ? `background-color: ${dormantDotBackground}` : undefined}
                  />
                  <span class="dormant-mode">{idleModeLabel}</span>
                  <span class="dormant-mode-chevron" aria-hidden="true">{modeMenuOpen ? '▴' : '▾'}</span>
                </button>
                {#if modeMenuOpen}
                  <div class="mode-dropdown mode-dropdown-dormant" role="menu">
                    {#each modesList as m (m.id)}
                      <button type="button" role="menuitem" on:click|stopPropagation={() => setActiveModeFromOverlay(m.id)}>{m.name}</button>
                    {/each}
                  </div>
                {/if}
              </div>
              {#if polishEnabledOverlay}
                <span class="polish-badge" title="Text clean-up is on">P</span>
              {/if}
              <span class="dormant-hotkey">{hotkeyDisplayStr}</span>
            </div>
            {#if currentTime}
              <span class="dormant-clock" aria-hidden="true">{currentTime}</span>
            {/if}
            <div class="dormant-icons" data-tauri-drag-region="false" data-overlay-menu-root>
              <button type="button" class="icon-btn" title={copyLastTitle} on:click|stopPropagation={() => copyLastTranscription()}>
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true"><rect x="9" y="9" width="13" height="13" rx="2" stroke="currentColor" stroke-width="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" stroke="currentColor" stroke-width="2"/></svg>
              </button>
              <button type="button" class="icon-btn" title="Expand panel" on:click|stopPropagation={() => expandToFullChrome()}>
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
              </button>
            </div>
          </div>
        </div>
      {/if}
      {#if contextMenuOpen}
        <div
          class="ctx-menu"
          data-tauri-drag-region="false"
          data-overlay-menu-root
          style="left: {contextMenuX}px; top: {contextMenuY}px;"
          role="menu"
        >
          <button type="button" role="menuitem" on:click|stopPropagation={() => { contextMenuOpen = false; modeMenuOpen = true }}>Switch mode…</button>
          <button type="button" role="menuitem" on:click|stopPropagation={() => copyLastTranscription()}>Copy last</button>
          <button type="button" role="menuitem" on:click|stopPropagation={() => { contextMenuOpen = false; invoke('ui_toggle_dictation') }}>Start recording</button>
          <button type="button" role="menuitem" on:click|stopPropagation={() => expandToFullChrome()}>Expand panel</button>
        </div>
      {/if}

    <!-- ═══════════════ LISTENING ═══════════════ -->
    {:else if state.kind === 'Listening'}
      <div class="content listening" class:sensitive={isSensitiveApp} data-tauri-drag-region>
        {#if isSensitiveApp}
          <svg class="lock-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path d="M7 11V8a5 5 0 0 1 10 0v3M6 11h12a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <span class="label sensitive-label">Local only</span>
        {:else}
          <span class="status-dot listening-dot" aria-hidden="true" />
          <span class="label">Listening…</span>
        {/if}
      </div>
    {:else if state.kind === 'SensitiveAppPeek'}
      <div class="content listening sensitive" data-tauri-drag-region>
        <svg class="lock-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M7 11V8a5 5 0 0 1 10 0v3M6 11h12a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span class="label sensitive-label">Local only</span>
      </div>

    <!-- ═══════════════ SHORT PRESS ═══════════════ -->
    {:else if state.kind === 'ShortPress'}
      <div class="content hint" data-tauri-drag-region>
        <span class="label">Hold longer to dictate</span>
      </div>

    <!-- ═══════════════ STATUS ═══════════════ -->
    {:else if state.kind === 'Status'}
      <div class="content status-message" data-tauri-drag-region>
        <span class="label">
          {state.message}
          {#if state.highlight}
            <span class="highlight-text">{state.highlight}</span>
          {/if}
        </span>
      </div>

    <!-- ═══════════════ RECORDING ═══════════════ -->
    {:else if state.kind === 'Recording'}
      {#if overlayLayoutTier === 'Full'}
        <div class="full-panel rec-full">
          <div class="full-header">
            <div class="full-mode-identity">
              <span class="full-mode-swatch" style="background-color: {activeModeAccent}" aria-hidden="true" />
              <span class="full-mode-title" data-tauri-drag-region>{isVoiceEdit ? (voiceEditHasSelection ? 'Editing selection' : 'Editing') : activeModeName}</span>
              {#if polishEnabledOverlay}
                <span class="header-polish-tag">Clean up</span>
              {/if}
            </div>
            <button type="button" class="pill-stop-btn" title="Stop recording" data-tauri-drag-region="false" on:click|stopPropagation={() => invoke('ui_toggle_dictation')}>
              <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor" aria-hidden="true"><rect width="10" height="10" rx="2"/></svg>
              Stop
            </button>
          </div>
          {#if isSensitiveApp}
            <div class="local-banner">
              <svg class="lock-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                <path d="M7 11V8a5 5 0 0 1 10 0v3M6 11h12a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <span>Local only — no cloud, no context</span>
            </div>
          {:else if fullContextSummary}
            <div class="ctx-summary">{fullContextSummary}</div>
          {/if}
          <div class="content waveform full-wave">
            {#if waveformStyle === 'SiriWave' || waveformStyle === 'Oscilloscope' || waveformStyle === 'Aurora'}
              <canvas bind:this={waveCanvas} class="wave-canvas" aria-hidden="true"></canvas>
            {:else}
              <div bind:this={cssVizEl} class="viz-css" class:viz-cmd={isCommand} class:viz-voice-edit={isVoiceEdit && !isCommand} class:viz-sensitive={!isCommand && !isVoiceEdit && isSensitiveApp} data-viz={waveformStyle}>
                {#if waveformStyle === 'EchoRing'}
                  <div class="viz-echo"><div class="ring r3"></div><div class="ring r2"></div><div class="ring r1"></div><div class="core"></div></div>
                {:else if waveformStyle === 'RoundedBars'}
                  <div class="viz-bars"><span></span><span></span><span></span><span></span><span></span><span></span><span></span><span></span><span></span><span></span><span></span></div>
                {:else if waveformStyle === 'BreathingAura'}
                  <div class="viz-aura"><div class="aura-orb"></div></div>
                {:else if waveformStyle === 'NeonPulse'}
                  <div class="viz-neon"><div class="beam"></div></div>
                {/if}
              </div>
            {/if}
          </div>
          <div class="full-footer" data-tauri-drag-region="false" data-overlay-menu-root>
            <div class="mode-wrap">
              <button type="button" class="action-chip" aria-expanded={modeMenuOpen} on:click|stopPropagation={() => toggleModeMenu()}>{idleModeLabel} <span class="chevron" aria-hidden="true">{modeMenuOpen ? '▴' : '▾'}</span></button>
              {#if modeMenuOpen}
                <div class="mode-dropdown" role="menu">
                  {#each modesList as m (m.id)}
                    <button type="button" role="menuitem" on:click|stopPropagation={() => setActiveModeFromOverlay(m.id)}>{m.name}</button>
                  {/each}
                </div>
              {/if}
            </div>
            <button type="button" class="icon-btn icon-cancel" title="Cancel transcription" on:click|stopPropagation={cancelTranscription}>
              <svg width="12" height="12" viewBox="0 0 18 18" fill="none" aria-hidden="true"><path d="M5 5l8 8M13 5l-8 8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
            </button>
            <button
              type="button"
              class="icon-btn"
              title="Shrink to compact pill"
              data-tauri-drag-region="false"
              on:pointerdown|stopPropagation={onCollapseChromePointerDown}
              on:click|stopPropagation={onCollapseChromeClick}
            >
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M4 14h6v6M20 10h-6V4M14 10l7-7M10 14l-7 7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </button>
          </div>
        </div>
      {:else}
        <!-- Mini recording: waveform fills the pill as a background layer, controls float on top -->
        <div class="mini-rec" data-tauri-drag-region="false">
          <!-- Waveform layer — fills the entire pill, fades at edges -->
          <div class="mini-rec-wave-layer">
            {#if waveformStyle === 'SiriWave' || waveformStyle === 'Oscilloscope' || waveformStyle === 'Aurora'}
              <canvas bind:this={waveCanvas} class="wave-canvas" aria-hidden="true"></canvas>
            {:else}
              <div bind:this={cssVizEl} class="viz-css" class:viz-cmd={isCommand} class:viz-voice-edit={isVoiceEdit && !isCommand} class:viz-sensitive={!isCommand && !isVoiceEdit && isSensitiveApp} data-viz={waveformStyle}>
                {#if waveformStyle === 'EchoRing'}
                  <div class="viz-echo"><div class="ring r3"></div><div class="ring r2"></div><div class="ring r1"></div><div class="core"></div></div>
                {:else if waveformStyle === 'RoundedBars'}
                  <div class="viz-bars"><span></span><span></span><span></span><span></span><span></span><span></span><span></span><span></span><span></span><span></span><span></span></div>
                {:else if waveformStyle === 'BreathingAura'}
                  <div class="viz-aura"><div class="aura-orb"></div></div>
                {:else if waveformStyle === 'NeonPulse'}
                  <div class="viz-neon"><div class="beam"></div></div>
                {/if}
              </div>
            {/if}
          </div>
          <!-- Controls layer — floats above the waveform -->
          <div class="mini-rec-controls">
            <div class="mini-rec-left">
              <span class="mini-rec-dot" style="--rec-accent: {accentColor()}" aria-hidden="true" />
              <span class="mini-rec-mode">{isVoiceEdit ? (voiceEditHasSelection ? 'Edit ✂' : 'Edit') : activeModeName}</span>
            </div>
            <div class="mini-rec-right">
              <button type="button" class="mini-rec-btn mini-rec-cancel" title="Cancel transcription" on:click|stopPropagation={cancelTranscription}>
                <svg width="10" height="10" viewBox="0 0 18 18" fill="none" aria-hidden="true"><path d="M5 5l8 8M13 5l-8 8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
              </button>
              <button type="button" class="mini-rec-btn mini-rec-stop" title="Stop recording" on:click|stopPropagation={() => invoke('ui_toggle_dictation')}>
                <svg width="9" height="9" viewBox="0 0 10 10" fill="currentColor" aria-hidden="true"><rect width="10" height="10" rx="2"/></svg>
              </button>
            </div>
          </div>
        </div>
      {/if}

    <!-- ═══════════════ PROCESSING ═══════════════ -->
    {:else if state.kind === 'Processing'}
      {#if overlayLayoutTier === 'Full'}
        <div class="full-panel proc-full">
          <div class="full-header">
            <div class="full-mode-identity">
              <span class="full-mode-swatch" style="background-color: {activeModeAccent}" aria-hidden="true" />
              <span class="full-mode-title" data-tauri-drag-region>{isProcessingVoiceEdit ? 'Editing' : activeModeName}</span>
            </div>
          </div>
          {#if isSensitiveApp}
            <div class="local-banner">
              <svg class="lock-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                <path d="M7 11V8a5 5 0 0 1 10 0v3M6 11h12a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <span>Local only</span>
            </div>
          {:else if fullContextSummary}
            <div class="ctx-summary">{fullContextSummary}</div>
          {/if}
          <div class="proc-center" data-tauri-drag-region="false">
            <div class="proc-indicator">
              <div class="spinner" aria-hidden="true" />
              <span class="proc-label" class:is-long={processingPastHalfExpected && processingAttempt === 1} class:is-retry={processingAttempt > 1}>
                {processingLabel}
                {#if showElapsedCounter}
                  <span class="elapsed">{processingElapsed}s</span>
                {/if}
              </span>
            </div>
            <button type="button" class="pill-cancel-btn" title="Cancel transcription" on:click|stopPropagation={cancelTranscription}>Cancel</button>
          </div>
          <div class="full-footer" data-tauri-drag-region="false" data-overlay-menu-root>
            <div class="mode-wrap">
              <button type="button" class="action-chip" aria-expanded={modeMenuOpen} on:click|stopPropagation={() => toggleModeMenu()}>{idleModeLabel} <span class="chevron" aria-hidden="true">{modeMenuOpen ? '▴' : '▾'}</span></button>
              {#if modeMenuOpen}
                <div class="mode-dropdown" role="menu">
                  {#each modesList as m (m.id)}
                    <button type="button" role="menuitem" on:click|stopPropagation={() => setActiveModeFromOverlay(m.id)}>{m.name}</button>
                  {/each}
                </div>
              {/if}
            </div>
            <button
              type="button"
              class="icon-btn"
              title="Shrink to compact pill"
              data-tauri-drag-region="false"
              on:pointerdown|stopPropagation={onCollapseChromePointerDown}
              on:click|stopPropagation={onCollapseChromeClick}
            >
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M4 14h6v6M20 10h-6V4M14 10l7-7M10 14l-7 7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </button>
          </div>
        </div>
      {:else}
        <!-- Mini processing -->
        <div class="content processing-body mini-proc-row" data-tauri-drag-region="false">
          <div class="spinner" aria-hidden="true" />
          <span class="proc-label" class:is-long={processingPastHalfExpected && processingAttempt === 1} class:is-retry={processingAttempt > 1}>
            {processingLabel}
            {#if showElapsedCounter}
              <span class="elapsed">{processingElapsed}s</span>
            {/if}
          </span>
          <button type="button" class="pill-cancel-btn mini-cancel ml-auto" title="Cancel transcription" on:click|stopPropagation={cancelTranscription}>Cancel</button>
        </div>
      {/if}

    <!-- ═══════════════ CANCELLING ═══════════════ -->
    {:else if state.kind === 'Cancelling'}
      <div class="content cancelling-state" data-tauri-drag-region>
        <div class="spinner spinner-small" aria-hidden="true" />
        <span class="label cancelling-label">Cancelling…</span>
      </div>

    <!-- ═══════════════ SUCCESS ═══════════════ -->
    {:else if state.kind === 'Success'}
      <div class="content success-state" data-tauri-drag-region>
        <div class="success-icon-wrap">
          <svg width="16" height="16" viewBox="0 0 20 20" fill="none" aria-hidden="true">
            <path d="M4 10.5L8 14.5L16 6.5" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <span class="label success-label">Transcribed</span>
      </div>

    <!-- ═══════════════ ERROR ═══════════════ -->
    {:else if state.kind === 'Error'}
      <div class="content error-state" role="alert">
        <div class="error-row">
          <div class="error-icon-wrap">
            <svg width="11" height="11" viewBox="0 0 18 18" fill="none" aria-hidden="true">
              <path d="M5 5L13 13M13 5L5 13" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"/>
            </svg>
          </div>
          <span class="label error-text">{friendlyError(state.message || 'Something went wrong')}</span>
        </div>
        <div class="error-actions" data-tauri-drag-region="false">
          <button type="button" class="pill-retry-btn" on:click={retryAfterError}>Retry</button>
          <button type="button" class="pill-dismiss-btn" on:click={dismissOverlayMessage}>Dismiss</button>
        </div>
      </div>
    {/if}
  </div>
</div>
{/if}

<style>
  /* ── Design tokens ── */
  :root {
    /* Cool-violet neutrals: slightly higher chroma + hue 260 for a cohesive surface personality */
    --ov-surface: oklch(12% 0.018 260);
    --ov-surface-raised: oklch(16% 0.02 260);
    --ov-border: oklch(24% 0.018 260);
    --ov-border-subtle: oklch(19% 0.015 260);
    --ov-text: oklch(92% 0.006 260);
    --ov-text-secondary: oklch(68% 0.012 260);
    --ov-text-muted: oklch(48% 0.01 260);
    --ov-accent: oklch(68% 0.1 240);
    --ov-command: oklch(68% 0.13 15);
    --ov-voice-edit: oklch(65% 0.1 290);
    --ov-sensitive: oklch(72% 0.12 75);
    --ov-success: oklch(72% 0.14 155);
    --ov-error: oklch(65% 0.16 25);
    --ov-error-soft: oklch(65% 0.08 25);
    --ease-out-quart: cubic-bezier(0.25, 1, 0.5, 1);
    --ease-out-expo: cubic-bezier(0.16, 1, 0.3, 1);
    /* Primary UI motion: smooth deceleration without feeling sluggish */
    --ease-smooth: cubic-bezier(0.22, 1, 0.36, 1);
    --ease-spring: cubic-bezier(0.34, 1.08, 0.64, 1);
    /* Decisive deceleration for dropdowns and entrances — no overshoot */
    --ease-spring-bouncy: cubic-bezier(0.16, 1, 0.3, 1);
    /* Quick press feedback — snappy settle, no rubber */
    --ease-squish: cubic-bezier(0.25, 1, 0.5, 1);
    --ov-dur-shell: 380ms; /* Slightly longer for the spring to settle */
    --ov-dur-reveal: 300ms;
    --ov-dur-micro: 140ms;
    --ov-dur-dropdown: 170ms;
    --ov-dur-panel-in: 290ms;
    --ov-dur-content: 200ms;
    --ov-dur-feedback: 320ms;
  }

  :global(html),
  :global(body),
  :global(#app) {
    background: transparent !important;
    border: none !important;
    outline: none !important;
    width: 100vw !important;
    height: 100vh !important;
    margin: 0 !important;
    padding: 0 !important;
    overflow: hidden !important;
  }

  /* Mode/context menus paint outside the pill; allow overflow while a menu is open */
  :global(html.kalam-ov-menu),
  :global(body.kalam-ov-menu),
  :global(#app.kalam-ov-menu) {
    overflow: visible !important;
  }

  /* ── Root container ── */
  .ov-root {
    position: relative;
    width: 100vw;
    height: 100vh;
    display: flex;
    justify-content: center;
    background: transparent;
    overflow: hidden;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
    font-size: 13px;
    line-height: 1.4;
    color: var(--ov-text);
    -webkit-font-smoothing: antialiased;
  }

  .ov-root.expand-up { align-items: flex-end; }
  .ov-root.expand-down { align-items: flex-start; }
  .ov-root.expand-center { align-items: center; }

  .ov-root.has-menu-open {
    overflow: visible;
  }

  /* ── Toast ── */
  .toast {
    position: absolute;
    /* Tight overlay window margin: keep toasts inside the chrome */
    top: 4px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 100;
    max-width: min(90vw, 320px);
    padding: 6px 12px;
    border-radius: 8px;
    font-size: 12px;
    font-weight: 500;
    color: var(--ov-text);
    background: var(--ov-surface-raised);
    border: 1px solid var(--ov-border-subtle);
    pointer-events: none;
    text-align: center;
    animation: toast-enter 220ms var(--ease-smooth) both;
  }

  /* Copy feedback sits below the pill so it never stacks on the auto-activate toast */
  .toast.toast-bottom {
    top: auto;
    bottom: 4px;
  }

  /* ── The pill ── */
  .pill {
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 100px;
    background: var(--ov-surface);
    border: 1px solid var(--ov-border-subtle);
    box-sizing: border-box;
    transition:
      max-width var(--ov-dur-shell) var(--ease-out-expo),
      min-height var(--ov-dur-shell) var(--ease-out-expo),
      height var(--ov-dur-shell) var(--ease-out-expo),
      border-radius var(--ov-dur-shell) var(--ease-out-expo),
      padding var(--ov-dur-shell) var(--ease-out-expo),
      opacity 200ms var(--ease-smooth),
      box-shadow 260ms var(--ease-smooth),
      border-color 260ms var(--ease-smooth);
    overflow: hidden;
    position: relative;
    flex-shrink: 0;
  }

  /* Let mode / context menus paint outside the pill (otherwise overflow:hidden clips them). */
  .pill.menu-open {
    overflow: visible;
  }

  /* Idle pill only (collapsed dot, mini strip, or maximized panel): black shell */
  .pill.dormant-pill {
    background: #000;
    border-color: oklch(26% 0.02 260);
  }

  /* ── Dormant tier ── */
  .pill.tier-dormant {
    width: auto;
    min-width: 48px;
    max-width: 48px;
    min-height: 28px;
    height: 28px;
    border-radius: 14px;
    opacity: 0.85;
    flex-direction: column;
    padding: 0;
    cursor: default;
    overflow: hidden;
    transition:
      max-width var(--ov-dur-shell) var(--ease-out-expo),
      min-height var(--ov-dur-shell) var(--ease-out-expo),
      height var(--ov-dur-shell) var(--ease-out-expo),
      border-radius var(--ov-dur-shell) var(--ease-out-expo),
      padding var(--ov-dur-shell) var(--ease-out-expo),
      opacity 260ms var(--ease-smooth),
      box-shadow 300ms var(--ease-smooth),
      border-color 260ms var(--ease-smooth);
  }

  .pill.tier-dormant.always-visible { opacity: 0.95; }
  /* Magnetic pre-hover: .ov-root receives mouseenter from the hit-test padding zone (~12px)
     before the cursor reaches .pill. Subtle opacity lift signals "I see you approaching." */
  .ov-root:hover .pill.tier-dormant { opacity: 0.95; }
  .pill.tier-dormant:hover { opacity: 1; }

  /* `.tier-dormant` sets overflow:hidden after `.menu-open` — re-allow dropdowns to escape the pill */
  .pill.tier-dormant.menu-open {
    overflow: visible;
  }

  /* Expanded dormant (on hover) - restores to last tier size */
  .pill.tier-dormant:has(.dormant-bar.expanded) {
    max-width: 290px;
    width: auto;
  }

  /* ── Mini tier ── */
  .pill.tier-mini {
    width: calc(100% - 8px);
    max-width: 290px;
    min-height: 42px;
    height: 44px;
    border-radius: 100px;
    opacity: 1;
  }

  /* ── Full tier ── */
  .pill.tier-full {
    width: calc(100% - 8px);
    max-width: 320px;
    min-height: 160px;
    height: auto;
    max-height: 200px;
    border-radius: 14px;
    opacity: 1;
    flex-direction: column;
    align-items: stretch;
    padding: 10px 12px 10px;
  }

  /* ── State accents ── */
  .pill.recording {
    border-color: color-mix(in oklch, var(--ov-accent) 50%, transparent);
    box-shadow: 0 0 10px color-mix(in oklch, var(--ov-accent) 20%, transparent);
    animation: recording-glow 2.2s cubic-bezier(0.45, 0, 0.55, 1) infinite;
    --glow-color: var(--ov-accent);
  }
  .pill.recording.is-command {
    border-color: color-mix(in oklch, var(--ov-command) 55%, transparent);
    box-shadow: 0 0 10px color-mix(in oklch, var(--ov-command) 20%, transparent);
    --glow-color: var(--ov-command);
  }
  .pill.recording.is-voice-edit {
    border-color: color-mix(in oklch, var(--ov-voice-edit) 55%, transparent);
    box-shadow: 0 0 10px color-mix(in oklch, var(--ov-voice-edit) 20%, transparent);
    --glow-color: var(--ov-voice-edit);
  }
  .pill.processing {
    border-color: color-mix(in oklch, var(--ov-accent) 25%, transparent);
  }
  .pill.processing-long {
    border-color: color-mix(in oklch, var(--ov-sensitive) 35%, transparent);
  }
  .pill.success {
    border-color: color-mix(in oklch, var(--ov-success) 55%, transparent);
    /* Brief tactile “done” cue without fighting layout (transform-origin keeps scale centered) */
    transform-origin: center center;
    animation: success-pulse 500ms var(--ease-out-expo);
    box-shadow: 0 0 14px color-mix(in oklch, var(--ov-success) 25%, transparent);
  }
  .pill.sensitive-app {
    border-color: color-mix(in oklch, var(--ov-sensitive) 40%, transparent);
  }
  .pill.error {
    border-color: color-mix(in oklch, var(--ov-error) 35%, transparent);
  }

  /* ── Dormant bar ── */
  .dormant-bar {
    display: flex;
    align-items: center;
    gap: 0;
    width: 100%;
    padding: 4px;
    box-sizing: border-box;
    /* Match shell easing so bar spacing opens in sync with the pill width */
    transition:
      gap var(--ov-dur-shell) var(--ease-smooth),
      padding var(--ov-dur-shell) var(--ease-smooth);
  }

  .dormant-bar:not(.expanded) {
    justify-content: center;
  }

  .dormant-bar:not(.expanded) .dormant-expand-inner {
    flex: 0 0 0;
    width: 0;
    min-width: 0;
    margin: 0;
    padding: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .dormant-idle-mark {
    flex-shrink: 0;
    /* WHY: The swatch is the only visible element when dormant. If it captures pointer events,
    hovering directly onto it (especially from top/bottom) may not trigger mouseenter on the
    parent .pill. pointer-events: none lets events pass through so .pill hover works reliably. */
    pointer-events: none;
  }

  .dormant-bar.expanded {
    gap: 8px;
    justify-content: flex-start;
    padding: 4px 6px 4px 12px;
  }

  .dormant-bar.expanded .dormant-expand-inner {
    pointer-events: auto;
  }

  /* Orchestrated reveal: shell opens first (expo), then content fades + slides in with a stagger. */
  .dormant-expand-inner {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 1;
    min-width: 0;
    opacity: 0;
    max-width: 0;
    overflow: hidden;
    transition:
      opacity 200ms var(--ease-smooth) 80ms,
      max-width var(--ov-dur-reveal) var(--ease-out-expo);
  }

  .dormant-expand-inner.menu-open {
    overflow: visible;
  }

  .dormant-bar.expanded .dormant-expand-inner {
    opacity: 1;
    max-width: 280px;
  }

  .dormant-left {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
    opacity: 0;
    transform: translateX(-4px);
    transition: opacity 180ms var(--ease-smooth) 100ms, transform 220ms var(--ease-out-expo) 100ms;
  }
  .dormant-bar.expanded .dormant-left {
    opacity: 1;
    transform: translateX(0);
  }

  .dormant-mode-wrap {
    position: relative;
    flex: 1;
    min-width: 0;
  }

  .dormant-mode-trigger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    padding: 2px 6px 2px 4px;
    margin: 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--ov-text);
    font: inherit;
    cursor: pointer;
    text-align: left;
    flex-shrink: 1;
    min-width: 0;
    transition: background var(--ov-dur-micro) var(--ease-smooth);
  }

  .dormant-mode-trigger:hover {
    background: color-mix(in oklch, var(--ov-text) 8%, transparent);
  }
  .dormant-mode-trigger:active {
    background: color-mix(in oklch, var(--ov-text) 12%, transparent);
  }

  .dormant-mode-swatch {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
    box-shadow: 0 0 0 1px color-mix(in oklch, var(--ov-text) 10%, transparent);
    transition: transform 200ms var(--ease-smooth), box-shadow 240ms var(--ease-smooth);
  }

  .dormant-idle-mark {
    cursor: pointer;
    animation: dormant-beacon 4.5s cubic-bezier(0.45, 0, 0.55, 1) infinite;
    transition: transform 240ms var(--ease-out-expo), box-shadow 280ms var(--ease-smooth);
  }
  /* Magnetic zone: swatch perks up slightly before full hover */
  .ov-root:hover .pill.tier-dormant .dormant-idle-mark {
    animation: none;
    transform: scale(1.12);
    box-shadow: 0 0 4px color-mix(in oklch, var(--ov-accent) 18%, transparent);
  }
  .pill.tier-dormant:hover .dormant-idle-mark {
    animation: none;
    transform: scale(1.25);
    box-shadow:
      0 0 0 2px color-mix(in oklch, var(--ov-text) 6%, transparent),
      0 0 8px color-mix(in oklch, var(--ov-accent) 30%, transparent);
  }

  .dormant-mode {
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.01em;
    color: var(--ov-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .dormant-mode-chevron {
    font-size: 0.65em;
    opacity: 0.55;
    flex-shrink: 0;
    line-height: 1;
  }

  .dormant-hotkey {
    font-size: 10px;
    font-weight: 500;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
    color: var(--ov-text-muted);
    white-space: nowrap;
    flex-shrink: 0;
    padding: 1px 5px;
    border-radius: 4px;
    background: color-mix(in oklch, var(--ov-text) 5%, transparent);
    border: 1px solid color-mix(in oklch, var(--ov-text) 8%, transparent);
    letter-spacing: 0.02em;
  }

  .dormant-icons {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
    opacity: 0;
    transform: translateX(4px);
    transition: opacity 180ms var(--ease-smooth) 160ms, transform 220ms var(--ease-out-expo) 160ms;
  }
  .dormant-bar.expanded .dormant-icons {
    opacity: 1;
    transform: translateX(0);
  }

  .dormant-clock {
    font-size: 10px;
    font-weight: 450;
    color: var(--ov-text-muted);
    white-space: nowrap;
    flex-shrink: 0;
    opacity: 0.7;
    font-variant-numeric: tabular-nums;
    margin-left: auto;
  }

  .mode-dropdown.mode-dropdown-dormant {
    z-index: 60;
  }

  .polish-badge {
    font-size: 9px;
    font-weight: 700;
    color: var(--ov-accent);
    background: color-mix(in oklch, var(--ov-accent) 15%, transparent);
    padding: 1px 4px;
    border-radius: 3px;
    line-height: 1.2;
    flex-shrink: 0;
  }

  /* ── Icon buttons (replace text chips in tight spaces) ── */
  .icon-btn {
    width: 26px;
    height: 26px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--ov-text-muted);
    cursor: pointer;
    flex-shrink: 0;
    transition:
      color var(--ov-dur-micro) var(--ease-smooth),
      background var(--ov-dur-micro) var(--ease-smooth),
      transform 160ms var(--ease-smooth);
  }
  .icon-btn:hover {
    color: var(--ov-text);
    background: color-mix(in oklch, var(--ov-text) 8%, transparent);
    transform: scale(1.08);
  }
  .icon-btn:active {
    transform: scale(0.93) translateY(1px);
    transition: transform 100ms var(--ease-squish);
  }
  .icon-btn:focus-visible {
    outline: 2px solid var(--ov-accent);
    outline-offset: 1px;
  }
  .icon-cancel:hover {
    color: var(--ov-error);
    background: color-mix(in oklch, var(--ov-error) 12%, transparent);
  }
  .ml-auto { margin-left: auto; }

  /* ── Status dots ── */
  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .listening-dot {
    background: var(--ov-accent);
    animation: breathe 2s cubic-bezier(0.45, 0, 0.55, 1) infinite;
  }



  /* ── Action chips ── */
  .action-chip {
    font-size: 12px;
    font-weight: 450;
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid var(--ov-border);
    background: var(--ov-surface-raised);
    color: var(--ov-text-secondary);
    cursor: pointer;
    white-space: nowrap;
    transition:
      background var(--ov-dur-micro) var(--ease-smooth),
      color var(--ov-dur-micro) var(--ease-smooth),
      transform 160ms var(--ease-smooth),
      border-color var(--ov-dur-micro) var(--ease-smooth);
  }
  .action-chip:hover {
    background: color-mix(in oklch, var(--ov-text) 8%, var(--ov-surface));
    color: var(--ov-text);
    border-color: var(--ov-border);
    transform: translateY(-1px);
  }
  .action-chip:active {
    transform: scale(0.95) translateY(1px);
    transition: transform 100ms var(--ease-squish);
  }
  .action-chip:focus-visible {
    outline: 2px solid var(--ov-accent);
    outline-offset: 1px;
  }

  .chevron {
    font-size: 0.75em;
    opacity: 0.6;
  }

  /* ── Mode dropdown ── */
  .mode-wrap {
    position: relative;
    display: inline-flex;
  }

  .mode-dropdown {
    position: absolute;
    bottom: 100%;
    left: 0;
    margin-bottom: 6px;
    min-width: 140px;
    padding: 4px;
    border-radius: 8px;
    background: color-mix(in oklch, var(--ov-surface-raised) 85%, transparent);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid color-mix(in oklch, var(--ov-border) 60%, transparent);
    box-shadow: 0 8px 24px color-mix(in oklch, black 25%, transparent);
    z-index: 20;
    display: flex;
    flex-direction: column;
    gap: 1px;
    animation: dropdown-enter 400ms var(--ease-spring-bouncy) both;
    transform-origin: bottom left;
  }

  /* Compact pill at top of HWND: default `bottom:100%` would clip above the window */
  .ov-root.expand-down .pill.tier-mini .mode-dropdown,
  .ov-root.expand-down .pill.tier-dormant .mode-dropdown {
    top: 100%;
    bottom: auto;
    margin-top: 6px;
    margin-bottom: 0;
    transform-origin: top left;
  }

  .mode-dropdown button {
    text-align: left;
    padding: 6px 10px;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--ov-text);
    font-size: 12px;
    cursor: pointer;
    animation: item-slide-in 300ms var(--ease-spring) both;
  }
  .mode-dropdown button:nth-child(1) { animation-delay: 20ms; }
  .mode-dropdown button:nth-child(2) { animation-delay: 40ms; }
  .mode-dropdown button:nth-child(3) { animation-delay: 60ms; }
  .mode-dropdown button:nth-child(4) { animation-delay: 80ms; }
  .mode-dropdown button:nth-child(5) { animation-delay: 100ms; }
  .mode-dropdown button:nth-child(6) { animation-delay: 120ms; }
  .mode-dropdown button:hover {
    background: color-mix(in oklch, var(--ov-text) 8%, var(--ov-surface));
  }
  .mode-dropdown button:focus-visible {
    outline: 2px solid var(--ov-accent);
    outline-offset: -1px;
  }

  /* ── Context menu ── */
  .ctx-menu {
    position: fixed;
    z-index: 50;
    min-width: 160px;
    padding: 4px;
    border-radius: 8px;
    background: color-mix(in oklch, var(--ov-surface-raised) 85%, transparent);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid color-mix(in oklch, var(--ov-border) 60%, transparent);
    box-shadow: 0 8px 24px color-mix(in oklch, black 25%, transparent);
    display: flex;
    flex-direction: column;
    gap: 1px;
    animation: dropdown-enter 400ms var(--ease-spring-bouncy) both;
    transform-origin: top left;
  }
  .ctx-menu button {
    text-align: left;
    padding: 7px 10px;
    border: none;
    background: transparent;
    color: var(--ov-text);
    font-size: 12px;
    cursor: pointer;
    border-radius: 5px;
    animation: item-slide-in 300ms var(--ease-spring) both;
  }
  .ctx-menu button:nth-child(1) { animation-delay: 20ms; }
  .ctx-menu button:nth-child(2) { animation-delay: 40ms; }
  .ctx-menu button:nth-child(3) { animation-delay: 60ms; }
  .ctx-menu button:nth-child(4) { animation-delay: 80ms; }
  .ctx-menu button:hover {
    background: color-mix(in oklch, var(--ov-text) 8%, var(--ov-surface));
  }

  /* ── Full panel ── */
  .full-panel {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
    min-height: 0;
    flex: 1;
    animation: panel-enter var(--ov-dur-panel-in) var(--ease-smooth) both;
  }

  .full-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 4px;
    flex-shrink: 0;
  }

  /* Mode identity row: swatch + title (+ optional tag) */
  .full-mode-identity {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex: 1;
  }

  .full-mode-swatch {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex-shrink: 0;
    box-shadow: 0 0 0 1.5px color-mix(in oklch, var(--ov-text) 10%, transparent);
  }

  .full-mode-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--ov-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .header-polish-tag {
    font-size: 10px;
    font-weight: 500;
    color: var(--ov-accent);
    background: color-mix(in oklch, var(--ov-accent) 12%, transparent);
    padding: 1px 5px;
    border-radius: 3px;
    flex-shrink: 0;
    line-height: 1.3;
  }

  .full-sub {
    font-size: 12px;
    color: var(--ov-text-muted);
    margin: 0;
    text-align: center;
    flex-shrink: 0;
  }

  .hotkey-kbd {
    color: var(--ov-accent);
    font-weight: 600;
    font-family: inherit;
    background: none;
    border: none;
    padding: 0;
  }

  .polish-chip {
    font-size: 11px;
    font-weight: 450;
    padding: 3px 8px;
    border-radius: 5px;
    border: none;
    background: color-mix(in oklch, var(--ov-text) 6%, transparent);
    color: var(--ov-text-muted);
    cursor: pointer;
    flex-shrink: 0;
    transition: background var(--ov-dur-micro) var(--ease-smooth), color var(--ov-dur-micro) var(--ease-smooth);
  }
  .polish-chip:hover {
    background: color-mix(in oklch, var(--ov-text) 10%, transparent);
    color: var(--ov-text-secondary);
  }
  .polish-chip[aria-pressed="true"] {
    background: color-mix(in oklch, var(--ov-accent) 15%, transparent);
    color: var(--ov-accent);
  }

  /* Context summary — source names only, not content; wrap so long titles stay readable */
  .ctx-summary {
    font-size: 11px;
    line-height: 1.35;
    color: var(--ov-text-muted);
    padding: 4px 6px;
    border-radius: 6px;
    background: color-mix(in oklch, var(--ov-surface) 50%, black);
    min-width: 0;
    max-width: 100%;
    width: fit-content;
    align-self: center;
    box-sizing: border-box;
    white-space: normal;
    /* Prefer breaks between words; allow mid-token breaks for very long titles/paths */
    overflow-wrap: anywhere;
    text-align: center;
  }

  .local-banner {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 8px;
    border-radius: 6px;
    background: color-mix(in oklch, var(--ov-sensitive) 10%, var(--ov-surface));
    border: 1px solid color-mix(in oklch, var(--ov-sensitive) 30%, transparent);
    color: var(--ov-sensitive);
    font-size: 11px;
    font-weight: 500;
  }

  /* ── Stop / Cancel / Retry / Dismiss buttons ── */
  .pill-stop-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px 4px 8px;
    border-radius: 6px;
    border: none;
    background: color-mix(in oklch, var(--ov-error) 14%, transparent);
    color: var(--ov-error);
    font-size: 11px;
    font-weight: 550;
    cursor: pointer;
    flex-shrink: 0;
    transition: background var(--ov-dur-micro) var(--ease-smooth), transform 160ms var(--ease-smooth);
  }
  .pill-stop-btn:hover {
    background: color-mix(in oklch, var(--ov-error) 22%, transparent);
  }
  .pill-stop-btn:active { transform: scale(0.95) translateY(1px); transition: transform 100ms var(--ease-squish); }

  .pill-cancel-btn {
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid color-mix(in oklch, var(--ov-text) 12%, transparent);
    background: transparent;
    color: var(--ov-text-muted);
    font-size: 11px;
    font-weight: 450;
    cursor: pointer;
    flex-shrink: 0;
    transition:
      color var(--ov-dur-micro) var(--ease-smooth),
      border-color var(--ov-dur-micro) var(--ease-smooth),
      background var(--ov-dur-micro) var(--ease-smooth),
      transform 160ms var(--ease-smooth);
  }
  .pill-cancel-btn:hover {
    color: var(--ov-error);
    border-color: color-mix(in oklch, var(--ov-error) 30%, transparent);
    background: color-mix(in oklch, var(--ov-error) 6%, transparent);
  }
  .pill-cancel-btn:active { transform: scale(0.95) translateY(1px); transition: transform 100ms var(--ease-squish); }
  .pill-cancel-btn.mini-cancel {
    padding: 3px 8px;
    font-size: 10px;
  }

  .pill-retry-btn {
    padding: 5px 14px;
    border-radius: 6px;
    border: 1px solid color-mix(in oklch, var(--ov-error) 40%, transparent);
    background: color-mix(in oklch, var(--ov-error) 12%, transparent);
    color: var(--ov-error);
    font-size: 12px;
    font-weight: 550;
    cursor: pointer;
    transition: background var(--ov-dur-micro) var(--ease-smooth), transform 160ms var(--ease-smooth);
  }
  .pill-retry-btn:hover {
    background: color-mix(in oklch, var(--ov-error) 20%, transparent);
  }
  .pill-retry-btn:active { transform: scale(0.95) translateY(1px); transition: transform 100ms var(--ease-squish); }

  .pill-dismiss-btn {
    padding: 5px 12px;
    border-radius: 6px;
    border: 1px solid color-mix(in oklch, var(--ov-text) 10%, transparent);
    background: transparent;
    color: var(--ov-text-muted);
    font-size: 12px;
    font-weight: 450;
    cursor: pointer;
    transition:
      color var(--ov-dur-micro) var(--ease-smooth),
      background var(--ov-dur-micro) var(--ease-smooth),
      transform 160ms var(--ease-smooth);
  }
  .pill-dismiss-btn:hover {
    color: var(--ov-text);
    background: color-mix(in oklch, var(--ov-text) 6%, transparent);
  }
  .pill-dismiss-btn:active { transform: scale(0.95) translateY(1px); transition: transform 100ms var(--ease-squish); }

  /* Processing center area */
  .proc-center {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    flex: 1;
    justify-content: center;
    padding: 8px 0;
    animation: content-enter var(--ov-dur-content) var(--ease-smooth) both;
  }
  .proc-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .full-footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: 6px;
    margin-top: auto;
    padding-top: 8px;
    border-top: 1px solid var(--ov-border-subtle);
    flex-shrink: 0;
  }

  /* ── Mini rows ── */
  .mini-proc-row {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 100%;
    padding: 0 8px 0 12px;
    box-sizing: border-box;
  }

  /* ── Mini recording — layered: waveform behind, controls on top ── */
  .mini-rec {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    border-radius: inherit;
    animation: content-enter var(--ov-dur-content) var(--ease-smooth) both;
  }

  /* Waveform fills the entire pill area as a background */
  .mini-rec-wave-layer {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    /* Soft fade at edges so the waveform bleeds into the pill shape */
    -webkit-mask-image: linear-gradient(to right, transparent 0%, black 18%, black 82%, transparent 100%);
    mask-image: linear-gradient(to right, transparent 0%, black 18%, black 82%, transparent 100%);
    opacity: 0.7;
  }

  /* Controls float above the waveform */
  .mini-rec-controls {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    height: 100%;
    padding: 0 6px 0 12px;
    box-sizing: border-box;
  }

  .mini-rec-left {
    display: flex;
    align-items: center;
    gap: 7px;
    flex-shrink: 0;
  }

  .mini-rec-dot {
    --rec-accent: var(--ov-accent);
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--rec-accent);
    box-shadow: 0 0 5px color-mix(in oklch, var(--rec-accent) 50%, transparent);
    animation: rec-pulse 1.2s ease-in-out infinite;
    flex-shrink: 0;
  }

  .mini-rec-mode {
    font-size: 11px;
    font-weight: 600;
    color: var(--ov-text);
    white-space: nowrap;
    text-shadow: 0 0 8px var(--ov-surface), 0 0 12px var(--ov-surface);
  }

  .mini-rec-right {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }

  .mini-rec-btn {
    width: 28px;
    height: 28px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    border: none;
    cursor: pointer;
    flex-shrink: 0;
    transition:
      background var(--ov-dur-micro) var(--ease-smooth),
      color var(--ov-dur-micro) var(--ease-smooth),
      transform 160ms var(--ease-smooth);
    /* Subtle backdrop so buttons read clearly over the waveform */
    background: color-mix(in oklch, var(--ov-surface) 60%, transparent);
    color: var(--ov-text-muted);
  }
  .mini-rec-btn:hover {
    background: color-mix(in oklch, var(--ov-surface) 85%, transparent);
    color: var(--ov-text);
    transform: scale(1.08);
  }
  .mini-rec-btn:active {
    transform: scale(0.93) translateY(1px);
    transition: transform 100ms var(--ease-squish);
  }
  .mini-rec-cancel:hover {
    color: var(--ov-text-muted);
  }
  .mini-rec-stop:hover {
    color: var(--ov-error);
    background: color-mix(in oklch, var(--ov-error) 12%, color-mix(in oklch, var(--ov-surface) 70%, transparent));
  }

  /* ── Content wrapper ── */
  .content {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    animation: content-enter var(--ov-dur-content) var(--ease-smooth) both;
    width: auto;
    height: auto;
    max-width: 100%;
    max-height: 100%;
    flex-shrink: 1;
  }
  .content.waveform {
    width: 100%;
    height: 100%;
  }

  /* ── Listening ── */
  .listening.sensitive .lock-icon {
    color: var(--ov-sensitive);
    flex-shrink: 0;
  }
  .sensitive-label {
    color: var(--ov-sensitive) !important;
    font-weight: 550;
  }

  .label {
    font-size: 13px;
    font-weight: 450;
    color: var(--ov-text);
    white-space: nowrap;
  }
  .hint .label {
    font-size: 12px;
    color: var(--ov-text-muted);
  }

  /* ── Status ── */
  .status-message .label {
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 230px;
  }
  .highlight-text {
    color: var(--ov-success);
    font-weight: 600;
    margin-left: 2px;
  }

  /* ── Success ── */
  .success-state {
    animation: success-enter var(--ov-dur-feedback) var(--ease-smooth) both;
    gap: 10px;
  }
  .success-icon-wrap {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: color-mix(in oklch, var(--ov-success) 20%, transparent);
    box-shadow: 0 0 10px color-mix(in oklch, var(--ov-success) 18%, transparent);
    color: var(--ov-success);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    animation: success-icon-pop 340ms var(--ease-smooth) 70ms both;
  }
  .success-label {
    color: var(--ov-success) !important;
    font-weight: 550;
    font-size: 13px;
  }

  /* ── Cancelling ── */
  .cancelling-state {
    gap: 8px;
  }
  .cancelling-label {
    color: var(--ov-text-muted);
    font-size: 12px;
    font-weight: 450;
  }
  .spinner-small {
    width: 12px;
    height: 12px;
    border-width: 1.5px;
  }

  /* ── Error ── */
  .error-state {
    flex-direction: column;
    align-items: center;
    gap: 10px;
    max-width: min(280px, 92vw);
    padding: 8px 14px;
    text-align: center;
    animation: error-enter var(--ov-dur-feedback) var(--ease-smooth) both;
  }
  .error-row {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: center;
    gap: 8px;
    text-align: left;
  }
  .error-icon-wrap {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: color-mix(in oklch, var(--ov-error) 14%, transparent);
    color: var(--ov-error);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .error-text {
    white-space: normal !important;
    max-width: 200px;
    color: color-mix(in oklch, var(--ov-error) 80%, var(--ov-text)) !important;
    flex: 1 1 auto;
    min-width: 0;
    font-size: 12px !important;
    text-align: left;
  }
  .error-actions {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    flex-shrink: 0;
  }

  /* ── Processing ── */
  .processing-body {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    box-sizing: border-box;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid color-mix(in oklch, var(--ov-accent) 20%, transparent);
    border-top-color: var(--ov-accent);
    border-radius: 50%;
    background: none;
    box-shadow: none;
    animation: spin 0.8s var(--ease-smooth) infinite;
    flex-shrink: 0;
  }

  .proc-label {
    font-size: 12px;
    font-weight: 450;
    color: var(--ov-text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .proc-label.is-long { color: var(--ov-sensitive); }
  .proc-label.is-retry { color: var(--ov-error); }

  .elapsed {
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--ov-text-muted);
    margin-left: 4px;
  }
  .proc-label.is-long .elapsed { color: var(--ov-sensitive); }
  .proc-label.is-retry .elapsed { color: var(--ov-error); }

  /* ── Waveform ── */
  .waveform {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    padding: 0;
    box-sizing: border-box;
  }

  .full-wave {
    flex: 1;
    min-height: 50px;
    max-height: 65px;
    -webkit-mask-image: linear-gradient(to right, transparent 0%, black 8%, black 92%, transparent 100%);
    mask-image: linear-gradient(to right, transparent 0%, black 8%, black 92%, transparent 100%);
  }

  .wave-canvas {
    width: 100%;
    height: 100%;
    display: block;
  }

  .viz-css {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* ── CSS visualizations (EchoRing, RoundedBars, BreathingAura, NeonPulse) ── */
  .viz-echo { position: relative; width: 28px; height: 28px; display: flex; align-items: center; justify-content: center; }
  .viz-echo .core { width: 8px; height: 8px; border-radius: 50%; background: var(--ov-accent); transform: scale(var(--echo-core, 1)); will-change: transform; }
  .viz-echo .ring { position: absolute; border-radius: 50%; border: 1.5px solid var(--ov-accent); box-sizing: border-box; will-change: transform, opacity; }
  .viz-echo .r1 { width: 16px; height: 16px; transform: scale(var(--echo-scale1, 0.5)); opacity: var(--echo-op1, 0); }
  .viz-echo .r2 { width: 24px; height: 24px; transform: scale(var(--echo-scale2, 0.5)); opacity: var(--echo-op2, 0); }
  .viz-echo .r3 { width: 34px; height: 34px; transform: scale(var(--echo-scale3, 0.5)); opacity: var(--echo-op3, 0); }
  .viz-cmd .viz-echo .core { background: var(--ov-command); }
  .viz-cmd .viz-echo .ring { border-color: var(--ov-command); }
  .viz-voice-edit .viz-echo .core { background: var(--ov-voice-edit); }
  .viz-voice-edit .viz-echo .ring { border-color: var(--ov-voice-edit); }
  .viz-sensitive .viz-echo .core { background: var(--ov-sensitive); }
  .viz-sensitive .viz-echo .ring { border-color: var(--ov-sensitive); }

  .viz-bars { display: flex; align-items: center; justify-content: center; gap: 3px; height: 28px; width: 100%; }
  .viz-bars span { width: 3.5px; height: 24px; border-radius: 3px; background: var(--ov-accent); transform-origin: center; transform: scaleY(var(--b0, 0.15)); will-change: transform; }
  .viz-bars span:nth-child(1) { transform: scaleY(var(--b0, 0.15)); }
  .viz-bars span:nth-child(2) { transform: scaleY(var(--b1, 0.15)); }
  .viz-bars span:nth-child(3) { transform: scaleY(var(--b2, 0.15)); }
  .viz-bars span:nth-child(4) { transform: scaleY(var(--b3, 0.15)); }
  .viz-bars span:nth-child(5) { transform: scaleY(var(--b4, 0.15)); }
  .viz-bars span:nth-child(6) { transform: scaleY(var(--b5, 0.15)); }
  .viz-bars span:nth-child(7) { transform: scaleY(var(--b6, 0.15)); }
  .viz-bars span:nth-child(8) { transform: scaleY(var(--b7, 0.15)); }
  .viz-bars span:nth-child(9) { transform: scaleY(var(--b8, 0.15)); }
  .viz-bars span:nth-child(10) { transform: scaleY(var(--b9, 0.15)); }
  .viz-bars span:nth-child(11) { transform: scaleY(var(--b10, 0.15)); }
  .viz-cmd .viz-bars span { background: var(--ov-command); }
  .viz-voice-edit .viz-bars span { background: var(--ov-voice-edit); }
  .viz-sensitive .viz-bars span { background: var(--ov-sensitive); }

  .viz-aura { display: flex; align-items: center; justify-content: center; width: 100%; height: 100%; }
  .aura-orb { width: 28px; height: 28px; border-radius: 50%; background: radial-gradient(circle at 40% 40%, color-mix(in oklch, var(--ov-accent) 90%, white), color-mix(in oklch, var(--ov-accent) 40%, transparent) 60%, transparent 80%); transform: scale(var(--aura-scale, 0.5)); box-shadow: 0 0 calc(var(--aura-glow, 4) * 1px) var(--ov-accent); will-change: transform, box-shadow; }
  .viz-cmd .aura-orb { background: radial-gradient(circle at 40% 40%, color-mix(in oklch, var(--ov-command) 90%, white), color-mix(in oklch, var(--ov-command) 40%, transparent) 60%, transparent 80%); box-shadow: 0 0 calc(var(--aura-glow, 4) * 1px) var(--ov-command); }
  .viz-voice-edit .aura-orb { background: radial-gradient(circle at 40% 40%, color-mix(in oklch, var(--ov-voice-edit) 90%, white), color-mix(in oklch, var(--ov-voice-edit) 40%, transparent) 60%, transparent 80%); box-shadow: 0 0 calc(var(--aura-glow, 4) * 1px) var(--ov-voice-edit); }
  .viz-sensitive .aura-orb { background: radial-gradient(circle at 40% 40%, color-mix(in oklch, var(--ov-sensitive) 90%, white), color-mix(in oklch, var(--ov-sensitive) 40%, transparent) 60%, transparent 80%); box-shadow: 0 0 calc(var(--aura-glow, 4) * 1px) var(--ov-sensitive); }

  .viz-neon { width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; }
  .viz-neon .beam { width: calc(40px + var(--neon-w, 0) * 100px); height: calc(2px + var(--neon-h, 0) * 8px); background: var(--ov-accent); border-radius: 10px; box-shadow: 0 0 calc(5px + var(--neon-g, 0) * 12px) var(--ov-accent); will-change: width, height, box-shadow; }
  .viz-cmd .viz-neon .beam { background: var(--ov-command); box-shadow: 0 0 calc(5px + var(--neon-g, 0) * 12px) var(--ov-command); }
  .viz-voice-edit .viz-neon .beam { background: var(--ov-voice-edit); box-shadow: 0 0 calc(5px + var(--neon-g, 0) * 12px) var(--ov-voice-edit); }
  .viz-sensitive .viz-neon .beam { background: var(--ov-sensitive); box-shadow: 0 0 calc(5px + var(--neon-g, 0) * 12px) var(--ov-sensitive); }

  /* ── Keyframes ── */
  /* Hello entrance — confident arrival when the pill first appears after app launch */
  .pill.hello {
    animation: pill-hello 600ms var(--ease-out-expo) both;
  }
  @keyframes pill-hello {
    0% { opacity: 0; transform: scale(0.6) translateY(8px); }
    100% { opacity: 1; transform: scale(1) translateY(0); }
  }

  @keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes toast-enter {
    from { opacity: 0; transform: translateX(-50%) translateY(-4px); }
    to { opacity: 1; transform: translateX(-50%) translateY(0); }
  }

  @keyframes breathe {
    0%, 100% { transform: scale(1); opacity: 1; }
    50% { transform: scale(1.35); opacity: 0.5; }
  }

  @keyframes success-pulse {
    0% { transform: scale(1); }
    30% { transform: scale(1.04); }
    60% { transform: scale(0.997); }
    100% { transform: scale(1); }
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Full panel content entrance — staggered fade-in */
  @keyframes panel-enter {
    from { opacity: 0; transform: translateY(2px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* Dormant swatch — slow, deep breath with a soft accent glow ring */
  @keyframes dormant-beacon {
    0%, 100% { opacity: 0.75; transform: scale(1); box-shadow: 0 0 0 1px color-mix(in oklch, var(--ov-text) 10%, transparent); }
    50% { opacity: 1; transform: scale(1.3); box-shadow: 0 0 6px 1px color-mix(in oklch, var(--ov-accent) 22%, transparent), 0 0 0 1px color-mix(in oklch, var(--ov-text) 6%, transparent); }
  }

  /* Recording pill border glow — alive, breathing */
  @keyframes recording-glow {
    0%, 100% { box-shadow: 0 0 8px color-mix(in oklch, var(--glow-color) 18%, transparent); }
    50% { box-shadow: 0 0 14px color-mix(in oklch, var(--glow-color) 30%, transparent); }
  }

  /* Recording dot — faster pulse to feel urgent/alive */
  @keyframes rec-pulse {
    0%, 100% { transform: scale(1); opacity: 1; }
    50% { transform: scale(1.3); opacity: 0.65; }
  }

  /* Content entrance — fade + slight upward drift */
  @keyframes content-enter {
    from { opacity: 0; transform: translateY(6px) scale(0.98); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }

  /* Success entrance — scale up from slightly small */
  @keyframes success-enter {
    from { opacity: 0; transform: scale(0.96); }
    to { opacity: 1; transform: scale(1); }
  }

  /* Success icon — small overshoot reads crisp without a bouncy settle */
  @keyframes success-icon-pop {
    0% { transform: scale(0.4) rotate(-15deg); opacity: 0; }
    60% { transform: scale(1.2) rotate(5deg); opacity: 1; }
    100% { transform: scale(1) rotate(0); opacity: 1; }
  }

  /* Dropdown entrance — scale up from origin */
  @keyframes dropdown-enter {
    from { opacity: 0; transform: scale(0.85) translateY(10px); }
    to { opacity: 1; transform: scale(1) translateY(0); }
  }

  @keyframes item-slide-in {
    from { opacity: 0; transform: translateX(-6px); }
    to { opacity: 1; transform: translateX(0); }
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Error entrance — subtle horizontal shake */
  @keyframes error-enter {
    0% { opacity: 0; transform: translateX(-3px); }
    22% { transform: translateX(2px); }
    48% { transform: translateX(-1px); }
    74% { transform: translateX(0.5px); }
    100% { opacity: 1; transform: translateX(0); }
  }

  /* ── Reduced motion ── */
  @media (prefers-reduced-motion: reduce) {
    .listening-dot { animation: none; opacity: 0.8; transform: none; }
    .pill.success { animation: none; }
    .pill.recording { animation: none; }
    .pill { transition-duration: 0ms; }
    .spinner { animation-duration: 2s; }
    .toast { animation: none; }
    .content { animation: none; }
    .success-state { animation: none; }
    .error-state { animation: none; }
    .full-panel { animation: none; }
    .mode-dropdown { animation: none; }
    .ctx-menu { animation: none; }
    .dormant-expand-inner { transition: none; }
    .dormant-bar { transition: none; }
    .dormant-idle-mark { animation: none; }
    .dormant-left { transition: none; }
    .dormant-icons { transition: none; }
    .pill.hello { animation: none; }
    .mini-rec-dot { animation: none; }
    .mini-rec-btn { transition: none; }
    .icon-btn { transition: none; }
    .action-chip { transition: none; }
    .success-icon-wrap { animation: none; }
    .pill-stop-btn, .pill-cancel-btn, .pill-retry-btn, .pill-dismiss-btn { transition: none; }
    .proc-center { animation: none; }
  }
</style>
