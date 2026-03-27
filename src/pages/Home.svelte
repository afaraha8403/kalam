<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import type { ApexOptions } from 'apexcharts'
  import { invoke, listenSafe } from '$lib/backend'
  import Icon from '@iconify/svelte'
  import DashboardApex from '../components/DashboardApex.svelte'
  import { selectedTaskId } from '../lib/taskDetailStore'
  import { selectedNoteId } from '../lib/noteDetailStore'
  import { noteDetailReturnTo, taskDetailReturnTo } from '../lib/detailReturnStore'
  import { selectedHistoryId } from '../lib/historyDetailStore'
  import { recognitionDisplay, sttChipKind } from '../lib/historySttChip'
  import { historyLanguageLabel } from '../lib/languages'
  import type { HistoryEntry, DashboardStats, Entry } from '../types'

  export let navigate: (page: string) => void = () => {}
  /** Matches App shell theme so ApexCharts (incl. tooltips) use the correct palette. */
  export let darkMode = true

  let unlistenTranscription: (() => void) | null = null

  let dashboard: DashboardStats | null = null
  let dashboardLoading = true

  // Tasks and reminders due today
  let tasksDueToday: Entry[] = []
  let remindersDueToday: Entry[] = []
  let tasksLoadError = ''
  let remindersLoadError = ''
  let todayIso = ''

  // History State (recent only on home; full list on History page)
  const RECENT_HISTORY_LIMIT = 5
  let entries: HistoryEntry[] = []
  let loading = true

  function getTodayIso(): string {
    const d = new Date()
    return d.getFullYear() + '-' + String(d.getMonth() + 1).padStart(2, '0') + '-' + String(d.getDate()).padStart(2, '0')
  }

  /** Local calendar day as UTC instants for the backend range query (matches Reminders "today" in any timezone). */
  function getLocalDayBoundsIso(): { dayStart: string; dayEnd: string; todayIso: string } {
    const d = new Date()
    const todayIso =
      d.getFullYear() + '-' + String(d.getMonth() + 1).padStart(2, '0') + '-' + String(d.getDate()).padStart(2, '0')
    const start = new Date(d.getFullYear(), d.getMonth(), d.getDate(), 0, 0, 0, 0)
    const end = new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1, 0, 0, 0, 0)
    return { dayStart: start.toISOString(), dayEnd: end.toISOString(), todayIso }
  }

  async function loadDashboard() {
    dashboardLoading = true
    try {
      dashboard = (await invoke('get_dashboard_stats')) as DashboardStats
    } catch (e) {
      console.error('Failed to load dashboard stats:', e)
    } finally {
      dashboardLoading = false
    }
  }

  async function loadTasksAndRemindersToday() {
    const { dayStart, dayEnd, todayIso: calToday } = getLocalDayBoundsIso()
    todayIso = calToday
    tasksLoadError = ''
    remindersLoadError = ''
    // Tauri maps JSON keys to Rust parameter names; these commands take one param named `args`, so the payload must be { args: { ...fields } }.
    const payload = { args: { dayStart, dayEnd, limit: 10 } }
    try {
      const result = await invoke('get_tasks_due_on', payload)
      tasksDueToday = Array.isArray(result) ? (result as Entry[]) : []
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      tasksLoadError = msg
      console.error('Failed to load tasks:', e)
    }
    try {
      const result = await invoke('get_reminders_due_on', payload)
      remindersDueToday = Array.isArray(result) ? (result as Entry[]) : []
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      remindersLoadError = msg
      console.error('Failed to load reminders:', e)
    }
  }

  // Refetch tasks/reminders when date may have changed so browser and Tauri stay in sync.
  // - visibilitychange: browser tab becomes visible (e.g. tab left open overnight).
  // - focus: Tauri window gains focus; desktop webviews often don't fire visibilitychange,
  //   and we refetch here so returning to the window shows latest tasks/reminders.
  function refetchIfDateChanged() {
    const newToday = getTodayIso()
    if (newToday !== todayIso) {
      todayIso = newToday
      loadTasksAndRemindersToday()
    }
  }

  function onVisibilityChange() {
    if (typeof document === 'undefined' || document.visibilityState !== 'visible') return
    refetchIfDateChanged()
    // Overview charts: refresh only on mount (returning to Overview remounts Home) or transcription-saved — not on every focus/visibility (avoids flicker).
  }

  function onWindowFocus() {
    refetchIfDateChanged()
    loadTasksAndRemindersToday()
  }

  onMount(async () => {
    todayIso = getTodayIso()
    await loadDashboard()
    await loadTasksAndRemindersToday()
    await loadHistory()
    document.addEventListener('visibilitychange', onVisibilityChange)
    window.addEventListener('focus', onWindowFocus)
    unlistenTranscription = await listenSafe('transcription-saved', () => {
      loadDashboard()
      loadHistory()
    })
  })

  onDestroy(() => {
    document.removeEventListener('visibilitychange', onVisibilityChange)
    window.removeEventListener('focus', onWindowFocus)
    unlistenTranscription?.()
  })

  /** Backend may send mode "cloud"; normalize so chip gets .dictation/.command and colored styling. */
  function isDictationMode(mode: string | undefined): boolean {
    if (!mode) return true
    const m = mode.toLowerCase()
    return m === 'dictation' || m === 'cloud'
  }
  function isCommandMode(mode: string | undefined): boolean {
    return mode?.toLowerCase() === 'command'
  }
  /** Display label for mode chip (prototype uses "DICTATION" / "COMMAND"). */
  function modeLabel(mode: string | undefined): string {
    if (isCommandMode(mode)) return 'command'
    return 'dictation'
  }
  function openHistoryEntry(entry: HistoryEntry) {
    selectedHistoryId.set(entry.id)
    navigate('history-detail')
  }

  function onRecentHistoryKeydown(e: KeyboardEvent, entry: HistoryEntry) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      openHistoryEntry(entry)
    }
  }
  async function loadHistory() {
    loading = true
    try {
      entries = await invoke('get_history', { limit: RECENT_HISTORY_LIMIT, offset: 0 })
    } catch (e) {
      console.error('Failed to load history:', e)
    } finally {
      loading = false
    }
  }

  function formatTime(date: string): string {
    return new Date(date).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })
  }

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text)
  }

  /** Open the specific task in full-page task detail (same as Tasks list). */
  function openTask(task: Entry) {
    taskDetailReturnTo.set(null)
    selectedTaskId.set(task.id)
    navigate('task-detail')
  }

  /** Open the specific note/task in the right detail page (same as Reminders list). */
  function openReminderEntry(entry: Entry) {
    if (entry.entry_type === 'note') {
      noteDetailReturnTo.set(null)
      selectedNoteId.set(entry.id)
      navigate('note-detail')
    } else if (entry.entry_type === 'task') {
      taskDetailReturnTo.set(null)
      selectedTaskId.set(entry.id)
      navigate('task-detail')
    } else {
      navigate('reminders')
    }
  }

  /** Title or content preview for dashboard lines (same idea as Reminders list). */
  function entryPrimaryLine(e: Entry, fallback: string): string {
    return e.title?.trim() || e.content?.trim() || fallback
  }

  /** Priority dot color — prototype uses Apple-style green/orange/red. */
  function getPriorityColor(p: number | null | undefined): string {
    if (p == null || p < 1) return 'var(--primary)'
    return ['#34C759', '#FF9500', '#FF3B30'][p - 1] ?? 'var(--primary)'
  }

  /** Greeting for prototype-style header (time-based). */
  $: greeting = (() => {
    const hour = new Date().getHours()
    if (hour < 12) return 'Good morning'
    if (hour < 17) return 'Good afternoon'
    return 'Good evening'
  })()

  $: wordsWeekTotal = dashboard?.words_dictated_7d.reduce((s, d) => s + d.words, 0) ?? 0

  /** Typing-speed baseline for "time you would have spent typing" (minutes). */
  const TYPING_WPM_BASELINE = 40

  /** Whole hours + minutes for the weekly time-saved KPI. */
  function formatHoursMinutes(totalMinutes: number): string {
    if (!Number.isFinite(totalMinutes) || totalMinutes <= 0) return '0<span class="kpi-unit">min</span>'
    const rounded = Math.round(totalMinutes)
    const h = Math.floor(rounded / 60)
    const m = rounded % 60
    if (h === 0) return `${m}<span class="kpi-unit">min</span>`
    if (m === 0) return `${h}<span class="kpi-unit">h</span>`
    return `${h}<span class="kpi-unit">h</span> ${m}<span class="kpi-unit">min</span>`
  }

  /**
   * Per-day estimated minutes saved vs typing at TYPING_WPM_BASELINE, using one weekly dictation WPM
   * from total words and total dictation time (when time > 0). If that pace is unknown, saved stays 0
   * so the chart still renders words + a flat zero line (stable UI).
   */
  $: valueMetrics = ((): {
    dictationWpm: number | null
    savedPerDay: number[]
    weeklySavedMinutes: number
  } => {
    if (!dashboard) {
      return { dictationWpm: null, savedPerDay: [], weeklySavedMinutes: 0 }
    }
    const rows = dashboard.words_dictated_7d
    const weekWords = rows.reduce((s, d) => s + d.words, 0)
    const dictationWpm =
      dashboard.total_time_dictating_7d_ms > 0 && weekWords > 0
        ? weekWords / (dashboard.total_time_dictating_7d_ms / 60000)
        : null
    let weeklySaved = 0
    const savedPerDay = rows.map((d) => {
      const typingMin = d.words / TYPING_WPM_BASELINE
      if (dictationWpm != null && dictationWpm > 0) {
        const dictationMin = d.words / dictationWpm
        const saved = Math.max(0, typingMin - dictationMin)
        weeklySaved += saved
        return saved
      }
      return 0
    })
    return { dictationWpm, savedPerDay, weeklySavedMinutes: weeklySaved }
  })()

  function shortenAppLabel(name: string): string {
    const n = name.replace(/\.exe$/i, '').replace(/\.app$/i, '')
    return n.length > 18 ? `${n.slice(0, 16)}…` : n
  }


  function generateHeatmapSeries(flow: { created_at: string }[]) {
    // 7 days (Sun-Sat), 4 blocks
    const days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']
    const matrix = Array.from({ length: 7 }, () => new Array(4).fill(0))
    
    for (const p of flow) {
      const d = new Date(p.created_at)
      const h = d.getHours()
      let block = 0 // Night (0-5)
      if (h >= 6 && h < 12) block = 1 // Morning (6-11)
      else if (h >= 12 && h < 18) block = 2 // Afternoon (12-17)
      else if (h >= 18) block = 3 // Evening (18-23)
      
      matrix[d.getDay()][block]++
    }
    
    const blockNames = ['Night', 'Morning', 'Afternoon', 'Evening']
    return days.map((day, dayIdx) => ({
      name: day,
      data: blockNames.map((b, i) => ({
        x: b,
        y: matrix[dayIdx][i]
      }))
    })).reverse() // Reverse so Sun is at bottom or top depending on preference
  }

  /** Local wall-clock span for each aggregated block (must stay in sync with `generateHeatmapSeries`). */
  function periodLocalHourRange(period: string): string {
    switch (period) {
      case 'Night':
        return '12:00 AM – 5:59 AM'
      case 'Morning':
        return '6:00 AM – 11:59 AM'
      case 'Afternoon':
        return '12:00 PM – 5:59 PM'
      case 'Evening':
        return '6:00 PM – 11:59 PM'
      default:
        return ''
    }
  }

  $: speakingTimeHeatmapOptions = ((): ApexOptions | null => {
    if (!dashboard) return null
    const series = generateHeatmapSeries(dashboard.dictation_flow_7d)
    // Softer grid lines on dark vs light so structure stays visible without heavy contrast.
    const gridStroke = darkMode ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.06)'
    // Opaque hex stops only: Apex heatmaps often flatten `transparent` + low-alpha rgba() into one fill (WebView/SVG).
    const heatRanges = darkMode
      ? [
          { from: 0, to: 0, color: '#2a2d34', name: 'none' },
          { from: 1, to: 2, color: '#334155', name: 'low' },
          { from: 3, to: 5, color: '#3d4f6f', name: 'mid' },
          { from: 6, to: 10, color: '#4c6290', name: 'high' },
          { from: 11, to: 999, color: '#5b74ab', name: 'max' },
        ]
      : [
          { from: 0, to: 0, color: '#f4f4f5', name: 'none' },
          { from: 1, to: 2, color: '#E8ECFE', name: 'low' },
          { from: 3, to: 5, color: '#D8DFFC', name: 'mid' },
          { from: 6, to: 10, color: '#C4CFFA', name: 'high' },
          { from: 11, to: 999, color: '#A8B8F6', name: 'max' },
        ]
    return {
      theme: { mode: darkMode ? 'dark' : 'light' },
      chart: {
        type: 'heatmap',
        height: 240,
        width: '100%',
        toolbar: { show: false },
        background: 'transparent',
        fontFamily: "'Google Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
        animations: { enabled: false },
      },
      series,
      plotOptions: {
        heatmap: {
          // Enough intensity that each bucket reads as a distinct fill (too low → one flat color).
          shadeIntensity: 0.55,
          radius: 0,
          useFillColorAsStroke: false,
          enableShades: true,
          colorScale: { ranges: heatRanges },
        },
      },
      dataLabels: { enabled: false },
      stroke: { width: 1, colors: [gridStroke] },
      legend: { show: false },
      xaxis: {
        labels: { style: { fontSize: '11px', fontWeight: 500, colors: 'var(--text-secondary)' } },
        tooltip: { enabled: false },
        axisBorder: { show: false },
        axisTicks: { show: false },
      },
      yaxis: {
        labels: { style: { fontSize: '11px', fontWeight: 500, colors: 'var(--text-secondary)' } },
        axisBorder: { show: false },
        axisTicks: { show: false },
      },
      grid: { show: false, padding: { right: 10 } },
      tooltip: {
        fillSeriesColor: false,
        cssClass: 'kalam-dash-tooltip',
        custom: ({ seriesIndex, dataPointIndex, w }: { seriesIndex: number; dataPointIndex: number; w: unknown }) => {
          const cfg = w as {
            config: { series: { name: string; data: { x: string; y: number }[] }[] }
          }
          const row = cfg.config.series[seriesIndex]
          const cell = row?.data?.[dataPointIndex]
          if (!row || cell == null) return ''
          const day = row.name
          const period = String(cell.x)
          const count = cell.y
          const range = periodLocalHourRange(period)
          const cap = count === 1 ? 'capture' : 'captures'
          // Custom body only; outer tooltip chrome + theme colors come from `kalam-dash-tooltip` in app.css.
          return `<div class="kalam-heatmap-tip"><div class="kalam-heatmap-tip-day">${day}</div><div class="kalam-heatmap-tip-meta">${period} · ${range}</div><div class="kalam-heatmap-tip-count">${count} ${cap}</div></div>`
        },
      },
    }
  })()

  function formatRelativeDate(dateStr: string): string {
    // If it's a YYYY-MM-DD string, parse it as local time to avoid timezone shifts
    const isDateOnly = /^\d{4}-\d{2}-\d{2}$/.test(dateStr)
    let d: Date
    if (isDateOnly) {
      const [y, m, day] = dateStr.split('-').map(Number)
      d = new Date(y, m - 1, day)
    } else {
      d = new Date(dateStr)
    }
    
    const today = new Date()
    today.setHours(0, 0, 0, 0)

    const dDate = new Date(d)
    dDate.setHours(0, 0, 0, 0)

    if (dDate.getTime() === today.getTime()) return 'Today'
    // Other days: short weekday (e.g. Mon) — same calendar semantics as backend YYYY-MM-DD rows.
    return d.toLocaleDateString(undefined, { weekday: 'short' })
  }

  /** Series colors shared by hero chart + dual y-axes so left/right scales read as paired with each curve. */
  const HERO_WORDS_COLOR = '#AEC6CF'
  const HERO_TIME_SAVED_COLOR = '#FF9500' // Orange for time saved

  /** Sparkline uses a different accent color (e.g. a nice blue) for its peak values. */
  const SPARKLINE_MAX_COLOR = '#007AFF'

  function hexToRgb(hex: string): { r: number; g: number; b: number } {
    const h = hex.replace('#', '').slice(0, 6)
    return {
      r: parseInt(h.slice(0, 2), 16),
      g: parseInt(h.slice(2, 4), 16),
      b: parseInt(h.slice(4, 6), 16),
    }
  }

  /** Per-bar fills: low counts stay near `HERO_WORDS_COLOR`, highest day in the week reaches the sparkline max color. */
  function wordsSparklineBarColors(values: number[]): string[] {
    const maxW = Math.max(1, ...values)
    const lo = hexToRgb(HERO_WORDS_COLOR)
    const hi = hexToRgb(SPARKLINE_MAX_COLOR)
    return values.map((w) => {
      const t = Math.min(1, w / maxW)
      const r = Math.round(lo.r + (hi.r - lo.r) * t)
      const g = Math.round(lo.g + (hi.g - lo.g) * t)
      const b = Math.round(lo.b + (hi.b - lo.b) * t)
      return `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`
    })
  }

  /** Hero: words (area) + estimated minutes saved (area); dual y-axis with axis colors matching series. */
  $: valueHeroChartOptions = ((): ApexOptions | null => {
    if (!dashboard) return null
    const rows = dashboard.words_dictated_7d
    const { savedPerDay } = valueMetrics
    const gridLine = darkMode ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.08)'
    return {
      theme: { mode: darkMode ? 'dark' : 'light' },
      chart: {
        type: 'line',
        height: 268,
        width: '100%',
        toolbar: { show: false },
        zoom: { enabled: false },
        background: 'transparent',
        fontFamily: "'Google Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
        animations: { easing: 'easeinout', speed: 420 },
        stacked: false,
      },
      series: [
        {
          name: 'Words dictated',
          type: 'area',
          data: rows.map((x) => x.words),
        },
        {
          name: 'Est. time saved',
          type: 'area',
          data: savedPerDay,
        },
      ],
      stroke: { curve: 'smooth', width: [2, 2] },
      fill: {
        type: 'gradient',
        gradient: {
          shadeIntensity: 1,
          opacityFrom: 0.42,
          opacityTo: 0.06,
          stops: [0, 90, 100],
        },
      },
      colors: [HERO_WORDS_COLOR, HERO_TIME_SAVED_COLOR],
      dataLabels: { enabled: false },
      // No point markers — dual series are read from the filled areas and axes.
      markers: {
        size: 0,
        strokeWidth: 0,
        hover: { sizeOffset: 0 },
      },
      states: {
        hover: { filter: { type: 'lighten', value: 0.12 } },
        active: { filter: { type: 'none' } },
      },
      legend: { show: false },
      xaxis: {
        categories: rows.map((x) => formatRelativeDate(x.date)),
        labels: { style: { fontSize: '11px', fontWeight: 500, colors: 'var(--text-secondary)' } },
        axisBorder: { show: false },
        axisTicks: { show: false },
      },
      yaxis: [
        {
          seriesName: 'Words dictated',
          min: 0,
          decimalsInFloat: 0,
          title: {
            text: 'Words',
            style: { fontSize: '11px', fontWeight: 600, color: HERO_WORDS_COLOR },
          },
          labels: {
            style: { colors: HERO_WORDS_COLOR, fontSize: '11px', fontWeight: 500 },
            formatter: (v: string | number) => Math.round(Number(v)).toLocaleString(),
          },
        },
        {
          opposite: true,
          seriesName: 'Est. time saved',
          min: 0,
          decimalsInFloat: 1,
          title: {
            text: 'Minutes saved (est.)',
            style: { fontSize: '11px', fontWeight: 600, color: HERO_TIME_SAVED_COLOR },
          },
          labels: {
            style: { colors: HERO_TIME_SAVED_COLOR, fontSize: '11px', fontWeight: 500 },
            formatter: (v: string | number) => Number(v).toFixed(1),
          },
        },
      ],
      grid: {
        borderColor: gridLine,
        strokeDashArray: 4,
        padding: { top: 4, right: 8, bottom: 0, left: 8 },
        xaxis: { lines: { show: false } },
        yaxis: { lines: { show: true } },
      },
      tooltip: {
        enabled: true,
        shared: true,
        intersect: false,
        fillSeriesColor: false,
        cssClass: 'kalam-dash-tooltip',
        y: {
          formatter: (val: number, opts: { seriesIndex?: number }) => {
            if (opts?.seriesIndex === 0) return `${Math.round(val).toLocaleString()} words`
            return `${Number(val).toFixed(1)} min saved (est.)`
          },
        },
      },
    } as ApexOptions
  })()

  /** Tiny 7-day word bars for the stat tile — taller chart + day labels; y/grid hidden for a sparkline-like look. */
  $: wordsSparklineOptions = ((): ApexOptions | null => {
    if (!dashboard) return null
    const rows = dashboard.words_dictated_7d
    const values = rows.map((x) => x.words)
    const categories = rows.map((x) => formatRelativeDate(x.date))
    const barColors = wordsSparklineBarColors(values)
    return {
      theme: { mode: darkMode ? 'dark' : 'light' },
      chart: {
        type: 'bar',
        height: 130,
        width: '100%',
        toolbar: { show: false },
        background: 'transparent',
        fontFamily: "'Google Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
        animations: { easing: 'easeinout', speed: 320 },
      },
      series: [{ name: 'Words', data: values }],
      colors: barColors,
      // Distributed series otherwise get a default numbered legend (1…7).
      legend: { show: false },
      plotOptions: {
        // `distributed` applies `colors[i]` to bar i so each day can reflect its relative volume.
        bar: { distributed: true, borderRadius: 2, columnWidth: '72%' },
      },
      xaxis: {
        // Same relative labels as the hero week chart (Today / weekday short); backend sends 7 days oldest→today.
        categories,
        tickPlacement: 'on',
        labels: {
          show: true,
          hideOverlappingLabels: false,
          trim: false,
          style: { fontSize: '10px', fontWeight: 500, colors: 'var(--text-secondary)' },
        },
        axisBorder: { show: false },
        axisTicks: { show: false },
      },
      yaxis: {
        labels: { show: false },
        axisBorder: { show: false },
        axisTicks: { show: false },
      },
      // Extra horizontal padding so the last category ("Today") is not clipped by Apex label layout.
      grid: { show: false, padding: { top: 4, right: 10, bottom: 0, left: 6 } },
      dataLabels: { enabled: false },
      tooltip: {
        enabled: true,
        cssClass: 'kalam-dash-tooltip',
        y: {
          formatter: (v: number) => `${Math.round(v).toLocaleString()} words`,
        },
      },
      stroke: { colors: ['transparent'] },
    } as ApexOptions
  })()

  $: appsChartOptions = ((): ApexOptions | null => {
    if (!dashboard) return null
    const apps = dashboard.top_apps_7d.filter(a => a.app !== '(unknown)')
    if (apps.length === 0) {
      return {
        theme: { mode: darkMode ? 'dark' : 'light' },
        chart: { 
          type: 'donut', 
          height: 200, 
          width: '100%', 
          toolbar: { show: false },
          background: 'transparent',
          fontFamily: "'Google Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif"
        },
        series: [1],
        labels: ['No data yet'],
        colors: ['#888'],
        tooltip: { enabled: false },
        dataLabels: { enabled: false },
        plotOptions: { pie: { donut: { size: '65%' } } }
      } as ApexOptions
    }
    const pastelColors = ['#FFB3BA', '#FFDFBA', '#FFFFBA', '#BAFFC9', '#BAE1FF', '#AEC6CF', '#CBAACB', '#F49AC2']
    return {
      theme: { mode: darkMode ? 'dark' : 'light' },
      chart: { 
        type: 'donut', 
        height: 200, 
        width: '100%', 
        toolbar: { show: false }, 
        background: 'transparent',
        fontFamily: "'Google Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
        animations: { easing: 'easeinout', speed: 400 },
      },
      series: apps.map(a => a.count),
      labels: apps.map(a => shortenAppLabel(a.app)),
      colors: pastelColors,
      dataLabels: { enabled: false },
      stroke: { show: false },
      plotOptions: {
        pie: {
          donut: {
            size: '65%',
            labels: {
              show: true,
              name: { fontSize: '11px', color: 'var(--text-secondary)' },
              value: { fontSize: '20px', fontWeight: 600, color: 'var(--text-primary)' },
              total: { show: true, label: 'Total', formatter: (w) => w.globals.seriesTotals.reduce((a: number, b: number) => a + b, 0) }
            }
          }
        }
      },
      legend: { show: false },
      tooltip: {
        // Default pie tooltip fills with the slice color; pastel fills + light text are unreadable.
        fillSeriesColor: false,
        cssClass: 'kalam-dash-tooltip',
        y: {
          formatter: (v: number, opts: { seriesIndex?: number; dataPointIndex?: number }) => {
            // Donut is one series; slice index is `dataPointIndex` (not `seriesIndex`).
            const idx =
              typeof opts.dataPointIndex === 'number' ? opts.dataPointIndex : (opts.seriesIndex ?? 0)
            const full = apps[idx]?.app ?? 'App'
            return `${v} capture${v === 1 ? '' : 's'} · ${full}`
          },
        },
      },
    } as ApexOptions
  })()

  $: averageSessionLengthSec = ((): number => {
    if (!dashboard || dashboard.session_lengths_7d_ms.length === 0) return 0
    const total = dashboard.session_lengths_7d_ms.reduce((a, b) => a + b, 0)
    return total / dashboard.session_lengths_7d_ms.length / 1000
  })()

  $: heatmapMax = dashboard ? Math.max(1, ...dashboard.activity_heatmap_14d.map((d) => d.count)) : 1
