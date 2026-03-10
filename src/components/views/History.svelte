<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import Icon from '@iconify/svelte'
  import type { HistoryEntry } from '../../types'

  let entries: HistoryEntry[] = []
  let loading = true
  let error: string | null = null
  let searchQuery = ''

  async function load() {
    loading = true
    error = null
    try {
      if (searchQuery.trim()) {
        entries = (await invoke('search_history', { query: searchQuery })) as HistoryEntry[]
      } else {
        entries = (await invoke('get_history', { limit: 100, offset: 0 })) as HistoryEntry[]
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    } finally {
      loading = false
    }
  }

  onMount(() => {
    load()
    let unlisten: (() => void) | null = null
    listen('transcription-saved', () => {
      load()
    }).then((fn) => { unlisten = fn })
    return () => { unlisten?.() }
  })

  let searchTimeout: ReturnType<typeof setTimeout>;
  function handleSearch() {
    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => {
      load();
    }, 300);
  }

  function formatDate(iso: string) {
    try {
      const d = new Date(iso)
      const today = new Date()
      const isToday = d.toDateString() === today.toDateString()
      
      if (isToday) {
        return `Today at ${d.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })}`
      }
      return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' })
    } catch {
      return iso
    }
  }

  function copyText(text: string) {
    navigator.clipboard.writeText(text)
  }
</script>

