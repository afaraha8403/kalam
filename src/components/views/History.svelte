<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '$lib/backend'
  import { listen } from '@tauri-apps/api/event'
  import Icon from '@iconify/svelte'
  import type { HistoryEntry } from '../../types'
  import { selectedHistoryId } from '../../lib/historyDetailStore'
  import { recognitionDisplay, sttChipKind } from '../../lib/historySttChip'

  export let navigate: (page: string) => void = () => {}

  let entries: HistoryEntry[] = []
  let loading = true
  let error: string | null = null
  let searchQuery = ''
  /** True after a successful non-search load with at least one row — keeps "Clear all" visible while search returns no matches. */
  let historyExists = false
  let clearing = false
  /** In-app confirm (native `window.confirm` is unreliable in Tauri / WebView2). */
  let clearConfirmOpen = false
  let clearConfirmError: string | null = null

  function openClearConfirm() {
    clearConfirmError = null
    clearConfirmOpen = true
  }

  function closeClearConfirm() {
    if (clearing) return
    clearConfirmOpen = false
    clearConfirmError = null
  }

  function onClearDialogKeydown(e: KeyboardEvent) {
    if (!clearConfirmOpen || clearing) return
    if (e.key === 'Escape') {
      e.preventDefault()
      closeClearConfirm()
    }
  }

  /** Timeline order within each day (matches Notes / Tasks sort control pattern). */
  type HistorySortMode = 'newest' | 'oldest'
  let historySortMode: HistorySortMode = 'newest'
  const HISTORY_SORT_LABELS: Record<HistorySortMode, string> = {
    newest: 'Newest first',
    oldest: 'Oldest first'
  }
  const HISTORY_SORT_ICONS: Record<HistorySortMode, string> = {
    newest: 'ph:arrow-down',
    oldest: 'ph:arrow-up'
  }
  function cycleHistorySort() {
    historySortMode = historySortMode === 'newest' ? 'oldest' : 'newest'
  }
  $: historySortLabel = HISTORY_SORT_LABELS[historySortMode]
  $: historySortIcon = HISTORY_SORT_ICONS[historySortMode]

  async function load() {
    loading = true
    error = null
    try {
      if (searchQuery.trim()) {
        entries = (await invoke('search_history', { query: searchQuery })) as HistoryEntry[]
      } else {
        entries = (await invoke('get_history', { limit: 100, offset: 0 })) as HistoryEntry[]
        historyExists = entries.length > 0
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    } finally {
      loading = false
    }
  }

  async function runClearHistory() {
    clearing = true
    clearConfirmError = null
    error = null
    try {
      await invoke('clear_history')
      historyExists = false
      selectedHistoryId.set(null)
      clearConfirmOpen = false
      await load()
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      clearConfirmError = msg
      error = msg
    } finally {
      clearing = false
    }
  }

  $: showClearAll =
    !loading && (entries.length > 0 || (searchQuery.trim() !== '' && historyExists))

  onMount(() => {
    load()
    let unlisten: (() => void) | null = null
    listen('transcription-saved', () => {
      load()
    }).then((fn) => { unlisten = fn })
    return () => { unlisten?.() }
  })

  let searchTimeout: ReturnType<typeof setTimeout>
  function handleSearch() {
    clearTimeout(searchTimeout)
    searchTimeout = setTimeout(() => {
      load()
    }, 300)
  }

  function formatTime(iso: string) {
    return new Date(iso).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })
  }

  /**
   * Split transcript text for search highlights. Matches backend search_history (case-insensitive substring).
   * Not semantic search — vector search is unused for History today.
   */
  function searchHighlightSegments(
    text: string,
    query: string
  ): { text: string; hl: boolean }[] {
    const q = query.trim()
    if (!q) return [{ text, hl: false }]
    const escaped = q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    const re = new RegExp(escaped, 'gi')
    const segments: { text: string; hl: boolean }[] = []
    let lastIndex = 0
    let m: RegExpExecArray | null
    while ((m = re.exec(text)) !== null) {
      const match = m[0]
      const start = m.index
      if (start > lastIndex) segments.push({ text: text.slice(lastIndex, start), hl: false })
      segments.push({ text: match, hl: true })
      lastIndex = start + match.length
      // Avoid stuck lastIndex on empty match (should not happen for non-empty q).
      if (match.length === 0) {
        re.lastIndex++
        if (re.lastIndex > text.length) break
      }
    }
    if (lastIndex < text.length) segments.push({ text: text.slice(lastIndex), hl: false })
    return segments.length > 0 ? segments : [{ text, hl: false }]
  }

  /** Which row just copied — drives check icon + button animation. */
  let copiedEntryId: string | null = null
  let copyResetTimer: ReturnType<typeof setTimeout> | undefined

  async function copyText(entryId: string, text: string) {
    try {
      await navigator.clipboard.writeText(text)
    } catch {
      return
    }
    copiedEntryId = entryId
    if (copyResetTimer !== undefined) clearTimeout(copyResetTimer)
    copyResetTimer = setTimeout(() => {
      copiedEntryId = null
      copyResetTimer = undefined
    }, 1600)
  }

  /** Backend may send mode "cloud"; normalize so chip gets .dictation/.command and colored styling. */
  function isDictationMode(mode: string | undefined): boolean {
    if (!mode) return true
    return mode.toLowerCase() === 'dictation' || mode.toLowerCase() === 'cloud'
  }
  function isCommandMode(mode: string | undefined): boolean {
    return mode?.toLowerCase() === 'command'
  }
  function modeLabel(mode: string | undefined): string {
    return isCommandMode(mode) ? 'command' : 'dictation'
  }
  function openHistoryDetail(entry: HistoryEntry) {
    selectedHistoryId.set(entry.id)
    navigate('history-detail')
  }

  function onHistoryRowKeydown(e: KeyboardEvent, entry: HistoryEntry) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      openHistoryDetail(entry)
    }
  }

  $: sortedHistoryEntries = [...entries].sort((a, b) => {
    const ta = new Date(a.created_at).getTime()
    const tb = new Date(b.created_at).getTime()
    return historySortMode === 'newest' ? tb - ta : ta - tb
  })

  /** Group entries by day (Today, Yesterday, or date) — prototype structure. */
  $: groupedHistory = sortedHistoryEntries.reduce((acc, entry) => {
    const date = new Date(entry.created_at)
    const today = new Date()
    const yesterday = new Date(today)
    yesterday.setDate(yesterday.getDate() - 1)
    let dateLabel = ''
    let dateSub = ''
    if (date.toDateString() === today.toDateString()) {
      dateLabel = 'Today'
      dateSub = date.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric' })
    } else if (date.toDateString() === yesterday.toDateString()) {
      dateLabel = 'Yesterday'
      dateSub = date.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric' })
    } else {
      dateLabel = date.toLocaleDateString(undefined, { month: 'long', day: 'numeric' })
      dateSub = date.toLocaleDateString(undefined, { weekday: 'long', year: 'numeric' })
    }
    if (!acc[dateLabel]) acc[dateLabel] = { entries: [], sub: dateSub }
    acc[dateLabel].entries.push(entry)
    return acc
  }, {} as Record<string, { entries: HistoryEntry[]; sub: string }>)
