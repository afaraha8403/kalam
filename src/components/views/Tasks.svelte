<script lang="ts">
  import { onMount } from 'svelte'
  import { fade } from 'svelte/transition'
  import { getEntriesByType, createEntry, updateEntry, deleteEntry, newEntry } from '../../lib/api/db'
  import type { Entry, Subtask } from '../../types'
  import Icon from '@iconify/svelte'
  import { selectedTaskId as selectedTaskIdStore } from '../../lib/taskDetailStore'
  import { marked } from 'marked'
  import DOMPurify from 'dompurify'
  import SidePanel from '../ui/SidePanel.svelte'
  import SearchFilterBar from '../ui/SearchFilterBar.svelte'

  export let navigate: ((page: string) => void) | undefined = undefined

  let entries: Entry[] = []
  let loading = true
  let error: string | null = null
  
  let searchQuery = ''
  let filterMode: 'all' | 'active' | 'completed' = 'all'

  // Panel State
  let isPanelOpen = false
  let panelMode: 'add' | 'edit' = 'add'
  let panelTaskId: string | null = null
  let draftEntry: Partial<Entry> = {}
  
  let detailsPreviewActive = false

  async function load() {
    loading = true
    error = null
    try {
      const result = await getEntriesByType('task')
      entries = Array.isArray(result) ? result : []
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    } finally {
      loading = false
    }
  }

  function openAddPanel() {
    panelMode = 'add'
    panelTaskId = null
    draftEntry = {
      title: '',
      content: '',
      due_date: null,
      reminder_at: null,
      subtasks: []
    }
    isPanelOpen = true
  }

  function openEditPanel(task: Entry) {
    panelMode = 'edit'
    panelTaskId = task.id
    draftEntry = { ...task }
    isPanelOpen = true
  }

  function closePanel() {
    isPanelOpen = false
    panelTaskId = null
  }

  async function savePanel() {
    if (!draftEntry.title?.trim()) {
      error = "Title is required"
      return
    }

    try {
      if (panelMode === 'add') {
        const entry = newEntry('task', draftEntry.content || '', { 
          title: draftEntry.title,
          due_date: draftEntry.due_date,
          reminder_at: draftEntry.reminder_at,
          subtasks: draftEntry.subtasks
        })
        await createEntry(entry)
        entries = [entry, ...entries]
      } else if (panelMode === 'edit' && panelTaskId) {
        const updated = { ...draftEntry, id: panelTaskId, updated_at: new Date().toISOString() } as Entry
        await updateEntry(updated)
        entries = entries.map(e => e.id === updated.id ? updated : e)
      }
      closePanel()
      error = null
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
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
      if (panelTaskId === id) closePanel()
      await deleteEntry(id)
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  function formatDate(iso: string | null) {
    if (!iso) return ''
    try {
      return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
    } catch {
      return iso
    }
  }

  function formatDateTimeLocal(iso: string | null) {
    if (!iso) return ''
    try {
      const d = new Date(iso)
      const y = d.getFullYear()
      const m = String(d.getMonth() + 1).padStart(2, '0')
      const day = String(d.getDate()).padStart(2, '0')
      const h = String(d.getHours()).padStart(2, '0')
      const min = String(d.getMinutes()).padStart(2, '0')
      return `${y}-${m}-${day}T${h}:${min}`
    } catch {
      return ''
    }
  }

  function toISOStartOfDay(dateStr: string) {
    if (!dateStr) return null
    const d = new Date(dateStr + 'T00:00:00')
    return isNaN(d.getTime()) ? null : d.toISOString()
  }

  function toISODateTime(dateTimeStr: string) {
    if (!dateTimeStr) return null
    const d = new Date(dateTimeStr)
    return isNaN(d.getTime()) ? null : d.toISOString()
  }

  function getInputValue(e: Event): string {
    const el = e.currentTarget as HTMLInputElement | HTMLTextAreaElement | null
    return el?.value ?? ''
  }

  function addSubtask() {
    draftEntry.subtasks = [...(draftEntry.subtasks || []), { title: '', is_completed: false }]
  }

  function toggleSubtask(index: number) {
    if (!draftEntry.subtasks) return
    draftEntry.subtasks = draftEntry.subtasks.map((s, i) => 
      i === index ? { ...s, is_completed: !s.is_completed } : s
    )
  }

  function removeSubtask(index: number) {
    if (!draftEntry.subtasks) return
    draftEntry.subtasks = draftEntry.subtasks.filter((_, i) => i !== index)
  }

  function renderDetailsMarkdown(content: string): string {
    if (!content?.trim()) return ''
    const raw = marked.parse(content, { async: false }) as string
    return DOMPurify.sanitize(raw)
  }

  $: filteredEntries = entries.filter(e => {
    const matchesSearch = (e.title || '').toLowerCase().includes(searchQuery.toLowerCase()) || 
                          (e.content || '').toLowerCase().includes(searchQuery.toLowerCase())
    if (!matchesSearch) return false
    
    if (filterMode === 'active') return !e.is_completed
    if (filterMode === 'completed') return e.is_completed
    return true
  })

  $: activeTasks = filteredEntries.filter((e) => !e.is_completed)
  $: completedTasks = filteredEntries.filter((e) => e.is_completed)

  onMount(() => {
    load()
    const unsub = selectedTaskIdStore.subscribe((id) => {
      if (id) {
        const task = entries.find(e => e.id === id)
        if (task) openEditPanel(task)
        selectedTaskIdStore.set(null)
      }
    })
    return () => unsub()
  })
</script>

<div class="view tasks-view">
  <header class="page-header">
    <div class="header-content">
      <div class="title-wrapper">
        <Icon icon="ph:check-square-offset-duotone" class="header-icon" />
        <h2>Tasks</h2>
      </div>
      <p class="subtitle">Stay on top of what needs to be done.</p>
    </div>
    <div class="header-actions">
      {#if navigate}
        <button type="button" class="btn-ghost" on:click={() => navigate('home')} title="Home">Home</button>
      {/if}
      <button class="btn-primary" on:click={openAddPanel}>
        <Icon icon="ph:plus-bold" /> Add Task
      </button>
    </div>
  </header>

  <SearchFilterBar bind:searchQuery placeholder="Search tasks...">
    <svelte:fragment slot="filters">
      <button class="filter-chip" class:active={filterMode === 'all'} on:click={() => filterMode = 'all'}>All</button>
      <button class="filter-chip" class:active={filterMode === 'active'} on:click={() => filterMode = 'active'}>Active</button>
      <button class="filter-chip" class:active={filterMode === 'completed'} on:click={() => filterMode = 'completed'}>Completed</button>
    </svelte:fragment>
  </SearchFilterBar>

  {#if error}
    <div class="state-container error-state">
      <Icon icon="ph:warning-circle-duotone" class="error-icon" />
      <p>{error}</p>
    </div>
  {/if}

  {#if loading && entries.length === 0}
    <div class="state-container">
      <Icon icon="ph:spinner-gap-duotone" class="spin-icon" />
      <p>Loading tasks...</p>
    </div>
  {:else if entries.length === 0}
    <div class="state-container empty-state">
      <div class="empty-icon-wrapper">
        <Icon icon="ph:check-circle-duotone" class="empty-icon" />
      </div>
      <h3>All caught up!</h3>
      <p>You have no tasks. Enjoy your day or add a new one above.</p>
    </div>
  {:else if filteredEntries.length === 0}
    <div class="state-container empty-state">
      <div class="empty-icon-wrapper">
        <Icon icon="ph:magnifying-glass-duotone" class="empty-icon" />
      </div>
      <h3>No results found</h3>
      <p>Try adjusting your search or filters.</p>
    </div>
  {:else}
    <div class="task-sections">
      {#if activeTasks.length > 0}
        <div class="task-list">
          {#each activeTasks as entry (entry.id)}
            <div class="task-item" role="button" tabindex="0" on:click={() => openEditPanel(entry)} on:keydown={(e) => e.key === 'Enter' && openEditPanel(entry)}>
              <button class="checkbox" on:click|stopPropagation={() => toggleComplete(entry)}>
                <div class="check-circle">
                  <Icon icon="ph:check-bold" class="check-icon" />
                </div>
              </button>
              <div class="task-content">
                <span class="task-title">{entry.title || entry.content}</span>
                <div class="task-meta">
                  {#if entry.due_date}
                    <span class="task-due">
                      <Icon icon="ph:calendar-blank-duotone" />
                      {formatDate(entry.due_date)}
                    </span>
                  {/if}
                  {#if entry.reminder_at}
                    <span class="task-reminder">
                      <Icon icon="ph:bell-duotone" />
                      {formatDate(entry.reminder_at)}
                    </span>
                  {/if}
                  {#if entry.subtasks && entry.subtasks.length > 0}
                    <span class="task-subtask-count">
                      {entry.subtasks.filter((s) => s.is_completed).length}/{entry.subtasks.length}
                    </span>
                  {/if}
                </div>
              </div>
              <div class="task-actions" on:click|stopPropagation on:keydown|stopPropagation>
                <button class="action-btn delete" on:click={() => remove(entry.id)} title="Delete task">
                  <Icon icon="ph:trash-duotone" />
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}

      {#if completedTasks.length > 0}
        <div class="completed-section">
          <div class="section-header">
            <h3>Completed</h3>
            <span class="count">{completedTasks.length}</span>
          </div>
          <div class="task-list completed">
            {#each completedTasks as entry (entry.id)}
              <div class="task-item is-completed" role="button" tabindex="0" on:click={() => openEditPanel(entry)} on:keydown={(e) => e.key === 'Enter' && openEditPanel(entry)}>
                <button class="checkbox" on:click|stopPropagation={() => toggleComplete(entry)}>
                  <div class="check-circle checked">
                    <Icon icon="ph:check-bold" class="check-icon" />
                  </div>
                </button>
                <div class="task-content">
                  <span class="task-title">{entry.title || entry.content}</span>
                </div>
                <div class="task-actions" on:click|stopPropagation on:keydown|stopPropagation>
                  <button class="action-btn delete" on:click={() => remove(entry.id)} title="Delete task">
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

  <SidePanel 
    isOpen={isPanelOpen} 
    title={panelMode === 'add' ? 'Add Task' : 'Edit Task'} 
    on:close={closePanel}
  >
    <div slot="body" class="panel-form">
      <div class="field">
        <label for="task-detail-title">Title</label>
        <input
          id="task-detail-title"
          type="text"
          bind:value={draftEntry.title}
          placeholder="Task title"
        />
        {#if (draftEntry.title ?? '').trim() === ''}
          <span class="field-hint">Title is required</span>
        {/if}
      </div>
      <div class="field">
        <label for="task-details">Details (markdown)</label>
        <div class="details-tabs">
          <button type="button" class:active={!detailsPreviewActive} on:click={() => (detailsPreviewActive = false)}>Edit</button>
          <button type="button" class:active={detailsPreviewActive} on:click={() => (detailsPreviewActive = true)}>Preview</button>
        </div>
        {#if !detailsPreviewActive}
          <textarea
            id="task-details"
            class="details-textarea"
            bind:value={draftEntry.content}
            placeholder="Add details (markdown supported)"
            rows="6"
          ></textarea>
        {:else}
          <div class="details-preview" data-details-preview>
            {@html renderDetailsMarkdown(draftEntry.content || '')}
          </div>
        {/if}
      </div>
      <div class="field row">
        <div class="field-half">
          <label for="task-due">Due date</label>
          <input
            id="task-due"
            type="date"
            value={draftEntry.due_date ? draftEntry.due_date.slice(0, 10) : ''}
            on:change={(e) => draftEntry.due_date = toISOStartOfDay(getInputValue(e))}
          />
        </div>
        <div class="field-half">
          <label for="task-reminder">Reminder</label>
          <input
            id="task-reminder"
            type="datetime-local"
            value={formatDateTimeLocal(draftEntry.reminder_at || null)}
            on:change={(e) => draftEntry.reminder_at = toISODateTime(getInputValue(e))}
          />
        </div>
      </div>
      <div class="field">
        <label for="task-subtasks-list">Subtasks</label>
        <div id="task-subtasks-list" class="subtasks-list" role="group">
          {#each draftEntry.subtasks ?? [] as subtask, i}
            <div class="subtask-row">
              <button type="button" class="subtask-checkbox" on:click={() => toggleSubtask(i)} aria-label="Toggle subtask">
                <div class="check-circle" class:checked={subtask.is_completed}>
                  {#if subtask.is_completed}
                    <Icon icon="ph:check-bold" class="check-icon" />
                  {/if}
                </div>
              </button>
              <input
                type="text"
                class="subtask-input"
                bind:value={subtask.title}
                placeholder="Subtask"
              />
              <button type="button" class="action-btn delete subtask-delete" on:click={() => removeSubtask(i)} title="Remove subtask">
                <Icon icon="ph:trash-duotone" />
              </button>
            </div>
          {/each}
          <button type="button" class="add-subtask-btn" on:click={addSubtask}>
            <Icon icon="ph:plus-duotone" />
            Add subtask
          </button>
        </div>
      </div>
    </div>
    
    <div slot="footer">
      <button class="btn-primary" on:click={savePanel} disabled={!draftEntry.title?.trim()}>
        <Icon icon="ph:check-bold" /> Save
      </button>
      <button class="btn-ghost" on:click={closePanel}>Cancel</button>
    </div>
  </SidePanel>
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
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: 16px;
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

  .header-actions {
    display: flex;
    gap: 12px;
  }

  .btn-primary {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 24px;
    background: var(--primary);
    color: white;
    border: none;
    border-radius: 10px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
    box-shadow: 0 4px 12px var(--primary-alpha);
  }

  .btn-primary:hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow: 0 6px 16px var(--primary-alpha);
    background: var(--primary-dark);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    box-shadow: none;
  }

  .btn-ghost {
    padding: 12px 20px;
    background: transparent;
    color: var(--text-secondary);
    border: none;
    border-radius: 10px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-ghost:hover {
    background: var(--bg-input);
    color: var(--navy-deep);
  }

  .filter-chip {
    padding: 6px 12px;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-card);
    font-size: 13px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s;
  }

  .filter-chip:hover,
  .filter-chip.active {
    background: var(--primary);
    color: white;
    border-color: var(--primary);
  }

  /* Task List */
  .task-sections {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  .task-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .task-item {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    transition: all 0.2s ease;
    cursor: pointer;
  }

  .task-item:hover {
    border-color: var(--border-visible);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.03);
  }

  .checkbox {
    background: none;
    border: none;
    padding: 0;
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

  .task-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .task-title {
    font-size: 16px;
    color: var(--navy-deep);
    font-weight: 500;
    transition: all 0.2s;
  }

  .task-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
  }

  .task-due,
  .task-reminder,
  .task-subtask-count {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--text-muted);
    font-weight: 600;
  }

  .task-subtask-count {
    font-weight: 500;
  }

  .task-actions {
    opacity: 0;
    transition: opacity 0.2s;
  }

  .task-item:hover .task-actions {
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

  .task-item.is-completed {
    background: transparent;
    border-color: transparent;
    box-shadow: none;
  }

  .task-item.is-completed .task-title {
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

  /* Panel Form */
  .panel-form {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .field label {
    display: block;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }

  .field input[type="text"],
  .field input[type="date"],
  .field input[type="datetime-local"],
  .field textarea {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    font-size: 15px;
    font-family: inherit;
    background: var(--bg-input);
    color: var(--text-primary);
    transition: all 0.2s;
  }

  .field input[type="date"]::-webkit-calendar-picker-indicator,
  .field input[type="datetime-local"]::-webkit-calendar-picker-indicator {
    cursor: pointer;
    opacity: 0.6;
    transition: 0.2s;
  }

  .field input[type="date"]::-webkit-calendar-picker-indicator:hover,
  .field input[type="datetime-local"]::-webkit-calendar-picker-indicator:hover {
    opacity: 1;
  }

  .field input:focus, .field textarea:focus {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 3px var(--primary-alpha);
  }

  .field-hint {
    font-size: 12px;
    color: var(--text-muted);
    margin-top: 4px;
    display: block;
  }

  .field.row {
    display: flex;
    gap: 12px;
  }

  .field-half {
    flex: 1;
  }

  .details-tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 8px;
  }

  .details-tabs button {
    padding: 6px 12px;
    font-size: 13px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-input);
    border-radius: 8px;
    cursor: pointer;
    color: var(--text-secondary);
  }

  .details-tabs button.active {
    background: var(--primary);
    color: white;
    border-color: var(--primary);
  }

  .details-textarea {
    min-height: 120px;
    resize: vertical;
  }

  .details-preview {
    min-height: 120px;
    padding: 12px;
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    background: var(--bg-input);
    font-size: 14px;
    line-height: 1.5;
  }

  .details-preview :global(p) { margin: 0 0 0.5em; }
  .details-preview :global(ul) { margin: 0 0 0.5em; padding-left: 1.2em; }
  .details-preview :global(ol) { margin: 0 0 0.5em; padding-left: 1.2em; }
  .details-preview :global(h1), .details-preview :global(h2), .details-preview :global(h3) { margin: 0.6em 0 0.3em; font-size: 1em; }

  .subtasks-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .subtask-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-left: 12px;
    border-left: 3px solid var(--border-visible);
  }

  .subtask-checkbox {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    flex-shrink: 0;
  }

  .subtask-row .check-circle {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 2px solid var(--border-visible);
    display: flex;
    align-items: center;
    justify-content: center;
    color: transparent;
  }

  .subtask-row .check-circle.checked {
    background: var(--primary);
    border-color: var(--primary);
    color: white;
  }

  .subtask-input {
    flex: 1;
    padding: 8px 10px;
    font-size: 14px;
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    background: var(--bg-input);
  }

  .subtask-delete {
    width: 28px;
    height: 28px;
    flex-shrink: 0;
  }

  .add-subtask-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    font-size: 14px;
    color: var(--primary);
    background: transparent;
    border: 1px dashed var(--border-visible);
    border-radius: 8px;
    cursor: pointer;
    margin-top: 4px;
  }

  .add-subtask-btn:hover {
    background: var(--primary-alpha);
  }

  @media (max-width: 768px) {
    .task-actions {
      opacity: 1;
    }
    .subtitle {
      padding-left: 0;
    }
  }
</style>
