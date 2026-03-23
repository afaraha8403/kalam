<script lang="ts">
  import { get } from 'svelte/store'
  import { onMount } from 'svelte'
  import { getEntry, createEntry, updateEntry, deleteEntry, newEntry } from '../../lib/api/db'
  import type { Entry, Subtask } from '../../types'
  import Icon from '@iconify/svelte'
  import { selectedTaskId } from '../../lib/taskDetailStore'
  import { selectedHistoryId } from '../../lib/historyDetailStore'
  import { taskDetailReturnTo } from '../../lib/detailReturnStore'

  export let navigate: (page: string) => void = () => {}

  let taskId: string | null = null
  let draft: {
    title: string
    content: string
    due_date: string
    reminder_at: string
    priority: number | null
    is_completed: boolean
    tags: string[]
    subtasks: Subtask[]
  } = {
    title: '',
    content: '',
    due_date: '',
    reminder_at: '',
    priority: null,
    is_completed: false,
    tags: [],
    subtasks: []
  }
  let newLabelInput = ''
  let newSubtaskInput = ''
  let loading = true
  let saving = false

  onMount(() => {
    return selectedTaskId.subscribe((id) => {
      taskId = id ?? null
      loadDraft(taskId)
    })
  })

  async function loadDraft(id: string | null) {
    loading = true
    if (id) {
      try {
        const entry = await getEntry(id)
        if (entry && entry.entry_type === 'task') {
          draft = {
            title: entry.title || '',
            content: entry.content || '',
            due_date: entry.due_date ? new Date(entry.due_date).toISOString().slice(0, 16) : '',
            reminder_at: entry.reminder_at ? new Date(entry.reminder_at).toISOString().slice(0, 16) : '',
            priority: entry.priority ?? null,
            is_completed: entry.is_completed ?? false,
            tags: [...(entry.tags || [])],
            subtasks: [...(entry.subtasks || [])]
          }
        }
      } catch {
        draft = { title: '', content: '', due_date: '', reminder_at: '', priority: null, is_completed: false, tags: [], subtasks: [] }
      }
    } else {
      draft = { title: '', content: '', due_date: '', reminder_at: '', priority: null, is_completed: false, tags: [], subtasks: [] }
    }
    loading = false
  }

  /** After a successful save we always land on Tasks (discard “return to history” context). */
  function exitToTasksList() {
    taskDetailReturnTo.set(null)
    selectedTaskId.set(null)
    navigate('tasks')
  }

  /** Cancel / back / delete: return to the dictation if we opened from History detail. */
  function back() {
    const ret = get(taskDetailReturnTo)
    taskDetailReturnTo.set(null)
    selectedTaskId.set(null)
    if (ret?.type === 'history-detail') {
      selectedHistoryId.set(ret.entryId)
      navigate('history-detail')
      return
    }
    navigate('tasks')
  }

  function addTag(t: string) {
    const tag = t.trim()
    if (tag && !draft.tags.includes(tag)) {
      draft.tags = [...draft.tags, tag]
      newLabelInput = ''
    }
  }

  function removeTag(tag: string) {
    draft.tags = draft.tags.filter((x) => x !== tag)
  }

  function addSubtask(title: string) {
    const t = title.trim()
    if (!t) return
    draft.subtasks = [...draft.subtasks, { title: t, is_completed: false }]
    newSubtaskInput = ''
  }

  function removeSubtask(i: number) {
    draft.subtasks = draft.subtasks.filter((_, idx) => idx !== i)
  }

  function toggleSubtask(i: number) {
    draft.subtasks = draft.subtasks.map((s, idx) =>
      idx === i ? { ...s, is_completed: !s.is_completed } : s
    )
  }

  async function save() {
    if (!draft.title.trim()) return
    saving = true
    try {
      const dueDate = draft.due_date.trim() ? new Date(draft.due_date).toISOString() : null
      const reminderAt = draft.reminder_at.trim() ? new Date(draft.reminder_at).toISOString() : null
      if (taskId) {
        const entry = await getEntry(taskId)
        if (entry && entry.entry_type === 'task') {
          const updated: Entry = {
            ...entry,
            title: draft.title.trim(),
            content: draft.content.trim(),
            due_date: dueDate,
            reminder_at: reminderAt,
            priority: draft.priority,
            is_completed: draft.is_completed,
            tags: [...draft.tags],
            subtasks: draft.subtasks.length ? draft.subtasks : null,
            updated_at: new Date().toISOString()
          }
          await updateEntry(updated)
        }
      } else {
        const entry = newEntry('task', draft.content.trim(), {
          title: draft.title.trim(),
          due_date: dueDate,
          reminder_at: reminderAt,
          priority: draft.priority,
          is_completed: draft.is_completed,
          tags: draft.tags,
          subtasks: draft.subtasks.length ? draft.subtasks : null,
          archived_at: null,
          deleted_at: null
        })
        await createEntry(entry)
      }
      exitToTasksList()
    } catch (e) {
      console.error(e)
    } finally {
      saving = false
    }
  }

  async function deleteTask() {
    if (!taskId) return
    try {
      await deleteEntry(taskId)
      back()
    } catch (e) {
      console.error(e)
    }
  }

  /** Priority in UI: 0 = none, 1–3 = low/medium/high; store null for 0. */
  function setPriority(p: number) {
    draft.priority = p === 0 ? null : p
  }
