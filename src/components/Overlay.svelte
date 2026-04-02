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

  async function goToDormantIdle() {
    // Save current tier before going dormant (for hover restore)
    if (overlayLayoutTier !== 'Dormant') {
      lastTier = overlayLayoutTier
    }
    overlayLayoutTier = 'Dormant'
    state = makeLocalDormant()
    await invoke('resize_overlay_to', { size: 'Dormant' }).catch(console.error)
  }

  function dismissOverlayMessage() {
    if (statusTimeout) clearTimeout(statusTimeout)
    statusTimeout = null
    void goToDormantIdle()
  }

  async function retryAfterError() {
    dismissOverlayMessage()
    try {
      await invoke('focus_main_window')
    } catch (e) {
      console.error('focus_main_window failed:', e)
    }
  }

  /** Cancel is always visible during processing — no hover gate. */
  $: showCancelButton = state.kind === 'Processing'
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
    if (processingAttempt > 1) return `Retrying… (attempt ${processingAttempt})`
    if (processingPastHalfExpected) return 'Still processing…'
    return processingMessage ?? 'Transcribing…'
  })()

  $: rawLevel = state.kind === 'Recording' ? Number(state.level) || 0 : 0
  $: isCommand = state.kind === 'Recording' ? Boolean(state.is_command) : false
  $: isVoiceEdit = state.kind === 'Recording' ? Boolean(state.is_voice_edit) : false
  $: isProcessingVoiceEdit = state.kind === 'Processing' ? Boolean(state.is_voice_edit) : false

  $: isSensitiveApp =
    state.kind === 'SensitiveAppPeek' ||
    (state.kind === 'Listening' && Boolean(state.sensitive_app)) ||
    (state.kind === 'Recording' && Boolean(state.sensitive_app))

  /** Context source labels for tooltip (no emojis — plain text). */
  const SOURCE_LABELS: Record<string, string> = {
    app: 'App',
    clipboard: 'Clipboard',
    selection: 'Selection',
    system: 'System',
  }

  function contextSourcesSummary(sources: string[] | undefined): string {
    if (!sources?.length) return ''
    return sources.map((s) => SOURCE_LABELS[s] ?? s).join(' · ')
  }

  $: contextUiActive =
    !isSensitiveApp &&
    ((state.kind === 'Recording' && Boolean(state.context_active)) ||
      (state.kind === 'Processing' && Boolean(state.context_active)))
  $: contextUiLabel =
    state.kind === 'Recording' || state.kind === 'Processing'
      ? (typeof state.context_label === 'string' ? state.context_label : '') || ''
      : ''
  $: contextUiSummary =
    state.kind === 'Recording' || state.kind === 'Processing'
      ? contextSourcesSummary(state.context_sources)
      : ''

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


  $: idleModeLabel =
    state.kind === 'Dormant' && typeof state.mode_name === 'string' && state.mode_name.trim() !== ''
      ? state.mode_name
      : activeModeName

  $: copyLastTitle =
    state.kind === 'Dormant' && state.last_transcription_preview
      ? `Copy: ${state.last_transcription_preview.slice(0, 42)}${state.last_transcription_preview.length > 42 ? '…' : ''}`
      : 'Copy last transcription'

  $: isExpanded =
    state.kind !== 'Dormant' && state.kind !== 'Hidden'

  let isHovered = false

  async function onBlipMouseEnter() {
    isHovered = true
    // If dormant, restore to last tier (Mini or Full)
    if (state.kind === 'Dormant' && overlayLayoutTier === 'Dormant') {
      overlayLayoutTier = lastTier
      await invoke('resize_overlay_to', { size: lastTier }).catch(() => {})
    }
  }

  async function onBlipMouseLeave() {
    isHovered = false
    // If still dormant (not recording/processing), retract to dormant
    if (state.kind === 'Dormant' && overlayLayoutTier !== 'Dormant') {
      // Save current tier before retracting
      lastTier = overlayLayoutTier
      overlayLayoutTier = 'Dormant'
      await invoke('resize_overlay_to', { size: 'Dormant' }).catch(() => {})
    }
  }

  /** Close menus when state leaves dormant. */
  $: if (state.kind !== 'Dormant') {
    modeMenuOpen = false
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
      overlayLayoutTier = 'Dormant'
      await invoke('resize_overlay_to', { size: 'Dormant' }).catch(() => {})
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
      overlayLayoutTier = newTier
      await invoke('resize_overlay_to', { size: newTier }).catch(() => {})
      if (newTier === 'Full') await loadContextPreviewsOnly()
      return
    }
    if (kind === 'Recording' || kind === 'Processing') {
      // Use active preference or maintain current tier
      const newTier = overlayLayoutTier === 'Dormant' ? lastTier : overlayLayoutTier
      if (overlayLayoutTier === 'Dormant') {
        overlayLayoutTier = newTier
      }
      await invoke('resize_overlay_to', { size: newTier }).catch(() => {})
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
    lastTier = 'Full'
    overlayLayoutTier = 'Full'
    contextPreviews = null
    await invoke('resize_overlay_to', { size: 'Full' }).catch(() => {})
    await loadContextPreviewsOnly()
  }

  /** Collapse to dormant idle (not Mini) — returns to the compact pill. */
  async function collapseToDormant() {
    // Save current tier before going dormant (for hover restore)
    if (overlayLayoutTier !== 'Dormant') {
      lastTier = overlayLayoutTier
    }
    overlayLayoutTier = 'Dormant'
    contextPreviews = null
    state = makeLocalDormant()
    await invoke('resize_overlay_to', { size: 'Dormant' }).catch(() => {})
  }

  async function collapseToMiniChrome() {
    lastTier = 'Mini'
    overlayLayoutTier = 'Mini'
    contextPreviews = null
    await invoke('resize_overlay_to', { size: 'Mini' }).catch(() => {})
  }

  async function copyLastTranscription() {
    try {
      await invoke('copy_last_transcription')
    } catch (e) {
      console.error('copy_last_transcription failed:', e)
    }
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
      document.removeEventListener('pointerdown', onDocPointerDown, true)
      document.removeEventListener('keydown', onKeyDown, true)
      if (statusTimeout) clearTimeout(statusTimeout)
      if (pendingSuccessTimer) clearTimeout(pendingSuccessTimer)
      if (autoActivateToastTimer) clearTimeout(autoActivateToastTimer)
      keepAliveWorker?.terminate()
    }
  })
