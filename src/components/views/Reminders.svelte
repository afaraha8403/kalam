<script lang="ts">
  import { onMount } from 'svelte'
  import { getEntriesWithReminder, createEntry, updateEntry, deleteEntry, newEntry } from '../../lib/api/db'
  import type { Entry } from '../../types'
  import Icon from '@iconify/svelte'
  import { selectedTaskId } from '../../lib/taskDetailStore'
  import { RRule, rrulestr } from 'rrule'
  import SidePanel from '../ui/SidePanel.svelte'
  import SearchFilterBar from '../ui/SearchFilterBar.svelte'

  export let onNavigateToPage: ((page: 'notes' | 'tasks') => void) | undefined = undefined

  let entries: Entry[] = []
  let loading = true
  let error: string | null = null
  let searchQuery = ''

  // Panel State
  let isPanelOpen = false
  let panelMode: 'add' | 'edit' = 'add'
  let panelReminderId: string | null = null
  let draftContent = ''
  let draftReminderAt = ''
  let draftRruleRaw: string | null = null
  let draftRrulePreset = 'none'

  // Custom RRule Modal State
  let showCustomModal = false
  let customFreq = 'WEEKLY'
  let customInterval = 1
  let customDays: string[] = []
  let customEndType = 'never'
  let customUntil = ''
  let customCount = 1
  let previousPreset = 'none'

  const DAYS_OF_WEEK = [
    { value: 'SU', label: 'S' },
    { value: 'MO', label: 'M' },
    { value: 'TU', label: 'T' },
    { value: 'WE', label: 'W' },
    { value: 'TH', label: 'T' },
    { value: 'FR', label: 'F' },
    { value: 'SA', label: 'S' }
  ]

  $: draftPresets = getDynamicPresets(draftReminderAt, draftRruleRaw)

  function getDynamicPresets(dateStr: string | null, currentRaw: string | null) {
    const base = [
      { value: 'none', label: 'Does not repeat', rrule: null },
      { value: 'daily', label: 'Daily', rrule: 'FREQ=DAILY' },
    ]
    if (!dateStr) {
      base.push({ value: 'weekly', label: 'Weekly', rrule: 'FREQ=WEEKLY' })
      base.push({ value: 'monthly', label: 'Monthly', rrule: 'FREQ=MONTHLY' })
      base.push({ value: 'yearly', label: 'Annually', rrule: 'FREQ=YEARLY' })
      base.push({ value: 'weekday', label: 'Every weekday (Monday to Friday)', rrule: 'FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR' })
    } else {
      const d = new Date(dateStr)
      if (!isNaN(d.getTime())) {
        const days = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday']
        const rruleDays = ['SU', 'MO', 'TU', 'WE', 'TH', 'FR', 'SA']
        const dayName = days[d.getDay()]
        const rruleDay = rruleDays[d.getDay()]
        
        const date = d.getDate()
        const monthNames = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December']
        const monthName = monthNames[d.getMonth()]
        
        const weekOfMonth = Math.ceil(date / 7)
        const isLast = (date + 7) > new Date(d.getFullYear(), d.getMonth() + 1, 0).getDate()
        const prefix = isLast ? -1 : weekOfMonth
        const weekStr = isLast ? 'last' : ['first', 'second', 'third', 'fourth', 'fifth'][weekOfMonth - 1]

        base.push({ value: 'weekly', label: `Weekly on ${dayName}`, rrule: `FREQ=WEEKLY;BYDAY=${rruleDay}` })
        base.push({ value: 'monthly', label: `Monthly on the ${weekStr} ${dayName}`, rrule: `FREQ=MONTHLY;BYDAY=${prefix}${rruleDay}` })
        base.push({ value: 'yearly', label: `Annually on ${monthName} ${date}`, rrule: `FREQ=YEARLY;BYMONTH=${d.getMonth() + 1};BYMONTHDAY=${date}` })
        base.push({ value: 'weekday', label: 'Every weekday (Monday to Friday)', rrule: 'FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR' })
      }
    }
    
    if (currentRaw && !base.find(p => p.rrule === currentRaw)) {
      base.push({ value: 'custom_saved', label: formatRRule(currentRaw), rrule: currentRaw })
    }
    
    base.push({ value: 'custom', label: 'Custom...', rrule: 'custom' })
    return base
  }

  function handlePresetChange() {
    const preset = draftPresets.find(p => p.value === draftRrulePreset)
    
    if (preset?.value === 'custom') {
      openCustomModal()
    } else {
      draftRruleRaw = preset?.rrule ?? null
    }
  }

  function openCustomModal() {
    previousPreset = draftRrulePreset
    
    customFreq = 'WEEKLY'
    customInterval = 1
    customDays = []
    customEndType = 'never'
    customUntil = ''
    customCount = 1

    if (draftRruleRaw) {
      try {
        const rule = rrulestr(draftRruleRaw)
        const opts = rule.options
        customFreq = ['YEARLY', 'MONTHLY', 'WEEKLY', 'DAILY', 'HOURLY', 'MINUTELY', 'SECONDLY'][opts.freq] || 'WEEKLY'
        customInterval = opts.interval || 1
        if (opts.byweekday) {
          const rruleDays = ['MO', 'TU', 'WE', 'TH', 'FR', 'SA', 'SU']
          customDays = opts.byweekday.map((d: any) => rruleDays[d.weekday])
        }
        if (opts.until) {
          customEndType = 'until'
          customUntil = opts.until.toISOString().slice(0, 10)
        } else if (opts.count) {
          customEndType = 'count'
          customCount = opts.count
        }
      } catch (e) {
        console.error('Failed to parse existing rrule for custom modal', e)
      }
    } else {
      const d = new Date(draftReminderAt || Date.now())
      const rruleDays = ['SU', 'MO', 'TU', 'WE', 'TH', 'FR', 'SA']
      customDays = [rruleDays[d.getDay()]]
    }
    
    showCustomModal = true
  }

  function cancelCustomModal() {
    draftRrulePreset = previousPreset
    showCustomModal = false
  }

  function saveCustomModal() {
    const parts = [`FREQ=${customFreq}`]
    if (customInterval > 1) parts.push(`INTERVAL=${customInterval}`)
    if (customFreq === 'WEEKLY' && customDays.length > 0) {
      parts.push(`BYDAY=${customDays.join(',')}`)
    }
    if (customEndType === 'until' && customUntil) {
      const d = new Date(customUntil)
      const untilStr = d.toISOString().replace(/[-:]/g, '').slice(0, 15) + 'Z'
      parts.push(`UNTIL=${untilStr}`)
    } else if (customEndType === 'count' && customCount > 0) {
      parts.push(`COUNT=${customCount}`)
    }

    const rruleStr = parts.join(';')
    
    draftRruleRaw = rruleStr
    draftRrulePreset = 'custom_saved'
    
    showCustomModal = false
  }

  function formatRRule(rruleStr: string | null) {
    if (!rruleStr) return ''
    try {
      const rule = rrulestr(rruleStr)
      const text = rule.toText()
      if (!text) return rruleStr
      return text.charAt(0).toUpperCase() + text.slice(1)
    } catch (e) {
      return rruleStr
    }
  }

  function setQuickDate(type: 'today' | 'tomorrow' | 'next_week') {
    const d = new Date()
    if (type === 'today') {
      d.setHours(18, 0, 0, 0) // 6 PM today
      if (d.getTime() < Date.now()) d.setHours(20, 0, 0, 0) // 8 PM if 6 PM passed
    } else if (type === 'tomorrow') {
      d.setDate(d.getDate() + 1)
      d.setHours(9, 0, 0, 0) // 9 AM tomorrow
    } else if (type === 'next_week') {
      d.setDate(d.getDate() + 7)
      d.setHours(9, 0, 0, 0) // 9 AM next week
    }
    // format to YYYY-MM-DDThh:mm
    const y = d.getFullYear()
    const m = String(d.getMonth() + 1).padStart(2, '0')
    const day = String(d.getDate()).padStart(2, '0')
    const h = String(d.getHours()).padStart(2, '0')
    const min = String(d.getMinutes()).padStart(2, '0')
    draftReminderAt = `${y}-${m}-${day}T${h}:${min}`
  }

  function openAddPanel() {
    panelMode = 'add'
    panelReminderId = null
    draftContent = ''
    draftReminderAt = ''
    draftRruleRaw = null
    draftRrulePreset = 'none'
    isPanelOpen = true
  }

  function openEditPanel(entry: Entry) {
    if (entry.entry_type !== 'reminder') return
    panelMode = 'edit'
    panelReminderId = entry.id
    draftContent = entry.content
    draftReminderAt = entry.reminder_at
      ? new Date(entry.reminder_at).toISOString().slice(0, 16)
      : ''
    draftRruleRaw = entry.rrule
    
    const presets = getDynamicPresets(draftReminderAt, draftRruleRaw)
    const preset = presets.find((p) => p.rrule === entry.rrule || (p.rrule === null && !entry.rrule))
    if (preset && preset.value !== 'custom') {
      draftRrulePreset = preset.value
    } else if (entry.rrule) {
      draftRrulePreset = 'custom_saved'
    } else {
      draftRrulePreset = 'none'
    }
    isPanelOpen = true
  }

  function closePanel() {
    isPanelOpen = false
    panelReminderId = null
  }

  async function savePanel() {
    const content = draftContent.trim()
    if (!content) return

    const reminderAt = draftReminderAt.trim() || null

    try {
      if (panelMode === 'add') {
        const entry = newEntry('reminder', content, { reminder_at: reminderAt, rrule: draftRruleRaw })
        await createEntry(entry)
        entries = [entry, ...entries]
      } else if (panelMode === 'edit' && panelReminderId) {
        const original = entries.find(e => e.id === panelReminderId)
        if (!original) return
        const updated: Entry = {
          ...original,
          content,
          reminder_at: reminderAt,
          rrule: draftRruleRaw,
          updated_at: new Date().toISOString()
        }
        await updateEntry(updated)
        entries = entries.map((e) => (e.id === updated.id ? updated : e))
      }
      closePanel()
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
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

  async function toggleComplete(entry: Entry) {
    try {
      const updated = { ...entry, is_completed: !entry.is_completed, updated_at: new Date().toISOString() }
      entries = entries.map(e => e.id === entry.id ? updated : e)
      await updateEntry(updated)
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  async function remove(id: string) {
    try {
      entries = entries.filter(e => e.id !== id)
      if (panelReminderId === id) closePanel()
      await deleteEntry(id)
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  async function snooze(entry: Entry) {
    const inOneHour = new Date(Date.now() + 60 * 60 * 1000).toISOString()
    const updated = { ...entry, reminder_at: inOneHour, updated_at: new Date().toISOString() }
    try {
      await updateEntry(updated)
      entries = entries.map((e) => (e.id === entry.id ? updated : e))
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  async function clearReminder(entry: Entry) {
    if (entry.entry_type === 'reminder') return
    try {
      const updated = { ...entry, reminder_at: null, updated_at: new Date().toISOString() }
      await updateEntry(updated)
      entries = entries.filter((e) => e.id !== entry.id)
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  function openNote(entry: Entry) {
    onNavigateToPage?.('notes')
  }

  function openTask(entry: Entry) {
    selectedTaskId.set(entry.id)
    onNavigateToPage?.('tasks')
  }

  function formatDateTime(iso: string | null) {
    if (!iso) return 'No date set'
    try {
      const d = new Date(iso)
      const today = new Date()
      const tomorrow = new Date(today)
      tomorrow.setDate(tomorrow.getDate() + 1)

      const timeStr = d.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })

      if (d.toDateString() === today.toDateString()) {
        return `Today at ${timeStr}`
      } else if (d.toDateString() === tomorrow.toDateString()) {
        return `Tomorrow at ${timeStr}`
      }
      return d.toLocaleString(undefined, { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' })
    } catch {
      return iso
    }
  }

  function handleItemClick(entry: Entry) {
    if (entry.entry_type === 'reminder') {
      openEditPanel(entry)
    } else if (entry.entry_type === 'note') {
      openNote(entry)
    } else if (entry.entry_type === 'task') {
      openTask(entry)
    }
  }

  $: filteredEntries = entries.filter(e => {
    const matchesSearch = (e.title || '').toLowerCase().includes(searchQuery.toLowerCase()) || 
                          (e.content || '').toLowerCase().includes(searchQuery.toLowerCase())
    return matchesSearch
  })

  $: activeReminders = filteredEntries.filter(
    (e) => (e.entry_type === 'reminder' ? !e.is_completed : true)
  )
  $: completedReminders = filteredEntries.filter(
    (e) => e.entry_type === 'reminder' && !!e.is_completed
  )

  type DateGroup = 'overdue' | 'today' | 'tomorrow' | 'later'
  function getDateGroup(reminderAt: string | null): DateGroup {
    if (!reminderAt) return 'later'
    const t = new Date(reminderAt).getTime()
    const now = new Date()
    const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime()
    const tomorrowStart = todayStart + 24 * 60 * 60 * 1000
    const dayAfterStart = tomorrowStart + 24 * 60 * 60 * 1000
    if (t < todayStart) return 'overdue'
    if (t >= todayStart && t < tomorrowStart) return 'today'
    if (t >= tomorrowStart && t < dayAfterStart) return 'tomorrow'
    return 'later'
  }

  $: groupedActive = (() => {
    const groups: Record<DateGroup, Entry[]> = { overdue: [], today: [], tomorrow: [], later: [] }
    for (const e of activeReminders) {
      const g = getDateGroup(e.reminder_at)
      groups[g].push(e)
    }
    return groups
  })()

  const groupLabels: Record<DateGroup, string> = {
    overdue: 'Overdue',
    today: 'Today',
    tomorrow: 'Tomorrow',
    later: 'Later'
  }
  const dateGroupOrder: DateGroup[] = ['overdue', 'today', 'tomorrow', 'later']

  onMount(() => load())
</script>

<div class="view reminders-view">
  <header class="page-header">
    <div class="header-content">
      <div class="title-wrapper">
        <Icon icon="ph:bell-ringing-duotone" class="header-icon" />
        <h2>Reminders</h2>
      </div>
      <p class="subtitle">Never forget an important detail or follow-up.</p>
    </div>
    <div class="header-actions">
      <button class="btn-primary" on:click={openAddPanel}>
        <Icon icon="ph:plus-bold" /> Add Reminder
      </button>
    </div>
  </header>

  <SearchFilterBar bind:searchQuery placeholder="Search reminders..." />

  {#if error}
    <div class="state-container error-state">
      <Icon icon="ph:warning-circle-duotone" class="error-icon" />
      <p>{error}</p>
    </div>
  {/if}

  {#if loading && entries.length === 0}
    <div class="state-container">
      <Icon icon="ph:spinner-gap-duotone" class="spin-icon" />
      <p>Loading reminders...</p>
    </div>
  {:else if entries.length === 0}
    <div class="state-container empty-state">
      <div class="empty-icon-wrapper">
        <Icon icon="ph:bell-z-duotone" class="empty-icon" />
      </div>
      <h3>No reminders</h3>
      <p>You're all clear! Add a new reminder above to get notified.</p>
    </div>
  {:else if filteredEntries.length === 0}
    <div class="state-container empty-state">
      <div class="empty-icon-wrapper">
        <Icon icon="ph:magnifying-glass-duotone" class="empty-icon" />
      </div>
      <h3>No results found</h3>
      <p>Try adjusting your search query.</p>
    </div>
  {:else}
    <div class="reminder-sections">
      {#each dateGroupOrder as group}
        {#if groupedActive[group].length > 0}
          <div class="reminder-group">
            <div class="section-header">
              <h3 class:overdue={group === 'overdue'}>{groupLabels[group]}</h3>
              <span class="count">{groupedActive[group].length}</span>
            </div>
            <div class="reminder-list">
              {#each groupedActive[group] as entry (entry.id)}
                <div class="reminder-item" class:is-note={entry.entry_type === 'note'} class:is-task={entry.entry_type === 'task'} role="button" tabindex="0" on:click={() => handleItemClick(entry)} on:keydown={(e) => e.key === 'Enter' && handleItemClick(entry)}>
                  {#if entry.entry_type === 'reminder'}
                    <button class="checkbox" on:click|stopPropagation={() => toggleComplete(entry)}>
                      <div class="check-circle">
                        <Icon icon="ph:check-bold" class="check-icon" />
                      </div>
                    </button>
                    <div class="reminder-content">
                      <span class="reminder-title">{entry.content}</span>
                      <div class="reminder-meta">
                        <span class="reminder-time" class:has-date={entry.reminder_at}>
                          <Icon icon="ph:clock-duotone" />
                          {formatDateTime(entry.reminder_at)}
                        </span>
                        {#if entry.rrule}
                          <span class="reminder-repeat">
                            <Icon icon="ph:arrows-clockwise-duotone" />
                            {formatRRule(entry.rrule)}
                          </span>
                        {/if}
                      </div>
                    </div>
                    <div class="reminder-actions" on:click|stopPropagation on:keydown|stopPropagation>
                      <button class="action-btn" on:click={() => snooze(entry)} title="Snooze 1 hour">
                        <Icon icon="ph:clock-countdown-duotone" />
                      </button>
                      <button class="action-btn delete" on:click={() => remove(entry.id)} title="Delete reminder">
                        <Icon icon="ph:trash-duotone" />
                      </button>
                    </div>
                  {:else if entry.entry_type === 'note'}
                    <div class="checkbox-spacer" aria-hidden="true"></div>
                    <div class="reminder-content">
                      <span class="reminder-type-badge">Note</span>
                      <span class="reminder-title">{entry.title || entry.content || '(no title)'}</span>
                      <div class="reminder-meta">
                        <span class="reminder-time" class:has-date={entry.reminder_at}>
                          <Icon icon="ph:clock-duotone" />
                          {formatDateTime(entry.reminder_at)}
                        </span>
                      </div>
                    </div>
                    <div class="reminder-actions" on:click|stopPropagation on:keydown|stopPropagation>
                      <button class="action-btn" on:click={() => snooze(entry)} title="Snooze 1 hour">
                        <Icon icon="ph:clock-countdown-duotone" />
                      </button>
                      <button class="action-btn" on:click={() => clearReminder(entry)} title="Remove reminder">
                        <Icon icon="ph:bell-slash-duotone" />
                      </button>
                    </div>
                  {:else if entry.entry_type === 'task'}
                    <div class="checkbox-spacer" aria-hidden="true"></div>
                    <div class="reminder-content">
                      <span class="reminder-type-badge">Task</span>
                      <span class="reminder-title">{entry.title || entry.content || '(no title)'}</span>
                      <div class="reminder-meta">
                        <span class="reminder-time" class:has-date={entry.reminder_at}>
                          <Icon icon="ph:clock-duotone" />
                          {formatDateTime(entry.reminder_at)}
                        </span>
                      </div>
                    </div>
                <div class="reminder-actions" on:click|stopPropagation on:keydown|stopPropagation>
                  <button class="action-btn" on:click={() => snooze(entry)} title="Snooze 1 hour">
                    <Icon icon="ph:clock-countdown-duotone" />
                  </button>
                  <button class="action-btn" on:click={() => clearReminder(entry)} title="Remove reminder">
                    <Icon icon="ph:bell-slash-duotone" />
                  </button>
                  <button class="action-btn delete" on:click={() => remove(entry.id)} title="Delete task">
                    <Icon icon="ph:trash-duotone" />
                  </button>
                </div>
              {/if}
                </div>
              {/each}
            </div>
          </div>
        {/if}
      {/each}

      {#if completedReminders.length > 0}
        <div class="completed-section">
          <div class="section-header">
            <h3>Completed</h3>
            <span class="count">{completedReminders.length}</span>
          </div>
          <div class="reminder-list completed">
            {#each completedReminders as entry (entry.id)}
              <div class="reminder-item is-completed" class:is-task={entry.entry_type === 'task'} role="button" tabindex="0" on:click={() => handleItemClick(entry)} on:keydown={(e) => e.key === 'Enter' && handleItemClick(entry)}>
                <button class="checkbox" on:click|stopPropagation={() => toggleComplete(entry)}>
                  <div class="check-circle checked">
                    <Icon icon="ph:check-bold" class="check-icon" />
                  </div>
                </button>
                <div class="reminder-content">
                  {#if entry.entry_type === 'task'}
                    <span class="reminder-type-badge">Task</span>
                  {/if}
                  <span class="reminder-title">{entry.entry_type === 'task' ? (entry.title || entry.content || '(no title)') : entry.content}</span>
                </div>
                <div class="reminder-actions" on:click|stopPropagation on:keydown|stopPropagation>
                  <button class="action-btn delete" on:click={() => remove(entry.id)} title={entry.entry_type === 'task' ? 'Delete task' : 'Delete reminder'}>
                    <Icon icon="ph:trash-duotone" />
                  </button>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}

  <SidePanel 
    isOpen={isPanelOpen} 
    title={panelMode === 'add' ? 'Add Reminder' : 'Edit Reminder'} 
    on:close={closePanel}
  >
    <div slot="body" class="panel-form">
      <div class="field">
        <label for="edit-content">Content</label>
        <input id="edit-content" type="text" class="edit-input" bind:value={draftContent} placeholder="Remind me to..." />
      </div>
      <div class="field">
        <label for="edit-datetime">Date & time</label>
        <div class="datetime-quick-actions">
          <button type="button" class="quick-btn" on:click={() => setQuickDate('today')}>Today</button>
          <button type="button" class="quick-btn" on:click={() => setQuickDate('tomorrow')}>Tomorrow</button>
          <button type="button" class="quick-btn" on:click={() => setQuickDate('next_week')}>Next Week</button>
        </div>
        <input id="edit-datetime" type="datetime-local" class="edit-input" bind:value={draftReminderAt} />
      </div>
      <div class="field">
        <label for="edit-rrule">Repeat</label>
        <select id="edit-rrule" class="edit-select" bind:value={draftRrulePreset} on:change={handlePresetChange}>
          {#each draftPresets as preset}
            <option value={preset.value}>{preset.label}</option>
          {/each}
        </select>
      </div>
    </div>
    <div slot="footer">
      <button class="btn-primary" on:click={savePanel} disabled={!draftContent.trim()}>
        <Icon icon="ph:check-bold" /> Save
      </button>
      <button class="btn-ghost" on:click={closePanel}>Cancel</button>
    </div>
  </SidePanel>

  {#if showCustomModal}
    <div class="edit-modal-backdrop" role="button" tabindex="0" aria-label="Close modal" on:click={cancelCustomModal} on:keydown={(e) => e.key === 'Escape' && cancelCustomModal()}>
      <div class="edit-modal custom-rrule-modal" role="dialog" aria-labelledby="custom-rrule-title" tabindex="-1" on:click|stopPropagation on:keydown|stopPropagation>
        <h3 id="custom-rrule-title" class="edit-modal-title">Custom recurrence</h3>
        <div class="edit-modal-body">
          <div class="form-row">
            <label class="edit-label" for="custom-interval">Repeat every</label>
            <div class="interval-group">
              <input id="custom-interval" type="number" min="1" class="edit-input num-input" bind:value={customInterval} />
              <select class="edit-select" bind:value={customFreq}>
                <option value="DAILY">day{customInterval > 1 ? 's' : ''}</option>
                <option value="WEEKLY">week{customInterval > 1 ? 's' : ''}</option>
                <option value="MONTHLY">month{customInterval > 1 ? 's' : ''}</option>
                <option value="YEARLY">year{customInterval > 1 ? 's' : ''}</option>
              </select>
            </div>
          </div>

          {#if customFreq === 'WEEKLY'}
            <div class="form-row">
              <label class="edit-label" for="custom-repeat-days">Repeat on</label>
              <div id="custom-repeat-days" class="days-group" role="group">
                {#each DAYS_OF_WEEK as day}
                  <button 
                    type="button" 
                    class="day-btn" 
                    class:selected={customDays.includes(day.value)}
                    on:click={() => {
                      if (customDays.includes(day.value)) {
                        customDays = customDays.filter(d => d !== day.value)
                      } else {
                        customDays = [...customDays, day.value]
                      }
                    }}
                  >
                    {day.label}
                  </button>
                {/each}
              </div>
            </div>
          {/if}

          <div class="form-row">
            <label class="edit-label" for="custom-ends">Ends</label>
            <div id="custom-ends" class="ends-group" role="group">
              <label class="radio-label">
                <input type="radio" name="endType" value="never" bind:group={customEndType} />
                Never
              </label>
              <label class="radio-label">
                <input type="radio" name="endType" value="until" bind:group={customEndType} />
                On
                <input type="date" class="edit-input inline-input" bind:value={customUntil} disabled={customEndType !== 'until'} />
              </label>
              <label class="radio-label">
                <input type="radio" name="endType" value="count" bind:group={customEndType} />
                After
                <input type="number" min="1" class="edit-input inline-input num-input" bind:value={customCount} disabled={customEndType !== 'count'} />
                occurrences
              </label>
            </div>
          </div>
        </div>
        <div class="edit-modal-actions">
          <button type="button" class="btn-ghost" on:click={cancelCustomModal}>Cancel</button>
          <button type="button" class="btn-primary" on:click={saveCustomModal}>
            Done
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .view {
    max-width: 800px;
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

  /* List */
  .reminder-sections {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  .reminder-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .reminder-item {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    padding: 20px;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    transition: all 0.2s ease;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.01);
    cursor: pointer;
  }

  .reminder-item:hover {
    border-color: var(--border-visible);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.04);
    transform: translateY(-2px);
  }

  .checkbox,
  .checkbox-spacer {
    flex-shrink: 0;
    width: 24px;
    height: 24px;
  }

  .checkbox-spacer {
    margin-top: 2px;
  }

  .checkbox {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .reminder-type-badge {
    display: inline-block;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    margin-bottom: 2px;
  }

  .check-circle {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 2px solid var(--border-visible);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
    color: transparent;
  }

  .checkbox:hover .check-circle {
    border-color: var(--primary);
    background: var(--primary-alpha);
  }

  .check-circle.checked {
    background: var(--primary);
    border-color: var(--primary);
    color: white;
  }

  .check-icon {
    font-size: 14px;
  }

  .reminder-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .reminder-title {
    font-size: 16px;
    color: var(--navy-deep);
    font-weight: 600;
    line-height: 1.4;
  }

  .reminder-meta {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }

  .reminder-time, .reminder-repeat {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    background: var(--bg-input);
    padding: 4px 8px;
    border-radius: 6px;
  }

  .reminder-time.has-date {
    color: var(--primary-dark);
    background: var(--primary-alpha-light);
    border: 1px solid var(--primary-alpha);
  }

  .reminder-actions {
    opacity: 0;
    transition: opacity 0.2s;
  }

  .reminder-item:hover .reminder-actions {
    opacity: 1;
  }

  .action-btn {
    width: 32px;
    height: 32px;
    border-radius: 8px;
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

  /* Completed Section */
  .completed-section {
    opacity: 0.8;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
    padding-left: 16px;
  }

  .section-header h3 {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0;
  }

  .count {
    background: var(--bg-input);
    color: var(--text-secondary);
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 700;
  }

  .reminder-item.is-completed {
    background: transparent;
    border-color: transparent;
    box-shadow: none;
  }

  .reminder-item.is-completed:hover {
    transform: none;
    background: var(--bg-card);
    border-color: var(--border-subtle);
  }

  .reminder-item.is-completed .reminder-title {
    text-decoration: line-through;
    color: var(--text-muted);
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
    gap: 20px;
  }

  .field label {
    display: block;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }

  .edit-input,
  .edit-select {
    width: 100%;
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-input);
    color: var(--text-primary);
    font-size: 14px;
    font-family: inherit;
    transition: all 0.2s;
  }

  .edit-input[type="datetime-local"]::-webkit-calendar-picker-indicator {
    cursor: pointer;
    opacity: 0.6;
    transition: 0.2s;
  }

  .edit-input[type="datetime-local"]::-webkit-calendar-picker-indicator:hover {
    opacity: 1;
  }

  .edit-select {
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 256 256'%3E%3Cpath fill='%2364748B' d='M213.66 101.66l-80 80a8 8 0 0 1-11.32 0l-80-80a8 8 0 0 1 11.32-11.32L128 164.69l74.34-74.35a8 8 0 0 1 11.32 11.32z'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 12px center;
    padding-right: 36px;
    cursor: pointer;
  }

  .edit-input:focus, .edit-select:focus {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 3px var(--primary-alpha);
  }

  .datetime-quick-actions {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
  }

  .quick-btn {
    flex: 1;
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    background: var(--bg-input);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .quick-btn:hover {
    background: var(--primary-alpha-light);
    color: var(--primary-dark);
    border-color: var(--primary-alpha);
  }

  /* Custom RRule Modal */
  .edit-modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(7, 16, 41, 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 110; /* above side panel */
    padding: 20px;
  }

  .edit-modal {
    background: var(--bg-card);
    border-radius: 20px;
    box-shadow: 0 24px 48px rgba(7, 16, 41, 0.2), 0 0 0 1px rgba(7, 16, 41, 0.05);
    max-width: 420px;
    width: 100%;
    overflow: hidden;
  }

  .edit-modal-title {
    margin: 0;
    padding: 20px 20px 12px;
    font-size: 18px;
    font-weight: 700;
    color: var(--navy-deep);
  }

  .edit-modal-body {
    padding: 12px 20px 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .edit-label {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .edit-modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 20px 20px;
    border-top: 1px solid var(--border-subtle);
  }

  .form-row {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 8px;
  }

  .interval-group {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .num-input {
    width: 70px;
  }

  .days-group {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .day-btn {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: 1px solid var(--border-subtle);
    background: var(--bg-input);
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
  }

  .day-btn:hover {
    background: var(--bg-hover);
  }

  .day-btn.selected {
    background: var(--primary);
    border-color: var(--primary);
    color: white;
  }

  .ends-group {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    color: var(--text-primary);
    cursor: pointer;
  }

  .inline-input {
    padding: 6px 10px;
    margin-left: 8px;
  }

  @media (max-width: 768px) {
    .reminder-actions {
      opacity: 1;
    }
    .subtitle {
      padding-left: 0;
    }
  }
</style>