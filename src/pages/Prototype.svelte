<script lang="ts">
  import { onMount } from 'svelte'
  import { flip } from 'svelte/animate'
  import { fade } from 'svelte/transition'
  import StatusBar from '../components/StatusBar.svelte'
  import Icon from '@iconify/svelte'
  import SveltyPicker from 'svelty-picker'
  import { getKalamSveltyPickerLocaleOptions } from '$lib/sveltyPickerLocale'
  import type { KalamSveltyPickerLocaleOptions } from '$lib/sveltyPickerLocale'
  import { modifierSortIndex, superKeyLabel } from '../lib/platformHotkey'
  import type { AppConfig } from '../types'

  export let currentPage: string = 'home'
  export let navigate: (page: string) => void = () => {}
  export let dictationEnabled: boolean = true
  export let setDictation: (next: boolean) => void = () => {}
  export let statusBarConfig: AppConfig | null = null
  export let dbStatus: { ok: boolean } | null = null
  export let statusBarPlatform: string = ''
  export let lastLatencyMs: number | null = null
  /** Status bar: refresh DB status (parent owns `dbStatus` state). */
  export let onRetryDb: () => void | Promise<void> = () => {}

  let sdtLocale: KalamSveltyPickerLocaleOptions = getKalamSveltyPickerLocaleOptions()

  let darkMode = true
  let isLoading = true
  let greeting = 'Welcome'
  
  // --- Settings state ---
  let settingsActiveTab = 'general'
  let settingsCollapsed: Record<string, boolean> = {
    general_hotkeys: false,
    general_startup: true,
    general_appearance: false,
    dictation_audio: false,
    dictation_mode: false,
    dictation_formatting: true,
    privacy_data: false,
    advanced_logs: false,
    advanced_danger: false,
    license: true
  }

  // Hotkey capture state
  let capturingHotkey: string | null = null
  let tempHotkeyValue: string = ''

  function startHotkeyCapture(hotkeyId: string) {
    capturingHotkey = hotkeyId
    tempHotkeyValue = ''
  }

  function stopHotkeyCapture() {
    capturingHotkey = null
    tempHotkeyValue = ''
  }

  function confirmHotkeyCapture(hotkeyId: string, value: string) {
    if (hotkeyId === 'hotkey') settingsConfig.hotkey = value
    if (hotkeyId === 'toggle') settingsConfig.toggle_dictation_hotkey = value
    if (hotkeyId === 'language') settingsConfig.language_toggle_hotkey = value
    if (hotkeyId === 'command') settingsConfig.command_hotkey = value
    stopHotkeyCapture()
  }

  function clearHotkey(hotkeyId: string) {
    if (hotkeyId === 'hotkey') settingsConfig.hotkey = ''
    if (hotkeyId === 'toggle') settingsConfig.toggle_dictation_hotkey = ''
    if (hotkeyId === 'language') settingsConfig.language_toggle_hotkey = ''
    if (hotkeyId === 'command') settingsConfig.command_hotkey = ''
  }

  function handleHotkeyKeydown(event: KeyboardEvent, hotkeyId: string) {
    if (!capturingHotkey) return
    event.preventDefault()
    event.stopPropagation()

    const metaLabel = superKeyLabel(statusBarPlatform || 'windows')
    const keyMap: Record<string, string> = {
      'Control': 'Ctrl', 'ControlLeft': 'Ctrl', 'ControlRight': 'Ctrl',
      'Shift': 'Shift', 'ShiftLeft': 'Shift', 'ShiftRight': 'Shift',
      'Alt': 'Alt', 'AltLeft': 'Alt', 'AltRight': 'Alt',
      'Meta': metaLabel, 'MetaLeft': metaLabel, 'MetaRight': metaLabel, 'OS': metaLabel,
      'Escape': 'Esc', 'Enter': 'Enter', 'Space': 'Space',
      'ArrowUp': '↑', 'ArrowDown': '↓', 'ArrowLeft': '←', 'ArrowRight': '→'
    }

    const key = keyMap[event.code] || keyMap[event.key] || event.key

    if (!tempHotkeyValue.includes(key)) {
      const keys = tempHotkeyValue ? tempHotkeyValue.split('+') : []
      keys.push(key)
      const sorted = keys.sort((a, b) => {
        const ai = modifierSortIndex(a)
        const bi = modifierSortIndex(b)
        if (ai !== bi) return ai - bi
        return a.localeCompare(b)
      })
      tempHotkeyValue = sorted.join('+')
    }

    // Auto-confirm on regular key press (non-modifier)
    if (modifierSortIndex(key) >= 4 && key !== 'Esc' && key !== 'Enter' && key !== 'Tab') {
      setTimeout(() => confirmHotkeyCapture(hotkeyId, tempHotkeyValue), 100)
    }
  }

  function handleHotkeyKeyup(event: KeyboardEvent, hotkeyId: string) {
    if (!capturingHotkey) return
    event.preventDefault()
    event.stopPropagation()

    // When all keys released, confirm if we have a value
    if (tempHotkeyValue) {
      setTimeout(() => confirmHotkeyCapture(hotkeyId, tempHotkeyValue), 50)
    }
  }
  
  // Mock settings config
  let settingsConfig = {
    hotkey: 'Ctrl+Space',
    toggle_dictation_hotkey: 'Ctrl+Shift+D',
    language_toggle_hotkey: 'Ctrl+Shift+L',
    auto_start: false,
    start_in_focus: true,
    dictation_enabled: true,
    recording_mode: 'Hold' as 'Hold' | 'Toggle' | 'Both',
    min_hold_ms: 300,
    stt_mode: 'Cloud' as 'Cloud' | 'Local' | 'Hybrid',
    stt_provider: 'groq' as 'groq' | 'openai',
    local_model: 'sensevoice' as 'sensevoice' | 'whisper_base',
    audio_device: '',
    auto_punctuation: true,
    voice_commands: true,
    filler_word_removal: true,
    injection_method: 'Auto' as 'Auto' | 'Keystrokes' | 'Clipboard',
    history_retention_days: 90,
    telemetry_enabled: false,
    sidebar_collapsed: false,
    waveform_style: 'Aurora' as
      | 'SiriWave'
      | 'EchoRing'
      | 'RoundedBars'
      | 'BreathingAura'
      | 'Oscilloscope'
      | 'NeonPulse'
      | 'Aurora',
    overlay_position: 'BottomCenter' as 'BottomCenter' | 'BottomLeft' | 'BottomRight' | 'TopCenter' | 'Center',
    overlay_offset_x: 0,
    overlay_offset_y: 0,
    command_mode_enabled: false,
    command_hotkey: 'Ctrl+Shift+C',
    languages: ['en'],
    api_key_input: '',
    api_key_valid: null as boolean | null,
    has_api_key: true
  }
  
  // Local model mock status
  let localModelStatus = {
    sensevoice: { installed: true, status: 'Stopped' as 'Stopped' | 'Running' | 'Starting', size_mb: 830, label: 'SenseVoice Small', quality: 'Good', languages: 'Multilingual' },
    whisper_base: { installed: false, status: 'Stopped' as 'Stopped' | 'Running' | 'Starting', size_mb: 75, label: 'Whisper Base', quality: 'Moderate', languages: 'English' }
  }
  
  const settingsTabs = [
    { id: 'general', label: 'General', icon: 'ph:sliders-horizontal' },
    { id: 'dictation', label: 'Audio & Dictation', icon: 'ph:microphone' },
    { id: 'privacy', label: 'Privacy', icon: 'ph:shield' },
    { id: 'advanced', label: 'Advanced', icon: 'ph:terminal' },
    { id: 'about', label: 'About', icon: 'ph:info' }
  ]
  
  function toggleSettingsSection(section: string) {
    settingsCollapsed[section] = !settingsCollapsed[section]
    settingsCollapsed = { ...settingsCollapsed }
  }
  
  function setSettingsTab(tab: string) {
    settingsActiveTab = tab
  }

  const navItems = [
    { id: 'home', label: 'Overview', icon: 'ph:squares-four' },
    { id: 'history', label: 'History', icon: 'ph:clock' },
    { id: 'notes', label: 'Notes', icon: 'ph:notebook' },
    { id: 'tasks', label: 'Tasks', icon: 'ph:check-circle' },
    { id: 'reminders', label: 'Reminders', icon: 'ph:bell' },
    { id: 'snippets', label: 'Snippets', icon: 'ph:text-aa' },
  ]

  const mockHistory = [
    { id: '1', text: 'Send the quarterly report to the team by Friday and schedule a follow-up meeting to discuss results.', created_at: new Date(Date.now() - 1000 * 60 * 15).toISOString(), mode: 'dictation', stt_mode: 'Cloud', language: 'en', duration_ms: 4200 },
    { id: '2', text: 'Reminder: call the dentist tomorrow at 9 AM to confirm the appointment.', created_at: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString(), mode: 'dictation', stt_mode: 'Local', language: 'en', duration_ms: 3100 },
    { id: '3', text: 'Project timeline updated. Design review on the 20th, launch end of month.', created_at: new Date(Date.now() - 1000 * 60 * 60 * 5).toISOString(), mode: 'command', stt_mode: 'Hybrid', language: 'en', duration_ms: 2100 },
    { id: '4', text: 'Team lunch ideas: Thai place on Main, Italian bistro, or pizza in the park.', created_at: new Date(Date.now() - 1000 * 60 * 60 * 26).toISOString(), mode: 'dictation', stt_mode: 'Cloud', language: 'en', duration_ms: 3500 },
    { id: '5', text: 'Refactor authentication module to use token-based system. Add unit tests.', created_at: new Date(Date.now() - 1000 * 60 * 60 * 48).toISOString(), mode: 'dictation', stt_mode: 'Hybrid', language: 'en', duration_ms: 3600 },
  ]

  /** Note shape aligned with app Entry: supports color, reminder, labels, pin, scope (archived/trash). */
  let mockNotes: Array<{
    id: string
    title: string
    content: string
    pinned: boolean
    color: string
    reminder_at: string | null
    tags: string[]
    updated_at: string
    archived_at: string | null
    deleted_at: string | null
  }> = [
    { id: 'n1', title: 'Product sync outcomes', content: 'Beta moved to March 15. Mobile prioritized. Draft FAQ needed.', pinned: true, color: '', reminder_at: null, tags: ['work', 'product'], updated_at: new Date().toISOString(), archived_at: null, deleted_at: null },
    { id: 'n2', title: 'Blog post ideas', content: 'Why voice saves time. Tips for accuracy. Shortcut library guide.', pinned: false, color: '#fef9c3', reminder_at: null, tags: ['ideas'], updated_at: new Date(Date.now() - 86400000).toISOString(), archived_at: null, deleted_at: null },
    { id: 'n3', title: 'Shopping list', content: 'Oat milk, sourdough, avocados, coffee beans, batteries', pinned: false, color: '#ccfbf1', reminder_at: new Date(Date.now() + 86400000).toISOString().slice(0, 16), tags: [], updated_at: new Date(Date.now() - 3600000).toISOString(), archived_at: null, deleted_at: null },
    { id: 'n4', title: 'Archived idea', content: 'Old brainstorm — might revisit later.', pinned: false, color: '', reminder_at: null, tags: ['archive'], updated_at: new Date(Date.now() - 86400000 * 2).toISOString(), archived_at: new Date(Date.now() - 86400000).toISOString(), deleted_at: null },
  ]

  const NOTE_COLORS = [
    { name: 'default', value: '' },
    { name: 'yellow', value: '#fef08a' },
    { name: 'orange', value: '#fed7aa' },
    { name: 'red', value: '#fecaca' },
    { name: 'pink', value: '#fbcfe8' },
    { name: 'purple', value: '#e9d5ff' },
    { name: 'blue', value: '#bfdbfe' },
    { name: 'cyan', value: '#a5f3fc' },
    { name: 'gray', value: '#e2e8f0' },
  ]

  let notesScope: 'active' | 'archived' | 'trash' = 'active'
  let notesSearchQuery = ''
  let notesSelectedLabel: string | null = null
  /** Order of note ids for drag-and-drop; initialized from mockNotes. */
  let notesOrder: string[] = []
  let selectedNoteId: string | null = null

  let mockTasks = [
    { 
      id: 't1', 
      title: 'Review PR #347', 
      content: 'Make sure to check the new authentication flow and run the test suite.',
      due_date: new Date().toISOString(), 
      priority: 3, 
      is_completed: false,
      subtasks: [
        { title: 'Check auth flow', is_completed: true },
        { title: 'Run test suite', is_completed: false }
      ],
      tags: ['work', 'review'],
      color: '',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    },
    { 
      id: 't2', 
      title: 'Update API docs', 
      content: 'Add the new endpoints for user management.',
      due_date: new Date(Date.now() + 86400000).toISOString(), 
      priority: 2, 
      is_completed: false,
      subtasks: [],
      tags: ['docs'],
      color: '#dbeafe',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    },
    { 
      id: 't3', 
      title: 'Draft release notes', 
      content: 'Include the new features and bug fixes for v1.2.',
      due_date: new Date(Date.now() + 86400000 * 3).toISOString(), 
      priority: 2, 
      is_completed: false,
      subtasks: [],
      tags: ['product'],
      color: '',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    },
    { 
      id: 't4', 
      title: 'Schedule interviews', 
      content: '',
      due_date: new Date(Date.now() - 86400000).toISOString(), 
      priority: 1, 
      is_completed: true,
      subtasks: [],
      tags: ['hr'],
      color: '',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    },
  ]

  let mockReminders = [
    { 
      id: 'r1', 
      title: 'Daily standup', 
      reminder_at: new Date().toISOString().slice(0, 16), 
      rrule: 'FREQ=DAILY;BYHOUR=9;BYMINUTE=30',
      type: 'recurring',
      tags: ['work'],
      created_at: new Date().toISOString()
    },
    { 
      id: 'r2', 
      title: 'Submit expense report', 
      reminder_at: new Date(Date.now() + 86400000).toISOString().slice(0, 16), 
      rrule: '',
      type: 'once',
      tags: ['finance'],
      created_at: new Date().toISOString()
    },
    { 
      id: 'r3', 
      title: 'Call mom — birthday', 
      reminder_at: new Date(Date.now() + 86400000 * 2).toISOString().slice(0, 16), 
      rrule: 'FREQ=YEARLY;BYMONTH=3;BYMONTHDAY=16',
      type: 'birthday',
      tags: ['personal'],
      created_at: new Date().toISOString()
    },
  ]

  let mockSnippets = [
    { id: 's1', trigger: 'sig', expansion: 'Best regards,\nJohn Doe', uses: 156, tags: ['email'] },
    { id: 's2', trigger: 'addr', expansion: '123 Innovation Drive, San Francisco', uses: 23, tags: ['personal'] },
    { id: 's3', trigger: 'omw', expansion: 'On my way! Be there in ~10 min', uses: 89, tags: ['quick'] },
  ]

  $: stats = { streak: 5, hoursSaved: 2.3, words: 1247, todayCount: 3 }
  $: activeTasks = mockTasks.filter(t => !t.is_completed).length

  $: groupedHistory = mockHistory.reduce((acc, entry) => {
    const date = new Date(entry.created_at)
    const today = new Date()
    const yesterday = new Date(today)
    yesterday.setDate(yesterday.getDate() - 1)
    let dateLabel = '', dateSub = ''
    if (date.toDateString() === today.toDateString()) { dateLabel = 'Today'; dateSub = date.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric' }) }
    else if (date.toDateString() === yesterday.toDateString()) { dateLabel = 'Yesterday'; dateSub = date.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric' }) }
    else { dateLabel = date.toLocaleDateString(undefined, { month: 'long', day: 'numeric' }); dateSub = date.toLocaleDateString(undefined, { weekday: 'long', year: 'numeric' }) }
    if (!acc[dateLabel]) acc[dateLabel] = { entries: [], sub: dateSub }
    acc[dateLabel].entries.push(entry)
    return acc
  }, {} as Record<string, { entries: typeof mockHistory, sub: string }>)

  onMount(() => {
    const hour = new Date().getHours()
    darkMode = hour < 6 || hour >= 18
    if (hour < 12) greeting = 'Good morning'
    else if (hour < 17) greeting = 'Good afternoon'
    else greeting = 'Good evening'
    setTimeout(() => isLoading = false, 300)
    const onPriorityDoc = (e: MouseEvent) => {
      if (!priorityPopoverEl) return
      if (priorityPopoverEl.contains(e.target as Node)) return
      priorityMenuOpen = false
    }
    document.addEventListener('click', onPriorityDoc)
    const onPriorityKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') priorityMenuOpen = false
    }
    window.addEventListener('keydown', onPriorityKey)
    return () => {
      document.removeEventListener('click', onPriorityDoc)
      window.removeEventListener('keydown', onPriorityKey)
    }
  })

  function formatTime(iso: string) {
    return new Date(iso).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })
  }
  function toggleTheme() { darkMode = !darkMode }
  function copyToClipboard(text: string) { navigator.clipboard.writeText(text) }
  function getPriorityColor(p: number) { return ['#34C759', '#FF9500', '#FF3B30'][p - 1] || '#8E8E93' } // Apple colors

  // --- Notes: init order, labels, filtered list ---
  $: if (notesOrder.length === 0 && mockNotes.length > 0) {
    notesOrder = mockNotes.filter(n => !n.archived_at && !n.deleted_at).map(n => n.id)
  }
  $: allLabels = [...new Set(mockNotes.flatMap(n => n.tags || []))].sort()
  $: notesFilteredByScope = mockNotes.filter(n => {
    if (notesScope === 'active') return !n.archived_at && !n.deleted_at
    if (notesScope === 'archived') return !!n.archived_at && !n.deleted_at
    return !!n.deleted_at
  })
  $: notesFilteredBySearch = notesSearchQuery.trim()
    ? notesFilteredByScope.filter(n => {
        const q = notesSearchQuery.trim().toLowerCase()
        return (n.title || '').toLowerCase().includes(q) || (n.content || '').toLowerCase().includes(q)
      })
    : notesFilteredByScope
  $: notesFilteredByLabel = notesSelectedLabel
    ? notesFilteredBySearch.filter(n => (n.tags || []).includes(notesSelectedLabel!))
    : notesFilteredBySearch
  $: notesDisplayOrder = notesScope === 'active' && notesOrder.length > 0
    ? notesOrder.map(id => notesFilteredByLabel.find(n => n.id === id)).filter(Boolean) as typeof notesFilteredByLabel
    : notesFilteredByLabel
  
  $: pinnedNotes = notesDisplayOrder.filter(n => n.pinned)
  $: otherNotes = notesDisplayOrder.filter(n => !n.pinned)

  function openNote(id: string) {
    selectedNoteId = id
    navigate('note-detail')
  }
  function openNewNote() {
    selectedNoteId = null
    navigate('note-detail')
  }
  function backToNotes() {
    navigate('notes')
    selectedNoteId = null
  }
  function formatNoteDate(iso: string) {
    const d = new Date(iso)
    const today = new Date()
    if (d.toDateString() === today.toDateString()) return `Today, ${d.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })}`
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  }
  function formatReminder(iso: string | null) {
    if (!iso) return ''
    const d = new Date(iso)
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' })
  }
  
  function formatReminderShort(iso: string | null) {
    if (!iso) return ''
    const d = new Date(iso)
    const today = new Date()
    const isToday = d.toDateString() === today.toDateString()
    
    if (isToday) {
      return d.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })
    }
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  }
  function addDraftTag(tag: string) {
    const t = tag.trim()
    if (t && !noteDraft.tags.includes(t)) noteDraft.tags = [...noteDraft.tags, t]
    newLabelInput = ''
  }
  function removeDraftTag(tag: string) {
    noteDraft.tags = noteDraft.tags.filter(x => x !== tag)
  }

  /** Draft for note detail page (create or edit). */
  let noteDraft = { title: '', content: '', color: '', reminder_at: '', pinned: false, tags: [] as string[], updated_at: '' }
  let newLabelInput = ''
  
  /** UI state for popovers */
  let showColorPicker = false
  let showReminderInput = false
  let showLabelInput = false
  
  /** Click outside to close popovers */
  function handleClickOutside(event: MouseEvent) {
    const target = event.target as HTMLElement
    if (!target.closest('.color-dropdown-container')) {
      showColorPicker = false
    }
    if (!target.closest('.reminder-dropdown-container')) {
      // Don't auto-close reminder input - let user finish
    }
    if (!target.closest('.sleek-labels') && !target.closest('.sleek-label-input')) {
      showLabelInput = false
    }
  }
  $: if (currentPage === 'note-detail') {
    if (selectedNoteId) {
      const note = mockNotes.find(n => n.id === selectedNoteId)
      if (note) noteDraft = { title: note.title, content: note.content, color: note.color || '', reminder_at: note.reminder_at ? note.reminder_at.slice(0, 16) : '', pinned: note.pinned, tags: [...(note.tags || [])], updated_at: note.updated_at || '' }
    } else {
      noteDraft = { title: '', content: '', color: '', reminder_at: '', pinned: false, tags: [], updated_at: '' }
    }
  }

  function saveNote() {
    const title = noteDraft.title.trim()
    const content = noteDraft.content.trim()
    if (!title && !content) return
    const now = new Date().toISOString()
    const reminderAt = noteDraft.reminder_at.trim() ? new Date(noteDraft.reminder_at).toISOString() : null
    if (selectedNoteId) {
      const idx = mockNotes.findIndex(n => n.id === selectedNoteId)
      if (idx >= 0) mockNotes = mockNotes.map((n, i) => i === idx ? { ...n, title: title || n.title, content: content || n.content, color: noteDraft.color || '', reminder_at: reminderAt, pinned: noteDraft.pinned, tags: [...noteDraft.tags], updated_at: now } : n)
    } else {
      const newId = 'n' + Date.now()
      mockNotes = [...mockNotes, { id: newId, title, content, pinned: noteDraft.pinned, color: noteDraft.color || '', reminder_at: reminderAt, tags: [...noteDraft.tags], updated_at: now, archived_at: null, deleted_at: null }]
      notesOrder = [...notesOrder, newId]
    }
    backToNotes()
  }
  function deleteNote() {
    if (!selectedNoteId) return
    mockNotes = mockNotes.map(n => n.id === selectedNoteId ? { ...n, deleted_at: new Date().toISOString(), updated_at: new Date().toISOString() } : n)
    backToNotes()
  }
  function moveNoteToTrash(note: typeof mockNotes[0]) {
    mockNotes = mockNotes.map(n => n.id === note.id ? { ...n, deleted_at: new Date().toISOString(), updated_at: new Date().toISOString() } : n)
    notesOrder = notesOrder.filter(id => id !== note.id)
  }
  function moveNoteToTrashFromArchived(note: typeof mockNotes[0]) {
    mockNotes = mockNotes.map(n => n.id === note.id ? { ...n, deleted_at: new Date().toISOString(), archived_at: null, updated_at: new Date().toISOString() } : n)
  }
  function archiveNote(note: typeof mockNotes[0]) {
    mockNotes = mockNotes.map(n => n.id === note.id ? { ...n, archived_at: new Date().toISOString(), updated_at: new Date().toISOString() } : n)
    notesOrder = notesOrder.filter(id => id !== note.id)
  }
  function unarchiveNote(note: typeof mockNotes[0]) {
    mockNotes = mockNotes.map(n => n.id === note.id ? { ...n, archived_at: null, updated_at: new Date().toISOString() } : n)
    notesOrder = [...notesOrder, note.id]
  }
  function restoreNote(note: typeof mockNotes[0]) {
    mockNotes = mockNotes.map(n => n.id === note.id ? { ...n, deleted_at: null, updated_at: new Date().toISOString() } : n)
    notesOrder = [...notesOrder, note.id]
  }
  function permanentlyDeleteNote(note: typeof mockNotes[0]) {
    mockNotes = mockNotes.filter(n => n.id !== note.id)
    notesOrder = notesOrder.filter(id => id !== note.id)
  }

  /** Drag-and-drop reorder (active scope only). */
  let dragNoteId: string | null = null
  function handleNoteDragStart(e: DragEvent, id: string) {
    dragNoteId = id
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'
  }
  function handleNoteDragOver(e: DragEvent) {
    e.preventDefault()
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
  }
  function handleNoteDrop(e: DragEvent, targetId: string) {
    e.preventDefault()
    if (!dragNoteId || dragNoteId === targetId) { dragNoteId = null; return }
    const fromIdx = notesOrder.indexOf(dragNoteId)
    const toIdx = notesOrder.indexOf(targetId)
    if (fromIdx === -1 || toIdx === -1) { dragNoteId = null; return }
    const next = [...notesOrder]
    next.splice(fromIdx, 1)
    next.splice(toIdx, 0, dragNoteId)
    notesOrder = next
    dragNoteId = null
  }
  function handleNoteDragEnd() {
    dragNoteId = null
  }

  // --- Tasks: state and functions ---
  let selectedTaskId: string | null = null
  let taskDraft = { title: '', content: '', due_date: '', priority: 0, is_completed: false, subtasks: [] as {title: string, is_completed: boolean}[], tags: [] as string[] }
  let newTaskLabelInput = ''
  let newSubtaskInput = ''
  let priorityMenuOpen = false
  let priorityPopoverEl: HTMLDivElement | null = null

  const prototypePriorityChoices: { value: number; label: string; short: string }[] = [
    { value: 0, label: 'No priority', short: 'None' },
    { value: 1, label: 'Low priority', short: 'Low' },
    { value: 2, label: 'Medium priority', short: 'Med' },
    { value: 3, label: 'High priority', short: 'High' }
  ]
  function prototypePriorityShort(p: number) {
    return prototypePriorityChoices.find((c) => c.value === p)?.short ?? 'None'
  }
  function pickPrototypePriority(v: number) {
    taskDraft.priority = v
    priorityMenuOpen = false
  }

  $: if (currentPage === 'task-detail') {
    if (selectedTaskId) {
      const task = mockTasks.find(t => t.id === selectedTaskId)
      if (task) taskDraft = { title: task.title, content: task.content || '', due_date: task.due_date ? task.due_date.slice(0, 16) : '', priority: task.priority || 0, is_completed: task.is_completed, subtasks: [...(task.subtasks || [])], tags: [...(task.tags || [])] }
    } else {
      taskDraft = { title: '', content: '', due_date: '', priority: 0, is_completed: false, subtasks: [], tags: [] }
    }
  }

  function openTask(id: string) {
    selectedTaskId = id
    navigate('task-detail')
  }
  function openNewTask() {
    selectedTaskId = null
    navigate('task-detail')
  }
  function backToTasks() {
    navigate('tasks')
    selectedTaskId = null
  }
  function saveTask() {
    const title = taskDraft.title.trim()
    if (!title) return
    const now = new Date().toISOString()
    const dueDate = taskDraft.due_date.trim() ? new Date(taskDraft.due_date).toISOString() : null
    if (selectedTaskId) {
      const idx = mockTasks.findIndex(t => t.id === selectedTaskId)
      if (idx >= 0) mockTasks = mockTasks.map((t, i) => i === idx ? { ...t, title, content: taskDraft.content, due_date: dueDate, priority: taskDraft.priority, is_completed: taskDraft.is_completed, subtasks: [...taskDraft.subtasks], tags: [...taskDraft.tags], updated_at: now } : t)
    } else {
      const newId = 't' + Date.now()
      mockTasks = [...mockTasks, { id: newId, title, content: taskDraft.content, due_date: dueDate, priority: taskDraft.priority, is_completed: taskDraft.is_completed, subtasks: [...taskDraft.subtasks], tags: [...taskDraft.tags], color: '', created_at: now, updated_at: now }]
    }
    backToTasks()
  }
  function deleteTask() {
    if (!selectedTaskId) return
    mockTasks = mockTasks.filter(t => t.id !== selectedTaskId)
    backToTasks()
  }
  function toggleTaskCompletion(id: string) {
    mockTasks = mockTasks.map(t => t.id === id ? { ...t, is_completed: !t.is_completed, updated_at: new Date().toISOString() } : t)
  }
  function addTaskDraftTag(tag: string) {
    const t = tag.trim()
    if (t && !taskDraft.tags.includes(t)) taskDraft.tags = [...taskDraft.tags, t]
    newTaskLabelInput = ''
  }
  function removeTaskDraftTag(tag: string) {
    taskDraft.tags = taskDraft.tags.filter(x => x !== tag)
  }
  function addDraftSubtask(title: string) {
    const t = title.trim()
    if (t) taskDraft.subtasks = [...taskDraft.subtasks, { title: t, is_completed: false }]
    newSubtaskInput = ''
  }
  function removeDraftSubtask(index: number) {
    taskDraft.subtasks = taskDraft.subtasks.filter((_, i) => i !== index)
  }
  function toggleDraftSubtask(index: number) {
    taskDraft.subtasks = taskDraft.subtasks.map((st, i) => i === index ? { ...st, is_completed: !st.is_completed } : st)
  }

  // --- Subtask drag-and-drop ---
  let dragSubtaskIndex: number | null = null
  function handleSubtaskDragStart(e: DragEvent, index: number) {
    dragSubtaskIndex = index
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'
  }
  function handleSubtaskDragOver(e: DragEvent) {
    e.preventDefault()
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
  }
  function handleSubtaskDrop(e: DragEvent, targetIndex: number) {
    e.preventDefault()
    if (dragSubtaskIndex === null || dragSubtaskIndex === targetIndex) { dragSubtaskIndex = null; return }
    const next = [...taskDraft.subtasks]
    const [moved] = next.splice(dragSubtaskIndex, 1)
    next.splice(targetIndex, 0, moved)
    taskDraft.subtasks = next
    dragSubtaskIndex = null
  }
  function handleSubtaskDragEnd() {
    dragSubtaskIndex = null
  }

  // --- Tasks list: search and order ---
  let tasksSearchQuery = ''
  let tasksOrder: string[] = []
  $: if (tasksOrder.length === 0 && mockTasks.length > 0) {
    tasksOrder = mockTasks.map(t => t.id)
  }
  $: tasksFiltered = tasksSearchQuery.trim()
    ? mockTasks.filter(t => t.title.toLowerCase().includes(tasksSearchQuery.trim().toLowerCase()))
    : mockTasks
  $: tasksDisplayOrder = tasksOrder
    .map(id => tasksFiltered.find(t => t.id === id))
    .filter(Boolean) as typeof mockTasks

  // --- Task drag-and-drop ---
  let dragTaskId: string | null = null
  function handleTaskDragStart(e: DragEvent, id: string) {
    dragTaskId = id
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'
  }
  function handleTaskDragOver(e: DragEvent) {
    e.preventDefault()
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
  }
  function handleTaskDrop(e: DragEvent, targetId: string) {
    e.preventDefault()
    if (!dragTaskId || dragTaskId === targetId) { dragTaskId = null; return }
    const fromIdx = tasksOrder.indexOf(dragTaskId)
    const toIdx = tasksOrder.indexOf(targetId)
    if (fromIdx === -1 || toIdx === -1) { dragTaskId = null; return }
    const next = [...tasksOrder]
    next.splice(fromIdx, 1)
    next.splice(toIdx, 0, dragTaskId)
    tasksOrder = next
    dragTaskId = null
  }
  function handleTaskDragEnd() {
    dragTaskId = null
  }

  // --- Reminders: state and functions ---
  let selectedReminderId: string | null = null
  let reminderDraft = { title: '', reminder_at: '', rrule: '', tags: [] as string[] }
  let newReminderTagInput = ''
  let remindersSearchQuery = ''

  function sveltyDetailToString(e: CustomEvent<string | null | undefined>): string {
    const v = e.detail
    return v == null || v === '' ? '' : String(v)
  }
  function onPrototypeNoteReminderChange(e: CustomEvent<string | null | undefined>) {
    noteDraft.reminder_at = sveltyDetailToString(e)
    showReminderInput = false
  }
  function onPrototypeTaskDueChange(e: CustomEvent<string | null | undefined>) {
    taskDraft.due_date = sveltyDetailToString(e)
  }
  function onPrototypeReminderDraftChange(e: CustomEvent<string | null | undefined>) {
    reminderDraft.reminder_at = sveltyDetailToString(e)
  }

  $: if (currentPage === 'reminder-detail') {
    if (selectedReminderId) {
      const reminder = mockReminders.find(r => r.id === selectedReminderId)
      if (reminder) reminderDraft = { title: reminder.title, reminder_at: reminder.reminder_at ? reminder.reminder_at.slice(0, 16) : '', rrule: reminder.rrule || '', tags: [...(reminder.tags || [])] }
    } else {
      reminderDraft = { title: '', reminder_at: '', rrule: '', tags: [] }
    }
  }

  // --- Combined Reminders: standalone + note reminders + task reminders ---
  type CombinedReminder = {
    id: string
    title: string
    reminder_at: string | null
    rrule: string
    type: 'standalone' | 'note' | 'task'
    sourceId: string
    tags: string[]
    sourceItem?: any
  }
  
  $: allReminders = [
    // Standalone reminders
    ...mockReminders.map(r => ({
      id: `standalone-${r.id}`,
      title: r.title,
      reminder_at: r.reminder_at,
      rrule: r.rrule || '',
      type: 'standalone' as const,
      sourceId: r.id,
      tags: r.tags || [],
      sourceItem: r
    })),
    // Note reminders
    ...mockNotes
      .filter(n => n.reminder_at && !n.archived_at && !n.deleted_at)
      .map(n => ({
        id: `note-${n.id}`,
        title: n.title || 'Untitled Note',
        reminder_at: n.reminder_at,
        rrule: '',
        type: 'note' as const,
        sourceId: n.id,
        tags: n.tags || [],
        sourceItem: n
      })),
    // Task reminders (due dates)
    ...mockTasks
      .filter(t => t.due_date && !t.is_completed)
      .map(t => ({
        id: `task-${t.id}`,
        title: t.title,
        reminder_at: t.due_date,
        rrule: '',
        type: 'task' as const,
        sourceId: t.id,
        tags: t.tags || [],
        sourceItem: t
      }))
  ].sort((a, b) => {
    // Sort by reminder_at date (nulls last)
    if (!a.reminder_at && !b.reminder_at) return 0
    if (!a.reminder_at) return 1
    if (!b.reminder_at) return -1
    return new Date(a.reminder_at).getTime() - new Date(b.reminder_at).getTime()
  })
  
  $: remindersFiltered = remindersSearchQuery.trim()
    ? allReminders.filter(r => r.title.toLowerCase().includes(remindersSearchQuery.trim().toLowerCase()))
    : allReminders

  function openReminder(id: string) {
    selectedReminderId = id
    navigate('reminder-detail')
  }
  function openNewReminder() {
    selectedReminderId = null
    navigate('reminder-detail')
  }
  function backToReminders() {
    navigate('reminders')
    selectedReminderId = null
  }
  function saveReminder() {
    const title = reminderDraft.title.trim()
    if (!title) return
    const now = new Date().toISOString()
    if (selectedReminderId) {
      const idx = mockReminders.findIndex(r => r.id === selectedReminderId)
      if (idx >= 0) mockReminders = mockReminders.map((r, i) => i === idx ? { ...r, title, reminder_at: reminderDraft.reminder_at, rrule: reminderDraft.rrule, tags: [...reminderDraft.tags], created_at: r.created_at } : r)
    } else {
      const newId = 'r' + Date.now()
      mockReminders = [...mockReminders, { id: newId, title, reminder_at: reminderDraft.reminder_at, rrule: reminderDraft.rrule, type: 'once', tags: [...reminderDraft.tags], created_at: now }]
    }
    backToReminders()
  }
  function deleteReminder() {
    if (!selectedReminderId) return
    mockReminders = mockReminders.filter(r => r.id !== selectedReminderId)
    backToReminders()
  }
  function addReminderDraftTag(tag: string) {
    const t = tag.trim()
    if (t && !reminderDraft.tags.includes(t)) reminderDraft.tags = [...reminderDraft.tags, t]
    newReminderTagInput = ''
  }
  function removeReminderDraftTag(tag: string) {
    reminderDraft.tags = reminderDraft.tags.filter(x => x !== tag)
  }

  // --- Snippets: state and functions ---
  let selectedSnippetId: string | null = null
  let snippetDraft = { trigger: '', expansion: '', tags: [] as string[] }
  let newSnippetTagInput = ''
  let snippetsSearchQuery = ''

  $: if (currentPage === 'snippet-detail') {
    if (selectedSnippetId) {
      const snippet = mockSnippets.find(s => s.id === selectedSnippetId)
      if (snippet) snippetDraft = { trigger: snippet.trigger, expansion: snippet.expansion, tags: [...(snippet.tags || [])] }
    } else {
      snippetDraft = { trigger: '', expansion: '', tags: [] }
    }
  }

  $: snippetsFiltered = snippetsSearchQuery.trim()
    ? mockSnippets.filter(s => s.trigger.toLowerCase().includes(snippetsSearchQuery.trim().toLowerCase()) || s.expansion.toLowerCase().includes(snippetsSearchQuery.trim().toLowerCase()))
    : mockSnippets

  function openSnippet(id: string) {
    selectedSnippetId = id
    navigate('snippet-detail')
  }
  function openNewSnippet() {
    selectedSnippetId = null
    navigate('snippet-detail')
  }
  function backToSnippets() {
    navigate('snippets')
    selectedSnippetId = null
  }
  function saveSnippet() {
    const trigger = snippetDraft.trigger.trim()
    const expansion = snippetDraft.expansion.trim()
    if (!trigger || !expansion) return
    if (selectedSnippetId) {
      const idx = mockSnippets.findIndex(s => s.id === selectedSnippetId)
      if (idx >= 0) mockSnippets = mockSnippets.map((s, i) => i === idx ? { ...s, trigger, expansion, tags: [...snippetDraft.tags] } : s)
    } else {
      const newId = 's' + Date.now()
      mockSnippets = [...mockSnippets, { id: newId, trigger, expansion, uses: 0, tags: [...snippetDraft.tags] }]
    }
    backToSnippets()
  }
  function deleteSnippet() {
    if (!selectedSnippetId) return
    mockSnippets = mockSnippets.filter(s => s.id !== selectedSnippetId)
    backToSnippets()
  }
  function addSnippetDraftTag(tag: string) {
    const t = tag.trim()
    if (t && !snippetDraft.tags.includes(t)) snippetDraft.tags = [...snippetDraft.tags, t]
    newSnippetTagInput = ''
  }
  function removeSnippetDraftTag(tag: string) {
    snippetDraft.tags = snippetDraft.tags.filter(x => x !== tag)
  }
