<script lang="ts">
  import { get } from 'svelte/store'
  import { onMount } from 'svelte'
  import { getEntry, createEntry, updateEntry, deleteEntry, newEntry } from '../../lib/api/db'
  import type { Entry, Subtask } from '../../types'
  import Icon from '@iconify/svelte'
  import SveltyPicker from 'svelty-picker'
  import { getKalamSveltyPickerLocaleOptions } from '$lib/sveltyPickerLocale'
  import type { KalamSveltyPickerLocaleOptions } from '$lib/sveltyPickerLocale'
  import TiptapEditor from '../TiptapEditor.svelte'
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
  /** Save blocked / API failure (same idea as Note detail). */
  let saveError: string | null = null
  /** Set when the loaded task is in trash (soft-deleted). */
  let taskDeletedAt: string | null = null

  /** Custom priority menu (replaces native select); compact trigger matches previous control footprint. */
  let priorityMenuOpen = false
  let priorityPopoverEl: HTMLDivElement | null = null

  const priorityChoices: { value: number | null; label: string; short: string }[] = [
    { value: null, label: 'No priority', short: 'None' },
    { value: 1, label: 'Low priority', short: 'Low' },
    { value: 2, label: 'Medium priority', short: 'Med' },
    { value: 3, label: 'High priority', short: 'High' }
  ]

  function priorityShort(p: number | null) {
    return priorityChoices.find((c) => c.value === p)?.short ?? 'None'
  }

  function pickPriority(v: number | null) {
    draft = { ...draft, priority: v }
    priorityMenuOpen = false
  }

  /** Reassign so Svelte 4 sees updates to description HTML from Tiptap. */
  function onTiptapHtmlChange(e: CustomEvent<{ html: string }>) {
    draft = { ...draft, content: e.detail.html }
  }

  /** OS locale for svelty-picker labels + display format; storage stays `yyyy-mm-ddThh:ii`. */
  let sdtLocale: KalamSveltyPickerLocaleOptions = getKalamSveltyPickerLocaleOptions()

  onMount(() => {
    const unsub = selectedTaskId.subscribe((id) => {
      taskId = id ?? null
      loadDraft(taskId)
    })
    const onLang = () => {
      sdtLocale = getKalamSveltyPickerLocaleOptions()
    }
    window.addEventListener('languagechange', onLang)
    const onDocClick = (e: MouseEvent) => {
      if (!priorityPopoverEl) return
      if (priorityPopoverEl.contains(e.target as Node)) return
      priorityMenuOpen = false
    }
    document.addEventListener('click', onDocClick)
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') priorityMenuOpen = false
    }
    window.addEventListener('keydown', onKey)
    return () => {
      unsub()
      window.removeEventListener('languagechange', onLang)
      document.removeEventListener('click', onDocClick)
      window.removeEventListener('keydown', onKey)
    }
  })

  async function loadDraft(id: string | null) {
    saveError = null
    loading = true
    if (id) {
      try {
        const entry = await getEntry(id)
        if (entry && entry.entry_type === 'task') {
          taskDeletedAt = entry.deleted_at ?? null
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
        } else {
          taskDeletedAt = null
        }
      } catch {
        taskDeletedAt = null
        draft = { title: '', content: '', due_date: '', reminder_at: '', priority: null, is_completed: false, tags: [], subtasks: [] }
      }
    } else {
      taskDeletedAt = null
      draft = { title: '', content: '', due_date: '', reminder_at: '', priority: null, is_completed: false, tags: [], subtasks: [] }
    }
    loading = false
    priorityMenuOpen = false
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
    if (ret?.type === 'reminders') {
      navigate('reminders')
      return
    }
    navigate('tasks')
  }

  function addTag(t: string) {
    const tag = t.trim()
    if (tag && !draft.tags.includes(tag)) {
      draft = { ...draft, tags: [...draft.tags, tag] }
      newLabelInput = ''
    }
  }

  function removeTag(tag: string) {
    draft = { ...draft, tags: draft.tags.filter((x) => x !== tag) }
  }

  function addSubtask(title: string) {
    const t = title.trim()
    if (!t) return
    draft = { ...draft, subtasks: [...draft.subtasks, { title: t, is_completed: false }] }
    newSubtaskInput = ''
  }

  function removeSubtask(i: number) {
    draft = { ...draft, subtasks: draft.subtasks.filter((_, idx) => idx !== i) }
  }

  function toggleSubtask(i: number) {
    draft = {
      ...draft,
      subtasks: draft.subtasks.map((s, idx) =>
        idx === i ? { ...s, is_completed: !s.is_completed } : s
      )
    }
  }

  /** Same HTML5 DnD pattern as the prototype task editor; order is persisted on Save. */
  let dragSubtaskIndex: number | null = null
  function handleSubtaskDragStart(e: DragEvent, index: number) {
    dragSubtaskIndex = index
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'
  }
  function handleSubtaskDragOver(e: DragEvent) {
    e.preventDefault()
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
  }
  function handleSubtaskDrop(e: DragEvent, targetIndex: number) {
    e.preventDefault()
    if (dragSubtaskIndex === null || dragSubtaskIndex === targetIndex) {
      dragSubtaskIndex = null
      return
    }
    const next = [...draft.subtasks]
    const [moved] = next.splice(dragSubtaskIndex, 1)
    next.splice(targetIndex, 0, moved)
    draft = { ...draft, subtasks: next }
    dragSubtaskIndex = null
  }
  function handleSubtaskDragEnd() {
    dragSubtaskIndex = null
  }

  async function save() {
    saveError = null
    if (!draft.title.trim()) {
      saveError = 'Add a task title before saving.'
      return
    }
    saving = true
    try {
      const dueDate = draft.due_date.trim() ? new Date(draft.due_date).toISOString() : null
      const reminderAt = draft.reminder_at.trim() ? new Date(draft.reminder_at).toISOString() : null
      if (taskId) {
        const entry = await getEntry(taskId)
        if (!entry || entry.entry_type !== 'task') {
          saveError = 'Could not load this task to save. Go back and open it again.'
          return
        }
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
          updated_at: new Date().toISOString(),
          /* Keep in trash until the user restores (same as editing a trashed note). */
          deleted_at: entry.deleted_at ?? null
        }
        await updateEntry(updated)
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
      saveError = e instanceof Error ? e.message : 'Save failed.'
    } finally {
      saving = false
    }
  }

  async function restoreTask() {
    if (!taskId) return
    try {
      const entry = await getEntry(taskId)
      if (!entry || entry.entry_type !== 'task') return
      await updateEntry({
        ...entry,
        deleted_at: null,
        updated_at: new Date().toISOString()
      })
      taskDeletedAt = null
    } catch (e) {
      console.error(e)
    }
  }

  async function deleteTask() {
    if (!taskId) return
    if (taskDeletedAt) {
      if (!confirm('Permanently delete this task?')) return
    } else {
      if (!confirm('Move this task to trash?')) return
    }
    saving = true
    try {
      const entry = await getEntry(taskId)
      if (!entry || entry.entry_type !== 'task') return
      if (entry.deleted_at) {
        await deleteEntry(taskId)
      } else {
        await updateEntry({
          ...entry,
          deleted_at: new Date().toISOString(),
          updated_at: new Date().toISOString()
        })
      }
      back()
    } catch (e) {
      console.error(e)
    } finally {
      saving = false
    }
  }

  /** SveltyPicker — logic in script (Svelte templates cannot use TS `as`). */
  function onDueDatePickerChange(e: CustomEvent<string | null | undefined>) {
    const v = e.detail
    draft = { ...draft, due_date: v == null || v === '' ? '' : String(v) }
  }

  function onTaskReminderPickerChange(e: CustomEvent<string | null | undefined>) {
    const v = e.detail
    draft = { ...draft, reminder_at: v == null || v === '' ? '' : String(v) }
  }
