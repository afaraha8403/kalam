<script lang="ts">
  import { get } from 'svelte/store'
  import { fade } from 'svelte/transition'
  import { onMount } from 'svelte'
  import { getEntry, createEntry, updateEntry, deleteEntry, newEntry } from '../../lib/api/db'
  import type { Entry } from '../../types'
  import Icon from '@iconify/svelte'
  import { selectedNoteId } from '../../lib/noteDetailStore'
  import { selectedHistoryId } from '../../lib/historyDetailStore'
  import { noteDetailReturnTo } from '../../lib/detailReturnStore'

  const NOTE_COLORS = [
    { name: 'default', value: '' },
    { name: 'yellow', value: '#fef08a' },
    { name: 'orange', value: '#fed7aa' },
    { name: 'red', value: '#fecaca' },
    { name: 'pink', value: '#fbcfe8' },
    { name: 'purple', value: '#e9d5ff' },
    { name: 'blue', value: '#bfdbfe' },
    { name: 'cyan', value: '#a5f3fc' },
    { name: 'gray', value: '#e2e8f0' }
  ]

  export let navigate: (page: string) => void = () => {}

  let noteId: string | null = null
  let draft: { title: string; content: string; color: string; reminder_at: string; pinned: boolean; tags: string[]; updated_at: string } = {
    title: '',
    content: '',
    color: '',
    reminder_at: '',
    pinned: false,
    tags: [],
    updated_at: ''
  }
  let newLabelInput = ''
  let showColorPicker = false
  let showReminderInput = false
  let loading = true
  let saving = false
  /** From DB: controls archive / unarchive in header (not in draft). */
  let entryArchivedAt: string | null = null
  let entryDeletedAt: string | null = null

  onMount(() => {
    return selectedNoteId.subscribe((id) => {
      noteId = id ?? null
      loadDraft(noteId)
    })
  })

  async function loadDraft(id: string | null) {
    loading = true
    entryArchivedAt = null
    entryDeletedAt = null
    if (id) {
      try {
        const entry = await getEntry(id)
        if (entry && entry.entry_type === 'note') {
          entryArchivedAt = entry.archived_at ?? null
          entryDeletedAt = entry.deleted_at ?? null
          draft = {
            title: entry.title || '',
            content: entry.content || '',
            color: entry.color || '',
            reminder_at: entry.reminder_at ? new Date(entry.reminder_at).toISOString().slice(0, 16) : '',
            pinned: entry.is_pinned ?? false,
            tags: [...(entry.tags || [])],
            updated_at: entry.updated_at || ''
          }
        }
      } catch {
        draft = { title: '', content: '', color: '', reminder_at: '', pinned: false, tags: [], updated_at: '' }
      }
    } else {
      draft = { title: '', content: '', color: '', reminder_at: '', pinned: false, tags: [], updated_at: '' }
    }
    loading = false
  }

  function exitToNotesList() {
    noteDetailReturnTo.set(null)
    selectedNoteId.set(null)
    navigate('notes')
  }

  function back() {
    const ret = get(noteDetailReturnTo)
    noteDetailReturnTo.set(null)
    selectedNoteId.set(null)
    if (ret?.type === 'history-detail') {
      selectedHistoryId.set(ret.entryId)
      navigate('history-detail')
      return
    }
    navigate('notes')
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
    const title = draft.title.trim()
    const content = draft.content.trim()
    if (!title && !content) return
    saving = true
    try {
      const reminderAt = draft.reminder_at.trim() ? new Date(draft.reminder_at).toISOString() : null
      if (noteId) {
        const entry = await getEntry(noteId)
        if (entry && entry.entry_type === 'note') {
          const updated: Entry = {
            ...entry,
            title: title || null,
            content: content || '',
            color: draft.color || null,
            reminder_at: reminderAt,
            is_pinned: draft.pinned,
            tags: [...draft.tags],
            updated_at: new Date().toISOString()
          }
          await updateEntry(updated)
        }
      } else {
        const entry = newEntry('note', content, {
          title: title || null,
          color: draft.color || null,
          reminder_at: reminderAt,
          is_pinned: draft.pinned,
          tags: [...draft.tags],
          archived_at: null,
          deleted_at: null
        })
        await createEntry(entry)
      }
      exitToNotesList()
    } catch (e) {
      console.error('Failed to save note:', e)
    } finally {
      saving = false
    }
  }

  async function deleteNote() {
    if (!noteId) return
    if (entryDeletedAt) {
      if (!confirm('Permanently delete this note?')) return
    } else {
      if (!confirm('Move this note to trash?')) return
    }
    saving = true
    try {
      if (entryDeletedAt) {
        await deleteEntry(noteId)
      } else {
        const entry = await getEntry(noteId)
        if (entry?.entry_type === 'note') {
          await updateEntry({
            ...entry,
            deleted_at: new Date().toISOString(),
            updated_at: new Date().toISOString()
          })
        }
      }
      back()
    } catch (e) {
      console.error('Failed to delete note:', e)
    } finally {
      saving = false
    }
  }

  async function archiveFromDetail() {
    if (!noteId || saving) return
    if (!confirm('Archive this note?')) return
    saving = true
    try {
      const entry = await getEntry(noteId)
      if (entry?.entry_type === 'note') {
        await updateEntry({
          ...entry,
          archived_at: new Date().toISOString(),
          updated_at: new Date().toISOString()
        })
        back()
      }
    } catch (e) {
      console.error('Failed to archive note:', e)
    } finally {
      saving = false
    }
  }

  async function unarchiveFromDetail() {
    if (!noteId || saving) return
    saving = true
    try {
      const entry = await getEntry(noteId)
      if (entry?.entry_type === 'note') {
        await updateEntry({
          ...entry,
          archived_at: null,
          updated_at: new Date().toISOString()
        })
        back()
      }
    } catch (e) {
      console.error('Failed to unarchive note:', e)
    } finally {
      saving = false
    }
  }

  function formatNoteDate(iso: string) {
    if (!iso) return ''
    const d = new Date(iso)
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  }

  function formatReminder(iso: string) {
    if (!iso) return ''
    return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' })
  }

  /** Close color/reminder popovers when clicking outside the control that opened them. */
  function handleClickOutside(event: MouseEvent) {
    const target = event.target as HTMLElement
    if (!target.closest('.color-dropdown-container')) showColorPicker = false
    if (!target.closest('.reminder-dropdown-container')) showReminderInput = false
  }
