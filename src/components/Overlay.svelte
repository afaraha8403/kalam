<script lang="ts">
  import { onMount } from 'svelte'
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'

  const KINDS = ['Collapsed', 'Listening', 'ShortPress', 'Recording', 'Processing', 'Success', 'Error'] as const
  type OverlayEvent =
    | { kind: 'Collapsed' }
    | { kind: 'Listening' }
    | { kind: 'ShortPress' }
    | { kind: 'Recording'; level: number }
    | { kind: 'Processing' }
    | { kind: 'Success' }
    | { kind: 'Error'; message: string }

  let state: OverlayEvent = { kind: 'Collapsed' }
  let prevLevel = 0
  let smoothLevel = 0

  function isValidPayload(p: unknown): p is OverlayEvent {
    if (!p || typeof p !== 'object') return false
    const k = (p as { kind?: string }).kind
    if (typeof k !== 'string' || !KINDS.includes(k as typeof KINDS[number])) return false
    if (k === 'Recording') {
      const rec = p as { level?: unknown }
      if (rec.level !== undefined && typeof rec.level !== 'number') return false
    }
    return true
  }

  $: rawLevel = state.kind === 'Recording' ? Number(state.level) || 0 : 0
  $: {
    const r = Math.min(1, Math.max(0, rawLevel))
    smoothLevel = r > prevLevel ? r * 0.6 + prevLevel * 0.4 : r * 0.3 + prevLevel * 0.7
    prevLevel = smoothLevel
  }

  $: isExpanded = state.kind !== 'Collapsed'

  // Rolling history of mic levels for the live wave (last N samples)
  const WAVE_POINTS = 52
  let levelHistory: number[] = []

  $: if (state.kind === 'Recording') {
    levelHistory = [...levelHistory, smoothLevel].slice(-WAVE_POINTS)
  } else if (state.kind !== 'Recording' && levelHistory.length > 0) {
    levelHistory = []
  }

  // Build SVG polyline points: x 0..WAVE_POINTS-1, y = center - level * amplitude
  $: wavePoints = (() => {
    const centerY = 12
    const amplitude = 10
    const pad = Math.max(0, WAVE_POINTS - levelHistory.length)
    const padded = [...Array(pad).fill(0), ...levelHistory].slice(-WAVE_POINTS)
    return padded.map((l, i) => `${i},${centerY - l * amplitude}`).join(' ')
  })()

  onMount(() => {
    let unlisten: (() => void) | null = null
    getCurrentWebviewWindow().listen<OverlayEvent>('overlay-state', (e) => {
      const p = e?.payload
      if (isValidPayload(p)) state = p
    }).then((fn) => {
      unlisten = fn
    })
    return () => {
      unlisten?.()
    }
  })
</script>