</script>

<svelte:window on:keydown={onClearDialogKeydown} />

<!-- Prototype structure: page, page-header, search-bar, timeline, day-group, entry-row -->
<div class="page fade-in">
  <header class="page-header notes-header">
    <div>
      <h1 class="page-title">History</h1>
    </div>
    {#if showClearAll}
      <button
        type="button"
        class="btn-danger-outline"
        disabled={clearing}
        title="Delete every dictation from history"
        aria-label="Clear all history"
        on:click={openClearConfirm}
      >
        <Icon icon="ph:trash" />
        Clear all
      </button>
    {/if}
  </header>

  <div class="notes-toolbar">
    <div class="notes-search-bar">
      <span class="notes-search-bar-icon" aria-hidden="true">
        <Icon icon="ph:magnifying-glass" />
      </span>
      <input
        type="text"
        bind:value={searchQuery}
        on:input={handleSearch}
        placeholder="Search your dictations..."
      />
    </div>
    <div class="notes-toolbar-actions">
      <div class="notes-toolbar-actions-scroll">
        <button
          type="button"
          class="notes-sort-cycle"
          title={historySortLabel}
          aria-label="Sort: {historySortLabel}. Click to change sort order."
          on:click={cycleHistorySort}
        >
          <span aria-hidden="true"><Icon icon={historySortIcon} /></span>
        </button>
      </div>
    </div>
  </div>

  {#if loading && entries.length === 0}
    <div class="state-container">
      <Icon icon="ph:spinner-gap-duotone" />
      <p>Loading history...</p>
    </div>
  {:else if error}
    <div class="state-container">
      <Icon icon="ph:warning-circle" />
      <p>{error}</p>
      <button type="button" class="btn-ghost" on:click={load}>Try Again</button>
    </div>
  {:else if entries.length === 0}
    <div class="empty-state">
      <Icon icon="ph:microphone" />
      <p>{searchQuery ? 'Try a different search term.' : 'Use a dictation hotkey to start dictating.'}</p>
    </div>
  {:else}
    <div class="timeline">
      {#each Object.entries(groupedHistory) as [dayLabel, dayData]}
        <div class="day-group">
          <h3 class="day-label">{dayLabel} <span class="day-sub">{dayData.sub}</span></h3>
          <div class="entries">
            {#each dayData.entries as entry (entry.id)}
              <div
                class="entry-row"
                role="button"
                tabindex="0"
                on:click={() => openHistoryDetail(entry)}
                on:keydown={(e) => onHistoryRowKeydown(e, entry)}
              >
                <div class="entry-time">{formatTime(entry.created_at)}</div>
                <div class="entry-content">
                  <p class="entry-text">
                    {#if searchQuery.trim() && (entry.text ?? '') !== ''}
                      {#each searchHighlightSegments(entry.text, searchQuery) as seg, i (i)}
                        {#if seg.hl}<mark class="history-search-hit">{seg.text}</mark>{:else}{seg.text}{/if}
                      {/each}
                    {:else}
                      {entry.text || '(empty)'}
                    {/if}
                  </p>
                  <div class="entry-actions">
                    <span class="chip chip-mode" class:dictation={isDictationMode(entry.mode)} class:command={isCommandMode(entry.mode)}>{modeLabel(entry.mode)}</span>
                    <span
                      class="chip chip-stt"
                      class:cloud={sttChipKind(entry.stt_mode, entry.stt_provider) === 'cloud'}
                      class:local={sttChipKind(entry.stt_mode, entry.stt_provider) === 'local'}
                      class:hybrid={sttChipKind(entry.stt_mode, entry.stt_provider) === 'hybrid'}
                      class:auto={sttChipKind(entry.stt_mode, entry.stt_provider) === 'auto'}
                      class:unknown={sttChipKind(entry.stt_mode, entry.stt_provider) === 'unknown'}
                    >{recognitionDisplay(entry.stt_provider, entry.stt_mode)}</span>
                    {#if entry.duration_ms != null}
                      <span class="entry-duration">{Math.round(entry.duration_ms / 1000)}s</span>
                    {/if}
                    <button
                      type="button"
                      class="icon-btn small"
                      class:copied={copiedEntryId === entry.id}
                      on:click|stopPropagation={() => copyText(entry.id, entry.text)}
                      title={copiedEntryId === entry.id ? 'Copied' : 'Copy'}
                      aria-label={copiedEntryId === entry.id ? 'Copied' : 'Copy'}
                    >
                      <Icon icon={copiedEntryId === entry.id ? 'ph:check' : 'ph:copy'} />
                    </button>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}

  {#if clearConfirmOpen}
    <div class="history-clear-root" aria-live="polite">
      <button
        type="button"
        class="history-clear-backdrop"
        aria-label="Cancel and close"
        disabled={clearing}
        on:click={closeClearConfirm}
      />
      <div
        class="history-clear-panel"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="history-clear-title"
        aria-describedby="history-clear-desc"
        on:click|stopPropagation
      >
        <h2 id="history-clear-title" class="history-clear-title">Clear all history?</h2>
        <p id="history-clear-desc" class="history-clear-desc">
          This removes every dictation from history. You cannot undo this.
        </p>
        {#if clearConfirmError}
          <p class="history-clear-error" role="alert">{clearConfirmError}</p>
        {/if}
        <div class="history-clear-actions">
          <button type="button" class="btn-ghost" disabled={clearing} on:click={closeClearConfirm}>
            Cancel
          </button>
          <button
            type="button"
            class="btn-danger-outline"
            disabled={clearing}
            on:click={runClearHistory}
          >
            {#if clearing}
              <span class="history-clear-spin" aria-hidden="true">
                <Icon icon="ph:spinner-gap-duotone" />
              </span>
              Clearing…
            {:else}
              <Icon icon="ph:trash" />
              Clear all
            {/if}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .history-clear-root {
    position: fixed;
    inset: 0;
    z-index: 4000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-md, 1rem);
    pointer-events: none;
  }
  .history-clear-root > * {
    pointer-events: auto;
  }
  .history-clear-backdrop {
    position: absolute;
    inset: 0;
    margin: 0;
    padding: 0;
    border: none;
    cursor: pointer;
    background: rgba(7, 16, 41, 0.45);
  }
  .history-clear-backdrop:disabled {
    cursor: not-allowed;
    opacity: 0.85;
  }
  .history-clear-panel {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 22rem;
    padding: var(--space-lg, 1.25rem);
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
  }
  .history-clear-title {
    margin: 0 0 0.5rem;
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .history-clear-desc {
    margin: 0 0 1rem;
    font-size: 0.9375rem;
    line-height: 1.45;
    color: var(--text-secondary);
  }
  .history-clear-error {
    margin: 0 0 1rem;
    font-size: 0.875rem;
    color: var(--error);
  }
  .history-clear-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    justify-content: flex-end;
  }
  .history-clear-spin {
    display: inline-flex;
    vertical-align: middle;
    margin-right: 0.35rem;
    animation: history-clear-spin 0.85s linear infinite;
  }
  .history-clear-spin :global(svg) {
    display: block;
  }
  @keyframes history-clear-spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
