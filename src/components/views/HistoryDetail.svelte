<script lang="ts">
  import { onMount } from 'svelte'
  import Icon from '@iconify/svelte'
  import { getEntry, createEntry, deleteEntry, newEntry } from '../../lib/api/db'
  import { getAppIcon } from '../../lib/api/appInfo'
  import type { Entry } from '../../types'
  import { selectedHistoryId } from '../../lib/historyDetailStore'
  import { selectedNoteId } from '../../lib/noteDetailStore'
  import { selectedTaskId } from '../../lib/taskDetailStore'
  import { noteDetailReturnTo, taskDetailReturnTo } from '../../lib/detailReturnStore'
  import { recognitionDisplay, sttChipKind } from '../../lib/historySttChip'

  export let navigate: (page: string) => void = () => {}

  let historyId: string | null = null
  let entry: Entry | null = null
  /** Data URL for target app icon from `applications` cache (when available). */
  let appIconUrl: string | null = null
  let loading = true
  let loadError: string | null = null
  let busy = false

  onMount(() => {
    return selectedHistoryId.subscribe((id) => {
      historyId = id ?? null
      loadEntry(historyId)
    })
  })

  async function loadEntry(id: string | null) {
    loading = true
    loadError = null
    entry = null
    if (!id) {
      loading = false
      return
    }
    try {
      const e = await getEntry(id)
      if (e && e.entry_type === 'history') {
        entry = e
      } else {
        loadError = 'This dictation could not be found.'
      }
    } catch (e) {
      loadError = e instanceof Error ? e.message : String(e)
    } finally {
      loading = false
    }
  }

  function back() {
    selectedHistoryId.set(null)
    navigate('history')
  }

  function wordCountFromEntry(e: Entry): number {
    if (e.word_count != null && e.word_count >= 0) return e.word_count
    return e.content.split(/\s+/).filter(Boolean).length
  }

  /** Rough speaking rate from recording length (not pure speech time). */
  function wordsPerMinute(e: Entry): number | null {
    const words = wordCountFromEntry(e)
    const ms = e.duration_ms
    if (ms == null || ms <= 0 || words <= 0) return null
    return Math.round((words * 60_000) / ms)
  }

  $: wpm = entry ? wordsPerMinute(entry) : null

  function formatDateTime(iso: string): string {
    return new Date(iso).toLocaleString(undefined, {
      weekday: 'short',
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit'
    })
  }

  function sessionModeLabel(e: Entry): string {
    const m = (e.session_mode ?? 'dictation').toLowerCase()
    return m === 'command' ? 'Command' : 'Dictation'
  }

  function titleFromTranscript(text: string): string {
    const line = text.split('\n').find((l) => l.trim()) ?? text
    const t = line.trim().slice(0, 120)
    return t || 'From dictation'
  }

  async function copyTranscript() {
    if (!entry?.content) return
    try {
      await navigator.clipboard.writeText(entry.content)
    } catch {
      /* ignore */
    }
  }

  async function convertToNote() {
    if (!entry || busy) return
    busy = true
    try {
      const note = newEntry('note', entry.content, {
        title: titleFromTranscript(entry.content)
      })
      await createEntry(note)
      noteDetailReturnTo.set({ type: 'history-detail', entryId: entry.id })
      selectedHistoryId.set(null)
      selectedNoteId.set(note.id)
      navigate('note-detail')
    } catch (e) {
      console.error(e)
    } finally {
      busy = false
    }
  }

  async function convertToTask() {
    if (!entry || busy) return
    busy = true
    try {
      const task = newEntry('task', entry.content, {
        title: titleFromTranscript(entry.content),
        is_completed: false
      })
      await createEntry(task)
      taskDetailReturnTo.set({ type: 'history-detail', entryId: entry.id })
      selectedHistoryId.set(null)
      selectedTaskId.set(task.id)
      navigate('task-detail')
    } catch (e) {
      console.error(e)
    } finally {
      busy = false
    }
  }

  async function removeFromHistory() {
    if (!entry || busy) return
    if (!confirm('Delete this dictation from history? This cannot be undone.')) return
    busy = true
    try {
      await deleteEntry(entry.id)
      back()
    } catch (e) {
      console.error(e)
    } finally {
      busy = false
    }
  }
</script>

