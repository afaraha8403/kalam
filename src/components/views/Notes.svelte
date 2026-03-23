<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import {
    getNotes,
    searchNotes,
    getNoteLabels,
    updateEntry,
    deleteEntry,
    emptyTrash
  } from '../../lib/api/db'
  import type { Entry } from '../../types'
  import Icon from '@iconify/svelte'
  import { flip } from 'svelte/animate'
  import { selectedNoteId } from '../../lib/noteDetailStore'

  export let navigate: (page: string) => void = () => {}

  /** Drag state for prototype-matching card reorder (visual only; order not persisted). */
  let dragNoteId: string | null = null
  function handleNoteDragStart(e: DragEvent, id: string) {
    dragNoteId = id
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'
  }
  function handleNoteDragOver(e: DragEvent) {
    e.preventDefault()
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
  }
  function handleNoteDrop(e: DragEvent, _targetId: string) {
    e.preventDefault()
    dragNoteId = null
  }
  function handleNoteDragEnd() {
    dragNoteId = null
  }

  let entries: Entry[] = []
  let labels: string[] = []
  let loading = true
  let error: string | null = null
  let notesScope: 'active' | 'archived' | 'trash' = 'active'
  let searchQuery = ''
  let searchDebounced = ''
  let selectedLabel: string | null = null
  let searchTimeout: ReturnType<typeof setTimeout>

  type NotesSortMode = 'updated_desc' | 'updated_asc' | 'title_asc' | 'title_desc'
  let sortMode: NotesSortMode = 'updated_desc'
  let scopeMenuOpen = false
  let scopeDropdownEl: HTMLDivElement | null = null

  function closeScopeMenuIfOutside(e: PointerEvent) {
    if (!scopeMenuOpen) return
    const el = scopeDropdownEl
    const t = e.target
    if (el && t instanceof Node && el.contains(t)) return
    scopeMenuOpen = false
  }

  function onGlobalKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && scopeMenuOpen) scopeMenuOpen = false
  }

  onMount(() => {
    document.addEventListener('pointerdown', closeScopeMenuIfOutside)
    document.addEventListener('keydown', onGlobalKeydown)
  })
  onDestroy(() => {
    document.removeEventListener('pointerdown', closeScopeMenuIfOutside)
    document.removeEventListener('keydown', onGlobalKeydown)
  })

  /** Search, sort, label chips — hidden until there is data, filters, or an error (not during first-load empty). */
  $: showNotesListChrome =
    !!error ||
    entries.length > 0 ||
    !!searchQuery.trim() ||
    selectedLabel !== null

  $: scopeDisplayLabel =
    notesScope === 'active' ? 'Notes' : notesScope === 'archived' ? 'Archive' : 'Trash'

  function pickScope(scope: typeof notesScope) {
    notesScope = scope
    scopeMenuOpen = false
  }

  function compareNotes(a: Entry, b: Entry): number {
    if (sortMode === 'title_asc' || sortMode === 'title_desc') {
      const ta = (a.title || '').trim().toLowerCase()
      const tb = (b.title || '').trim().toLowerCase()
      const c = ta.localeCompare(tb, undefined, { sensitivity: 'base' })
      return sortMode === 'title_asc' ? c : -c
    }
    const da = new Date(a.updated_at).getTime()
    const db = new Date(b.updated_at).getTime()
    return sortMode === 'updated_asc' ? da - db : db - da
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
    selectedNoteId.set(null)
    navigate('note-detail')
  }

  function openEditPanel(entry: Entry) {
    selectedNoteId.set(entry.id)
    navigate('note-detail')
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

  $: pinnedNotes = entries
    .filter((e) => e.is_pinned)
    .slice()
    .sort(compareNotes)
  $: otherNotes = entries
    .filter((e) => !e.is_pinned)
    .slice()
    .sort(compareNotes)
</script>

<div class="page fade-in">
  <header class="page-header notes-header">
    <div>
      <h1 class="page-title">Notes</h1>
      <p class="page-subtitle">Jot down your thoughts, ideas, and transcriptions.</p>
    </div>
    {#if notesScope !== 'trash'}
      <button type="button" class="btn-primary" on:click={openAddPanel}>
        <Icon icon="ph:plus" />
        New Note
      </button>
    {/if}
  </header>

  <div class="notes-toolbar">
    <!-- Custom menu: native <details> does not reliably close on outside click in embedded WebViews. -->
    <div class="notes-scope-dropdown" bind:this={scopeDropdownEl}>
      <button
        type="button"
        class="notes-scope-trigger"
        aria-label="Notes location: {scopeDisplayLabel}"
        aria-expanded={scopeMenuOpen}
        aria-haspopup="listbox"
        on:click|stopPropagation={() => (scopeMenuOpen = !scopeMenuOpen)}
      >
        <span>{scopeDisplayLabel}</span>
        <span class="notes-scope-caret" class:open={scopeMenuOpen} aria-hidden="true">
          <Icon icon="ph:caret-down" />
        </span>
      </button>
      {#if scopeMenuOpen}
        <div class="notes-scope-menu" role="listbox">
          <button type="button" class="notes-scope-option" class:active={notesScope === 'active'} on:click={() => pickScope('active')}>Notes</button>
          <button type="button" class="notes-scope-option" class:active={notesScope === 'archived'} on:click={() => pickScope('archived')}>Archive</button>
          <button type="button" class="notes-scope-option" class:active={notesScope === 'trash'} on:click={() => pickScope('trash')}>Trash</button>
        </div>
      {/if}
    </div>
    {#if showNotesListChrome}
      <select class="notes-sort-select" bind:value={sortMode} aria-label="Sort notes">
        <option value="updated_desc">Newest first</option>
        <option value="updated_asc">Oldest first</option>
        <option value="title_asc">Title A–Z</option>
        <option value="title_desc">Title Z–A</option>
      </select>
    {/if}
  </div>

  {#if showNotesListChrome}
    <div class="notes-search-bar">
      <span class="notes-search-bar-icon" aria-hidden="true">
        <Icon icon="ph:magnifying-glass" />
      </span>
      <input type="text" bind:value={searchQuery} placeholder="Search notes..." />
    </div>
    {#if notesScope === 'active' && labels.length > 0}
      <div class="notes-label-filters">
        <button type="button" class="label-chip" class:active={selectedLabel === null} on:click={() => selectedLabel = null}>All</button>
        {#each labels as label}
          <button type="button" class="label-chip" class:active={selectedLabel === label} on:click={() => selectedLabel = selectedLabel === label ? null : label}>{label}</button>
        {/each}
      </div>
    {/if}

    {#if notesScope === 'trash' && entries.length > 0}
      <div style="margin-bottom: var(--space-md);">
        <button type="button" class="btn-ghost danger" on:click={emptyTrashConfirm}>Empty trash</button>
      </div>
    {/if}
  {/if}

  {#if error}
    <div class="state-container">
      <Icon icon="ph:warning-circle" />
      <p>{error}</p>
    </div>
  {:else if loading && entries.length === 0}
    <div class="state-container">
      <Icon icon="ph:spinner-gap-duotone" />
      <p>Loading notes...</p>
    </div>
  {:else if entries.length === 0}
    <div class="notes-empty">
      <Icon icon="ph:notebook" />
      <p>{notesScope === 'trash' ? 'Trash is empty' : notesScope === 'archived' ? 'No archived notes' : (searchQuery || selectedLabel) ? 'No results' : 'No notes yet'}</p>
    </div>
  {:else}
    <div class="notes-lists-container">
      {#if pinnedNotes.length > 0}
        <div class="notes-section">
          <h3 class="notes-section-title">Pinned</h3>
          <div class="notes-masonry">
            {#each pinnedNotes as entry (entry.id)}
              <article
                class="note-card"
                class:pinned={entry.is_pinned}
                class:dragging={dragNoteId === entry.id}
                class:has-custom-color={!!entry.color}
                style:background-color={entry.color || 'var(--bg-elevated)'}
                animate:flip={{ duration: 250 }}
                role="button"
                tabindex="0"
                draggable={notesScope === 'active'}
                on:click={() => openEditPanel(entry)}
                on:keydown={(e) => e.key === 'Enter' && openEditPanel(entry)}
                on:dragstart={(e) => handleNoteDragStart(e, entry.id)}
                on:dragover={handleNoteDragOver}
                on:drop={(e) => handleNoteDrop(e, entry.id)}
                on:dragend={handleNoteDragEnd}
              >
                <div class="note-inner">
                  {#if entry.is_pinned}
                    <div class="pin-icon"><Icon icon="ph:push-pin-fill" /></div>
                  {/if}
                  <h4 class="note-title" class:note-title-placeholder={!entry.title?.trim()}>
                    {entry.title?.trim() ? entry.title : 'Untitled'}
                  </h4>
                  {#if (entry.tags?.length ?? 0) > 0 || entry.reminder_at}
                    <div class="note-card-meta">
                      {#if (entry.tags?.length ?? 0) > 0}
                        <div class="note-tags-row">
                          {#each entry.tags as tag}
                            <span class="note-tag">{tag}</span>
                          {/each}
                        </div>
                      {/if}
                      {#if entry.reminder_at}
                        <div class="note-reminder-row">
                          <Icon icon="ph:bell" />
                          {formatReminder(entry.reminder_at)}
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>
                <div class="note-footer" role="group" on:click|stopPropagation on:keydown|stopPropagation>
                  <span class="note-date">{formatDate(entry.updated_at)}</span>
                  <div class="note-actions">
                    {#if notesScope === 'active'}
                      <button type="button" class="note-action-btn" on:click|stopPropagation={() => archiveNote(entry)} title="Archive" aria-label="Archive"><Icon icon="ph:archive" /></button>
                      <button type="button" class="note-action-btn delete" on:click|stopPropagation={() => moveToTrash(entry)} title="Delete" aria-label="Delete"><Icon icon="ph:trash" /></button>
                    {:else if notesScope === 'archived'}
                      <button type="button" class="note-action-btn" on:click|stopPropagation={() => unarchiveNote(entry)} title="Unarchive" aria-label="Unarchive"><Icon icon="ph:archive-tray" /></button>
                      <button type="button" class="note-action-btn delete" on:click|stopPropagation={() => moveToTrash(entry)} title="Delete" aria-label="Delete"><Icon icon="ph:trash" /></button>
                    {:else}
                      <button type="button" class="note-action-btn" on:click|stopPropagation={() => restoreNote(entry)} title="Restore" aria-label="Restore"><Icon icon="ph:arrow-counter-clockwise" /></button>
                      <button type="button" class="note-action-btn delete" on:click|stopPropagation={() => permanentlyDelete(entry.id)} title="Delete permanently" aria-label="Delete permanently"><Icon icon="ph:trash" /></button>
                    {/if}
                  </div>
                </div>
              </article>
            {/each}
          </div>
        </div>
      {/if}
      {#if otherNotes.length > 0}
        <div class="notes-section">
          {#if pinnedNotes.length > 0}
            <h3 class="notes-section-title">Others</h3>
          {/if}
          <div class="notes-masonry">
            {#each otherNotes as entry (entry.id)}
              <article
                class="note-card"
                class:pinned={entry.is_pinned}
                class:dragging={dragNoteId === entry.id}
                class:has-custom-color={!!entry.color}
                style:background-color={entry.color || 'var(--bg-elevated)'}
                animate:flip={{ duration: 250 }}
                role="button"
                tabindex="0"
                draggable={notesScope === 'active'}
                on:click={() => openEditPanel(entry)}
                on:keydown={(e) => e.key === 'Enter' && openEditPanel(entry)}
                on:dragstart={(e) => handleNoteDragStart(e, entry.id)}
                on:dragover={handleNoteDragOver}
                on:drop={(e) => handleNoteDrop(e, entry.id)}
                on:dragend={handleNoteDragEnd}
              >
                <div class="note-inner">
                  {#if entry.is_pinned}
                    <div class="pin-icon"><Icon icon="ph:push-pin-fill" /></div>
                  {/if}
                  <h4 class="note-title" class:note-title-placeholder={!entry.title?.trim()}>
                    {entry.title?.trim() ? entry.title : 'Untitled'}
                  </h4>
                  {#if (entry.tags?.length ?? 0) > 0 || entry.reminder_at}
                    <div class="note-card-meta">
                      {#if (entry.tags?.length ?? 0) > 0}
                        <div class="note-tags-row">
                          {#each entry.tags as tag}
                            <span class="note-tag">{tag}</span>
                          {/each}
                        </div>
                      {/if}
                      {#if entry.reminder_at}
                        <div class="note-reminder-row">
                          <Icon icon="ph:bell" />
                          {formatReminder(entry.reminder_at)}
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>
                <div class="note-footer" role="group" on:click|stopPropagation on:keydown|stopPropagation>
                  <span class="note-date">{formatDate(entry.updated_at)}</span>
                  <div class="note-actions">
                    {#if notesScope === 'active'}
                      <button type="button" class="note-action-btn" on:click|stopPropagation={() => archiveNote(entry)} title="Archive" aria-label="Archive"><Icon icon="ph:archive" /></button>
                      <button type="button" class="note-action-btn delete" on:click|stopPropagation={() => moveToTrash(entry)} title="Delete" aria-label="Delete"><Icon icon="ph:trash" /></button>
                    {:else if notesScope === 'archived'}
                      <button type="button" class="note-action-btn" on:click|stopPropagation={() => unarchiveNote(entry)} title="Unarchive" aria-label="Unarchive"><Icon icon="ph:archive-tray" /></button>
                      <button type="button" class="note-action-btn delete" on:click|stopPropagation={() => moveToTrash(entry)} title="Delete" aria-label="Delete"><Icon icon="ph:trash" /></button>
                    {:else}
                      <button type="button" class="note-action-btn" on:click|stopPropagation={() => restoreNote(entry)} title="Restore" aria-label="Restore"><Icon icon="ph:arrow-counter-clockwise" /></button>
                      <button type="button" class="note-action-btn delete" on:click|stopPropagation={() => permanentlyDelete(entry.id)} title="Delete permanently" aria-label="Delete permanently"><Icon icon="ph:trash" /></button>
                    {/if}
                  </div>
                </div>
              </article>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<!-- Styles come from App.svelte (.kalam-sleek .page-content). Only Notes-specific overrides below. -->
<style>
  .btn-ghost.danger {
    color: #ff3b30;
  }
  .btn-ghost.danger:hover {
    background: rgba(255, 59, 48, 0.1);
  }
</style>
