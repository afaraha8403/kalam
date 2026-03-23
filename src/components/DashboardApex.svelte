<script lang="ts">
  import { onDestroy } from 'svelte'
  import ApexCharts from 'apexcharts'
  import type { ApexOptions } from 'apexcharts'

  /** ApexCharts options; parent passes a new object when data or theme should refresh. */
  export let options: ApexOptions

  let container: HTMLDivElement
  let chart: ApexCharts | null = null
  /** Detect real data/theme changes without relying on object identity (avoids destroy/recreate noise). */
  let lastFingerprint = ''
  let resizeCleanup: (() => void) | null = null
  let resizeFrame: number | null = null
  let syncGen = 0

  function fingerprint(opts: ApexOptions): string {
    try {
      return JSON.stringify({
        type: opts.chart?.type,
        height: opts.chart?.height,
        width: opts.chart?.width,
        theme: opts.theme,
        series: opts.series,
        categories: opts.xaxis?.categories,
        colors: opts.colors,
        labels: opts.labels,
      })
    } catch {
      return `${Date.now()}`
    }
  }

  function scheduleResize() {
    if (!chart) return
    if (resizeFrame != null) cancelAnimationFrame(resizeFrame)
    resizeFrame = requestAnimationFrame(() => {
      resizeFrame = null
      try {
        chart?.resize()
      } catch {
        /* WebView resize races */
      }
    })
  }

  function ensureResizeWatch() {
    if (!container || typeof ResizeObserver === 'undefined' || resizeCleanup) return
    const ro = new ResizeObserver(() => scheduleResize())
    ro.observe(container)
    window.addEventListener('resize', scheduleResize)
    resizeCleanup = () => {
      window.removeEventListener('resize', scheduleResize)
      ro.disconnect()
    }
  }

  async function syncChart() {
    const gen = ++syncGen
    if (typeof window === 'undefined' || !container || !options) return

    await Promise.resolve()
    if (gen !== syncGen) return

    const fp = fingerprint(options)
    ensureResizeWatch()

    try {
      if (!chart) {
        chart = new ApexCharts(container, options)
        await chart.render()
        lastFingerprint = fp
        scheduleResize()
        return
      }
      if (fp !== lastFingerprint) {
        // redrawPaths true so series/axes stay in sync after data or theme changes
        await chart.updateOptions(options, true, true, true)
        lastFingerprint = fp
      }
      scheduleResize()
    } catch (e) {
      console.error('Dashboard chart sync failed:', e)
      try {
        chart?.destroy()
      } catch {
        /* ignore */
      }
      chart = null
      lastFingerprint = ''
      chart = new ApexCharts(container, options)
      await chart.render()
      lastFingerprint = fingerprint(options)
      scheduleResize()
    }
  }

  // One microtask deferral batches Svelte layout + bind:this before first paint.
  $: if (container && options) {
    queueMicrotask(() => void syncChart())
  }

  onDestroy(() => {
    if (resizeFrame != null) cancelAnimationFrame(resizeFrame)
    resizeCleanup?.()
    resizeCleanup = null
    try {
      chart?.destroy()
    } catch {
      /* ignore */
    }
    chart = null
    lastFingerprint = ''
  })
</script>

<div bind:this={container} class="dashboard-apex-host"></div>

<style>
  .dashboard-apex-host {
    width: 100%;
    max-width: 100%;
    min-width: 0;
    min-height: inherit;
    box-sizing: border-box;
  }
</style>
