<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte'
  import { getEntriesWithReminder } from '../../lib/api/db'
  import type { Entry } from '../../types'
  import Icon from '@iconify/svelte'
  import { selectedTaskId } from '../../lib/taskDetailStore'
  import { selectedNoteId } from '../../lib/noteDetailStore'
  import { noteDetailReturnTo, taskDetailReturnTo } from '../../lib/detailReturnStore'
  export let navigate: (page: string) => void = () => {}

  let entries: Entry[] = []
  let flatDisplayOrder: Entry[] = []
  let loading = true
  let error: string | null = null
  let searchQuery = ''

  /** OR filter: reminder must have at least one of these tags. */
  let selectedTags: string[] = []

  let labelFilterOpen = false
  let labelFilterEl: HTMLDivElement | null = null
  /** Narrows the tag list inside the popover (not the main reminders search). */
  let labelFilterSearch = ''
  let labelFilterSearchInput: HTMLInputElement | null = null

  function closeLabelFilterIfOutside(e: MouseEvent) {
    if (!labelFilterOpen) return
    const el = labelFilterEl
    const t = e.target
    if (el && t instanceof Node && el.contains(t)) return
    labelFilterOpen = false
  }

  function onGlobalKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && labelFilterOpen) labelFilterOpen = false
  }

  function toggleTagFilterLabel(tag: string) {
    if (selectedTags.includes(tag)) {
      selectedTags = selectedTags.filter((t) => t !== tag)
    } else {
      selectedTags = [...selectedTags, tag].sort((a, b) =>
        a.localeCompare(b, undefined, { sensitivity: 'base' })
      )
    }
  }

  function clearTagFilters() {
    selectedTags = []
  }

  async function toggleTagFilterPopover() {
    labelFilterOpen = !labelFilterOpen
    if (labelFilterOpen) {
      await tick()
      labelFilterSearchInput?.focus()
    }
  }

  type RemindersSortMode = 'grouped' | 'time_asc' | 'time_desc' | 'title_asc' | 'title_desc'
  let sortMode: RemindersSortMode = 'grouped'

  const SORT_CYCLE: RemindersSortMode[] = [
    'grouped',
    'time_asc',
    'time_desc',
    'title_asc',
    'title_desc'
  ]

  const SORT_LABELS: Record<RemindersSortMode, string> = {
    grouped: 'Grouped',
    time_asc: 'Soonest first',
    time_desc: 'Latest first',
    title_asc: 'Title A–Z',
    title_desc: 'Title Z–A'
  }

  const SORT_ICONS: Record<RemindersSortMode, string> = {
    grouped: 'ph:rows',
    time_asc: 'ph:arrow-up',
    time_desc: 'ph:arrow-down',
    title_asc: 'ph:sort-ascending',
    title_desc: 'ph:sort-descending'
  }

  function cycleSortMode() {
    labelFilterOpen = false
    const i = SORT_CYCLE.indexOf(sortMode)
    sortMode = SORT_CYCLE[(i + 1) % SORT_CYCLE.length]
  }

  $: sortModeLabel = SORT_LABELS[sortMode]
  $: sortModeIcon = SORT_ICONS[sortMode]

  $: showRemindersListChrome =
    !!error || entries.length > 0 || !!searchQuery.trim() || selectedTags.length > 0

  $: reminderTags = [...new Set(entries.flatMap((e) => e.tags || []))].sort((a, b) =>
    a.localeCompare(b, undefined, { sensitivity: 'base' })
  )

  $: labelsMatchingPopoverSearch = reminderTags.filter((l) =>
    l.toLowerCase().includes(labelFilterSearch.trim().toLowerCase())
  )

  /** Drop tag filters when nothing in the list has tags (e.g. after reload). */
  $: if (reminderTags.length === 0 && selectedTags.length > 0) selectedTags = []

  function formatReminder(iso: string | null) {
    if (!iso) return ''
    const d = new Date(iso)
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' })
  }

  function handleRowClick(entry: Entry) {
    if (entry.entry_type === 'note') {
      noteDetailReturnTo.set({ type: 'reminders' })
      selectedNoteId.set(entry.id)
      navigate('note-detail')
    } else if (entry.entry_type === 'task') {
      taskDetailReturnTo.set({ type: 'reminders' })
      selectedTaskId.set(entry.id)
      navigate('task-detail')
    }
  }

  async function load() {
    loading = true
    error = null
    try {
      const result = await getEntriesWithReminder()
      entries = Array.isArray(result) ? result : []
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    } finally {
      loading = false
    }
  }

  function reminderTitle(entry: Entry): string {
    return entry.title?.trim() || entry.content?.trim() || '(no title)'
  }

  function sourceBadgeLabel(entry: Entry): string {
    if (entry.entry_type === 'note') return 'Note'
    if (entry.entry_type === 'task') return 'Task'
    return 'Entry'
  }

  /** Task rows may surface due_date when no reminder_at; notes use reminder_at. */
  function effectiveReminderAt(entry: Entry): string | null {
    if (entry.entry_type === 'task') {
      return entry.reminder_at ?? entry.due_date ?? null
    }
    return entry.reminder_at
  }

  function effectiveTimeMs(entry: Entry): number {
    const iso = effectiveReminderAt(entry)
    return iso ? new Date(iso).getTime() : 0
  }

  function startOfLocalDay(d: Date): number {
    const x = new Date(d)
    x.setHours(0, 0, 0, 0)
    return x.getTime()
  }

  function startOfTomorrowFrom(now: Date): number {
    const x = new Date(startOfLocalDay(now))
    x.setDate(x.getDate() + 1)
    return x.getTime()
  }

  $: filteredEntries = entries.filter((e) => {
    if (selectedTags.length > 0) {
      const tags = e.tags || []
      if (!selectedTags.some((t) => tags.includes(t))) return false
    }
    const q = searchQuery.toLowerCase()
    const matchesSearch =
      (e.title || '').toLowerCase().includes(q) || (e.content || '').toLowerCase().includes(q)
    return matchesSearch
  })

  function compareFlat(a: Entry, b: Entry): number {
    if (sortMode === 'title_asc' || sortMode === 'title_desc') {
      const ta = reminderTitle(a).toLowerCase()
      const tb = reminderTitle(b).toLowerCase()
      const c = ta.localeCompare(tb, undefined, { sensitivity: 'base' })
      return sortMode === 'title_asc' ? c : -c
    }
    const da = effectiveTimeMs(a)
    const db = effectiveTimeMs(b)
    if (da !== db) return sortMode === 'time_desc' ? db - da : da - db
    return a.id.localeCompare(b.id)
  }

  $: {
    void sortMode
    flatDisplayOrder = [...filteredEntries].sort(compareFlat)
  }

  /** Grouped: recurring first section; then non-rrule by calendar day vs today. */
  $: groupedSections = ((): { title: string; items: Entry[] }[] => {
    if (sortMode !== 'grouped') return []
    const now = new Date()
    const sod = startOfLocalDay(now)
    const stom = startOfTomorrowFrom(now)

    const recurring: Entry[] = []
    const dueToday: Entry[] = []
    const upcoming: Entry[] = []
    const past: Entry[] = []

    for (const e of filteredEntries) {
      if (e.rrule) {
        recurring.push(e)
        continue
      }
      const t = effectiveTimeMs(e)
      if (t < sod) past.push(e)
      else if (t < stom) dueToday.push(e)
      else upcoming.push(e)
    }

    const byTimeAsc = (a: Entry, b: Entry) => {
      const c = effectiveTimeMs(a) - effectiveTimeMs(b)
      return c !== 0 ? c : a.id.localeCompare(b.id)
    }
    const byTimeDesc = (a: Entry, b: Entry) => {
      const c = effectiveTimeMs(b) - effectiveTimeMs(a)
      return c !== 0 ? c : a.id.localeCompare(b.id)
    }

    recurring.sort(byTimeAsc)
    dueToday.sort(byTimeAsc)
    upcoming.sort(byTimeAsc)
    past.sort(byTimeDesc)

    const out: { title: string; items: Entry[] }[] = []
    if (recurring.length) out.push({ title: 'Recurring', items: recurring })
    if (dueToday.length) out.push({ title: 'Due today', items: dueToday })
    if (upcoming.length) out.push({ title: 'Upcoming', items: upcoming })
    if (past.length) out.push({ title: 'Past', items: past })
    return out
  })()

  /** One render pipeline: grouped sections or a single flat list (no section title). */
  $: reminderListBlocks =
    sortMode === 'grouped'
      ? groupedSections.map((s) => ({ sectionTitle: s.title, items: s.items }))
      : [{ sectionTitle: null as string | null, items: flatDisplayOrder }]

  onMount(() => {
    load()
    document.addEventListener('click', closeLabelFilterIfOutside)
    document.addEventListener('keydown', onGlobalKeydown)
  })

  onDestroy(() => {
    document.removeEventListener('click', closeLabelFilterIfOutside)
    document.removeEventListener('keydown', onGlobalKeydown)
  })
