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
  import SidePanel from '../ui/SidePanel.svelte'
  import SearchFilterBar from '../ui/SearchFilterBar.svelte'

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
  
  // Panel State
  let isPanelOpen = false
  let panelMode: 'add' | 'edit' = 'add'
  let panelNoteId: string | null = null
  let draftEntry: Partial<Entry> & { tags: string[] } = { tags: [] }
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

  function openAddPanel() {
    panelMode = 'add'
    panelNoteId = null
    draftEntry = {
      title: '',
      content: '',
      color: '',
      reminder_at: null,
      tags: [],
      is_pinned: false
    }
    isPanelOpen = true
  }

  function openEditPanel(entry: Entry) {
    panelMode = 'edit'
    panelNoteId = entry.id
    draftEntry = { 
      ...entry, 
      tags: [...(entry.tags || [])],
      reminder_at: entry.reminder_at ? new Date(entry.reminder_at).toISOString().slice(0, 16) : null
    }
    isPanelOpen = true
  }

  function closePanel() {
    isPanelOpen = false
    panelNoteId = null
    labelDropdownOpen = false
  }

  async function savePanel() {
    const content = draftEntry.content?.trim() || draftEntry.title?.trim()
    if (!content) return

    try {
      const reminderAt = draftEntry.reminder_at?.trim() ? new Date(draftEntry.reminder_at.trim()).toISOString() : null

      if (panelMode === 'add') {
        const entry = newEntry('note', draftEntry.content || '', {
          title: draftEntry.title?.trim() || null,
          color: draftEntry.color || null,
          reminder_at: reminderAt,
          tags: [...draftEntry.tags],
          is_pinned: draftEntry.is_pinned ?? false,
          archived_at: null,
          deleted_at: null
        })
        await createEntry(entry)
      } else if (panelMode === 'edit' && panelNoteId) {
        const original = entries.find(e => e.id === panelNoteId)
        if (!original) return
        
        await updateEntry({
          ...original,
          title: draftEntry.title?.trim() || null,
          content: draftEntry.content?.trim() || original.content,
          color: draftEntry.color || null,
          reminder_at: reminderAt,
          tags: [...draftEntry.tags],
          is_pinned: draftEntry.is_pinned ?? false,
          updated_at: new Date().toISOString()
        })
      }
      
      closePanel()
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
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
      if (panelNoteId === id) closePanel()
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

  function addTag(tag: string) {
    const t = tag.trim()
    if (t && !draftEntry.tags.includes(t)) draftEntry.tags = [...draftEntry.tags, t]
    newLabelInput = ''
    labelDropdownOpen = false
  }

  function removeTag(tag: string) {
    draftEntry.tags = draftEntry.tags.filter((x) => x !== tag)
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
    <div class="header-actions">
      {#if notesScope !== 'trash'}
        <button class="btn-primary" on:click={openAddPanel}>
          <Icon icon="ph:plus-bold" /> Add Note
        </button>
      {/if}
    </div>
  </header>

  <div class="sub-nav">
    <button type="button" class="sub-nav-btn" class:active={notesScope === 'active'} on:click={() => notesScope = 'active'}>Notes</button>
    <button type="button" class="sub-nav-btn" class:active={notesScope === 'archived'} on:click={() => notesScope = 'archived'}>Archive</button>
    <button type="button" class="sub-nav-btn" class:active={notesScope === 'trash'} on:click={() => notesScope = 'trash'}>Trash</button>
  </div>

  <SearchFilterBar bind:searchQuery placeholder="Search notes...">
    <svelte:fragment slot="filters">
      {#if notesScope === 'active' && labels.length > 0}
        <button type="button" class="label-chip" class:active={selectedLabel === null} on:click={() => selectedLabel = null}>All</button>
        {#each labels as label}
          <button type="button" class="label-chip" class:active={selectedLabel === label} on:click={() => selectedLabel = selectedLabel === label ? null : label}>{label}</button>
        {/each}
      {/if}
    </svelte:fragment>
  </SearchFilterBar>

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
        <div class="note-card" class:pinned={entry.is_pinned} style:background-color={entry.color || 'var(--bg-card)'} role="button" tabindex="0" on:click={() => openEditPanel(entry)} on:keydown={(e) => e.key === 'Enter' && openEditPanel(entry)}>
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
          <div class="note-footer" role="group" on:click|stopPropagation on:keydown|stopPropagation>
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
        </div>
      {/each}
    </div>
  {/if}

  <SidePanel 
    isOpen={isPanelOpen} 
    title={panelMode === 'add' ? 'Add Note' : 'Edit Note'} 
    on:close={closePanel}
  >
    <div slot="body" class="panel-form">
      <input type="text" class="edit-input" bind:value={draftEntry.title} placeholder="Title" />
      <textarea class="edit-textarea" bind:value={draftEntry.content} placeholder="Content" rows="6"></textarea>
      
      <div class="edit-toolbar">
        <div class="color-palette">
          {#each NOTE_COLORS as c}
            <button type="button" class="color-dot" class:selected={draftEntry.color === c.value} style:background-color={c.value || 'var(--bg-card)'} on:click={() => draftEntry.color = c.value}></button>
          {/each}
        </div>
        <input type="datetime-local" bind:value={draftEntry.reminder_at} class="toolbar-datetime" />
        <button type="button" class="toolbar-btn" class:active={draftEntry.is_pinned} on:click={() => draftEntry.is_pinned = !draftEntry.is_pinned}><Icon icon="ph:push-pin-duotone" /></button>
      </div>
      
      {#if draftEntry.tags.length > 0}
        <div class="edit-tags">
          {#each draftEntry.tags as tag}
            <span class="tag-chip"><span>{tag}</span><button type="button" on:click={() => removeTag(tag)}><Icon icon="ph:x" /></button></span>
          {/each}
        </div>
      {/if}
      
      <div class="edit-labels-add">
        <div class="label-dropdown-wrap">
          <button type="button" class="btn-ghost" on:click={() => labelDropdownOpen = !labelDropdownOpen}>
            <Icon icon="ph:tag-duotone" /> Add Label
          </button>
          {#if labelDropdownOpen}
            <div class="label-dropdown">
              {#each labels as label}
                {#if !draftEntry.tags.includes(label)}
                  <button type="button" class="label-option" on:click={() => addTag(label)}>{label}</button>
                {/if}
              {/each}
              <div class="label-add">
                <input type="text" bind:value={newLabelInput} placeholder="New label" on:keydown={(e) => e.key === 'Enter' && addTag(newLabelInput)} />
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>
    
    <div slot="footer">
      <button class="btn-ghost" on:click={closePanel}>Cancel</button>
      {#if notesScope === 'trash' && panelMode === 'edit' && panelNoteId}
        <button type="button" class="btn-ghost danger" on:click={() => permanentlyDelete(panelNoteId || '')}>Delete permanently</button>
      {/if}
      <button class="btn-primary" on:click={savePanel} disabled={!draftEntry.title?.trim() && !draftEntry.content?.trim()}>
        <Icon icon="ph:check-bold" /> Save
      </button>
    </div>
  </SidePanel>
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

  .sub-nav {
    display: flex;
    gap: 4px;
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

  /* Panel Form */
  .panel-form {
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

  .edit-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
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

  .edit-labels-add {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
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

  @media (max-width: 768px) {
    .note-actions {
      opacity: 1;
    }
    .subtitle {
      padding-left: 0;
    }
  }
</style>