<div class="blip-root">
  <div
    class="blip"
    class:collapsed={!isExpanded}
    class:expanded={isExpanded}
    class:recording={state.kind === 'Recording'}
    class:processing={state.kind === 'Processing'}
    class:success={state.kind === 'Success'}
    class:error={state.kind === 'Error'}
    data-tauri-drag-region
  >
    {#if state.kind === 'Collapsed'}
      <!-- idle: just the pill shape itself -->
    {:else if state.kind === 'Listening'}
      <div class="content listening">
        <div class="listen-dot" />
        <span class="label">Listening</span>
      </div>
    {:else if state.kind === 'ShortPress'}
      <div class="content hint">
        <span class="label">Hold longer to dictate</span>
      </div>
    {:else if state.kind === 'Recording'}
      <div class="content waveform">
        <svg class="wave-svg" viewBox="0 0 52 24" preserveAspectRatio="none">
          <defs>
            <linearGradient id="wave-grad" x1="0%" y1="0%" x2="100%" y2="0%">
              <stop offset="0%" stop-color="#4fc1ff" stop-opacity="0.4" />
              <stop offset="50%" stop-color="#4fc1ff" stop-opacity="1" />
              <stop offset="100%" stop-color="#4fc1ff" stop-opacity="0.4" />
            </linearGradient>
            <filter id="wave-glow" x="-20%" y="-20%" width="140%" height="140%">
              <feGaussianBlur in="SourceGraphic" stdDeviation="0.8" result="blur" />
              <feMerge>
                <feMergeNode in="blur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
          </defs>
          <polyline
            class="wave-line"
            points={wavePoints}
            fill="none"
            stroke="url(#wave-grad)"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
            filter="url(#wave-glow)"
          />
        </svg>
      </div>
    {:else if state.kind === 'Processing'}
      <div class="content processing-anim">
        <div class="dot-pulse">
          <span /><span /><span />
        </div>
      </div>
    {:else if state.kind === 'Success'}
      <div class="content status-icon success-icon">
        <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
          <path d="M4 10.5L8 14.5L16 6.5" stroke="#4ade80" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </div>
    {:else if state.kind === 'Error'}
      <div class="content status-icon error-icon">
        <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
          <path d="M5 5L13 13M13 5L5 13" stroke="#f87171" stroke-width="2.5" stroke-linecap="round"/>
        </svg>
      </div>
    {/if}
  </div>
</div>

<style>
  /* Overlay window only: force full transparency, no box/border */
  :global(html),
  :global(body),
  :global(#app) {
    background: transparent !important;
    border: none !important;
    outline: none !important;
  }

  /* Pill’s parent: full overlay area, transparent, centers the pill at bottom */
  .blip-root {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding-bottom: 8px;
    background: transparent;
  }

  /* The pill (blue in your screenshot): animates width/height, contains wave/dots/error */
  .blip {
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 100px;
    background: #0a0a0c;
    box-sizing: border-box;
    will-change: width, height, opacity;
    transition:
      width 0.35s cubic-bezier(0.34, 1.56, 0.64, 1),
      height 0.35s cubic-bezier(0.34, 1.56, 0.64, 1),
      box-shadow 0.4s ease,
      opacity 0.3s ease;
    overflow: hidden;
    position: relative;
    flex-shrink: 0;
  }

  .blip.collapsed {
    width: 48px;
    min-width: 48px;
    height: 5px;
    min-height: 5px;
    opacity: 0.7;
    box-shadow: 0 0 6px rgba(79, 193, 255, 0.15);
    animation: idle-breathe 3s ease-in-out infinite;
  }

  .blip.expanded {
    width: 200px;
    min-width: 200px;
    height: 48px;
    min-height: 48px;
    opacity: 1;
    box-shadow: 0 2px 20px rgba(0, 0, 0, 0.6);
  }

  .blip.recording {
    box-shadow:
      0 2px 20px rgba(0, 0, 0, 0.6),
      0 0 16px rgba(79, 193, 255, 0.15);
  }

  .blip.success {
    box-shadow:
      0 2px 20px rgba(0, 0, 0, 0.6),
      0 0 12px rgba(74, 222, 128, 0.2);
  }

  .blip.error {
    box-shadow:
      0 2px 20px rgba(0, 0, 0, 0.6),
      0 0 12px rgba(248, 113, 113, 0.2);
  }

  /* ── Content wrapper (inside the pill) ── */
  .content {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    animation: content-in 0.25s ease-out both;
    width: auto;
    height: auto;
    max-width: 100%;
    max-height: 100%;
    flex-shrink: 1;
  }

  /* ── Listening ── */
  .listening .listen-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #4fc1ff;
    animation: dot-blink 1.2s ease-in-out infinite;
  }

  .label {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    font-size: 13px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8);
    letter-spacing: 0.02em;
    white-space: nowrap;
  }

  .hint .label {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.55);
  }

  /* ── Live wave: single continuous line driven by mic level ── */
  .waveform {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 28px;
    padding: 0 16px;
  }

  .wave-svg {
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  .wave-line {
    vector-effect: non-scaling-stroke;
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

  /* ── Status icons ── */
  .status-icon {
    animation: pop-in 0.3s cubic-bezier(0.34, 1.56, 0.64, 1) both;
  }

  /* ── Keyframes ── */
  @keyframes idle-breathe {
    0%, 100% { opacity: 0.5; }
    50% { opacity: 0.8; }
  }

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