</script>

<!-- Layout and markup match Prototype home exactly. Functionality preserved: real data, loading/empty states, copy, navigate. -->
<div class="page fade-in">
  <header class="page-header">
    <h1 class="page-title">{greeting}</h1>
    <p class="page-subtitle">Here's what's happening today.</p>
    <p class="page-subnote">Overview charts use live data from your history and daily stats (not mock data).</p>
  </header>

  <div class="overview-dashboard">
    {#if dashboardLoading}
      <div class="dashboard-chart-stack">
        <div class="stat-box dash-tile wide"><span class="stat-label">Time saved</span><span class="stat-num">—</span></div>
        <div class="dashboard-tiles-row two-cols">
          <div class="stat-box dash-tile"><span class="stat-label">Top destinations</span><span class="stat-num">—</span></div>
          <div class="stat-box dash-tile"><span class="stat-label">Words this week</span><span class="stat-num">—</span></div>
        </div>
      </div>
      <div class="stat-box dash-tile wide"><span class="stat-label">Activity patterns</span></div>
      <div class="dashboard-tiles-bottom">
        <div class="stat-box dash-tile"><span class="stat-label">Typical capture</span></div>
        <div class="stat-box dash-tile"><span class="stat-label">Capture consistency</span></div>
      </div>
    {:else if dashboard && valueHeroChartOptions && wordsSparklineOptions && appsChartOptions && speakingTimeHeatmapOptions}
      <div class="dashboard-chart-stack">
        <div class="stat-box dash-tile wide hero-value-tile">
          <span class="stat-label">Time saved</span>
          <div class="hero-kpi-block">
            <span class="hero-kpi-value">{@html formatHoursMinutes(valueMetrics.weeklySavedMinutes)}</span>
            <span class="hero-kpi-caption">~ saved this week</span>
          </div>
          <div class="dash-chart-wrap hero">
            <DashboardApex options={valueHeroChartOptions} />
          </div>
        </div>
        <div class="dashboard-tiles-row two-cols">
          <div class="stat-box dash-tile">
            <span class="stat-label">Top destinations</span>
            <p class="dash-tile-sub">Share of dictation captures by app (7 days)</p>
            <div class="dash-chart-wrap apps">
              <DashboardApex options={appsChartOptions} />
            </div>
          </div>
          <div class="stat-box dash-tile words-week-tile">
            <span class="stat-label">Words this week</span>
            <span class="stat-num">{wordsWeekTotal.toLocaleString()}</span>
            <p class="dash-tile-sub words-week-pages">
              ~{(wordsWeekTotal / 250).toFixed(1)} pages
            </p>
            <p class="dash-tile-sub words-week-alltime">All time {(dashboard.total_words ?? 0).toLocaleString()} words</p>
            <div class="dash-chart-wrap words-sparkline">
              <DashboardApex options={wordsSparklineOptions} />
            </div>
          </div>
        </div>
      </div>
      <div class="stat-box dash-tile wide">
        <span class="stat-label">Activity patterns</span>
        <p class="dash-tile-sub">Captures by weekday and time of day (7 days)</p>
        <div class="dash-chart-wrap flow">
          <DashboardApex options={speakingTimeHeatmapOptions} />
        </div>
      </div>
      <div class="dashboard-tiles-bottom">
        <div class="stat-box dash-tile">
          <span class="stat-label">Typical capture length</span>
          <span class="stat-num" style="margin-top: 12px; display: block; font-size: 32px;">{averageSessionLengthSec.toFixed(1)}s</span>
          <p class="dash-tile-sub" style="margin-top: 8px;">Average session over the last 7 days</p>
          <!-- Same cohort as avg duration: timed dictations only (`duration_ms` &gt; 0). -->
          {#if dashboard.avg_words_per_dictation_7d != null}
            <p class="dash-tile-sub" style="margin-top: 4px;">
              Avg. {Math.round(dashboard.avg_words_per_dictation_7d).toLocaleString()} words per dictation
            </p>
          {/if}
        </div>
        <div class="stat-box dash-tile streak-tile">
          <span class="stat-label">Capture consistency</span>
          <span class="stat-num streak-num">{dashboard.streak_days ?? 0}</span>
          <p class="dash-tile-sub">Consecutive days with dictation (from daily totals)</p>
          <div class="heatmap-row" role="img" aria-label="Dictation activity last 14 days">
            {#each dashboard.activity_heatmap_14d as day (day.date)}
              <div
                class="heatmap-cell"
                style="opacity: {0.2 + (0.8 * day.count) / heatmapMax}; background: {day.count > 0 ? '#34C759' : 'var(--border)'};"
                title="{formatRelativeDate(day.date)}: {day.count}"
              ></div>
            {/each}
          </div>
        </div>
      </div>
    {:else}
      <div class="stat-box dash-tile wide">
        <span class="stat-label">Overview</span>
        <p class="dash-tile-sub">Charts could not be loaded. Try reopening the app.</p>
      </div>
    {/if}
  </div>

  <!-- Dashboard Grid: Recent (wide) + Tasks | Reminders -->
  <div class="dashboard-grid">
    <section class="dash-section wide">
      <div class="section-header">
        <h3>Recent</h3>
        <button type="button" class="text-btn" on:click={() => navigate('history')}>See all</button>
      </div>
      {#if loading}
        <div class="state-container">
          <Icon icon="ph:spinner-gap-duotone" />
          <p>Loading history...</p>
        </div>
      {:else if entries.length === 0}
        <div class="state-container">
          <Icon icon="ph:microphone" />
          <p>Press your hotkey and speak to create your first transcription.</p>
        </div>
      {:else}
        <!-- Recent cell: list-item + item-meta-row with chips, time, and copy button (History page uses entry-actions + icon-btn small). -->
        <div class="history-list">
          {#each entries.slice(0, 3) as entry (entry.id)}
            <div
              class="list-item"
              role="button"
              tabindex="0"
              on:click={() => openHistoryEntry(entry)}
              on:keydown={(e) => onRecentHistoryKeydown(e, entry)}
            >
              <div class="item-icon">
                <Icon icon={entry.mode === 'command' ? 'ph:terminal-window' : 'ph:quotes'} />
              </div>
              <div class="item-content">
                <p class="item-text">{entry.text}</p>
                <!-- entry-actions ensures prototype layout (gap 12px) and copy button alignment; class: forces it into DOM -->
                <div class="item-meta-row" class:entry-actions={true}>
                  <span class="chip chip-mode small" class:dictation={isDictationMode(entry.mode)} class:command={isCommandMode(entry.mode)}>{modeLabel(entry.mode)}</span>
                  <span
                    class="chip chip-stt small"
                    class:cloud={sttChipKind(entry.stt_mode, entry.stt_provider) === 'cloud'}
                    class:local={sttChipKind(entry.stt_mode, entry.stt_provider) === 'local'}
                    class:hybrid={sttChipKind(entry.stt_mode, entry.stt_provider) === 'hybrid'}
                    class:auto={sttChipKind(entry.stt_mode, entry.stt_provider) === 'auto'}
                    class:unknown={sttChipKind(entry.stt_mode, entry.stt_provider) === 'unknown'}
                  >{recognitionDisplay(entry.stt_provider, entry.stt_mode)}</span>
                  <span class="chip chip-lang small">{historyLanguageLabel(entry.language)}</span>
                  <span class="item-meta">{formatTime(entry.created_at)}</span>
                  {#if entry.duration_ms != null}
                    <span class="entry-duration">{Math.round(entry.duration_ms / 1000)}s</span>
                  {/if}
                  <button type="button" class="icon-btn small" on:click|stopPropagation={() => copyToClipboard(entry.text)} title="Copy" aria-label="Copy">
                    <Icon icon="ph:copy" />
                  </button>
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <div class="dash-columns">
      <section class="dash-section">
        <div class="section-header">
          <h3>Tasks</h3>
          <button type="button" class="text-btn" on:click={() => navigate('tasks')}>See all</button>
        </div>
        <!-- Click opens this task in full-page task detail (same as Tasks list). -->
        <div class="simple-list">
          {#if tasksLoadError}
            <p class="simple-list-empty">Couldn't load tasks: {tasksLoadError}</p>
            <button type="button" class="text-btn" on:click={() => loadTasksAndRemindersToday()}>Try again</button>
          {:else}
            {#each tasksDueToday.slice(0, 3) as task (task.id)}
              <div class="simple-item" role="button" tabindex="0" on:click={() => openTask(task)} on:keydown={(e) => e.key === 'Enter' && openTask(task)}>
                <div class="priority-dot" style="background: {getPriorityColor(task.priority)};"></div>
                <span class="simple-text">{entryPrimaryLine(task, 'Task')}</span>
              </div>
            {:else}
              <p class="simple-list-empty">No tasks due today</p>
            {/each}
          {/if}
        </div>
      </section>
      <section class="dash-section">
        <div class="section-header">
          <h3>Reminders</h3>
          <button type="button" class="text-btn" on:click={() => navigate('reminders')}>See all</button>
        </div>
        <!-- Click opens this reminder/note/task in the correct detail page (same as Reminders list). -->
        <div class="simple-list">
          {#if remindersLoadError}
            <p class="simple-list-empty">Couldn't load reminders: {remindersLoadError}</p>
            <button type="button" class="text-btn" on:click={() => loadTasksAndRemindersToday()}>Try again</button>
          {:else}
            {#each remindersDueToday.slice(0, 3) as rem (rem.id)}
              <div class="simple-item" role="button" tabindex="0" on:click={() => openReminderEntry(rem)} on:keydown={(e) => e.key === 'Enter' && openReminderEntry(rem)}>
                <Icon icon="ph:clock" class="muted-icon" />
                <span class="simple-text">{entryPrimaryLine(rem, 'Reminder')}</span>
              </div>
            {:else}
              <p class="simple-list-empty">No reminders due</p>
            {/each}
          {/if}
        </div>
      </section>
    </div>
  </div>
</div>

<!-- Styles for this view come from App.svelte (prototype-matching .kalam-sleek .page-content). -->
<style>
  .overview-dashboard {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
    margin-bottom: var(--space-3xl);
    min-width: 0;
    max-width: 100%;
  }
  /* At most two columns per row; radial needs horizontal space (was clipped in a 3-col row). */
  .dashboard-chart-stack {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
  }
  .dashboard-tiles-row.two-cols {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-lg);
    min-width: 0;
  }
  /* Grid items default to min-width:auto; chart SVG min-content width was clipping past the card */
  .dashboard-tiles-row.two-cols > :global(.stat-box) {
    min-width: 0;
    max-width: 100%;
  }
  @media (max-width: 960px) {
    .dashboard-tiles-row.two-cols {
      grid-template-columns: 1fr;
    }
  }
  .dashboard-tiles-bottom {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-lg);
    min-width: 0;
  }
  .dashboard-tiles-bottom > :global(.stat-box) {
    min-width: 0;
    max-width: 100%;
  }
  @media (max-width: 960px) {
    .dashboard-tiles-bottom {
      grid-template-columns: 1fr;
    }
  }
  :global(.kalam-sleek .page-content .dash-tile) {
    min-height: 0;
  }
  :global(.kalam-sleek .page-content .dash-tile.wide) {
    width: 100%;
  }
  .dash-tile-sub {
    margin: 0 0 4px 0;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .dash-chart-wrap {
    width: 100%;
    max-width: 100%;
    min-width: 0;
    min-height: 112px;
    flex: 1 1 auto;
  }
  .hero-value-tile .dash-tile-sub.hero-kpi-sub {
    line-height: 1.45;
    margin-bottom: 10px;
  }
  .hero-kpi-block {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 10px 14px;
    margin: 10px 0 0 0;
  }
  .hero-kpi-value {
    font-size: clamp(28px, 5vw, 36px);
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }
  :global(.hero-kpi-value .kpi-unit) {
    font-size: 0.55em;
    font-weight: 600;
    margin-left: 2px;
    margin-right: 4px;
    color: var(--text-secondary);
  }
  .hero-kpi-caption {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
  }
  .dash-chart-wrap.hero {
    min-height: 268px;
  }
  .dash-chart-wrap.flow {
    min-height: 240px;
  }
  .dash-chart-wrap.apps {
    min-height: 200px;
  }
  /* Stack label → KPI → page estimate → all-time, then chart grows to fill the tile. */
  .words-week-tile {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .words-week-tile .dash-tile-sub.words-week-pages {
    margin-top: 4px;
  }
  .words-week-tile .dash-tile-sub.words-week-alltime {
    margin-top: 2px;
    margin-bottom: 0;
  }
  .dash-chart-wrap.words-sparkline {
    min-height: 120px;
    margin-top: 8px;
    flex: 1 1 auto;
  }
  .streak-num {
    display: block;
    margin-bottom: 4px;
  }
  .heatmap-row {
    display: flex;
    gap: 4px;
    align-items: stretch;
    margin-top: 8px;
  }
  .heatmap-cell {
    flex: 1;
    min-width: 10px;
    height: 32px;
    border-radius: 4px;
    background: var(--accent);
  }

  .page-subnote {
    margin: 8px 0 0 0;
    font-size: 13px;
    color: var(--text-secondary);
    opacity: 0.9;
  }
</style>
