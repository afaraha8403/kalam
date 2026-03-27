<script lang="ts">
  /**
   * Illustrative frequency sketch (not a lab-grade plot): shows high-pass knee and relative level.
   * Helps users understand how the audio cleanup chain roughly shapes the signal before transcription.
   */
  import { afterUpdate } from 'svelte'
  import type { AudioFilterConfig } from '../types'

  export let filter: AudioFilterConfig

  let canvas: HTMLCanvasElement

  const F_MIN = 40
  const F_MAX = 16000

  function log10(x: number) {
    return Math.log(x) / Math.LN10
  }

  function draw() {
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const w = canvas.width
    const h = canvas.height
    const padL = 36
    const padR = 8
    const padT = 12
    const padB = 22
    const plotW = w - padL - padR
    const plotH = h - padT - padB

    ctx.clearRect(0, 0, w, h)
    ctx.fillStyle = 'var(--bg-elevated, rgba(255,255,255,0.04))'
    ctx.fillRect(padL, padT, plotW, plotH)

    // Grid + frequency labels (log axis)
    ctx.strokeStyle = 'var(--border-light, rgba(255,255,255,0.12))'
    ctx.lineWidth = 1
    ctx.font = '10px system-ui, sans-serif'
    ctx.fillStyle = 'var(--text-muted, #888)'
    const ticks = [100, 500, 1000, 4000, 10000]
    for (const f of ticks) {
      const t = (log10(f) - log10(F_MIN)) / (log10(F_MAX) - log10(F_MIN))
      const x = padL + t * plotW
      ctx.beginPath()
      ctx.moveTo(x, padT)
      ctx.lineTo(x, padT + plotH)
      ctx.stroke()
      ctx.fillText(f >= 1000 ? `${f / 1000}k` : `${f}`, x - 10, padT + plotH + 14)
    }

    ctx.fillText('Relative level (illustration)', padL, 10)

    if (!filter.enabled || filter.preset === 'Off') {
      ctx.fillStyle = 'var(--text-muted, #888)'
      ctx.fillText('Filter off — flat response', padL + 6, padT + plotH / 2)
      return
    }

    const fc = Math.max(40, Math.min(200, filter.highpass_cutoff_hz))
    // Rough high-pass magnitude sketch (dB), normalized to display
    const pts: { x: number; y: number }[] = []
    const steps = 48
    for (let i = 0; i <= steps; i++) {
      const t = i / steps
      const f = F_MIN * Math.pow(F_MAX / F_MIN, t)
      let db = 0
      if (f < fc) {
        const ratio = Math.log2(fc / Math.max(f, 1e-6))
        db = -6 * Math.min(ratio, 4)
      }
      const ng = filter.noise_gate_threshold_db
      const noiseFloor = -50
      if (db < noiseFloor - 5) {
        db += (ng + 45) * 0.15
      }
      const x = padL + t * plotW
      const yNorm = (-db) / 30
      const y = padT + plotH * (0.15 + 0.7 * Math.min(1, Math.max(0, yNorm)))
      pts.push({ x, y })
    }

    ctx.strokeStyle = 'var(--accent, #3b82f6)'
    ctx.lineWidth = 2
    ctx.beginPath()
    ctx.moveTo(pts[0].x, pts[0].y)
    for (let i = 1; i < pts.length; i++) ctx.lineTo(pts[i].x, pts[i].y)
    ctx.stroke()

    ctx.fillStyle = 'var(--text-secondary, #aaa)'
    ctx.font = '11px system-ui, sans-serif'
    ctx.fillText(`Bass roll-off near ${Math.round(fc)} Hz`, padL + 6, padT + 14)
  }

  afterUpdate(() => draw())
</script>

<canvas bind:this={canvas} width="320" height="150" class="filter-preview-canvas" aria-hidden="true"></canvas>

<style>
  .filter-preview-canvas {
    width: 100%;
    max-width: 320px;
    height: auto;
    border-radius: var(--radius-md, 8px);
    border: 1px solid var(--border-light, rgba(255, 255, 255, 0.1));
    display: block;
    margin-top: var(--space-sm, 8px);
  }
</style>