</script>

{#if state.kind !== 'Hidden'}
<div
  class="ov-root"
  class:expand-up={expandDirection === 'Up'}
  class:expand-down={expandDirection === 'Down'}
  class:expand-center={expandDirection === 'Center'}
  role="status"
  aria-label="Kalam dictation status"
>
  {#if autoActivateToast}
    <div class="toast" role="status" aria-live="polite">{autoActivateToast}</div>
  {/if}
  <div
    class="pill"
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
    data-tauri-drag-region
    on:mouseenter={onBlipMouseEnter}
    on:mouseleave={onBlipMouseLeave}
    on:contextmenu={onRootContextMenu}
  >
    <!-- ═══════════════ DORMANT ═══════════════ -->
    {#if state.kind === 'Dormant'}
      {#if overlayLayoutTier === 'Full'}
        <div class="full-panel dormant-full">
          <div class="full-header">
            <span class="full-mode-title">{idleModeLabel}</span>
            <button
              type="button"
              class="polish-chip"
              aria-pressed={polishEnabledOverlay}
              data-overlay-menu-root
              on:click|stopPropagation={() => togglePolishFromOverlay()}
            >
              Clean up {polishEnabledOverlay ? 'on' : 'off'}
            </button>
          </div>
          <p class="full-sub">Press <kbd class="hotkey-kbd">{hotkeyDisplayStr}</kbd> to dictate</p>
          {#if fullContextSummary}
            <div class="ctx-summary">Context: {fullContextSummary}</div>
          {/if}
          <div class="full-footer" data-overlay-menu-root>
            <div class="mode-wrap">
              <button type="button" class="action-chip" aria-expanded={modeMenuOpen} on:click|stopPropagation={() => (modeMenuOpen = !modeMenuOpen)}
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
            <!-- Collapse back to dormant compact pill -->
            <button type="button" class="icon-btn" title="Collapse to pill" on:click|stopPropagation={() => collapseToDormant()}>
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M4 14h6v6M20 10h-6V4M14 10l7-7M10 14l-7 7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </button>
          </div>
        </div>
      {:else}
        <!-- Dormant compact: ultra-minimal dot only, expands on hover -->
        <div class="dormant-bar" class:expanded={overlayLayoutTier !== 'Dormant'}>
          <span
            class="status-dot dormant-dot"
            aria-hidden="true"
            style={dormantDotBackground ? `background-color: ${dormantDotBackground}` : undefined}
          />
          <div class="dormant-content">
            <span class="dormant-mode">{idleModeLabel}</span>
            {#if polishEnabledOverlay}
              <span class="polish-badge" title="Text clean-up is on">P</span>
            {/if}
            <span class="dormant-hotkey">{hotkeyDisplayStr}</span>
            <div class="dormant-icons" data-overlay-menu-root>
              <!-- Copy last transcription -->
              <button type="button" class="icon-btn" title={copyLastTitle} on:click|stopPropagation={() => copyLastTranscription()}>
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true"><rect x="9" y="9" width="13" height="13" rx="2" stroke="currentColor" stroke-width="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" stroke="currentColor" stroke-width="2"/></svg>
              </button>
              <!-- Mode switcher -->
              <div class="mode-wrap">
                <button type="button" class="icon-btn" title="Switch mode" aria-expanded={modeMenuOpen} on:click|stopPropagation={() => (modeMenuOpen = !modeMenuOpen)}>
                  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h16" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
                </button>
                {#if modeMenuOpen}
                  <div class="mode-dropdown" role="menu">
                    {#each modesList as m (m.id)}
                      <button type="button" role="menuitem" on:click|stopPropagation={() => setActiveModeFromOverlay(m.id)}>{m.name}</button>
                    {/each}
                  </div>
                {/if}
              </div>
              <!-- Expand to full panel -->
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
          data-overlay-menu-root
          style="left: {contextMenuX}px; top: {contextMenuY}px;"
          role="menu"
        >
          <button type="button" role="menuitem" on:click|stopPropagation={() => { contextMenuOpen = false; modeMenuOpen = true }}>Switch mode…</button>
          <button type="button" role="menuitem" on:click|stopPropagation={() => copyLastTranscription()}>Copy last</button>
          <button type="button" role="menuitem" on:click|stopPropagation={() => invoke('ui_toggle_dictation')}>Start recording</button>
          <button type="button" role="menuitem" on:click|stopPropagation={() => expandToFullChrome()}>Expand panel</button>
        </div>
      {/if}

    <!-- ═══════════════ LISTENING ═══════════════ -->
    {:else if state.kind === 'Listening'}
      <div class="content listening" class:sensitive={isSensitiveApp}>
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
      <div class="content listening sensitive">
        <svg class="lock-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M7 11V8a5 5 0 0 1 10 0v3M6 11h12a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span class="label sensitive-label">Local only</span>
      </div>

    <!-- ═══════════════ SHORT PRESS ═══════════════ -->
    {:else if state.kind === 'ShortPress'}
      <div class="content hint">
        <span class="label">Hold the key longer to start dictating</span>
      </div>

    <!-- ═══════════════ STATUS ═══════════════ -->
    {:else if state.kind === 'Status'}
      <div class="content status-message">
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
            <span class="full-mode-title">{isVoiceEdit ? 'Editing' : activeModeName}{polishEnabledOverlay ? ' · Clean up' : ''}</span>
            <button
              type="button"
              class="polish-chip"
              aria-pressed={polishEnabledOverlay}
              data-overlay-menu-root
              on:click|stopPropagation={() => togglePolishFromOverlay()}
            >
              Clean up {polishEnabledOverlay ? 'on' : 'off'}
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
            <div class="ctx-summary">Context: {fullContextSummary}</div>
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
          <div class="full-footer" data-overlay-menu-root>
            <div class="mode-wrap">
              <button type="button" class="action-chip" aria-expanded={modeMenuOpen} on:click|stopPropagation={() => (modeMenuOpen = !modeMenuOpen)}>{idleModeLabel} <span class="chevron" aria-hidden="true">{modeMenuOpen ? '▴' : '▾'}</span></button>
              {#if modeMenuOpen}
                <div class="mode-dropdown" role="menu">
                  {#each modesList as m (m.id)}
                    <button type="button" role="menuitem" on:click|stopPropagation={() => setActiveModeFromOverlay(m.id)}>{m.name}</button>
                  {/each}
                </div>
              {/if}
            </div>
            <!-- Stop recording -->
            <button type="button" class="icon-btn icon-stop" title="Stop recording" on:click|stopPropagation={() => invoke('ui_toggle_dictation')}>
              <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor" aria-hidden="true"><rect x="1" y="1" width="10" height="10" rx="2"/></svg>
            </button>
            <!-- Collapse to compact pill -->
            <button type="button" class="icon-btn" title="Collapse to pill" on:click|stopPropagation={() => collapseToDormant()}>
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M4 14h6v6M20 10h-6V4M14 10l7-7M10 14l-7 7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </button>
          </div>
        </div>
      {:else}
        <!-- Mini recording -->
        <div class="content waveform mini-rec-row">
          <span class="status-dot rec-dot" aria-hidden="true" />
          <span class="mode-tag" title={isVoiceEdit ? 'Voice-activated editing' : 'Active dictation mode'}>{isVoiceEdit ? 'Edit' : activeModeName}{polishEnabledOverlay ? ' · P' : ''}</span>
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
          <button type="button" class="icon-btn icon-stop ml-auto" title="Stop recording" on:click|stopPropagation={() => invoke('ui_toggle_dictation')}>
            <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor" aria-hidden="true"><rect x="1" y="1" width="10" height="10" rx="2"/></svg>
          </button>
        </div>
      {/if}

    <!-- ═══════════════ PROCESSING ═══════════════ -->
    {:else if state.kind === 'Processing'}
      {#if overlayLayoutTier === 'Full'}
        <div class="full-panel proc-full">
          <div class="full-header">
            <span class="full-mode-title">{isProcessingVoiceEdit ? 'Editing' : activeModeName}</span>
            <button type="button" class="polish-chip" aria-pressed={polishEnabledOverlay} data-overlay-menu-root on:click|stopPropagation={() => togglePolishFromOverlay()}>Clean up {polishEnabledOverlay ? 'on' : 'off'}</button>
          </div>
          {#if isSensitiveApp}
            <div class="local-banner">
              <svg class="lock-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                <path d="M7 11V8a5 5 0 0 1 10 0v3M6 11h12a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <span>Local only</span>
            </div>
          {:else if fullContextSummary}
            <div class="ctx-summary">Context: {fullContextSummary}</div>
          {/if}
          <div class="content processing-body full-proc-body">
            <div class="spinner" aria-hidden="true" />
            <span class="proc-label" class:is-long={processingPastHalfExpected && processingAttempt === 1} class:is-retry={processingAttempt > 1}>
              {processingLabel}
              {#if showElapsedCounter}
                <span class="elapsed">{processingElapsed}s</span>
              {/if}
            </span>
            <button type="button" class="icon-btn icon-cancel" title="Cancel transcription" on:click|stopPropagation={cancelTranscription}>
              <svg width="12" height="12" viewBox="0 0 18 18" fill="none" aria-hidden="true"><path d="M5 5l8 8M13 5l-8 8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
            </button>
          </div>
          <div class="full-footer" data-overlay-menu-root>
            <div class="mode-wrap">
              <button type="button" class="action-chip" aria-expanded={modeMenuOpen} on:click|stopPropagation={() => (modeMenuOpen = !modeMenuOpen)}>{idleModeLabel} <span class="chevron" aria-hidden="true">{modeMenuOpen ? '▴' : '▾'}</span></button>
              {#if modeMenuOpen}
                <div class="mode-dropdown" role="menu">
                  {#each modesList as m (m.id)}
                    <button type="button" role="menuitem" on:click|stopPropagation={() => setActiveModeFromOverlay(m.id)}>{m.name}</button>
                  {/each}
                </div>
              {/if}
            </div>
            <button type="button" class="icon-btn" title="Collapse to pill" on:click|stopPropagation={() => collapseToDormant()}>
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M4 14h6v6M20 10h-6V4M14 10l7-7M10 14l-7 7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </button>
          </div>
        </div>
      {:else}
        <!-- Mini processing -->
        <div class="content processing-body mini-proc-row">
          <div class="spinner" aria-hidden="true" />
          <span class="proc-label" class:is-long={processingPastHalfExpected && processingAttempt === 1} class:is-retry={processingAttempt > 1}>
            {processingLabel}
            {#if showElapsedCounter}
              <span class="elapsed">{processingElapsed}s</span>
            {/if}
          </span>
          <button type="button" class="icon-btn icon-cancel ml-auto" title="Cancel transcription" on:click|stopPropagation={cancelTranscription}>
            <svg width="12" height="12" viewBox="0 0 18 18" fill="none" aria-hidden="true"><path d="M5 5l8 8M13 5l-8 8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
          </button>
        </div>
      {/if}

    <!-- ═══════════════ CANCELLING ═══════════════ -->
    {:else if state.kind === 'Cancelling'}
      <div class="content status-message">
        <span class="label">Cancelling…</span>
      </div>

    <!-- ═══════════════ SUCCESS ═══════════════ -->
    {:else if state.kind === 'Success'}
      <div class="content success-state">
        <svg width="18" height="18" viewBox="0 0 20 20" fill="none" aria-hidden="true">
          <path d="M4 10.5L8 14.5L16 6.5" stroke="var(--ov-success)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span class="label success-label">Done</span>
      </div>

    <!-- ═══════════════ ERROR ═══════════════ -->
    {:else if state.kind === 'Error'}
      <div class="content error-state" role="alert">
        <svg width="14" height="14" viewBox="0 0 18 18" fill="none" class="error-icon" aria-hidden="true">
          <path d="M5 5L13 13M13 5L5 13" stroke="var(--ov-error)" stroke-width="2.5" stroke-linecap="round"/>
        </svg>
        <span class="label error-text">{friendlyError(state.message || 'Something went wrong')}</span>
        <div class="error-actions">
          <button type="button" class="action-chip action-retry" on:click={retryAfterError}>Retry</button>
          <button type="button" class="action-chip" on:click={dismissOverlayMessage}>Dismiss</button>
        </div>
      </div>
    {/if}
  </div>
</div>
{/if}

<style>
  /* ── Design tokens ── */
  :root {
    --ov-surface: oklch(13% 0.01 250);
    --ov-surface-raised: oklch(17% 0.012 250);
    --ov-border: oklch(25% 0.015 250);
    --ov-border-subtle: oklch(20% 0.01 250);
    --ov-text: oklch(90% 0.008 250);
    --ov-text-secondary: oklch(65% 0.01 250);
    --ov-text-muted: oklch(50% 0.008 250);
    --ov-accent: oklch(68% 0.1 240);
    --ov-command: oklch(68% 0.13 15);
    --ov-voice-edit: oklch(65% 0.1 290);
    --ov-sensitive: oklch(72% 0.12 75);
    --ov-success: oklch(72% 0.14 155);
    --ov-error: oklch(65% 0.16 25);
    --ov-error-soft: oklch(65% 0.08 25);
    --ease-out-quart: cubic-bezier(0.25, 1, 0.5, 1);
    --ease-out-expo: cubic-bezier(0.16, 1, 0.3, 1);
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

  /* ── Toast ── */
  .toast {
    position: absolute;
    top: 8px;
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
    animation: fade-in 150ms var(--ease-out-quart) both;
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
      opacity 200ms var(--ease-out-quart),
      box-shadow 300ms var(--ease-out-quart);
    overflow: hidden;
    position: relative;
    flex-shrink: 0;
  }

  /* ── Dormant tier ── */
  .pill.tier-dormant {
    width: auto;
    min-width: 48px;
    max-width: 48px;
    min-height: 28px;
    height: 28px;
    border-radius: 14px;
    opacity: 0.7;
    flex-direction: column;
    padding: 0;
    cursor: default;
    transition: max-width 250ms var(--ease-out-quart), opacity 200ms var(--ease-out-quart);
    overflow: hidden;
  }

  .pill.tier-dormant.always-visible { opacity: 0.9; }
  .pill.tier-dormant:hover { opacity: 1; }

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
    transition: gap 200ms var(--ease-out-quart), padding 200ms var(--ease-out-quart);
  }

  .dormant-bar.expanded {
    gap: 8px;
    padding: 4px 6px 4px 12px;
  }

  /* Hidden content when dormant */
  .dormant-content {
    display: flex;
    align-items: center;
    gap: 8px;
    opacity: 0;
    width: 0;
    overflow: hidden;
    transition: opacity 150ms var(--ease-out-quart), width 200ms var(--ease-out-quart);
  }

  .dormant-bar.expanded .dormant-content {
    opacity: 1;
    width: auto;
  }

  .dormant-mode {
    font-size: 13px;
    font-weight: 550;
    color: var(--ov-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 120px;
  }

  .dormant-hotkey {
    font-size: 11px;
    font-weight: 450;
    color: var(--ov-text-muted);
    white-space: nowrap;
    flex-shrink: 0;
  }

  /* Icon row pinned to the right of the dormant bar */
  .dormant-icons {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
    flex-shrink: 0;
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
    border: 1px solid var(--ov-border);
    background: var(--ov-surface-raised);
    color: var(--ov-text-muted);
    cursor: pointer;
    flex-shrink: 0;
    transition: color 100ms, border-color 100ms, background 100ms;
  }
  .icon-btn:hover {
    color: var(--ov-text);
    background: color-mix(in oklch, var(--ov-text) 8%, var(--ov-surface));
  }
  .icon-btn:focus-visible {
    outline: 2px solid var(--ov-accent);
    outline-offset: 1px;
  }
  .icon-stop:hover {
    color: var(--ov-error);
    border-color: color-mix(in oklch, var(--ov-error) 40%, transparent);
  }
  .icon-cancel:hover {
    color: var(--ov-error);
    border-color: color-mix(in oklch, var(--ov-error) 40%, transparent);
  }
  .ml-auto { margin-left: auto; }

  /* ── Status dots ── */
  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .dormant-dot {
    background: var(--ov-accent);
    flex-shrink: 0;
    margin: 0 auto;
    transition: margin 200ms var(--ease-out-quart);
  }

  .dormant-bar.expanded .dormant-dot {
    margin: 0;
  }
  .pill.is-command .dormant-dot { background: var(--ov-command); }
  .pill.sensitive-app .dormant-dot { background: var(--ov-sensitive); }

  .listening-dot {
    background: var(--ov-accent);
    animation: gentle-pulse 1.8s ease-in-out infinite;
  }

  .rec-dot {
    background: var(--ov-success);
  }
  .pill.is-command .rec-dot { background: var(--ov-command); }
  .pill.is-voice-edit .rec-dot { background: var(--ov-voice-edit); }
  .pill.sensitive-app .rec-dot { background: var(--ov-sensitive); }

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
    transition: background 100ms, color 100ms;
  }
  .action-chip:hover {
    background: color-mix(in oklch, var(--ov-text) 8%, var(--ov-surface));
    color: var(--ov-text);
  }
  .action-chip:focus-visible {
    outline: 2px solid var(--ov-accent);
    outline-offset: 1px;
  }

  .action-retry {
    border-color: color-mix(in oklch, var(--ov-error) 35%, transparent);
    color: var(--ov-error);
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
    background: var(--ov-surface-raised);
    border: 1px solid var(--ov-border);
    z-index: 20;
    display: flex;
    flex-direction: column;
    gap: 1px;
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
  }
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
    background: var(--ov-surface-raised);
    border: 1px solid var(--ov-border);
    display: flex;
    flex-direction: column;
    gap: 1px;
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
  }
  .ctx-menu button:hover {
    background: color-mix(in oklch, var(--ov-text) 8%, var(--ov-surface));
  }

  /* ── Full panel ── */
  .full-panel {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
    min-height: 0;
    flex: 1;
  }

  .full-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .full-mode-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--ov-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .full-sub {
    font-size: 12px;
    color: var(--ov-text-muted);
    margin: 0;
    text-align: center;
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
    border: 1px solid var(--ov-border);
    background: var(--ov-surface-raised);
    color: var(--ov-text-secondary);
    cursor: pointer;
    flex-shrink: 0;
  }
  .polish-chip[aria-pressed="true"] {
    border-color: color-mix(in oklch, var(--ov-accent) 35%, transparent);
    color: var(--ov-accent);
  }

  /* Context summary — source names only, not content */
  .ctx-summary {
    font-size: 11px;
    color: var(--ov-text-muted);
    padding: 4px 6px;
    border-radius: 6px;
    background: color-mix(in oklch, var(--ov-surface) 50%, black);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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

  .full-wave {
    flex: 1;
    min-height: 50px;
    max-height: 65px;
  }

  .full-proc-body {
    flex: 1;
    min-height: 44px;
  }

  .full-footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: 6px;
    margin-top: auto;
    padding-top: 4px;
  }

  /* ── Mini rows ── */
  .mini-rec-row,
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

  .mode-tag {
    font-size: 11px;
    font-weight: 550;
    color: var(--ov-text-secondary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  /* ── Content wrapper ── */
  .content {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    animation: fade-in 150ms var(--ease-out-quart) both;
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
    animation: fade-in 200ms var(--ease-out-quart) both;
  }
  .success-label {
    color: var(--ov-success) !important;
    font-weight: 550;
  }

  /* ── Error ── */
  .error-state {
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    max-width: min(280px, 92vw);
    padding: 4px 10px;
  }
  .error-text {
    white-space: normal !important;
    max-width: 200px;
    color: color-mix(in oklch, var(--ov-error) 80%, var(--ov-text)) !important;
    flex: 1 1 auto;
    min-width: 0;
    font-size: 12px !important;
  }
  .error-icon {
    flex-shrink: 0;
  }
  .error-actions {
    display: flex;
    align-items: center;
    gap: 6px;
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
    width: 14px;
    height: 14px;
    border: 2px solid var(--ov-border);
    border-top-color: var(--ov-accent);
    border-radius: 50%;
    animation: spin 800ms linear infinite;
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
    -webkit-mask-image: radial-gradient(50% 50% at 50% 50%, black 60%, transparent 100%);
    mask-image: radial-gradient(50% 50% at 50% 50%, black 60%, transparent 100%);
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
  @keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes gentle-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.45; }
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* ── Reduced motion ── */
  @media (prefers-reduced-motion: reduce) {
    .listening-dot { animation: none; opacity: 0.8; }
    .spinner { animation-duration: 2s; }
    .toast { animation: none; }
    .content { animation: none; }
    .success-state { animation: none; }
  }
</style>