</script>

<svelte:head>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600&display=swap" rel="stylesheet">
</svelte:head>

<div class="kalam-sleek" class:dark={darkMode} class:light={!darkMode}>
  
  <!-- Sidebar -->
  <aside class="sidebar">
    <div class="sidebar-content">
      <!-- Logo -->
      <div class="logo-section">
        <div class="logo">
          <svg viewBox="0 0 40 40" class="logo-svg">
            <circle cx="20" cy="20" r="14" fill="currentColor"/>
            <circle cx="20" cy="20" r="4" fill="var(--bg-elevated)"/>
          </svg>
          <span class="logo-text">Kalam</span>
        </div>
      </div>

      <!-- Navigation -->
      <nav class="main-nav">
        {#each navItems as item}
          <button class="nav-link" class:active={currentPage === item.id || (item.id === 'notes' && currentPage === 'note-detail') || (item.id === 'tasks' && currentPage === 'task-detail') || (item.id === 'reminders' && currentPage === 'reminder-detail') || (item.id === 'snippets' && currentPage === 'snippet-detail')} on:click={() => navigate(item.id)}>
            <Icon icon={(currentPage === item.id || (item.id === 'notes' && currentPage === 'note-detail') || (item.id === 'tasks' && currentPage === 'task-detail') || (item.id === 'reminders' && currentPage === 'reminder-detail') || (item.id === 'snippets' && currentPage === 'snippet-detail')) ? item.icon + '-fill' : item.icon} />
            <span>{item.label}</span>
          </button>
        {/each}
      </nav>

      <!-- Bottom Section -->
      <div class="sidebar-bottom">
        <!-- Dictation Toggle -->
        <button class="dictation-btn" class:active={dictationEnabled} on:click={() => setDictation(!dictationEnabled)}>
          <Icon icon={dictationEnabled ? 'ph:microphone-fill' : 'ph:microphone'} />
          <span>{dictationEnabled ? 'Listening...' : 'Start Dictation'}</span>
          {#if dictationEnabled}
            <div class="pulse-dot"></div>
          {/if}
        </button>

        <div class="bottom-links">
          <button class="icon-btn" on:click={toggleTheme} title="Toggle Theme">
            <Icon icon={darkMode ? 'ph:sun' : 'ph:moon'} />
          </button>
          <button class="icon-btn" class:active={currentPage === 'settings'} on:click={() => navigate('settings')} title="Settings">
            <Icon icon="ph:gear" />
          </button>
        </div>
      </div>
    </div>
  </aside>

  <!-- Main Content -->
  <main class="main">
    {#if isLoading}
      <div class="loading">
        <div class="spinner"></div>
      </div>
    {:else}
      <div class="page-content">
        
        <!-- HOME -->
        {#if currentPage === 'home'}
          <div class="page fade-in">
            <header class="page-header">
              <h1 class="page-title">{greeting}</h1>
              <p class="page-subtitle">Here's what's happening today.</p>
            </header>

            <!-- Stats Row -->
            <div class="stats-row">
              <div class="stat-box">
                <span class="stat-label">Words Dictated</span>
                <span class="stat-num">{stats.words.toLocaleString()}</span>
              </div>
              <div class="stat-box">
                <span class="stat-label">Hours Saved</span>
                <span class="stat-num">{stats.hoursSaved}</span>
              </div>
              <div class="stat-box">
                <span class="stat-label">Day Streak</span>
                <span class="stat-num">{stats.streak}</span>
              </div>
            </div>

            <!-- Dashboard Grid -->
            <div class="dashboard-grid">
              <!-- Recent History -->
              <section class="dash-section wide">
                <div class="section-header">
                  <h3>Recent</h3>
                  <button class="text-btn" on:click={() => navigate('history')}>See all</button>
                </div>
                <div class="history-list">
                  {#each mockHistory.slice(0, 3) as entry}
                    <div class="list-item">
                      <div class="item-icon">
                        <Icon icon={entry.mode === 'command' ? 'ph:terminal-window' : 'ph:quotes'} />
                      </div>
                      <div class="item-content">
                        <p class="item-text">{entry.text}</p>
                        <div class="item-meta-row">
                          <span class="chip chip-mode small" class:dictation={entry.mode === 'dictation'} class:command={entry.mode === 'command'}>{entry.mode}</span>
                          <span class="chip chip-stt small" class:cloud={entry.stt_mode === 'Cloud'} class:local={entry.stt_mode === 'Local'} class:hybrid={entry.stt_mode === 'Hybrid'}>{entry.stt_mode}</span>
                          <span class="item-meta">{formatTime(entry.created_at)}</span>
                        </div>
                      </div>
                    </div>
                  {/each}
                </div>
              </section>

              <div class="dash-columns">
                <!-- Tasks -->
                <section class="dash-section">
                  <div class="section-header">
                    <h3>Tasks</h3>
                  </div>
                  <div class="simple-list">
                    {#each mockTasks.filter(t => !t.is_completed).slice(0, 3) as task}
                      <div class="simple-item">
                        <div class="priority-dot" style="background: {getPriorityColor(task.priority)}"></div>
                        <span class="simple-text">{task.title}</span>
                      </div>
                    {/each}
                  </div>
                </section>

                <!-- Reminders -->
                <section class="dash-section">
                  <div class="section-header">
                    <h3>Reminders</h3>
                  </div>
                  <div class="simple-list">
                    {#each mockReminders.slice(0, 3) as r}
                      <div class="simple-item">
                        <Icon icon="ph:clock" class="muted-icon" />
                        <span class="simple-text">{r.text}</span>
                      </div>
                    {/each}
                  </div>
                </section>
              </div>
            </div>
          </div>
        {/if}

        <!-- HISTORY -->
        {#if currentPage === 'history'}
          <div class="page fade-in">
            <header class="page-header">
              <h1 class="page-title">History</h1>
            </header>

            <div class="search-bar">
              <span class="search-bar-icon" aria-hidden="true">
                <Icon icon="ph:magnifying-glass" />
              </span>
              <input type="text" placeholder="Search your dictations..." />
            </div>

            <div class="timeline">
              {#each Object.entries(groupedHistory) as [dayLabel, dayData]}
                <div class="day-group">
                  <h3 class="day-label">{dayLabel} <span class="day-sub">{dayData.sub}</span></h3>
                  
                  <div class="entries">
                    {#each dayData.entries as entry}
                      <div class="entry-row">
                        <div class="entry-time">{formatTime(entry.created_at)}</div>
                        <div class="entry-content">
                          <p class="entry-text">{entry.text}</p>
                          <div class="entry-actions">
                            <span class="chip chip-mode" class:dictation={entry.mode === 'dictation'} class:command={entry.mode === 'command'}>{entry.mode}</span>
                            <span class="chip chip-stt" class:cloud={entry.stt_mode === 'Cloud'} class:local={entry.stt_mode === 'Local'} class:hybrid={entry.stt_mode === 'Hybrid'}>{entry.stt_mode}</span>
                            <span class="entry-duration">{Math.round(entry.duration_ms / 1000)}s</span>
                            <button class="icon-btn small" on:click={() => copyToClipboard(entry.text)} title="Copy">
                              <Icon icon="ph:copy" />
                            </button>
                          </div>
                        </div>
                      </div>
                    {/each}
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- NOTES LIST -->
        {#if currentPage === 'notes'}
          <div class="page fade-in">
            <header class="page-header notes-header">
              <div>
                <h1 class="page-title">Notes</h1>
                <p class="page-subtitle">Jot down your thoughts, ideas, and transcriptions.</p>
              </div>
              <button class="btn-primary" on:click={openNewNote}>
                <Icon icon="ph:plus" />
                New Note
              </button>
            </header>

            <div class="notes-subnav">
              <button type="button" class="subnav-btn" class:active={notesScope === 'active'} on:click={() => notesScope = 'active'}>Notes</button>
              <button type="button" class="subnav-btn" class:active={notesScope === 'archived'} on:click={() => notesScope = 'archived'}>Archive</button>
              <button type="button" class="subnav-btn" class:active={notesScope === 'trash'} on:click={() => notesScope = 'trash'}>Trash</button>
            </div>

            <div class="notes-search-bar">
              <Icon icon="ph:magnifying-glass" />
              <input type="text" bind:value={notesSearchQuery} placeholder="Search notes..." />
            </div>
            {#if notesScope === 'active' && allLabels.length > 0}
              <div class="notes-label-filters">
                <button type="button" class="label-chip" class:active={notesSelectedLabel === null} on:click={() => notesSelectedLabel = null}>All</button>
                {#each allLabels as label}
                  <button type="button" class="label-chip" class:active={notesSelectedLabel === label} on:click={() => notesSelectedLabel = notesSelectedLabel === label ? null : label}>{label}</button>
                {/each}
              </div>
            {/if}

            {#if notesDisplayOrder.length === 0}
              <div class="notes-empty">
                <Icon icon="ph:notebook" />
                <p>{notesScope === 'trash' ? 'Trash is empty' : notesScope === 'archived' ? 'No archived notes' : (notesSearchQuery || notesSelectedLabel) ? 'No results' : 'No notes yet'}</p>
              </div>
            {:else}
              <div class="notes-lists-container">
                {#if pinnedNotes.length > 0}
                  <div class="notes-section">
                    <h3 class="notes-section-title">Pinned</h3>
                    <div class="notes-masonry">
                      {#each pinnedNotes as note (note.id)}
                        <article
                          class="note-card"
                          class:pinned={note.pinned}
                          class:dragging={dragNoteId === note.id}
                          class:has-custom-color={!!note.color}
                          style:background-color={note.color || 'var(--bg-elevated)'}
                          animate:flip={{ duration: 250 }}
                          role="button"
                          tabindex="0"
                          draggable={notesScope === 'active'}
                          on:click={() => openNote(note.id)}
                          on:keydown={(e) => e.key === 'Enter' && openNote(note.id)}
                          on:dragstart={(e) => handleNoteDragStart(e, note.id)}
                          on:dragover={(e) => handleNoteDragOver(e)}
                          on:drop={(e) => handleNoteDrop(e, note.id)}
                          on:dragend={handleNoteDragEnd}
                        >
                          <div class="note-inner">
                            {#if note.pinned}
                              <div class="pin-icon"><Icon icon="ph:push-pin-fill" /></div>
                            {/if}
                            {#if note.title}
                              <h4 class="note-title">{note.title}</h4>
                            {/if}
                            <p class="note-content-preview">{note.content || '(empty)'}</p>
                            {#if (note.tags?.length ?? 0) > 0}
                              <div class="note-tags-row">
                                {#each note.tags as tag}
                                  <span class="note-tag">{tag}</span>
                                {/each}
                              </div>
                            {/if}
                            {#if note.reminder_at}
                              <div class="note-reminder-row">
                                <Icon icon="ph:bell" />
                                {formatReminder(note.reminder_at)}
                              </div>
                            {/if}
                          </div>
                          <div class="note-footer" role="group" on:click|stopPropagation on:keydown|stopPropagation>
                            <span class="note-date">{formatNoteDate(note.updated_at)}</span>
                            <div class="note-actions">
                              {#if notesScope === 'active'}
                                <button type="button" class="note-action-btn" on:click|stopPropagation={() => archiveNote(note)} title="Archive"><Icon icon="ph:archive" /></button>
                                <button type="button" class="note-action-btn delete" on:click|stopPropagation={() => moveNoteToTrash(note)} title="Delete"><Icon icon="ph:trash" /></button>
                              {:else if notesScope === 'archived'}
                                <button type="button" class="note-action-btn" on:click|stopPropagation={() => unarchiveNote(note)} title="Unarchive"><Icon icon="ph:archive-tray" /></button>
                                <button type="button" class="note-action-btn delete" on:click|stopPropagation={() => moveNoteToTrashFromArchived(note)} title="Delete"><Icon icon="ph:trash" /></button>
                              {:else}
                                <button type="button" class="note-action-btn" on:click|stopPropagation={() => restoreNote(note)} title="Restore"><Icon icon="ph:arrow-counter-clockwise" /></button>
                                <button type="button" class="note-action-btn delete" on:click|stopPropagation={() => permanentlyDeleteNote(note)} title="Delete permanently"><Icon icon="ph:trash" /></button>
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
                      {#each otherNotes as note (note.id)}
                        <article
                          class="note-card"
                          class:pinned={note.pinned}
                          class:dragging={dragNoteId === note.id}
                          class:has-custom-color={!!note.color}
                          style:background-color={note.color || 'var(--bg-elevated)'}
                          animate:flip={{ duration: 250 }}
                          role="button"
                          tabindex="0"
                          draggable={notesScope === 'active'}
                          on:click={() => openNote(note.id)}
                          on:keydown={(e) => e.key === 'Enter' && openNote(note.id)}
                          on:dragstart={(e) => handleNoteDragStart(e, note.id)}
                          on:dragover={(e) => handleNoteDragOver(e)}
                          on:drop={(e) => handleNoteDrop(e, note.id)}
                          on:dragend={handleNoteDragEnd}
                        >
                          <div class="note-inner">
                            {#if note.pinned}
                              <div class="pin-icon"><Icon icon="ph:push-pin-fill" /></div>
                            {/if}
                            {#if note.title}
                              <h4 class="note-title">{note.title}</h4>
                            {/if}
                            <p class="note-content-preview">{note.content || '(empty)'}</p>
                            {#if (note.tags?.length ?? 0) > 0}
                              <div class="note-tags-row">
                                {#each note.tags as tag}
                                  <span class="note-tag">{tag}</span>
                                {/each}
                              </div>
                            {/if}
                            {#if note.reminder_at}
                              <div class="note-reminder-row">
                                <Icon icon="ph:bell" />
                                {formatReminder(note.reminder_at)}
                              </div>
                            {/if}
                          </div>
                          <div class="note-footer" role="group" on:click|stopPropagation on:keydown|stopPropagation>
                            <span class="note-date">{formatNoteDate(note.updated_at)}</span>
                            <div class="note-actions">
                              {#if notesScope === 'active'}
                                <button type="button" class="note-action-btn" on:click|stopPropagation={() => archiveNote(note)} title="Archive"><Icon icon="ph:archive" /></button>
                                <button type="button" class="note-action-btn delete" on:click|stopPropagation={() => moveNoteToTrash(note)} title="Delete"><Icon icon="ph:trash" /></button>
                              {:else if notesScope === 'archived'}
                                <button type="button" class="note-action-btn" on:click|stopPropagation={() => unarchiveNote(note)} title="Unarchive"><Icon icon="ph:archive-tray" /></button>
                                <button type="button" class="note-action-btn delete" on:click|stopPropagation={() => moveNoteToTrashFromArchived(note)} title="Delete"><Icon icon="ph:trash" /></button>
                              {:else}
                                <button type="button" class="note-action-btn" on:click|stopPropagation={() => restoreNote(note)} title="Restore"><Icon icon="ph:arrow-counter-clockwise" /></button>
                                <button type="button" class="note-action-btn delete" on:click|stopPropagation={() => permanentlyDeleteNote(note)} title="Delete permanently"><Icon icon="ph:trash" /></button>
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
        {/if}

        <!-- NOTE DETAIL (full-page edit) -->
        {#if currentPage === 'note-detail'}
          <div class="page fade-in sleek-editor-page" on:click={handleClickOutside}>
            <header class="sleek-header">
              <button type="button" class="sleek-back" on:click={backToNotes}>
                <Icon icon="ph:caret-left" /> Notes
              </button>
              <div class="sleek-actions">
                {#if selectedNoteId}
                  <button type="button" class="sleek-icon-btn danger" on:click={deleteNote} title="Delete">
                    <Icon icon="ph:trash" />
                  </button>
                {/if}
                <button type="button" class="sleek-cancel" on:click={backToNotes}>Cancel</button>
                <button type="button" class="sleek-save" on:click={saveNote} disabled={!noteDraft.title?.trim() && !noteDraft.content?.trim()}>Save</button>
              </div>
            </header>

            <div class="sleek-body">
              <input type="text" class="sleek-title" bind:value={noteDraft.title} placeholder="Note Title" />
              
              <textarea class="sleek-content" bind:value={noteDraft.content} placeholder="Start typing..."></textarea>

              <div class="sleek-labels">
                <Icon icon="ph:tag" />
                {#if noteDraft.tags.length > 0}
                  {#each noteDraft.tags as tag}
                    <span class="sleek-label-chip">
                      {tag}
                      <button type="button" on:click={() => removeDraftTag(tag)}><Icon icon="ph:x" /></button>
                    </span>
                  {/each}
                {/if}
                <input 
                  type="text" 
                  class="sleek-label-input"
                  bind:value={newLabelInput}
                  placeholder={noteDraft.tags.length ? "Add another..." : "Add label..."}
                  on:keydown={(e) => e.key === 'Enter' && (addDraftTag(newLabelInput), e.preventDefault())}
                />
              </div>
            </div>

            <footer class="sleek-footer">
              <div class="sleek-meta">
                {noteDraft.updated_at ? `Edited ${formatNoteDate(noteDraft.updated_at)}` : 'New Note'}
              </div>
              <div class="sleek-tools">
                <div class="color-dropdown-container">
                  <button type="button" class="sleek-tool-btn color-toggle" on:click={() => { showColorPicker = !showColorPicker; showReminderInput = false; }} title="Change color">
                    <span class="current-color-indicator" style:background-color={noteDraft.color || (darkMode ? '#333' : '#e5e5e5')}></span>
                  </button>
                  {#if showColorPicker}
                    <div class="sleek-popover color-popover" transition:fade={{ duration: 150 }}>
                      <div class="sleek-colors-grid">
                        {#each NOTE_COLORS as c}
                          <button 
                            type="button" 
                            class="sleek-color-dot" 
                            class:selected={noteDraft.color === c.value}
                            style:background-color={c.value || (darkMode ? '#333' : '#e5e5e5')}
                            on:click={() => { noteDraft.color = c.value; showColorPicker = false; }}
                            title={c.name}
                          >
                          </button>
                        {/each}
                      </div>
                    </div>
                  {/if}
                </div>
                
                <div class="reminder-dropdown-container">
                  <button type="button" class="sleek-tool-btn" class:active={noteDraft.reminder_at} on:click={() => { showReminderInput = !showReminderInput; showColorPicker = false; }} title={noteDraft.reminder_at ? formatReminder(noteDraft.reminder_at) : 'Set reminder'}>
                    <Icon icon={noteDraft.reminder_at ? 'ph:bell-fill' : 'ph:bell'} />
                  </button>
                  {#if showReminderInput}
                    <div class="sleek-popover reminder-popover" transition:fade={{ duration: 150 }}>
                      <div class="kalam-sdt-datetime">
                        <SveltyPicker
                          mode="datetime"
                          format={sdtLocale.format}
                          displayFormat={sdtLocale.displayFormat}
                          displayFormatType={sdtLocale.displayFormatType}
                          i18n={sdtLocale.i18n}
                          weekStart={sdtLocale.weekStart}
                          value={noteDraft.reminder_at || null}
                          inputClasses="sleek-datetime-input"
                          on:change={onPrototypeNoteReminderChange}
                        />
                      </div>
                      {#if noteDraft.reminder_at}
                        <button type="button" class="sleek-clear-btn" on:click={() => { noteDraft.reminder_at = ''; showReminderInput = false; }}>
                          Clear Reminder
                        </button>
                      {/if}
                    </div>
                  {/if}
                </div>

                <button type="button" class="sleek-tool-btn" class:active={noteDraft.pinned} on:click={() => noteDraft.pinned = !noteDraft.pinned} title="Pin note">
                  <Icon icon={noteDraft.pinned ? 'ph:push-pin-fill' : 'ph:push-pin'} />
                </button>
              </div>
            </footer>
          </div>
        {/if}

        <!-- TASKS -->
        {#if currentPage === 'tasks'}
          <div class="page fade-in">
            <header class="page-header notes-header">
              <div>
                <h1 class="page-title">Tasks</h1>
                <p class="page-subtitle">Track what needs to get done.</p>
              </div>
              <button class="btn-primary" on:click={openNewTask}>
                <Icon icon="ph:plus" />
                New Task
              </button>
            </header>
            
            <div class="notes-search-bar">
              <Icon icon="ph:magnifying-glass" />
              <input type="text" placeholder="Search tasks..." bind:value={tasksSearchQuery} />
            </div>
            
            <div class="task-list-large">
              {#each tasksDisplayOrder as task (task.id)}
                <div 
                  class="task-row" 
                  class:completed={task.is_completed} 
                  class:dragging={dragTaskId === task.id}
                  draggable={true}
                  on:dragstart={(e) => handleTaskDragStart(e, task.id)}
                  on:dragover={handleTaskDragOver}
                  on:drop={(e) => handleTaskDrop(e, task.id)}
                  on:dragend={handleTaskDragEnd}
                  on:click={() => openTask(task.id)}
                >
                  <button class="drag-handle" title="Drag to reorder" on:click|stopPropagation>
                    <Icon icon="ph:dots-six-vertical" />
                  </button>
                  <button class="checkbox" on:click|stopPropagation={() => toggleTaskCompletion(task.id)}>
                    {#if task.is_completed}
                      <Icon icon="ph:check" />
                    {/if}
                  </button>
                  <div class="task-info">
                    <span class="task-title">{task.title}</span>
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
                          {task.subtasks.filter(s => s.is_completed).length}/{task.subtasks.length}
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
                  {#if task.priority}
                    <div class="priority-indicator" style="background: {getPriorityColor(task.priority)}"></div>
                  {/if}
                </div>
              {/each}
              {#if tasksDisplayOrder.length === 0}
                <div class="empty-state">
                  <Icon icon="ph:check-circle" />
                  <p>{tasksSearchQuery ? 'No tasks match your search' : 'All caught up!'}</p>
                </div>
              {/if}
            </div>
          </div>
        {/if}

        <!-- TASK DETAIL (full-page edit) -->
        {#if currentPage === 'task-detail'}
          <div class="page fade-in sleek-editor-page">
            <header class="sleek-header">
              <button type="button" class="sleek-back" on:click={backToTasks}>
                <Icon icon="ph:caret-left" /> Tasks
              </button>
              <div class="sleek-actions">
                {#if selectedTaskId}
                  <button type="button" class="sleek-icon-btn danger" on:click={deleteTask} title="Delete">
                    <Icon icon="ph:trash" />
                  </button>
                {/if}
                <button type="button" class="sleek-cancel" on:click={backToTasks}>Cancel</button>
                <button type="button" class="sleek-save" on:click={saveTask} disabled={!taskDraft.title?.trim()}>Save</button>
              </div>
            </header>

            <div class="sleek-body">
              <div class="task-title-row">
                <button 
                  type="button" 
                  class="sleek-tool-btn complete-toggle compact task-complete-btn" 
                  class:completed={taskDraft.is_completed}
                  on:click={() => taskDraft.is_completed = !taskDraft.is_completed}
                  title={taskDraft.is_completed ? 'Mark as incomplete' : 'Mark as complete'}
                >
                  <Icon icon={taskDraft.is_completed ? 'ph:check-circle-fill' : 'ph:circle-bold'} />
                </button>

                <input type="text" class="sleek-title" bind:value={taskDraft.title} placeholder="Task Title" autofocus />

                <div class="task-priority-popover" bind:this={priorityPopoverEl}>
                  <button
                    type="button"
                    class="task-priority-trigger"
                    aria-haspopup="listbox"
                    aria-expanded={priorityMenuOpen}
                    aria-label="Priority: {prototypePriorityShort(taskDraft.priority)}"
                    on:click|stopPropagation={() => (priorityMenuOpen = !priorityMenuOpen)}
                  >
                    <Icon
                      icon={taskDraft.priority > 0 ? 'ph:flag-fill' : 'ph:flag'}
                      class="task-priority-trigger-flag p{taskDraft.priority}"
                    />
                    <span class="task-priority-trigger-text">{prototypePriorityShort(taskDraft.priority)}</span>
                    <Icon icon="ph:caret-down" class="task-priority-trigger-caret" />
                  </button>
                  {#if priorityMenuOpen}
                    <ul class="task-priority-menu" role="listbox" aria-label="Choose priority">
                      {#each prototypePriorityChoices as choice}
                        <li role="none">
                          <button
                            type="button"
                            class="task-priority-option"
                            class:selected={choice.value === taskDraft.priority}
                            role="option"
                            aria-selected={choice.value === taskDraft.priority}
                            on:click|stopPropagation={() => pickPrototypePriority(choice.value)}
                          >
                            <Icon
                              icon={choice.value > 0 ? 'ph:flag-fill' : 'ph:flag'}
                              class="task-priority-option-flag p{choice.value}"
                            />
                            <span class="task-priority-option-label">{choice.label}</span>
                            {#if choice.value === taskDraft.priority}
                              <Icon icon="ph:check" class="task-priority-option-check" />
                            {/if}
                          </button>
                        </li>
                      {/each}
                    </ul>
                  {/if}
                </div>
              </div>
              
              <textarea class="sleek-content task-desc" bind:value={taskDraft.content} placeholder="Add description..."></textarea>

              <div class="due-date-section">
                <h3 class="section-title">Due Date</h3>
                <div class="due-date-input-row">
                  <Icon icon="ph:calendar-blank" />
                  <div class="kalam-sdt-datetime">
                    <SveltyPicker
                      mode="datetime"
                      format="yyyy-mm-ddThh:ii"
                      value={taskDraft.due_date || null}
                      inputClasses="sleek-datetime-input"
                      on:change={onPrototypeTaskDueChange}
                    />
                  </div>
                  {#if taskDraft.due_date}
                    <button type="button" class="sleek-clear-btn" on:click={() => taskDraft.due_date = ''}>
                      <Icon icon="ph:x" />
                    </button>
                  {/if}
                </div>
              </div>

              <div class="subtasks-section">
                <h3 class="section-title">Subtasks</h3>
                <div class="subtasks-list">
                  {#each taskDraft.subtasks as subtask, i (i)}
                    <div 
                      class="subtask-row" 
                      class:completed={subtask.is_completed}
                      draggable={true}
                      on:dragstart={(e) => handleSubtaskDragStart(e, i)}
                      on:dragover={handleSubtaskDragOver}
                      on:drop={(e) => handleSubtaskDrop(e, i)}
                      on:dragend={handleSubtaskDragEnd}
                    >
                      <button class="drag-handle" title="Drag to reorder">
                        <Icon icon="ph:dots-six-vertical" />
                      </button>
                      <button class="checkbox small" on:click={() => toggleDraftSubtask(i)}>
                        {#if subtask.is_completed}
                          <Icon icon="ph:check" />
                        {/if}
                      </button>
                      <input type="text" class="subtask-input" bind:value={subtask.title} />
                      <button class="remove-subtask" on:click={() => removeDraftSubtask(i)}>
                        <Icon icon="ph:x" />
                      </button>
                    </div>
                  {/each}
                  <div class="add-subtask-row">
                    <Icon icon="ph:plus" />
                    <input 
                      type="text" 
                      class="add-subtask-input" 
                      bind:value={newSubtaskInput} 
                      placeholder="Add subtask..."
                      on:keydown={(e) => e.key === 'Enter' && (addDraftSubtask(newSubtaskInput), e.preventDefault())}
                    />
                  </div>
                </div>
              </div>

              <div class="sleek-labels">
                <Icon icon="ph:tag" />
                {#if taskDraft.tags.length > 0}
                  {#each taskDraft.tags as tag}
                    <span class="sleek-label-chip">
                      {tag}
                      <button type="button" on:click={() => removeTaskDraftTag(tag)}><Icon icon="ph:x" /></button>
                    </span>
                  {/each}
                {/if}
                <input 
                  type="text" 
                  class="sleek-label-input"
                  bind:value={newTaskLabelInput}
                  placeholder={taskDraft.tags.length ? "Add another..." : "Add label..."}
                  on:keydown={(e) => e.key === 'Enter' && (addTaskDraftTag(newTaskLabelInput), e.preventDefault())}
                />
              </div>
            </div>
          </div>
        {/if}

        <!-- REMINDERS -->
        {#if currentPage === 'reminders'}
          <div class="page fade-in">
            <header class="page-header notes-header">
              <div>
                <h1 class="page-title">Reminders</h1>
                <p class="page-subtitle">Never forget the important things.</p>
              </div>
              <button class="btn-primary" on:click={openNewReminder}>
                <Icon icon="ph:plus" />
                New Reminder
              </button>
            </header>
            
            <div class="notes-search-bar">
              <Icon icon="ph:magnifying-glass" />
              <input type="text" placeholder="Search reminders..." bind:value={remindersSearchQuery} />
            </div>
            
            <div class="reminder-list-large">
              {#each remindersFiltered as r}
                <div 
                  class="reminder-row" 
                  class:from-note={r.type === 'note'}
                  class:from-task={r.type === 'task'}
                  on:click={() => {
                    if (r.type === 'standalone') openReminder(r.sourceId)
                    else if (r.type === 'note') { selectedNoteId = r.sourceId; navigate('note-detail') }
                    else if (r.type === 'task') { selectedTaskId = r.sourceId; navigate('task-detail') }
                  }}
                >
                  <div class="reminder-icon-large" class:recurring={r.rrule}>
                    {#if r.type === 'note'}
                      <Icon icon="ph:note" />
                    {:else if r.type === 'task'}
                      <Icon icon="ph:check-circle" />
                    {:else}
                      <Icon icon={r.rrule ? 'ph:arrows-clockwise' : 'ph:bell'} />
                    {/if}
                  </div>
                  <div class="reminder-info">
                    <div class="reminder-title-row">
                      <span class="reminder-text">{r.title}</span>
                      <span class="reminder-source-badge" class:note={r.type === 'note'} class:task={r.type === 'task'}>
                        {r.type === 'standalone' ? 'Reminder' : r.type === 'note' ? 'Note' : 'Task'}
                      </span>
                    </div>
                    <div class="reminder-meta">
                      {#if r.reminder_at}
                        <span class="reminder-time">
                          <Icon icon="ph:clock" />
                          {formatReminder(r.reminder_at)}
                        </span>
                      {/if}
                      {#if r.rrule}
                        <span class="reminder-recurring-badge">
                          <Icon icon="ph:repeat" />
                          Recurring
                        </span>
                      {/if}
                      {#if r.tags && r.tags.length > 0}
                        <div class="reminder-tags">
                          {#each r.tags as tag}
                            <span class="reminder-tag">{tag}</span>
                          {/each}
                        </div>
                      {/if}
                    </div>
                  </div>
                </div>
              {/each}
              {#if remindersFiltered.length === 0}
                <div class="empty-state">
                  <Icon icon="ph:bell-slash" />
                  <p>{remindersSearchQuery ? 'No reminders match your search' : 'No reminders yet'}</p>
                </div>
              {/if}
            </div>
          </div>
        {/if}

        <!-- REMINDER DETAIL (full-page edit) -->
        {#if currentPage === 'reminder-detail'}
          <div class="page fade-in sleek-editor-page">
            <header class="sleek-header">
              <button type="button" class="sleek-back" on:click={backToReminders}>
                <Icon icon="ph:caret-left" /> Reminders
              </button>
              <div class="sleek-actions">
                {#if selectedReminderId}
                  <button type="button" class="sleek-icon-btn danger" on:click={deleteReminder} title="Delete">
                    <Icon icon="ph:trash" />
                  </button>
                {/if}
                <button type="button" class="sleek-cancel" on:click={backToReminders}>Cancel</button>
                <button type="button" class="sleek-save" on:click={saveReminder} disabled={!reminderDraft.title?.trim()}>Save</button>
              </div>
            </header>

            <div class="sleek-body">
              <input type="text" class="sleek-title" bind:value={reminderDraft.title} placeholder="Reminder Title" autofocus />
              
              <div class="reminder-form-row">
                <label class="form-label" for="prototype-reminder-datetime">Date & Time</label>
                <div class="kalam-sdt-datetime">
                  <SveltyPicker
                    mode="datetime"
                    format={sdtLocale.format}
                    displayFormat={sdtLocale.displayFormat}
                    displayFormatType={sdtLocale.displayFormatType}
                    i18n={sdtLocale.i18n}
                    weekStart={sdtLocale.weekStart}
                    value={reminderDraft.reminder_at || null}
                    inputClasses="sleek-datetime-input full-width"
                    inputId="prototype-reminder-datetime"
                    on:change={onPrototypeReminderDraftChange}
                  />
                </div>
              </div>

              <div class="reminder-form-row">
                <label class="form-label">Repeat</label>
                <select class="form-select" bind:value={reminderDraft.rrule}>
                  <option value="">Don't repeat</option>
                  <option value="FREQ=DAILY">Every day</option>
                  <option value="FREQ=WEEKLY">Every week</option>
                  <option value="FREQ=MONTHLY">Every month</option>
                  <option value="FREQ=YEARLY">Every year</option>
                </select>
              </div>

              <div class="sleek-labels">
                <Icon icon="ph:tag" />
                {#if reminderDraft.tags.length > 0}
                  {#each reminderDraft.tags as tag}
                    <span class="sleek-label-chip">
                      {tag}
                      <button type="button" on:click={() => removeReminderDraftTag(tag)}><Icon icon="ph:x" /></button>
                    </span>
                  {/each}
                {/if}
                <input 
                  type="text" 
                  class="sleek-label-input"
                  bind:value={newReminderTagInput}
                  placeholder={reminderDraft.tags.length ? "Add another..." : "Add label..."}
                  on:keydown={(e) => e.key === 'Enter' && (addReminderDraftTag(newReminderTagInput), e.preventDefault())}
                />
              </div>
            </div>
          </div>
        {/if}

        <!-- SNIPPETS -->
        {#if currentPage === 'snippets'}
          <div class="page fade-in">
            <header class="page-header notes-header">
              <div>
                <h1 class="page-title">Snippets</h1>
                <p class="page-subtitle">Text shortcuts for quick expansion.</p>
              </div>
              <button class="btn-primary" on:click={openNewSnippet}>
                <Icon icon="ph:plus" />
                New Snippet
              </button>
            </header>
            
            <div class="notes-search-bar">
              <Icon icon="ph:magnifying-glass" />
              <input type="text" placeholder="Search snippets..." bind:value={snippetsSearchQuery} />
            </div>
            
            <div class="snippets-grid">
              {#each snippetsFiltered as s}
                <div class="snippet-card" on:click={() => openSnippet(s.id)}>
                  <div class="snippet-header">
                    <code class="trigger-code">/{s.trigger}</code>
                    <span class="uses-count">{s.uses} uses</span>
                  </div>
                  <p class="expansion-text">{s.expansion}</p>
                  {#if s.tags && s.tags.length > 0}
                    <div class="snippet-tags">
                      {#each s.tags as tag}
                        <span class="snippet-tag">{tag}</span>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/each}
              {#if snippetsFiltered.length === 0}
                <div class="empty-state">
                  <Icon icon="ph:textbox" />
                  <p>{snippetsSearchQuery ? 'No snippets match your search' : 'No snippets yet'}</p>
                </div>
              {/if}
            </div>
          </div>
        {/if}

        <!-- SNIPPET DETAIL (full-page edit) -->
        {#if currentPage === 'snippet-detail'}
          <div class="page fade-in sleek-editor-page">
            <header class="sleek-header">
              <button type="button" class="sleek-back" on:click={backToSnippets}>
                <Icon icon="ph:caret-left" /> Snippets
              </button>
              <div class="sleek-actions">
                {#if selectedSnippetId}
                  <button type="button" class="sleek-icon-btn danger" on:click={deleteSnippet} title="Delete">
                    <Icon icon="ph:trash" />
                  </button>
                {/if}
                <button type="button" class="sleek-cancel" on:click={backToSnippets}>Cancel</button>
                <button type="button" class="sleek-save" on:click={saveSnippet} disabled={!snippetDraft.trigger?.trim() || !snippetDraft.expansion?.trim()}>Save</button>
              </div>
            </header>

            <div class="sleek-body">
              <div class="snippet-form-row">
                <label class="form-label">Trigger (prefix with /)</label>
                <div class="trigger-input-wrapper">
                  <span class="trigger-prefix">/</span>
                  <input type="text" class="sleek-title trigger-input" bind:value={snippetDraft.trigger} placeholder="e.g., sig" autofocus />
                </div>
              </div>

              <div class="snippet-form-row">
                <label class="form-label">Expansion</label>
                <textarea class="sleek-content snippet-expansion" bind:value={snippetDraft.expansion} placeholder="Text that will be inserted when you type the trigger..."></textarea>
              </div>

              <div class="sleek-labels">
                <Icon icon="ph:tag" />
                {#if snippetDraft.tags.length > 0}
                  {#each snippetDraft.tags as tag}
                    <span class="sleek-label-chip">
                      {tag}
                      <button type="button" on:click={() => removeSnippetDraftTag(tag)}><Icon icon="ph:x" /></button>
                    </span>
                  {/each}
                {/if}
                <input 
                  type="text" 
                  class="sleek-label-input"
                  bind:value={newSnippetTagInput}
                  placeholder={snippetDraft.tags.length ? "Add another..." : "Add label..."}
                  on:keydown={(e) => e.key === 'Enter' && (addSnippetDraftTag(newSnippetTagInput), e.preventDefault())}
                />
              </div>
            </div>
          </div>
        {/if}

        <!-- SETTINGS -->
        {#if currentPage === 'settings'}
          <div class="page fade-in settings-page">
            <header class="page-header settings-header">
              <h1 class="page-title">Settings</h1>
            </header>
            
            <div class="settings-tabs">
              {#each settingsTabs as tab}
                <button 
                  class="settings-tab" 
                  class:active={settingsActiveTab === tab.id}
                  on:click={() => setSettingsTab(tab.id)}
                >
                  <Icon icon={tab.icon} />
                  <span>{tab.label}</span>
                </button>
              {/each}
            </div>
            
            <div class="settings-content">
              <!-- GENERAL TAB -->
              {#if settingsActiveTab === 'general'}
                <div class="settings-tab-content">
                  <!-- Hotkeys Section -->
                  <section class="settings-section" class:collapsed={settingsCollapsed.general_hotkeys}>
                    <button class="section-header" on:click={() => toggleSettingsSection('general_hotkeys')}>
                      <h3>Dictation Hotkeys</h3>
                      <Icon icon={settingsCollapsed.general_hotkeys ? 'ph:caret-down' : 'ph:caret-up'} />
                    </button>
                    {#if !settingsCollapsed.general_hotkeys}
                      <div class="section-content">
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Hold to Dictate</span>
                            <span class="setting-desc">Press and hold to start dictating</span>
                          </div>
                          <div class="setting-control">
                            <!-- svelte-ignore a11y-click-events-have-key-events -->
                            <!-- svelte-ignore a11y-no-static-element-interactions -->
                            <div
                              class="hotkey-capture-area"
                              class:capturing={capturingHotkey === 'hotkey'}
                              on:click={() => capturingHotkey === 'hotkey' ? stopHotkeyCapture() : startHotkeyCapture('hotkey')}
                              on:keydown={(e) => capturingHotkey === 'hotkey' && handleHotkeyKeydown(e, 'hotkey')}
                              on:keyup={(e) => capturingHotkey === 'hotkey' && handleHotkeyKeyup(e, 'hotkey')}
                              tabindex="0"
                            >
                              {#if capturingHotkey === 'hotkey'}
                                {#if tempHotkeyValue}
                                  <div class="hotkey-pills">
                                    {#each tempHotkeyValue.split('+') as key}
                                      <span class="hotkey-pill">{key}</span>
                                    {/each}
                                  </div>
                                {:else}
                                  <span class="hotkey-placeholder">Hold two or more keys at once, then release</span>
                                {/if}
                                <button class="hotkey-cancel" on:click|stopPropagation={stopHotkeyCapture}>
                                  <Icon icon="ph:x" />
                                </button>
                              {:else}
                                {#if settingsConfig.hotkey}
                                  <div class="hotkey-pills">
                                    {#each settingsConfig.hotkey.split('+') as key}
                                      <span class="hotkey-pill">{key}</span>
                                    {/each}
                                  </div>
                                  <button class="hotkey-clear" on:click|stopPropagation={() => clearHotkey('hotkey')}>
                                    <Icon icon="ph:x" />
                                  </button>
                                {:else}
                                  <span class="hotkey-placeholder">Click, then hold two or more keys</span>
                                  <Icon icon="ph:pencil-simple" class="hotkey-edit-icon" />
                                {/if}
                              {/if}
                            </div>
                          </div>
                        </div>
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Toggle Dictation</span>
                            <span class="setting-desc">Press to start/stop dictating</span>
                          </div>
                          <div class="setting-control">
                            <!-- svelte-ignore a11y-click-events-have-key-events -->
                            <!-- svelte-ignore a11y-no-static-element-interactions -->
                            <div
                              class="hotkey-capture-area"
                              class:capturing={capturingHotkey === 'toggle'}
                              on:click={() => capturingHotkey === 'toggle' ? stopHotkeyCapture() : startHotkeyCapture('toggle')}
                              on:keydown={(e) => capturingHotkey === 'toggle' && handleHotkeyKeydown(e, 'toggle')}
                              on:keyup={(e) => capturingHotkey === 'toggle' && handleHotkeyKeyup(e, 'toggle')}
                              tabindex="0"
                            >
                              {#if capturingHotkey === 'toggle'}
                                {#if tempHotkeyValue}
                                  <div class="hotkey-pills">
                                    {#each tempHotkeyValue.split('+') as key}
                                      <span class="hotkey-pill">{key}</span>
                                    {/each}
                                  </div>
                                {:else}
                                  <span class="hotkey-placeholder">Hold two or more keys at once, then release</span>
                                {/if}
                                <button class="hotkey-cancel" on:click|stopPropagation={stopHotkeyCapture}>
                                  <Icon icon="ph:x" />
                                </button>
                              {:else}
                                {#if settingsConfig.toggle_dictation_hotkey}
                                  <div class="hotkey-pills">
                                    {#each settingsConfig.toggle_dictation_hotkey.split('+') as key}
                                      <span class="hotkey-pill">{key}</span>
                                    {/each}
                                  </div>
                                  <button class="hotkey-clear" on:click|stopPropagation={() => clearHotkey('toggle')}>
                                    <Icon icon="ph:x" />
                                  </button>
                                {:else}
                                  <span class="hotkey-placeholder">Click, then hold two or more keys</span>
                                  <Icon icon="ph:pencil-simple" class="hotkey-edit-icon" />
                                {/if}
                              {/if}
                            </div>
                          </div>
                        </div>
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Toggle Language</span>
                            <span class="setting-desc">Switch between recognition languages</span>
                          </div>
                          <div class="setting-control">
                            <!-- svelte-ignore a11y-click-events-have-key-events -->
                            <!-- svelte-ignore a11y-no-static-element-interactions -->
                            <div
                              class="hotkey-capture-area"
                              class:capturing={capturingHotkey === 'language'}
                              on:click={() => capturingHotkey === 'language' ? stopHotkeyCapture() : startHotkeyCapture('language')}
                              on:keydown={(e) => capturingHotkey === 'language' && handleHotkeyKeydown(e, 'language')}
                              on:keyup={(e) => capturingHotkey === 'language' && handleHotkeyKeyup(e, 'language')}
                              tabindex="0"
                            >
                              {#if capturingHotkey === 'language'}
                                {#if tempHotkeyValue}
                                  <div class="hotkey-pills">
                                    {#each tempHotkeyValue.split('+') as key}
                                      <span class="hotkey-pill">{key}</span>
                                    {/each}
                                  </div>
                                {:else}
                                  <span class="hotkey-placeholder">Hold two or more keys at once, then release</span>
                                {/if}
                                <button class="hotkey-cancel" on:click|stopPropagation={stopHotkeyCapture}>
                                  <Icon icon="ph:x" />
                                </button>
                              {:else}
                                {#if settingsConfig.language_toggle_hotkey}
                                  <div class="hotkey-pills">
                                    {#each settingsConfig.language_toggle_hotkey.split('+') as key}
                                      <span class="hotkey-pill">{key}</span>
                                    {/each}
                                  </div>
                                  <button class="hotkey-clear" on:click|stopPropagation={() => clearHotkey('language')}>
                                    <Icon icon="ph:x" />
                                  </button>
                                {:else}
                                  <span class="hotkey-placeholder">Click, then hold two or more keys</span>
                                  <Icon icon="ph:pencil-simple" class="hotkey-edit-icon" />
                                {/if}
                              {/if}
                            </div>
                          </div>
                        </div>
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Command Mode</span>
                            <span class="setting-desc">Create notes, tasks, and reminders by voice</span>
                          </div>
                          <div class="setting-control">
                            <!-- svelte-ignore a11y-click-events-have-key-events -->
                            <!-- svelte-ignore a11y-no-static-element-interactions -->
                            <div
                              class="hotkey-capture-area"
                              class:capturing={capturingHotkey === 'command'}
                              on:click={() => capturingHotkey === 'command' ? stopHotkeyCapture() : startHotkeyCapture('command')}
                              on:keydown={(e) => capturingHotkey === 'command' && handleHotkeyKeydown(e, 'command')}
                              on:keyup={(e) => capturingHotkey === 'command' && handleHotkeyKeyup(e, 'command')}
                              tabindex="0"
                            >
                              {#if capturingHotkey === 'command'}
                                {#if tempHotkeyValue}
                                  <div class="hotkey-pills">
                                    {#each tempHotkeyValue.split('+') as key}
                                      <span class="hotkey-pill">{key}</span>
                                    {/each}
                                  </div>
                                {:else}
                                  <span class="hotkey-placeholder">Hold two or more keys at once, then release</span>
                                {/if}
                                <button class="hotkey-cancel" on:click|stopPropagation={stopHotkeyCapture}>
                                  <Icon icon="ph:x" />
                                </button>
                              {:else}
                                {#if settingsConfig.command_hotkey}
                                  <div class="hotkey-pills">
                                    {#each settingsConfig.command_hotkey.split('+') as key}
                                      <span class="hotkey-pill">{key}</span>
                                    {/each}
                                  </div>
                                  <button class="hotkey-clear" on:click|stopPropagation={() => clearHotkey('command')}>
                                    <Icon icon="ph:x" />
                                  </button>
                                {:else}
                                  <span class="hotkey-placeholder">Click, then hold two or more keys</span>
                                  <Icon icon="ph:pencil-simple" class="hotkey-edit-icon" />
                                {/if}
                              {/if}
                            </div>
                          </div>
                        </div>
                      </div>
                    {/if}
                  </section>
                  
                  <!-- Startup Section -->
                  <section class="settings-section" class:collapsed={settingsCollapsed.general_startup}>
                    <button class="section-header" on:click={() => toggleSettingsSection('general_startup')}>
                      <h3>Startup & Behavior</h3>
                      <Icon icon={settingsCollapsed.general_startup ? 'ph:caret-down' : 'ph:caret-up'} />
                    </button>
                    {#if !settingsCollapsed.general_startup}
                      <div class="section-content">
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Launch at Login</span>
                            <span class="setting-desc">Start Kalam automatically when you log in</span>
                          </div>
                          <div class="setting-control">
                            <label class="toggle-switch">
                              <input type="checkbox" bind:checked={settingsConfig.auto_start} />
                              <span class="slider"></span>
                            </label>
                          </div>
                        </div>
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Start Dictation on Focus</span>
                            <span class="setting-desc">Automatically start when you focus the input field</span>
                          </div>
                          <div class="setting-control">
                            <label class="toggle-switch">
                              <input type="checkbox" bind:checked={settingsConfig.start_in_focus} />
                              <span class="slider"></span>
                            </label>
                          </div>
                        </div>
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Recording Mode</span>
                            <span class="setting-desc">How to trigger dictation</span>
                          </div>
                          <div class="setting-control">
                            <div class="segmented-control">
                              <button class:active={settingsConfig.recording_mode === 'Hold'} on:click={() => settingsConfig.recording_mode = 'Hold'}>Hold</button>
                              <button class:active={settingsConfig.recording_mode === 'Toggle'} on:click={() => settingsConfig.recording_mode = 'Toggle'}>Toggle</button>
                              <button class:active={settingsConfig.recording_mode === 'Both'} on:click={() => settingsConfig.recording_mode = 'Both'}>Both</button>
                            </div>
                          </div>
                        </div>
                      </div>
                    {/if}
                  </section>
                  
                  <!-- Appearance Section -->
                  <section class="settings-section" class:collapsed={settingsCollapsed.general_appearance}>
                    <button class="section-header" on:click={() => toggleSettingsSection('general_appearance')}>
                      <h3>Appearance</h3>
                      <Icon icon={settingsCollapsed.general_appearance ? 'ph:caret-down' : 'ph:caret-up'} />
                    </button>
                    {#if !settingsCollapsed.general_appearance}
                      <div class="section-content">
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Dark Mode</span>
                            <span class="setting-desc">Use dark theme throughout the app</span>
                          </div>
                          <div class="setting-control">
                            <label class="toggle-switch">
                              <input type="checkbox" checked={darkMode} on:change={toggleTheme} />
                              <span class="slider"></span>
                            </label>
                          </div>
                        </div>
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Waveform Style</span>
                            <span class="setting-desc">Visual style of the recording indicator</span>
                          </div>
                          <div class="setting-control">
                            <select class="form-select" bind:value={settingsConfig.waveform_style}>
                              <option value="SiriWave">Siri Wave</option>
                              <option value="EchoRing">Echo Ring</option>
                              <option value="RoundedBars">Rounded Bars</option>
                              <option value="BreathingAura">Breathing Aura</option>
                              <option value="Oscilloscope">Oscilloscope</option>
                              <option value="NeonPulse">Neon Pulse</option>
                              <option value="Aurora">Aurora Borealis</option>
                            </select>
                          </div>
                        </div>
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Overlay Position</span>
                            <span class="setting-desc">Where to show the recording overlay</span>
                          </div>
                          <div class="setting-control">
                            <select class="form-select" bind:value={settingsConfig.overlay_position}>
                              <option value="BottomCenter">Bottom Center</option>
                              <option value="BottomLeft">Bottom Left</option>
                              <option value="BottomRight">Bottom Right</option>
                              <option value="TopCenter">Top Center</option>
                              <option value="Center">Center</option>
                            </select>
                          </div>
                        </div>
                        <div class="setting-row sub-setting">
                          <div class="setting-label">
                            <span class="setting-name">Offset X</span>
                            <span class="setting-desc">Horizontal adjustment (pixels)</span>
                          </div>
                          <div class="setting-control">
                            <div class="number-input">
                              <input type="number" bind:value={settingsConfig.overlay_offset_x} step="10" />
                              <span class="unit">px</span>
                            </div>
                          </div>
                        </div>
                        <div class="setting-row sub-setting">
                          <div class="setting-label">
                            <span class="setting-name">Offset Y</span>
                            <span class="setting-desc">Vertical adjustment (pixels)</span>
                          </div>
                          <div class="setting-control">
                            <div class="number-input">
                              <input type="number" bind:value={settingsConfig.overlay_offset_y} step="10" />
                              <span class="unit">px</span>
                            </div>
                          </div>
                        </div>
                      </div>
                    {/if}
                  </section>
                </div>
              {/if}
              
              <!-- AUDIO & DICTATION TAB -->
              {#if settingsActiveTab === 'dictation'}
                <div class="settings-tab-content">
                  <!-- Audio Input Section -->
                  <section class="settings-section" class:collapsed={settingsCollapsed.dictation_audio}>
                    <button class="section-header" on:click={() => toggleSettingsSection('dictation_audio')}>
                      <h3>Audio Input</h3>
                      <Icon icon={settingsCollapsed.dictation_audio ? 'ph:caret-down' : 'ph:caret-up'} />
                    </button>
                    {#if !settingsCollapsed.dictation_audio}
                      <div class="section-content">
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Microphone</span>
                            <span class="setting-desc">Select your audio input device</span>
                          </div>
                          <div class="setting-control">
                            <select class="form-select" bind:value={settingsConfig.audio_device}>
                              <option value="">System Default</option>
                              <option value="mic1">Built-in Microphone</option>
                              <option value="mic2">External Microphone</option>
                            </select>
                          </div>
                        </div>
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Minimum Hold Time</span>
                            <span class="setting-desc">Minimum milliseconds to hold the hotkey</span>
                          </div>
                          <div class="setting-control">
                            <div class="number-input">
                              <input type="number" bind:value={settingsConfig.min_hold_ms} min="100" max="1000" step="50" />
                              <span class="unit">ms</span>
                            </div>
                          </div>
                        </div>
                      </div>
                    {/if}
                  </section>
                  
                  <!-- STT Mode Section -->
                  <section class="settings-section" class:collapsed={settingsCollapsed.dictation_mode}>
                    <button class="section-header" on:click={() => toggleSettingsSection('dictation_mode')}>
                      <h3>Speech-to-Text Mode</h3>
                      <Icon icon={settingsCollapsed.dictation_mode ? 'ph:caret-down' : 'ph:caret-up'} />
                    </button>
                    {#if !settingsCollapsed.dictation_mode}
                      <div class="section-content">
                        <div class="stt-mode-cards">
                          {#each ['Cloud', 'Local', 'Hybrid'] as mode}
                            <button
                              class="stt-mode-card"
                              class:active={settingsConfig.stt_mode === mode}
                              on:click={() => settingsConfig.stt_mode = mode}
                            >
                              <div class="mode-icon">
                                <Icon icon={mode === 'Cloud' ? 'ph:cloud' : mode === 'Local' ? 'ph:hard-drives' : 'ph:arrows-left-right'} />
                              </div>
                              <div class="mode-info">
                                <span class="mode-name">{mode}</span>
                                <span class="mode-desc">
                                  {mode === 'Cloud' ? 'Fastest, requires internet' : 
                                   mode === 'Local' ? 'Private, runs on your device' : 
                                   'Combines both for best results'}
                                </span>
                              </div>
                            </button>
                          {/each}
                        </div>
                        
                        {#if settingsConfig.stt_mode === 'Cloud' || settingsConfig.stt_mode === 'Hybrid'}
                          <div class="setting-row sub-setting">
                            <div class="setting-label">
                              <span class="setting-name">Cloud Provider</span>
                              <span class="setting-desc">API service for transcription</span>
                            </div>
                            <div class="setting-control">
                              <select class="form-select" bind:value={settingsConfig.stt_provider}>
                                <option value="groq">Groq (whisper-large-v3-turbo)</option>
                                <option value="openai">OpenAI (whisper-1)</option>
                              </select>
                            </div>
                          </div>
                          <div class="api-key-section">
                            <div class="api-key-row">
                              <input 
                                type="password" 
                                class="api-key-input" 
                                placeholder="Enter API key..."
                                bind:value={settingsConfig.api_key_input}
                              />
                              <button class="secondary-btn" on:click={() => settingsConfig.api_key_valid = settingsConfig.api_key_input.length > 0}>
                                Validate
                              </button>
                              {#if settingsConfig.has_api_key && !settingsConfig.api_key_input}
                                <button class="secondary-btn danger" on:click={() => { settingsConfig.has_api_key = false; settingsConfig.api_key_valid = null; }}>
                                  Clear
                                </button>
                              {/if}
                            </div>
                            {#if settingsConfig.api_key_valid !== null}
                              <span class="validation-badge" class:valid={settingsConfig.api_key_valid}>
                                {settingsConfig.api_key_valid ? '✓ Valid' : '✗ Invalid'}
                              </span>
                            {/if}
                            <p class="api-key-hint">
                              {#if settingsConfig.stt_provider === 'openai'}
                                <a href="https://platform.openai.com/api-keys" target="_blank">Get your API key from OpenAI →</a>
                              {:else}
                                <a href="https://console.groq.com" target="_blank">Get your API key from Groq →</a>
                              {/if}
                            </p>
                          </div>
                        {/if}
                        
                        {#if settingsConfig.stt_mode === 'Local' || settingsConfig.stt_mode === 'Hybrid'}
                          <div class="local-models-section">
                            <p class="local-models-hint">Select one model; it is used when mode is Local. Download, start, or stop from the list.</p>
                            <div class="model-list">
                              {#each Object.entries(localModelStatus) as [modelId, status]}
                                {@const isActive = settingsConfig.local_model === modelId}
                                <div class="model-item" class:active={isActive}>
                                  <div class="model-info-row" on:click={() => settingsConfig.local_model = modelId}>
                                    <span class="model-radio" class:checked={isActive}></span>
                                    <div class="model-details">
                                      <span class="model-name">{status.label}</span>
                                      <span class="model-meta">{status.quality} • {status.languages} • {status.size_mb} MB</span>
                                    </div>
                                  </div>
                                  <div class="model-actions">
                                    {#if !status.installed}
                                      <button class="secondary-btn">Download</button>
                                    {:else}
                                      {#if status.status === 'Stopped'}
                                        <button class="secondary-btn" on:click|stopPropagation={() => localModelStatus[modelId].status = 'Running'}>Start</button>
                                      {:else if status.status === 'Running'}
                                        <button class="secondary-btn" on:click|stopPropagation={() => localModelStatus[modelId].status = 'Stopped'}>Stop</button>
                                        <button class="secondary-btn" on:click|stopPropagation={() => {}}>Restart</button>
                                      {:else if status.status === 'Starting'}
                                        <button class="secondary-btn" disabled>Starting...</button>
                                      {/if}
                                      <button class="secondary-btn danger">Delete</button>
                                    {/if}
                                  </div>
                                </div>
                              {/each}
                            </div>
                          </div>
                        {/if}
                      </div>
                    {/if}
                  </section>
                  
                  <!-- Formatting Section -->
                  <section class="settings-section" class:collapsed={settingsCollapsed.dictation_formatting}>
                    <button class="section-header" on:click={() => toggleSettingsSection('dictation_formatting')}>
                      <h3>Formatting & Output</h3>
                      <Icon icon={settingsCollapsed.dictation_formatting ? 'ph:caret-down' : 'ph:caret-up'} />
                    </button>
                    {#if !settingsCollapsed.dictation_formatting}
                      <div class="section-content">
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Auto-punctuation</span>
                            <span class="setting-desc">Automatically insert commas and periods</span>
                          </div>
                          <div class="setting-control">
                            <label class="toggle-switch">
                              <input type="checkbox" bind:checked={settingsConfig.auto_punctuation} />
                              <span class="slider"></span>
                            </label>
                          </div>
                        </div>
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Voice Commands</span>
                            <span class="setting-desc">Say "new line", "delete", etc. to control text</span>
                          </div>
                          <div class="setting-control">
                            <label class="toggle-switch">
                              <input type="checkbox" bind:checked={settingsConfig.voice_commands} />
                              <span class="slider"></span>
                            </label>
                          </div>
                        </div>
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Filler Word Removal</span>
                            <span class="setting-desc">Remove "um", "uh", "like", etc.</span>
                          </div>
                          <div class="setting-control">
                            <label class="toggle-switch">
                              <input type="checkbox" bind:checked={settingsConfig.filler_word_removal} />
                              <span class="slider"></span>
                            </label>
                          </div>
                        </div>
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Text Injection</span>
                            <span class="setting-desc">How to insert text into applications</span>
                          </div>
                          <div class="setting-control">
                            <select class="form-select" bind:value={settingsConfig.injection_method}>
                              <option value="Auto">Automatic</option>
                              <option value="Keystrokes">Simulate Keystrokes</option>
                              <option value="Clipboard">Use Clipboard</option>
                            </select>
                          </div>
                        </div>
                      </div>
                    {/if}
                  </section>
                </div>
              {/if}
              
              <!-- PRIVACY TAB -->
              {#if settingsActiveTab === 'privacy'}
                <div class="settings-tab-content">
                  <section class="settings-section" class:collapsed={settingsCollapsed.privacy_data}>
                    <button class="section-header" on:click={() => toggleSettingsSection('privacy_data')}>
                      <h3>Data & Privacy</h3>
                      <Icon icon={settingsCollapsed.privacy_data ? 'ph:caret-down' : 'ph:caret-up'} />
                    </button>
                    {#if !settingsCollapsed.privacy_data}
                      <div class="section-content">
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">History Retention</span>
                            <span class="setting-desc">How long to keep dictation history</span>
                          </div>
                          <div class="setting-control">
                            <select class="form-select" bind:value={settingsConfig.history_retention_days}>
                              <option value={7}>7 days</option>
                              <option value={30}>30 days</option>
                              <option value={90}>90 days</option>
                              <option value={365}>1 year</option>
                              <option value={0}>Forever</option>
                            </select>
                          </div>
                        </div>
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Telemetry</span>
                            <span class="setting-desc">Send anonymous usage statistics</span>
                          </div>
                          <div class="setting-control">
                            <label class="toggle-switch">
                              <input type="checkbox" bind:checked={settingsConfig.telemetry_enabled} />
                              <span class="slider"></span>
                            </label>
                          </div>
                        </div>
                        <div class="privacy-info">
                          <Icon icon="ph:shield-check" />
                          <p>Your voice data is never stored on our servers. Transcriptions are processed in real-time and discarded immediately. Local mode keeps everything on your device.</p>
                        </div>
                      </div>
                    {/if}
                  </section>
                </div>
              {/if}
              
              <!-- ADVANCED TAB -->
              {#if settingsActiveTab === 'advanced'}
                <div class="settings-tab-content">
                  <section class="settings-section" class:collapsed={settingsCollapsed.advanced_logs}>
                    <button class="section-header" on:click={() => toggleSettingsSection('advanced_logs')}>
                      <h3>Logging & Diagnostics</h3>
                      <Icon icon={settingsCollapsed.advanced_logs ? 'ph:caret-down' : 'ph:caret-up'} />
                    </button>
                    {#if !settingsCollapsed.advanced_logs}
                      <div class="section-content">
                        <div class="setting-row">
                          <div class="setting-label">
                            <span class="setting-name">Enable Logging</span>
                            <span class="setting-desc">Keep app logs for troubleshooting</span>
                          </div>
                          <div class="setting-control">
                            <label class="toggle-switch">
                              <input type="checkbox" />
                              <span class="slider"></span>
                            </label>
                          </div>
                        </div>
                        <div class="log-actions">
                          <button class="secondary-btn">
                            <Icon icon="ph:download" />
                            Export Logs
                          </button>
                          <button class="secondary-btn">
                            <Icon icon="ph:folder-open" />
                            Open Data Folder
                          </button>
                        </div>
                      </div>
                    {/if}
                  </section>
                  
                  <section class="settings-section danger" class:collapsed={settingsCollapsed.advanced_danger}>
                    <button class="section-header" on:click={() => toggleSettingsSection('advanced_danger')}>
                      <h3>Danger Zone</h3>
                      <Icon icon={settingsCollapsed.advanced_danger ? 'ph:caret-down' : 'ph:caret-up'} />
                    </button>
                    {#if !settingsCollapsed.advanced_danger}
                      <div class="section-content">
                        <div class="danger-item">
                          <div class="danger-info">
                            <span class="danger-title">Reset All Settings</span>
                            <span class="danger-desc">Restore default configuration</span>
                          </div>
                          <button class="danger-btn">Reset</button>
                        </div>
                        <div class="danger-item">
                          <div class="danger-info">
                            <span class="danger-title">Clear All Data</span>
                            <span class="danger-desc">Delete history, notes, tasks, and settings</span>
                          </div>
                          <button class="danger-btn">Clear Data</button>
                        </div>
                      </div>
                    {/if}
                  </section>
                </div>
              {/if}
              
              <!-- ABOUT TAB -->
              {#if settingsActiveTab === 'about'}
                <div class="settings-tab-content about-content">
                  <!-- Top Row: Version + Updates Combined -->
                  <section class="about-top-section">
                    <div class="about-top-content">
                      <div class="version-block">
                        <span class="version-label">Current version: <strong>1.2.0-beta</strong></span>
                      </div>
                      <div class="updates-block">
                        <div class="channel-selector">
                          <select class="channel-select">
                            <option value="stable">Stable (Recommended)</option>
                            <option value="beta">Beta (Pre-releases)</option>
                          </select>
                        </div>
                        <button type="button" class="btn-check">Check Now</button>
                      </div>
                    </div>
                  </section>

                  <!-- Bottom Row: Community & Support -->
                  <div class="about-grid two-col">
                    <!-- Community Card -->
                    <section class="about-card">
                      <div class="card-icon"><Icon icon="ph:users-three-duotone" /></div>
                      <h3>Community</h3>
                      <p class="byline">Created by <a href="https://github.com/afaraha8403" target="_blank" rel="noopener">Ali Farahat</a>, founder of <a href="https://balacode.io" target="_blank" rel="noopener">Balacode.io</a>.</p>
                      <div class="action-group">
                        <a href="https://github.com/afaraha8403/kalam" target="_blank" rel="noopener" class="action-link">
                          <Icon icon="ph:github-logo-duotone" />
                          <span>GitHub Repository</span>
                        </a>
                        <a href="#" class="action-link secondary">
                          <Icon icon="ph:file-text-duotone" />
                          <span>Terms & Conditions</span>
                        </a>
                        <a href="#" class="action-link secondary">
                          <Icon icon="ph:shield-check-duotone" />
                          <span>Privacy Policy</span>
                        </a>
                      </div>
                    </section>

                    <!-- Support Card -->
                    <section class="about-card highlight">
                      <div class="card-icon"><Icon icon="ph:heart-duotone" /></div>
                      <h3>Support Kalam</h3>
                      <p class="card-text">Keep Kalam sustainable and free. Commercial use requires a separate license.</p>
                      <div class="action-group">
                        <a href="https://github.com/sponsors/afaraha8403" target="_blank" rel="noopener" class="btn-primary about-btn-primary">
                          <Icon icon="ph:heart-straight-fill" /> Sponsor
                        </a>
                        <a href="#" class="action-link secondary">Commercial License</a>
                      </div>
                    </section>
                  </div>

                  <!-- License Section -->
                  <section class="license-section">
                    <button type="button" class="accordion" on:click={() => toggleSettingsSection('license')}>
                      <span class="accordion-title"><Icon icon="ph:file-text-duotone" /> License Information</span>
                      <Icon icon={settingsCollapsed.license ? 'ph:caret-down' : 'ph:caret-up'} />
                    </button>
                    {#if !settingsCollapsed.license}
                      <div class="license-content">
                        <pre class="license-text">Dual License: MIT (Noncommercial) + Commercial by Permission

Copyright (c) 2026 Kalam Voice Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

(1) Use is limited to NONCOMMERCIAL purposes.
(2) COMMERCIAL USE requires a separate written license from the copyright holder.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND.</pre>
                      </div>
                    {/if}
                  </section>
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <div class="status-bar-wrap">
      <StatusBar
        config={statusBarConfig}
        dbStatus={dbStatus}
        platform={statusBarPlatform}
        {lastLatencyMs}
        dictationEnabled={dictationEnabled}
        onRetryDb={onRetryDb}
      />
    </div>
  </main>
</div>

<style>
  /* === SLEEK MINIMALIST DESIGN SYSTEM === */
  .kalam-sleek {
    --font: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    
    --space-xs: 4px;
    --space-sm: 8px;
    --space-md: 16px;
    --space-lg: 24px;
    --space-xl: 32px;
    --space-2xl: 48px;
    --space-3xl: 64px;
    
    --radius-sm: 8px;
    --radius-md: 12px;
    --radius-lg: 20px;
    --radius-full: 9999px;
    
    --transition: 200ms cubic-bezier(0.4, 0, 0.2, 1);
  }

  /* Light Theme - Apple/OpenAI inspired */
  .kalam-sleek.light {
    --bg: #ffffff;
    --bg-elevated: #f5f5f7;
    --bg-card: #ffffff;
    --bg-hover: rgba(0, 0, 0, 0.04);
    --border: rgba(0, 0, 0, 0.08);
    --border-light: rgba(0, 0, 0, 0.04);
    --text: #1d1d1f;
    --text-secondary: #86868b;
    --text-muted: #a1a1a6;
    --accent: #000000;
    --accent-fg: #ffffff;
    --accent-dim: rgba(0, 0, 0, 0.04);
    --shadow: 0 2px 12px rgba(0, 0, 0, 0.03);
    /* Override global app.css vars so no dark blue leaks (body/h1–h6 use --text-primary, --navy-deep) */
    --text-primary: #1d1d1f;
    --navy-deep: #1d1d1f;
  }

  /* Dark Theme */
  .kalam-sleek.dark {
    --bg: #000000;
    --bg-elevated: #1c1c1e;
    --bg-card: #1c1c1e;
    --bg-hover: rgba(255, 255, 255, 0.08);
    --border: rgba(255, 255, 255, 0.12);
    --border-light: rgba(255, 255, 255, 0.06);
    --text: #f5f5f7;
    --text-secondary: #a1a1a6;
    --text-muted: #6e6e73;
    --accent: #ffffff;
    --accent-fg: #000000;
    --accent-dim: rgba(255, 255, 255, 0.08);
    --shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
    /* Override global app.css vars so no dark blue leaks in dark mode */
    --text-primary: #f5f5f7;
    --navy-deep: #f5f5f7;
  }

  .kalam-sleek {
    display: flex;
    min-height: 100vh;
    height: 100vh;
    background: var(--bg);
    color: var(--text);
    font-family: var(--font);
    overflow: hidden;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  /* === ANIMATIONS === */
  .fade-in {
    animation: fadeIn 0.4s ease-out forwards;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* === SIDEBAR === */
  .sidebar {
    width: 240px;
    flex-shrink: 0;
    background: var(--bg-elevated);
    display: flex;
    flex-direction: column;
    /* No right border for a cleaner look */
  }

  .sidebar-content {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: var(--space-lg) var(--space-md);
  }

  .logo-section {
    padding: 0 var(--space-sm);
    margin-bottom: var(--space-xl);
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .logo-svg {
    width: 28px;
    height: 28px;
    color: var(--text);
  }

  .logo-text {
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.03em;
  }

  .main-nav {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nav-link {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition);
    text-align: left;
  }

  .nav-link:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .nav-link.active {
    background: var(--bg-card);
    color: var(--text);
    box-shadow: var(--shadow);
    font-weight: 600;
  }

  .nav-link :global(svg) {
    font-size: 18px;
  }

  .sidebar-bottom {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .dictation-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 14px;
    border: none;
    border-radius: var(--radius-full);
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition);
    position: relative;
    overflow: hidden;
  }

  .dictation-btn:hover {
    transform: scale(0.98);
    opacity: 0.9;
  }

  .dictation-btn :global(svg) {
    font-size: 18px;
  }

  .pulse-dot {
    width: 6px;
    height: 6px;
    background: #ff3b30;
    border-radius: 50%;
    position: absolute;
    right: 16px;
    animation: pulse 2s infinite;
  }

  @keyframes pulse {
    0% { box-shadow: 0 0 0 0 rgba(255, 59, 48, 0.4); }
    70% { box-shadow: 0 0 0 6px rgba(255, 59, 48, 0); }
    100% { box-shadow: 0 0 0 0 rgba(255, 59, 48, 0); }
  }

  .bottom-links {
    display: flex;
    justify-content: center;
    gap: var(--space-sm);
  }

  .icon-btn {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: var(--transition);
  }

  .icon-btn:hover, .icon-btn.active {
    background: var(--bg-hover);
    color: var(--text);
  }

  .icon-btn :global(svg) {
    font-size: 18px;
  }

  /* === MAIN CONTENT === */
  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
  }

  .loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border);
    border-top-color: var(--text);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .page-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-3xl) var(--space-2xl);
    max-width: 900px;
    margin: 0 auto;
    width: 100%;
  }

  .page-header {
    margin-bottom: var(--space-2xl);
  }

  .page-title {
    font-size: 36px;
    font-weight: 600;
    letter-spacing: -0.04em;
    margin: 0 0 8px 0;
  }

  .page-subtitle {
    font-size: 16px;
    color: var(--text-secondary);
    margin: 0;
  }

  /* === HOME PAGE === */
  .stats-row {
    display: flex;
    gap: var(--space-lg);
    margin-bottom: var(--space-3xl);
  }

  .stat-box {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: var(--space-lg);
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
  }

  .stat-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .stat-num {
    font-size: 32px;
    font-weight: 600;
    letter-spacing: -0.03em;
    color: var(--text);
  }

  .dashboard-grid {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xl);
  }

  .dash-columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-2xl);
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-md);
    padding-bottom: var(--space-sm);
    border-bottom: 1px solid var(--border-light);
  }

  .section-header h3 {
    font-size: 16px;
    font-weight: 600;
    margin: 0;
  }

  .text-btn {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
  }

  .text-btn:hover {
    color: var(--text);
  }

  .history-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .list-item {
    display: flex;
    gap: var(--space-md);
    padding: var(--space-md);
    background: var(--bg-elevated);
    border-radius: var(--radius-md);
    transition: var(--transition);
  }

  .list-item:hover {
    background: var(--bg-hover);
  }

  .item-icon {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--bg);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .item-content {
    flex: 1;
  }

  .item-text {
    font-size: 14px;
    line-height: 1.5;
    margin: 0 0 6px 0;
    color: var(--text);
  }

  .item-meta-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .item-meta {
    font-size: 12px;
    color: var(--text-muted);
  }

  .simple-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .simple-item {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 14px;
    padding: 8px 0;
  }

  .priority-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .muted-icon {
    color: var(--text-muted);
    font-size: 16px;
  }

  .simple-text {
    color: var(--text);
  }

  /* === HISTORY PAGE === */
  .search-bar {
    position: relative;
    margin-bottom: var(--space-3xl);
  }

  .search-bar .search-bar-icon {
    position: absolute;
    left: 16px;
    top: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    color: var(--text-muted);
  }

  .search-bar .search-bar-icon :global(svg) {
    font-size: 18px;
    width: 1em;
    height: 1em;
    display: block;
  }

  .search-bar input {
    width: 100%;
    padding: 16px 16px 16px 48px;
    background: var(--bg-elevated);
    border: 1px solid transparent;
    border-radius: var(--radius-lg);
    color: var(--text);
    font-size: 15px;
    font-family: var(--font);
    transition: var(--transition);
  }

  .search-bar input:focus {
    outline: none;
    background: var(--bg);
    border-color: var(--border);
    box-shadow: var(--shadow);
  }

  .timeline {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xl);
  }

  .day-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
  }

  .day-label {
    font-size: 18px;
    font-weight: 600;
    margin: 0;
    display: flex;
    align-items: baseline;
    gap: 12px;
  }

  .day-sub {
    font-size: 13px;
    font-weight: 400;
    color: var(--text-secondary);
  }

  .entries {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .entry-row {
    display: flex;
    gap: var(--space-lg);
    padding: var(--space-lg);
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    transition: var(--transition);
  }

  .entry-row:hover {
    background: var(--bg-hover);
  }

  .entry-time {
    width: 60px;
    font-size: 13px;
    color: var(--text-secondary);
    flex-shrink: 0;
    padding-top: 2px;
  }

  .entry-content {
    flex: 1;
  }

  .entry-text {
    font-size: 15px;
    line-height: 1.6;
    margin: 0 0 12px 0;
  }

  .entry-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  /* Chips: mode (dictation vs command) and STT (Cloud / Local / Hybrid) */
  .chip {
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 4px 8px;
    border-radius: 6px;
  }

  .chip-mode.dictation {
    background: rgba(52, 199, 89, 0.15);
    color: #34C759;
  }

  .chip-mode.command {
    background: rgba(175, 82, 222, 0.15);
    color: #AF52DE;
  }

  .chip-stt.cloud {
    background: rgba(10, 132, 255, 0.15);
    color: #0A84FF;
  }

  .chip-stt.local {
    background: rgba(48, 209, 88, 0.15);
    color: #30D158;
  }

  .chip-stt.hybrid {
    background: rgba(255, 159, 10, 0.15);
    color: #FF9F0A;
  }

  .chip.small {
    font-size: 10px;
    padding: 2px 6px;
  }

  .entry-duration {
    font-size: 12px;
    color: var(--text-muted);
  }

  .icon-btn.small {
    width: 28px;
    height: 28px;
    margin-left: auto;
  }

  /* === NOTES PAGE === */
  .notes-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: var(--space-md);
  }

  .btn-primary {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 12px 20px;
    background: var(--accent);
    color: var(--accent-fg);
    border: none;
    border-radius: var(--radius-full);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition);
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
    transform: scale(0.98);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .notes-subnav {
    display: flex;
    gap: 4px;
    margin-bottom: var(--space-lg);
  }

  .subnav-btn {
    padding: 8px 16px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: var(--transition);
  }

  .subnav-btn:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .subnav-btn.active {
    background: var(--bg-elevated);
    color: var(--text);
  }

  .notes-search-bar {
    position: relative;
    margin-bottom: var(--space-md);
  }

  .notes-search-bar :global(svg) {
    position: absolute;
    left: 14px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 18px;
    color: var(--text-muted);
  }

  .notes-search-bar input {
    width: 100%;
    padding: 12px 14px 12px 44px;
    background: var(--bg-elevated);
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 14px;
    transition: var(--transition);
  }

  .notes-search-bar input:focus {
    outline: none;
    border-color: var(--border);
  }

  .notes-label-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: var(--space-lg);
  }

  .label-chip {
    padding: 6px 12px;
    border-radius: var(--radius-full);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: pointer;
    transition: var(--transition);
  }

  .label-chip:hover,
  .label-chip.active {
    background: var(--bg-hover);
    color: var(--text);
    border-color: var(--border);
  }

  .notes-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-3xl);
    color: var(--text-muted);
    gap: var(--space-md);
  }

  .notes-empty :global(svg) {
    font-size: 48px;
  }

  .notes-lists-container {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xl);
  }

  .notes-section-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0 0 var(--space-lg) 0;
  }

  /* Masonry-style grid: columns with variable-height cards (break-inside: avoid). */
  .notes-masonry {
    column-count: 3;
    column-gap: var(--space-lg);
  }

  @media (max-width: 900px) {
    .notes-masonry { column-count: 2; }
  }

  @media (max-width: 560px) {
    .notes-masonry { column-count: 1; }
  }

  .note-card {
    break-inside: avoid;
    margin-bottom: var(--space-lg);
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    position: relative;
    text-align: left;
    border: none;
    transition: transform 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    cursor: pointer;
    aspect-ratio: 1 / 1;
    height: auto;
    /* Default: use theme text colors. */
    --note-fg: var(--text);
    --note-fg-secondary: var(--text-secondary);
    --note-fg-muted: var(--text-muted);
    --note-border: var(--border-light);
    --note-bg-hover: var(--bg-hover);
  }

  /* Colored background: force dark text so it's readable on yellow, mint, etc. (light/dark mode). */
  .note-card.has-custom-color {
    --note-fg: #1d1d1f;
    --note-fg-secondary: #424245;
    --note-fg-muted: #6e6e73;
    --note-border: rgba(0, 0, 0, 0.12);
    --note-bg-hover: rgba(0, 0, 0, 0.06);
  }

  .note-card:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow);
  }

  .note-card.dragging {
    opacity: 0.8;
    transform: scale(1.02) rotate(2deg);
    box-shadow: 0 20px 40px rgba(0,0,0,0.1);
    z-index: 10;
  }

  .note-card.pinned::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: var(--accent);
  }

  .note-card.has-custom-color.pinned::before {
    background: var(--note-fg-muted);
  }

  .note-inner {
    padding: var(--space-lg);
    flex: 1;
    min-height: 0;
  }

  .note-card .note-title {
    font-size: 16px;
    font-weight: 600;
    margin: 0 0 8px 0;
    color: var(--note-fg);
  }

  .note-content-preview {
    font-size: 14px;
    line-height: 1.5;
    color: var(--note-fg-secondary);
    margin: 0 0 8px 0;
    /* Variable height up to a limit for masonry; then clamp. */
    max-height: 10em;
    display: -webkit-box;
    -webkit-line-clamp: 5;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .note-tags-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 8px;
  }

  .note-tag {
    font-size: 11px;
    padding: 2px 8px;
    background: var(--note-bg-hover);
    color: var(--note-fg-secondary);
    border-radius: 4px;
  }

  .note-reminder-row {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--note-fg-muted);
  }

  .note-reminder-row :global(svg) {
    font-size: 14px;
  }

  .note-footer {
    padding: 12px var(--space-lg);
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid var(--note-border);
  }

  .note-date {
    font-size: 12px;
    color: var(--note-fg-muted);
  }

  .note-actions {
    display: flex;
    gap: 4px;
  }

  .note-action-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: none;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    color: var(--note-fg-muted);
    transition: var(--transition);
  }

  .note-action-btn:hover {
    background: var(--note-bg-hover);
    color: var(--note-fg);
  }

  .note-action-btn.delete:hover {
    color: #FF3B30;
    background: rgba(255, 59, 48, 0.1);
  }

  .pin-icon {
    position: absolute;
    top: var(--space-md);
    right: var(--space-md);
    color: var(--note-fg-muted);
    font-size: 14px;
  }

  /* === POLISHED NOTE EDITOR === */
  .polished-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    max-width: 800px;
    margin: 0 auto;
    padding: var(--space-xl) var(--space-2xl);
  }

  .sleek-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-2xl);
  }

  .sleek-back {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 15px;
    font-weight: 500;
    cursor: pointer;
    padding: 8px 0;
    transition: var(--transition);
  }

  .sleek-back:hover {
    color: var(--text);
  }

  .sleek-actions {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .sleek-cancel {
    padding: 6px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    background: var(--bg);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition);
  }

  .sleek-cancel:hover {
    background: var(--bg-hover);
  }

  .sleek-save {
    padding: 6px 16px;
    border: none;
    border-radius: var(--radius-full);
    background: var(--text);
    color: var(--bg);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition);
  }

  .sleek-save:hover:not(:disabled) {
    opacity: 0.9;
  }

  .sleek-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .sleek-icon-btn {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: var(--transition);
  }

  .sleek-icon-btn.danger:hover {
    color: #FF3B30;
    background: rgba(255, 59, 48, 0.1);
  }

  .sleek-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .sleek-title {
    width: 100%;
    font-size: 36px;
    font-weight: 700;
    letter-spacing: -0.03em;
    border: 1px solid var(--border-light);
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    color: var(--text);
    outline: none;
    padding: 16px 20px;
    margin-bottom: var(--space-md);
    font-family: var(--font);
    transition: all 0.2s ease;
  }

  .task-title-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: var(--space-xl, 24px);
  }

  .task-title-row .sleek-title {
    margin-bottom: 0;
    flex: 1;
  }

  .sleek-title:hover {
    border-color: var(--border);
  }

  .sleek-title:focus {
    border-color: var(--text-muted);
    background: var(--bg);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.08);
  }

  .sleek-title::placeholder {
    color: var(--text-muted);
    opacity: 0.6;
  }

  .sleek-labels {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-top: var(--space-xl);
    margin-bottom: var(--space-md);
    color: var(--text-muted);
    padding: 0 4px;
  }

  .sleek-label-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 14px;
    color: var(--text-secondary);
    background: var(--bg-elevated);
    padding: 4px 10px;
    border-radius: var(--radius-full);
    border: 1px solid var(--border-light);
  }

  .sleek-label-chip button {
    background: transparent;
    border: none;
    padding: 0;
    color: inherit;
    cursor: pointer;
    display: flex;
    opacity: 0.6;
  }

  .sleek-label-chip button:hover {
    opacity: 1;
    color: #FF3B30;
  }

  .sleek-label-input {
    border: 1px solid var(--border-light);
    background: var(--bg-elevated);
    border-radius: var(--radius-full);
    color: var(--text);
    font-size: 14px;
    outline: none;
    width: 140px;
    padding: 6px 12px;
    font-family: var(--font);
    transition: all 0.2s ease;
  }

  .sleek-label-input:focus {
    border-color: var(--text-muted);
    background: var(--bg);
  }

  .sleek-label-input::placeholder {
    color: var(--text-muted);
  }

  .sleek-content {
    width: 100%;
    flex: 1;
    font-size: 16px;
    line-height: 1.6;
    border: 1px solid var(--border-light);
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    color: var(--text);
    outline: none;
    padding: 20px;
    resize: none;
    font-family: var(--font);
    min-height: 300px;
    transition: all 0.2s ease;
  }

  .sleek-content:hover {
    border-color: var(--border);
  }

  .sleek-content:focus {
    border-color: var(--text-muted);
    background: var(--bg);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.08);
  }

  .sleek-content::placeholder {
    color: var(--text-muted);
    opacity: 0.6;
  }

  .sleek-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: var(--space-xl);
    margin-top: var(--space-xl);
  }

  .sleek-meta {
    font-size: 13px;
    color: var(--text-muted);
    font-weight: 500;
  }

  .sleek-tools {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .color-dropdown-container,
  .reminder-dropdown-container {
    position: relative;
  }

  .current-color-indicator {
    display: block;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1px solid var(--border);
    position: relative;
    overflow: hidden;
  }

  .color-toggle[title="Change color"] .current-color-indicator[style*="background-color: #333"],
  .color-toggle[title="Change color"] .current-color-indicator[style*="background-color: #e5e5e5"] {
    /* Target the default color state to add the slash */
  }

  .color-toggle[title="Change color"] .current-color-indicator[style*="background-color: #333"]::after,
  .color-toggle[title="Change color"] .current-color-indicator[style*="background-color: #e5e5e5"]::after {
    content: '';
    position: absolute;
    top: 50%;
    left: -20%;
    width: 140%;
    height: 1px;
    background-color: #ff3b30;
    transform: rotate(-45deg);
    opacity: 0.7;
  }

  .sleek-popover {
    position: absolute;
    bottom: calc(100% + 12px);
    left: 50%;
    transform: translateX(-50%);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 12px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
    z-index: 100;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .color-popover {
    width: 160px;
  }

  .sleek-colors-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: center;
  }

  .sleek-datetime-input {
    border: 1px solid var(--border-light);
    background: var(--bg);
    color: var(--text);
    font-size: 14px;
    font-family: var(--font);
    outline: none;
    cursor: pointer;
    padding: 8px 12px;
    border-radius: var(--radius-md);
  }

  .sleek-clear-btn {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-muted);
    padding: 6px 12px;
    border-radius: var(--radius-md);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .sleek-clear-btn:hover {
    background: rgba(255, 59, 48, 0.1);
    color: #FF3B30;
    border-color: rgba(255, 59, 48, 0.2);
  }

  .sleek-color-dot {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 1px solid var(--border);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.15s ease, border-color 0.15s ease;
  }

  .sleek-color-dot[title="default"] {
    position: relative;
    overflow: hidden;
  }

  .sleek-color-dot[title="default"]::after {
    content: '';
    position: absolute;
    top: 50%;
    left: -20%;
    width: 140%;
    height: 1px;
    background-color: #ff3b30;
    transform: rotate(-45deg);
    opacity: 0.7;
  }

  .sleek-color-dot:hover {
    transform: scale(1.15);
  }

  .sleek-color-dot.selected {
    transform: scale(1.15);
    border-color: var(--text);
    border-width: 2px;
  }

  .sleek-color-dot :global(svg) {
    font-size: 12px;
    color: rgba(0, 0, 0, 0.5);
  }

  .kalam-sleek.dark .sleek-color-dot :global(svg) {
    color: rgba(0, 0, 0, 0.8);
  }

  .sleek-tool-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: var(--transition);
  }

  .sleek-tool-btn:hover,
  .sleek-tool-btn.active {
    color: var(--text);
    background: var(--bg-elevated);
  }

  .sleek-tool-btn :global(svg) {
    font-size: 24px;
  }

  .sleek-tool-btn.complete-toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    width: auto;
    height: auto;
    border-radius: var(--radius-full);
    font-size: 13px;
    font-weight: 500;
  }

  .sleek-tool-btn.complete-toggle span {
    font-size: 13px;
  }

  .sleek-tool-btn.complete-toggle.completed {
    color: #34C759;
    background: rgba(52, 199, 89, 0.1);
  }

  .sleek-tool-btn.complete-toggle.completed:hover {
    background: rgba(52, 199, 89, 0.15);
  }

  .sleek-tool-btn.complete-toggle.compact {
    width: 36px;
    height: 36px;
    padding: 0;
  }

  .sleek-tool-btn.complete-toggle.compact :global(svg) {
    font-size: 20px;
  }

  /* Reduced motion preference */
  @media (prefers-reduced-motion: reduce) {
    .header-back,
    .action-btn,
    .label-chip,
    .label-remove,
    .add-label-chip,
    .title-input,
    .content-input,
    .color-selector-compact,
    .color-option,
    .reminder-toggle-btn,
    .tool-btn {
      transition: none;
    }
  }

  /* === TASKS PAGE === */
  .task-list-large {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .task-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    background: var(--bg-elevated);
    border-radius: var(--radius-md);
    transition: var(--transition);
    cursor: pointer;
    border: 1px solid transparent;
  }

  .task-row:hover {
    background: var(--bg-hover);
    border-color: var(--border-light);
  }

  .task-row.dragging {
    opacity: 0.5;
    transform: scale(0.98);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }

  .task-row.completed {
    opacity: 0.6;
  }

  .task-row.completed .task-title {
    text-decoration: line-through;
    color: var(--text-secondary);
  }

  .drag-handle {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: grab;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    border-radius: var(--radius-sm);
    opacity: 0;
    transition: var(--transition);
  }

  .task-row:hover .drag-handle {
    opacity: 1;
  }

  .drag-handle:hover {
    background: var(--bg);
    color: var(--text);
  }

  .drag-handle:active {
    cursor: grabbing;
  }

  .checkbox {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 1px solid var(--border);
    background: var(--bg);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    color: var(--text);
    padding: 0;
    transition: all 0.2s ease;
  }

  .checkbox:hover {
    border-color: var(--text-muted);
  }

  .checkbox.large {
    width: 32px;
    height: 32px;
  }

  .checkbox.small {
    width: 20px;
    height: 20px;
  }

  .checkbox.checked,
  .task-row.completed .checkbox {
    background: var(--text);
    color: var(--bg);
    border-color: var(--text);
  }

  .task-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .task-title {
    font-size: 15px;
    font-weight: 500;
  }

  .task-meta {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .task-due {
    font-size: 12px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .task-due.urgent {
    color: #FF3B30;
  }

  .task-subtasks-count {
    font-size: 12px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .task-tags {
    display: flex;
    gap: 6px;
  }

  .task-tag-pill {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }

  .priority-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 64px 0;
    color: var(--text-muted);
    gap: 12px;
  }

  .empty-state :global(svg) {
    font-size: 48px;
    opacity: 0.5;
  }

  /* Task Editor Specifics */
  .task-desc {
    min-height: 120px;
    margin-bottom: var(--space-xl);
  }

  .section-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: var(--space-md);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .due-date-section {
    margin-bottom: var(--space-2xl);
  }

  .due-date-input-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-light);
    border-radius: var(--radius-md);
    transition: all 0.2s ease;
  }

  .due-date-input-row:focus-within {
    border-color: var(--text-muted);
    background: var(--bg);
  }

  .due-date-input-row :global(.sleek-datetime-input) {
    background: transparent;
    border-color: transparent;
    box-shadow: none;
    transition: color 0.2s ease;
  }

  .due-date-input-row :global(svg) {
    color: var(--text-muted);
    font-size: 20px;
  }

  .subtasks-section {
    margin-bottom: var(--space-2xl);
  }

  .subtasks-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .subtask-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-light);
    border-radius: var(--radius-md);
    transition: all 0.2s ease;
  }

  .subtask-row:focus-within {
    border-color: var(--text-muted);
    background: var(--bg);
  }

  .subtask-row.completed {
    opacity: 0.6;
  }

  .subtask-row.completed .subtask-input {
    text-decoration: line-through;
  }

  .subtask-row .drag-handle {
    opacity: 0.5;
    font-size: 16px;
  }

  .subtask-row:hover .drag-handle {
    opacity: 0.8;
  }

  .subtask-input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 14px;
    font-family: var(--font);
    outline: none;
  }

  .remove-subtask {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .subtask-row:hover .remove-subtask {
    opacity: 1;
  }

  .remove-subtask:hover {
    color: #FF3B30;
  }

  .add-subtask-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    color: var(--text-muted);
  }

  .add-subtask-input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 14px;
    font-family: var(--font);
    outline: none;
  }

  .add-subtask-input::placeholder {
    color: var(--text-muted);
  }

  .task-priority-selector {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .tool-label {
    font-size: 13px;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .priority-options {
    display: flex;
    gap: 4px;
  }

  .priority-btn {
    padding: 4px 12px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .priority-btn:hover {
    background: var(--bg-hover);
  }

  .priority-btn.selected {
    background: var(--text);
    color: var(--bg);
    border-color: var(--text);
  }

  /* === REMINDERS PAGE === */
  .reminder-list-large {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .reminder-row {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 20px;
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    transition: var(--transition);
    cursor: pointer;
    border: 1px solid transparent;
  }

  .reminder-row:hover {
    background: var(--bg-hover);
    border-color: var(--border-light);
  }

  .reminder-icon-large {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: var(--bg);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 20px;
    color: var(--text-secondary);
  }

  .reminder-icon-large.recurring {
    color: var(--accent);
    background: rgba(0, 122, 255, 0.1);
  }

  .reminder-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .reminder-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .reminder-text {
    font-size: 15px;
    font-weight: 500;
  }

  .reminder-source-badge {
    font-size: 11px;
    font-weight: 500;
    padding: 4px 8px;
    border-radius: 6px;
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .reminder-source-badge.note {
    background: #fef08a;
    color: #854d0e;
    border-color: #fde047;
  }

  .reminder-source-badge.task {
    background: #bfdbfe;
    color: #1e40af;
    border-color: #93c5fd;
  }

  .reminder-meta {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .reminder-time {
    font-size: 13px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .reminder-recurring-badge {
    font-size: 12px;
    color: var(--accent);
    display: flex;
    align-items: center;
    gap: 4px;
    font-weight: 500;
  }

  .reminder-tags {
    display: flex;
    gap: 6px;
  }

  .reminder-tag {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }

  .reminder-form-row {
    margin-bottom: var(--space-xl);
  }

  .form-label {
    display: block;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: var(--space-sm);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .form-select {
    width: 100%;
    padding: 12px 16px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-light);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 15px;
    font-family: var(--font);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .form-select:focus {
    outline: none;
    border-color: var(--text-muted);
    background: var(--bg);
  }

  .full-width {
    width: 100%;
  }

  /* === SNIPPETS PAGE === */
  .snippets-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: var(--space-lg);
  }

  .snippet-card {
    padding: var(--space-lg);
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    transition: var(--transition);
    cursor: pointer;
    border: 1px solid transparent;
  }

  .snippet-card:hover {
    background: var(--bg-hover);
    border-color: var(--border-light);
  }

  .snippet-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .trigger-code {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    background: var(--bg);
    padding: 4px 8px;
    border-radius: 6px;
    border: 1px solid var(--border-light);
  }

  .uses-count {
    font-size: 12px;
    color: var(--text-muted);
  }

  .expansion-text {
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-secondary);
    margin: 0;
    white-space: pre-line;
  }

  .snippet-tags {
    display: flex;
    gap: 6px;
    margin-top: 12px;
    flex-wrap: wrap;
  }

  .snippet-tag {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }

  .snippet-form-row {
    margin-bottom: var(--space-xl);
  }

  .trigger-input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .trigger-prefix {
    position: absolute;
    left: 16px;
    font-size: 24px;
    color: var(--text-muted);
    font-weight: 500;
  }

  .trigger-input {
    padding-left: 32px;
  }

  .snippet-expansion {
    min-height: 200px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  }

  /* === SETTINGS PAGE === */
  .settings-page {
    max-width: 800px;
  }
  
  .settings-header {
    margin-bottom: var(--space-xl);
  }
  
  .settings-tabs {
    display: flex;
    gap: 4px;
    margin-bottom: var(--space-xl);
    border-bottom: 1px solid var(--border);
    padding-bottom: 1px;
  }
  
  .settings-tab {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 20px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition);
    border-bottom: 2px solid transparent;
    margin-bottom: -2px;
  }
  
  .settings-tab:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  
  .settings-tab.active {
    color: var(--text);
    border-bottom-color: var(--accent);
    font-weight: 600;
  }
  
  .settings-tab :global(svg) {
    font-size: 18px;
  }
  
  .settings-content {
    min-height: 400px;
  }
  
  .settings-tab-content {
    animation: fadeIn 0.2s ease;
  }
  
  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  
  .settings-section {
    margin-bottom: var(--space-lg);
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    overflow: hidden;
  }

  .settings-section.collapsed {
    margin-bottom: var(--space-xs);
  }

  .settings-section.collapsed .section-header {
    margin: 0;
    padding: var(--space-lg);
  }

  .settings-section.danger {
    border-color: rgba(255, 59, 48, 0.3);
  }
  
  .settings-section .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: var(--space-lg);
    margin: 0;
    background: transparent;
    border: none;
    cursor: pointer;
    transition: var(--transition);
  }
  
  .section-header:hover {
    background: var(--bg-hover);
  }
  
  .section-header h3 {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
    margin: 0;
  }
  
  .section-header :global(svg) {
    color: var(--text-muted);
    font-size: 16px;
    transition: transform 0.2s ease;
  }
  
  .settings-section.collapsed .section-header :global(svg) {
    transform: rotate(-180deg);
  }
  
  .section-content {
    padding: 0 var(--space-lg) var(--space-lg);
    animation: slideDown 0.2s ease;
  }
  
  @keyframes slideDown {
    from { opacity: 0; transform: translateY(-8px); }
    to { opacity: 1; transform: translateY(0); }
  }
  
  .setting-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-md) 0;
    gap: var(--space-lg);
    border-bottom: 1px solid var(--border-light);
  }
  
  .setting-row:last-child {
    border-bottom: none;
  }
  
  .setting-row.sub-setting {
    padding-left: var(--space-xl);
    border-left: 2px solid var(--border);
    margin-left: var(--space-md);
  }
  
  .setting-label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }
  
  .setting-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text);
  }
  
  .setting-desc {
    font-size: 13px;
    color: var(--text-secondary);
  }
  
  .setting-control {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }
  
  /* Hotkey Capture Component */
  .hotkey-capture-area {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-width: 160px;
    min-height: 40px;
    padding: 8px 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
    position: relative;
    gap: 8px;
  }

  .hotkey-capture-area:hover {
    background: var(--bg-hover);
    border-color: var(--text-muted);
  }

  .hotkey-capture-area.capturing {
    border-color: var(--accent);
    background: rgba(0, 122, 255, 0.05);
    box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.15);
    cursor: default;
  }

  .hotkey-pills {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1;
    flex-wrap: wrap;
  }

  .hotkey-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 4px 8px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
    text-transform: uppercase;
    letter-spacing: 0.3px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
    min-width: 28px;
    height: 24px;
  }

  .hotkey-pill:global(.modifier) {
    background: rgba(0, 122, 255, 0.1);
    border-color: rgba(0, 122, 255, 0.3);
    color: var(--accent);
  }

  .hotkey-placeholder {
    color: var(--text-muted);
    font-size: 13px;
    flex: 1;
  }

  .hotkey-capture-area :global(.hotkey-edit-icon) {
    font-size: 14px;
    color: var(--text-muted);
  }

  .hotkey-clear,
  .hotkey-cancel {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s;
    padding: 0;
    margin-left: 4px;
  }

  .hotkey-clear:hover {
    background: rgba(255, 59, 48, 0.1);
    color: #FF3B30;
  }

  .hotkey-cancel:hover {
    background: rgba(0, 122, 255, 0.1);
    color: var(--accent);
  }
  
  .segmented-control {
    display: flex;
    gap: 2px;
    padding: 3px;
    background: var(--bg);
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
  }
  
  .segmented-control button {
    padding: 6px 16px;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition);
  }
  
  .segmented-control button.active {
    background: var(--accent);
    color: var(--accent-fg);
  }
  
  .stt-mode-cards {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-md);
    margin-bottom: var(--space-lg);
  }
  
  .stt-mode-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-lg);
    background: var(--bg);
    border: 2px solid var(--border);
    border-radius: var(--radius-lg);
    cursor: pointer;
    transition: var(--transition);
    text-align: center;
  }
  
  .stt-mode-card:hover {
    border-color: var(--text-muted);
    background: var(--bg-hover);
  }
  
  .stt-mode-card.active {
    border-color: var(--accent);
    background: rgba(0, 122, 255, 0.05);
  }
  
  .mode-icon {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: var(--bg-elevated);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    font-size: 24px;
  }
  
  .mode-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  
  .mode-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }
  
  .mode-desc {
    font-size: 12px;
    color: var(--text-secondary);
  }
  
  .api-key-section {
    padding-left: var(--space-xl);
    border-left: 2px solid var(--border);
    margin-left: var(--space-md);
    margin-top: var(--space-md);
  }
  
  .api-key-row {
    display: flex;
    gap: var(--space-sm);
    margin-bottom: var(--space-sm);
  }
  
  .api-key-input {
    flex: 1;
    padding: 10px 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 14px;
  }
  
  .validation-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 13px;
    font-weight: 500;
    padding: 4px 8px;
    border-radius: var(--radius-md);
    background: rgba(255, 59, 48, 0.1);
    color: #FF3B30;
  }
  
  .validation-badge.valid {
    background: rgba(52, 199, 89, 0.1);
    color: #34C759;
  }
  
  .api-key-hint {
    font-size: 13px;
    margin: var(--space-sm) 0 0 0;
  }
  
  .api-key-hint a {
    color: var(--accent);
    text-decoration: none;
  }
  
  .api-key-hint a:hover {
    text-decoration: underline;
  }
  
  .local-models-section {
    padding-left: var(--space-xl);
    border-left: 2px solid var(--border);
    margin-left: var(--space-md);
    margin-top: var(--space-md);
  }
  
  .local-models-hint {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0 0 var(--space-md) 0;
  }
  
  .model-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }
  
  .model-item {
    padding: var(--space-md);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    transition: var(--transition);
  }
  
  .model-item.active {
    border-color: var(--accent);
    background: rgba(0, 122, 255, 0.05);
  }
  
  .model-info-row {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    cursor: pointer;
    margin-bottom: var(--space-sm);
  }
  
  .model-radio {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  
  .model-radio.checked {
    border-color: var(--accent);
    background: var(--accent);
  }
  
  .model-radio.checked::after {
    content: '';
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: white;
  }
  
  .model-details {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  
  .model-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text);
  }
  
  .model-meta {
    font-size: 12px;
    color: var(--text-secondary);
  }
  
  .model-actions {
    display: flex;
    gap: var(--space-sm);
    margin-left: 34px;
  }
  
  .number-input {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  
  .number-input input {
    width: 80px;
    padding: 8px 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 14px;
    text-align: center;
  }
  
  .number-input .unit {
    font-size: 13px;
    color: var(--text-secondary);
  }
  
  .text-input {
    padding: 10px 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 14px;
    min-width: 200px;
  }
  
  .privacy-info {
    display: flex;
    align-items: flex-start;
    gap: var(--space-md);
    padding: var(--space-lg);
    background: rgba(0, 122, 255, 0.05);
    border-radius: var(--radius-md);
    margin-top: var(--space-md);
  }
  
  .privacy-info :global(svg) {
    font-size: 24px;
    color: var(--accent);
    flex-shrink: 0;
  }
  
  .privacy-info p {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.5;
  }
  
  .log-actions {
    display: flex;
    gap: var(--space-md);
    margin-top: var(--space-md);
    padding-top: var(--space-md);
    border-top: 1px solid var(--border-light);
  }
  
  .secondary-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition);
  }

  .secondary-btn:hover {
    background: var(--bg-hover);
    border-color: var(--text-muted);
  }
  
  .secondary-btn.danger {
    color: #FF3B30;
    border-color: rgba(255, 59, 48, 0.3);
  }
  
  .secondary-btn.danger:hover {
    background: rgba(255, 59, 48, 0.1);
    border-color: rgba(255, 59, 48, 0.5);
  }
  
  .secondary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .danger-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-md) 0;
    border-bottom: 1px solid var(--border-light);
  }
  
  .danger-item:last-child {
    border-bottom: none;
  }
  
  .danger-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  
  .danger-title {
    font-size: 14px;
    font-weight: 500;
    color: #FF3B30;
  }
  
  .danger-desc {
    font-size: 13px;
    color: var(--text-secondary);
  }
  
  .danger-btn {
    padding: 8px 16px;
    background: transparent;
    border: 1px solid #FF3B30;
    border-radius: var(--radius-md);
    color: #FF3B30;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition);
  }
  
  .danger-btn:hover {
    background: rgba(255, 59, 48, 0.1);
  }
  
  /* About Tab */
  .about-content {
    display: flex;
    flex-direction: column;
    gap: var(--space-xl);
    padding: var(--space-lg) 0;
  }

  .about-top-section {
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    padding: var(--space-lg) var(--space-xl);
  }

  .about-top-content {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-xl);
    flex-wrap: wrap;
  }

  .version-block {
    display: flex;
    align-items: center;
  }

  .version-label {
    font-size: 15px;
    color: var(--text-secondary);
  }

  .version-label strong {
    color: var(--accent);
    font-weight: 600;
    font-family: ui-monospace, monospace;
  }

  .updates-block {
    display: flex;
    align-items: center;
    gap: var(--space-md);
  }

  .about-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: var(--space-lg);
  }

  .about-grid.two-col {
    grid-template-columns: repeat(2, 1fr);
  }

  .about-card {
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    padding: var(--space-xl);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    transition: transform 0.3s ease, box-shadow 0.3s ease;
  }

  .about-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
  }

  .about-card.highlight {
    background: linear-gradient(to bottom right, var(--bg-elevated), rgba(0, 122, 255, 0.05));
    border-color: rgba(0, 122, 255, 0.3);
  }

  .card-icon {
    width: 40px;
    height: 40px;
    background: var(--bg);
    color: var(--accent);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 22px;
    margin-bottom: 4px;
  }

  .about-card.highlight .card-icon {
    background: var(--accent);
    color: white;
    box-shadow: 0 4px 12px rgba(0, 122, 255, 0.3);
  }

  .about-card h3 {
    font-size: 18px;
    font-weight: 700;
    color: var(--text);
    margin: 0;
  }

  .byline, .card-text {
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-secondary);
    margin: 0;
    flex-grow: 1;
  }

  .byline a {
    color: var(--accent);
    font-weight: 600;
    text-decoration: none;
    border-bottom: 1px solid transparent;
    transition: border-color 0.2s;
  }

  .byline a:hover {
    border-color: var(--accent);
  }

  .action-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    margin-top: auto;
  }

  .action-link {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
    text-decoration: none;
    padding: 10px 16px;
    background: var(--bg);
    border-radius: var(--radius-md);
    transition: all 0.2s ease;
    border: 1px solid var(--border);
    width: 100%;
  }

  .action-link:hover {
    background: var(--bg-hover);
    border-color: var(--text-muted);
  }

  .action-link.secondary {
    background: transparent;
    border-color: var(--border);
    font-weight: 500;
  }

  .action-link.secondary:hover {
    background: var(--bg);
    border-color: var(--text-muted);
  }

  .about-btn-primary {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 12px 20px;
    background: var(--accent);
    color: white;
    font-size: 14px;
    font-weight: 600;
    border-radius: var(--radius-md);
    text-decoration: none;
    transition: all 0.2s ease;
    box-shadow: 0 4px 12px rgba(0, 122, 255, 0.3);
    width: 100%;
  }

  .about-btn-primary:hover {
    background: #0051d5;
    transform: translateY(-2px);
    box-shadow: 0 6px 16px rgba(0, 122, 255, 0.4);
  }

  /* Updates Block (Horizontal in Top Section) */
  .updates-block {
    display: flex;
    align-items: center;
    gap: var(--space-md);
  }

  .updates-block .channel-selector {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .updates-block .channel-selector label {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .channel-select {
    min-width: 180px;
    padding: 8px 32px 8px 12px;
    font-size: 14px;
    font-weight: 500;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    cursor: pointer;
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='%2364748B' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 8px center;
    background-size: 16px;
  }

  .btn-check {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 8px 16px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
    cursor: pointer;
    transition: all 0.2s ease;
    white-space: nowrap;
  }

  .btn-check:hover {
    background: var(--bg-hover);
    border-color: var(--text-muted);
  }

  /* License Section */
  .license-section {
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    overflow: hidden;
  }

  .license-section .accordion {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-lg) var(--space-xl);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: background 0.2s ease;
  }

  .license-section .accordion:hover {
    background: var(--bg-hover);
  }

  .accordion-title {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 16px;
    font-weight: 600;
    color: var(--text);
  }

  .license-content {
    border-top: 1px solid var(--border);
    padding: var(--space-lg) var(--space-xl);
    background: var(--bg);
  }

  .license-text {
    margin: 0;
    font-family: ui-monospace, monospace;
    font-size: 13px;
    line-height: 1.6;
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 300px;
    overflow-y: auto;
    padding-right: 12px;
  }
  
  /* Old Settings Styles (kept for compatibility) */
  .settings-container {
    max-width: 600px;
    display: flex;
    flex-direction: column;
    gap: var(--space-3xl);
  }
  
  .settings-card {
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }
  
  .setting-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
  }
  
  .divider {
    height: 1px;
    background: var(--border-light);
    margin: 0 20px;
  }

  /* Custom Toggle Switch (Apple Style) */
  .toggle-switch {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;
  }

  .toggle-switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .slider {
    position: absolute;
    cursor: pointer;
    top: 0; left: 0; right: 0; bottom: 0;
    background-color: var(--border);
    transition: .3s;
    border-radius: 24px;
  }

  .slider:before {
    position: absolute;
    content: "";
    height: 20px;
    width: 20px;
    left: 2px;
    bottom: 2px;
    background-color: white;
    transition: .3s;
    border-radius: 50%;
    box-shadow: 0 2px 4px rgba(0,0,0,0.2);
  }

  input:checked + .slider {
    background-color: #34C759; /* Apple Green */
  }

  input:checked + .slider:before {
    transform: translateX(20px);
  }

  /* === STATUS BAR === */
  .status-bar-wrap {
    flex-shrink: 0;
    /* Map variables for StatusBar to match sleek theme */
    --bg-card: var(--bg);
    --text-secondary: var(--text-secondary);
    --text-primary: var(--text);
    --text-muted: var(--text-muted);
    --primary: var(--text);
    --border-subtle: var(--border-light);
    --bg-input: var(--bg-elevated);
    --border: var(--border);
    --navy-deep: var(--text);
    border-top: 1px solid var(--border-light);
  }

  /* === RESPONSIVE === */
  @media (max-width: 768px) {
    .sidebar {
      position: fixed;
      left: 0; top: 0; bottom: 0;
      transform: translateX(-100%);
      z-index: 100;
      box-shadow: var(--shadow);
    }
    .dash-columns {
      grid-template-columns: 1fr;
    }
    .page-content {
      padding: var(--space-xl) var(--space-lg);
    }
    .stats-row {
      flex-direction: column;
    }
  }
</style>
