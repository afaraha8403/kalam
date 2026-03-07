<script lang="ts">
  import {
    getNotes,
    searchNotes,
    getNoteLabels,
    createEntry,
    updateEntry,
    deleteEntry,
    emptyTrash,
    newEntry
  } from '../../lib/api/db'
  import type { Entry } from '../../types'
  import Icon from '@iconify/svelte'
  import { marked } from 'marked'
  import DOMPurify from 'dompurify'

  const NOTE_COLORS = [
    { name: 'default', value: '' },
    { name: 'white', value: '#fffef0' },
    { name: 'yellow', value: '#fef9c3' },
    { name: 'orange', value: '#ffedd5' },
    { name: 'peach', value: '#fecdd3' },
    { name: 'pink', value: '#fce7f3' },
    { name: 'lavender', value: '#ede9fe' },
    { name: 'blue', value: '#dbeafe' },
    { name: 'mint', value: '#ccfbf1' },
    { name: 'gray', value: '#f1f5f9' }
  ]

  let entries: Entry[] = []
  let labels: string[] = []
  let loading = true
  let error: string | null = null
  let notesScope: 'active' | 'archived' | 'trash' = 'active'
  let searchQuery = ''
  let searchDebounced = ''
  let selectedLabel: string | null = null
  let newTitle = ''
  let newContent = ''
  let newColor = ''
  let newReminderAt = ''
  let newTags: string[] = []
  let newPin = false
  let isComposerExpanded = false
  let editingNote: Entry | null = null
  let editTitle = ''
  let editContent = ''
  let editColor = ''
  let editReminderAt = ''
  let editTags: string[] = []
  let editPin = false
  let labelDropdownOpen = false
  let newLabelInput = ''
  let searchTimeout: ReturnType<typeof setTimeout>

  function renderMarkdown(raw: string): string {
    if (!raw?.trim()) return ''
    const html = marked.parse(raw, { async: false }) as string
    return DOMPurify.sanitize(html)
  }

  async function load() {
    loading = true
    error = null
    try {
      if (searchDebounced.trim() || selectedLabel) {
        const result = await searchNotes({
          query: searchDebounced.trim() || undefined,
          label: selectedLabel ?? undefined,
          scope: notesScope
        })
        entries = Array.isArray(result) ? result : []
      } else {
        const result = await getNotes(notesScope)
        entries = Array.isArray(result) ? result : []
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    } finally {
      loading = false
    }
  }

  async function loadLabels() {
    try {
      labels = await getNoteLabels(notesScope === 'trash' ? 'active' : notesScope)
    } catch {
      labels = []
    }
  }

  function triggerSearch() {
    clearTimeout(searchTimeout)
    searchTimeout = setTimeout(() => {
      searchDebounced = searchQuery
      load()
    }, 300)
  }

  $: void (notesScope, searchDebounced, selectedLabel), load()
  $: void notesScope, loadLabels()
  $: void searchQuery, triggerSearch()

  async function addNote() {
    const content = newContent.trim() || newTitle.trim()
    if (!content) return
    const reminderAt = newReminderAt.trim() ? new Date(newReminderAt.trim()).toISOString() : null
    const entry = newEntry('note', content, {
      title: newTitle.trim() || null,
      color: newColor || null,
      reminder_at: reminderAt,
      tags: [...newTags],
      is_pinned: newPin,
      archived_at: null,
      deleted_at: null
    })
    try {
      await createEntry(entry)
      newTitle = ''
      newContent = ''
      newColor = ''
      newReminderAt = ''
      newTags = []
      newPin = false
      isComposerExpanded = false
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  async function archiveNote(entry: Entry) {
    try {
      await updateEntry({
        ...entry,
        archived_at: new Date().toISOString(),
        updated_at: new Date().toISOString()
      })
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    }
  }

  async function unarchiveNote(entry: Entry) {
    try {
      await updateEntry({
        ...entry,
        archived_at: null,
        updated_at: new Date().toISOString()
      })
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    }
  }

  async function moveToTrash(entry: Entry) {
    try {
      await updateEntry({
        ...entry,
        deleted_at: new Date().toISOString(),
        updated_at: new Date().toISOString()
      })
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    }
  }

  async function restoreNote(entry: Entry) {
    try {
      await updateEntry({
        ...entry,
        deleted_at: null,
        updated_at: new Date().toISOString()
      })
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    }
  }

  async function permanentlyDelete(id: string) {
    try {
      await deleteEntry(id)
      editingNote = null
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    }
  }

  async function emptyTrashConfirm() {
    if (!confirm('Empty trash? All trashed notes will be permanently deleted.')) return
    try {
      await emptyTrash()
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    }
  }

  function openEdit(entry: Entry) {
    editingNote = entry
    editTitle = entry.title ?? ''
    editContent = entry.content ?? ''
    editColor = entry.color ?? ''
    editReminderAt = entry.reminder_at ? new Date(entry.reminder_at).toISOString().slice(0, 16) : ''
    editTags = [...(entry.tags || [])]
    editPin = entry.is_pinned ?? false
  }

  function closeEdit() {
    editingNote = null
  }

  async function saveEdit() {
    if (!editingNote) return
    try {
      await updateEntry({
        ...editingNote,
        title: editTitle.trim() || null,
        content: editContent.trim() || editingNote.content,
        color: editColor || null,
        reminder_at: editReminderAt.trim() ? new Date(editReminderAt.trim()).toISOString() : null,
        tags: [...editTags],
        is_pinned: editPin,
        updated_at: new Date().toISOString()
      })
      closeEdit()
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    }
  }

  function addNewTag(tag: string) {
    const t = tag.trim()
    if (t && !newTags.includes(t)) newTags = [...newTags, t]
    newLabelInput = ''
    labelDropdownOpen = false
  }

  function removeNewTag(tag: string) {
    newTags = newTags.filter((x) => x !== tag)
  }

  function addEditTag(tag: string) {
    const t = tag.trim()
    if (t && !editTags.includes(t)) editTags = [...editTags, t]
  }

  function removeEditTag(tag: string) {
    editTags = editTags.filter((x) => x !== tag)
  }

  function formatDate(iso: string) {
    try {
      const d = new Date(iso)
      const today = new Date()
      if (d.toDateString() === today.toDateString()) {
        return `Today, ${d.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })}`
      }
      return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
    } catch {
      return iso
    }
  }

  function formatReminder(iso: string | null) {
    if (!iso) return ''
    try {
      const d = new Date(iso)
      const today = new Date()
      if (d.toDateString() === today.toDateString()) {
        return `Today ${d.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })}`
      }
      return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' })
    } catch {
      return iso
    }
  }

  function expandComposer() {
    isComposerExpanded = true
  }

  function collapseComposer() {
    if (!newTitle.trim() && !newContent.trim()) {
      isComposerExpanded = false
    }
  }

</script>

<div class="view notes-view">
  <header class="page-header">
    <div class="header-content">
      <div class="title-wrapper">
        <Icon icon="ph:note-duotone" class="header-icon" />
        <h2>Notes</h2>
      </div>
      <p class="subtitle">Jot down your thoughts, ideas, and transcriptions.</p>
    </div>
    <div class="sub-nav">
      <button type="button" class="sub-nav-btn" class:active={notesScope === 'active'} on:click={() => notesScope = 'active'}>Notes</button>
      <button type="button" class="sub-nav-btn" class:active={notesScope === 'archived'} on:click={() => notesScope = 'archived'}>Archive</button>
      <button type="button" class="sub-nav-btn" class:active={notesScope === 'trash'} on:click={() => notesScope = 'trash'}>Trash</button>
    </div>
    <div class="search-row">
      <div class="search-wrap">
        <Icon icon="ph:magnifying-glass-duotone" class="search-icon" />
        <input type="text" class="search-input" bind:value={searchQuery} placeholder="Search notes..." />
      </div>
      {#if notesScope === 'active' && labels.length > 0}
        <div class="label-chips">
          <button type="button" class="label-chip" class:active={selectedLabel === null} on:click={() => selectedLabel = null}>All</button>
          {#each labels as label}
            <button type="button" class="label-chip" class:active={selectedLabel === label} on:click={() => selectedLabel = selectedLabel === label ? null : label}>{label}</button>
          {/each}
        </div>
      {/if}
    </div>
  </header>

  {#if notesScope !== 'trash'}
  <div class="composer-container" class:expanded={isComposerExpanded}>
    <div class="composer-card" style:background-color={newColor || 'var(--bg-card)'}>
      {#if isComposerExpanded}
        <input type="text" bind:value={newTitle} placeholder="Title" class="title-input" />
      {/if}
      <textarea
        bind:value={newContent}
        placeholder="Take a note..."
        rows={isComposerExpanded ? 4 : 1}
        class="content-input"
        on:focus={expandComposer}
        on:blur={collapseComposer}
      ></textarea>
      {#if isComposerExpanded}
        <div class="composer-toolbar">
          <div class="color-palette">
            {#each NOTE_COLORS as c}
              <button type="button" class="color-dot" class:selected={newColor === c.value} style:background-color={c.value || 'var(--bg-card)'} on:click={() => newColor = c.value} title={c.name}></button>
            {/each}
          </div>
          <div class="toolbar-right">
            <input type="datetime-local" bind:value={newReminderAt} class="toolbar-datetime" title="Remind me" />
            <div class="label-dropdown-wrap">
              <button type="button" class="toolbar-btn" on:click={() => labelDropdownOpen = !labelDropdownOpen} title="Labels">
                <Icon icon="ph:tag-duotone" />
              </button>
              {#if labelDropdownOpen}
                <div class="label-dropdown">
                  {#each labels as label}
                    <button type="button" class="label-option" on:click={() => addNewTag(label)}>{label}</button>
                  {/each}
                  <div class="label-add">
                    <input type="text" bind:value={newLabelInput} placeholder="New label" on:keydown={(e) => e.key === 'Enter' && addNewTag(newLabelInput)} />
                  </div>
                </div>
              {/if}
            </div>
            <button type="button" class="toolbar-btn" class:active={newPin} on:click={() => newPin = !newPin} title="Pin">
              <Icon icon="ph:push-pin-duotone" />
            </button>
          </div>
        </div>
        {#if newTags.length > 0}
          <div class="composer-tags">
            {#each newTags as tag}
              <span class="tag-chip"><span>{tag}</span><button type="button" on:click={() => removeNewTag(tag)} aria-label="Remove"><Icon icon="ph:x" /></button></span>
            {/each}
          </div>
        {/if}
        <div class="composer-actions">
          <button type="button" class="btn-ghost" on:click={() => { newTitle=''; newContent=''; newColor=''; newReminderAt=''; newTags=[]; newPin=false; isComposerExpanded=false; }}>Close</button>
          <button type="button" class="btn-primary" on:click={addNote} disabled={!newTitle.trim() && !newContent.trim()}>
            <Icon icon="ph:plus-bold" /> Add Note
          </button>
        </div>
      {/if}
    </div>
  </div>
  {/if}

  {#if notesScope === 'trash' && entries.length > 0}
    <div class="empty-trash-row">
      <button type="button" class="btn-ghost danger" on:click={emptyTrashConfirm}>Empty trash</button>
    </div>
  {/if}

  {#if error}
    <div class="state-container error-state">
      <Icon icon="ph:warning-circle-duotone" class="error-icon" />
      <p>{error}</p>
    </div>
  {/if}

  {#if loading && entries.length === 0}
    <div class="state-container">
      <Icon icon="ph:spinner-gap-duotone" class="spin-icon" />
      <p>Loading notes...</p>
    </div>
  {:else if entries.length === 0}
    <div class="state-container empty-state">
      <div class="empty-icon-wrapper">
        <Icon icon="ph:notebook-duotone" class="empty-icon" />
      </div>
      <h3>{notesScope === 'trash' ? 'Trash is empty' : notesScope === 'archived' ? 'No archived notes' : (searchQuery || selectedLabel) ? 'No results' : 'No notes yet'}</h3>
      <p>{notesScope === 'trash' ? 'Deleted notes will appear here.' : 'Your captured thoughts will appear here.'}</p>
    </div>
  {:else}
    <div class="notes-grid">
      {#each entries as entry (entry.id)}
        <article class="note-card" class:pinned={entry.is_pinned} style:background-color={entry.color || 'var(--bg-card)'} role="button" on:click={() => openEdit(entry)} on:keydown={(e) => e.key === 'Enter' && openEdit(entry)} tabindex="0">
          <div class="note-inner">
            {#if entry.title}
              <h3 class="note-title">{entry.title}</h3>
            {/if}
            <div class="note-content markdown" class:no-title={!entry.title}>
              {#if entry.content?.trim()}
                {@html renderMarkdown(entry.content)}
              {:else}
                <span class="empty-placeholder">(empty)</span>
              {/if}
            </div>
            {#if (entry.tags?.length ?? 0) > 0}
              <div class="note-tags">
                {#each entry.tags as tag}
                  <span class="tag-chip">{tag}</span>
                {/each}
              </div>
            {/if}
            {#if entry.reminder_at}
              <div class="note-reminder">
                <Icon icon="ph:bell-duotone" />
                {formatReminder(entry.reminder_at)}
              </div>
            {/if}
          </div>
          <div class="note-footer" on:click|stopPropagation on:keydown|stopPropagation>
            <span class="note-date">{formatDate(entry.updated_at)}</span>
            <div class="note-actions">
              {#if notesScope === 'active'}
                <button type="button" class="action-btn" on:click={() => archiveNote(entry)} title="Archive"><Icon icon="ph:archive-duotone" /></button>
                <button type="button" class="action-btn delete" on:click={() => moveToTrash(entry)} title="Delete"><Icon icon="ph:trash-duotone" /></button>
              {:else if notesScope === 'archived'}
                <button type="button" class="action-btn" on:click={() => unarchiveNote(entry)} title="Unarchive"><Icon icon="ph:archive-tray-duotone" /></button>
                <button type="button" class="action-btn delete" on:click={() => moveToTrash(entry)} title="Delete"><Icon icon="ph:trash-duotone" /></button>
              {:else}
                <button type="button" class="action-btn" on:click={() => restoreNote(entry)} title="Restore"><Icon icon="ph:arrow-counter-clockwise-duotone" /></button>
                <button type="button" class="action-btn delete" on:click={() => permanentlyDelete(entry.id)} title="Delete permanently"><Icon icon="ph:trash-duotone" /></button>
              {/if}
            </div>
          </div>
        </article>
      {/each}
    </div>
  {/if}

  {#if editingNote}
    <div class="edit-modal-backdrop" role="presentation" on:click={closeEdit} on:keydown={(e) => e.key === 'Escape' && closeEdit()}>
      <div class="edit-modal" role="dialog" on:click|stopPropagation on:keydown|stopPropagation>
        <h3 class="edit-modal-title">Edit note</h3>
        <div class="edit-modal-body">
          <input type="text" class="edit-input" bind:value={editTitle} placeholder="Title" />
          <textarea class="edit-textarea" bind:value={editContent} placeholder="Content" rows="6"></textarea>
          <div class="edit-toolbar">
            <div class="color-palette">
              {#each NOTE_COLORS as c}
                <button type="button" class="color-dot" class:selected={editColor === c.value} style:background-color={c.value || 'var(--bg-card)'} on:click={() => editColor = c.value}></button>
              {/each}
            </div>
            <input type="datetime-local" bind:value={editReminderAt} class="toolbar-datetime" />
            <button type="button" class="toolbar-btn" class:active={editPin} on:click={() => editPin = !editPin}><Icon icon="ph:push-pin-duotone" /></button>
          </div>
          {#if editTags.length > 0}
            <div class="edit-tags">
              {#each editTags as tag}
                <span class="tag-chip"><span>{tag}</span><button type="button" on:click={() => removeEditTag(tag)}><Icon icon="ph:x" /></button></span>
              {/each}
            </div>
          {/if}
          <div class="edit-labels-add">
            {#each labels as label}
              {#if !editTags.includes(label)}
                <button type="button" class="label-option" on:click={() => addEditTag(label)}>{label}</button>
              {/if}
            {/each}
          </div>
        </div>
        <div class="edit-modal-actions">
          <button type="button" class="btn-ghost" on:click={closeEdit}>Cancel</button>
          {#if notesScope === 'trash'}
            <button type="button" class="btn-ghost danger" on:click={() => permanentlyDelete(editingNote.id)}>Delete permanently</button>
          {/if}
          <button type="button" class="btn-primary" on:click={saveEdit}>Save</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .view {
    max-width: 1000px;
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

  .sub-nav {
    display: flex;
    gap: 4px;
    margin-top: 16px;
  }

  .sub-nav-btn {
    padding: 8px 16px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 14px;
    font-weight: 500;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .sub-nav-btn:hover {
    background: var(--bg-input);
    color: var(--text-primary);
  }

  .sub-nav-btn.active {
    background: var(--primary);
    color: white;
  }

  .search-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    margin-top: 12px;
  }

  .search-wrap {
    display: flex;
    align-items: center;
    background: var(--bg-input);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    padding: 8px 12px;
    flex: 1;
    min-width: 200px;
  }

  .search-icon {
    font-size: 18px;
    color: var(--text-muted);
    margin-right: 8px;
  }

  .search-input {
    border: none;
    background: transparent;
    font-size: 14px;
    color: var(--text-primary);
    flex: 1;
  }

  .search-input:focus {
    outline: none;
  }

  .label-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .label-chip {
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-card);
    font-size: 12px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s;
  }

  .label-chip:hover,
  .label-chip.active {
    background: var(--primary);
    color: white;
    border-color: var(--primary);
  }

  /* Composer */
  .composer-container {
    max-width: 600px;
    margin: 0 auto;
    width: 100%;
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .composer-card {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.03), 0 1px 3px rgba(0, 0, 0, 0.02);
    overflow: hidden;
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    display: flex;
    flex-direction: column;
  }

  .composer-container.expanded .composer-card {
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.08), 0 1px 3px rgba(0, 0, 0, 0.02);
    border-color: var(--border-visible);
    transform: translateY(-2px);
  }

  .title-input {
    width: 100%;
    border: none;
    padding: 16px 20px 4px;
    font-size: 16px;
    font-weight: 700;
    color: var(--navy-deep);
    background: transparent;
    font-family: inherit;
  }

  .title-input:focus {
    outline: none;
  }

  .title-input::placeholder {
    color: var(--text-muted);
    font-weight: 600;
  }

  .content-input {
    width: 100%;
    border: none;
    padding: 16px 20px;
    font-size: 15px;
    color: var(--text-primary);
    background: transparent;
    resize: none;
    font-family: inherit;
    line-height: 1.5;
  }

  .content-input:focus {
    outline: none;
  }

  .composer-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 8px;
    padding: 8px 16px;
    border-top: 1px solid var(--border-subtle);
  }

  .color-palette {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .color-dot {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    transition: all 0.2s;
  }

  .color-dot:hover {
    transform: scale(1.1);
  }

  .color-dot.selected {
    border-color: var(--navy-deep);
    box-shadow: 0 0 0 1px var(--navy-deep);
  }

  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .toolbar-datetime {
    font-size: 12px;
    padding: 4px 8px;
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    background: var(--bg-app);
    color: var(--text-primary);
  }

  .toolbar-btn {
    padding: 6px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .toolbar-btn:hover,
  .toolbar-btn.active {
    background: var(--bg-input);
    color: var(--primary);
  }

  .label-dropdown-wrap {
    position: relative;
  }

  .label-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    margin-top: 4px;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.12);
    padding: 8px;
    min-width: 160px;
    z-index: 10;
  }

  .label-option {
    display: block;
    width: 100%;
    padding: 8px 12px;
    text-align: left;
    border: none;
    background: transparent;
    font-size: 13px;
    color: var(--text-primary);
    border-radius: 6px;
    cursor: pointer;
  }

  .label-option:hover {
    background: var(--bg-input);
  }

  .label-add {
    padding: 8px 0 0;
    border-top: 1px solid var(--border-subtle);
    margin-top: 4px;
  }

  .label-add input {
    width: 100%;
    padding: 6px 10px;
    font-size: 13px;
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    background: var(--bg-app);
  }

  .composer-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 0 16px 8px;
  }

  .tag-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    background: var(--bg-input);
    border-radius: 999px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .tag-chip button {
    padding: 0;
    border: none;
    background: transparent;
    color: inherit;
    cursor: pointer;
    display: flex;
    align-items: center;
  }

  .composer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    background: var(--bg-app);
    border-top: 1px solid var(--border-subtle);
  }

  .btn-ghost.danger {
    color: var(--error, #dc2626);
  }

  .btn-ghost.danger:hover {
    background: rgba(220, 38, 38, 0.1);
  }

  .empty-trash-row {
    padding: 8px 0;
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

  /* Grid */
  .notes-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 20px;
    align-items: start;
  }

  .note-card {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    display: flex;
    flex-direction: column;
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    position: relative;
    overflow: hidden;
    cursor: pointer;
  }

  .note-card:hover {
    transform: translateY(-4px);
    box-shadow: 0 12px 24px rgba(0, 0, 0, 0.06);
    border-color: var(--border-visible);
  }

  .note-card.pinned::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 4px;
    background: var(--primary);
  }

  .note-inner {
    padding: 20px;
    flex: 1;
  }

  .note-title {
    font-size: 16px;
    font-weight: 700;
    color: var(--navy-deep);
    margin: 0 0 8px 0;
    line-height: 1.3;
  }

  .note-content {
    font-size: 15px;
    line-height: 1.6;
    color: var(--text-primary);
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    display: -webkit-box;
    -webkit-line-clamp: 8;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .note-content.no-title {
    font-size: 16px;
    color: var(--navy-deep);
  }

  .note-content.markdown {
    white-space: normal;
  }

  .note-content.markdown :global(p) {
    margin: 0 0 0.5em 0;
  }

  .note-content.markdown :global(p:last-child) {
    margin-bottom: 0;
  }

  .note-content.markdown :global(ul),
  .note-content.markdown :global(ol) {
    margin: 0.5em 0;
    padding-left: 1.25em;
  }

  .note-content.markdown :global(strong) {
    font-weight: 700;
  }

  .note-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 8px;
  }

  .note-tags .tag-chip {
    padding: 2px 8px;
    font-size: 11px;
  }

  .note-reminder {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .note-reminder :global(svg) {
    font-size: 14px;
    flex-shrink: 0;
  }

  .note-footer {
    padding: 12px 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px dashed var(--border-subtle);
    background: rgba(0,0,0,0.01);
  }

  .note-date {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .note-actions {
    display: flex;
    gap: 4px;
    opacity: 0;
    transition: opacity 0.2s;
  }

  .note-card:hover .note-actions {
    opacity: 1;
  }

  .action-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
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

  /* Edit modal */
  .edit-modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 20px;
  }

  .edit-modal {
    background: var(--bg-card);
    border-radius: 16px;
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.15);
    max-width: 560px;
    width: 100%;
    max-height: 90vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .edit-modal-title {
    padding: 20px 24px;
    margin: 0;
    font-size: 18px;
    font-weight: 700;
    border-bottom: 1px solid var(--border-subtle);
  }

  .edit-modal-body {
    padding: 20px 24px;
    overflow-y: auto;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .edit-input {
    width: 100%;
    padding: 12px 16px;
    font-size: 16px;
    font-weight: 600;
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    background: var(--bg-app);
    color: var(--navy-deep);
  }

  .edit-input:focus {
    outline: none;
    border-color: var(--primary);
  }

  .edit-textarea {
    width: 100%;
    padding: 12px 16px;
    font-size: 14px;
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    background: var(--bg-app);
    color: var(--text-primary);
    resize: vertical;
    font-family: inherit;
  }

  .edit-textarea:focus {
    outline: none;
    border-color: var(--primary);
  }

  .edit-toolbar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }

  .edit-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .edit-labels-add {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .edit-modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 24px;
    border-top: 1px solid var(--border-subtle);
  }

  @media (max-width: 768px) {
    .note-actions {
      opacity: 1;
    }
    .subtitle {
      padding-left: 0;
    }
  }
</style>
