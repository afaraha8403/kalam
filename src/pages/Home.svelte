<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import Icon from '@iconify/svelte'
  import type { HistoryEntry, AggregateStats, Entry } from '../types'

  export let navigate: (page: string) => void = () => {}

  let unlistenTranscription: (() => void) | null = null

  // Dashboard stats from daily_stats (loaded via get_aggregate_stats)
  let stats: AggregateStats | null = null
  let statsLoading = true

  // Tasks and reminders due today
  let tasksDueToday: Entry[] = []
  let remindersDueToday: Entry[] = []
  let todayIso = ''

  // History State (recent only on home; full list on History page)
  const RECENT_HISTORY_LIMIT = 5
  let entries: HistoryEntry[] = []
  let loading = true

  function getTodayIso(): string {
    const d = new Date()
    return d.getFullYear() + '-' + String(d.getMonth() + 1).padStart(2, '0') + '-' + String(d.getDate()).padStart(2, '0')
  }

  async function loadStats() {
    statsLoading = true
    try {
      stats = (await invoke('get_aggregate_stats')) as AggregateStats
    } catch (e) {
      console.error('Failed to load aggregate stats:', e)
    } finally {
      statsLoading = false
    }
  }

  async function loadTasksAndRemindersToday() {
    todayIso = getTodayIso()
    try {
      tasksDueToday = (await invoke('get_tasks_due_on', { date: todayIso, limit: 10 })) as Entry[]
      remindersDueToday = (await invoke('get_reminders_due_on', { date: todayIso, limit: 10 })) as Entry[]
    } catch (e) {
      console.error('Failed to load tasks/reminders:', e)
    }
  }

  // Group entries by day (for recent history)
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
    todayIso = getTodayIso()
    await loadStats()
    await loadTasksAndRemindersToday()
    await loadHistory()
    unlistenTranscription = await listen('transcription-saved', () => {
      loadStats()
      loadHistory()
    })
  })

  onDestroy(() => {
    unlistenTranscription?.()
  })

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
</script>

