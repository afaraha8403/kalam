<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte'
  import { flip } from 'svelte/animate'
  import { getEntriesByType, updateEntry, deleteEntry, emptyTaskTrash } from '../../lib/api/db'
  import type { Entry } from '../../types'
  import Icon from '@iconify/svelte'
  import { selectedTaskId } from '../../lib/taskDetailStore'
  import { taskDetailReturnTo } from '../../lib/detailReturnStore'

  export let navigate: (page: string) => void = () => {}

  let entries: Entry[] = []
  let tasksDisplayOrder: Entry[] = []
  let loading = true
  let error: string | null = null
  let searchQuery = ''
  type TasksScope = 'open' | 'completed' | 'all' | 'trash'
  /** Default: all active tasks, shown as Open + Closed groups. */
  let tasksScope: TasksScope = 'all'

  let scopePopoverOpen = false
  let scopePopoverEl: HTMLDivElement | null = null

  const TASKS_SCOPE_OPTIONS = [
    { id: 'all' as const, label: 'All', icon: 'ph:squares-four', title: 'All tasks' },
    { id: 'open' as const, label: 'Open', icon: 'ph:circle', title: 'Open tasks' },
    { id: 'completed' as const, label: 'Completed', icon: 'ph:check-circle', title: 'Completed tasks' },
    { id: 'trash' as const, label: 'Trash', icon: 'ph:trash', title: 'Trashed tasks' }
  ]
  $: tasksScopeOption = TASKS_SCOPE_OPTIONS.find((o) => o.id === tasksScope) ?? TASKS_SCOPE_OPTIONS[0]

  /** OR filter: task must have at least one of these tags. */
  let selectedTags: string[] = []

  type TasksSortMode = 'updated_desc' | 'updated_asc' | 'title_asc' | 'title_desc' | 'manual'
  let sortMode: TasksSortMode = 'updated_desc'

  const SORT_CYCLE: TasksSortMode[] = ['updated_desc', 'updated_asc', 'title_asc', 'title_desc', 'manual']

  const SORT_LABELS: Record<TasksSortMode, string> = {
    updated_desc: 'Newest first',
    updated_asc: 'Oldest first',
    title_asc: 'Title A–Z',
    title_desc: 'Title Z–A',
    manual: 'Custom order'
  }

  const SORT_ICONS: Record<TasksSortMode, string> = {
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

  /** Tags from last non-trash load — used for the label filter menu while browsing trash. */
  let lastActiveTaskTags: string[] = []
  $: if (tasksScope !== 'trash') {
    lastActiveTaskTags = [...new Set(entries.flatMap((e) => e.tags || []))].sort((a, b) =>
      a.localeCompare(b, undefined, { sensitivity: 'base' })
    )
  }

  $: if (tasksScope === 'trash') selectedTags = []

  /** Search, sort, label filter — same visibility idea as Notes. */
  $: showTasksListChrome =
    !!error || entries.length > 0 || !!searchQuery.trim() || selectedTags.length > 0

  let labelFilterOpen = false
  let labelFilterEl: HTMLDivElement | null = null
  let labelFilterSearch = ''
  let labelFilterSearchInput: HTMLInputElement | null = null

  $: labelsMatchingPopoverSearch = lastActiveTaskTags.filter((l) =>
    l.toLowerCase().includes(labelFilterSearch.trim().toLowerCase())
  )

  /** Drop tag filters when no tags exist in the catalog (e.g. after scope change). */
  $: if (lastActiveTaskTags.length === 0 && selectedTags.length > 0) selectedTags = []

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

  async function toggleTasksScopePopover() {
    labelFilterOpen = false
    scopePopoverOpen = !scopePopoverOpen
    if (scopePopoverOpen) await tick()
  }

  function selectTasksScope(id: TasksScope) {
    tasksScope = id
    scopePopoverOpen = false
  }

  function toggleTaskTagFilterLabel(tag: string) {
    if (selectedTags.includes(tag)) {
      selectedTags = selectedTags.filter((t) => t !== tag)
    } else {
      selectedTags = [...selectedTags, tag].sort((a, b) =>
        a.localeCompare(b, undefined, { sensitivity: 'base' })
      )
    }
  }

  function clearTaskTagFilters() {
    selectedTags = []
  }

  async function toggleTaskTagFilterPopover() {
    scopePopoverOpen = false
    labelFilterOpen = !labelFilterOpen
    if (labelFilterOpen) {
      await tick()
      labelFilterSearchInput?.focus()
    }
  }

  /** Pointer-driven reorder (HTML5 DnD is unreliable in Tauri / WebView2). Whole row drags; checkbox/actions stay click-only. */
  let dragTaskId: string | null = null
  let dragTaskZone: string | null = null
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
    containerOffsetX: number
    containerOffsetY: number
    zone: string
  } | null = null
  let dragTaskOffset = { x: 0, y: 0 }
  let dragTaskPos = { x: 0, y: 0 }
  let dragFloatSize = { w: 0, h: 0 }
  let dragContainerOffset = { x: 0, y: 0 }
  let dropIndicator: {
    targetId: string | null
    zone: string | null
    position: 'before' | 'after' | null
  } = { targetId: null, zone: null, position: null }
  let ignoreNextTaskRowClick = false
  let taskReorderWindowListeners = false

  const TASK_REORDER_DRAG_THRESHOLD_PX = 6

  $: tasksReorderEnabled =
    tasksScope !== 'trash' && !searchQuery.trim() && selectedTags.length === 0 && !loading

  function taskOrderKey(e: Entry): number {
    const n = e.note_order
    return typeof n === 'number' && Number.isFinite(n) ? n : 0
  }

  function attachTaskReorderWindowListeners() {
    if (taskReorderWindowListeners) return
    taskReorderWindowListeners = true
    window.addEventListener('pointermove', onTaskReorderPointerMove)
    window.addEventListener('pointerup', onTaskReorderPointerUp)
    window.addEventListener('pointercancel', onTaskReorderPointerUp)
  }

  function detachTaskReorderWindowListeners() {
    if (!taskReorderWindowListeners) return
    taskReorderWindowListeners = false
    window.removeEventListener('pointermove', onTaskReorderPointerMove)
    window.removeEventListener('pointerup', onTaskReorderPointerUp)
    window.removeEventListener('pointercancel', onTaskReorderPointerUp)
  }

  function taskReorderPointerDown(e: PointerEvent, task: Entry) {
    if (!tasksReorderEnabled) return
    if (e.button !== 0) return
    const t = e.target as HTMLElement | null
    if (t?.closest('.task-row-actions, .checkbox')) return
    const row = e.currentTarget as HTMLElement
    const rect = row.getBoundingClientRect()
    const zoneEl = row.closest('[data-tasks-zone]')
    const zone = zoneEl?.getAttribute('data-tasks-zone') ?? 'flat'
    const page = row.closest('.page.fade-in') as HTMLElement | null
    const pageRect = page?.getBoundingClientRect()
    const containerOffsetX = pageRect ? pageRect.left : 0
    const containerOffsetY = pageRect ? pageRect.top : 0
    pointerReorderPending = {
      id: task.id,
      x: e.clientX,
      y: e.clientY,
      offsetX: e.clientX - rect.left,
      offsetY: e.clientY - rect.top,
      startX: rect.left,
      startY: rect.top,
      floatW: rect.width,
      floatH: rect.height,
      containerOffsetX,
      containerOffsetY,
      zone
    }
    dragTaskId = null
    attachTaskReorderWindowListeners()
  }

  function onTaskReorderPointerMove(e: PointerEvent) {
    if (dragTaskId) {
      dragTaskPos = { x: e.clientX - dragTaskOffset.x, y: e.clientY - dragTaskOffset.y }
      if (dragTaskZone) {
        const drop = resolveTaskDropAtPoint(e.clientX, e.clientY, dragTaskId, dragTaskZone)
        if (drop) {
          if (drop.kind === 'before') {
            dropIndicator = { targetId: drop.targetId, zone: drop.zone, position: 'before' }
          } else {
            dropIndicator = { targetId: null, zone: drop.zone, position: 'after' }
          }
        } else {
          dropIndicator = { targetId: null, zone: null, position: null }
        }
      }
      return
    }
    if (!pointerReorderPending) return
    const dx = e.clientX - pointerReorderPending.x
    const dy = e.clientY - pointerReorderPending.y
    if (Math.hypot(dx, dy) >= TASK_REORDER_DRAG_THRESHOLD_PX) {
      dragTaskId = pointerReorderPending.id
      dragTaskZone = pointerReorderPending.zone
      dragTaskOffset = { x: pointerReorderPending.offsetX, y: pointerReorderPending.offsetY }
      dragFloatSize = { w: pointerReorderPending.floatW, h: pointerReorderPending.floatH }
      dragContainerOffset = { x: pointerReorderPending.containerOffsetX, y: pointerReorderPending.containerOffsetY }
      dragTaskPos = { x: pointerReorderPending.startX, y: pointerReorderPending.startY }
      pointerReorderPending = null
    }
  }

  type TaskDropResolution =
    | { kind: 'before'; targetId: string; zone: string }
    | { kind: 'append'; zone: string }

  function resolveTaskDropAtPoint(
    clientX: number,
    clientY: number,
    dragId: string,
    dragZone: string
  ): TaskDropResolution | null {
    // Hide the dragged row from hit-testing so we see the target underneath (Notes use the same pattern;
    // clearing drag state before this can restore pointer-events before layout settles).
    const draggedEl = document.querySelector(
      `.task-row[data-task-id="${dragId}"]`
    ) as HTMLElement | null
    const prevPointerEvents = draggedEl?.style.pointerEvents
    if (draggedEl) draggedEl.style.pointerEvents = 'none'

    const el = document.elementFromPoint(clientX, clientY)

    if (draggedEl) {
      draggedEl.style.pointerEvents = prevPointerEvents || ''
    }

    if (!el) return null

    let node: Element | null = el
    while (node) {
      if (node instanceof HTMLElement) {
        const tid = node.dataset.taskId
        if (tid && tid !== dragId) {
          const zoneEl = node.closest('[data-tasks-zone]')
          const z = zoneEl?.getAttribute('data-tasks-zone')
          if (z === dragZone) {
            return { kind: 'before', targetId: tid, zone: z }
          }
        }
      }
      node = node.parentElement
    }

    node = el
    while (node) {
      if (node instanceof HTMLElement) {
        const z = node.getAttribute('data-tasks-zone')
        if (z === dragZone) {
          return { kind: 'append', zone: z }
        }
      }
      node = node.parentElement
    }
    return null
  }

  function applyTaskDropResolution(dragId: string, drop: TaskDropResolution) {
    const from = entries.find((x) => x.id === dragId)
    if (!from) return

    const section = taskListSections.find((s) => s.key === drop.zone)
    if (!section) return

    let list = section.tasks.filter((t) => t.id !== dragId)
    const moving = from

    if (drop.kind === 'append') {
      list = [...list, moving]
    } else {
      const idx = list.findIndex((t) => t.id === drop.targetId)
      list.splice(idx >= 0 ? idx : list.length, 0, moving)
    }

    void finalizeTaskReorder(drop.zone, list)
  }

  function rebuildFullOrderAfterSectionReorder(zone: string, newSectionTasks: Entry[]): Entry[] {
    if (tasksScope === 'all') {
      if (zone === 'open') {
        const closed = tasksDisplayOrder.filter((t) => taskIsCompleted(t))
        return [...newSectionTasks, ...closed]
      }
      if (zone === 'closed') {
        const open = tasksDisplayOrder.filter((t) => !taskIsCompleted(t))
        return [...open, ...newSectionTasks]
      }
    }
    return newSectionTasks
  }

  function applyLocalTaskOrderFromOrder(order: Entry[]) {
    const map = new Map(order.map((row, i) => [row.id, i]))
    entries = entries.map((row) => {
      const i = map.get(row.id)
      return i !== undefined ? { ...row, note_order: i } : row
    })
  }

  async function persistTaskOrder(order: Entry[]) {
    const now = new Date().toISOString()
    for (let i = 0; i < order.length; i++) {
      const ok = await updateEntry({ ...order[i], note_order: i, updated_at: now })
      if (!ok) throw new Error('Failed to save task order')
    }
  }

  async function finalizeTaskReorder(zone: string, sectionTasks: Entry[]) {
    sortMode = 'manual'
    const fullOrder = rebuildFullOrderAfterSectionReorder(zone, sectionTasks)
    applyLocalTaskOrderFromOrder(fullOrder)
    try {
      await persistTaskOrder(fullOrder)
    } catch (err) {
      error = err instanceof Error ? err.message : String(err)
      await load()
    }
  }

  function onTaskReorderPointerUp(e: PointerEvent) {
    if (pointerReorderPending && !dragTaskId) {
      pointerReorderPending = null
      detachTaskReorderWindowListeners()
      return
    }

    const hadDrag = dragTaskId !== null
    const id = dragTaskId
    const z = dragTaskZone
    const reorderOk = tasksReorderEnabled

    let drop: TaskDropResolution | null = null
    if (hadDrag && id && z && reorderOk) {
      drop = resolveTaskDropAtPoint(e.clientX, e.clientY, id, z)
    }

    pointerReorderPending = null
    dragTaskId = null
    dragTaskZone = null
    dropIndicator = { targetId: null, zone: null, position: null }
    detachTaskReorderWindowListeners()

    if (hadDrag) ignoreNextTaskRowClick = true

    if (!drop || !id) return
    applyTaskDropResolution(id, drop)
  }

  function handleTaskRowClick(task: Entry) {
    if (ignoreNextTaskRowClick) {
      ignoreNextTaskRowClick = false
      return
    }
    openTask(task)
  }

  function taskIsCompleted(e: Entry): boolean {
    return e.is_completed === true
  }

  $: loadScope = (tasksScope === 'trash' ? 'trash' : 'active') as 'active' | 'trash'

  $: loadScope, void load()

  async function load() {
    loading = true
    error = null
    try {
      const result = await getEntriesByType('task', 500, 0, loadScope)
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
    taskDetailReturnTo.set(null)
    selectedTaskId.set(null)
    navigate('task-detail')
  }

  function openTask(task: Entry) {
    taskDetailReturnTo.set(null)
    selectedTaskId.set(task.id)
    navigate('task-detail')
  }

  async function toggleComplete(entry: Entry) {
    try {
      const updated = { ...entry, is_completed: !entry.is_completed, updated_at: new Date().toISOString() }
      entries = entries.map((e) => (e.id === entry.id ? updated : e))
      await updateEntry(updated)
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  async function moveTaskToTrash(entry: Entry) {
    if (!confirm('Move this task to trash?')) return
    try {
      const updated = {
        ...entry,
        deleted_at: new Date().toISOString(),
        updated_at: new Date().toISOString()
      }
      entries = entries.filter((e) => e.id !== entry.id)
      await updateEntry(updated)
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  async function restoreTaskFromTrash(entry: Entry) {
    try {
      const updated = {
        ...entry,
        deleted_at: null,
        updated_at: new Date().toISOString()
      }
      entries = entries.filter((e) => e.id !== entry.id)
      await updateEntry(updated)
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  async function permanentlyDeleteTask(id: string) {
    if (!confirm('Permanently delete this task?')) return
    try {
      entries = entries.filter((e) => e.id !== id)
      await deleteEntry(id)
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  async function emptyTrashConfirm() {
    if (!confirm('Empty task trash? All tasks in trash will be permanently deleted.')) return
    try {
      await emptyTaskTrash()
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    }
  }

  function compareTasks(a: Entry, b: Entry): number {
    if (sortMode === 'manual') {
      return taskOrderKey(a) - taskOrderKey(b)
    }
    if (sortMode === 'title_asc' || sortMode === 'title_desc') {
      const ta = (a.title || a.content || '').trim().toLowerCase()
      const tb = (b.title || b.content || '').trim().toLowerCase()
      const c = ta.localeCompare(tb, undefined, { sensitivity: 'base' })
      return sortMode === 'title_asc' ? c : -c
    }
    const da = new Date(a.updated_at).getTime()
    const db = new Date(b.updated_at).getTime()
    return sortMode === 'updated_asc' ? da - db : db - da
  }

  $: scopeFiltered = entries.filter((e) => {
    if (tasksScope === 'trash') return true
    if (tasksScope === 'open') return !taskIsCompleted(e)
    if (tasksScope === 'completed') return taskIsCompleted(e)
    return true
  })

  $: tagFiltered =
    selectedTags.length > 0
      ? scopeFiltered.filter((e) => selectedTags.some((t) => (e.tags || []).includes(t)))
      : scopeFiltered

  $: filteredEntries = tagFiltered.filter((e) => {
    const q = searchQuery.toLowerCase()
    return (e.title || '').toLowerCase().includes(q) || (e.content || '').toLowerCase().includes(q)
  })

  /** Within “all”, open tasks first; then apply the same sort modes as Notes. */
  $: {
    void sortMode
    tasksDisplayOrder = [...filteredEntries].sort((a, b) => {
      if (tasksScope === 'all') {
        const ca = taskIsCompleted(a) ? 1 : 0
        const cb = taskIsCompleted(b) ? 1 : 0
        if (ca !== cb) return ca - cb
      }
      return compareTasks(a, b)
    })
  }

  /** “All” scope: two blocks — Open (incomplete), then Closed (completed). Other scopes: one flat list. */
  $: taskListSections =
    tasksScope === 'all'
      ? [
          {
            key: 'open',
            title: 'Open',
            tasks: tasksDisplayOrder.filter((t) => !taskIsCompleted(t))
          },
          {
            key: 'closed',
            title: 'Closed',
            tasks: tasksDisplayOrder.filter((t) => taskIsCompleted(t))
          }
        ]
      : [{ key: 'flat', title: null as string | null, tasks: tasksDisplayOrder }]

  onMount(() => {
    document.addEventListener('click', closeToolbarPopoversIfOutside)
    document.addEventListener('keydown', onGlobalKeydown)
  })

  onDestroy(() => {
    document.removeEventListener('click', closeToolbarPopoversIfOutside)
    document.removeEventListener('keydown', onGlobalKeydown)
    detachTaskReorderWindowListeners()
  })
</script>

<div class="page fade-in">
  <header class="page-header notes-header">
    <div>
      <h1 class="page-title">Tasks</h1>
      <p class="page-subtitle">Track what needs to get done.</p>
    </div>
    {#if tasksScope !== 'trash'}
      <button type="button" class="btn-primary" on:click={openNewTask}>
        <Icon icon="ph:plus" />
        New Task
      </button>
    {/if}
  </header>

  <div class="notes-toolbar">
    {#if showTasksListChrome}
      <div class="notes-search-bar">
        <span class="notes-search-bar-icon" aria-hidden="true">
          <Icon icon="ph:magnifying-glass" />
        </span>
        <input type="text" placeholder="Search tasks..." bind:value={searchQuery} />
      </div>
    {/if}
    <div class="notes-toolbar-actions">
      <div class="notes-toolbar-scope-dropdown" bind:this={scopePopoverEl}>
        <button
          type="button"
          class="notes-sort-cycle notes-toolbar-scope-trigger"
          title={tasksScopeOption.title}
          aria-label="Tasks list: {tasksScopeOption.label}. Open to change."
          aria-expanded={scopePopoverOpen}
          aria-haspopup="dialog"
          on:click|stopPropagation={toggleTasksScopePopover}
        >
          <span aria-hidden="true"><Icon icon={tasksScopeOption.icon} /></span>
          <span class="notes-toolbar-scope-trigger-label">{tasksScopeOption.label}</span>
        </button>
        {#if scopePopoverOpen}
          <div
            class="notes-scope-menu notes-label-filter-popover notes-toolbar-scope-popover"
            role="dialog"
            aria-label="Which tasks to show"
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
                {#each TASKS_SCOPE_OPTIONS as opt (opt.id)}
                  <button
                    type="button"
                    class="notes-scope-option"
                    class:active={tasksScope === opt.id}
                    on:click={() => selectTasksScope(opt.id)}
                  >
                    <span class="notes-toolbar-scope-option-icon" aria-hidden="true">
                      <Icon icon={opt.icon} />
                    </span>
                    {opt.label}
                  </button>
                {/each}
              </div>
            </div>
          </div>
        {/if}
      </div>
      {#if showTasksListChrome}
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
        {#if showTasksListChrome && tasksScope !== 'trash' && lastActiveTaskTags.length > 0}
          <div class="notes-label-filter-dropdown" bind:this={labelFilterEl}>
            <button
              type="button"
              class="notes-sort-cycle notes-label-filter-toggle"
              class:has-filter={selectedTags.length > 0}
              title={selectedTags.length > 0
                ? `${selectedTags.length} tag filter${selectedTags.length === 1 ? '' : 's'} active — click to change`
                : 'Filter tasks by tag'}
              aria-label={selectedTags.length > 0
                ? `Tag filters: ${selectedTags.length} active. Open to search or change.`
                : 'Open tag filter'}
              aria-expanded={labelFilterOpen}
              aria-haspopup="dialog"
              on:click|stopPropagation={toggleTaskTagFilterPopover}
            >
              <Icon icon="ph:tag" />
              {#if selectedTags.length > 0}
                <span class="notes-label-filter-count">{selectedTags.length}</span>
              {/if}
            </button>
            {#if labelFilterOpen}
              <div
                class="notes-scope-menu notes-label-filter-popover"
                role="dialog"
                aria-label="Filter tasks by tags"
                on:click|stopPropagation
              >
                <div class="notes-label-filter-popover-head">
                  <div class="notes-label-filter-popover-meta">
                    <span class="notes-label-filter-popover-heading">Tags</span>
                    <button type="button" class="notes-label-filter-clear-all" on:click={clearTaskTagFilters}>
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
                      placeholder="Search tags…"
                      bind:value={labelFilterSearch}
                      aria-label="Search tags"
                    />
                  </div>
                </div>
                <div class="notes-label-filter-popover-body">
                  <div class="notes-label-filter-menu notes-label-filter-menu-scroll">
                    {#if labelsMatchingPopoverSearch.length === 0}
                      <div class="notes-label-filter-no-match">No tags match</div>
                    {:else}
                      {#each labelsMatchingPopoverSearch as tag}
                        <label class="notes-label-filter-row">
                          <input
                            type="checkbox"
                            checked={selectedTags.includes(tag)}
                            on:change={() => toggleTaskTagFilterLabel(tag)}
                          />
                          <span>{tag}</span>
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

  {#if showTasksListChrome && tasksScope === 'trash' && entries.length > 0}
    <div style="margin-bottom: var(--space-md);">
      <button type="button" class="btn-ghost danger" on:click={emptyTrashConfirm}>Empty trash</button>
    </div>
  {/if}

  {#if error}
    <div class="state-container">
      <Icon icon="ph:warning-circle" />
      <p>{error}</p>
    </div>
  {:else if loading && entries.length === 0}
    <div class="state-container">
      <Icon icon="ph:spinner-gap-duotone" />
      <p>Loading tasks...</p>
    </div>
  {:else if entries.length === 0}
    <div class="notes-empty">
      <Icon icon={tasksScope === 'trash' ? 'ph:trash' : 'ph:list-checks'} />
      <p>
        {tasksScope === 'trash' ? 'Trash is empty' : 'No tasks yet'}
      </p>
    </div>
  {:else if filteredEntries.length === 0}
    <div class="notes-empty">
      <Icon icon="ph:funnel" />
      <p>No tasks match your filters</p>
    </div>
  {:else}
    <div class="task-list-large">
      {#each taskListSections as section (section.key)}
        <div class="notes-section">
          {#if section.title}
            <h3 class="notes-section-title">{section.title}</h3>
          {/if}
          <div class="task-list-section-rows" data-tasks-zone={section.key}>
          {#each section.tasks as task (task.id)}
            <div
              class="task-row-wrapper"
              animate:flip={{ duration: dragTaskId === task.id ? 0 : 200 }}
            >
              {#if dragTaskId && dropIndicator.targetId === task.id && dropIndicator.zone === section.key && dropIndicator.position === 'before'}
                <div class="task-drop-spacer" data-task-drop-spacer="true"></div>
              {/if}
              <div
                class="task-row"
                class:task-row--reorder={tasksReorderEnabled}
                class:task-row--trash={tasksScope === 'trash'}
                class:completed={task.is_completed}
                class:dragging={dragTaskId === task.id}
                data-task-id={task.id}
                on:pointerdown={(e) => taskReorderPointerDown(e, task)}
                style:position={dragTaskId === task.id ? 'fixed' : undefined}
                style:left={dragTaskId === task.id ? `${dragTaskPos.x - dragContainerOffset.x}px` : undefined}
                style:top={dragTaskId === task.id ? `${dragTaskPos.y - dragContainerOffset.y}px` : undefined}
                style:width={dragTaskId === task.id && dragFloatSize.w > 0 ? `${dragFloatSize.w}px` : undefined}
                style:height={dragTaskId === task.id && dragFloatSize.h > 0 ? `${dragFloatSize.h}px` : undefined}
                style:z-index={dragTaskId === task.id ? '1000' : undefined}
                style:margin={dragTaskId === task.id ? '0' : undefined}
                role="button"
                tabindex="0"
                aria-grabbed={tasksReorderEnabled && dragTaskId === task.id ? true : undefined}
                on:click={() => handleTaskRowClick(task)}
                on:keydown={(e) => e.key === 'Enter' && handleTaskRowClick(task)}
              >
              {#if tasksScope !== 'trash'}
                <button type="button" class="checkbox" on:click|stopPropagation={() => toggleComplete(task)}>
                  {#if task.is_completed}
                    <Icon icon="ph:check" />
                  {/if}
                </button>
              {/if}
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
                      {task.subtasks.filter((s) => s.is_completed).length}/{task.subtasks.length}
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
              <div class="task-row-actions" role="group" on:click|stopPropagation on:keydown|stopPropagation>
                {#if tasksScope === 'trash'}
                  <button
                    type="button"
                    class="note-action-btn"
                    title="Restore"
                    aria-label="Restore task"
                    on:click|stopPropagation={() => restoreTaskFromTrash(task)}
                  >
                    <Icon icon="ph:arrow-counter-clockwise" />
                  </button>
                  <button
                    type="button"
                    class="note-action-btn delete"
                    title="Delete permanently"
                    aria-label="Delete permanently"
                    on:click|stopPropagation={() => permanentlyDeleteTask(task.id)}
                  >
                    <Icon icon="ph:trash" />
                  </button>
                {:else}
                  <button
                    type="button"
                    class="note-action-btn delete"
                    title="Move to trash"
                    aria-label="Move to trash"
                    on:click|stopPropagation={() => moveTaskToTrash(task)}
                  >
                    <Icon icon="ph:trash" />
                  </button>
                {/if}
              </div>
            </div>
            </div>
          {/each}
          {#if dragTaskId && dropIndicator.zone === section.key && dropIndicator.position === 'after'}
            <div class="task-drop-spacer" data-task-drop-spacer="true"></div>
          {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