</script>

<div class="page fade-in sleek-editor-page">
  <header class="sleek-header">
    <button type="button" class="sleek-back" on:click={back}>
      <Icon icon="ph:caret-left" /> Tasks
    </button>
    <div class="sleek-actions">
      {#if taskId}
        {#if taskDeletedAt}
          <button type="button" class="sleek-icon-btn" on:click={restoreTask} title="Restore task">
            <Icon icon="ph:arrow-counter-clockwise" />
          </button>
        {/if}
        <button
          type="button"
          class="sleek-icon-btn danger"
          on:click={deleteTask}
          disabled={saving}
          title={taskDeletedAt ? 'Delete permanently' : 'Move to trash'}
        >
          <Icon icon="ph:trash" />
        </button>
      {/if}
      <button type="button" class="sleek-cancel" on:click={back}>Cancel</button>
      <button type="button" class="sleek-save" on:click={save} disabled={!draft.title?.trim() || saving}>Save</button>
    </div>
  </header>

  {#if saveError}
    <p class="task-detail-save-error" role="alert">{saveError}</p>
  {/if}

  <div class="sleek-body">
    <div class="task-meta-row">
      <div class="task-meta-item">
        <span class="task-meta-label">Status</span>
        <button
          type="button"
          class="task-status-trigger"
          class:completed={draft.is_completed}
          on:click={() => (draft = { ...draft, is_completed: !draft.is_completed })}
          title={draft.is_completed ? 'Mark as incomplete' : 'Mark as complete'}
        >
          <Icon icon={draft.is_completed ? 'ph:check-circle-fill' : 'ph:circle'} class="task-status-icon" />
          <span class="task-status-text">{draft.is_completed ? 'Completed' : 'Pending'}</span>
        </button>
      </div>

      <div class="task-meta-item">
        <span class="task-meta-label">Priority</span>
        <div class="task-priority-popover" bind:this={priorityPopoverEl}>
          <button
            type="button"
            class="task-priority-trigger"
            aria-haspopup="listbox"
            aria-expanded={priorityMenuOpen}
            aria-label="Priority: {priorityShort(draft.priority)}"
            on:click|stopPropagation={() => (priorityMenuOpen = !priorityMenuOpen)}
          >
            <Icon
              icon={draft.priority ? 'ph:flag-fill' : 'ph:flag'}
              class="task-priority-trigger-flag p{draft.priority == null ? 0 : draft.priority}"
            />
            <span class="task-priority-trigger-text">{priorityShort(draft.priority)}</span>
            <Icon icon="ph:caret-down" class="task-priority-trigger-caret" />
          </button>
          {#if priorityMenuOpen}
            <ul class="task-priority-menu" role="listbox" aria-label="Choose priority">
              {#each priorityChoices as choice}
                <li role="none">
                  <button
                    type="button"
                    class="task-priority-option"
                    class:selected={(choice.value === null && draft.priority === null) || choice.value === draft.priority}
                    role="option"
                    aria-selected={(choice.value === null && draft.priority === null) || choice.value === draft.priority}
                    on:click|stopPropagation={() => pickPriority(choice.value)}
                  >
                    <Icon
                      icon={choice.value ? 'ph:flag-fill' : 'ph:flag'}
                      class="task-priority-option-flag p{choice.value == null ? 0 : choice.value}"
                    />
                    <span class="task-priority-option-label">{choice.label}</span>
                    {#if (choice.value === null && draft.priority === null) || choice.value === draft.priority}
                      <Icon icon="ph:check" class="task-priority-option-check" />
                    {/if}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </div>
    </div>

    <div class="task-title-row">
      <input 
        type="text" 
        class="sleek-title" 
        class:completed={draft.is_completed}
        bind:value={draft.title} 
        placeholder="Task Title" 
      />
    </div>
    <TiptapEditor
      documentKey={taskId ?? 'new-task'}
      html={draft.content}
      placeholder="Add description… Type / for formatting"
      shellClass="sleek-content task-desc"
      on:change={onTiptapHtmlChange}
    />

    <div class="subtasks-section">
      <h3 class="section-title">Subtasks</h3>
      <div class="subtasks-list" role="list" aria-label="Subtasks">
        {#each draft.subtasks as subtask, i (subtask)}
          <div
            class="subtask-row"
            role="listitem"
            class:completed={subtask.is_completed}
            draggable={true}
            on:dragstart={(e) => handleSubtaskDragStart(e, i)}
            on:dragover={handleSubtaskDragOver}
            on:drop={(e) => handleSubtaskDrop(e, i)}
            on:dragend={handleSubtaskDragEnd}
          >
            <button type="button" class="drag-handle" title="Drag to reorder" on:click|stopPropagation>
              <Icon icon="ph:dots-six-vertical" />
            </button>
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

    <div class="due-date-section">
      <h3 class="section-title">Due Date</h3>
      <div class="due-date-input-row">
        <Icon icon="ph:calendar-blank" />
        <!-- svelty-picker: ISO-like string matches prior datetime-local (YYYY-MM-DDTHH:mm) for API/DB -->
        <div class="kalam-sdt-datetime">
          <SveltyPicker
            mode="datetime"
            format={sdtLocale.format}
            displayFormat={sdtLocale.displayFormat}
            displayFormatType={sdtLocale.displayFormatType}
            i18n={sdtLocale.i18n}
            weekStart={sdtLocale.weekStart}
            value={draft.due_date || null}
            inputClasses="sleek-datetime-input"
            on:change={onDueDatePickerChange}
          />
        </div>
        {#if draft.due_date}
          <button type="button" class="sleek-clear-btn" on:click={() => (draft = { ...draft, due_date: '' })}>
            <Icon icon="ph:x" />
          </button>
        {/if}
      </div>
      {#if draft.due_date && new Date(draft.due_date).getTime() < Date.now()}
        <p class="reminder-past-hint">Due time is in the past; you can still save.</p>
      {/if}
    </div>

    <div class="reminder-section">
      <h3 class="section-title">Reminder</h3>
      <div class="due-date-input-row">
        <Icon icon="ph:bell" />
        <div class="kalam-sdt-datetime">
          <SveltyPicker
            mode="datetime"
            format={sdtLocale.format}
            displayFormat={sdtLocale.displayFormat}
            displayFormatType={sdtLocale.displayFormatType}
            i18n={sdtLocale.i18n}
            weekStart={sdtLocale.weekStart}
            value={draft.reminder_at || null}
            inputClasses="sleek-datetime-input"
            on:change={onTaskReminderPickerChange}
          />
        </div>
        {#if draft.reminder_at}
          <button type="button" class="sleek-clear-btn" on:click={() => (draft = { ...draft, reminder_at: '' })}>
            <Icon icon="ph:x" />
          </button>
        {/if}
      </div>
      {#if draft.reminder_at && new Date(draft.reminder_at).getTime() < Date.now()}
        <p class="reminder-past-hint">Reminder time is in the past; you can still save.</p>
      {/if}
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

<style>
  .task-detail-save-error {
    margin: 0 var(--space-lg, 16px) var(--space-sm, 8px);
    padding: var(--space-sm, 8px) var(--space-md, 12px);
    border-radius: var(--radius-md, 8px);
    background: color-mix(in srgb, var(--danger, #dc2626) 12%, transparent);
    color: var(--text, inherit);
    font-size: 0.875rem;
    line-height: 1.4;
  }

  .task-meta-row {
    display: flex;
    align-items: center;
    gap: var(--space-xl, 24px);
    margin-bottom: var(--space-md, 16px);
  }
  .task-meta-item {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .task-meta-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary, #86868b);
  }
  .task-status-trigger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 36px;
    padding: 0 10px;
    margin: 0;
    border: 1px solid var(--border, #e5e5ea);
    border-radius: var(--radius-md, 8px);
    background: var(--bg-elevated, #fff);
    color: var(--text-secondary, #86868b);
    font-family: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }
  .task-status-trigger:hover {
    border-color: var(--text-muted, #86868b);
    background: var(--bg, #fff);
  }
  .task-status-trigger.completed {
    background: rgba(52, 199, 89, 0.1);
    border-color: rgba(52, 199, 89, 0.3);
    color: #34c759;
  }
  .task-status-trigger.completed .task-status-icon {
    color: #34c759;
  }
  .task-status-icon {
    font-size: 16px;
  }

  .task-title-row {
    display: flex;
    align-items: center;
    margin-bottom: var(--space-xl, 24px);
  }
  .task-title-row :global(.sleek-title) {
    margin-bottom: 0 !important;
    flex: 1;
    transition: all 0.2s ease;
  }
  .task-title-row :global(.sleek-title.completed) {
    text-decoration: line-through;
    color: var(--text-secondary, #86868b);
  }

  .reminder-past-hint {
    margin-top: 8px;
    font-size: 12px;
    color: var(--text-secondary, #86868b);
  }

  .reminder-section {
    margin-bottom: var(--space-2xl, 32px);
  }
</style>
