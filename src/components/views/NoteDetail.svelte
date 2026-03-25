<script lang="ts">
  import { get } from 'svelte/store'
  import { fade } from 'svelte/transition'
  import { onMount } from 'svelte'
  import { getEntry, createEntry, updateEntry, deleteEntry, newEntry } from '../../lib/api/db'
  import type { Entry } from '../../types'
  import Icon from '@iconify/svelte'
  import SveltyPicker from 'svelty-picker'
  import { getKalamSveltyPickerLocaleOptions } from '$lib/sveltyPickerLocale'
  import type { KalamSveltyPickerLocaleOptions } from '$lib/sveltyPickerLocale'
  import TiptapEditor from '../TiptapEditor.svelte'
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
  let loading = true
  let saving = false
  /** Inline message when save is blocked or the API fails (Svelte 4 needs `draft = { ...draft }` for nested updates to refresh bindings). */
  let saveError: string | null = null
  /** From DB: controls archive / unarchive in header (not in draft). */
  let entryArchivedAt: string | null = null
  let entryDeletedAt: string | null = null

  let sdtLocale: KalamSveltyPickerLocaleOptions = getKalamSveltyPickerLocaleOptions()

  onMount(() => {
    const unsub = selectedNoteId.subscribe((id) => {
      noteId = id ?? null
      loadDraft(noteId)
    })
    const onLang = () => {
      sdtLocale = getKalamSveltyPickerLocaleOptions()
    }
    window.addEventListener('languagechange', onLang)
    return () => {
      unsub()
      window.removeEventListener('languagechange', onLang)
    }
  })

  async function loadDraft(id: string | null) {
    saveError = null
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
    if (ret?.type === 'reminders') {
      navigate('reminders')
      return
    }
    navigate('notes')
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

  /** Strip HTML for empty-body check; stored content is HTML from Tiptap. */
  function plainTextFromHtml(html: string): string {
    return html.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim()
  }

  async function save() {
    saveError = null
    const title = draft.title.trim()
    const content = draft.content.trim()
    if (!title && !plainTextFromHtml(content)) {
      saveError = 'Add a title or some note text before saving.'
      return
    }
    saving = true
    try {
      const reminderAt = draft.reminder_at.trim() ? new Date(draft.reminder_at).toISOString() : null
      if (noteId) {
        const entry = await getEntry(noteId)
        if (!entry || entry.entry_type !== 'note') {
          saveError = 'Could not load this note to save. Go back and open it again.'
          return
        }
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
      saveError = e instanceof Error ? e.message : 'Save failed.'
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

  /** Trashed notes: clear deleted_at so the note returns to Notes (same as list Restore). */
  async function restoreFromDetail() {
    if (!noteId || saving) return
    saving = true
    try {
      const entry = await getEntry(noteId)
      if (entry?.entry_type === 'note') {
        await updateEntry({
          ...entry,
          deleted_at: null,
          updated_at: new Date().toISOString()
        })
        back()
      }
    } catch (e) {
      console.error('Failed to restore note:', e)
    } finally {
      saving = false
    }
  }

  function formatNoteDate(iso: string) {
    if (!iso) return ''
    const d = new Date(iso)
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  }

  /** SveltyPicker change payload — keep logic in script (Svelte templates cannot use TS `as`). */
  function onReminderPickerChange(e: CustomEvent<string | null | undefined>) {
    const v = e.detail
    draft = { ...draft, reminder_at: v == null || v === '' ? '' : String(v) }
  }

  /** Must reassign `draft` so Svelte 4 re-runs `disabled` / labels that depend on `draft.content`. */
  function onTiptapHtmlChange(e: CustomEvent<{ html: string }>) {
    draft = { ...draft, content: e.detail.html }
  }

  /** Close color popover when clicking outside its anchor. */
  function handleClickOutside(event: MouseEvent) {
    const target = event.target as HTMLElement
    if (!target.closest('.color-dropdown-container')) showColorPicker = false
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
        <div class="color-dropdown-container">
          <button
            type="button"
            class="sleek-tool-btn color-toggle"
            on:click|stopPropagation={() => (showColorPicker = !showColorPicker)}
            title="Change color"
            aria-label="Change note color"
          >
            <span class="current-color-indicator" style:background-color={draft.color || 'var(--bg-elevated)'}></span>
          </button>
          {#if showColorPicker}
            <div class="sleek-popover color-popover header-anchor-popover" transition:fade={{ duration: 150 }}>
              <div class="sleek-colors-grid">
                {#each NOTE_COLORS as c}
                  <button
                    type="button"
                    class="sleek-color-dot"
                    class:selected={draft.color === c.value}
                    style:background-color={c.value || 'var(--bg-elevated)'}
                    on:click={() => {
                      draft = { ...draft, color: c.value }
                      showColorPicker = false
                    }}
                    title={c.name}
                  ></button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
        <button
          type="button"
          class="sleek-tool-btn"
          class:active={draft.pinned}
          on:click={() => (draft = { ...draft, pinned: !draft.pinned })}
          title="Pin note"
          aria-label={draft.pinned ? 'Unpin note' : 'Pin note'}
        >
          <Icon icon={draft.pinned ? 'ph:push-pin-fill' : 'ph:push-pin'} />
        </button>
        {#if noteId && entryDeletedAt}
          <button
            type="button"
            class="sleek-cancel"
            on:click={restoreFromDetail}
            disabled={saving}
            title="Return to Notes"
            aria-label="Restore note from trash"
          >
            <Icon icon="ph:arrow-counter-clockwise" />
            Restore
          </button>
        {/if}
        {#if noteId}
          <button
            type="button"
            class="sleek-icon-btn danger"
            on:click={deleteNote}
            disabled={saving}
            title={entryDeletedAt ? 'Delete permanently' : 'Move to trash'}
            aria-label={entryDeletedAt ? 'Delete permanently' : 'Move to trash'}
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
              class="sleek-cancel"
              on:click={unarchiveFromDetail}
              disabled={saving}
              title="Return to Notes"
              aria-label="Unarchive"
            >
              <Icon icon="ph:archive-tray" />
              Unarchive
            </button>
          {/if}
        {/if}
        <button type="button" class="sleek-cancel" on:click={back}>Cancel</button>
        <button
          type="button"
          class="sleek-save"
          on:click={save}
          disabled={(!draft.title?.trim() && !plainTextFromHtml(draft.content)) || saving}
        >
          Save
        </button>
      </div>
    </header>

    {#if saveError}
      <p class="note-detail-save-error" role="alert">{saveError}</p>
    {/if}

    <div class="sleek-body">
      <input type="text" class="sleek-title" bind:value={draft.title} placeholder="Note Title" />
      <TiptapEditor
        documentKey={noteId ?? 'new-note'}
        html={draft.content}
        placeholder="Start typing… Type / for formatting"
        shellClass="sleek-content"
        on:change={onTiptapHtmlChange}
      />
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
      <!-- Same structure as TaskDetail reminder: globals in App.svelte + app.css (.due-date-input-row, .section-title) -->
      <div class="reminder-section">
        <h3 class="section-title">Reminder</h3>
        <div class="due-date-input-row">
          <Icon icon="ph:bell" aria-hidden="true" />
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
              on:change={onReminderPickerChange}
            />
          </div>
          {#if draft.reminder_at}
            <button type="button" class="sleek-clear-btn" on:click={() => (draft = { ...draft, reminder_at: '' })} aria-label="Clear reminder">
              <Icon icon="ph:x" />
            </button>
          {/if}
        </div>
        {#if draft.reminder_at && new Date(draft.reminder_at).getTime() < Date.now()}
          <p class="reminder-past-hint">Reminder time is in the past; you can still save.</p>
        {/if}
      </div>
    </div>

    <footer class="sleek-footer">
      <div class="sleek-meta">
        {draft.updated_at ? `Edited ${formatNoteDate(draft.updated_at)}` : 'New Note'}
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

  .note-detail-save-error {
    margin: 0 var(--space-lg, 16px) var(--space-sm, 8px);
    padding: var(--space-sm, 8px) var(--space-md, 12px);
    border-radius: var(--radius-md, 8px);
    background: color-mix(in srgb, var(--danger, #dc2626) 12%, transparent);
    color: var(--text, inherit);
    font-size: 0.875rem;
    line-height: 1.4;
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

  /* Color popover in header: open below anchor (global .sleek-popover uses bottom:… for footer tools). */
  .note-detail-header-actions :global(.header-anchor-popover.sleek-popover) {
    bottom: auto;
    top: calc(100% + 8px);
    left: auto;
    right: 0;
    transform: none;
  }

  /* Spacing after labels; section-title / due-date-input-row match TaskDetail via App.svelte globals */
  .note-detail-page .reminder-section {
    margin-top: var(--space-xl, 24px);
  }

  .reminder-past-hint {
    margin-top: 8px;
    font-size: 12px;
    color: var(--text-secondary, #86868b);
  }
</style>
