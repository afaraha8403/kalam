<script lang="ts">
  import { onMount } from 'svelte'
  import { getEntriesByType, createEntry, updateEntry, deleteEntry, newEntry } from '../../lib/api/db'
  import type { Entry } from '../../types'
  import Icon from '@iconify/svelte'

  let entries: Entry[] = []
  let loading = true
  let error: string | null = null
  let newContent = ''
  let newReminderAt = ''
  let isComposerExpanded = false

  async function load() {
    loading = true
    error = null
    try {
      const result = await getEntriesByType('reminder')
      entries = Array.isArray(result) ? result : []
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    } finally {
      loading = false
    }
  }

  async function addReminder() {
    const content = newContent.trim()
    if (!content) return
    const reminderAt = newReminderAt.trim() || null
    const entry = newEntry('reminder', content, { reminder_at: reminderAt })
    try {
      await createEntry(entry)
      newContent = ''
      newReminderAt = ''
      isComposerExpanded = false
      entries = [entry, ...entries]
      error = null
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  async function toggleComplete(entry: Entry) {
    try {
      const updated = { ...entry, is_completed: !entry.is_completed, updated_at: new Date().toISOString() }
      entries = entries.map(e => e.id === entry.id ? updated : e)
      await updateEntry(updated)
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  async function remove(id: string) {
    try {
      entries = entries.filter(e => e.id !== id)
      await deleteEntry(id)
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  function formatDateTime(iso: string | null) {
    if (!iso) return 'No date set'
    try {
      const d = new Date(iso)
      const today = new Date()
      const tomorrow = new Date(today)
      tomorrow.setDate(tomorrow.getDate() + 1)

      const timeStr = d.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })

      if (d.toDateString() === today.toDateString()) {
        return `Today at ${timeStr}`
      } else if (d.toDateString() === tomorrow.toDateString()) {
        return `Tomorrow at ${timeStr}`
      }
      return d.toLocaleString(undefined, { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' })
    } catch {
      return iso
    }
  }

  function expandComposer() {
    isComposerExpanded = true
  }

  function collapseComposer() {
    if (!newContent.trim() && !newReminderAt) {
      isComposerExpanded = false
    }
  }

  $: activeReminders = entries.filter(e => !e.is_completed)
  $: completedReminders = entries.filter(e => e.is_completed)

  onMount(() => load())
</script>

<div class="view reminders-view">
  <header class="page-header">
    <div class="header-content">
      <div class="title-wrapper">
        <Icon icon="ph:bell-ringing-duotone" class="header-icon" />
        <h2>Reminders</h2>
      </div>
      <p class="subtitle">Never forget an important detail or follow-up.</p>
    </div>
  </header>

  <div class="composer-container" class:expanded={isComposerExpanded}>
    <div class="composer-card">
      <div class="input-wrapper">
        <Icon icon="ph:plus-bold" class="input-icon" />
        <input 
          type="text" 
          bind:value={newContent} 
          placeholder="Remind me to..." 
          class="content-input"
          on:focus={expandComposer}
          on:keydown={(e) => e.key === 'Enter' && addReminder()}
        />
      </div>
      
      {#if isComposerExpanded}
        <div class="composer-details">
          <div class="datetime-wrapper">
            <Icon icon="ph:calendar-plus-duotone" class="detail-icon" />
            <input 
              type="datetime-local" 
              bind:value={newReminderAt} 
              class="datetime-input" 
            />
          </div>
        </div>
        <div class="composer-actions">
          <button type="button" class="btn-ghost" on:click={() => { newContent=''; newReminderAt=''; isComposerExpanded=false; }}>Cancel</button>
          <button type="button" class="btn-primary" on:click={addReminder} disabled={!newContent.trim()}>
            <Icon icon="ph:bell-plus-bold" /> Add Reminder
          </button>
        </div>
      {/if}
    </div>
  </div>

  {#if error}
    <div class="state-container error-state">
      <Icon icon="ph:warning-circle-duotone" class="error-icon" />
      <p>{error}</p>
    </div>
  {/if}

  {#if loading && entries.length === 0}
    <div class="state-container">
      <Icon icon="ph:spinner-gap-duotone" class="spin-icon" />
      <p>Loading reminders...</p>
    </div>
  {:else if entries.length === 0}
    <div class="state-container empty-state">
      <div class="empty-icon-wrapper">
        <Icon icon="ph:bell-z-duotone" class="empty-icon" />
      </div>
      <h3>No reminders</h3>
      <p>You're all clear! Add a new reminder above to get notified.</p>
    </div>
  {:else}
    <div class="reminder-sections">
      {#if activeReminders.length > 0}
        <div class="reminder-list">
          {#each activeReminders as entry (entry.id)}
            <div class="reminder-item">
              <button class="checkbox" on:click={() => toggleComplete(entry)}>
                <div class="check-circle">
                  <Icon icon="ph:check-bold" class="check-icon" />
                </div>
              </button>
              <div class="reminder-content">
                <span class="reminder-title">{entry.content}</span>
                <div class="reminder-meta">
                  <span class="reminder-time" class:has-date={entry.reminder_at}>
                    <Icon icon="ph:clock-duotone" />
                    {formatDateTime(entry.reminder_at)}
                  </span>
                  {#if entry.rrule}
                    <span class="reminder-repeat">
                      <Icon icon="ph:arrows-clockwise-duotone" />
                      {entry.rrule}
                    </span>
                  {/if}
                </div>
              </div>
              <div class="reminder-actions">
                <button class="action-btn delete" on:click={() => remove(entry.id)} title="Delete reminder">
                  <Icon icon="ph:trash-duotone" />
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}

      {#if completedReminders.length > 0}
        <div class="completed-section">
          <div class="section-header">
            <h3>Completed</h3>
            <span class="count">{completedReminders.length}</span>
          </div>
          <div class="reminder-list completed">
            {#each completedReminders as entry (entry.id)}
              <div class="reminder-item is-completed">
                <button class="checkbox" on:click={() => toggleComplete(entry)}>
                  <div class="check-circle checked">
                    <Icon icon="ph:check-bold" class="check-icon" />
                  </div>
                </button>
                <div class="reminder-content">
                  <span class="reminder-title">{entry.content}</span>
                </div>
                <div class="reminder-actions">
                  <button class="action-btn delete" on:click={() => remove(entry.id)} title="Delete reminder">
                    <Icon icon="ph:trash-duotone" />
                  </button>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}
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

  /* Composer */
  .composer-container {
    position: relative;
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .composer-card {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.02);
    overflow: hidden;
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    display: flex;
    flex-direction: column;
  }

  .composer-container.expanded .composer-card {
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.06), 0 2px 8px rgba(0, 0, 0, 0.02);
    border-color: var(--primary);
    transform: translateY(-2px);
  }

  .input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .input-icon {
    position: absolute;
    left: 20px;
    font-size: 20px;
    color: var(--primary);
    pointer-events: none;
  }

  .content-input {
    width: 100%;
    border: none;
    padding: 16px 20px 16px 52px;
    font-size: 16px;
    color: var(--text-primary);
    background: transparent;
    font-family: inherit;
  }

  .content-input:focus {
    outline: none;
  }

  .composer-details {
    padding: 0 20px 16px 52px;
    display: flex;
    gap: 12px;
  }

  .datetime-wrapper {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-input);
    padding: 8px 12px;
    border-radius: 10px;
    border: 1px solid var(--border-subtle);
  }

  .detail-icon {
    color: var(--text-muted);
    font-size: 18px;
  }

  .datetime-input {
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-family: inherit;
    font-size: 14px;
    font-weight: 500;
    outline: none;
  }

  .composer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    background: var(--bg-app);
    border-top: 1px solid var(--border-subtle);
  }

  .btn-primary {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    background: var(--primary);
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--primary-dark);
    transform: translateY(-1px);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-ghost {
    padding: 8px 16px;
    background: transparent;
    color: var(--text-secondary);
    border: none;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-ghost:hover {
    background: var(--bg-input);
    color: var(--navy-deep);
  }

  /* List */
  .reminder-sections {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  .reminder-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .reminder-item {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    padding: 20px;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    transition: all 0.2s ease;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.01);
  }

  .reminder-item:hover {
    border-color: var(--border-visible);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.04);
    transform: translateY(-2px);
  }

  .checkbox {
    background: none;
    border: none;
    padding: 2px 0 0 0;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .check-circle {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 2px solid var(--border-visible);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
    color: transparent;
  }

  .checkbox:hover .check-circle {
    border-color: var(--primary);
    background: var(--primary-alpha);
  }

  .check-circle.checked {
    background: var(--primary);
    border-color: var(--primary);
    color: white;
  }

  .check-icon {
    font-size: 14px;
  }

  .reminder-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .reminder-title {
    font-size: 16px;
    color: var(--navy-deep);
    font-weight: 600;
    line-height: 1.4;
  }

  .reminder-meta {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }

  .reminder-time, .reminder-repeat {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    background: var(--bg-input);
    padding: 4px 8px;
    border-radius: 6px;
  }

  .reminder-time.has-date {
    color: var(--primary-dark);
    background: var(--primary-alpha-light);
    border: 1px solid var(--primary-alpha);
  }

  .reminder-actions {
    opacity: 0;
    transition: opacity 0.2s;
  }

  .reminder-item:hover .reminder-actions {
    opacity: 1;
  }

  .action-btn {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    border: none;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    color: var(--text-muted);
    transition: all 0.2s;
  }

  .action-btn:hover {
    background: var(--bg-input);
    color: var(--navy-deep);
  }

  .action-btn.delete:hover {
    color: var(--error);
    background: rgba(239, 68, 68, 0.1);
  }

  /* Completed Section */
  .completed-section {
    opacity: 0.8;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
    padding-left: 16px;
  }

  .section-header h3 {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0;
  }

  .count {
    background: var(--bg-input);
    color: var(--text-secondary);
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 700;
  }

  .reminder-item.is-completed {
    background: transparent;
    border-color: transparent;
    box-shadow: none;
  }

  .reminder-item.is-completed:hover {
    transform: none;
    background: var(--bg-card);
    border-color: var(--border-subtle);
  }

  .reminder-item.is-completed .reminder-title {
    text-decoration: line-through;
    color: var(--text-muted);
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
    margin: 0;
  }

  .error-state {
    border-color: rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.02);
    padding: 24px;
    flex-direction: row;
    color: var(--error);
  }

  @media (max-width: 768px) {
    .reminder-actions {
      opacity: 1;
    }
    .subtitle {
      padding-left: 0;
    }
    .composer-details {
      padding-left: 20px;
    }
  }
</style>
