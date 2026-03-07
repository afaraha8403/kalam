<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import Icon from '@iconify/svelte'
  import type { HistoryEntry } from '../types'

  // Gamification Stats (Static for now)
  let stats = {
    streak: 4,
    totalWords: 12450,
    avgWpm: 145,
    timeSavedHours: 5.1 // Assuming 40 WPM typing speed
  }

  // History State
  let entries: HistoryEntry[] = []
  let searchQuery = ''
  let loading = true
  let exporting = false

  // Group entries by day
  $: groupedEntries = entries.reduce((acc, entry) => {
    const date = new Date(entry.created_at)
    const today = new Date()
    const yesterday = new Date(today)
    yesterday.setDate(yesterday.getDate() - 1)

    let dateLabel = ''
    if (date.toDateString() === today.toDateString()) {
      dateLabel = 'Today'
    } else if (date.toDateString() === yesterday.toDateString()) {
      dateLabel = 'Yesterday'
    } else {
      dateLabel = date.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
    }

    if (!acc[dateLabel]) {
      acc[dateLabel] = []
    }
    acc[dateLabel].push(entry)
    return acc
  }, {} as Record<string, HistoryEntry[]>)

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

  function formatTime(date: string): string {
    return new Date(date).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })
  }

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text)
  }
</script>