<div class="page fade-in sleek-editor-page history-detail-page">
  <header class="sleek-header">
    <button type="button" class="sleek-back" on:click={back}>
      <Icon icon="ph:caret-left" /> History
    </button>
    <div class="sleek-actions history-detail-header-actions">
      <button
        type="button"
        class="sleek-icon-btn danger"
        on:click={removeFromHistory}
        disabled={!entry || busy}
        title="Delete from history"
        aria-label="Delete from history"
      >
        <Icon icon="ph:trash" />
      </button>
      <button
        type="button"
        class="sleek-icon-btn"
        on:click={convertToNote}
        disabled={!entry || busy}
        title="Save as note"
        aria-label="Save as note"
      >
        <Icon icon="ph:notebook" />
      </button>
      <button
        type="button"
        class="sleek-icon-btn"
        on:click={convertToTask}
        disabled={!entry || busy}
        title="Save as task"
        aria-label="Save as task"
      >
        <Icon icon="ph:check-square" />
      </button>
      <button type="button" class="sleek-save" on:click={copyTranscript} disabled={!entry}>
        Copy
      </button>
    </div>
  </header>

  <div class="sleek-body history-detail-body">
    {#if loading}
      <div class="state-container">
        <Icon icon="ph:spinner-gap-duotone" />
        <p>Loading…</p>
      </div>
    {:else if loadError}
      <div class="state-container">
        <Icon icon="ph:warning-circle" />
        <p>{loadError}</p>
        <button type="button" class="btn-ghost" on:click={back}>Back</button>
      </div>
    {:else if entry}
      <p class="history-detail-when">{formatDateTime(entry.created_at)}</p>
      <div class="history-detail-chips">
        <span
          class="chip chip-mode small"
          class:dictation={sessionModeLabel(entry) === 'Dictation'}
          class:command={sessionModeLabel(entry) === 'Command'}
        >
          {sessionModeLabel(entry).toLowerCase()}
        </span>
        <span
          class="chip chip-stt small"
          class:cloud={sttChipKind(entry.stt_mode, entry.stt_provider) === 'cloud'}
          class:local={sttChipKind(entry.stt_mode, entry.stt_provider) === 'local'}
          class:hybrid={sttChipKind(entry.stt_mode, entry.stt_provider) === 'hybrid'}
          class:auto={sttChipKind(entry.stt_mode, entry.stt_provider) === 'auto'}
          class:unknown={sttChipKind(entry.stt_mode, entry.stt_provider) === 'unknown'}
        >
          {recognitionDisplay(entry.stt_provider, entry.stt_mode)}
        </span>
      </div>

      <dl class="history-stats">
        <div class="history-stat">
          <dt>Words</dt>
          <dd>{wordCountFromEntry(entry).toLocaleString()}</dd>
        </div>
        {#if wpm != null}
          <div class="history-stat">
            <dt>Words / min</dt>
            <dd>~{wpm} <span class="history-stat-hint">(vs recording length)</span></dd>
          </div>
        {/if}
        <div class="history-stat">
          <dt>Recording length</dt>
          <dd>
            {#if entry.duration_ms != null && entry.duration_ms > 0}
              {(entry.duration_ms / 1000).toFixed(1)}s
            {:else}
              —
            {/if}
          </dd>
        </div>
        <div class="history-stat">
          <dt>STT latency</dt>
          <dd>{entry.stt_latency_ms != null ? `${entry.stt_latency_ms} ms` : '—'}</dd>
        </div>
        <div class="history-stat">
          <dt>Recognition</dt>
          <dd>{recognitionDisplay(entry.stt_provider, entry.stt_mode)}</dd>
        </div>
        <div class="history-stat">
          <dt>Target app</dt>
          <dd class="history-target-app">
            {#if appIconUrl}
              <img
                src={appIconUrl}
                alt=""
                class="history-target-app-icon"
                width="18"
                height="18"
              />
            {/if}
            <span title={entry.target_app?.trim() || undefined}>
              {entry.target_app_name?.trim() || entry.target_app?.trim() || '—'}
            </span>
          </dd>
        </div>
      </dl>

      <div class="history-transcript-head">
        <h3 class="section-title history-transcript-title">Transcript</h3>
        <div class="history-transcript-head-actions">
          <button type="button" class="sleek-icon-btn" on:click={copyTranscript} title="Copy transcript" aria-label="Copy transcript">
            <Icon icon="ph:copy" />
          </button>
        </div>
      </div>
      {#key entry.id}
        <textarea class="sleek-content history-transcript" readonly rows="12">{entry.content}</textarea>
      {/key}
    {:else}
      <div class="state-container">
        <p>Choose a dictation from History.</p>
        <button type="button" class="btn-ghost" on:click={back}>Back</button>
      </div>
    {/if}
  </div>
</div>

<style>
  /* Back on the left; delete / note / task / Copy grouped on the right (not hugging the back control). */
  .history-detail-page :global(.sleek-header) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md, 16px);
    width: 100%;
    min-width: 0;
  }

  .history-detail-page .history-detail-header-actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    flex-shrink: 0;
    min-width: 0;
  }

  :global(.kalam-sleek .page-content .history-detail-header-actions .sleek-icon-btn:disabled) {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .history-transcript-head-actions :global(.sleek-icon-btn) {
    width: 36px;
    height: 36px;
    flex-shrink: 0;
  }

  .history-transcript-head-actions :global(.sleek-icon-btn svg) {
    width: 18px;
    height: 18px;
  }

  .history-transcript-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md, 16px);
    margin-bottom: var(--space-sm, 8px);
  }

  .history-transcript-head .history-transcript-title {
    margin-bottom: 0;
  }

  .history-detail-body {
    max-width: 720px;
  }

  .history-detail-when {
    font-size: 15px;
    color: var(--text-muted, #86868b);
    margin-bottom: var(--space-md, 16px);
  }

  .history-detail-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: var(--space-lg, 24px);
  }

  .history-stats {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: var(--space-md, 16px);
    margin-bottom: var(--space-xl, 32px);
  }

  .history-stat {
    padding: var(--space-md, 16px);
    border-radius: var(--radius-md, 12px);
    background: var(--bg-elevated, rgba(255, 255, 255, 0.06));
  }

  .history-stat dt {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted, #86868b);
    margin-bottom: 6px;
  }

  .history-stat dd {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    word-break: break-word;
  }

  .history-stat-hint {
    font-weight: 400;
    font-size: 12px;
    color: var(--text-muted, #86868b);
  }

  .history-target-app {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    font-weight: 600;
  }

  .history-target-app-icon {
    flex-shrink: 0;
    border-radius: 4px;
    object-fit: contain;
  }

  .history-transcript {
    width: 100%;
    min-height: 200px;
    resize: vertical;
    font-family: inherit;
    line-height: 1.5;
  }

  :global(.kalam-sleek.light) .history-stat {
    background: rgba(0, 0, 0, 0.04);
  }
</style>
