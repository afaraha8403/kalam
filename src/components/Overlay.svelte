<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { listen } from '@tauri-apps/api/event'
  import type { AppConfig, WaveformStyle, ExpandDirection } from '../types'

  const KINDS = ['Hidden', 'Collapsed', 'Listening', 'ShortPress', 'Recording', 'Processing', 'Success', 'Error'] as const
  type OverlayEvent =
    | { kind: 'Hidden' }
    | { kind: 'Collapsed' }
    | { kind: 'Listening' }
    | { kind: 'ShortPress' }
    | { kind: 'Recording'; level: number }
    | { kind: 'Processing' }
    | { kind: 'Success' }
    | { kind: 'Error'; message: string }

  let state: OverlayEvent = { kind: 'Hidden' }
  let prevLevel = 0
  let smoothLevel = 0
  let waveformStyle: WaveformStyle = 'Heartbeat'
  let expandDirection: ExpandDirection = 'Up'
  let hotkeyStr = ''

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

  $: isExpanded = state.kind !== 'Collapsed' && state.kind !== 'Hidden'

  // Rolling history of mic levels for the live wave
  const WAVE_POINTS = 100
  let levelHistory: number[] = []
  let currentLevel = 0
  let snakeOffset = 0
  let animationFrameId: number | null = null

  function animateWave() {
    if (state.kind !== 'Recording') {
      levelHistory = []
      snakeOffset = 0
      currentLevel = 0
      animationFrameId = null
      return
    }

    const r = Math.min(1, Math.max(0, rawLevel))
    const gain = Math.sqrt(r) * 1.4 // boost low levels
    const targetLevel = Math.min(1, gain)

    // Smooth interpolation (runs at ~60fps)
    if (targetLevel > currentLevel) {
      currentLevel += (targetLevel - currentLevel) * 0.25 // Fast attack
    } else {
      currentLevel += (targetLevel - currentLevel) * 0.08 // Smooth decay
    }

    levelHistory = [...levelHistory, currentLevel].slice(-WAVE_POINTS)
    snakeOffset += 0.15

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

  // Build SVG points/data based on the selected style
  $: waveData = (() => {
    const centerY = 12
    const pad = Math.max(0, WAVE_POINTS - levelHistory.length)
    const padded = [...Array(pad).fill(0), ...levelHistory].slice(-WAVE_POINTS)
    
    let points = ''
    let points2 = ''
    let points3 = ''
    let bars: {x: number, y: number, w: number, h: number}[] = []

    if (waveformStyle === 'Symmetric') {
      const amplitude = 10
      const topHalf = padded.map((l, i) => `${i},${centerY - l * amplitude}`).join(' ')
      const bottomHalf = padded.slice().reverse().map((l, i) => `${WAVE_POINTS - 1 - i},${centerY + l * amplitude}`).join(' ')
      points = `${topHalf} ${bottomHalf}`
    } else if (waveformStyle === 'Heartbeat') {
      const amplitude = 20
      points = padded.map((l, i) => {
        const direction = i % 2 === 0 ? 1 : -1
        const y = centerY + (l * amplitude * direction)
        return `${i},${Math.max(1, Math.min(23, y))}`
      }).join(' ')
    } else if (waveformStyle === 'Snake') {
      const amplitude = 18
      const frequency = 0.2
      points = padded.map((l, i) => {
        const phase = (i * frequency) - snakeOffset
        const y = centerY + Math.sin(phase) * (l * amplitude)
        return `${i},${Math.max(1, Math.min(23, y))}`
      }).join(' ')
    } else if (waveformStyle === 'DoubleHelix') {
      const amplitude = 15
      const frequency = 0.15
      points = padded.map((l, i) => {
        const phase = (i * frequency) - snakeOffset
        const y = centerY + Math.sin(phase) * (l * amplitude)
        return `${i},${Math.max(1, Math.min(23, y))}`
      }).join(' ')
      points2 = padded.map((l, i) => {
        const phase = (i * frequency) - snakeOffset + Math.PI
        const y = centerY + Math.sin(phase) * (l * amplitude)
        return `${i},${Math.max(1, Math.min(23, y))}`
      }).join(' ')
    } else if (waveformStyle === 'Liquid') {
      const amplitude = 15
      const frequency = 0.05
      const topEdge = padded.map((l, i) => {
        const wave = Math.sin(i * frequency - snakeOffset * 0.5) * 3
        const y = centerY + 8 - l * amplitude + wave
        return `${i},${Math.max(1, Math.min(23, y))}`
      }).join(' ')
      points = `0,24 ${topEdge} ${WAVE_POINTS - 1},24`
    } else if (waveformStyle === 'Waves') {
      const amplitude = 16
      const freq1 = 0.04
      const freq2 = 0.06
      const freq3 = 0.09
      
      const topEdge1 = padded.map((l, i) => {
        const wave = Math.sin(i * freq1 - snakeOffset * 0.8) * 3
        const y = centerY + 4 - l * amplitude + wave
        return `${i},${Math.max(1, Math.min(23, y))}`
      }).join(' ')
      
      const topEdge2 = padded.map((l, i) => {
        const wave = Math.sin(i * freq2 - snakeOffset * 1.2 + Math.PI/3) * 4
        const y = centerY + 7 - l * (amplitude * 0.8) + wave
        return `${i},${Math.max(1, Math.min(23, y))}`
      }).join(' ')

      const topEdge3 = padded.map((l, i) => {
        const wave = Math.sin(i * freq3 - snakeOffset * 1.6 + Math.PI) * 2
        const y = centerY + 10 - l * (amplitude * 0.5) + wave
        return `${i},${Math.max(1, Math.min(23, y))}`
      }).join(' ')

      points = `0,24 ${topEdge1} ${WAVE_POINTS - 1},24`
      points2 = `0,24 ${topEdge2} ${WAVE_POINTS - 1},24`
      points3 = `0,24 ${topEdge3} ${WAVE_POINTS - 1},24`
    } else if (waveformStyle === 'Glitch') {
      const amplitude = 20
      points = padded.map((l, i) => {
        const jump = Math.sin(i * 12.9898 + Math.floor(snakeOffset * 2)) * 43758.5453 % 1 > 0 ? 1 : -1
        const y = centerY + jump * l * amplitude
        return `${i},${Math.max(1, Math.min(23, y))}`
      }).join(' ')
    } else if (waveformStyle === 'CenterSplit') {
      const amplitude = 18
      const half = Math.floor(WAVE_POINTS / 2)
      const reversed = [...padded].reverse()
      const leftHalf = reversed.slice(0, half).map((l, i) => `${50 - i},${Math.max(1, Math.min(23, centerY - l * amplitude))}`)
      const rightHalf = reversed.slice(0, half).map((l, i) => `${50 + i},${Math.max(1, Math.min(23, centerY - l * amplitude))}`)
      points = [...leftHalf.reverse(), ...rightHalf].join(' ')
    } else if (waveformStyle === 'Bars') {
      const numBars = 20
      for(let i=0; i<numBars; i++) {
        const chunk = padded.slice(i * 5, (i+1) * 5)
        const avg = chunk.reduce((a,b)=>a+b,0)/5
        const h = Math.max(2, avg * 20)
        bars.push({ x: i * 5 + 1, y: 24 - h, w: 3, h: h })
      }
    } else {
      const amplitude = 18
      points = padded.map((l, i) => {
        const y = centerY - l * amplitude
        return `${i},${Math.max(1, Math.min(23, y))}`
      }).join(' ')
    }

    return { points, points2, points3, bars }
  })()

  onMount(() => {
    let unlisten: (() => void) | null = null
    let unlistenSettings: (() => void) | null = null

    // Load initial settings
    invoke('get_settings').then((config) => {
      const cfg = config as AppConfig
      if (cfg.waveform_style) {
        waveformStyle = cfg.waveform_style
      }
      if (cfg.overlay_expand_direction) {
        expandDirection = cfg.overlay_expand_direction
      }
      if (cfg.hotkey) {
        hotkeyStr = cfg.hotkey
      }
    }).catch(console.error)

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
      }
    }).then((fn) => {
      unlistenSettings = fn
    })

    getCurrentWebviewWindow().listen<OverlayEvent>('overlay-state', (e) => {
      const p = e?.payload
      if (isValidPayload(p)) state = p
    }).then((fn) => {
      unlisten = fn
    })
    return () => {
      unlisten?.()
      unlistenSettings?.()
    }
  })