<div class="home">
  <!-- Gamification Stats Row -->
  <section class="stats-grid">
    <!-- Streak -->
    <div class="stat-card">
      <div class="stat-header">
        <Icon icon="ph:fire-duotone" class="stat-icon text-orange" />
        <span class="stat-label">Current Streak</span>
      </div>
      <div class="stat-value">{stats.streak} <span class="stat-unit">Days</span></div>
    </div>

    <!-- Time Saved -->
    <div class="stat-card">
      <div class="stat-header">
        <Icon icon="ph:clock-duotone" class="stat-icon text-emerald" />
        <span class="stat-label">Time Saved</span>
      </div>
      <div class="stat-value">{stats.timeSavedHours} <span class="stat-unit">hrs</span></div>
      <div class="stat-subtext">vs. typing at 40 WPM</div>
    </div>

    <!-- Total Words -->
    <div class="stat-card">
      <div class="stat-header">
        <Icon icon="ph:text-aa-duotone" class="stat-icon text-blue" />
        <span class="stat-label">Words Spoken</span>
      </div>
      <div class="stat-value">{stats.totalWords.toLocaleString()}</div>
    </div>

    <!-- WPM -->
    <div class="stat-card">
      <div class="stat-header">
        <Icon icon="ph:lightning-duotone" class="stat-icon text-purple" />
        <span class="stat-label">Average Speed</span>
      </div>
      <div class="stat-value">{stats.avgWpm} <span class="stat-unit">WPM</span></div>
    </div>
  </section>

  <!-- History Section -->
  <section class="history-section">
    <header>
      <h2>Transcription History</h2>
      <div class="actions">
        <div class="export-dropdown">
          <button class="btn-secondary" disabled={exporting}>
            <Icon icon="ph:export-duotone" />
            {exporting ? 'Exporting...' : 'Export'}
          </button>
          <div class="dropdown-menu">
            <button on:click={() => exportHistory('json')}>Export as JSON</button>
            <button on:click={() => exportHistory('csv')}>Export as CSV</button>
            <button on:click={() => exportHistory('txt')}>Export as Text</button>
          </div>
        </div>
        <button class="btn-danger" on:click={clearHistory}>
          <Icon icon="ph:trash-duotone" />
          Clear All
        </button>
      </div>
    </header>

    <div class="search">
      <Icon icon="ph:magnifying-glass-duotone" class="search-icon" />
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
      <div class="entries-container">
        {#each Object.entries(groupedEntries) as [dateLabel, dayEntries]}
          <div class="day-group">
            <div class="day-header">
              <h3>{dateLabel}</h3>
              <div class="day-line"></div>
            </div>
            <div class="entries">
              {#each dayEntries as entry}
                <div class="entry">
                  <div class="entry-header">
                    <span class="time">{formatTime(entry.created_at)}</span>
                    <span class="mode">{entry.mode}</span>
                    {#if entry.language}
                      <span class="language">{entry.language}</span>
                    {/if}
                  </div>
                  <p class="text">{entry.text}</p>
                  <div class="entry-actions">
                    <button on:click={() => copyToClipboard(entry.text)}>
                      <Icon icon="ph:copy-duotone" />
                      Copy
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>
</div>

<style>
  .home {
    max-width: 900px;
    animation: fadeIn 0.4s ease-out;
    display: flex;
    flex-direction: column;
    gap: 40px;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(10px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* Stats Grid */
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 16px;
  }

  .stat-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    padding: 20px;
    box-shadow: var(--shadow-sm);
    display: flex;
    flex-direction: column;
    gap: 8px;
    transition: transform 0.2s ease, box-shadow 0.2s ease;
  }

  .stat-card:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow-md);
    border-color: var(--border-visible);
  }

  .stat-header {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
  }

  .stat-label {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  :global(.stat-icon) {
    font-size: 20px;
  }

  :global(.text-orange) { color: #f97316; }
  :global(.text-emerald) { color: #10b981; }
  :global(.text-blue) { color: #3b82f6; }
  :global(.text-purple) { color: #a855f7; }

  .stat-value {
    font-size: 32px;
    font-weight: 700;
    color: var(--navy-deep);
    line-height: 1;
  }

  .stat-unit {
    font-size: 16px;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .stat-subtext {
    font-size: 12px;
    color: var(--text-muted);
  }

  /* History Section */
  .history-section {
    display: flex;
    flex-direction: column;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px;
  }

  h2 {
    font-size: 24px;
    font-weight: 700;
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
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 8px 0;
    margin-top: 8px;
    min-width: 180px;
    box-shadow: var(--shadow-lg);
    z-index: 100;
  }

  .export-dropdown:hover .dropdown-menu {
    display: block;
    animation: dropdownFade 0.2s ease-out;
  }

  @keyframes dropdownFade {
    from { opacity: 0; transform: translateY(-4px); }
    to { opacity: 1; transform: translateY(0); }
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
    font-size: 14px;
    font-weight: 500;
    transition: background 0.2s;
  }

  .dropdown-menu button:hover {
    background: var(--bg-input);
    color: var(--navy-deep);
  }

  .btn-secondary, .btn-danger {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    border-radius: var(--radius-md);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .btn-secondary {
    background: var(--bg-card);
    border: 1px solid var(--border-visible);
    color: var(--navy-deep);
    box-shadow: var(--shadow-sm);
  }

  .btn-secondary:hover {
    background: var(--bg-input);
    border-color: var(--navy-deep);
  }

  .btn-danger {
    background: transparent;
    border: 1px solid var(--error);
    color: var(--error);
  }

  .btn-danger:hover {
    background: rgba(239, 68, 68, 0.1);
  }

  .search {
    position: relative;
    margin-bottom: 32px;
    display: flex;
    align-items: center;
  }

  :global(.search-icon) {
    position: absolute;
    left: 16px;
    font-size: 20px;
    color: var(--text-muted);
  }

  .search input {
    width: 100%;
    padding: 14px 20px 14px 44px;
    background: var(--bg-input);
    border: 2px solid transparent;
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font-size: 15px;
    font-weight: 500;
    transition: all 0.2s ease;
    box-shadow: var(--shadow-inner);
  }

  .search input:focus {
    outline: none;
    background: var(--bg-card);
    border-color: var(--primary);
    box-shadow: 0 4px 12px var(--primary-alpha);
  }

  .search input:hover {
    background: var(--bg-input-hover);
  }

  .loading,
  .empty {
    text-align: center;
    padding: 80px 20px;
    color: var(--text-muted);
    background: var(--bg-card);
    border-radius: var(--radius-lg);
    border: 1px dashed var(--border);
  }

  .empty p {
    font-size: 16px;
    font-weight: 500;
  }

  .empty .hint {
    margin-top: 12px;
    color: var(--primary-dark);
    font-size: 14px;
  }

  .entries-container {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  .day-group {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .day-header {
    display: flex;
    align-items: center;
    gap: 16px;
    position: sticky;
    top: -48px;
    background: var(--bg-content);
    padding: 8px 0;
    z-index: 10;
  }

  .day-header h3 {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    white-space: nowrap;
  }

  .day-line {
    flex: 1;
    height: 1px;
    background: var(--border-subtle);
  }

  .entries {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .entry {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 24px;
    box-shadow: var(--shadow-sm);
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .entry:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow-md);
    border-color: var(--border-visible);
  }

  .entry-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
    font-size: 13px;
  }

  .time {
    color: var(--text-muted);
    font-weight: 600;
  }

  .mode {
    background: var(--primary-alpha);
    color: var(--primary-dark);
    padding: 4px 10px;
    border-radius: var(--radius-pill);
    font-weight: 600;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .language {
    background: var(--bg-input);
    color: var(--text-secondary);
    padding: 4px 10px;
    border-radius: var(--radius-pill);
    font-weight: 500;
    font-size: 12px;
  }

  .text {
    font-size: 16px;
    line-height: 1.6;
    color: var(--navy-deep);
    margin-bottom: 20px;
  }

  .entry-actions {
    display: flex;
    gap: 8px;
  }

  .entry-actions button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    background: var(--bg-input);
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .entry-actions button:hover {
    background: var(--bg-card);
    border-color: var(--border-visible);
    color: var(--navy-deep);
    box-shadow: var(--shadow-sm);
  }

  @media (max-width: 768px) {
    header {
      flex-direction: column;
      align-items: flex-start;
      gap: 16px;
      margin-bottom: 24px;
    }

    .actions {
      width: 100%;
      flex-direction: column;
    }

    .export-dropdown, .export-dropdown button {
      width: 100%;
    }

    .dropdown-menu {
      width: 100%;
      min-width: unset;
    }

    .entry {
      padding: 16px;
    }

    .entry-header {
      flex-wrap: wrap;
    }
  }
</style>
