<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import Icon from '@iconify/svelte'
  import type { HistoryEntry } from '../types'

  let unlistenTranscription: (() => void) | null = null

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
      dateLabel = date.toLocaleDateString(undefined, { month: 'long', day: 'numeric', year: 'numeric' })
    }

    if (!acc[dateLabel]) {
      acc[dateLabel] = []
    }
    acc[dateLabel].push(entry)
    return acc
  }, {} as Record<string, HistoryEntry[]>)

  onMount(async () => {
    await loadHistory()
    unlistenTranscription = await listen('transcription-saved', () => {
      loadHistory()
    })
  })

  onDestroy(() => {
    unlistenTranscription?.()
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

  let searchTimeout: ReturnType<typeof setTimeout>;
  function handleSearch() {
    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(async () => {
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
    }, 300);
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

<div class="home-view">
  <!-- Gamification Stats Row -->
  <section class="stats-grid">
    <!-- Streak -->
    <div class="stat-card streak-card">
      <div class="stat-icon-wrapper">
        <Icon icon="ph:fire-duotone" class="stat-icon" />
      </div>
      <div class="stat-content">
        <span class="stat-label">Current Streak</span>
        <div class="stat-value-row">
          <span class="stat-value">{stats.streak}</span>
          <span class="stat-unit">Days</span>
        </div>
      </div>
    </div>

    <!-- Time Saved -->
    <div class="stat-card time-card">
      <div class="stat-icon-wrapper">
        <Icon icon="ph:clock-duotone" class="stat-icon" />
      </div>
      <div class="stat-content">
        <span class="stat-label">Time Saved</span>
        <div class="stat-value-row">
          <span class="stat-value">{stats.timeSavedHours}</span>
          <span class="stat-unit">hrs</span>
        </div>
        <span class="stat-subtext">vs. typing at 40 WPM</span>
      </div>
    </div>

    <!-- Total Words -->
    <div class="stat-card words-card">
      <div class="stat-icon-wrapper">
        <Icon icon="ph:text-aa-duotone" class="stat-icon" />
      </div>
      <div class="stat-content">
        <span class="stat-label">Words Spoken</span>
        <div class="stat-value-row">
          <span class="stat-value">{stats.totalWords.toLocaleString()}</span>
        </div>
      </div>
    </div>

    <!-- WPM -->
    <div class="stat-card speed-card">
      <div class="stat-icon-wrapper">
        <Icon icon="ph:lightning-duotone" class="stat-icon" />
      </div>
      <div class="stat-content">
        <span class="stat-label">Average Speed</span>
        <div class="stat-value-row">
          <span class="stat-value">{stats.avgWpm}</span>
          <span class="stat-unit">WPM</span>
        </div>
      </div>
    </div>
  </section>

  <!-- History Section -->
  <section class="history-section">
    <header class="section-header">
      <div class="title-wrapper">
        <Icon icon="ph:clock-counter-clockwise-duotone" class="header-icon" />
        <h2>History</h2>
      </div>
      
      <div class="header-actions">
        <div class="search-wrapper">
          <Icon icon="ph:magnifying-glass-duotone" class="search-icon" />
          <input
            type="text"
            placeholder="Search history..."
            bind:value={searchQuery}
            on:input={handleSearch}
          />
        </div>

        <div class="export-dropdown">
          <button class="btn-ghost" disabled={exporting}>
            <Icon icon="ph:export-duotone" />
            <span>{exporting ? 'Exporting...' : 'Export'}</span>
          </button>
          <div class="dropdown-menu">
            <button on:click={() => exportHistory('json')}>
              <Icon icon="ph:file-json-duotone" /> JSON
            </button>
            <button on:click={() => exportHistory('csv')}>
              <Icon icon="ph:file-csv-duotone" /> CSV
            </button>
            <button on:click={() => exportHistory('txt')}>
              <Icon icon="ph:file-text-duotone" /> Text
            </button>
          </div>
        </div>

        <button class="btn-ghost danger" on:click={clearHistory} title="Clear All History">
          <Icon icon="ph:trash-duotone" />
        </button>
      </div>
    </header>

    {#if loading}
      <div class="state-container">
        <Icon icon="ph:spinner-gap-duotone" class="spin-icon" />
        <p>Loading history...</p>
      </div>
    {:else if entries.length === 0}
      <div class="state-container empty-state">
        <div class="empty-icon-wrapper">
          <Icon icon="ph:microphone-stage-duotone" class="empty-icon" />
        </div>
        <h3>No transcriptions found</h3>
        <p>{searchQuery ? 'Try a different search term.' : 'Press Ctrl+Win to start dictating!'}</p>
      </div>
    {:else}
      <div class="timeline">
        {#each Object.entries(groupedEntries) as [dateLabel, dayEntries]}
          <div class="day-group">
            <div class="day-header">
              <span class="day-label">{dateLabel}</span>
              <div class="day-line"></div>
            </div>
            
            <div class="day-entries">
              {#each dayEntries as entry}
                <div class="history-card">
                  <div class="history-header">
                    <span class="history-time">
                      <Icon icon="ph:clock-duotone" />
                      {formatTime(entry.created_at)}
                    </span>
                    <div class="history-badges">
                      <span class="badge mode-badge">{entry.mode}</span>
                      {#if entry.language}
                        <span class="badge lang-badge">{entry.language}</span>
                      {/if}
                    </div>
                  </div>
                  
                  <div class="history-body">
                    <p>{entry.text}</p>
                  </div>
                  
                  <div class="history-actions">
                    <button class="action-btn" on:click={() => copyToClipboard(entry.text)}>
                      <Icon icon="ph:copy-duotone" />
                      <span>Copy</span>
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
  .home-view {
    max-width: 1000px;
    margin: 0 auto;
    animation: fadeSlideUp 0.5s cubic-bezier(0.16, 1, 0.3, 1);
    display: flex;
    flex-direction: column;
    gap: 48px;
  }

  @keyframes fadeSlideUp {
    from { opacity: 0; transform: translateY(20px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* Stats Grid */
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 20px;
  }

  .stat-card {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 20px;
    padding: 24px;
    display: flex;
    align-items: flex-start;
    gap: 16px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.02);
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    position: relative;
    overflow: hidden;
  }

  .stat-card:hover {
    transform: translateY(-4px);
    box-shadow: 0 12px 24px rgba(0, 0, 0, 0.06);
    border-color: var(--border-visible);
  }

  .stat-icon-wrapper {
    width: 48px;
    height: 48px;
    border-radius: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .stat-icon {
    font-size: 24px;
  }

  .streak-card .stat-icon-wrapper { background: rgba(249, 115, 22, 0.1); color: #f97316; }
  .time-card .stat-icon-wrapper { background: rgba(16, 185, 129, 0.1); color: #10b981; }
  .words-card .stat-icon-wrapper { background: rgba(59, 130, 246, 0.1); color: #3b82f6; }
  .speed-card .stat-icon-wrapper { background: rgba(168, 85, 247, 0.1); color: #a855f7; }

  .stat-content {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .stat-label {
    font-size: 13px;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .stat-value-row {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .stat-value {
    font-size: 32px;
    font-weight: 800;
    color: var(--navy-deep);
    line-height: 1;
    letter-spacing: -0.02em;
  }

  .stat-unit {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .stat-subtext {
    font-size: 12px;
    color: var(--text-muted);
    margin-top: 4px;
  }

  /* History Section */
  .history-section {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 20px;
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

  .header-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .search-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 16px;
    font-size: 18px;
    color: var(--text-muted);
    pointer-events: none;
  }

  .search-wrapper input {
    width: 240px;
    padding: 10px 16px 10px 44px;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    color: var(--text-primary);
    font-size: 14px;
    font-family: inherit;
    transition: all 0.2s;
  }

  .search-wrapper input:focus {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 3px var(--primary-alpha);
    width: 280px;
  }

  .export-dropdown {
    position: relative;
  }

  .dropdown-menu {
    display: none;
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    padding: 8px;
    min-width: 160px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.08);
    z-index: 100;
  }

  .export-dropdown:hover .dropdown-menu {
    display: flex;
    flex-direction: column;
    gap: 4px;
    animation: dropdownFade 0.2s ease-out;
  }

  @keyframes dropdownFade {
    from { opacity: 0; transform: translateY(-8px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .dropdown-menu button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 10px 12px;
    background: transparent;
    border: none;
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .dropdown-menu button:hover {
    background: var(--bg-input);
    color: var(--navy-deep);
  }

  .btn-ghost {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-ghost:hover {
    background: var(--bg-input);
    color: var(--navy-deep);
    border-color: var(--border-visible);
  }

  .btn-ghost.danger:hover {
    color: var(--error);
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.2);
  }

  /* Timeline */
  .timeline {
    display: flex;
    flex-direction: column;
    gap: 40px;
  }

  .day-group {
    display: flex;
    flex-direction: column;
    gap: 20px;
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

  .day-label {
    font-size: 14px;
    font-weight: 800;
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

  .day-entries {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .history-card {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    padding: 24px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.02);
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
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
    margin-bottom: 16px;
    flex-wrap: wrap;
    gap: 12px;
  }

  .history-time {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    font-weight: 700;
    color: var(--text-muted);
    letter-spacing: 0.02em;
  }

  .history-badges {
    display: flex;
    gap: 8px;
  }

  .badge {
    padding: 4px 10px;
    border-radius: 8px;
    font-size: 11px;
    font-weight: 800;
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
    margin-top: 20px;
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
    padding: 8px 16px;
    background: var(--bg-input);
    border: 1px solid transparent;
    border-radius: 10px;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
  }

  .action-btn:hover {
    background: var(--bg-card);
    color: var(--navy-deep);
    border-color: var(--border-visible);
    box-shadow: 0 2px 8px rgba(0,0,0,0.05);
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

  @media (max-width: 768px) {
    .section-header {
      flex-direction: column;
      align-items: flex-start;
    }

    .header-actions {
      width: 100%;
      flex-wrap: wrap;
    }

    .search-wrapper {
      width: 100%;
    }

    .search-wrapper input {
      width: 100%;
    }

    .search-wrapper input:focus {
      width: 100%;
    }

    .export-dropdown, .export-dropdown > button {
      flex: 1;
    }

    .history-actions {
      opacity: 1;
    }
  }
</style>