</script>

{#if state.kind !== 'Hidden'}
<div class="blip-root" class:expand-up={expandDirection === 'Up'} class:expand-down={expandDirection === 'Down'} class:expand-center={expandDirection === 'Center'}>
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
      <!-- idle: just the pill shape itself, but with hover text -->
      <div class="hover-hint">
        <span class="label">Hold <span class="hotkey-highlight">{hotkeyStr}</span> to start dictating</span>
      </div>
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
        <svg class="wave-svg" viewBox="0 0 {WAVE_POINTS} 24" preserveAspectRatio="none">
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
          <g>
            {#if waveformStyle === 'DoubleHelix'}
              <polyline class="wave-line" points={waveData.points} fill="none" stroke="url(#wave-grad)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" filter="url(#wave-glow)" />
              <polyline class="wave-line" points={waveData.points2} fill="none" stroke="url(#wave-grad)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" filter="url(#wave-glow)" opacity="0.5" />
            {:else if waveformStyle === 'Liquid'}
              <polygon class="wave-line" points={waveData.points} fill="url(#wave-grad)" stroke="none" filter="url(#wave-glow)" opacity="0.8" />
            {:else if waveformStyle === 'Waves'}
              <polygon class="wave-line" points={waveData.points3} fill="url(#wave-grad)" stroke="none" filter="url(#wave-glow)" opacity="0.4" />
              <polygon class="wave-line" points={waveData.points2} fill="url(#wave-grad)" stroke="none" filter="url(#wave-glow)" opacity="0.7" />
              <polygon class="wave-line" points={waveData.points} fill="url(#wave-grad)" stroke="none" filter="url(#wave-glow)" opacity="1.0" />
            {:else if waveformStyle === 'Bars'}
              {#each waveData.bars as bar}
                <rect x={bar.x} y={bar.y} width={bar.w} height={bar.h} fill="url(#wave-grad)" rx="1.5" filter="url(#wave-glow)" />
              {/each}
            {:else}
              <polyline
                class="wave-line"
                class:filled={waveformStyle === 'Symmetric'}
                points={waveData.points}
                fill={waveformStyle === 'Symmetric' ? "url(#wave-grad)" : "none"}
                stroke={waveformStyle === 'Symmetric' ? "none" : "url(#wave-grad)"}
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
                filter="url(#wave-glow)"
              />
            {/if}
          </g>
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

  /* Pill’s parent: full overlay area, transparent, centers the pill at bottom */
  .blip-root {
    width: 100vw;
    height: 100vh;
    display: flex;
    justify-content: center;
    background: transparent;
  }

  .blip-root.expand-up {
    align-items: flex-end;
    padding-bottom: 24px;
  }

  .blip-root.expand-down {
    align-items: flex-start;
    padding-top: 24px;
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
    height: 10px;
    min-height: 10px;
    opacity: 0.7;
    box-shadow: 0 0 6px rgba(79, 193, 255, 0.15);
    animation: idle-breathe 3s ease-in-out infinite;
    border: 1px solid rgba(255, 255, 255, 0.6) !important;
    cursor: default;
    transition:
      width 0.2s cubic-bezier(0.34, 1.56, 0.64, 1),
      height 0.2s cubic-bezier(0.34, 1.56, 0.64, 1),
      box-shadow 0.2s ease,
      opacity 0.2s ease;
  }

  .blip.collapsed:hover {
    width: 250px;
    height: 36px;
    opacity: 1;
    animation: none;
  }

  .blip.expanded {
    width: 200px;
    min-width: 200px;
    height: 48px;
    min-height: 48px;
    opacity: 1;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.1);
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
    color: rgba(255, 255, 255, 0.8) !important;
    letter-spacing: 0.02em;
    white-space: nowrap;
  }

  .hint .label {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.55) !important;
  }

  .hover-hint {
    opacity: 0;
    transition: opacity 0.15s ease;
    display: flex;
    align-items: center;
    justify-content: center;
    white-space: nowrap;
  }

  .blip.collapsed:hover .hover-hint {
    opacity: 1;
    transition-delay: 0.05s;
  }

  .hotkey-highlight {
    color: #4fc1ff;
    font-weight: 600;
    padding: 0 4px;
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
