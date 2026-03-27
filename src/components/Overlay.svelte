<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { invoke } from '$lib/backend'
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { listen } from '@tauri-apps/api/event'
  import type { AppConfig, WaveformStyle, ExpandDirection } from '../types'
  import { formatHotkeyForDisplay } from '$lib/platformHotkey'

  // T6: earliest trace in overlay JS (latency debugging; only when KALAM_LATENCY_DEBUG=1)
  invoke('trace_latency', { event: 'T6', jsTimestamp: Date.now() * 1000 }).catch(() => {})

  const KINDS = ['Hidden', 'Collapsed', 'Listening', 'ShortPress', 'Recording', 'Processing', 'Success', 'Error', 'Status', 'Cancelling', 'SensitiveAppPeek'] as const
  type OverlayEvent =
    | { kind: 'Hidden' }
    | { kind: 'Collapsed' }
    | { kind: 'Listening'; sensitive_app?: boolean }
    | { kind: 'ShortPress' }
    | { kind: 'Recording'; level: number; is_command: boolean; sensitive_app?: boolean }
    | { kind: 'Processing'; elapsed_secs?: number; expected_secs?: number; attempt?: number; message?: string | null }
    | { kind: 'Success' }
    | { kind: 'Error'; message: string }
    | { kind: 'Status'; message: string; highlight?: string }
    | { kind: 'Cancelling' }
    /** Focus moved to a sensitive app while idle — same lock UI as listening; auto-collapses. */
    | { kind: 'SensitiveAppPeek' }

  let state: OverlayEvent = { kind: 'Hidden' }
  let waveformStyle: WaveformStyle = 'Aurora'
  let expandDirection: ExpandDirection = 'Up'
  let hotkeyStr = ''
  let overlayPlatform = 'windows'

  $: hotkeyDisplayStr =
    hotkeyStr.trim() !== '' ? formatHotkeyForDisplay(hotkeyStr, overlayPlatform) : ''

  function isValidPayload(p: unknown): p is OverlayEvent {
    if (!p || typeof p !== 'object') return false
    const k = (p as { kind?: string }).kind
    if (typeof k !== 'string' || !KINDS.includes(k as typeof KINDS[number])) return false
    if (k === 'Listening') {
      const li = p as { sensitive_app?: unknown }
      if (li.sensitive_app !== undefined && typeof li.sensitive_app !== 'boolean') return false
    }
    if (k === 'Recording') {
      const rec = p as { level?: unknown, is_command?: unknown, sensitive_app?: unknown }
      if (rec.level !== undefined && typeof rec.level !== 'number') return false
      if (rec.is_command !== undefined && typeof rec.is_command !== 'boolean') return false
      if (rec.sensitive_app !== undefined && typeof rec.sensitive_app !== 'boolean') return false
    }
    if (k === 'Processing') {
      const proc = p as { elapsed_secs?: unknown, expected_secs?: unknown, attempt?: unknown, message?: unknown }
      if (proc.elapsed_secs !== undefined && typeof proc.elapsed_secs !== 'number') return false
      if (proc.expected_secs !== undefined && typeof proc.expected_secs !== 'number') return false
      if (proc.attempt !== undefined && typeof proc.attempt !== 'number') return false
      if (proc.message !== undefined && proc.message !== null && typeof proc.message !== 'string') return false
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

  function dismissOverlayMessage() {
    if (statusTimeout) clearTimeout(statusTimeout)
    statusTimeout = null
    state = { kind: 'Collapsed' }
  }

  /** Audio is already gone; focus main window so the user can dictate again. */
  async function retryAfterError() {
    dismissOverlayMessage()
    try {
      await invoke('focus_main_window')
    } catch (e) {
      console.error('focus_main_window failed:', e)
    }
  }

  $: showCancelButton = state.kind === 'Processing' && (isHovered || (state.elapsed_secs ?? 0) >= 5)
  $: processingElapsed = state.kind === 'Processing' ? (state.elapsed_secs ?? 0) : 0
  $: processingExpected = state.kind === 'Processing' ? (state.expected_secs ?? 120) : 120
  $: processingAttempt = state.kind === 'Processing' ? (state.attempt ?? 1) : 1
  $: processingMessage = state.kind === 'Processing' ? (state.message ?? null) : null
  // Matches Rust progress tier: show elapsed counter and “long” styling from half of expected (min 8s).
  $: processingPastHalfExpected =
    state.kind === 'Processing' &&
    processingElapsed >= Math.max(8, Math.floor(processingExpected / 2))

  $: rawLevel = state.kind === 'Recording' ? Number(state.level) || 0 : 0
  $: isCommand = state.kind === 'Recording' ? Boolean(state.is_command) : false

  /** Hybrid/Auto forced local STT due to sensitive app patterns — amber pill + waveform. */
  $: isSensitiveApp =
    state.kind === 'SensitiveAppPeek' ||
    (state.kind === 'Listening' && Boolean(state.sensitive_app)) ||
    (state.kind === 'Recording' && Boolean(state.sensitive_app))

  $: isExpanded =
    state.kind !== 'Collapsed' && state.kind !== 'Hidden'

  let isHovered = false
  let showHoverExpansion = false
  let hoverExpandTimeout: ReturnType<typeof setTimeout> | null = null
  $: requiresLargeWindow = isExpanded || isHovered

  function onBlipMouseEnter() {
    isHovered = true
    if (hoverExpandTimeout != null) {
      clearTimeout(hoverExpandTimeout)
      hoverExpandTimeout = null
    }
    hoverExpandTimeout = setTimeout(() => {
      showHoverExpansion = true
      hoverExpandTimeout = null
    }, 60)
  }

  function onBlipMouseLeave() {
    isHovered = false
    showHoverExpansion = false
    if (hoverExpandTimeout != null) {
      clearTimeout(hoverExpandTimeout)
      hoverExpandTimeout = null
    }
  }

  let resizeTimeout: ReturnType<typeof setTimeout> | null = null
  /** Auto-collapse after Success / Status / Error; cleared on manual dismiss. */
  let statusTimeout: ReturnType<typeof setTimeout> | null = null
  $: {
    if (requiresLargeWindow) {
      if (resizeTimeout != null) {
        clearTimeout(resizeTimeout)
        resizeTimeout = null
      }
      invoke('resize_overlay', { expanded: true }).catch(console.error)
    } else {
      if (resizeTimeout != null) clearTimeout(resizeTimeout)
      resizeTimeout = setTimeout(() => {
        invoke('resize_overlay', { expanded: false }).catch(console.error)
      }, 400)
    }
  }

  // Rolling history of mic levels — mutated in rAF (no reactive SVG rebuild per frame).
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
    // Command mode (rose) > sensitive app (amber) > default (blue).
    const stroke = cmd ? '#fb7185' : isSensitiveApp ? '#f59e0b' : '#4fc1ff'
    const strokeSoft = cmd
      ? 'rgba(251, 113, 133,'
      : isSensitiveApp
        ? 'rgba(245, 158, 11,'
        : 'rgba(79, 193, 255,'

    if (waveformStyle === 'Oscilloscope') {
      ctx.beginPath()
      for (let i = 0; i < WAVE_POINTS; i++) {
        const l = padded[i] ?? 0
        const x = (i / (WAVE_POINTS - 1)) * w
        const y = h * 0.5 - l * (h * 0.5 - 2) * 0.92
        if (i === 0) ctx.moveTo(x, y)
        else ctx.lineTo(x, y)
      }
      ctx.shadowBlur = 10
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
            'rgba(251, 113, 133, 0.9)', // Rose
            'rgba(244, 63, 94, 0.8)', // Darker Rose
            'rgba(245, 158, 11, 0.8)', // Amber
            'rgba(255, 255, 255, 0.4)', // White highlight
          ]
        : isSensitiveApp
          ? [
              'rgba(245, 158, 11, 0.9)', // Amber
              'rgba(251, 191, 36, 0.8)', // Amber light
              'rgba(234, 179, 8, 0.75)', // Yellow
              'rgba(255, 255, 255, 0.4)', // White highlight
            ]
          : [
              'rgba(79, 193, 255, 0.9)', // Blue
              'rgba(16, 185, 129, 0.8)', // Emerald
              'rgba(139, 92, 246, 0.8)', // Purple
              'rgba(255, 255, 255, 0.4)', // White highlight
            ]

      for (let layer = 0; layer < 4; layer++) {
        const phase = snakeOffset * (0.5 + layer * 0.2) + layer * 2.0
        const ampMul = 0.5 + layer * 0.15
        
        ctx.beginPath()
        for (let i = 0; i < WAVE_POINTS; i++) {
          const l = padded[i] ?? 0
          const x = (i / (WAVE_POINTS - 1)) * w
          
          // Complex organic wave
          const wave1 = Math.sin(i * 0.05 + phase) * 0.5
          const wave2 = Math.sin(i * 0.1 - phase * 0.8) * 0.3
          const wave = wave1 + wave2
          
          // The volume makes the aurora spike and warp
          const yOffset = (wave * h * 0.2) + (Math.sin(i * 0.08 + phase * 1.5) * l * h * ampMul)
          const y = h * 0.5 + yOffset
          
          if (i === 0) ctx.moveTo(x, y)
          else ctx.lineTo(x, y)
        }
        
        ctx.strokeStyle = colors[layer]
        // The highlight layer (layer 3) is thinner
        ctx.lineWidth = layer === 3 ? h * 0.2 : h * 0.6 
        ctx.lineCap = 'round'
        ctx.lineJoin = 'round'
        ctx.stroke()
      }
      
      ctx.filter = 'none'
      ctx.globalCompositeOperation = 'source-over'
      return
    }

    // SiriWave — layered translucent sine ribbons driven by level history
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
      const v0 = padded[WAVE_POINTS - 1] ?? 0;
      const v1 = padded[WAVE_POINTS - 8] ?? 0;
      const v2 = padded[WAVE_POINTS - 16] ?? 0;
      
      el.style.setProperty('--echo-core', String(0.8 + v0 * 0.6));
      
      el.style.setProperty('--echo-scale1', String(0.5 + v0 * 0.8));
      el.style.setProperty('--echo-op1', String(v0 * 0.8));

      el.style.setProperty('--echo-scale2', String(0.6 + v1 * 0.8));
      el.style.setProperty('--echo-op2', String(v1 * 0.6));

      el.style.setProperty('--echo-scale3', String(0.7 + v2 * 0.8));
      el.style.setProperty('--echo-op3', String(v2 * 0.4));
      return
    }

    if (waveformStyle === 'RoundedBars') {
      // 11 bars, symmetrical. Center is index 5.
      for (let i = 0; i <= 5; i++) {
        // i=5 is center (most reactive, latest data)
        const historyIdx = WAVE_POINTS - 1 - (5 - i) * 4;
        let sum = 0;
        for (let k = 0; k < 3; k++) sum += padded[historyIdx - k] ?? 0;
        const v = sum / 3;
        const scale = 0.15 + v * 0.85;
        
        if (i === 5) {
          el.style.setProperty(`--b5`, String(scale));
        } else {
          el.style.setProperty(`--b${i}`, String(scale));
          el.style.setProperty(`--b${10 - i}`, String(scale));
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
      const v = currentLevel;
      const jitter = Math.sin(snakeOffset * 2) * 0.05 * v;
      const w = Math.max(0, v + jitter);
      el.style.setProperty('--neon-w', String(w));
      el.style.setProperty('--neon-h', String(v));
      el.style.setProperty('--neon-g', String(v));
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
    const gain = Math.pow(r, 0.8) * 1.8 // boost low levels, exaggerate peaks
    const targetLevel = Math.min(1, gain)

    if (targetLevel > currentLevel) {
      currentLevel += (targetLevel - currentLevel) * 0.45 // fast attack
    } else {
      currentLevel += (targetLevel - currentLevel) * 0.15 // fast release
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

  // WebView2 aggressively throttles JS event loops in unfocused windows, delaying IPC
  // message delivery by up to ~1 s.  A tiny Worker posting messages at 100 ms keeps the
  // main-thread event loop responsive so overlay-state events arrive promptly.
  let keepAliveWorker: Worker | null = null
  try {
    const blob = new Blob(
      [`setInterval(()=>postMessage(""),100)`],
      { type: 'application/javascript' }
    )
    keepAliveWorker = new Worker(URL.createObjectURL(blob))
    keepAliveWorker.onmessage = () => {}
  } catch { /* best-effort; overlay still works without it */ }

  onMount(() => {
    let unlisten: (() => void) | null = null
    let unlistenSettings: (() => void) | null = null
    let pendingSuccessTimer: ReturnType<typeof setTimeout> | null = null
    let retryHoldUntil: number | null = null
    let lastSeenProcessingAttempt = 1

    // Load initial settings + OS so meta key matches StatusBar (Win / Cmd / Super).
    Promise.all([
      invoke('get_settings'),
      invoke('get_platform').catch(() => 'windows'),
    ])
      .then(([config, os]) => {
        overlayPlatform = typeof os === 'string' ? os : 'windows'
        const cfg = config as AppConfig
        if (cfg.waveform_style) {
          waveformStyle = cfg.waveform_style
        }
        if (cfg.overlay_expand_direction) {
          expandDirection = cfg.overlay_expand_direction
        }
        if (cfg.hotkey) {
          hotkeyStr = cfg.hotkey
        } else if (cfg.toggle_dictation_hotkey) {
          hotkeyStr = cfg.toggle_dictation_hotkey
        }
      })
      .catch(console.error)

    // Listen for settings updates
    listen<AppConfig>('settings_updated', (e) => {
      if (e.payload?.waveform_style) {
        waveformStyle = e.payload.waveform_style
      }
      if (e.payload?.overlay_expand_direction) {
        expandDirection = e.payload.overlay_expand_direction
      }
      if (e.payload?.hotkey) {
        hotkeyStr = e.payload.hotkey
      } else if (e.payload?.toggle_dictation_hotkey) {
        hotkeyStr = e.payload.toggle_dictation_hotkey
      }
    }).then((fn) => {
      unlistenSettings = fn
    })

    // Main-window StatusBar mirrors dictation phase via Rust `overlay-state-broadcast` (see dictationState.ts).
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
        if (pendingSuccessTimer) {
          clearTimeout(pendingSuccessTimer)
          pendingSuccessTimer = null
        }
        const att = p.attempt ?? 1
        if (att > 1 && att > lastSeenProcessingAttempt) {
          retryHoldUntil = Date.now() + 1500
        }
        lastSeenProcessingAttempt = att
        state = p
        traceAfterState()
        if (statusTimeout) clearTimeout(statusTimeout)
        return
      }

      if (p.kind === 'Success') {
        const now = Date.now()
        const holdEnd = retryHoldUntil
        const wasRetrying =
          state.kind === 'Processing' && (state.attempt ?? 1) > 1
        if (holdEnd !== null && now < holdEnd && wasRetrying) {
          if (pendingSuccessTimer) clearTimeout(pendingSuccessTimer)
          pendingSuccessTimer = setTimeout(() => {
            pendingSuccessTimer = null
            retryHoldUntil = null
            lastSeenProcessingAttempt = 1
            state = { kind: 'Success' }
            traceAfterState()
            if (statusTimeout) clearTimeout(statusTimeout)
            statusTimeout = setTimeout(() => {
              state = { kind: 'Collapsed' }
            }, 3000)
          }, holdEnd - now)
          return
        }
        retryHoldUntil = null
        lastSeenProcessingAttempt = 1
        if (pendingSuccessTimer) {
          clearTimeout(pendingSuccessTimer)
          pendingSuccessTimer = null
        }
        state = p
        traceAfterState()
        if (statusTimeout) clearTimeout(statusTimeout)
        statusTimeout = setTimeout(() => {
          state = { kind: 'Collapsed' }
        }, 3000)
        return
      }

      if (pendingSuccessTimer) {
        clearTimeout(pendingSuccessTimer)
        pendingSuccessTimer = null
      }
      retryHoldUntil = null
      lastSeenProcessingAttempt = 1
      state = p
      traceAfterState()
      if (statusTimeout) clearTimeout(statusTimeout)
      if (p.kind === 'Status' || p.kind === 'Cancelling') {
        statusTimeout = setTimeout(() => {
          state = { kind: 'Collapsed' }
        }, 3000)
      } else if (p.kind === 'SensitiveAppPeek') {
        statusTimeout = setTimeout(() => {
          state = { kind: 'Collapsed' }
        }, 2600)
      } else if (p.kind === 'Error') {
        statusTimeout = setTimeout(() => {
          state = { kind: 'Collapsed' }
        }, 10000)
      }
    }).then((fn) => {
      unlisten = fn
    })

    // Startup emit can happen before we're listening; fetch initial state when we mount
    invoke<OverlayEvent>('get_overlay_initial_state')
      .then((initial) => {
        if (isValidPayload(initial)) state = initial
      })
      .catch(console.error)

    return () => {
      unlisten?.()
      unlistenSettings?.()
      if (statusTimeout) clearTimeout(statusTimeout)
      if (pendingSuccessTimer) clearTimeout(pendingSuccessTimer)
      keepAliveWorker?.terminate()
    }
  })
</script>

{#if state.kind !== 'Hidden'}
<div class="blip-root" class:expand-up={expandDirection === 'Up'} class:expand-down={expandDirection === 'Down'} class:expand-center={expandDirection === 'Center'}>
  <div
    class="blip"
    class:collapsed={!isExpanded}
    class:expanded={isExpanded}
    class:hover-expanded={showHoverExpansion}
    class:recording={state.kind === 'Recording'}
    class:processing={state.kind === 'Processing'}
    class:processing-long={state.kind === 'Processing' && processingPastHalfExpected}
    class:success={state.kind === 'Success'}
    class:error={state.kind === 'Error'}
    class:sensitive-app={isSensitiveApp}
    data-tauri-drag-region
    on:mouseenter={onBlipMouseEnter}
    on:mouseleave={onBlipMouseLeave}
  >
    {#if state.kind === 'Collapsed'}
      <!-- idle: just the pill shape itself, but with hover text -->
      <div class="hover-hint">
        <span class="label">Press <span class="hotkey-highlight">{hotkeyDisplayStr}</span> to dictate</span>
      </div>
    {:else if state.kind === 'Listening'}
      <div class="content listening" class:sensitive={isSensitiveApp}>
        {#if isSensitiveApp}
          <svg class="lock-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path
              d="M7 11V8a5 5 0 0 1 10 0v3M6 11h12a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1Z"
              stroke="currentColor"
              stroke-width="1.75"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
          <span class="label sensitive-label">Sensitive app detected</span>
        {:else}
          <div class="listen-dot" />
          <span class="label">Listening</span>
        {/if}
      </div>
    {:else if state.kind === 'SensitiveAppPeek'}
      <div class="content listening sensitive">
        <svg class="lock-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path
            d="M7 11V8a5 5 0 0 1 10 0v3M6 11h12a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1Z"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        <span class="label sensitive-label">Sensitive app detected</span>
      </div>
    {:else if state.kind === 'ShortPress'}
      <div class="content hint">
        <span class="label">Hold longer to dictate</span>
      </div>
    {:else if state.kind === 'Status'}
      <div class="content status-message">
        <span class="label">
          {state.message}
          {#if state.highlight}
            <span class="highlight-text">{state.highlight}</span>
          {/if}
        </span>
      </div>
    {:else if state.kind === 'Recording'}
      <div class="content waveform">
        {#if waveformStyle === 'SiriWave' || waveformStyle === 'Oscilloscope' || waveformStyle === 'Aurora'}
          <canvas bind:this={waveCanvas} class="wave-canvas" aria-hidden="true"></canvas>
        {:else}
          <div
            bind:this={cssVizEl}
            class="viz-css"
            class:viz-cmd={isCommand}
            class:viz-sensitive={!isCommand && isSensitiveApp}
            data-viz={waveformStyle}
          >
            {#if waveformStyle === 'EchoRing'}
              <div class="viz-echo">
                <div class="ring r3"></div>
                <div class="ring r2"></div>
                <div class="ring r1"></div>
                <div class="core"></div>
              </div>
            {:else if waveformStyle === 'RoundedBars'}
              <div class="viz-bars">
                <span></span><span></span><span></span><span></span><span></span><span></span><span></span><span></span><span></span><span></span><span></span>
              </div>
            {:else if waveformStyle === 'BreathingAura'}
              <div class="viz-aura">
                <div class="aura-orb"></div>
              </div>
            {:else if waveformStyle === 'NeonPulse'}
              <div class="viz-neon">
                <div class="beam"></div>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {:else if state.kind === 'Processing'}
      <div class="content processing-anim">
        <div class="dot-pulse">
          <span /><span /><span />
        </div>
        {#if processingMessage}
          <div class="processing-text" class:is-long={processingPastHalfExpected && processingAttempt === 1} class:is-retry={processingAttempt > 1}>
            <span class="hint-text">{processingMessage}</span>
            {#if processingPastHalfExpected || processingAttempt > 1}
              <span class="time">{processingElapsed}s</span>
            {/if}
          </div>
        {/if}
        {#if showCancelButton}
          <button type="button" class="cancel-btn" on:click={cancelTranscription} title="Cancel transcription">&#10005;</button>
        {/if}
      </div>
    {:else if state.kind === 'Cancelling'}
      <div class="content status-message cancelling-message">
        <span class="label">Cancelling...</span>
      </div>
    {:else if state.kind === 'Success'}
      <div class="content status-icon success-icon">
        <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
          <path d="M4 10.5L8 14.5L16 6.5" stroke="#4ade80" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </div>
    {:else if state.kind === 'Error'}
      <div class="content status-message error-message error-message-with-actions">
        <svg width="16" height="16" viewBox="0 0 18 18" fill="none" class="error-icon" aria-hidden="true">
          <path d="M5 5L13 13M13 5L5 13" stroke="#f87171" stroke-width="2.5" stroke-linecap="round"/>
        </svg>
        <span class="label error-text">{state.message || 'Something went wrong'}</span>
        <div class="error-actions">
          <button type="button" class="error-retry-btn" on:click={retryAfterError}>Retry</button>
          <button type="button" class="error-dismiss-btn" on:click={dismissOverlayMessage} aria-label="Dismiss">×</button>
        </div>
      </div>
    {/if}
  </div>
</div>
{/if}

<style>
  /* Overlay window only: force full transparency, no box/border */
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

  /* Pill’s parent: full overlay area, transparent, centers the pill. No border-radius so the pill is not clipped on hover. */
  .blip-root {
    width: 100vw;
    height: 100vh;
    display: flex;
    justify-content: center;
    background: transparent;
    overflow: hidden;
  }

  .blip-root.expand-up {
    align-items: flex-end;
    padding-bottom: 0;
  }

  .blip-root.expand-down {
    align-items: flex-start;
    padding-top: 0;
  }

  .blip-root.expand-center {
    align-items: center;
  }

  /* The pill (blue in your screenshot): animates width/height, contains wave/dots/error */
  .blip {
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 100px;
    background: #0a0a0c !important;
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-sizing: border-box;
    will-change: width, height, opacity;
    transition:
      width 0.4s cubic-bezier(0.16, 1, 0.3, 1),
      height 0.4s cubic-bezier(0.16, 1, 0.3, 1),
      box-shadow 0.4s ease,
      opacity 0.3s ease;
    overflow: hidden;
    position: relative;
    flex-shrink: 0;
  }

  .blip.collapsed {
    width: 48px;
    min-width: 48px;
    height: 10px;
    min-height: 10px;
    opacity: 0.7;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.6) !important;
    cursor: default;
    transition:
      width 0.3s cubic-bezier(0.16, 1, 0.3, 1),
      height 0.3s cubic-bezier(0.16, 1, 0.3, 1),
      box-shadow 0.3s ease,
      opacity 0.3s ease;
  }

  .blip.collapsed:hover {
    opacity: 1;
  }

  /* Expand pill only after short delay so overlay window has time to resize (avoids clipped text) */
  .blip.collapsed.hover-expanded {
    width: 250px;
    height: 36px;
    opacity: 1;
    animation: none;
  }

  .blip.expanded {
    width: 250px;
    min-width: 200px;
    height: 48px;
    min-height: 48px;
    opacity: 1;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  }

  .blip.recording {
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
  }

  .blip.success {
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
  }

  .blip.error {
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
  }

  /* Hybrid/Auto: sensitive app matched — warm amber frame (privacy assurance, not an error). */
  .blip.sensitive-app {
    border-color: rgba(245, 158, 11, 0.4);
    box-shadow: 0 1px 6px rgba(245, 158, 11, 0.25);
  }

  .blip.sensitive-app.recording {
    box-shadow: 0 1px 6px rgba(245, 158, 11, 0.3);
  }

  /* ── Content wrapper (inside the pill) ── */
  .content {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    animation: content-in 0.25s ease-out 0.1s both;
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
  .listening .listen-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #4fc1ff;
    animation: dot-blink 1.2s ease-in-out infinite;
  }

  .listening.sensitive .lock-icon {
    color: #fbbf24;
    flex-shrink: 0;
    filter: drop-shadow(0 0 4px rgba(245, 158, 11, 0.35));
  }

  .sensitive-label {
    color: #fbbf24 !important;
    font-weight: 600;
  }

  .label {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    font-size: 13px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8) !important;
    letter-spacing: 0.02em;
    white-space: nowrap;
  }

  .hint .label {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.55) !important;
  }

  /* ── Status Message ── */
  .status-message .label {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.9) !important;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 230px;
  }

  .highlight-text {
    color: #4ade80;
    font-weight: 600;
    margin-left: 2px;
  }

  .error-message-with-actions {
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 6px 10px;
    max-width: min(280px, 92vw);
  }

  .error-message-with-actions .error-text {
    white-space: normal;
    max-width: 200px;
    color: #fca5a5 !important;
    flex: 1 1 auto;
    min-width: 0;
  }

  .error-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .error-retry-btn {
    font-size: 12px;
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid rgba(252, 165, 165, 0.45);
    background: rgba(255, 255, 255, 0.08);
    color: #fecaca;
    cursor: pointer;
  }

  .error-dismiss-btn {
    font-size: 18px;
    line-height: 1;
    padding: 0 6px;
    border: none;
    background: transparent;
    color: #fca5a5;
    cursor: pointer;
    opacity: 0.85;
  }

  .error-dismiss-btn:hover {
    opacity: 1;
  }

  .error-icon {
    flex-shrink: 0;
    margin-top: 2px;
  }

  .hover-hint {
    opacity: 0;
    transition: opacity 0.15s ease;
    display: flex;
    align-items: center;
    justify-content: center;
    white-space: nowrap;
  }

  .blip.collapsed.hover-expanded .hover-hint {
    opacity: 1;
    transition-delay: 0.05s;
  }

  .hotkey-highlight {
    color: #4fc1ff;
    font-weight: 600;
    padding: 0 4px;
  }

  /* ── Live visualization: canvas (Siri / scope) or CSS vars (dots, bars, aura, blob) ── */
  .waveform {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    padding: 0;
    box-sizing: border-box;
    /* Soften all 4 edges so the animations seamlessly blend into the pill's background */
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

  .viz-echo {
    position: relative;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .viz-echo .core {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #4fc1ff;
    transform: scale(var(--echo-core, 1));
    will-change: transform;
  }

  .viz-echo .ring {
    position: absolute;
    border-radius: 50%;
    border: 1.5px solid #4fc1ff;
    box-sizing: border-box;
    will-change: transform, opacity;
  }

  .viz-echo .r1 { 
    width: 16px; height: 16px; 
    transform: scale(var(--echo-scale1, 0.5)); 
    opacity: var(--echo-op1, 0); 
  }
  .viz-echo .r2 { 
    width: 24px; height: 24px; 
    transform: scale(var(--echo-scale2, 0.5)); 
    opacity: var(--echo-op2, 0); 
  }
  .viz-echo .r3 { 
    width: 34px; height: 34px; 
    transform: scale(var(--echo-scale3, 0.5)); 
    opacity: var(--echo-op3, 0); 
  }

  .viz-cmd .viz-echo .core { background: #fb7185; }
  .viz-cmd .viz-echo .ring { border-color: #fb7185; }

  .viz-sensitive .viz-echo .core { background: #f59e0b; }
  .viz-sensitive .viz-echo .ring { border-color: #f59e0b; }

  .viz-bars {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 3px;
    height: 28px;
    width: 100%;
  }

  .viz-bars span {
    width: 4px;
    height: 24px;
    border-radius: 4px;
    background: #4fc1ff;
    box-shadow: 0 0 6px rgba(79, 193, 255, 0.4);
    transform-origin: center;
    transform: scaleY(var(--b0, 0.15));
    will-change: transform;
  }

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

  .viz-cmd .viz-bars span {
    background: #fb7185;
    box-shadow: 0 0 6px rgba(251, 113, 133, 0.4);
  }

  .viz-sensitive .viz-bars span {
    background: #f59e0b;
    box-shadow: 0 0 6px rgba(245, 158, 11, 0.45);
  }

  .viz-aura {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
  }

  .aura-orb {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: radial-gradient(circle at 40% 40%, rgba(120, 210, 255, 0.95), rgba(79, 193, 255, 0.4) 60%, transparent 80%);
    transform: scale(var(--aura-scale, 0.5));
    box-shadow: 0 0 calc(var(--aura-glow, 4) * 1px) rgba(79, 193, 255, 0.6);
    will-change: transform, box-shadow;
  }

  .viz-cmd .aura-orb {
    background: radial-gradient(circle at 40% 40%, rgba(255, 170, 190, 0.95), rgba(251, 113, 133, 0.4) 60%, transparent 80%);
    box-shadow: 0 0 calc(var(--aura-glow, 4) * 1px) rgba(251, 113, 133, 0.6);
  }

  .viz-sensitive .aura-orb {
    background: radial-gradient(circle at 40% 40%, rgba(253, 224, 171, 0.95), rgba(245, 158, 11, 0.45) 60%, transparent 80%);
    box-shadow: 0 0 calc(var(--aura-glow, 4) * 1px) rgba(245, 158, 11, 0.55);
  }

  .viz-neon {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .viz-neon .beam {
    width: calc(40px + var(--neon-w, 0) * 100px);
    height: calc(2px + var(--neon-h, 0) * 8px);
    background: #4fc1ff;
    border-radius: 10px;
    box-shadow: 0 0 calc(5px + var(--neon-g, 0) * 15px) #4fc1ff,
                0 0 calc(10px + var(--neon-g, 0) * 30px) rgba(79, 193, 255, 0.5);
    will-change: width, height, box-shadow;
  }

  .viz-cmd .viz-neon .beam {
    background: #fb7185;
    box-shadow: 0 0 calc(5px + var(--neon-g, 0) * 15px) #fb7185,
                0 0 calc(10px + var(--neon-g, 0) * 30px) rgba(251, 113, 133, 0.5);
  }

  .viz-sensitive .viz-neon .beam {
    background: #f59e0b;
    box-shadow: 0 0 calc(5px + var(--neon-g, 0) * 15px) #f59e0b,
                0 0 calc(10px + var(--neon-g, 0) * 30px) rgba(245, 158, 11, 0.5);
  }

  /* ── Processing dots ── */
  .dot-pulse {
    display: flex;
    gap: 6px;
  }

  .dot-pulse span {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: rgba(79, 193, 255, 0.8);
    animation: pulse-dot 1.2s ease-in-out infinite;
  }

  .dot-pulse span:nth-child(2) {
    animation-delay: 0.15s;
  }

  .dot-pulse span:nth-child(3) {
    animation-delay: 0.3s;
  }

  /* ── Processing: progress hint and cancel ── */
  .content.processing-anim {
    width: 100%;
    padding: 0 12px;
    box-sizing: border-box;
  }

  .processing-anim {
    display: flex;
    align-items: center;
    height: 100%;
    gap: 8px;
  }

  .processing-text {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.7);
    min-width: 0;
    flex-shrink: 1;
  }

  .processing-text .hint-text {
    white-space: nowrap;
  }

  .processing-text.is-long {
    color: #fbbf24;
  }

  .processing-text.is-retry {
    color: #f87171;
  }

  .processing-text .time {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
    font-variant-numeric: tabular-nums;
    background: rgba(255, 255, 255, 0.08);
    padding: 1px 5px;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .processing-text.is-long .time {
    background: rgba(251, 191, 36, 0.15);
    color: #fbbf24;
  }

  .processing-text.is-retry .time {
    background: rgba(248, 113, 113, 0.15);
    color: #f87171;
  }

  .cancel-btn {
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: none;
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.5);
    font-size: 11px;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
    margin-left: auto;
  }

  .blip:hover .cancel-btn,
  .blip.processing-long .cancel-btn {
    opacity: 1;
  }

  .cancel-btn:hover {
    background: rgba(248, 113, 113, 0.3);
    color: #f87171;
  }

  .cancelling-message .label {
    color: rgba(255, 255, 255, 0.8);
  }

  /* ── Status icons ── */
  .status-icon {
    animation: pop-in 0.3s cubic-bezier(0.34, 1.56, 0.64, 1) both;
  }

  /* ── Keyframes ── */
  @keyframes content-in {
    from {
      opacity: 0;
      transform: scale(0.9);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  @keyframes dot-blink {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.3; transform: scale(0.8); }
  }

  @keyframes pulse-dot {
    0%, 80%, 100% {
      opacity: 0.3;
      transform: scale(0.8);
    }
    40% {
      opacity: 1;
      transform: scale(1);
    }
  }

  @keyframes pop-in {
    from {
      opacity: 0;
      transform: scale(0.5);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
