<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import {
    getNotes,
    searchNotes,
    getNoteLabels,
    getNoteScopeCounts,
    updateEntry,
    deleteEntry,
    emptyTrash
  } from '../../lib/api/db'
  import type { Entry } from '../../types'
  import Icon from '@iconify/svelte'
  import { flip } from 'svelte/animate'
  import { tick } from 'svelte'
  import { selectedNoteId } from '../../lib/noteDetailStore'
  import { noteDetailReturnTo } from '../../lib/detailReturnStore'

  export let navigate: (page: string) => void = () => {}

  /** Pointer-driven reorder (HTML5 DnD is unreliable in Tauri / WebView2). */
  let dragNoteId: string | null = null
  let pointerReorderPending: {
    id: string
    x: number
    y: number
    offsetX: number
    offsetY: number
    startX: number
    startY: number
    floatW: number
    floatH: number
    /** Offset of the nearest ancestor that creates a containing block for position:fixed
        (any ancestor with a transform, e.g. .page.fade-in with animation: fadeInPage). */
    containerOffsetX: number
    containerOffsetY: number
  } | null = null
  let dragNoteOffset = { x: 0, y: 0 }
  let dragNotePos = { x: 0, y: 0 }
  /** Captured from the real card at pointer-down (grid centers the card; must not use wrapper rect). */
  let dragFloatSize = { w: 0, h: 0 }
  /** Compensate for ancestor transforms that shift the containing block for position:fixed. */
  let dragContainerOffset = { x: 0, y: 0 }
  let dropIndicator: { targetId: string | null; zone: 'pinned' | 'others' | null; position: 'before' | 'after' | null } = { targetId: null, zone: null, position: null }
  let ignoreNextNoteCardClick = false
  let noteReorderWindowListeners = false

  const NOTE_REORDER_DRAG_THRESHOLD_PX = 6

  $: notesReorderEnabled =
    notesScope === 'active' && !searchDebounced.trim() && selectedLabels.length === 0 && !loading

  function noteOrderKey(e: Entry): number {
    const n = e.note_order
    return typeof n === 'number' && Number.isFinite(n) ? n : 0
  }

  function attachNoteReorderWindowListeners() {
    if (noteReorderWindowListeners) return
    noteReorderWindowListeners = true
    window.addEventListener('pointermove', onNoteReorderPointerMove)
    window.addEventListener('pointerup', onNoteReorderPointerUp)
    window.addEventListener('pointercancel', onNoteReorderPointerUp)
  }

  function detachNoteReorderWindowListeners() {
    if (!noteReorderWindowListeners) return
    noteReorderWindowListeners = false
    window.removeEventListener('pointermove', onNoteReorderPointerMove)
    window.removeEventListener('pointerup', onNoteReorderPointerUp)
    window.removeEventListener('pointercancel', onNoteReorderPointerUp)
  }

  function noteReorderPointerDown(e: PointerEvent, id: string) {
    if (!notesReorderEnabled) return
    if (e.button !== 0) return
    // Walk from the event target up to the article to get the card's actual viewport rect.
    let el: HTMLElement | null = e.target as HTMLElement
    while (el && el.tagName !== 'ARTICLE') el = el.parentElement
    const rect = el?.getBoundingClientRect()
    if (!rect || !el) return

    // A non-`none` transform on an ancestor makes position:fixed relative to that element
    // (fadeInPage ends with `transform: none` so this offset is usually 0 after the intro animation).
    const page = el.closest('.page.fade-in') as HTMLElement | null
    let containerOffsetX = 0
    let containerOffsetY = 0
    if (page) {
      const xf = getComputedStyle(page).transform
      if (xf && xf !== 'none') {
        const pageRect = page.getBoundingClientRect()
        containerOffsetX = pageRect.left
        containerOffsetY = pageRect.top
      }
    }

    pointerReorderPending = {
      id,
      x: e.clientX,
      y: e.clientY,
      offsetX: e.clientX - rect.left,
      offsetY: e.clientY - rect.top,
      startX: rect.left,
      startY: rect.top,
      floatW: rect.width,
      floatH: rect.height,
      containerOffsetX,
      containerOffsetY
    }
    dragNoteId = null
    attachNoteReorderWindowListeners()
  }

  function onNoteReorderPointerMove(e: PointerEvent) {
    // Update floating position when already dragging
    if (dragNoteId) {
      dragNotePos = { x: e.clientX - dragNoteOffset.x, y: e.clientY - dragNoteOffset.y }
      // Update drop indicator for visual feedback
      const drop = resolveNoteDropAtPoint(e.clientX, e.clientY, dragNoteId)
      if (drop) {
        if (drop.kind === 'before') {
          dropIndicator = { targetId: drop.targetId, zone: drop.zone, position: 'before' }
        } else {
          dropIndicator = { targetId: null, zone: drop.zone, position: 'after' }
        }
      } else {
        dropIndicator = { targetId: null, zone: null, position: null }
      }
      return
    }
    if (!pointerReorderPending) return
    const dx = e.clientX - pointerReorderPending.x
    const dy = e.clientY - pointerReorderPending.y
    if (Math.hypot(dx, dy) >= NOTE_REORDER_DRAG_THRESHOLD_PX) {
      dragNoteId = pointerReorderPending.id
      dragNoteOffset = { x: pointerReorderPending.offsetX, y: pointerReorderPending.offsetY }
      dragFloatSize = { w: pointerReorderPending.floatW, h: pointerReorderPending.floatH }
      dragContainerOffset = { x: pointerReorderPending.containerOffsetX, y: pointerReorderPending.containerOffsetY }
      dragNotePos = { x: pointerReorderPending.startX, y: pointerReorderPending.startY }
      pointerReorderPending = null
    }
  }

  type NoteDropResolution =
    | { kind: 'before'; targetId: string; zone: 'pinned' | 'others' }
    | { kind: 'append'; zone: 'pinned' | 'others' }

  function resolveNoteDropAtPoint(clientX: number, clientY: number, dragId: string): NoteDropResolution | null {
    // Find and temporarily hide the dragged card from hit testing so elementFromPoint
    // can see the drop target underneath. CSS pointer-events may not be applied yet due
    // to Svelte's async reactivity, so we set it explicitly here.
    const draggedEl = document.querySelector(`article.note-card[data-note-id="${dragId}"]`) as HTMLElement | null
    const prevPointerEvents = draggedEl?.style.pointerEvents
    if (draggedEl) draggedEl.style.pointerEvents = 'none'

    const el = document.elementFromPoint(clientX, clientY)

    // Restore the dragged card's pointer-events
    if (draggedEl) {
      draggedEl.style.pointerEvents = prevPointerEvents || ''
    }

    if (!el) return null

    let node: Element | null = el
    while (node) {
      if (node instanceof HTMLElement) {
        const nid = node.dataset.noteId
        if (nid && nid !== dragId) {
          const zoneEl = node.closest('[data-notes-zone]')
          const z = zoneEl?.getAttribute('data-notes-zone')
          if (z === 'pinned' || z === 'others') {
            return { kind: 'before', targetId: nid, zone: z }
          }
        }
        const hint = node.dataset.notesDropHint
        if (hint === 'pin') return { kind: 'append', zone: 'pinned' }
        if (hint === 'unpin') return { kind: 'append', zone: 'others' }
      }
      node = node.parentElement
    }

    node = el
    while (node) {
      if (node instanceof HTMLElement) {
        const z = node.getAttribute('data-notes-zone')
        if (z === 'pinned' || z === 'others') {
          return { kind: 'append', zone: z }
        }
      }
      node = node.parentElement
    }
    return null
  }

  function applyNoteDropResolution(dragId: string, drop: NoteDropResolution) {
    const from = entries.find((x) => x.id === dragId)
    if (!from) return

    if (drop.kind === 'append') {
      let nextPinned = pinnedNotes.filter((x) => x.id !== dragId)
      let nextOthers = otherNotes.filter((x) => x.id !== dragId)
      const moving: Entry = { ...from, is_pinned: drop.zone === 'pinned' }
      if (drop.zone === 'pinned') nextPinned.push(moving)
      else nextOthers.push(moving)
      void finalizeNoteReorder(nextPinned, nextOthers)
      return
    }

    const { targetId, zone } = drop
    if (dragId === targetId) return
    const toPinned = zone === 'pinned'
    let nextPinned = pinnedNotes.filter((x) => x.id !== dragId)
    let nextOthers = otherNotes.filter((x) => x.id !== dragId)
    const moving: Entry = { ...from, is_pinned: toPinned }
    if (toPinned) {
      const idx = nextPinned.findIndex((x) => x.id === targetId)
      nextPinned.splice(idx >= 0 ? idx : nextPinned.length, 0, moving)
    } else {
      const idx = nextOthers.findIndex((x) => x.id === targetId)
      nextOthers.splice(idx >= 0 ? idx : nextOthers.length, 0, moving)
    }
    void finalizeNoteReorder(nextPinned, nextOthers)
  }

  function onNoteReorderPointerUp(e: PointerEvent) {
    if (pointerReorderPending && !dragNoteId) {
      pointerReorderPending = null
      detachNoteReorderWindowListeners()
      return
    }

    const hadDrag = dragNoteId !== null
    const id = dragNoteId
    pointerReorderPending = null
    dragNoteId = null
    dropIndicator = { targetId: null, zone: null, position: null }
    detachNoteReorderWindowListeners()

    if (hadDrag) ignoreNextNoteCardClick = true
    if (!id || !notesReorderEnabled) return

    const drop = resolveNoteDropAtPoint(e.clientX, e.clientY, id)
    if (!drop) return
    applyNoteDropResolution(id, drop)
  }

  function applyLocalNoteOrder(newPinned: Entry[], newOthers: Entry[]) {
    const map = new Map<string, { is_pinned: boolean; note_order: number }>()
    newPinned.forEach((row, i) => map.set(row.id, { is_pinned: true, note_order: i }))
    newOthers.forEach((row, i) => map.set(row.id, { is_pinned: false, note_order: i }))
    entries = entries.map((row) => {
      const u = map.get(row.id)
      return u ? { ...row, is_pinned: u.is_pinned, note_order: u.note_order } : row
    })
  }

  async function persistNoteOrders(newPinned: Entry[], newOthers: Entry[]) {
    const now = new Date().toISOString()
    for (let i = 0; i < newPinned.length; i++) {
      await updateEntry({ ...newPinned[i], is_pinned: true, note_order: i, updated_at: now })
    }
    for (let i = 0; i < newOthers.length; i++) {
      await updateEntry({ ...newOthers[i], is_pinned: false, note_order: i, updated_at: now })
    }
  }

  async function finalizeNoteReorder(newPinned: Entry[], newOthers: Entry[]) {
    sortMode = 'manual'
    applyLocalNoteOrder(newPinned, newOthers)
    dragNoteId = null
    try {
      await persistNoteOrders(newPinned, newOthers)
    } catch (err) {
      error = err instanceof Error ? err.message : String(err)
      await load()
    }
  }

  function handleNoteCardClick(entry: Entry) {
    if (ignoreNextNoteCardClick) {
      ignoreNextNoteCardClick = false
      return
    }
    openEditPanel(entry)
  }

  let entries: Entry[] = []
  let pinnedNotes: Entry[] = []
  let otherNotes: Entry[] = []
  let labels: string[] = []
  let loading = true
  let error: string | null = null
  let notesScope: 'active' | 'archived' | 'trash' = 'active'
  let searchQuery = ''
  let searchDebounced = ''
  /** Active scope only: OR filter — note must have at least one of these tags. */
  let selectedLabels: string[] = []
  let searchTimeout: ReturnType<typeof setTimeout>

  let scopePopoverOpen = false
  let scopePopoverEl: HTMLDivElement | null = null
  /** Totals per scope (from DB; not filtered by search). */
  let scopeCounts: { active: number; archived: number; trash: number } | null = null

  const NOTES_SCOPE_OPTIONS = [
    { id: 'active' as const, label: 'Notes', icon: 'ph:notebook', title: 'Active notes' },
    { id: 'archived' as const, label: 'Archive', icon: 'ph:archive', title: 'Archived notes' },
    { id: 'trash' as const, label: 'Trash', icon: 'ph:trash', title: 'Trashed notes' }
  ]
  $: notesScopeOption = NOTES_SCOPE_OPTIONS.find((o) => o.id === notesScope) ?? NOTES_SCOPE_OPTIONS[0]

  function scopeCountFor(id: (typeof NOTES_SCOPE_OPTIONS)[number]['id']): number {
    if (!scopeCounts) return 0
    const n = scopeCounts[id]
    return typeof n === 'number' && Number.isFinite(n) ? n : 0
  }

  async function loadScopeCounts() {
    try {
      scopeCounts = await getNoteScopeCounts()
    } catch {
      scopeCounts = null
    }
  }

  let labelFilterOpen = false
  let labelFilterEl: HTMLDivElement | null = null
  /** Narrows the label list inside the popover (not the main notes search). */
  let labelFilterSearch = ''
  let labelFilterSearchInput: HTMLInputElement | null = null

  $: labelsMatchingPopoverSearch = labels.filter((l) =>
    l.toLowerCase().includes(labelFilterSearch.trim().toLowerCase())
  )

  /* Use `click`, not `pointerdown`: pointerdown bubbles to document before the control's click
     fires — closing popovers here re-renders and can swallow sort / other toolbar clicks. */
  function closeToolbarPopoversIfOutside(e: MouseEvent) {
    const t = e.target
    if (!(t instanceof Node)) return
    if (labelFilterOpen) {
      const el = labelFilterEl
      if (!el || !el.contains(t)) labelFilterOpen = false
    }
    if (scopePopoverOpen) {
      const el = scopePopoverEl
      if (!el || !el.contains(t)) scopePopoverOpen = false
    }
  }

  function onGlobalKeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape') return
    if (labelFilterOpen) labelFilterOpen = false
    if (scopePopoverOpen) scopePopoverOpen = false
  }

  async function toggleScopePopover() {
    labelFilterOpen = false
    scopePopoverOpen = !scopePopoverOpen
    if (scopePopoverOpen) {
      await tick()
      void loadScopeCounts()
    }
  }

  function selectNotesScope(id: (typeof NOTES_SCOPE_OPTIONS)[number]['id']) {
    notesScope = id
    scopePopoverOpen = false
  }

  function toggleLabelFilterLabel(label: string) {
    if (selectedLabels.includes(label)) {
      selectedLabels = selectedLabels.filter((l) => l !== label)
    } else {
      selectedLabels = [...selectedLabels, label].sort((a, b) =>
        a.localeCompare(b, undefined, { sensitivity: 'base' })
      )
    }
  }

  function clearLabelFilters() {
    selectedLabels = []
  }

  async function toggleLabelFilterPopover() {
    scopePopoverOpen = false
    labelFilterOpen = !labelFilterOpen
    if (labelFilterOpen) {
      await tick()
      labelFilterSearchInput?.focus()
    }
  }

  $: if (notesScope !== 'active' && selectedLabels.length > 0) {
    selectedLabels = []
  }

  type NotesSortMode = 'updated_desc' | 'updated_asc' | 'title_asc' | 'title_desc' | 'manual'
  let sortMode: NotesSortMode = 'updated_desc'

  const SORT_CYCLE: NotesSortMode[] = ['updated_desc', 'updated_asc', 'title_asc', 'title_desc', 'manual']

  const SORT_LABELS: Record<NotesSortMode, string> = {
    updated_desc: 'Newest first',
    updated_asc: 'Oldest first',
    title_asc: 'Title A–Z',
    title_desc: 'Title Z–A',
    manual: 'Custom order'
  }

  /** Icon hints sort dimension: arrows = by date, sort glyphs = by title (Phosphor names used elsewhere in app). */
  const SORT_ICONS: Record<NotesSortMode, string> = {
    updated_desc: 'ph:arrow-down',
    updated_asc: 'ph:arrow-up',
    title_asc: 'ph:sort-ascending',
    title_desc: 'ph:sort-descending',
    manual: 'ph:dots-six-vertical'
  }

  function cycleSortMode() {
    scopePopoverOpen = false
    labelFilterOpen = false
    const i = SORT_CYCLE.indexOf(sortMode)
    sortMode = SORT_CYCLE[(i + 1) % SORT_CYCLE.length]
  }

  $: sortModeLabel = SORT_LABELS[sortMode]
  $: sortModeIcon = SORT_ICONS[sortMode]

  /** Search, sort, label filter — hidden until there is data, filters, or an error (not during first-load empty). */
  $: showNotesListChrome =
    !!error ||
    entries.length > 0 ||
    !!searchQuery.trim() ||
    selectedLabels.length > 0

  function compareNotes(a: Entry, b: Entry): number {
    if (sortMode === 'manual') {
      return noteOrderKey(a) - noteOrderKey(b)
    }
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
      const q = searchDebounced.trim()
      const sel = selectedLabels

      let list: Entry[]
      if (!q && sel.length === 0) {
        const result = await getNotes(notesScope)
        list = Array.isArray(result) ? result : []
      } else if (!q && sel.length === 1) {
        const result = await searchNotes({ label: sel[0], scope: notesScope })
        list = Array.isArray(result) ? result : []
      } else if (q && sel.length === 1) {
        const result = await searchNotes({ query: q, label: sel[0], scope: notesScope })
        list = Array.isArray(result) ? result : []
      } else if (q && sel.length === 0) {
        const result = await searchNotes({ query: q, scope: notesScope })
        list = Array.isArray(result) ? result : []
      } else {
        const result = await searchNotes({ query: q || undefined, scope: notesScope })
        list = Array.isArray(result) ? result : []
        list = list.filter((e) => sel.some((l) => (e.tags || []).includes(l)))
      }

      entries = list
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    } finally {
      loading = false
    }
    void loadScopeCounts()
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

  $: void (notesScope, searchDebounced, selectedLabels), load()
  $: void notesScope, loadLabels()
  $: void searchQuery, triggerSearch()

  function openAddPanel() {
    noteDetailReturnTo.set(null)
    selectedNoteId.set(null)
    navigate('note-detail')
  }

  function openEditPanel(entry: Entry) {
    noteDetailReturnTo.set(null)
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
    if (!confirm('Move this note to trash?')) return
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
    if (!confirm('Permanently delete this note?')) return
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

  /** One-line excerpt for list cards (plain text; strip naive HTML if present). */
  function noteContentPreview(raw: string | null | undefined): string {
    if (raw == null || !String(raw).trim()) return ''
    let t = String(raw)
      .replace(/<[^>]*>/g, '')
      .replace(/\s+/g, ' ')
      .trim()
    if (!t) return ''
    const max = 180
    if (t.length <= max) return t
    const cut = t.slice(0, max)
    const lastSpace = cut.lastIndexOf(' ')
    return (lastSpace > 40 ? cut.slice(0, lastSpace) : cut) + '…'
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

  /* `sortMode` must be read in this block: Svelte does not track it inside `.sort(compareNotes)` alone. */
  $: {
    void sortMode
    pinnedNotes = entries
      .filter((e) => e.is_pinned)
      .slice()
      .sort(compareNotes)
    otherNotes = entries
      .filter((e) => !e.is_pinned)
      .slice()
      .sort(compareNotes)
  }

  onMount(() => {
    document.addEventListener('click', closeToolbarPopoversIfOutside)
    document.addEventListener('keydown', onGlobalKeydown)
  })

  onDestroy(() => {
    document.removeEventListener('click', closeToolbarPopoversIfOutside)
    document.removeEventListener('keydown', onGlobalKeydown)
    detachNoteReorderWindowListeners()
  })
</script>

<div class="page fade-in">
  <header class="page-header notes-header">
    <div>
      <h1 class="page-title">Notes</h1>
      <p class="page-subtitle">Jot down your thoughts, ideas, and transcriptions.</p>
    </div>
    {#if notesScope === 'trash'}
      {#if !loading && entries.length > 0}
        <button type="button" class="btn-danger-outline" on:click={emptyTrashConfirm}>
          <Icon icon="ph:trash" />
          Empty trash
        </button>
      {/if}
    {:else}
      <button type="button" class="btn-primary" on:click={openAddPanel}>
        <Icon icon="ph:plus" />
        New Note
      </button>
    {/if}
  </header>

  <div class="notes-toolbar">
    {#if showNotesListChrome}
      <div class="notes-search-bar">
        <span class="notes-search-bar-icon" aria-hidden="true">
          <Icon icon="ph:magnifying-glass" />
        </span>
        <input type="text" bind:value={searchQuery} placeholder="Search notes..." />
      </div>
    {/if}
    <div class="notes-toolbar-actions">
      <!-- Scope + popover live outside `.notes-toolbar-actions-scroll`: that strip uses
           overflow-x: auto, which forces overflow-y to clip and hides position:absolute popovers. -->
      <div class="notes-toolbar-scope-dropdown" bind:this={scopePopoverEl}>
        <button
          type="button"
          class="notes-sort-cycle notes-toolbar-scope-trigger"
          title={notesScopeOption.title}
          aria-label="Notes location: {notesScopeOption.label}. Open to change."
          aria-expanded={scopePopoverOpen}
          aria-haspopup="dialog"
          on:click|stopPropagation={toggleScopePopover}
        >
          <span aria-hidden="true"><Icon icon={notesScopeOption.icon} /></span>
          <span class="notes-toolbar-scope-trigger-label">{notesScopeOption.label}</span>
        </button>
        {#if scopePopoverOpen}
          <div
            class="notes-scope-menu notes-label-filter-popover notes-toolbar-scope-popover"
            role="dialog"
            aria-label="Which notes to show"
            on:click|stopPropagation
          >
            <div class="notes-label-filter-popover-head">
              <div class="notes-label-filter-popover-meta">
                <span class="notes-label-filter-popover-heading">Show</span>
              </div>
            </div>
            <div class="notes-label-filter-popover-body">
              <div
                class="notes-label-filter-menu notes-label-filter-menu-scroll notes-toolbar-scope-menu-scroll"
              >
                {#each NOTES_SCOPE_OPTIONS as opt (opt.id)}
                  <button
                    type="button"
                    class="notes-scope-option"
                    class:active={notesScope === opt.id}
                    aria-label={`${opt.label}, ${scopeCountFor(opt.id)} ${scopeCountFor(opt.id) === 1 ? 'note' : 'notes'}`}
                    on:click={() => selectNotesScope(opt.id)}
                  >
                    <span class="notes-toolbar-scope-option-icon" aria-hidden="true">
                      <Icon icon={opt.icon} />
                    </span>
                    <span class="notes-scope-option-label">{opt.label}</span>
                    <span class="notes-scope-option-count" aria-hidden="true">{scopeCountFor(
                      opt.id
                    ).toLocaleString()}</span>
                  </button>
                {/each}
              </div>
            </div>
          </div>
        {/if}
      </div>
      {#if showNotesListChrome}
        <div class="notes-toolbar-actions-scroll">
          <button
            type="button"
            class="notes-sort-cycle"
            title={sortModeLabel}
            aria-label="Sort: {sortModeLabel}. Click to change sort order."
            on:click|stopPropagation={cycleSortMode}
          >
            <span aria-hidden="true"><Icon icon={sortModeIcon} /></span>
          </button>
        </div>
      {/if}
        {#if showNotesListChrome && notesScope === 'active' && labels.length > 0}
          <div class="notes-label-filter-dropdown" bind:this={labelFilterEl}>
            <button
              type="button"
              class="notes-sort-cycle notes-label-filter-toggle"
              class:has-filter={selectedLabels.length > 0}
              title={selectedLabels.length > 0
                ? `${selectedLabels.length} label filter${selectedLabels.length === 1 ? '' : 's'} active — click to change`
                : 'Filter notes by label'}
              aria-label={selectedLabels.length > 0
                ? `Label filters: ${selectedLabels.length} active. Open to search or change.`
                : 'Open label filter'}
              aria-expanded={labelFilterOpen}
              aria-haspopup="dialog"
              on:click|stopPropagation={toggleLabelFilterPopover}
            >
              <Icon icon="ph:tag" />
              {#if selectedLabels.length > 0}
                <span class="notes-label-filter-count">{selectedLabels.length}</span>
              {/if}
            </button>
            {#if labelFilterOpen}
              <div
                class="notes-scope-menu notes-label-filter-popover"
                role="dialog"
                aria-label="Filter by labels"
                on:click|stopPropagation
              >
                <div class="notes-label-filter-popover-head">
                  <div class="notes-label-filter-popover-meta">
                    <span class="notes-label-filter-popover-heading">Labels</span>
                    <button type="button" class="notes-label-filter-clear-all" on:click={clearLabelFilters}>
                      Clear all
                    </button>
                  </div>
                  <div class="notes-label-filter-search">
                    <span class="notes-label-filter-search-icon" aria-hidden="true">
                      <Icon icon="ph:magnifying-glass" />
                    </span>
                    <input
                      bind:this={labelFilterSearchInput}
                      type="search"
                      placeholder="Search labels…"
                      bind:value={labelFilterSearch}
                      aria-label="Search labels"
                    />
                  </div>
                </div>
                <div class="notes-label-filter-popover-body">
                  <div class="notes-label-filter-menu notes-label-filter-menu-scroll">
                    {#if labelsMatchingPopoverSearch.length === 0}
                      <div class="notes-label-filter-no-match">No labels match</div>
                    {:else}
                      {#each labelsMatchingPopoverSearch as label}
                        <label class="notes-label-filter-row">
                          <input
                            type="checkbox"
                            checked={selectedLabels.includes(label)}
                            on:change={() => toggleLabelFilterLabel(label)}
                          />
                          <span>{label}</span>
                        </label>
                      {/each}
                    {/if}
                  </div>
                </div>
              </div>
            {/if}
          </div>
        {/if}
    </div>
  </div>

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
      <p>{notesScope === 'trash' ? 'Trash is empty' : notesScope === 'archived' ? 'No archived notes' : (searchQuery || selectedLabels.length > 0) ? 'No results' : 'No notes yet'}</p>
    </div>
  {:else}
    <div class="notes-lists-container">
      {#if pinnedNotes.length > 0 || (notesReorderEnabled && dragNoteId && otherNotes.length > 0)}
        <!-- Pinned chrome only when there are pins; while dragging, an empty pin zone is rendered without the section heading. -->
        <div class:notes-section={pinnedNotes.length > 0}>
          {#if pinnedNotes.length > 0}
            <h3 class="notes-section-title">Pinned</h3>
          {/if}
          <div
            class="notes-masonry"
            class:notes-masonry--reorder={notesReorderEnabled && dragNoteId}
            class:notes-masonry--empty-pinned={pinnedNotes.length === 0 && notesReorderEnabled && otherNotes.length > 0 && dragNoteId}
            data-notes-zone="pinned"
          >
            {#each pinnedNotes as entry, index (entry.id)}
              {@const previewText = noteContentPreview(entry.content)}
              <div class="note-card-wrapper" animate:flip={{ duration: dragNoteId === entry.id ? 0 : 250 }}>
                <!-- Drop spacer before this card -->
                {#if dragNoteId && dropIndicator.targetId === entry.id && dropIndicator.zone === 'pinned' && dropIndicator.position === 'before'}
                  <div class="note-drop-spacer" data-drop-spacer="true"></div>
                {/if}
                <article
                  class="note-card"
                  class:note-card--reorder={notesReorderEnabled}
                  class:pinned={entry.is_pinned}
                  class:dragging={dragNoteId === entry.id}
                  class:has-custom-color={!!entry.color}
                  style:background-color={entry.color || 'var(--bg-elevated)'}
                  style:position={dragNoteId === entry.id ? 'fixed' : undefined}
                  style:left={dragNoteId === entry.id ? `${dragNotePos.x - dragContainerOffset.x}px` : undefined}
                  style:top={dragNoteId === entry.id ? `${dragNotePos.y - dragContainerOffset.y}px` : undefined}
                  style:width={dragNoteId === entry.id && dragFloatSize.w > 0 ? `${dragFloatSize.w}px` : undefined}
                  style:height={dragNoteId === entry.id && dragFloatSize.h > 0 ? `${dragFloatSize.h}px` : undefined}
                  style:z-index={dragNoteId === entry.id ? '1000' : undefined}
                  style:margin={dragNoteId === entry.id ? '0' : undefined}
                  data-note-id={entry.id}
                  role="button"
                  tabindex="0"
                  aria-grabbed={notesReorderEnabled && dragNoteId === entry.id ? true : undefined}
                  title={notesReorderEnabled
                    ? 'Drag from the note body to reorder or move to Pinned / Others. Click to open.'
                    : undefined}
                  on:click={() => handleNoteCardClick(entry)}
                  on:keydown={(e) => e.key === 'Enter' && openEditPanel(entry)}
                >
                  <div
                    class="note-inner"
                    on:pointerdown|stopPropagation={(e) => noteReorderPointerDown(e, entry.id)}
                  >
                  {#if entry.is_pinned}
                    <div class="pin-icon"><Icon icon="ph:push-pin-fill" /></div>
                  {/if}
                  <h4 class="note-title" class:note-title-placeholder={!entry.title?.trim()}>
                    {entry.title?.trim() ? entry.title : 'Untitled'}
                  </h4>
                  {#if previewText}
                    <p class="note-preview">{previewText}</p>
                  {/if}
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
                <div class="note-footer" role="group" draggable="false" on:click|stopPropagation>
                  <span class="note-date" draggable="false">{formatDate(entry.updated_at)}</span>
                  <div class="note-actions">
                    {#if notesScope === 'active'}
                      <button type="button" class="note-action-btn" draggable="false" on:click|stopPropagation={() => archiveNote(entry)} title="Archive" aria-label="Archive"><Icon icon="ph:archive" /></button>
                      <button type="button" class="note-action-btn delete" draggable="false" on:click|stopPropagation={() => moveToTrash(entry)} title="Delete" aria-label="Delete"><Icon icon="ph:trash" /></button>
                    {:else if notesScope === 'archived'}
                      <button type="button" class="note-action-btn note-action-btn--labeled" draggable="false" on:click|stopPropagation={() => unarchiveNote(entry)} title="Return to Notes" aria-label="Unarchive"><Icon icon="ph:archive-tray" /><span>Unarchive</span></button>
                      <button type="button" class="note-action-btn note-action-btn--labeled delete" draggable="false" on:click|stopPropagation={() => moveToTrash(entry)} title="Move to trash" aria-label="Move to trash"><Icon icon="ph:trash" /><span>Trash</span></button>
                    {:else}
                      <button type="button" class="note-action-btn note-action-btn--labeled" draggable="false" on:click|stopPropagation={() => restoreNote(entry)} title="Return to Notes" aria-label="Restore from trash"><Icon icon="ph:arrow-counter-clockwise" /><span>Restore</span></button>
                      <button type="button" class="note-action-btn note-action-btn--labeled delete" draggable="false" on:click|stopPropagation={() => permanentlyDelete(entry.id)} title="Delete permanently" aria-label="Delete permanently"><Icon icon="ph:trash" /><span>Delete</span></button>
                    {/if}
                  </div>
                </div>
              </article>
            </div>
            {/each}
            <!-- Drop spacer at end of pinned list -->
            {#if dragNoteId && dropIndicator.zone === 'pinned' && dropIndicator.position === 'after'}
              <div class="note-drop-spacer" data-drop-spacer="true"></div>
            {/if}
            {#if pinnedNotes.length === 0 && notesReorderEnabled && otherNotes.length > 0 && dragNoteId}
              <div class="notes-pin-drop-hint" data-notes-drop-hint="pin">Drop here to pin</div>
            {/if}
          </div>
        </div>
      {/if}
      {#if otherNotes.length > 0 || (notesReorderEnabled && pinnedNotes.length > 0)}
        <div class="notes-section">
          {#if pinnedNotes.length > 0}
            <h3 class="notes-section-title">Others</h3>
          {/if}
          <div
            class="notes-masonry"
            class:notes-masonry--reorder={notesReorderEnabled && dragNoteId}
            class:notes-masonry--empty-others={otherNotes.length === 0 && notesReorderEnabled && pinnedNotes.length > 0}
            data-notes-zone="others"
          >
            {#each otherNotes as entry, index (entry.id)}
              {@const previewText = noteContentPreview(entry.content)}
              <div class="note-card-wrapper" animate:flip={{ duration: dragNoteId === entry.id ? 0 : 250 }}>
                <!-- Drop spacer before this card -->
                {#if dragNoteId && dropIndicator.targetId === entry.id && dropIndicator.zone === 'others' && dropIndicator.position === 'before'}
                  <div class="note-drop-spacer" data-drop-spacer="true"></div>
                {/if}
                <article
                  class="note-card"
                  class:note-card--reorder={notesReorderEnabled}
                  class:pinned={entry.is_pinned}
                  class:dragging={dragNoteId === entry.id}
                  class:has-custom-color={!!entry.color}
                  style:background-color={entry.color || 'var(--bg-elevated)'}
                  style:position={dragNoteId === entry.id ? 'fixed' : undefined}
                  style:left={dragNoteId === entry.id ? `${dragNotePos.x - dragContainerOffset.x}px` : undefined}
                  style:top={dragNoteId === entry.id ? `${dragNotePos.y - dragContainerOffset.y}px` : undefined}
                  style:width={dragNoteId === entry.id && dragFloatSize.w > 0 ? `${dragFloatSize.w}px` : undefined}
                  style:height={dragNoteId === entry.id && dragFloatSize.h > 0 ? `${dragFloatSize.h}px` : undefined}
                  style:z-index={dragNoteId === entry.id ? '1000' : undefined}
                  style:margin={dragNoteId === entry.id ? '0' : undefined}
                  data-note-id={entry.id}
                  role="button"
                  tabindex="0"
                  aria-grabbed={notesReorderEnabled && dragNoteId === entry.id ? true : undefined}
                  title={notesReorderEnabled
                    ? 'Drag from the note body to reorder or move to Pinned / Others. Click to open.'
                    : undefined}
                  on:click={() => handleNoteCardClick(entry)}
                  on:keydown={(e) => e.key === 'Enter' && openEditPanel(entry)}
                >
                  <div
                    class="note-inner"
                    on:pointerdown|stopPropagation={(e) => noteReorderPointerDown(e, entry.id)}
                  >
                  {#if entry.is_pinned}
                    <div class="pin-icon"><Icon icon="ph:push-pin-fill" /></div>
                  {/if}
                  <h4 class="note-title" class:note-title-placeholder={!entry.title?.trim()}>
                    {entry.title?.trim() ? entry.title : 'Untitled'}
                  </h4>
                  {#if previewText}
                    <p class="note-preview">{previewText}</p>
                  {/if}
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
                <div class="note-footer" role="group" draggable="false" on:click|stopPropagation>
                  <span class="note-date" draggable="false">{formatDate(entry.updated_at)}</span>
                  <div class="note-actions">
                    {#if notesScope === 'active'}
                      <button type="button" class="note-action-btn" draggable="false" on:click|stopPropagation={() => archiveNote(entry)} title="Archive" aria-label="Archive"><Icon icon="ph:archive" /></button>
                      <button type="button" class="note-action-btn delete" draggable="false" on:click|stopPropagation={() => moveToTrash(entry)} title="Delete" aria-label="Delete"><Icon icon="ph:trash" /></button>
                    {:else if notesScope === 'archived'}
                      <button type="button" class="note-action-btn note-action-btn--labeled" draggable="false" on:click|stopPropagation={() => unarchiveNote(entry)} title="Return to Notes" aria-label="Unarchive"><Icon icon="ph:archive-tray" /><span>Unarchive</span></button>
                      <button type="button" class="note-action-btn note-action-btn--labeled delete" draggable="false" on:click|stopPropagation={() => moveToTrash(entry)} title="Move to trash" aria-label="Move to trash"><Icon icon="ph:trash" /><span>Trash</span></button>
                    {:else}
                      <button type="button" class="note-action-btn note-action-btn--labeled" draggable="false" on:click|stopPropagation={() => restoreNote(entry)} title="Return to Notes" aria-label="Restore from trash"><Icon icon="ph:arrow-counter-clockwise" /><span>Restore</span></button>
                      <button type="button" class="note-action-btn note-action-btn--labeled delete" draggable="false" on:click|stopPropagation={() => permanentlyDelete(entry.id)} title="Delete permanently" aria-label="Delete permanently"><Icon icon="ph:trash" /><span>Delete</span></button>
                    {/if}
                  </div>
                </div>
              </article>
            </div>
            {/each}
            <!-- Drop spacer at end of others list -->
            {#if dragNoteId && dropIndicator.zone === 'others' && dropIndicator.position === 'after'}
              <div class="note-drop-spacer" data-drop-spacer="true"></div>
            {/if}
            {#if otherNotes.length === 0 && notesReorderEnabled && pinnedNotes.length > 0}
              <div class="notes-unpin-drop-hint" data-notes-drop-hint="unpin">Drop here to unpin</div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<!-- Styles come from App.svelte (.kalam-sleek .page-content). -->