</script>

<div class="page fade-in">
  <header class="page-header notes-header">
    <div>
      <h1 class="page-title">Reminders</h1>
      <p class="page-subtitle">Notes and tasks with a due time or reminder. Add times in Notes or Tasks.</p>
    </div>
  </header>

  <div class="notes-toolbar">
    {#if showRemindersListChrome}
      <div class="notes-search-bar">
        <span class="notes-search-bar-icon" aria-hidden="true">
          <Icon icon="ph:magnifying-glass" />
        </span>
        <input type="text" placeholder="Search reminders..." bind:value={searchQuery} />
      </div>
    {/if}
    <div class="notes-toolbar-actions">
      <div class="notes-toolbar-actions-scroll">
      {#if showRemindersListChrome}
        <button
          type="button"
          class="notes-sort-cycle"
          title={sortModeLabel}
          aria-label="Sort: {sortModeLabel}. Click to change sort order."
          on:click|stopPropagation={cycleSortMode}
        >
          <span aria-hidden="true"><Icon icon={sortModeIcon} /></span>
        </button>
      {/if}
      </div>
        {#if showRemindersListChrome && reminderTags.length > 0}
          <div class="notes-label-filter-dropdown" bind:this={labelFilterEl}>
            <button
              type="button"
              class="notes-sort-cycle notes-label-filter-toggle"
              class:has-filter={selectedTags.length > 0}
              title={selectedTags.length > 0
                ? `${selectedTags.length} tag filter${selectedTags.length === 1 ? '' : 's'} active — click to change`
                : 'Filter reminders by tag'}
              aria-label={selectedTags.length > 0
                ? `Tag filters: ${selectedTags.length} active. Open to search or change.`
                : 'Open tag filter'}
              aria-expanded={labelFilterOpen}
              aria-haspopup="dialog"
              on:click|stopPropagation={toggleTagFilterPopover}
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
                aria-label="Filter reminders by tags"
                on:click|stopPropagation
              >
                <div class="notes-label-filter-popover-head">
                  <div class="notes-label-filter-popover-meta">
                    <span class="notes-label-filter-popover-heading">Tags</span>
                    <button type="button" class="notes-label-filter-clear-all" on:click={clearTagFilters}>
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
                            on:change={() => toggleTagFilterLabel(tag)}
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

  {#if error}
    <div class="empty-state">
      <Icon icon="ph:warning-circle" />
      <p>{error}</p>
    </div>
  {:else if loading && entries.length === 0}
    <div class="empty-state">
      <Icon icon="ph:spinner-gap" class="spin-icon" />
      <p>Loading reminders...</p>
    </div>
  {:else if filteredEntries.length === 0}
    <div class="empty-state">
      <Icon icon="ph:bell-slash" />
      <p>
        {searchQuery || selectedTags.length > 0
          ? 'No reminders match your filters'
          : 'No reminders yet'}
      </p>
    </div>
  {:else}
    <div class:notes-lists-container={sortMode === 'grouped'}>
      {#each reminderListBlocks as block (block.sectionTitle ?? 'flat')}
        <div class:notes-section={sortMode === 'grouped'}>
          {#if block.sectionTitle}
            <h3 class="notes-section-title">{block.sectionTitle}</h3>
          {/if}
          <div class="reminder-list-large">
            {#each block.items as entry (entry.id)}
              <div
                class="reminder-row"
                class:from-note={entry.entry_type === 'note'}
                class:from-task={entry.entry_type === 'task'}
                on:click={() => handleRowClick(entry)}
                role="button"
                tabindex="0"
                on:keydown={(e) => e.key === 'Enter' && handleRowClick(entry)}
              >
                <div class="reminder-icon-large" class:recurring={!!entry.rrule}>
                  {#if entry.rrule}
                    <Icon icon="ph:arrows-clockwise" />
                  {:else if entry.entry_type === 'note'}
                    <Icon icon="ph:notebook" />
                  {:else if entry.entry_type === 'task'}
                    <Icon icon="ph:check-circle" />
                  {:else}
                    <Icon icon="ph:bell" />
                  {/if}
                </div>
                <div class="reminder-info">
                  <div class="reminder-title-row">
                    <span class="reminder-text">{reminderTitle(entry)}</span>
                    <span
                      class="reminder-source-badge"
                      class:note={entry.entry_type === 'note'}
                      class:task={entry.entry_type === 'task'}
                    >
                      {sourceBadgeLabel(entry)}
                    </span>
                  </div>
                  <div class="reminder-meta">
                    {#if effectiveReminderAt(entry)}
                      <span class="reminder-time">
                        <Icon icon="ph:clock" />
                        {formatReminder(effectiveReminderAt(entry))}
                      </span>
                    {/if}
                    {#if entry.rrule}
                      <span class="reminder-recurring-badge">
                        <Icon icon="ph:repeat" />
                        Recurring
                      </span>
                    {/if}
                    {#if entry.tags && entry.tags.length > 0}
                      <div class="reminder-tags">
                        {#each entry.tags as tag}
                          <span class="reminder-tag">{tag}</span>
                        {/each}
                      </div>
                    {/if}
                  </div>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
