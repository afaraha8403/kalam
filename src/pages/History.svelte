<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import type { HistoryEntry } from '../types'

  let entries: HistoryEntry[] = []
  let searchQuery = ''
  let loading = true
  let exporting = false

  onMount(async () => {
    await loadHistory()
  })

  async function loadHistory() {
    loading = true
    try {
      entries = await invoke('get_history', { limit: 100, offset: 0 })
    } catch (e) {
      console.error('Failed to load history:', e)
    } finally {
      loading = false
    }
  }

  async function search() {
    if (!searchQuery.trim()) {
      await loadHistory()
      return
    }

    loading = true
    try {
      entries = await invoke('search_history', { query: searchQuery })
    } catch (e) {
      console.error('Search failed:', e)
    } finally {
      loading = false
    }
  }

  async function clearHistory() {
    if (!confirm('Are you sure you want to clear all history? This cannot be undone.')) {
      return
    }

    try {
      await invoke('clear_history')
      entries = []
    } catch (e) {
      console.error('Failed to clear history:', e)
    }
  }

  async function exportHistory(format: 'json' | 'csv' | 'txt') {
    exporting = true
    try {
      const data = await invoke('export_history', { format })
      // Create and download file
      const blob = new Blob([data as string], { type: 'text/plain' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `kalam-history.${format}`
      a.click()
      URL.revokeObjectURL(url)
    } catch (e) {
      console.error('Export failed:', e)
    } finally {
      exporting = false
    }
  }

  function formatDate(date: string): string {
    return new Date(date).toLocaleString()
  }

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text)
  }
</script>

<div class="history">
  <header>
    <h2>Transcription History</h2>
    <div class="actions">
      <div class="export-dropdown">
        <button class="btn-secondary" disabled={exporting}>
          {exporting ? 'Exporting...' : 'Export'}
        </button>
        <div class="dropdown-menu">
          <button on:click={() => exportHistory('json')}>Export as JSON</button>
          <button on:click={() => exportHistory('csv')}>Export as CSV</button>
          <button on:click={() => exportHistory('txt')}>Export as Text</button>
        </div>
      </div>
      <button class="btn-danger" on:click={clearHistory}>Clear All</button>
    </div>
  </header>

  <div class="search">
    <input
      type="text"
      placeholder="Search transcriptions..."
      bind:value={searchQuery}
      on:input={search}
    />
  </div>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else if entries.length === 0}
    <div class="empty">
      <p>No transcriptions yet.</p>
      <p class="hint">Press Ctrl+Win to start dictating!</p>
    </div>
  {:else}
    <div class="entries">
      {#each entries as entry}
        <div class="entry">
          <div class="entry-header">
            <span class="date">{formatDate(entry.created_at)}</span>
            <span class="mode">{entry.mode}</span>
            {#if entry.language}
              <span class="language">{entry.language}</span>
            {/if}
          </div>
          <p class="text">{entry.text}</p>
          <div class="entry-actions">
            <button on:click={() => copyToClipboard(entry.text)}>Copy</button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .history {
    max-width: 900px;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 30px;
  }

  h2 {
    font-size: 28px;
    font-weight: 600;
  }

  .actions {
    display: flex;
    gap: 12px;
  }

  .export-dropdown {
    position: relative;
  }

  .dropdown-menu {
    display: none;
    position: absolute;
    top: 100%;
    right: 0;
    background: var(--bg-input);
    border-radius: 8px;
    padding: 8px 0;
    margin-top: 8px;
    min-width: 160px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    z-index: 100;
  }

  .export-dropdown:hover .dropdown-menu {
    display: block;
  }

  .dropdown-menu button {
    display: block;
    width: 100%;
    padding: 10px 16px;
    background: transparent;
    border: none;
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
  }

  .dropdown-menu button:hover {
    background: var(--border);
  }

  .btn-secondary {
    padding: 10px 20px;
    background: var(--bg-input);
    border: 1px solid var(--border-visible);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 14px;
    cursor: pointer;
  }

  .btn-danger {
    padding: 10px 20px;
    background: transparent;
    border: 1px solid var(--error);
    border-radius: 8px;
    color: var(--error);
    font-size: 14px;
    cursor: pointer;
  }

  .search {
    margin-bottom: 24px;
  }

  .search input {
    width: 100%;
    padding: 12px 16px;
    background: var(--bg-input);
    border: 1px solid var(--border-visible);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 14px;
  }

  .search input:focus {
    outline: none;
    border-color: var(--primary);
  }

  .loading,
  .empty {
    text-align: center;
    padding: 60px 20px;
    color: var(--text-muted);
  }

  .empty .hint {
    margin-top: 8px;
    color: var(--primary);
  }

  .entries {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .entry {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 20px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
  }

  .entry-header {
    display: flex;
    gap: 12px;
    margin-bottom: 12px;
    font-size: 12px;
  }

  .date {
    color: var(--text-secondary);
  }

  .mode {
    background: var(--primary);
    color: var(--white);
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: 500;
  }

  .language {
    background: var(--bg-input);
    color: var(--text-secondary);
    padding: 2px 8px;
    border-radius: 4px;
  }

  .text {
    font-size: 15px;
    line-height: 1.6;
    color: var(--text-primary);
    margin-bottom: 12px;
  }

  .entry-actions {
    display: flex;
    gap: 8px;
  }

  .entry-actions button {
    padding: 6px 12px;
    background: transparent;
    border: 1px solid var(--border-visible);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
  }

  .entry-actions button:hover {
    border-color: var(--primary);
    color: var(--primary);
  }
</style>