<div class="home-view">
  <!-- Dashboard Stats Row -->
  <section class="stats-grid">
    {#if statsLoading}
      <div class="stat-card loading-card">
        <span class="stat-loading">Loading stats…</span>
      </div>
    {:else}
      <!-- Streak -->
      <div class="stat-card streak-card">
        <div class="stat-icon-wrapper">
          <Icon icon="ph:fire-duotone" class="stat-icon" />
        </div>
        <div class="stat-content">
          <span class="stat-label">Current Streak</span>
          <div class="stat-value-row">
            <span class="stat-value">{stats?.streak_days ?? 0}</span>
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
            <span class="stat-value">{(stats?.time_saved_hours ?? 0).toFixed(1)}</span>
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
            <span class="stat-value">{(stats?.total_words ?? 0).toLocaleString()}</span>
          </div>
        </div>
      </div>

      <!-- Latency (today's avg) -->
      <div class="stat-card speed-card">
        <div class="stat-icon-wrapper">
          <Icon icon="ph:lightning-duotone" class="stat-icon" />
        </div>
        <div class="stat-content">
          <span class="stat-label">Latency (avg today)</span>
          <div class="stat-value-row">
            <span class="stat-value">{stats?.today_avg_latency_ms != null ? `${stats.today_avg_latency_ms}` : '—'}</span>
            <span class="stat-unit">{stats?.today_avg_latency_ms != null ? 'ms' : ''}</span>
          </div>
        </div>
      </div>
    {/if}
  </section>

  <!-- Tasks & Reminders due today -->
  <section class="dashboard-row">
    <div class="dashboard-widget">
      <header class="widget-header">
        <Icon icon="ph:check-square-duotone" class="widget-icon" />
        <h2>Tasks due today</h2>
        {#if tasksDueToday.length > 0}
          <button class="link-btn" on:click={() => navigate('tasks')}>See all</button>
        {/if}
      </header>
      {#if tasksDueToday.length === 0}
        <p class="widget-empty">No tasks due today</p>
      {:else}
        <ul class="widget-list">
          {#each tasksDueToday.slice(0, 5) as task (task.id)}
            <li class="widget-item" role="button" tabindex="0" on:click={() => navigate('tasks')} on:keydown={(e) => e.key === 'Enter' && navigate('tasks')}>
              <span class="widget-item-title">{task.title || task.content?.slice(0, 40) || 'Task'}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
    <div class="dashboard-widget">
      <header class="widget-header">
        <Icon icon="ph:bell-duotone" class="widget-icon" />
        <h2>Reminders today</h2>
        {#if remindersDueToday.length > 0}
          <button class="link-btn" on:click={() => navigate('reminders')}>See all</button>
        {/if}
      </header>
      {#if remindersDueToday.length === 0}
        <p class="widget-empty">No reminders today</p>
      {:else}
        <ul class="widget-list">
          {#each remindersDueToday.slice(0, 5) as rem (rem.id)}
            <li class="widget-item" role="button" tabindex="0" on:click={() => navigate('reminders')} on:keydown={(e) => e.key === 'Enter' && navigate('reminders')}>
              <span class="widget-item-title">{rem.title || rem.content?.slice(0, 40) || 'Reminder'}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </section>

  <!-- Recent History (full history on History page) -->
  <section class="history-section">
    <header class="section-header">
      <div class="title-wrapper">
        <Icon icon="ph:clock-counter-clockwise-duotone" class="header-icon" />
        <h2>Recent</h2>
      </div>
      <button class="link-btn" on:click={() => navigate('history')}>See all →</button>
    </header>

    {#if loading}
      <div class="state-container">
        <Icon icon="ph:spinner-gap-duotone" class="spin-icon" />
        <p>Loading history...</p>
      </div>
    {:else if entries.length === 0}
      <div class="state-container empty-state">
        <div class="empty-visual">
          <div class="empty-mic-pulse">
            <div class="pulse-ring r1"></div>
            <div class="pulse-ring r2"></div>
            <div class="pulse-ring r3"></div>
            <div class="mic-core">
              <Icon icon="ph:microphone-stage-duotone" class="empty-icon" />
            </div>
          </div>
          <div class="sound-waves">
            <span class="wave-bar"></span>
            <span class="wave-bar"></span>
            <span class="wave-bar"></span>
            <span class="wave-bar"></span>
            <span class="wave-bar"></span>
          </div>
        </div>
        <h3>Ready to capture your voice</h3>
        <p>Press <kbd>Ctrl+Win</kbd> and speak to create your first transcription</p>
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
                <div class="history-card" role="button" tabindex="0" on:click={() => copyToClipboard(entry.text)} on:keydown={(e) => e.key === 'Enter' && copyToClipboard(entry.text)}>
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
                  
                  <div class="history-actions" on:click|stopPropagation on:keydown|stopPropagation>
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
    transform: translateY(-2px);
    box-shadow: var(--shadow-md);
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

  .streak-card .stat-icon-wrapper { background: var(--stat-streak-alpha); color: var(--stat-streak); }
  .time-card .stat-icon-wrapper { background: var(--stat-time-alpha); color: var(--stat-time); }
  .words-card .stat-icon-wrapper { background: var(--stat-words-alpha); color: var(--stat-words); }
  .speed-card .stat-icon-wrapper { background: var(--stat-speed-alpha); color: var(--stat-speed); }

  .stat-content {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .stat-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    letter-spacing: 0.01em;
  }

  .stat-value-row {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .stat-value {
    font-size: 28px;
    font-weight: 600;
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

  .loading-card {
    grid-column: 1 / -1;
    justify-content: center;
    align-items: center;
    min-height: 100px;
  }

  .stat-loading {
    font-size: 14px;
    color: var(--text-muted);
  }

  /* Dashboard widgets: tasks & reminders today */
  .dashboard-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 24px;
  }

  .dashboard-widget {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 20px;
    padding: 24px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.02);
  }

  .widget-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 16px;
    flex-wrap: wrap;
  }

  .widget-header h2 {
    font-size: 18px;
    font-weight: 600;
    color: var(--navy-deep);
    margin: 0;
    flex: 1;
  }

  .widget-icon {
    font-size: 22px;
    color: var(--primary);
  }

  /* Tasks widget uses warm amber */
  .dashboard-widget:nth-child(1) .widget-icon {
    color: var(--warm-amber);
  }

  /* Reminders widget uses soft rose */
  .dashboard-widget:nth-child(2) .widget-icon {
    color: var(--soft-rose);
  }

  .link-btn {
    background: none;
    border: none;
    color: var(--primary);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    padding: 4px 0;
  }

  .link-btn:hover {
    text-decoration: underline;
  }

  .widget-empty {
    font-size: 14px;
    color: var(--text-muted);
    margin: 0;
  }

  .widget-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .widget-item {
    font-size: 14px;
    color: var(--navy-deep);
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-subtle);
    cursor: pointer;
    border-radius: 8px;
    transition: background 0.2s;
  }

  .widget-item:hover {
    background: var(--bg-input);
  }

  .widget-item:last-child {
    border-bottom: none;
  }

  .widget-item-title {
    font-weight: 500;
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

  /* Timeline — Calendar/Agenda Style */
  .timeline {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  .day-group {
    display: flex;
    flex-direction: column;
    gap: 16px;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    padding: 20px;
    box-shadow: var(--shadow-sm);
  }

  .day-group:nth-child(odd) {
    background: linear-gradient(180deg, var(--bg-card) 0%, rgba(212, 165, 116, 0.02) 100%);
  }

  .day-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 0 12px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .day-label {
    font-size: 13px;
    font-weight: 600;
    color: var(--navy);
    white-space: nowrap;
    padding: 6px 12px;
    background: var(--primary-alpha-subtle);
    border-radius: var(--radius-sm);
  }

  .day-line {
    flex: 1;
    height: 1px;
    background: var(--border-subtle);
  }

  .day-entries {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .history-card {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    padding: 24px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.02);
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    cursor: pointer;
  }

  .history-card:hover {
    transform: translateY(-1px);
    box-shadow: var(--shadow-md);
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
    font-weight: 500;
    color: var(--text-muted);
  }

  .history-badges {
    display: flex;
    gap: 8px;
  }

  .badge {
    padding: 3px 8px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0;
  }

  .mode-badge {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-subtle);
  }

  .lang-badge {
    background: transparent;
    color: var(--text-muted);
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
    padding: 48px 24px;
  }

  /* Empty state visual illustration */
  .empty-visual {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    margin-bottom: 8px;
  }

  .empty-mic-pulse {
    position: relative;
    width: 80px;
    height: 80px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .pulse-ring {
    position: absolute;
    border-radius: 50%;
    border: 2px solid var(--warm-amber);
    opacity: 0;
  }

  .pulse-ring.r1 {
    width: 80px;
    height: 80px;
    animation: pulse-ring 2.5s ease-out infinite;
  }

  .pulse-ring.r2 {
    width: 64px;
    height: 64px;
    animation: pulse-ring 2.5s ease-out 0.4s infinite;
  }

  .pulse-ring.r3 {
    width: 48px;
    height: 48px;
    animation: pulse-ring 2.5s ease-out 0.8s infinite;
  }

  @keyframes pulse-ring {
    0% { transform: scale(0.8); opacity: 0.5; }
    50% { transform: scale(1.1); opacity: 0.2; }
    100% { transform: scale(0.8); opacity: 0.5; }
  }

  .mic-core {
    width: 56px;
    height: 56px;
    background: linear-gradient(135deg, var(--primary-alpha) 0%, var(--warm-amber-alpha) 100%);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    z-index: 1;
  }

  .empty-icon {
    font-size: 28px;
    color: var(--primary);
  }

  /* Sound wave bars */
  .sound-waves {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 24px;
  }

  .wave-bar {
    width: 4px;
    background: linear-gradient(to top, var(--warm-amber), var(--primary-light));
    border-radius: 2px;
    animation: wave-bar 1.2s ease-in-out infinite;
  }

  .wave-bar:nth-child(1) { height: 8px; animation-delay: 0s; }
  .wave-bar:nth-child(2) { height: 16px; animation-delay: 0.1s; }
  .wave-bar:nth-child(3) { height: 20px; animation-delay: 0.2s; }
  .wave-bar:nth-child(4) { height: 16px; animation-delay: 0.3s; }
  .wave-bar:nth-child(5) { height: 8px; animation-delay: 0.4s; }

  @keyframes wave-bar {
    0%, 100% { transform: scaleY(1); opacity: 0.6; }
    50% { transform: scaleY(0.6); opacity: 0.3; }
  }

  .empty-state h3 {
    font-size: 18px;
    font-weight: 600;
    color: var(--navy-deep);
    margin: 0;
  }

  .empty-state p {
    font-size: 14px;
    max-width: 320px;
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .empty-state kbd {
    background: var(--bg-input);
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    font-family: 'Google Sans', ui-monospace, monospace;
    font-size: 12px;
    font-weight: 600;
    border: 1px solid var(--border);
    color: var(--navy-deep);
    box-shadow: 0 2px 0 var(--border);
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
