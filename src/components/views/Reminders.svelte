<script lang="ts">
  import { onMount } from 'svelte'
  import { getEntriesWithReminder, updateEntry, deleteEntry } from '../../lib/api/db'
  import type { Entry } from '../../types'
  import Icon from '@iconify/svelte'
  import { selectedTaskId } from '../../lib/taskDetailStore'
  import { selectedNoteId } from '../../lib/noteDetailStore'
  import { selectedReminderId } from '../../lib/reminderDetailStore'
  export let navigate: (page: string) => void = () => {}

  let entries: Entry[] = []
  let loading = true
  let error: string | null = null
  let searchQuery = ''

  function formatReminder(iso: string | null) {
    if (!iso) return ''
    const d = new Date(iso)
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' })
  }

  function openNewReminder() {
    selectedReminderId.set(null)
    navigate('reminder-detail')
  }

  function handleRowClick(entry: Entry) {
    if (entry.entry_type === 'reminder') {
      selectedReminderId.set(entry.id)
      navigate('reminder-detail')
    } else if (entry.entry_type === 'note') {
      selectedNoteId.set(entry.id)
      navigate('note-detail')
    } else if (entry.entry_type === 'task') {
      selectedTaskId.set(entry.id)
      navigate('task-detail')
    }
  }

  async function load() {
    loading = true
    error = null
    try {
      const result = await getEntriesWithReminder()
      entries = Array.isArray(result) ? result : []
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    } finally {
      loading = false
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

  async function snooze(entry: Entry) {
    const inOneHour = new Date(Date.now() + 60 * 60 * 1000).toISOString()
    const updated = { ...entry, reminder_at: inOneHour, updated_at: new Date().toISOString() }
    try {
      await updateEntry(updated)
      entries = entries.map((e) => (e.id === entry.id ? updated : e))
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  async function clearReminder(entry: Entry) {
    if (entry.entry_type === 'reminder') return
    try {
      const updated = { ...entry, reminder_at: null, updated_at: new Date().toISOString() }
      await updateEntry(updated)
      entries = entries.filter((e) => e.id !== entry.id)
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  function reminderTitle(entry: Entry): string {
    return entry.title?.trim() || entry.content?.trim() || '(no title)'
  }

  function sourceBadgeLabel(entry: Entry): string {
    if (entry.entry_type === 'reminder') return 'Reminder'
    if (entry.entry_type === 'note') return 'Note'
    if (entry.entry_type === 'task') return 'Task'
    return 'Reminder'
  }

  $: filteredEntries = entries.filter(e => {
    const matchesSearch = (e.title || '').toLowerCase().includes(searchQuery.toLowerCase()) || 
                          (e.content || '').toLowerCase().includes(searchQuery.toLowerCase())
    return matchesSearch
  })

  onMount(load)
</script>

<div class="page fade-in">
  <header class="page-header notes-header">
    <div>
      <h1 class="page-title">Reminders</h1>
      <p class="page-subtitle">Never forget the important things.</p>
    </div>
    <button type="button" class="btn-primary" on:click={openNewReminder}>
      <Icon icon="ph:plus" />
      New Reminder
    </button>
  </header>

  <div class="notes-search-bar">
    <span class="notes-search-bar-icon" aria-hidden="true">
      <Icon icon="ph:magnifying-glass" />
    </span>
    <input type="text" placeholder="Search reminders..." bind:value={searchQuery} />
  </div>

  {#if error}
    <div class="state-container empty-state">
      <Icon icon="ph:warning-circle" />
      <p>{error}</p>
    </div>
  {:else if loading && entries.length === 0}
    <div class="state-container empty-state">
      <Icon icon="ph:spinner-gap" class="spin-icon" />
      <p>Loading reminders...</p>
    </div>
  {:else}
    <div class="reminder-list-large">
      {#each filteredEntries as entry (entry.id)}
        <div
          class="reminder-row"
          class:from-note={entry.entry_type === 'note'}
          class:from-task={entry.entry_type === 'task'}
          on:click={() => handleRowClick(entry)}
          role="button"
          tabindex="0"
          on:keydown={(e) => e.key === 'Enter' && handleRowClick(entry)}
        >
          <div class="reminder-icon-large" class:recurring={!!entry.rrule}>
            {#if entry.entry_type === 'note'}
              <Icon icon="ph:note" />
            {:else if entry.entry_type === 'task'}
              <Icon icon="ph:check-circle" />
            {:else}
              <Icon icon={entry.rrule ? 'ph:arrows-clockwise' : 'ph:bell'} />
            {/if}
          </div>
          <div class="reminder-info">
            <div class="reminder-title-row">
              <span class="reminder-text">{reminderTitle(entry)}</span>
              <span class="reminder-source-badge" class:note={entry.entry_type === 'note'} class:task={entry.entry_type === 'task'}>
                {sourceBadgeLabel(entry)}
              </span>
            </div>
            <div class="reminder-meta">
              {#if entry.reminder_at}
                <span class="reminder-time">
                  <Icon icon="ph:clock" />
                  {formatReminder(entry.reminder_at)}
                </span>
              {/if}
              {#if entry.rrule}
                <span class="reminder-recurring-badge">
                  <Icon icon="ph:repeat" />
                  Recurring
                </span>
              {/if}
              {#if entry.tags && entry.tags.length > 0}
                <div class="reminder-tags">
                  {#each entry.tags as tag}
                    <span class="reminder-tag">{tag}</span>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        </div>
      {/each}
      {#if filteredEntries.length === 0}
        <div class="empty-state">
          <Icon icon="ph:bell-slash" />
          <p>{searchQuery ? 'No reminders match your search' : 'No reminders yet'}</p>
        </div>
      {/if}
    </div>
  {/if}
</div>