<div class="view history-view">
  <header class="page-header">
    <div class="header-content">
      <div class="title-wrapper">
        <Icon icon="ph:clock-counter-clockwise-duotone" class="header-icon" />
        <h2>History</h2>
      </div>
      <p class="subtitle">Your past dictations and transcriptions.</p>
    </div>
  </header>

  <div class="search-bar">
    <Icon icon="ph:magnifying-glass-duotone" class="search-icon" />
    <input 
      type="text" 
      bind:value={searchQuery} 
      on:input={handleSearch}
      placeholder="Search your history..."
    />
  </div>

  {#if loading && entries.length === 0}
    <div class="state-container">
      <Icon icon="ph:spinner-gap-duotone" class="spin-icon" />
      <p>Loading history...</p>
    </div>
  {:else if error}
    <div class="state-container error-state">
      <Icon icon="ph:warning-circle-duotone" class="error-icon" />
      <p>{error}</p>
      <button class="btn-ghost" on:click={load}>Try Again</button>
    </div>
  {:else if entries.length === 0}
    <div class="state-container empty-state">
      <div class="empty-icon-wrapper">
        <Icon icon="ph:microphone-stage-duotone" class="empty-icon" />
      </div>
      <h3>No transcriptions found</h3>
      <p>{searchQuery ? 'Try a different search term.' : 'Use a dictation hotkey to start dictating.'}</p>
    </div>
  {:else}
    <div class="timeline">
      {#each entries as entry (entry.id)}
        <div class="timeline-item">
          <div class="timeline-marker">
            <div class="marker-dot"></div>
          </div>
          <div class="history-card" role="button" tabindex="0" on:click={() => copyText(entry.text)} on:keydown={(e) => e.key === 'Enter' && copyText(entry.text)}>
            <div class="history-header">
              <span class="history-time">
                <Icon icon="ph:calendar-blank-duotone" />
                {formatDate(entry.created_at)}
              </span>
              <div class="history-badges">
                {#if entry.mode}
                  <span class="badge mode-badge">{entry.mode}</span>
                {/if}
                {#if entry.language}
                  <span class="badge lang-badge">{entry.language}</span>
                {/if}
              </div>
            </div>
            <div class="history-body">
              <p>{entry.text || '(empty)'}</p>
            </div>
            <div class="history-actions" on:click|stopPropagation on:keydown|stopPropagation>
              <button class="action-btn" on:click={() => copyText(entry.text)} title="Copy to clipboard">
                <Icon icon="ph:copy-duotone" />
                <span>Copy</span>
              </button>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .view {
    max-width: 800px;
    margin: 0 auto;
    animation: fadeSlideUp 0.5s cubic-bezier(0.16, 1, 0.3, 1);
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  @keyframes fadeSlideUp {
    from { opacity: 0; transform: translateY(20px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* Header */
  .page-header {
    position: relative;
  }

  .header-content {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .title-wrapper {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .header-icon {
    font-size: 24px;
    color: var(--primary);
  }

  h2 {
    font-size: 24px;
    font-weight: 700;
    color: var(--navy-deep);
    margin: 0;
  }

  .subtitle {
    color: var(--text-muted);
    font-size: 15px;
    margin: 0;
    padding-left: 34px;
  }

  /* Search */
  .search-bar {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 20px;
    font-size: 20px;
    color: var(--text-muted);
    pointer-events: none;
  }

  .search-bar input {
    width: 100%;
    padding: 16px 20px 16px 52px;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    color: var(--text-primary);
    font-size: 16px;
    font-family: inherit;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.02);
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .search-bar input:focus {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 4px var(--primary-alpha), 0 8px 24px rgba(0, 0, 0, 0.04);
    transform: translateY(-2px);
  }

  /* Timeline */
  .timeline {
    display: flex;
    flex-direction: column;
    gap: 24px;
    position: relative;
    padding-left: 24px;
  }

  .timeline::before {
    content: '';
    position: absolute;
    left: 7px;
    top: 12px;
    bottom: 24px;
    width: 2px;
    background: var(--border-subtle);
    border-radius: 2px;
  }

  .timeline-item {
    position: relative;
    display: flex;
    gap: 24px;
  }

  .timeline-marker {
    position: absolute;
    left: -24px;
    top: 16px;
    width: 16px;
    height: 16px;
    background: var(--bg-app);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2;
  }

  .marker-dot {
    width: 8px;
    height: 8px;
    background: var(--primary);
    border-radius: 50%;
    box-shadow: 0 0 0 4px var(--primary-alpha);
  }

  .history-card {
    flex: 1;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    padding: 20px 24px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.02);
    transition: all 0.3s ease;
    cursor: pointer;
  }

  .history-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 12px 24px rgba(0, 0, 0, 0.04);
    border-color: var(--border-visible);
  }

  .history-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
    flex-wrap: wrap;
    gap: 12px;
  }

  .history-time {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .history-badges {
    display: flex;
    gap: 8px;
  }

  .badge {
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .mode-badge {
    background: var(--primary-alpha-light);
    color: var(--primary-dark);
    border: 1px solid var(--primary-alpha);
  }

  .lang-badge {
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border-subtle);
  }

  .history-body p {
    font-size: 16px;
    line-height: 1.6;
    color: var(--navy-deep);
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .history-actions {
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px dashed var(--border-subtle);
    display: flex;
    justify-content: flex-end;
    opacity: 0.6;
    transition: opacity 0.2s;
  }

  .history-card:hover .history-actions {
    opacity: 1;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 8px;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .action-btn:hover {
    background: var(--bg-input);
    color: var(--navy-deep);
    border-color: var(--border-visible);
  }

  /* States */
  .state-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 64px 20px;
    background: var(--bg-card);
    border-radius: 20px;
    border: 1px dashed var(--border-visible);
    color: var(--text-muted);
    gap: 16px;
  }

  .spin-icon {
    font-size: 32px;
    animation: spin 1s linear infinite;
    color: var(--primary);
  }

  @keyframes spin {
    100% { transform: rotate(360deg); }
  }

  .empty-state {
    text-align: center;
  }

  .empty-icon-wrapper {
    width: 64px;
    height: 64px;
    background: var(--primary-alpha);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 8px;
  }

  .empty-icon {
    font-size: 32px;
    color: var(--primary);
  }

  .empty-state h3 {
    font-size: 20px;
    font-weight: 700;
    color: var(--navy-deep);
    margin: 0;
  }

  .empty-state p {
    font-size: 15px;
    max-width: 300px;
    margin: 0;
  }

  .error-state {
    border-color: rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.02);
  }

  .error-icon {
    font-size: 32px;
    color: var(--error);
  }

  .btn-ghost {
    padding: 8px 16px;
    background: var(--bg-card);
    border: 1px solid var(--border-visible);
    border-radius: 8px;
    font-weight: 600;
    cursor: pointer;
    color: var(--navy-deep);
  }

  @media (max-width: 768px) {
    .timeline {
      padding-left: 16px;
    }
    .timeline::before {
      left: 3px;
    }
    .timeline-marker {
      left: -20px;
      width: 12px;
      height: 12px;
    }
    .marker-dot {
      width: 6px;
      height: 6px;
    }
    .subtitle {
      padding-left: 0;
    }
    .history-actions {
      opacity: 1;
    }
  }
</style>