</script>

<div class="page fade-in sleek-editor-page">
  <header class="sleek-header">
    <button type="button" class="sleek-back" on:click={back}>
      <Icon icon="ph:caret-left" /> Tasks
    </button>
    <div class="sleek-actions">
      {#if taskId}
        <button type="button" class="sleek-icon-btn danger" on:click={deleteTask} title="Delete">
          <Icon icon="ph:trash" />
        </button>
      {/if}
      <div class="task-priority-selector compact">
        {#each [0, 1, 2, 3] as p}
          <button
            type="button"
            class="priority-btn"
            class:selected={(p === 0 && draft.priority === null) || (p > 0 && draft.priority === p)}
            on:click={() => setPriority(p)}
            title={p === 0 ? 'No priority' : `Priority ${p}`}
          >
            {#if p === 0}
              <Icon icon="ph:minus" />
            {:else}
              {'!'.repeat(p)}
            {/if}
          </button>
        {/each}
      </div>
      <button
        type="button"
        class="sleek-tool-btn complete-toggle compact"
        class:completed={draft.is_completed}
        on:click={() => draft.is_completed = !draft.is_completed}
        title={draft.is_completed ? 'Mark as incomplete' : 'Mark as complete'}
      >
        <Icon icon={draft.is_completed ? 'ph:check-circle-fill' : 'ph:circle'} />
      </button>
      <button type="button" class="sleek-cancel" on:click={back}>Cancel</button>
      <button type="button" class="sleek-save" on:click={save} disabled={!draft.title?.trim() || saving}>Save</button>
    </div>
  </header>

  <div class="sleek-body">
    <input type="text" class="sleek-title" bind:value={draft.title} placeholder="Task Title" />
    <textarea class="sleek-content task-desc" bind:value={draft.content} placeholder="Add description..."></textarea>

    <div class="due-date-section">
      <h3 class="section-title">Due Date</h3>
      <div class="due-date-input-row">
        <Icon icon="ph:calendar-blank" />
        <input type="datetime-local" class="sleek-datetime-input" bind:value={draft.due_date} />
        {#if draft.due_date}
          <button type="button" class="sleek-clear-btn" on:click={() => draft.due_date = ''}>
            <Icon icon="ph:x" />
          </button>
        {/if}
      </div>
    </div>

    <div class="subtasks-section">
      <h3 class="section-title">Subtasks</h3>
      <div class="subtasks-list">
        {#each draft.subtasks as subtask, i (i)}
          <div class="subtask-row" class:completed={subtask.is_completed}>
            <button type="button" class="drag-handle" title="Drag to reorder" disabled><Icon icon="ph:dots-six-vertical" /></button>
            <button type="button" class="checkbox small" on:click={() => toggleSubtask(i)}>
              {#if subtask.is_completed}
                <Icon icon="ph:check" />
              {/if}
            </button>
            <input type="text" class="subtask-input" bind:value={subtask.title} />
            <button type="button" class="remove-subtask" on:click={() => removeSubtask(i)}>
              <Icon icon="ph:x" />
            </button>
          </div>
        {/each}
        <div class="add-subtask-row">
          <Icon icon="ph:plus" />
          <input
            type="text"
            class="add-subtask-input"
            bind:value={newSubtaskInput}
            placeholder="Add subtask..."
            on:keydown={(e) => e.key === 'Enter' && (addSubtask(newSubtaskInput), e.preventDefault())}
          />
        </div>
      </div>
    </div>

    <div class="sleek-labels">
      <Icon icon="ph:tag" />
      {#if draft.tags.length > 0}
        {#each draft.tags as tag}
          <span class="sleek-label-chip">
            {tag}
            <button type="button" on:click={() => removeTag(tag)}><Icon icon="ph:x" /></button>
          </span>
        {/each}
      {/if}
      <input
        type="text"
        class="sleek-label-input"
        bind:value={newLabelInput}
        placeholder={draft.tags.length ? 'Add another...' : 'Add label...'}
        on:keydown={(e) => e.key === 'Enter' && (addTag(newLabelInput), e.preventDefault())}
      />
    </div>
  </div>
</div>