</script>

{#if loading}
  <div class="page fade-in state-container">
    <Icon icon="ph:spinner-gap-duotone" />
    <p>Loading…</p>
  </div>
{:else}
  <div class="page fade-in sleek-editor-page note-detail-page" on:click={handleClickOutside} role="presentation">
    <header class="sleek-header">
      <button type="button" class="sleek-back" on:click={back}>
        <Icon icon="ph:caret-left" /> Notes
      </button>
      <div class="sleek-actions note-detail-header-actions">
        {#if noteId}
          <button
            type="button"
            class="sleek-icon-btn danger"
            on:click={deleteNote}
            disabled={saving}
            title="Delete"
            aria-label="Delete"
          >
            <Icon icon="ph:trash" />
          </button>
        {/if}
        {#if noteId && !entryDeletedAt}
          {#if !entryArchivedAt}
            <button
              type="button"
              class="sleek-icon-btn"
              on:click={archiveFromDetail}
              disabled={saving}
              title="Archive"
              aria-label="Archive"
            >
              <Icon icon="ph:archive" />
            </button>
          {:else}
            <button
              type="button"
              class="sleek-icon-btn"
              on:click={unarchiveFromDetail}
              disabled={saving}
              title="Unarchive"
              aria-label="Unarchive"
            >
              <Icon icon="ph:archive-tray" />
            </button>
          {/if}
        {/if}
        <button type="button" class="sleek-cancel" on:click={back}>Cancel</button>
        <button type="button" class="sleek-save" on:click={save} disabled={(!draft.title?.trim() && !draft.content?.trim()) || saving}>
          Save
        </button>
      </div>
    </header>

    <div class="sleek-body">
      <input type="text" class="sleek-title" bind:value={draft.title} placeholder="Note Title" />
      <textarea class="sleek-content" bind:value={draft.content} placeholder="Start typing..."></textarea>
      <div class="sleek-labels">
        <Icon icon="ph:tag" />
        {#if draft.tags.length > 0}
          {#each draft.tags as tag}
            <span class="sleek-label-chip">
              {tag}
              <button type="button" on:click={() => removeTag(tag)} aria-label="Remove {tag}"><Icon icon="ph:x" /></button>
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

    <footer class="sleek-footer">
      <div class="sleek-meta">
        {draft.updated_at ? `Edited ${formatNoteDate(draft.updated_at)}` : 'New Note'}
      </div>
      <div class="sleek-tools">
        <div class="color-dropdown-container">
          <button type="button" class="sleek-tool-btn color-toggle" on:click={() => { showColorPicker = !showColorPicker; showReminderInput = false; }} title="Change color">
            <span class="current-color-indicator" style:background-color={draft.color || 'var(--bg-elevated)'}></span>
          </button>
          {#if showColorPicker}
            <div class="sleek-popover color-popover" transition:fade={{ duration: 150 }}>
              <div class="sleek-colors-grid">
                {#each NOTE_COLORS as c}
                  <button
                    type="button"
                    class="sleek-color-dot"
                    class:selected={draft.color === c.value}
                    style:background-color={c.value || 'var(--bg-elevated)'}
                    on:click={() => { draft.color = c.value; showColorPicker = false; }}
                    title={c.name}
                  ></button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
        <div class="reminder-dropdown-container">
          <button type="button" class="sleek-tool-btn" class:active={!!draft.reminder_at} on:click={() => { showReminderInput = !showReminderInput; showColorPicker = false; }} title={draft.reminder_at ? formatReminder(draft.reminder_at) : 'Set reminder'}>
            <Icon icon={draft.reminder_at ? 'ph:bell-fill' : 'ph:bell'} />
          </button>
          {#if showReminderInput}
            <div class="sleek-popover reminder-popover" transition:fade={{ duration: 150 }}>
              <input type="datetime-local" class="sleek-datetime-input" bind:value={draft.reminder_at} on:change={() => (showReminderInput = false)} />
              {#if draft.reminder_at}
                <button type="button" class="sleek-clear-btn" on:click={() => { draft.reminder_at = ''; showReminderInput = false; }}>Clear Reminder</button>
              {/if}
            </div>
          {/if}
        </div>
        <button type="button" class="sleek-tool-btn" class:active={draft.pinned} on:click={() => (draft.pinned = !draft.pinned)} title="Pin note">
          <Icon icon={draft.pinned ? 'ph:push-pin-fill' : 'ph:push-pin'} />
        </button>
      </div>
    </footer>
  </div>
{/if}

<style>
  /* Match History detail header: 40×40 icon buttons, 22px glyphs (see HistoryDetail.svelte). */
  .note-detail-page :global(.sleek-header) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md, 16px);
    width: 100%;
    min-width: 0;
  }

  .note-detail-page .note-detail-header-actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    flex-shrink: 0;
    min-width: 0;
  }

  :global(.kalam-sleek .page-content .note-detail-header-actions .sleek-icon-btn:disabled) {
    opacity: 0.35;
    cursor: not-allowed;
  }
</style>
