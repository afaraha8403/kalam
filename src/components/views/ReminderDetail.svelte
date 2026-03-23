<script lang="ts">
  import { getEntry, createEntry, updateEntry, deleteEntry, newEntry } from '../../lib/api/db'
  import type { Entry } from '../../types'
  import Icon from '@iconify/svelte'
  import { onMount } from 'svelte'
  import { selectedReminderId } from '../../lib/reminderDetailStore'

  export let navigate: (page: string) => void = () => {}

  let reminderId: string | null = null
  let draft: { content: string; reminder_at: string; rrule: string; tags: string[] } = {
    content: '',
    reminder_at: '',
    rrule: '',
    tags: []
  }
  let newLabelInput = ''
  let loading = true
  let saving = false

  const RRULE_OPTIONS = [
    { value: '', label: "Don't repeat" },
    { value: 'FREQ=DAILY', label: 'Every day' },
    { value: 'FREQ=WEEKLY', label: 'Every week' },
    { value: 'FREQ=MONTHLY', label: 'Every month' },
    { value: 'FREQ=YEARLY', label: 'Every year' }
  ]

  onMount(() => {
    return selectedReminderId.subscribe((id) => {
      reminderId = id ?? null
      loadDraft(reminderId)
    })
  })

  async function loadDraft(id: string | null) {
    loading = true
    if (id) {
      try {
        const entry = await getEntry(id)
        if (entry && entry.entry_type === 'reminder') {
          draft = {
            content: entry.content || '',
            reminder_at: entry.reminder_at ? new Date(entry.reminder_at).toISOString().slice(0, 16) : '',
            rrule: entry.rrule || '',
            tags: [...(entry.tags || [])]
          }
        }
      } catch {
        draft = { content: '', reminder_at: '', rrule: '', tags: [] }
      }
    } else {
      draft = { content: '', reminder_at: '', rrule: '', tags: [] }
    }
    loading = false
  }

  function back() {
    selectedReminderId.set(null)
    navigate('reminders')
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

  async function save() {
    if (!draft.content.trim()) return
    saving = true
    try {
      const reminderAt = draft.reminder_at.trim() ? new Date(draft.reminder_at).toISOString() : null
      const rrule = draft.rrule.trim() || null
      if (reminderId) {
        const entry = await getEntry(reminderId)
        if (entry && entry.entry_type === 'reminder') {
          const updated: Entry = {
            ...entry,
            content: draft.content.trim(),
            reminder_at: reminderAt,
            rrule,
            tags: [...draft.tags],
            updated_at: new Date().toISOString()
          }
          await updateEntry(updated)
        }
      } else {
        const entry = newEntry('reminder', draft.content.trim(), {
          reminder_at: reminderAt,
          rrule,
          tags: draft.tags,
          archived_at: null,
          deleted_at: null
        })
        await createEntry(entry)
      }
      back()
    } catch (e) {
      console.error(e)
    } finally {
      saving = false
    }
  }

  async function deleteReminder() {
    if (!reminderId) return
    try {
      await deleteEntry(reminderId)
      back()
    } catch (e) {
      console.error(e)
    }
  }
</script>

<div class="page fade-in sleek-editor-page">
  <header class="sleek-header">
    <button type="button" class="sleek-back" on:click={back}>
      <Icon icon="ph:caret-left" /> Reminders
    </button>
    <div class="sleek-actions">
      {#if reminderId}
        <button type="button" class="sleek-icon-btn danger" on:click={deleteReminder} title="Delete">
          <Icon icon="ph:trash" />
        </button>
      {/if}
      <button type="button" class="sleek-cancel" on:click={back}>Cancel</button>
      <button type="button" class="sleek-save" on:click={save} disabled={!draft.content?.trim() || saving}>Save</button>
    </div>
  </header>

  <div class="sleek-body">
    <input type="text" class="sleek-title" bind:value={draft.content} placeholder="Remind me to..." />

    <div class="reminder-form-row">
      <label class="form-label" for="reminder-datetime">Date & Time</label>
      <input
        id="reminder-datetime"
        type="datetime-local"
        class="sleek-datetime-input full-width"
        bind:value={draft.reminder_at}
      />
    </div>

    <div class="reminder-form-row">
      <label class="form-label" for="reminder-rrule">Repeat</label>
      <select id="reminder-rrule" class="form-select" bind:value={draft.rrule}>
        {#each RRULE_OPTIONS as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
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
