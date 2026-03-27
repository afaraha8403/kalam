<script lang="ts">
  import { onMount } from 'svelte'
  import Icon from '@iconify/svelte'
  import { getEntry, createEntry, deleteEntry, newEntry } from '../../lib/api/db'
  import { getAppIcon } from '../../lib/api/appInfo'
  import { invoke, listenSafe } from '../../lib/backend'
  import type { AppConfig, Entry, SensitiveAppPattern } from '../../types'
  import { isProcessInSensitiveList, processNameMatchesSensitivePattern } from '../../lib/sensitiveAppPatterns'
  import { selectedHistoryId } from '../../lib/historyDetailStore'
  import { selectedNoteId } from '../../lib/noteDetailStore'
  import { selectedTaskId } from '../../lib/taskDetailStore'
  import { noteDetailReturnTo, taskDetailReturnTo } from '../../lib/detailReturnStore'
  import { recognitionDisplay, sttChipKind } from '../../lib/historySttChip'
  import { historyLanguageLabel } from '../../lib/languages'

  export let navigate: (page: string) => void = () => {}

  let historyId: string | null = null
  let entry: Entry | null = null
  /** Data URL for target app icon from `applications` cache (when available). */
  let appIconUrl: string | null = null
  let loading = true
  let loadError: string | null = null
  let busy = false
  /** Process name from the open history row; kept in sync for `settings_updated` (closure-safe). */
  let targetProcessForPrivacy = ''
  let inSensitiveList = false
  let privacyBusy = false
  /** Matches History feed: check icon + label after a successful clipboard write. */
  let transcriptCopied = false
  let copyResetTimer: ReturnType<typeof setTimeout> | undefined

  function escapeRegex(str: string): string {
    return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  }

  /** Same regex shape as `addSensitiveAppFromPicker` in Settings. */
  function buildSensitivePatternForProcess(processName: string): SensitiveAppPattern {
    const baseName = processName.replace(/\.exe$/i, '').replace(/\.app$/i, '')
    const pattern = `(?i)^${escapeRegex(baseName)}(\\.exe)?$`
    return { pattern, pattern_type: 'ProcessName', action: 'ForceLocal' }
  }

  async function syncTargetAppChrome(processName: string) {
    const [icon, cfg] = await Promise.all([
      getAppIcon(processName).catch(() => null),
      invoke('get_settings').catch(() => null) as Promise<AppConfig | null>
    ])
    // Ignore late results if the user switched to another history row.
    if (targetProcessForPrivacy !== processName) return
    appIconUrl = icon
    inSensitiveList = isProcessInSensitiveList(cfg?.privacy?.sensitive_app_patterns, processName)
  }

  async function toggleSensitiveApp() {
    const proc = targetProcessForPrivacy
    if (!proc || privacyBusy) return
    privacyBusy = true
    try {
      const cfg = (await invoke('get_settings')) as AppConfig
      const patterns = [...(cfg.privacy?.sensitive_app_patterns ?? [])]
      const next = isProcessInSensitiveList(patterns, proc)
        ? patterns.filter((p) => !processNameMatchesSensitivePattern(proc, p))
        : [...patterns, buildSensitivePatternForProcess(proc)]
      await invoke('save_settings', {
        newConfig: { ...cfg, privacy: { ...cfg.privacy, sensitive_app_patterns: next } }
      })
      inSensitiveList = isProcessInSensitiveList(next, proc)
    } catch (e) {
      console.error(e)
    } finally {
      privacyBusy = false
    }
  }

  onMount(() => {
    const unsub = selectedHistoryId.subscribe((id) => {
      historyId = id ?? null
      loadEntry(historyId)
    })
    let unlisten: (() => void) | undefined
    void listenSafe<AppConfig>('settings_updated', (ev) => {
      const proc = targetProcessForPrivacy
      if (!proc || !ev.payload?.privacy) return
      inSensitiveList = isProcessInSensitiveList(ev.payload.privacy.sensitive_app_patterns, proc)
    }).then((fn) => {
      unlisten = fn
    })
    return () => {
      unsub()
      unlisten?.()
      if (copyResetTimer !== undefined) clearTimeout(copyResetTimer)
    }
  })

  async function loadEntry(id: string | null) {
    transcriptCopied = false
    if (copyResetTimer !== undefined) {
      clearTimeout(copyResetTimer)
      copyResetTimer = undefined
    }
    loading = true
    loadError = null
    entry = null
    targetProcessForPrivacy = ''
    appIconUrl = null
    inSensitiveList = false
    if (!id) {
      loading = false
      return
    }
    try {
      const e = await getEntry(id)
      if (e && e.entry_type === 'history') {
        entry = e
        const proc = e.target_app?.trim() ?? ''
        targetProcessForPrivacy = proc
        if (proc) {
          void syncTargetAppChrome(proc)
        } else {
          appIconUrl = null
          inSensitiveList = false
        }
      } else {
        loadError = 'This dictation could not be found.'
        targetProcessForPrivacy = ''
        appIconUrl = null
        inSensitiveList = false
      }
    } catch (e) {
      loadError = e instanceof Error ? e.message : String(e)
      targetProcessForPrivacy = ''
      appIconUrl = null
      inSensitiveList = false
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
      return
    }
    transcriptCopied = true
    if (copyResetTimer !== undefined) clearTimeout(copyResetTimer)
    copyResetTimer = setTimeout(() => {
      transcriptCopied = false
      copyResetTimer = undefined
    }, 1600)
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
      <button
        type="button"
        class="sleek-save"
        class:copied={transcriptCopied}
        on:click={copyTranscript}
        disabled={!entry}
      >
        {transcriptCopied ? 'Copied' : 'Copy'}
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

      <div class="history-stats">
        <!-- Row 1: numeric/session metrics — equal-width cells when they share a row -->
        <div class="history-stats-metrics">
          <dl class="history-stat">
            <dt>Words</dt>
            <dd>{wordCountFromEntry(entry).toLocaleString()}</dd>
          </dl>
          {#if wpm != null}
            <dl class="history-stat">
              <dt>Speaking pace</dt>
              <dd>~{wpm} <span class="history-stat-hint">(estimated from recording length)</span></dd>
            </dl>
          {/if}
          <dl class="history-stat">
            <dt>Recording length</dt>
            <dd>
              {#if entry.duration_ms != null && entry.duration_ms > 0}
                {(entry.duration_ms / 1000).toFixed(1)}s
              {:else}
                —
              {/if}
            </dd>
          </dl>
          <dl class="history-stat">
            <dt>STT latency</dt>
            <dd>{entry.stt_latency_ms != null ? `${entry.stt_latency_ms} ms` : '—'}</dd>
          </dl>
        </div>

        <!-- Row 2: same height, fills width (fixes lone Recognition cell) -->
        <div class="history-stats-meta">
          <div class="history-stat">
            <dt>Transcription</dt>
            <dd>{recognitionDisplay(entry.stt_provider, entry.stt_mode)}</dd>
          </div>
          <div class="history-stat">
            <dt>Language</dt>
            <dd>{historyLanguageLabel(entry.dictation_language)}</dd>
          </div>
        </div>

        <!-- Row 3: target + privacy needs full width -->
        <div class="history-stats-target">
          <dl class="history-stat">
            <dt>Target app</dt>
            <dd class="history-target-app-shell">
              <div class="history-target-app">
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
              </div>
              {#if targetProcessForPrivacy}
                <div class="history-privacy-block">
                  <div class="history-privacy-text">
                    <div class="history-privacy-title-row">
                      <span class="history-privacy-title" id="history-detail-privacy-toggle-label">
                        Include in Sensitive apps
                      </span>
                      <button
                        type="button"
                        class="history-privacy-info-btn"
                        popovertarget="history-sensitive-help-popover"
                        aria-label="More about Sensitive apps"
                      >
                        <Icon icon="ph:info" />
                      </button>
                    </div>
                    <p class="history-privacy-desc" id="history-detail-privacy-toggle-desc">
                      When this app is in focus, Kalam can keep your audio on this device instead of sending it to the
                      cloud (when you use Hybrid or Auto and have sensitive-app detection on in Settings).
                    </p>
                  </div>
                  <label class="toggle-switch history-sensitive-toggle" for="history-detail-privacy-toggle">
                    <input
                      id="history-detail-privacy-toggle"
                      type="checkbox"
                      checked={inSensitiveList}
                      disabled={privacyBusy}
                      aria-labelledby="history-detail-privacy-toggle-label"
                      aria-describedby="history-detail-privacy-toggle-desc"
                      on:click|preventDefault={() => void toggleSensitiveApp()}
                    />
                    <span class="slider"></span>
                  </label>
                </div>
              {/if}
            </dd>
          </dl>
        </div>
      </div>

      <div id="history-sensitive-help-popover" popover class="history-sensitive-help-popover">
        <p>
          This uses the same list as <strong>Settings → Data &amp; Privacy → Sensitive apps</strong>. For extra privacy
          in password managers or similar apps, turn on sensitive-app detection and use Hybrid or Auto — Kalam then
          prefers on-device transcription while that app is open.
        </p>
      </div>

      <div class="history-transcript-head">
        <h3 class="section-title history-transcript-title">Transcript</h3>
        <div class="history-transcript-head-actions">
          <button
            type="button"
            class="sleek-icon-btn"
            class:copied={transcriptCopied}
            on:click={copyTranscript}
            title={transcriptCopied ? 'Copied' : 'Copy transcript'}
            aria-label={transcriptCopied ? 'Copied' : 'Copy transcript'}
          >
            <Icon icon={transcriptCopied ? 'ph:check' : 'ph:copy'} />
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

  @keyframes history-detail-copy-pop {
    0% {
      transform: scale(1);
    }
    40% {
      transform: scale(1.14);
    }
    100% {
      transform: scale(1);
    }
  }

  /* Same feedback as History list rows (App.svelte `.icon-btn.small.copied`). */
  .history-transcript-head-actions :global(.sleek-icon-btn.copied) {
    animation: history-detail-copy-pop 0.38s cubic-bezier(0.34, 1.2, 0.64, 1);
    color: var(--text);
    background: var(--bg-hover);
  }

  .history-detail-page :global(.sleek-save.copied) {
    animation: history-detail-copy-pop 0.38s cubic-bezier(0.34, 1.2, 0.64, 1);
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
    display: flex;
    flex-direction: column;
    gap: var(--space-md, 16px);
    margin-bottom: var(--space-xl, 32px);
  }

  .history-stats-metrics {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(148px, 1fr));
    gap: var(--space-md, 16px);
  }

  .history-stats-meta {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-md, 16px);
  }

  @media (max-width: 420px) {
    .history-stats-meta {
      grid-template-columns: 1fr;
    }
  }

  .history-stat {
    margin: 0;
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

  .history-privacy-title-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-bottom: 6px;
  }

  .history-privacy-title-row .history-privacy-title {
    margin-bottom: 0;
  }

  .history-privacy-info-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin: 0;
    padding: 2px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted, #86868b);
    cursor: pointer;
    line-height: 0;
    flex-shrink: 0;
  }

  .history-privacy-info-btn:hover {
    color: var(--text, inherit);
    background: var(--bg-hover, rgba(255, 255, 255, 0.08));
  }

  .history-privacy-info-btn :global(svg) {
    width: 18px;
    height: 18px;
  }

  .history-sensitive-help-popover {
    margin: 0;
    max-width: min(360px, 92vw);
    padding: var(--space-md, 16px);
    border: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.12));
    border-radius: var(--radius-md, 12px);
    background: var(--bg-elevated);
    color: var(--text);
    box-shadow: var(--shadow-lg, 0 12px 40px rgba(0, 0, 0, 0.35));
    font-size: 13px;
    line-height: 1.5;
  }

  .history-sensitive-help-popover p {
    margin: 0;
  }

  .history-sensitive-help-popover :global(strong) {
    font-weight: 600;
  }

  :global(.kalam-sleek.light) .history-sensitive-help-popover {
    box-shadow: var(--shadow-lg, 0 12px 32px rgba(0, 0, 0, 0.12));
  }

  .history-target-app-shell {
    margin: 0;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 14px;
  }

  .history-target-app {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    font-weight: 600;
    min-width: 0;
  }

  .history-privacy-block {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px 20px;
    padding-top: 12px;
    border-top: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.08));
  }

  :global(.kalam-sleek.light) .history-privacy-block {
    border-top-color: rgba(0, 0, 0, 0.08);
  }

  .history-privacy-text {
    flex: 1;
    min-width: min(100%, 240px);
  }

  .history-privacy-title {
    display: block;
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 6px;
    color: var(--text, inherit);
  }

  .history-privacy-desc {
    margin: 0;
    font-size: 12px;
    font-weight: 400;
    line-height: 1.45;
    color: var(--text-muted, #86868b);
  }

  .history-privacy-desc :global(strong) {
    font-weight: 600;
    color: var(--text, inherit);
  }

  .history-sensitive-toggle {
    flex-shrink: 0;
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
