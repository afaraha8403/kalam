<script lang="ts">
  import { onMount } from 'svelte'
  import { getEntriesByType, updateEntry, deleteEntry } from '../../lib/api/db'
  import type { Entry } from '../../types'
  import Icon from '@iconify/svelte'
  import { selectedTaskId } from '../../lib/taskDetailStore'

  export let navigate: (page: string) => void = () => {}

  let entries: Entry[] = []
  let loading = true
  let error: string | null = null
  let searchQuery = ''
  /** For drag placeholder; reorder persistence can be wired later if API supports it. */
  let dragTaskId: string | null = null

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

  function getPriorityColor(p: number | null | undefined): string {
    if (p == null || p < 1) return '#8E8E93'
    return ['#34C759', '#FF9500', '#FF3B30'][p - 1] ?? '#8E8E93'
  }

  function formatReminderShort(iso: string | null | undefined): string {
    if (!iso) return ''
    const d = new Date(iso)
    const today = new Date()
    const isToday = d.toDateString() === today.toDateString()
    if (isToday) return d.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  }

  function openNewTask() {
    selectedTaskId.set(null)
    navigate('task-detail')
  }

  function openTask(task: Entry) {
    selectedTaskId.set(task.id)
    navigate('task-detail')
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

  function handleTaskDragStart(_e: DragEvent, id: string) {
    dragTaskId = id
  }
  function handleTaskDragOver(e: DragEvent) {
    e.preventDefault()
  }
  function handleTaskDrop(_e: DragEvent, _id: string) {
    dragTaskId = null
    // TODO: persist reorder when API supports task order
  }
  function handleTaskDragEnd() {
    dragTaskId = null
  }

  $: filteredEntries = entries.filter(e => {
    return (e.title || '').toLowerCase().includes(searchQuery.toLowerCase()) ||
      (e.content || '').toLowerCase().includes(searchQuery.toLowerCase())
  })

  /** Active first, then completed — prototype order. */
  $: tasksDisplayOrder = [...filteredEntries].sort((a, b) => (a.is_completed === b.is_completed ? 0 : a.is_completed ? 1 : -1))

  onMount(load)
</script>

<div class="page fade-in">
  <header class="page-header notes-header">
    <div>
      <h1 class="page-title">Tasks</h1>
      <p class="page-subtitle">Track what needs to get done.</p>
    </div>
    <button type="button" class="btn-primary" on:click={openNewTask}>
      <Icon icon="ph:plus" />
      New Task
    </button>
  </header>

  <div class="notes-search-bar">
    <span class="notes-search-bar-icon" aria-hidden="true">
      <Icon icon="ph:magnifying-glass" />
    </span>
    <input type="text" placeholder="Search tasks..." bind:value={searchQuery} />
  </div>

  {#if error}
    <div class="state-container empty-state">
      <Icon icon="ph:warning-circle" />
      <p>{error}</p>
    </div>
  {:else if loading && entries.length === 0}
    <div class="state-container empty-state">
      <Icon icon="ph:spinner-gap" class="spin-icon" />
      <p>Loading tasks...</p>
    </div>
  {:else}
    <div class="task-list-large">
      {#each tasksDisplayOrder as task (task.id)}
        <div
          class="task-row"
          class:completed={task.is_completed}
          class:dragging={dragTaskId === task.id}
          draggable="true"
          on:dragstart={(e) => handleTaskDragStart(e, task.id)}
          on:dragover={handleTaskDragOver}
          on:drop={(e) => handleTaskDrop(e, task.id)}
          on:dragend={handleTaskDragEnd}
          on:click={() => openTask(task)}
          role="button"
          tabindex="0"
          on:keydown={(e) => e.key === 'Enter' && openTask(task)}
        >
          <button type="button" class="drag-handle" title="Drag to reorder" on:click|stopPropagation>
            <Icon icon="ph:dots-six-vertical" />
          </button>
          <button type="button" class="checkbox" on:click|stopPropagation={() => toggleComplete(task)}>
            {#if task.is_completed}
              <Icon icon="ph:check" />
            {/if}
          </button>
          <div class="task-info">
            <span class="task-title">{task.title || task.content || 'Untitled'}</span>
            <div class="task-meta">
              {#if task.due_date}
                <span class="task-due" class:urgent={new Date(task.due_date).toDateString() === new Date().toDateString()}>
                  <Icon icon="ph:calendar-blank" />
                  {formatReminderShort(task.due_date)}
                </span>
              {/if}
              {#if task.subtasks && task.subtasks.length > 0}
                <span class="task-subtasks-count">
                  <Icon icon="ph:list-checks" />
                  {task.subtasks.filter(s => s.is_completed).length}/{task.subtasks.length}
                </span>
              {/if}
            </div>
          </div>
          {#if task.tags && task.tags.length > 0}
            <div class="task-tags">
              {#each task.tags as tag}
                <span class="task-tag-pill">{tag}</span>
              {/each}
            </div>
          {/if}
          {#if task.priority != null && task.priority > 0}
            <div class="priority-indicator" style="background: {getPriorityColor(task.priority)}"></div>
          {/if}
        </div>
      {/each}
      {#if tasksDisplayOrder.length === 0}
        <div class="empty-state">
          <Icon icon="ph:check-circle" />
          <p>{searchQuery ? 'No tasks match your search' : 'All caught up!'}</p>
        </div>
      {/if}
    </div>
  {/if}
</div>
