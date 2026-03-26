<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte'
  import { cubicOut } from 'svelte/easing'
  import { slide } from 'svelte/transition'
  import { invoke, listenSafe } from '$lib/backend'
  import Icon from '@iconify/svelte'
  import { initTelemetry, optOut } from '../lib/telemetry'
  import { sidebarDictationStore } from '../lib/sidebarDictation'
  import { LANGUAGE_OPTIONS, languageLabel, isLanguageSupportedByProvider } from '../lib/languages'
  import type {
    AppConfig,
    AudioDevice,
    AudioFilterPreset,
    DictionaryEntry,
    SensitiveAppPattern,
    ThemePreference,
  } from '../types'
  import HotkeyCapture from '../components/HotkeyCapture.svelte'
  import About from './About.svelte'

  let config: AppConfig | null = null
  let audioDevices: AudioDevice[] = []
  let activeTab = 'general'
  let saving = false
  let micLevel = 0
  let testingMic = false
  let levelPollIntervalId: ReturnType<typeof setInterval> | null = null
  let audioCtx: AudioContext | null = null
  let apiKeyValid: boolean | null = null
  let hasApiKey = false
  let apiKeyInput = ''
  let addLanguageCode = ''
  let logEmpty = true
  let logExportMessage: string | null = null
  let saveError: string | null = null
  let appDataPath: string | null = null
  let openFolderError: string | null = null
  let initialLoadDone = false
  let saveDebounceId: ReturnType<typeof setTimeout> | null = null
  let resetting = false
  let resetError: string | null = null

  type ModelRequirement = { can_run: boolean; reason: string | null }
  type ModelStatusEntry = {
    installed: boolean
    size_mb: number
    status: 'NotInstalled' | 'Stopped' | 'Starting' | 'Running' | 'Error'
    error?: string | null
    label?: string
    quality?: string
    languages?: string
    /** Resident set size of the sidecar process (bytes), when running. */
    rss_bytes?: number | null
  }

  const LOCAL_MODEL_IDS = ['sensevoice', 'whisper_base'] as const

  let hardwareReqs: Record<string, ModelRequirement> = {}
  let modelStatuses: Record<string, ModelStatusEntry> = {}
  let statusPollInterval: ReturnType<typeof setInterval> | null = null
  let downloadProgress: Record<string, { percent: number | null; downloaded_bytes: number; total_bytes: number | null }> = {}
  let engineDownloadProgress: Record<string, { percent: number | null; downloaded_bytes: number; total_bytes: number | null }> = {}
  let modelErrors: Record<string, string> = {}
  let unlistenDownloadProgress: (() => void) | null = null
  let unlistenEngineDownloadProgress: (() => void) | null = null
  let sidecarInstalled = false
  let sidecarAvailable: Record<string, boolean> = {}

  function modelIdToSidecarId(modelId: string): string | null {
    if (modelId === 'sensevoice') return 'sherpa-onnx'
    if (modelId === 'whisper_base') return 'whisper-cpp'
    return null
  }

  const tabs = [
    { id: 'general', label: 'General', icon: 'ph:sliders-horizontal' },
    { id: 'dictation', label: 'Audio & Dictation', icon: 'ph:microphone' },
    { id: 'dictionary', label: 'Dictionary', icon: 'ph:book-open' },
    { id: 'command', label: 'Command Mode', icon: 'ph:terminal' },
    { id: 'privacy', label: 'Privacy', icon: 'ph:shield' },
    { id: 'advanced', label: 'Advanced', icon: 'ph:wrench' },
    { id: 'about', label: 'About', icon: 'ph:info' },
  ]

  /** Horizontal tab strip: global CSS hides scrollbars; fades + chevrons hint at sideways overflow. */
  let tabsScrollEl: HTMLDivElement | null = null
  let canScrollTabsLeft = false
  let canScrollTabsRight = false

  function updateTabScrollHints() {
    const el = tabsScrollEl
    if (!el) {
      canScrollTabsLeft = false
      canScrollTabsRight = false
      return
    }
    const { scrollLeft, scrollWidth, clientWidth } = el
    const maxScroll = scrollWidth - clientWidth
    const slop = 2
    canScrollTabsLeft = scrollLeft > slop
    canScrollTabsRight = maxScroll > slop && scrollLeft < maxScroll - slop
  }

  function scrollSettingsTabsBy(delta: number) {
    tabsScrollEl?.scrollBy({ left: delta, behavior: 'smooth' })
  }

  /** Prototype: collapsed = section header only; names aligned with Prototype.svelte. */
  let collapsedSections: Record<string, boolean> = {
    general_hotkeys: false,
    general_startup: true,
    general_appearance: false,
    dictation_audio: false,
    dictation_mode: false,
    dictation_formatting: true,
    dictionary: false,
    command: false,
    privacy_data: false,
    privacy_notifications: false,
    advanced_logs: false,
    advanced_danger: false,
  }

  function recordingModeSegment(): 'Hold' | 'Toggle' | 'Both' {
    if (config?.recording_mode === 'Hold') return 'Hold'
    if (config?.recording_mode === 'Toggle') return 'Toggle'
    return 'Both'
  }

  function setRecordingMode(m: 'Hold' | 'Toggle' | 'Both') {
    if (!config) return
    config.recording_mode = m === 'Both' ? null : m
    scheduleSave()
  }

  /** Default dictation filter (Light); matches Rust `AudioFilterConfig::default()`.
   *  Normalization ensures the VAD threshold works regardless of mic gain. */
  function defaultAudioFilter() {
    return {
      enabled: true,
      preset: 'Light' as AudioFilterPreset,
      highpass_cutoff_hz: 80,
      noise_gate_threshold_db: -45,
      compressor_ratio: 3,
      compressor_threshold_db: -18,
      normalize_target_db: -6,
    }
  }

  function applyAudioFilterPreset(preset: AudioFilterPreset) {
    if (!config) return
    const prev = config.stt_config.audio_filter
    if (preset === 'Off') {
      config = {
        ...config,
        stt_config: {
          ...config.stt_config,
          audio_filter: { ...prev, enabled: false, preset: 'Off' },
        },
      }
      scheduleSave()
      return
    }
    const base = { ...prev, preset, enabled: true }
    if (preset === 'Light') {
      base.highpass_cutoff_hz = 80
      base.noise_gate_threshold_db = -45
      base.compressor_ratio = 3
      base.compressor_threshold_db = -18
      base.normalize_target_db = -6
    } else if (preset === 'Moderate') {
      base.highpass_cutoff_hz = 100
      base.noise_gate_threshold_db = -40
      base.compressor_ratio = 4
      base.compressor_threshold_db = -15
      base.normalize_target_db = -3
    }
    config = { ...config, stt_config: { ...config.stt_config, audio_filter: base } }
    scheduleSave()
  }

  function onAudioFilterPresetSelect(e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value
    if (v === 'Off' || v === 'Light' || v === 'Moderate' || v === 'Custom') {
      applyAudioFilterPreset(v)
    }
  }

  /** User moved a slider; force Custom + enabled so the chain runs with tuned values. */
  function markAudioFilterCustomAndSave() {
    if (!config) return
    const af = config.stt_config.audio_filter
    if (af.preset !== 'Custom' || !af.enabled) {
      config = {
        ...config,
        stt_config: {
          ...config.stt_config,
          audio_filter: { ...af, preset: 'Custom', enabled: true },
        },
      }
    }
    scheduleSave()
  }

  /** Same three states as the sidebar theme control (Auto / Dark / Light). */
  function themeSegment(): ThemePreference {
    const t = config?.theme_preference
    if (t === 'Light' || t === 'Dark' || t === 'Auto') return t
    return 'Auto'
  }

  function setThemePreferenceSetting(next: ThemePreference) {
    if (!config) return
    // Top-level reassignment so `class:active={themeSegment() === …}` invalidates (nested writes don't).
    config = { ...config, theme_preference: next }
    scheduleSave()
  }

  let commandApiKeyInput = ''
  let llmModels: string[] = []
  let loadingLlmModels = false
  let hasCommandApiKey = false

  let dictionaryEntries: DictionaryEntry[] = []
  let dictionaryNewTerm = ''
  let dictionaryLoading = false
  /** `get_platform`: windows | macos | linux — for hotkey Meta label (Win / Cmd / Super). */
  let appPlatform = ''

  function toggleSection(section: string) {
    collapsedSections[section] = !collapsedSections[section]
    collapsedSections = { ...collapsedSections }
  }

  function getCurrentSttProvider(): string {
    return config?.stt_config?.provider || 'groq'
  }

  function getStoredSttApiKey(): string | null {
    if (!config?.stt_config) return null
    const provider = getCurrentSttProvider()
    return config.stt_config.api_keys?.[provider] ?? config.stt_config.api_key ?? null
  }

  onMount(async () => {
    try {
      const [settings, devices, platform, sensevoiceReqs, whisperReqs, sensevoiceAvail, whisperAvail] = await Promise.all([
        invoke('get_settings') as Promise<AppConfig>,
        invoke('get_audio_devices') as Promise<AudioDevice[]>,
        invoke('get_platform') as Promise<string>,
        invoke('check_model_requirements', { modelId: 'sensevoice' }),
        invoke('check_model_requirements', { modelId: 'whisper_base' }),
        invoke('is_sidecar_available_for_model', { modelId: 'sensevoice' }) as Promise<boolean>,
        invoke('is_sidecar_available_for_model', { modelId: 'whisper_base' }) as Promise<boolean>,
      ])

      hardwareReqs['sensevoice'] = sensevoiceReqs as ModelRequirement
      hardwareReqs['whisper_base'] = whisperReqs as ModelRequirement
      sidecarAvailable['sensevoice'] = sensevoiceAvail
      sidecarAvailable['whisper_base'] = whisperAvail

      config = settings
      audioDevices = devices
      appPlatform = platform
      sidebarDictationStore.updateFromConfig(settings, platform)

      if (config) {
        if (config.audio_device == null || config.audio_device === '') {
          config.audio_device = ''
        }
        if (!config.logging) {
          config.logging = { enabled: false, level: 'Info', max_records: 2000 }
        }
        if (!Array.isArray(config.languages) || config.languages.length === 0) {
          config.languages = ['en']
        }
        if (!config.stt_config.local_model) {
          config.stt_config.local_model = 'sensevoice'
        }
        if (!config.stt_config.api_keys) {
          config.stt_config.api_keys = {}
        }
        const sttProvider = config.stt_config.provider || 'groq'
        if (!config.stt_config.provider) {
          config.stt_config.provider = sttProvider
        }
        if (config.stt_config.api_key && !config.stt_config.api_keys[sttProvider]) {
          config.stt_config.api_keys[sttProvider] = config.stt_config.api_key
        }
        if (!config.stt_config.audio_filter) {
          config.stt_config.audio_filter = defaultAudioFilter()
        } else {
          const af = config.stt_config.audio_filter
          // Legacy: any disabled state maps to preset Off in the unified dropdown (numeric fields kept).
          if (!af.enabled && af.preset !== 'Off') {
            config.stt_config.audio_filter = { ...af, preset: 'Off' }
          }
        }
        if (!config.waveform_style) config.waveform_style = 'Aurora'
        if (!config.overlay_position) config.overlay_position = 'BottomCenter'
        if (config.overlay_offset_x == null) config.overlay_offset_x = 0
        if (config.overlay_offset_y == null) config.overlay_offset_y = 0
        if (!config.overlay_expand_direction) config.overlay_expand_direction = 'Up'
        if (!config.command_config) {
          config.command_config = {
            enabled: false,
            hotkey: null,
            provider: null,
            api_keys: {},
            models: {}
          }
        }
        if (config.command_config.provider === undefined) config.command_config.provider = null
        if (config.command_config.hotkey === undefined) config.command_config.hotkey = null
        if (!config.command_config.api_keys) config.command_config.api_keys = {}
        if (!config.command_config.models) config.command_config.models = {}
      }

      if (config?.command_config?.provider) {
        hasCommandApiKey = !!config.command_config.api_keys[config.command_config.provider]
      } else {
        hasCommandApiKey = false
      }

      await checkLogEmpty()

      hasApiKey = !!getStoredSttApiKey()
      apiKeyInput = ''

      await refreshModelStatuses()
      statusPollInterval = setInterval(refreshModelStatuses, 2000)

      try {
        appDataPath = (await invoke('get_app_data_path')) as string
      } catch {
        appDataPath = null
      }

      unlistenDownloadProgress = await listenSafe<{ model_type: string; downloaded_bytes: number; total_bytes: number | null; percent: number | null }>(
        'model-download-progress',
        (e) => {
          const { model_type, downloaded_bytes, total_bytes, percent } = e.payload
          downloadProgress = { ...downloadProgress, [model_type]: { percent: percent ?? null, downloaded_bytes, total_bytes } }
        }
      )
      unlistenEngineDownloadProgress = await listenSafe<{ sidecar_id: string; downloaded_bytes: number; total_bytes: number | null; percent: number | null }>(
        'sidecar-download-progress',
        (e) => {
          const { sidecar_id, downloaded_bytes, total_bytes, percent } = e.payload
          engineDownloadProgress = { ...engineDownloadProgress, [sidecar_id]: { percent: percent ?? null, downloaded_bytes, total_bytes } }
        }
      )

    } catch (e) {
      console.error('Failed to load settings:', e)
    } finally {
      initialLoadDone = true
    }
  })

  onDestroy(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.style.overflow = ''
      document.body.style.overflow = ''
      document.documentElement.classList.remove('sensitive-app-picker-open')
    }
    if (statusPollInterval) clearInterval(statusPollInterval)
    if (unlistenDownloadProgress) unlistenDownloadProgress()
    if (unlistenEngineDownloadProgress) unlistenEngineDownloadProgress()
  })

  function tabsScrollAction(node: HTMLDivElement) {
    tabsScrollEl = node
    let ro: ResizeObserver | null = null
    const onWinResize = () => updateTabScrollHints()

    window.addEventListener('resize', onWinResize)
    node.addEventListener('scroll', updateTabScrollHints, { passive: true })
    ro = new ResizeObserver(updateTabScrollHints)
    ro.observe(node)
    
    // Double-check after a frame to ensure accurate measurements
    requestAnimationFrame(() => {
      updateTabScrollHints()
      // Multiple checks as fonts/layout settle
      setTimeout(updateTabScrollHints, 50)
      setTimeout(updateTabScrollHints, 150)
      setTimeout(updateTabScrollHints, 300)
    })

    return {
      destroy() {
        window.removeEventListener('resize', onWinResize)
        node.removeEventListener('scroll', updateTabScrollHints)
        ro?.disconnect()
        if (tabsScrollEl === node) tabsScrollEl = null
      }
    }
  }

  async function refreshModelStatuses() {
    try {
      const next = await invoke('get_model_status') as Record<string, ModelStatusEntry>
      modelStatuses = next
      let cleared = false
      const nextEngine: typeof engineDownloadProgress = { ...engineDownloadProgress }
      for (const modelId of LOCAL_MODEL_IDS) {
        const sid = modelIdToSidecarId(modelId)
        if (sid && next[modelId]?.status !== 'Starting' && nextEngine[sid]) {
          delete nextEngine[sid]
          cleared = true
        }
      }
      if (cleared) engineDownloadProgress = nextEngine

      if (
        config?.stt_config?.mode === 'Local' ||
        config?.stt_config?.mode === 'Hybrid' ||
        config?.stt_config?.mode === 'Auto'
      ) {
        const localModel = config.stt_config.local_model ?? 'sensevoice'
        try {
          sidecarInstalled = (await invoke('is_sidecar_installed_for_model', { modelId: localModel })) as boolean
        } catch {
          sidecarInstalled = false
        }
      } else {
        sidecarInstalled = false
      }
    } catch (e) {
      console.error('Failed to get model statuses:', e)
    }
  }

  function setModelError(modelId: string, message: string) {
    modelErrors = { ...modelErrors, [modelId]: message }
  }

  function clearModelError(modelId: string) {
    const next = { ...modelErrors }
    delete next[modelId]
    modelErrors = next
  }

  function selectSttModeCard(mode: string) {
    if (!config) return
    if (mode === 'Cloud' || mode === 'Local' || mode === 'Hybrid' || mode === 'Auto') {
      config.stt_config.mode = mode
      void onSttModeChange()
    }
  }

  const STT_MODE_LABELS: Record<string, string> = {
    Cloud: 'Fastest, requires internet',
    Local: 'Private, runs on your device',
    Hybrid: 'Combines both for best results',
    Auto:
      'Cloud when safe; switches to local STT when the focused app matches a pattern under Privacy → Sensitive apps',
  }

  // Sensitive Apps picker state
  type AppListEntry = {
    process_name: string
    display_name: string
    icon_base64?: string | null
    exe_path?: string | null
  }

  let sensitiveAppPickerOpen = false
  let sensitiveAppPickerTab: 'running' | 'installed' | 'browse' = 'running'
  let runningApps: AppListEntry[] = []
  let installedApps: AppListEntry[] = []
  /** User must click "Load installed apps" — scan is slow on some systems. */
  let installedAppsLoaded = false
  let sensitiveAppsLoading = false
  let sensitiveAppsSearch = ''
  /** Match main shell theme when the picker is portaled to `document.body` (outside `.kalam-sleek`). */
  let modalPickerDark = false

  function updateModalTheme() {
    if (typeof document !== 'undefined') {
      const htmlTheme = document.documentElement.getAttribute('data-theme')
      modalPickerDark = htmlTheme === 'dark'
    }
  }

  /** Move overlay to `body` so `position: fixed` is viewport-relative (not trapped by `.page.fade-in` transform). */
  function portalBody(node: HTMLElement) {
    updateModalTheme()
    document.body.appendChild(node)
    return {
      destroy() {
        node.remove()
      },
    }
  }

  $: if (typeof document !== 'undefined') {
    const lock = sensitiveAppPickerOpen ? 'hidden' : ''
    document.documentElement.style.overflow = lock
    document.body.style.overflow = lock
    document.documentElement.classList.toggle('sensitive-app-picker-open', sensitiveAppPickerOpen)
  }

  function ensureSensitivePatternsArray() {
    if (!config) return
    if (!Array.isArray(config.privacy.sensitive_app_patterns)) {
      config.privacy.sensitive_app_patterns = []
    }
  }

  function removeSensitiveAppPattern(index: number) {
    if (!config) return
    ensureSensitivePatternsArray()
    config.privacy.sensitive_app_patterns = config.privacy.sensitive_app_patterns.filter((_, i) => i !== index)
    scheduleSave()
  }

  async function openSensitiveAppPicker() {
    sensitiveAppPickerOpen = true
    sensitiveAppPickerTab = 'running'
    sensitiveAppsSearch = ''
    await tick()
    updateModalTheme()
    void loadRunningApps()
  }

  function closeSensitiveAppPicker() {
    sensitiveAppPickerOpen = false
    runningApps = []
    installedApps = []
    installedAppsLoaded = false
    sensitiveAppsSearch = ''
  }

  async function loadRunningApps() {
    sensitiveAppsLoading = true
    try {
      runningApps = (await invoke('get_running_apps')) as AppListEntry[]
    } catch (e) {
      console.error('Failed to load running apps:', e)
      runningApps = []
    } finally {
      sensitiveAppsLoading = false
    }
  }

  async function loadInstalledApps() {
    sensitiveAppsLoading = true
    try {
      installedApps = (await invoke('get_installed_apps')) as AppListEntry[]
      installedAppsLoaded = true
    } catch (e) {
      console.error('Failed to load installed apps:', e)
      installedApps = []
      installedAppsLoaded = true
    } finally {
      sensitiveAppsLoading = false
    }
  }

  async function pickExecutableFile() {
    try {
      const path = (await invoke('pick_executable_file')) as string | null
      if (path) {
        const processName = path.split(/[/\\]/).pop()?.toLowerCase() ?? ''
        if (processName) {
          addSensitiveAppFromPicker(processName, path)
        }
      }
    } catch (e) {
      console.error('Failed to pick executable:', e)
    }
  }

  function switchSensitivePickerTab(tab: 'running' | 'installed' | 'browse') {
    sensitiveAppPickerTab = tab
    sensitiveAppsSearch = ''
    if (tab === 'running' && runningApps.length === 0) {
      void loadRunningApps()
    }
    // Installed: user triggers load via button (scan can take several seconds).
  }

  function escapeRegex(str: string): string {
    return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  }

  function isAppAlreadyAdded(processName: string): boolean {
    if (!config?.privacy.sensitive_app_patterns) return false
    // Check if any pattern would match this exact process name
    return config.privacy.sensitive_app_patterns.some((p) => {
      if (p.pattern_type !== 'ProcessName') return false
      // Simple check: if pattern is just the name or contains it as a whole word
      const normalized = processName.toLowerCase()
      const pattern = p.pattern.toLowerCase()
      return pattern.includes(normalized) || normalized.includes(pattern.replace(/[^a-z0-9]/g, ''))
    })
  }

  function addSensitiveAppFromPicker(processName: string, exePath?: string | null) {
    if (!config) return
    if (isAppAlreadyAdded(processName)) {
      // Already added - just close picker
      closeSensitiveAppPicker()
      return
    }

    ensureSensitivePatternsArray()

    // Create a regex that matches the process name (case-insensitive, exact match)
    // If it's a simple name like "1password.exe", we create a pattern that matches it exactly
    const baseName = processName.replace(/\.exe$/i, '').replace(/\.app$/i, '')
    const pattern = `(?i)^${escapeRegex(baseName)}(\\.exe)?$`

    const next: SensitiveAppPattern = {
      pattern,
      pattern_type: 'ProcessName',
      action: 'ForceLocal',
    }
    config.privacy.sensitive_app_patterns = [...config.privacy.sensitive_app_patterns, next]
    scheduleSave()
    closeSensitiveAppPicker()
  }

  function getDisplayNameForPattern(pattern: SensitiveAppPattern): string {
    // Try to extract a readable name from the regex pattern
    if (pattern.pattern_type === 'ProcessName') {
      const pl = pattern.pattern.toLowerCase()
      if (
        pl.includes('1password') &&
        pl.includes('bitwarden') &&
        pl.includes('nordpass')
      ) {
        return 'Password managers (default bundle)'
      }
      // Remove regex escaping and look for the core name
      let simplified = pattern.pattern
        .replace(/\(\?i\)/g, '') // Remove case-insensitive flag
        .replace(/\^|\$/g, '') // Remove anchors
        .replace(/\\\./g, '.') // Unescape dots
        .replace(/\\/g, '') // Remove escape chars
        .replace(/\(\.exe\)\?/gi, '') // Remove optional .exe
        .replace(/\.exe$/i, '') // Remove .exe suffix
        .trim()

      // If it looks like a process name, capitalize it
      if (simplified && !simplified.includes('|') && !simplified.includes('(')) {
        return simplified
          .split(/[-_]/)
          .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
          .join(' ')
      }
    }
    return pattern.pattern
  }

  function getPatternIcon(pattern: SensitiveAppPattern): string {
    // Return a relevant icon based on pattern content
    const p = pattern.pattern.toLowerCase()
    if (
      p.includes('password') ||
      p.includes('1password') ||
      p.includes('bitwarden') ||
      p.includes('keepass') ||
      p.includes('lastpass') ||
      p.includes('dashlane') ||
      p.includes('nordpass')
    ) {
      return 'ph:key'
    }
    if (p.includes('bank') || p.includes('finance') || p.includes('crypto') || p.includes('wallet')) {
      return 'ph:currency-dollar'
    }
    if (p.includes('mail') || p.includes('email') || p.includes('outlook') || p.includes('thunderbird')) {
      return 'ph:envelope'
    }
    if (p.includes('chat') || p.includes('slack') || p.includes('teams') || p.includes('discord') || p.includes('signal') || p.includes('telegram') || p.includes('whatsapp')) {
      return 'ph:chat-circle'
    }
    if (p.includes('browser') || p.includes('chrome') || p.includes('firefox') || p.includes('safari') || p.includes('edge')) {
      return 'ph:globe'
    }
    if (p.includes('doc') || p.includes('word') || p.includes('excel') || p.includes('sheet') || p.includes('note')) {
      return 'ph:file-text'
    }
    return 'ph:app-window'
  }

  $: filteredRunningApps = sensitiveAppsSearch
    ? runningApps.filter((a) =>
        a.display_name.toLowerCase().includes(sensitiveAppsSearch.toLowerCase()) ||
        a.process_name.toLowerCase().includes(sensitiveAppsSearch.toLowerCase())
      )
    : runningApps

  $: filteredInstalledApps = sensitiveAppsSearch
    ? installedApps.filter((a) =>
        a.display_name.toLowerCase().includes(sensitiveAppsSearch.toLowerCase()) ||
        a.process_name.toLowerCase().includes(sensitiveAppsSearch.toLowerCase())
      )
    : installedApps

  async function onSttModeChange() {
    await saveSettingsImmediate()
    if (config?.stt_config?.mode === 'Cloud' || config?.stt_config?.mode === 'Hybrid') {
      try {
        await invoke('stop_all_local_models')
        await refreshModelStatuses()
      } catch (e) {
        console.error('Failed to stop local models on mode switch:', e)
      }
    }
  }

  async function onCloudProviderChange() {
    if (!config) return
    apiKeyInput = ''
    apiKeyValid = null
    hasApiKey = !!getStoredSttApiKey()
    await saveSettingsImmediate()
  }

  async function setActiveLocalModel(modelId: string) {
    if (!config) return
    config.stt_config.local_model = modelId
    scheduleSave()
  }

  async function startModel(modelId: string) {
    clearModelError(modelId)
    try {
      await invoke('start_local_model', { modelId })
      await refreshModelStatuses()
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      setModelError(modelId, msg)
    }
  }

  async function stopModel(modelId: string) {
    clearModelError(modelId)
    try {
      await invoke('stop_local_model', { modelId })
      await refreshModelStatuses()
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      setModelError(modelId, msg)
    }
  }

  async function restartModel(modelId: string) {
    clearModelError(modelId)
    try {
      await invoke('restart_local_model', { modelId })
      await refreshModelStatuses()
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      setModelError(modelId, msg)
    }
  }

  async function uninstallEngine(modelId: string) {
    if (!confirm('Uninstall the local STT engine?')) return
    try {
      await invoke('uninstall_sidecar', { modelId })
      await refreshModelStatuses()
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      console.error('Failed to uninstall engine:', e)
      alert(msg)
    }
  }

  async function deleteModel(modelId: string) {
    if (!confirm('Delete this model?')) return
    clearModelError(modelId)
    const installedCount = LOCAL_MODEL_IDS.filter((id) => modelStatuses[id]?.installed).length
    const isLastModel = installedCount === 1
    let shouldUninstallEngine = false
    if (isLastModel && confirm('Also uninstall the local STT engine?')) {
      shouldUninstallEngine = true
    }
    try {
      if (shouldUninstallEngine) {
        await invoke('uninstall_sidecar', { modelId })
      }
      await invoke('delete_local_model', { modelId })
      await refreshModelStatuses()
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      setModelError(modelId, msg)
    }
  }

  async function downloadModel(modelId: string) {
    clearModelError(modelId)
    downloadProgress = { ...downloadProgress, [modelId]: { percent: null, downloaded_bytes: 0, total_bytes: null } }
    try {
      await invoke('download_model', { modelType: modelId })
      await refreshModelStatuses()
      if (sidecarAvailable[modelId] !== false) {
        try {
          await invoke('start_local_model', { modelId })
          await refreshModelStatuses()
        } catch (startErr) {
          const msg = startErr instanceof Error ? startErr.message : String(startErr)
          setModelError(modelId, msg)
        }
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      setModelError(modelId, msg)
    } finally {
      const next = { ...downloadProgress }
      delete next[modelId]
      downloadProgress = next
    }
  }

  function scheduleSave() {
    if (!initialLoadDone || !config) return
    if (saveDebounceId != null) clearTimeout(saveDebounceId)
    saveDebounceId = setTimeout(() => {
      saveDebounceId = null
      saveSettings()
    }, 400)
  }

  async function saveSettingsImmediate() {
    if (!initialLoadDone || !config) return
    if (saveDebounceId != null) {
      clearTimeout(saveDebounceId)
      saveDebounceId = null
    }
    await saveSettings()
  }

  async function saveSettings() {
    if (!config) return
    saving = true

    if (apiKeyInput.trim()) {
      if (!config.stt_config.api_keys) config.stt_config.api_keys = {}
      config.stt_config.api_keys[getCurrentSttProvider()] = apiKeyInput.trim()
    }
    if (config.command_config && config.command_config.provider && commandApiKeyInput.trim()) {
      if (!config.command_config.api_keys) config.command_config.api_keys = {}
      config.command_config.api_keys[config.command_config.provider] = commandApiKeyInput.trim()
    }
    if (config.logging) {
      config.logging.max_records = Math.min(20000, Math.max(500, config.logging.max_records || 2000))
    }
    if (config.language_toggle_hotkey === '') config.language_toggle_hotkey = null
    if (config.command_config) {
      if ((config.command_config.provider as unknown as string) === '') config.command_config.provider = null
    }
    if (!Array.isArray(config.languages) || config.languages.length === 0) config.languages = ['en']
    if (!Array.isArray(config.privacy.sensitive_app_patterns)) config.privacy.sensitive_app_patterns = []

    saveError = null
    try {
      await invoke('save_settings', { newConfig: config })
      const platform = (await invoke('get_platform')) as string
      appPlatform = platform
      sidebarDictationStore.updateFromConfig(config, platform)
      if (config.privacy.telemetry_enabled) {
        initTelemetry(true)
      } else {
        optOut()
      }
      hasApiKey = !!getStoredSttApiKey()
      apiKeyInput = ''
      if (config.command_config?.provider) {
        hasCommandApiKey = !!config.command_config.api_keys?.[config.command_config.provider]
      } else {
        hasCommandApiKey = false
      }
      commandApiKeyInput = ''
      if (activeTab === 'advanced') {
        await checkLogEmpty()
      }
    } catch (e) {
      console.error('Failed to save settings:', e)
      const err = e as Error & { message?: string }
      saveError = err?.message ?? String(e)
    } finally {
      saving = false
    }
  }

  async function checkLogEmpty() {
    try {
      logEmpty = await invoke('get_app_log_empty') as boolean
    } catch {
      logEmpty = true
    }
  }

  async function downloadLog() {
    logExportMessage = null
    try {
      await invoke('save_log_to_file')
      logExportMessage = null
      await checkLogEmpty()
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      logExportMessage = msg === 'Save cancelled' ? null : msg
    }
  }

  async function downloadLogsCsv() {
    logExportMessage = null
    try {
      await invoke('save_logs_csv_to_file')
      logExportMessage = null
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      logExportMessage = msg === 'Save cancelled' ? null : msg
    }
  }

  async function openAppDataFolder() {
    openFolderError = null
    try {
      await invoke('open_app_data_folder')
    } catch (e) {
      const err = e as Error & { message?: string }
      openFolderError = err?.message ?? String(e)
    }
  }

  async function confirmAndReset() {
    if (!confirm('Reset the entire application? This will delete all settings, history, and data.')) return
    resetError = null
    resetting = true
    try {
      await invoke('reset_application')
    } catch (e) {
      const err = e as Error & { message?: string }
      resetError = err?.message ?? String(e)
    } finally {
      resetting = false
    }
  }

  async function playTestAudio(samples: number[], sampleRate: number) {
    if (!samples.length) return
    const ctx = audioCtx ?? new (window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext)()
    try {
      if (ctx.state === 'suspended') await ctx.resume()
      const buffer = ctx.createBuffer(1, samples.length, sampleRate)
      buffer.getChannelData(0).set(new Float32Array(samples))
      const source = ctx.createBufferSource()
      source.buffer = buffer
      source.connect(ctx.destination)
      source.start(0)
    } catch (e) {
      console.error('Playback failed:', e)
    }
  }

  async function startTestRecording() {
    try {
      audioCtx = new (window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext)()
      await audioCtx.resume()
      await invoke('test_microphone_start')
      micLevel = 0
      testingMic = true
      levelPollIntervalId = setInterval(async () => {
        try {
          const level = await invoke('test_microphone_level') as number
          micLevel = level
        } catch {
          // ignore
        }
      }, 100)
    } catch (e) {
      console.error('Failed to start test:', e)
    }
  }

  async function stopTestRecording() {
    if (!testingMic) return
    if (levelPollIntervalId != null) {
      clearInterval(levelPollIntervalId)
      levelPollIntervalId = null
    }
    try {
      const result = await invoke('test_microphone_stop') as { level: number; samples: number[]; sample_rate: number }
      micLevel = result.level
      if (result.samples?.length && result.sample_rate) {
        await playTestAudio(result.samples, result.sample_rate)
      }
    } catch (e) {
      micLevel = 0
    } finally {
      testingMic = false
    }
  }

  async function checkApiKey() {
    const keyToCheck = apiKeyInput.trim() || getStoredSttApiKey()
    if (!keyToCheck) return
    try {
      apiKeyValid = await invoke('check_api_key', {
        provider: config?.stt_config?.provider || 'groq',
        apiKey: keyToCheck
      })
    } catch (e) {
      apiKeyValid = false
    }
  }

  function clearApiKey() {
    if (config) {
      const provider = getCurrentSttProvider()
      if (!config.stt_config.api_keys) config.stt_config.api_keys = {}
      delete config.stt_config.api_keys[provider]
      config.stt_config.api_key = null
      hasApiKey = false
      apiKeyInput = ''
      scheduleSave()
    }
  }

  function refreshAudioDevices() {
    invoke('get_audio_devices')
      .then((devices) => {
        audioDevices = devices as AudioDevice[]
      })
      .catch((e) => {
        console.error('Failed to refresh audio devices:', e)
      })
  }

  function moveLanguageUp(index: number) {
    if (!config || index <= 0) return
    const langs = [...config.languages]
    ;[langs[index - 1], langs[index]] = [langs[index], langs[index - 1]]
    config = { ...config, languages: langs }
    scheduleSave()
  }

  function moveLanguageDown(index: number) {
    if (!config || index >= config.languages.length - 1) return
    const langs = [...config.languages]
    ;[langs[index], langs[index + 1]] = [langs[index + 1], langs[index]]
    config = { ...config, languages: langs }
    scheduleSave()
  }

  function removeLanguage(index: number) {
    if (!config || config.languages.length <= 1) return
    config = { ...config, languages: config.languages.filter((_, j) => j !== index) }
    scheduleSave()
  }

  function addSelectedLanguage() {
    if (!config || !addLanguageCode || config.languages.includes(addLanguageCode)) return
    config = { ...config, languages: [...config.languages, addLanguageCode] }
    addLanguageCode = ''
    scheduleSave()
  }

  function setHotkey(hotkey: string) {
    if (config) {
      config = { ...config, hotkey: hotkey === '' ? null : hotkey }
      scheduleSave()
    }
  }

  function setToggleHotkey(hotkey: string) {
    if (config) {
      config = { ...config, toggle_dictation_hotkey: hotkey === '' ? null : hotkey }
      scheduleSave()
    }
  }

  function setLanguageToggleHotkey(hotkey: string) {
    if (config) {
      config = { ...config, language_toggle_hotkey: hotkey === '' ? null : hotkey }
      scheduleSave()
    }
  }

  function setCommandProvider(e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value
    if (config && config.command_config) {
      config.command_config.provider = v ? (v as import('../types').CommandModeProvider) : null

      const provider = config.command_config.provider
      if (provider) {
        commandApiKeyInput = ''
        hasCommandApiKey = !!config.command_config.api_keys?.[provider]
      } else {
        commandApiKeyInput = ''
        hasCommandApiKey = false
      }

      commandApiKeyStatus = 'idle'
      commandApiKeyError = null
      commandModelStatus = 'idle'
      commandModelError = null
      llmModels = []
      scheduleSave()
    }
  }

  function setCommandHotkey(hotkey: string) {
    if (config?.command_config) {
      config = {
        ...config,
        command_config: { ...config.command_config, hotkey: hotkey === '' ? null : hotkey }
      }
      scheduleSave()
    }
  }

  let commandApiKeyStatus: 'idle' | 'testing' | 'valid' | 'invalid' = 'idle'
  let commandApiKeyError: string | null = null
  let testingModel = false
  let commandModelStatus: 'idle' | 'testing' | 'valid' | 'invalid' = 'idle'
  let commandModelError: string | null = null

  let commandModelInputText = ''
  let commandModelDropdownOpen = false
  let commandModelTestTimeout: ReturnType<typeof setTimeout> | null = null
  let comboboxEl: HTMLElement | null = null
  let dropdownTop = 0
  let dropdownLeft = 0
  let dropdownWidth = 0

  function updateDropdownPosition() {
    if (comboboxEl) {
      const rect = comboboxEl.getBoundingClientRect()
      dropdownTop = rect.bottom + 8
      dropdownLeft = rect.left
      dropdownWidth = rect.width
    }
  }

  function openDropdown() {
    updateDropdownPosition()
    commandModelDropdownOpen = true
    window.addEventListener('scroll', updateDropdownPosition, true)
    window.addEventListener('resize', updateDropdownPosition)
  }

  function closeDropdown() {
    setTimeout(() => {
      commandModelDropdownOpen = false
      window.removeEventListener('scroll', updateDropdownPosition, true)
      window.removeEventListener('resize', updateDropdownPosition)
    }, 150)
  }

  $: if (config?.command_config?.provider) {
    const savedModel = config.command_config.models?.[config.command_config.provider] ?? ''
    if (!commandModelDropdownOpen && commandModelInputText !== savedModel) {
      commandModelInputText = savedModel
    }
  }

  $: if (activeTab === 'command' && config?.command_config?.provider) {
    const provider = config.command_config.provider;
    const hasSavedKey = !!config.command_config.api_keys?.[provider];
    const isTypingNewKey = commandApiKeyInput.trim().length > 0;

    if (hasSavedKey && !isTypingNewKey && commandApiKeyStatus === 'idle' && !loadingLlmModels) {
      (async () => {
        await fetchCommandLlmModels();
        if (config?.command_config) {
          const savedModel = config.command_config.models?.[provider];
          if (savedModel && commandModelStatus === 'idle') {
            await testCommandModel(savedModel);
          }
        }
      })();
    }
  }

  let filteredModels: string[] = []
  $: {
    const search = commandModelInputText.toLowerCase()
    filteredModels = llmModels.filter(m => m.toLowerCase().includes(search))
  }

  function updateCommandModel(v: string) {
    if (config && config.command_config && config.command_config.provider) {
      if (!config.command_config.models) config.command_config.models = {}
      if (v) {
        config.command_config.models[config.command_config.provider] = v
      } else {
        delete config.command_config.models[config.command_config.provider]
      }
      scheduleSave()

      if (commandModelTestTimeout) clearTimeout(commandModelTestTimeout)
      if (v) {
        commandModelStatus = 'testing'
        commandModelTestTimeout = setTimeout(() => {
          testCommandModel(v)
        }, 800)
      } else {
        commandModelStatus = 'idle'
        commandModelError = null
      }
    }
  }

  function handleModelInput(e: Event) {
    const v = (e.currentTarget as HTMLInputElement).value
    commandModelInputText = v
    if (!commandModelDropdownOpen) openDropdown()
    updateCommandModel(v)
  }

  function selectModelFromDropdown(m: string) {
    commandModelInputText = m
    commandModelDropdownOpen = false
    updateCommandModel(m)
  }

  function clearCommandApiKey() {
    if (config && config.command_config && config.command_config.provider) {
      delete config.command_config.api_keys[config.command_config.provider]
      hasCommandApiKey = false
      commandApiKeyInput = ''
      commandApiKeyStatus = 'idle'
      commandApiKeyError = null
      llmModels = []
      scheduleSave()
    }
  }

  async function fetchCommandLlmModels() {
    if (!config?.command_config) return
    const provider = config.command_config.provider ?? 'groq'
    const apiKey = (commandApiKeyInput.trim() || config.command_config.api_keys?.[provider]) ?? ''
    if (!apiKey) return
    loadingLlmModels = true
    commandApiKeyStatus = 'testing'
    commandApiKeyError = null
    try {
      llmModels = await invoke('fetch_llm_models', { provider, apiKey }) as string[]
      commandApiKeyStatus = 'valid'
    } catch (e) {
      llmModels = []
      commandApiKeyStatus = 'invalid'
      commandApiKeyError = String(e)
    } finally {
      loadingLlmModels = false
    }
  }

  async function testCommandModel(model: string) {
    if (!config?.command_config) return
    const provider = config.command_config.provider ?? 'groq'
    const apiKey = (commandApiKeyInput.trim() || config.command_config.api_keys?.[provider]) ?? ''
    if (!apiKey || !model) return

    testingModel = true
    commandModelStatus = 'testing'
    commandModelError = null
    try {
      await invoke('test_llm_model', { provider, apiKey, model })
      commandModelStatus = 'valid'
    } catch (e) {
      commandModelStatus = 'invalid'
      commandModelError = String(e)
    } finally {
      testingModel = false
    }
  }

  async function loadDictionaryEntries() {
    dictionaryLoading = true
    try {
      dictionaryEntries = (await invoke('get_dictionary_entries')) as DictionaryEntry[]
    } catch (e) {
      dictionaryEntries = []
    } finally {
      dictionaryLoading = false
    }
  }

  async function addDictionaryTerm() {
    const term = dictionaryNewTerm.trim()
    if (!term) return
    try {
      await invoke('add_dictionary_entry', { term })
      dictionaryNewTerm = ''
      await loadDictionaryEntries()
    } catch (e) {
      console.error('Failed to add dictionary term:', e)
    }
  }

  async function deleteDictionaryEntry(id: string) {
    try {
      await invoke('delete_dictionary_entry', { id })
      await loadDictionaryEntries()
    } catch (e) {
      console.error('Failed to delete dictionary entry:', e)
    }
  }

  $: langProviderKey = config
    ? (
        config.stt_config.mode === 'Local'
          ? (config.stt_config.local_model || 'sensevoice')
          : (config.stt_config.provider || 'groq')
      )
    : 'groq'
  $: hasApiKey = !!getStoredSttApiKey()
</script>

{#if config}
  <div class="page fade-in settings-page">
    <header class="page-header settings-header">
      <h1 class="page-title">Settings</h1>
      {#if saving}
        <span class="save-status">Saving…</span>
      {:else if saveError}
        <span class="save-status error">Save failed</span>
      {/if}
    </header>

    {#if saveError}
      <p class="save-error" role="alert">{saveError}</p>
    {/if}

    <div class="settings-tabs-shell">
      {#if canScrollTabsLeft}
        <button
          type="button"
          class="settings-tabs-edge-btn settings-tabs-edge-btn--left"
          aria-label="Scroll settings tabs left"
          on:click={() => scrollSettingsTabsBy(-Math.min(280, (tabsScrollEl?.clientWidth ?? 280) * 0.85))}
        >
          <Icon icon="ph:caret-left" />
        </button>
      {/if}
      {#if canScrollTabsRight}
        <button
          type="button"
          class="settings-tabs-edge-btn settings-tabs-edge-btn--right"
          aria-label="Scroll settings tabs right"
          on:click={() => scrollSettingsTabsBy(Math.min(280, (tabsScrollEl?.clientWidth ?? 280) * 0.85))}
        >
          <Icon icon="ph:caret-right" />
        </button>
      {/if}
      <div class="settings-tabs-fade settings-tabs-fade--left" class:visible={canScrollTabsLeft} aria-hidden="true"></div>
      <div class="settings-tabs-fade settings-tabs-fade--right" class:visible={canScrollTabsRight} aria-hidden="true"></div>
      <div
        class="settings-tabs"
        use:tabsScrollAction
        role="tablist"
        aria-label="Settings sections"
      >
        {#each tabs as tab}
          <button
            type="button"
            class="settings-tab"
            class:active={activeTab === tab.id}
            role="tab"
            aria-selected={activeTab === tab.id}
            id="settings-tab-{tab.id}"
            on:click={() => {
              activeTab = tab.id
              if (tab.id !== 'advanced') logExportMessage = null
              if (tab.id === 'advanced') checkLogEmpty()
              if (tab.id === 'dictionary') loadDictionaryEntries()
              tick().then(updateTabScrollHints)
            }}
          >
            <Icon icon={tab.icon} />
            <span>{tab.label}</span>
          </button>
        {/each}
      </div>
    </div>

    <div class="settings-content">
      {#if activeTab === 'general'}
        <div class="settings-tab-content">
          <section class="settings-section" class:collapsed={collapsedSections.general_hotkeys}>
            <button type="button" class="section-header" on:click={() => toggleSection('general_hotkeys')}>
              <h3>Dictation Hotkeys</h3>
              <Icon icon={collapsedSections.general_hotkeys ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.general_hotkeys}
              <!-- accordion-animated: override .settings-section.collapsed .section-content { display:none } during Svelte slide outro -->
              <div
                class="section-content accordion-animated"
                transition:slide={{ duration: 180, easing: cubicOut, axis: 'y' }}
              >
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Hold to Dictate</span>
                    <span class="setting-desc">Press and hold to start dictating</span>
                  </div>
                  <div class="setting-control">
                    <HotkeyCapture value={config.hotkey ?? ''} platform={appPlatform} onChange={setHotkey} />
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Toggle Dictation</span>
                    <span class="setting-desc">Press to start/stop dictating</span>
                  </div>
                  <div class="setting-control">
                    <HotkeyCapture value={config.toggle_dictation_hotkey ?? ''} platform={appPlatform} onChange={setToggleHotkey} />
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Toggle Language</span>
                    <span class="setting-desc">Switch between recognition languages</span>
                  </div>
                  <div class="setting-control">
                    <HotkeyCapture value={config.language_toggle_hotkey ?? ''} platform={appPlatform} onChange={setLanguageToggleHotkey} />
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Command Mode</span>
                    <span class="setting-desc">Create notes, tasks, and reminders by voice</span>
                  </div>
                  <div class="setting-control">
                    <HotkeyCapture value={config.command_config?.hotkey ?? ''} platform={appPlatform} onChange={setCommandHotkey} />
                  </div>
                </div>
              </div>
            {/if}
          </section>

          <section class="settings-section" class:collapsed={collapsedSections.general_startup}>
            <button type="button" class="section-header" on:click={() => toggleSection('general_startup')}>
              <h3>Startup &amp; Behavior</h3>
              <Icon icon={collapsedSections.general_startup ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.general_startup}
              <div class="section-content">
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Launch at Login</span>
                    <span class="setting-desc">Start Kalam automatically when you log in</span>
                  </div>
                  <div class="setting-control">
                    <label class="toggle-switch">
                      <input type="checkbox" bind:checked={config.auto_start} on:change={scheduleSave} />
                      <span class="slider"></span>
                    </label>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Show Window on Startup</span>
                    <span class="setting-desc">Open the main window when the app starts; otherwise start in the tray</span>
                  </div>
                  <div class="setting-control">
                    <label class="toggle-switch">
                      <input type="checkbox" bind:checked={config.start_in_focus} on:change={scheduleSave} />
                      <span class="slider"></span>
                    </label>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Recording Mode</span>
                    <span class="setting-desc">Which dictation hotkeys are active (both can still be configured above)</span>
                  </div>
                  <div class="setting-control">
                    <div class="segmented-control">
                      <button
                        type="button"
                        class:active={recordingModeSegment() === 'Hold'}
                        on:click={() => setRecordingMode('Hold')}
                      >Hold</button>
                      <button
                        type="button"
                        class:active={recordingModeSegment() === 'Toggle'}
                        on:click={() => setRecordingMode('Toggle')}
                      >Toggle</button>
                      <button
                        type="button"
                        class:active={recordingModeSegment() === 'Both'}
                        on:click={() => setRecordingMode('Both')}
                      >Both</button>
                    </div>
                  </div>
                </div>
              </div>
            {/if}
          </section>

          <section class="settings-section" class:collapsed={collapsedSections.general_appearance}>
            <button type="button" class="section-header" on:click={() => toggleSection('general_appearance')}>
              <h3>Appearance</h3>
              <Icon icon={collapsedSections.general_appearance ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.general_appearance}
              <div class="section-content">
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Theme</span>
                    <span class="setting-desc">Auto follows your system; Dark and Light stay fixed</span>
                  </div>
                  <div class="setting-control">
                    <div class="segmented-control">
                      <button
                        type="button"
                        class:active={themeSegment() === 'Auto'}
                        on:click={() => setThemePreferenceSetting('Auto')}
                      >Auto</button>
                      <button
                        type="button"
                        class:active={themeSegment() === 'Dark'}
                        on:click={() => setThemePreferenceSetting('Dark')}
                      >Dark</button>
                      <button
                        type="button"
                        class:active={themeSegment() === 'Light'}
                        on:click={() => setThemePreferenceSetting('Light')}
                      >Light</button>
                    </div>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Waveform Style</span>
                    <span class="setting-desc">Visual style of the recording indicator</span>
                  </div>
                  <div class="setting-control">
                    <select class="form-select" bind:value={config.waveform_style} on:change={scheduleSave}>
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
                    <span class="setting-name">Expand Direction</span>
                    <span class="setting-desc">Which direction the overlay pill expands</span>
                  </div>
                  <div class="setting-control">
                    <select class="form-select" bind:value={config.overlay_expand_direction} on:change={scheduleSave}>
                      <option value="Up">Upwards</option>
                      <option value="Down">Downwards</option>
                      <option value="Center">Center</option>
                    </select>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Overlay Position</span>
                    <span class="setting-desc">Where to show the recording overlay</span>
                  </div>
                  <div class="setting-control">
                    <select class="form-select" bind:value={config.overlay_position} on:change={scheduleSave}>
                      <option value="BottomCenter">Bottom Center</option>
                      <option value="BottomLeft">Bottom Left</option>
                      <option value="BottomRight">Bottom Right</option>
                      <option value="TopCenter">Top Center</option>
                      <option value="TopLeft">Top Left</option>
                      <option value="TopRight">Top Right</option>
                      <option value="CenterLeft">Center Left</option>
                      <option value="CenterRight">Center Right</option>
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
                      <input type="number" bind:value={config.overlay_offset_x} step="10" on:input={scheduleSave} />
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
                      <input type="number" bind:value={config.overlay_offset_y} step="10" on:input={scheduleSave} />
                      <span class="unit">px</span>
                    </div>
                  </div>
                </div>
              </div>
            {/if}
          </section>
        </div>

      {:else if activeTab === 'dictation'}
        <div class="settings-tab-content">
          <section class="settings-section" class:collapsed={collapsedSections.dictation_audio}>
            <button type="button" class="section-header" on:click={() => toggleSection('dictation_audio')}>
              <h3>Audio Input</h3>
              <Icon icon={collapsedSections.dictation_audio ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.dictation_audio}
              <div class="section-content">
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Microphone</span>
                    <span class="setting-desc">Select your audio input device</span>
                  </div>
                  <div class="setting-control">
                    <button type="button" class="btn-refresh" on:click={refreshAudioDevices} title="Refresh devices">
                      <Icon icon="ph:arrow-clockwise" />
                    </button>
                    <select class="form-select" bind:value={config.audio_device} on:change={scheduleSave}>
                      {#if audioDevices.length === 0}
                        <option value="">No devices found</option>
                      {:else}
                        <option value="">System default</option>
                        {#each audioDevices as device}
                          <option value={device.id}>
                            {device.is_default ? 'Default — ' + device.name : device.name}
                          </option>
                        {/each}
                      {/if}
                    </select>
                  </div>
                </div>
                {#if audioDevices.length === 0}
                  <p class="hint warning">No audio devices found. Try refreshing.</p>
                {/if}

                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Minimum Hold Time</span>
                    <span class="setting-desc">Minimum milliseconds to hold the dictation hotkey</span>
                  </div>
                  <div class="setting-control">
                    <div class="number-input">
                      <input type="number" bind:value={config.min_hold_ms} min="0" max="2000" step="50" on:change={scheduleSave} />
                      <span class="unit">ms</span>
                    </div>
                  </div>
                </div>

                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Test Microphone</span>
                    <span class="setting-desc">Record a sample and hear playback</span>
                  </div>
                  <div class="setting-control">
                    {#if testingMic}
                      <button type="button" class="settings-secondary-btn" on:click={stopTestRecording}>Stop</button>
                    {:else}
                      <button type="button" class="settings-secondary-btn" on:click={startTestRecording}>Start</button>
                    {/if}
                  </div>
                </div>
                <div class="mic-level-container">
                  <div class="mic-level" role="meter" aria-valuenow={Math.round(micLevel * 100)}>
                    <div class="mic-bar" style="width: {micLevel * 100}%"></div>
                  </div>
                  {#if testingMic}
                    <span class="mic-status">Recording…</span>
                  {:else if micLevel > 0}
                    <span class="mic-status">Volume: {Math.round(micLevel * 100)}%</span>
                  {/if}
                </div>

                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">VAD Sensitivity</span>
                    <span class="setting-desc">Voice activity detection (silence before end)</span>
                  </div>
                  <div class="setting-control">
                    <select class="form-select" bind:value={config.stt_config.vad_preset} on:change={scheduleSave}>
                      <option value="Fast">Fast (0.8s silence)</option>
                      <option value="Balanced">Balanced (1.5s silence)</option>
                      <option value="Accurate">Accurate (2.5s silence)</option>
                    </select>
                  </div>
                </div>

                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Audio filter</span>
                    <span class="setting-desc"
                      >High-pass, noise gate, light compression, and normalize — same chain as dictation and mic test
                      playback. Off disables processing.</span
                    >
                  </div>
                  <div class="setting-control">
                    <select
                      class="form-select"
                      value={config.stt_config.audio_filter.preset}
                      on:change={onAudioFilterPresetSelect}
                    >
                      <option value="Off">Off</option>
                      <option value="Light">Light</option>
                      <option value="Moderate">Moderate</option>
                      <option value="Custom">Custom</option>
                    </select>
                  </div>
                </div>

                {#if config.stt_config.audio_filter.preset === 'Custom'}
                    <div class="setting-row audio-filter-range-row">
                      <div class="setting-label">
                        <span class="setting-name">High-pass cutoff</span>
                        <span class="setting-desc">{Math.round(config.stt_config.audio_filter.highpass_cutoff_hz)} Hz</span>
                      </div>
                      <div class="setting-control">
                        <input
                          type="range"
                          min="40"
                          max="200"
                          step="1"
                          bind:value={config.stt_config.audio_filter.highpass_cutoff_hz}
                          on:change={markAudioFilterCustomAndSave}
                        />
                      </div>
                    </div>
                    <div class="setting-row audio-filter-range-row">
                      <div class="setting-label">
                        <span class="setting-name">Noise gate</span>
                        <span class="setting-desc"
                          >{Math.round(config.stt_config.audio_filter.noise_gate_threshold_db)} dB</span
                        >
                      </div>
                      <div class="setting-control">
                        <input
                          type="range"
                          min="-60"
                          max="-20"
                          step="1"
                          bind:value={config.stt_config.audio_filter.noise_gate_threshold_db}
                          on:change={markAudioFilterCustomAndSave}
                        />
                      </div>
                    </div>
                    <div class="setting-row audio-filter-range-row">
                      <div class="setting-label">
                        <span class="setting-name">Compressor ratio</span>
                        <span class="setting-desc">{config.stt_config.audio_filter.compressor_ratio.toFixed(1)}:1</span>
                      </div>
                      <div class="setting-control">
                        <input
                          type="range"
                          min="1.5"
                          max="6"
                          step="0.1"
                          bind:value={config.stt_config.audio_filter.compressor_ratio}
                          on:change={markAudioFilterCustomAndSave}
                        />
                      </div>
                    </div>
                    <div class="setting-row audio-filter-range-row">
                      <div class="setting-label">
                        <span class="setting-name">Normalize target</span>
                        <span class="setting-desc"
                          >{Math.round(config.stt_config.audio_filter.normalize_target_db)} dBFS peak</span
                        >
                      </div>
                      <div class="setting-control">
                        <input
                          type="range"
                          min="-12"
                          max="-1"
                          step="1"
                          bind:value={config.stt_config.audio_filter.normalize_target_db}
                          on:change={markAudioFilterCustomAndSave}
                        />
                      </div>
                    </div>
                    <div class="setting-row audio-filter-range-row">
                      <div class="setting-label">
                        <span class="setting-name">Compressor threshold</span>
                        <span class="setting-desc"
                          >{Math.round(config.stt_config.audio_filter.compressor_threshold_db)} dB</span
                        >
                      </div>
                      <div class="setting-control">
                        <input
                          type="range"
                          min="-40"
                          max="-6"
                          step="1"
                          bind:value={config.stt_config.audio_filter.compressor_threshold_db}
                          on:change={markAudioFilterCustomAndSave}
                        />
                      </div>
                    </div>
                  {/if}
              </div>
            {/if}
          </section>

          <section class="settings-section" class:collapsed={collapsedSections.dictation_mode}>
            <button type="button" class="section-header" on:click={() => toggleSection('dictation_mode')}>
              <h3>Speech-to-Text Mode</h3>
              <Icon icon={collapsedSections.dictation_mode ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.dictation_mode}
              <div class="section-content">
                <div
                  class="stt-mode-cards stt-mode-row"
                  role="radiogroup"
                  aria-label="Speech-to-text mode"
                >
                  {#each ['Auto', 'Hybrid', 'Cloud', 'Local'] as mode}
                    <button
                      type="button"
                      class="stt-mode-card"
                      class:active={config.stt_config.mode === mode}
                      role="radio"
                      aria-checked={config.stt_config.mode === mode}
                      title={STT_MODE_LABELS[mode] ?? mode}
                      on:click={() => selectSttModeCard(mode)}
                    >
                      <div class="mode-icon">
                        <Icon
                          icon={mode === 'Cloud'
                            ? 'ph:cloud'
                            : mode === 'Local'
                              ? 'ph:hard-drives'
                              : mode === 'Hybrid'
                                ? 'ph:arrows-left-right'
                                : 'ph:magic-wand'}
                        />
                      </div>
                      <span class="mode-name">{mode}</span>
                    </button>
                  {/each}
                </div>
                <p class="stt-mode-selected-hint">
                  {STT_MODE_LABELS[config.stt_config.mode] ?? ''}
                </p>

                {#if config.stt_config.mode === 'Cloud' || config.stt_config.mode === 'Hybrid' || config.stt_config.mode === 'Auto'}
                  <div class="stt-cloud-group">
                    <div class="setting-row">
                      <div class="setting-label">
                        <span class="setting-name">Cloud Provider</span>
                        <span class="setting-desc">API service for transcription</span>
                      </div>
                      <div class="setting-control">
                        <select class="form-select" bind:value={config.stt_config.provider} on:change={onCloudProviderChange}>
                          <option value="groq">Groq (whisper-large-v3-turbo)</option>
                          <option value="openai">OpenAI (whisper-1)</option>
                        </select>
                      </div>
                    </div>

                    <div class="setting-row">
                      <div class="setting-label">
                        <span class="setting-name">API key</span>
                        {#if apiKeyValid !== null}
                          <span class="validation-badge" class:valid={apiKeyValid}>
                            {apiKeyValid ? '✓ Valid' : '✗ Invalid'}
                          </span>
                        {:else if hasApiKey && !apiKeyInput}
                          <span class="badge configured">✓ Configured</span>
                        {:else if !hasApiKey}
                          <span class="badge muted">Not set</span>
                        {/if}
                      </div>
                      <div class="setting-control full-width">
                        <div class="api-key-row">
                          <input
                            type="password"
                            class="api-key-input"
                            bind:value={apiKeyInput}
                            on:input={scheduleSave}
                            placeholder={hasApiKey ? 'Enter new key to change' : 'Enter API key...'}
                            aria-label={hasApiKey ? 'Replace API key' : 'API key'}
                          />
                          <button type="button" class="settings-secondary-btn" on:click={checkApiKey}>Validate</button>
                          {#if hasApiKey && !apiKeyInput}
                            <button type="button" class="settings-secondary-btn danger" on:click={clearApiKey}>Clear</button>
                          {/if}
                        </div>
                      </div>
                    </div>
                    <p class="api-key-hint">
                      {#if config.stt_config.provider === 'openai'}
                        <a href="https://platform.openai.com/api-keys" target="_blank" rel="noopener noreferrer">Get your API key from OpenAI →</a>
                      {:else}
                        <a href="https://console.groq.com" target="_blank" rel="noopener noreferrer">Get your API key from Groq →</a>
                      {/if}
                    </p>
                  </div>
                {/if}

            {#if config.stt_config.mode === 'Local' || config.stt_config.mode === 'Hybrid' || config.stt_config.mode === 'Auto'}
              <div class="local-models-section">
                <p class="local-models-hint">
                  Select one model; it is used when mode is Local. Download, start, or stop from the list.
                </p>
              <div class="setting-row local-model-row">
                <div class="setting-label full-width">
                  <span class="setting-name">Local Model</span>
                  <span class="setting-desc">Select and manage local STT models</span>

                  <div class="model-list">
                    {#each LOCAL_MODEL_IDS as modelId}
                      {@const status = modelStatuses[modelId]}
                      {@const progress = downloadProgress[modelId]}
                      {@const sidecarId = modelIdToSidecarId(modelId)}
                      {@const engineProgress = sidecarId ? engineDownloadProgress[sidecarId] : null}
                      {@const err = modelErrors[modelId]}
                      {@const isActive = (config.stt_config.local_model ?? 'sensevoice') === modelId}
                      <div class="model-item" class:active={isActive}>
                        <div class="model-item-main">
                          <div
                            class="model-radio-row"
                            role="button"
                            tabindex="0"
                            on:click={() => setActiveLocalModel(modelId)}
                            on:keydown={(ev) => ev.key === 'Enter' && setActiveLocalModel(modelId)}
                          >
                            <span class="model-radio" aria-checked={isActive} role="radio">
                              {#if isActive}
                                <Icon icon="ph:radio-button-fill" />
                              {:else}
                                <Icon icon="ph:circle" />
                              {/if}
                            </span>
                            <div class="model-info">
                              <strong>{status?.label ?? modelId}</strong>
                              <span>{status?.size_mb ?? 0} MB • {status?.quality ?? '—'} • {status?.languages ?? '—'}</span>
                              {#if hardwareReqs[modelId] && !hardwareReqs[modelId].can_run}
                                <span class="warning">⚠️ {hardwareReqs[modelId].reason}</span>
                              {/if}
                              {#if sidecarAvailable[modelId] === false}
                                <span class="warning">Engine not available on this platform</span>
                              {/if}
                              {#if status}
                                <span class="status-badge {status.status.toLowerCase()}">{status.status}</span>
                              {/if}
                              {#if status?.status === 'Running' && status.rss_bytes != null && status.rss_bytes > 0}
                                <span class="model-ram">RAM ~{Math.round(status.rss_bytes / (1024 * 1024))} MB</span>
                              {/if}
                            </div>
                          </div>
                          <div class="model-actions">
                            {#if !status?.installed}
                              <button
                                type="button"
                                class="settings-secondary-btn"
                                disabled={(hardwareReqs[modelId] && !hardwareReqs[modelId].can_run) ||
                                  sidecarAvailable[modelId] === false}
                                on:click|stopPropagation={() => downloadModel(modelId)}>Download</button>
                            {:else}
                              {#if status.status === 'Stopped' || status.status === 'Error'}
                                <button
                                  type="button"
                                  class="settings-secondary-btn"
                                  disabled={sidecarAvailable[modelId] === false}
                                  on:click|stopPropagation={() => startModel(modelId)}>Start</button>
                              {:else if status.status === 'Running'}
                                <button
                                  type="button"
                                  class="settings-secondary-btn"
                                  on:click|stopPropagation={() => stopModel(modelId)}>Stop</button>
                                <button
                                  type="button"
                                  class="settings-secondary-btn"
                                  on:click|stopPropagation={() => restartModel(modelId)}>Restart</button>
                              {/if}
                              <button
                                type="button"
                                class="settings-secondary-btn danger"
                                on:click|stopPropagation={() => deleteModel(modelId)}>Delete</button>
                            {/if}
                          </div>
                        </div>
                        {#if progress}
                          <div class="model-download-progress">
                            <progress value={progress.percent ?? 0} max="100" />
                            <span>{progress.percent != null ? Math.round(progress.percent) + '%' : 'Downloading…'}</span>
                          </div>
                        {/if}
                        {#if status?.status === 'Starting' && engineProgress}
                          <div class="model-download-progress">
                            <progress value={engineProgress.percent ?? 0} max="100" />
                            <span>{engineProgress.percent != null ? Math.round(engineProgress.percent) + '%' : 'Downloading engine…'}</span>
                          </div>
                        {/if}
                        {#if err}
                          <p class="model-error">{err}</p>
                        {/if}
                      </div>
                    {/each}
                  </div>

                  {#if sidecarInstalled}
                    <div class="local-engine-section">
                      <span class="setting-desc">Local STT engine is installed</span>
                      <button type="button" class="settings-secondary-btn danger" on:click={() => uninstallEngine(config?.stt_config?.local_model ?? 'sensevoice')}>Uninstall engine</button>
                    </div>
                  {/if}
                </div>
              </div>
              </div>
            {/if}

            <div class="setting-row language-row">
              <div class="setting-label full-width">
                <span class="setting-name">Recognition Languages</span>
                <span class="setting-desc">First language is the default. Support depends on provider.</span>

                <div class="selected-languages">
                  {#each (config.languages ?? ['en']) as code, i}
                    {@const supported = isLanguageSupportedByProvider(code, langProviderKey)}
                    <div class="lang-row" class:unsupported={!supported}>
                      <span class="lang-badge">
                        {#if i === 0}<span class="default-tag">Default</span>{/if}
                        {languageLabel(code)}
                        {#if !supported}
                          <span class="unsupported-icon" title="Not supported">⚠</span>
                        {/if}
                      </span>
                      <div class="lang-actions">
                        {#if i > 0}
                          <button type="button" class="btn-icon" title="Move up" on:click={() => moveLanguageUp(i)}><Icon icon="ph:arrow-up" /></button>
                        {/if}
                        {#if i < (config.languages?.length ?? 1) - 1}
                          <button type="button" class="btn-icon" title="Move down" on:click={() => moveLanguageDown(i)}><Icon icon="ph:arrow-down" /></button>
                        {/if}
                        <button type="button" class="btn-icon remove" title="Remove" on:click={() => removeLanguage(i)}><Icon icon="ph:x" /></button>
                      </div>
                    </div>
                  {/each}
                </div>
                <div class="add-language">
                  <select class="form-select" bind:value={addLanguageCode} on:change={addSelectedLanguage}>
                    <option value="">Add a language…</option>
                    {#each LANGUAGE_OPTIONS as opt}
                      <option value={opt.code} disabled={(config?.languages ?? []).includes(opt.code)}>{opt.label}</option>
                    {/each}
                  </select>
                </div>
              </div>
            </div>
          </div>
            {/if}
        </section>

        <section class="settings-section" class:collapsed={collapsedSections.dictation_formatting}>
          <button type="button" class="section-header" on:click={() => toggleSection('dictation_formatting')}>
            <h3>Formatting &amp; Output</h3>
            <Icon icon={collapsedSections.dictation_formatting ? 'ph:caret-down' : 'ph:caret-up'} />
          </button>
          {#if !collapsedSections.dictation_formatting}
          <div class="section-content">
            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">Auto-punctuation</span>
                <span class="setting-desc">Automatically insert commas and periods</span>
              </div>
              <div class="setting-control">
                <label class="toggle-switch">
                  <input type="checkbox" bind:checked={config.formatting.auto_punctuation} on:change={scheduleSave} />
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
                  <input type="checkbox" bind:checked={config.formatting.voice_commands} on:change={scheduleSave} />
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
                  <input type="checkbox" bind:checked={config.formatting.filler_word_removal} on:change={scheduleSave} />
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
                <select class="form-select" bind:value={config.formatting.injection_method} on:change={scheduleSave}>
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

      {:else if activeTab === 'dictionary'}
        <div class="settings-tab-content">
          <section class="settings-section" class:collapsed={collapsedSections.dictionary}>
            <button type="button" class="section-header" on:click={() => toggleSection('dictionary')}>
              <h3>Custom Vocabulary</h3>
              <Icon icon={collapsedSections.dictionary ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.dictionary}
              <div class="section-content">
                <p class="hint">
                  These words improve transcription accuracy for names, brands, and specialized terms.
                </p>
                <div class="setting-row">
                  <div class="setting-label full-width">
                    <span class="setting-name">Add Term</span>
                    <div class="input-group">
                      <input
                        type="text"
                        bind:value={dictionaryNewTerm}
                        placeholder="e.g. Kalam, Balacode"
                        on:keydown={(e) => e.key === 'Enter' && addDictionaryTerm()}
                      />
                      <button type="button" class="settings-secondary-btn" disabled={!dictionaryNewTerm.trim() || dictionaryLoading} on:click={addDictionaryTerm}>
                        Add
                      </button>
                    </div>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label full-width">
                    <span class="setting-name">Current Terms</span>
                    {#if dictionaryLoading && dictionaryEntries.length === 0}
                      <p class="hint">Loading…</p>
                    {:else if dictionaryEntries.length === 0}
                      <p class="hint">No terms yet. Add words above.</p>
                    {:else}
                      <ul class="dictionary-list">
                        {#each dictionaryEntries as entry (entry.id)}
                          <li>
                            <span class="dictionary-term">{entry.term}</span>
                            <button type="button" class="btn-icon remove" on:click={() => deleteDictionaryEntry(entry.id)}>
                              <Icon icon="ph:trash" />
                            </button>
                          </li>
                        {/each}
                      </ul>
                    {/if}
                  </div>
                </div>
              </div>
            {/if}
          </section>
        </div>

      {:else if activeTab === 'command'}
        <div class="settings-tab-content">
          <section class="settings-section" class:collapsed={collapsedSections.command}>
            <button type="button" class="section-header" on:click={() => toggleSection('command')}>
              <h3>Command Mode</h3>
              <Icon icon={collapsedSections.command ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.command}
              <div class="section-content">
                <p class="hint">
                  Use the command hotkey (General → Dictation Hotkeys) to create notes, tasks, or reminders by voice.
                </p>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Enable Command Mode</span>
                    <span class="setting-desc">When off, the command hotkey does nothing</span>
                  </div>
                  <div class="setting-control">
                    <label class="toggle-switch">
                      <input type="checkbox" bind:checked={config.command_config.enabled} on:change={scheduleSave} />
                      <span class="slider"></span>
                    </label>
                  </div>
                </div>

                {#if config.command_config.enabled}
              <div class="setting-row">
                <div class="setting-label">
                  <span class="setting-name">LLM Provider</span>
                  <span class="setting-desc">Optional: enables natural language understanding</span>
                </div>
                <div class="setting-control">
                  <select class="form-select" value={config.command_config.provider ?? ''} on:change={setCommandProvider}>
                    <option value="">None (basic parsing)</option>
                    <option value="groq">Groq</option>
                    <option value="openrouter">OpenRouter</option>
                    <option value="gemini">Gemini</option>
                    <option value="openai">OpenAI</option>
                    <option value="anthropic">Anthropic</option>
                  </select>
                </div>
              </div>

              {#if config.command_config.provider}
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">API Key</span>
                    {#if commandApiKeyStatus === 'valid'}
                      <span class="badge success">✓ Valid</span>
                    {:else if commandApiKeyStatus === 'invalid'}
                      <span class="badge error">✗ Invalid</span>
                    {:else if hasCommandApiKey && !commandApiKeyInput}
                      <span class="badge configured">✓ Configured</span>
                    {/if}
                  </div>
                  <div class="setting-control full-width">
                    <div class="input-group">
                      <input
                        type="password"
                        bind:value={commandApiKeyInput}
                        on:input={() => { commandApiKeyStatus = 'idle'; commandApiKeyError = null; scheduleSave(); }}
                        placeholder={hasCommandApiKey ? "Enter new key to change" : "Enter API key"}
                      />
                      <button type="button" class="settings-secondary-btn" disabled={loadingLlmModels || (!commandApiKeyInput.trim() && !config.command_config.api_keys?.[config.command_config.provider])} on:click={fetchCommandLlmModels}>
                        {loadingLlmModels ? 'Testing…' : 'Test & Load'}
                      </button>
                      {#if hasCommandApiKey && !commandApiKeyInput}
                        <button type="button" class="settings-secondary-btn danger" on:click={clearCommandApiKey}>Clear</button>
                      {/if}
                    </div>
                    {#if commandApiKeyError}
                      <p class="hint error">{commandApiKeyError}</p>
                    {/if}
                  </div>
                </div>

                {#if commandApiKeyStatus === 'valid' || llmModels.length > 0 || config.command_config.models?.[config.command_config.provider]}
                  <div class="setting-row">
                    <div class="setting-label">
                      <span class="setting-name">Model</span>
                      {#if commandModelStatus === 'valid'}
                        <span class="badge success">✓ Valid</span>
                      {:else if commandModelStatus === 'invalid'}
                        <span class="badge error">✗ Invalid</span>
                      {:else if commandModelStatus === 'testing'}
                        <span class="badge">Testing…</span>
                      {/if}
                    </div>
                    <div class="setting-control full-width">
                      <div class="custom-combobox" class:open={commandModelDropdownOpen} bind:this={comboboxEl}>
                        <div class="input-wrapper">
                          <input
                            type="text"
                            bind:value={commandModelInputText}
                            on:focus={openDropdown}
                            on:blur={closeDropdown}
                            on:input={handleModelInput}
                            placeholder="Type or select a model..."
                            autocomplete="off"
                          />
                          <Icon icon="ph:caret-down" class="dropdown-icon" />
                        </div>
                        {#if commandModelDropdownOpen}
                          <div class="combobox-dropdown-container" style="position: fixed; top: {dropdownTop}px; left: {dropdownLeft}px; width: {dropdownWidth}px;">
                            {#if filteredModels.length > 0}
                              <ul class="combobox-dropdown">
                                {#each filteredModels as m}
                                  <li class:selected={config.command_config.models?.[config.command_config.provider] === m} on:click={() => selectModelFromDropdown(m)}>
                                    <span>{m}</span>
                                    {#if config.command_config.models?.[config.command_config.provider] === m}
                                      <Icon icon="ph:check" />
                                    {/if}
                                  </li>
                                {/each}
                              </ul>
                            {:else}
                              <div class="combobox-empty">
                                <span>Use custom: {commandModelInputText || '...'}</span>
                              </div>
                            {/if}
                          </div>
                        {/if}
                      </div>
                      {#if commandModelError}
                        <p class="hint error">{commandModelError}</p>
                      {/if}
                    </div>
                  </div>
                {/if}
              {/if}
                {/if}
              </div>
            {/if}
          </section>
        </div>

      {:else if activeTab === 'privacy'}
        <div class="settings-tab-content">
          <section class="settings-section" class:collapsed={collapsedSections.privacy_data}>
            <button type="button" class="section-header" on:click={() => toggleSection('privacy_data')}>
              <h3>Data &amp; Privacy</h3>
              <Icon icon={collapsedSections.privacy_data ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.privacy_data}
              <div class="section-content">
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">History Retention</span>
                    <span class="setting-desc">How long to keep dictation history</span>
                  </div>
                  <div class="setting-control">
                    <select class="form-select" bind:value={config.privacy.history_retention_days} on:change={scheduleSave}>
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
                    <span class="setting-name">Sensitive app detection</span>
                    <span class="setting-desc"
                      >For Hybrid and Auto STT modes, use local transcription when the focused app matches a pattern below</span
                    >
                  </div>
                  <div class="setting-control">
                    <label class="toggle-switch">
                      <input type="checkbox" bind:checked={config.privacy.sensitive_app_detection} on:change={scheduleSave} />
                      <span class="slider"></span>
                    </label>
                  </div>
                </div>
                <div class="sensitive-apps-panel">
                  <div class="sensitive-apps-heading">
                    <div class="sensitive-apps-title-row">
                      <span class="setting-name">Sensitive apps</span>
                      <button type="button" class="settings-secondary-btn" on:click={openSensitiveAppPicker}>
                        <Icon icon="ph:plus" />
                        Add app
                      </button>
                    </div>
                    <span class="setting-desc">
                      Dictation switches to local mode when these apps are in focus. Apps are matched by process name.
                    </span>
                  </div>

                  {#if config.privacy.sensitive_app_patterns?.length > 0}
                    <div class="sensitive-apps-list">
                      {#each config.privacy.sensitive_app_patterns as p, i (i)}
                        <div class="sensitive-app-card">
                          <div class="sensitive-app-icon">
                            <Icon icon={getPatternIcon(p)} />
                          </div>
                          <div class="sensitive-app-info">
                            <span class="sensitive-app-name">{getDisplayNameForPattern(p)}</span>
                            <span class="sensitive-app-type">{p.pattern_type}</span>
                          </div>
                          <button
                            type="button"
                            class="btn-icon remove"
                            title="Remove app"
                            aria-label="Remove app"
                            on:click={() => removeSensitiveAppPattern(i)}
                          >
                            <Icon icon="ph:x" />
                          </button>
                        </div>
                      {/each}
                    </div>
                  {:else}
                    <div class="sensitive-apps-empty">
                      <Icon icon="ph:shield-check" />
                      <p>No sensitive apps configured. Click "Add app" to select apps that should trigger local mode.</p>
                    </div>
                  {/if}
                </div>

                {#if sensitiveAppPickerOpen}
                  <!-- svelte-ignore a11y-click-events-have-key-events -->
                  <!-- svelte-ignore a11y-no-static-element-interactions -->
                  <div
                    class="modal-backdrop sensitive-picker-backdrop"
                    use:portalBody
                    on:click={closeSensitiveAppPicker}
                    role="presentation"
                  >
                    <div
                      class="modal sensitive-app-modal kalam-sleek"
                      class:dark={modalPickerDark}
                      class:light={!modalPickerDark}
                      on:click|stopPropagation
                      role="dialog"
                      aria-modal="true"
                      aria-labelledby="sensitive-app-modal-title"
                    >
                      <div class="modal-header">
                        <h3 id="sensitive-app-modal-title">Add sensitive app</h3>
                        <button type="button" class="btn-icon" on:click={closeSensitiveAppPicker}>
                          <Icon icon="ph:x" />
                        </button>
                      </div>

                      <div class="modal-tabs" role="tablist">
                        <button
                          type="button"
                          class="modal-tab"
                          class:active={sensitiveAppPickerTab === 'running'}
                          on:click={() => switchSensitivePickerTab('running')}
                          role="tab"
                          aria-selected={sensitiveAppPickerTab === 'running'}
                        >
                          <Icon icon="ph:activity" />
                          <span class="modal-tab-label">Running now</span>
                        </button>
                        <button
                          type="button"
                          class="modal-tab"
                          class:active={sensitiveAppPickerTab === 'installed'}
                          on:click={() => switchSensitivePickerTab('installed')}
                          role="tab"
                          aria-selected={sensitiveAppPickerTab === 'installed'}
                        >
                          <Icon icon="ph:package" />
                          <span class="modal-tab-label">Installed apps</span>
                        </button>
                        <button
                          type="button"
                          class="modal-tab"
                          class:active={sensitiveAppPickerTab === 'browse'}
                          on:click={() => switchSensitivePickerTab('browse')}
                          role="tab"
                          aria-selected={sensitiveAppPickerTab === 'browse'}
                        >
                          <Icon icon="ph:folder-open" />
                          <span class="modal-tab-label">Browse file</span>
                        </button>
                      </div>

                      {#if sensitiveAppPickerTab === 'running' || (sensitiveAppPickerTab === 'installed' && installedAppsLoaded)}
                        <div class="modal-search">
                          <Icon icon="ph:magnifying-glass" />
                          <input
                            type="text"
                            placeholder={sensitiveAppPickerTab === 'running'
                              ? 'Search running apps...'
                              : 'Search installed apps...'}
                            bind:value={sensitiveAppsSearch}
                          />
                        </div>
                      {/if}

                      <div class="modal-content-scroll">
                        {#if sensitiveAppPickerTab === 'running'}
                          {#if sensitiveAppsLoading}
                            <div class="sensitive-apps-loading">
                              <Icon icon="ph:spinner" class="spin" />
                              <span>Loading running apps…</span>
                            </div>
                          {:else if filteredRunningApps.length > 0}
                            <ul class="sensitive-apps-list-select">
                              {#each filteredRunningApps as app}
                                <li>
                                  <button
                                    type="button"
                                    class="sensitive-app-select-row"
                                    class:already-added={isAppAlreadyAdded(app.process_name)}
                                    on:click={() => addSensitiveAppFromPicker(app.process_name, app.exe_path)}
                                    disabled={isAppAlreadyAdded(app.process_name)}
                                  >
                                    {#if app.icon_base64}
                                      <img
                                        src="data:image/png;base64,{app.icon_base64}"
                                        alt=""
                                        class="sensitive-app-select-icon"
                                      />
                                    {:else}
                                      <div class="sensitive-app-select-icon-placeholder">
                                        <Icon icon="ph:app-window" />
                                      </div>
                                    {/if}
                                    <span class="sensitive-app-select-name">{app.display_name}</span>
                                    {#if isAppAlreadyAdded(app.process_name)}
                                      <span class="sensitive-app-select-badge">Added</span>
                                    {:else}
                                      <Icon icon="ph:caret-right" class="sensitive-app-select-chevron" />
                                    {/if}
                                  </button>
                                </li>
                              {/each}
                            </ul>
                          {:else}
                            <div class="sensitive-apps-empty-state">
                              <Icon icon="ph:app-window" />
                              <p>{sensitiveAppsSearch ? 'No running apps match your search.' : 'No running apps found.'}</p>
                            </div>
                          {/if}
                        {:else if sensitiveAppPickerTab === 'installed'}
                          {#if !installedAppsLoaded && !sensitiveAppsLoading}
                            <div class="sensitive-apps-load-prompt">
                              <Icon icon="ph:package" />
                              <p>
                                Scanning installed applications can take a few seconds on some systems. Load the list when
                                you are ready.
                              </p>
                              <button type="button" class="settings-primary-btn compact" on:click={() => void loadInstalledApps()}>
                                <Icon icon="ph:arrows-clockwise" />
                                Load installed apps
                              </button>
                            </div>
                          {:else if sensitiveAppsLoading}
                            <div class="sensitive-apps-loading">
                              <Icon icon="ph:spinner" class="spin" />
                              <span>Scanning installed apps…</span>
                            </div>
                          {:else if filteredInstalledApps.length > 0}
                            <ul class="sensitive-apps-list-select">
                              {#each filteredInstalledApps as app}
                                <li>
                                  <button
                                    type="button"
                                    class="sensitive-app-select-row"
                                    class:already-added={isAppAlreadyAdded(app.process_name)}
                                    on:click={() => addSensitiveAppFromPicker(app.process_name, app.exe_path)}
                                    disabled={isAppAlreadyAdded(app.process_name)}
                                  >
                                    {#if app.icon_base64}
                                      <img
                                        src="data:image/png;base64,{app.icon_base64}"
                                        alt=""
                                        class="sensitive-app-select-icon"
                                      />
                                    {:else}
                                      <div class="sensitive-app-select-icon-placeholder">
                                        <Icon icon="ph:app-window" />
                                      </div>
                                    {/if}
                                    <span class="sensitive-app-select-name">{app.display_name}</span>
                                    {#if isAppAlreadyAdded(app.process_name)}
                                      <span class="sensitive-app-select-badge">Added</span>
                                    {:else}
                                      <Icon icon="ph:caret-right" class="sensitive-app-select-chevron" />
                                    {/if}
                                  </button>
                                </li>
                              {/each}
                            </ul>
                          {:else}
                            <div class="sensitive-apps-empty-state">
                              <Icon icon="ph:package" />
                              <p>{sensitiveAppsSearch ? 'No installed apps match your search.' : 'No installed apps found.'}</p>
                            </div>
                          {/if}
                        {:else if sensitiveAppPickerTab === 'browse'}
                          <div class="sensitive-apps-browse">
                            <p>Select an executable file (.exe, .app, .AppImage) to add to sensitive apps.</p>
                            <button type="button" class="settings-primary-btn" on:click={pickExecutableFile}>
                              <Icon icon="ph:folder-open" />
                              Choose file…
                            </button>
                          </div>
                        {/if}
                      </div>
                    </div>
                  </div>
                {/if}
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Telemetry</span>
                    <span class="setting-desc">Send anonymous usage statistics (no audio or transcribed text)</span>
                  </div>
                  <div class="setting-control">
                    <label class="toggle-switch">
                      <input type="checkbox" bind:checked={config.privacy.telemetry_enabled} on:change={scheduleSave} />
                      <span class="slider"></span>
                    </label>
                  </div>
                </div>
                <div class="privacy-info">
                  <Icon icon="ph:shield-check" />
                  <p>
                    Your voice data is never stored on our servers. Transcriptions are processed in real-time and discarded
                    immediately. Local mode keeps everything on your device.
                    <a href="https://kalam.stream/privacy.html" target="_blank" rel="noopener noreferrer">Privacy Policy →</a>
                  </p>
                </div>
              </div>
            {/if}
          </section>

          <section class="settings-section" class:collapsed={collapsedSections.privacy_notifications}>
            <button type="button" class="section-header" on:click={() => toggleSection('privacy_notifications')}>
              <h3>Notifications</h3>
              <Icon icon={collapsedSections.privacy_notifications ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.privacy_notifications}
              <div class="section-content">
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Play sounds</span>
                    <span class="setting-desc">Dictation start/end tones and background startup chime</span>
                  </div>
                  <div class="setting-control">
                    <label class="toggle-switch">
                      <input type="checkbox" bind:checked={config.notifications.sound_enabled} on:change={scheduleSave} />
                      <span class="slider"></span>
                    </label>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Show error notifications</span>
                    <span class="setting-desc">System notifications for update check failures and command-mode hints</span>
                  </div>
                  <div class="setting-control">
                    <label class="toggle-switch">
                      <input type="checkbox" bind:checked={config.notifications.show_errors} on:change={scheduleSave} />
                      <span class="slider"></span>
                    </label>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Show completion notifications</span>
                    <span class="setting-desc">Brief notice when dictated text is injected successfully</span>
                  </div>
                  <div class="setting-control">
                    <label class="toggle-switch">
                      <input type="checkbox" bind:checked={config.notifications.show_completion} on:change={scheduleSave} />
                      <span class="slider"></span>
                    </label>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Notify when updates are available</span>
                    <span class="setting-desc">Automatic check shortly after startup (Settings → About for install)</span>
                  </div>
                  <div class="setting-control">
                    <label class="toggle-switch">
                      <input type="checkbox" bind:checked={config.notifications.show_updates} on:change={scheduleSave} />
                      <span class="slider"></span>
                    </label>
                  </div>
                </div>
              </div>
            {/if}
          </section>
        </div>

      {:else if activeTab === 'advanced'}
        <div class="settings-tab-content">
          <section class="settings-section" class:collapsed={collapsedSections.advanced_logs}>
            <button type="button" class="section-header" on:click={() => toggleSection('advanced_logs')}>
              <h3>Logging &amp; Diagnostics</h3>
              <Icon icon={collapsedSections.advanced_logs ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.advanced_logs}
              <div class="section-content">
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Enable Logging</span>
                    <span class="setting-desc">Keep app logs for troubleshooting (in memory and database)</span>
                  </div>
                  <div class="setting-control">
                    <label class="toggle-switch">
                      <input type="checkbox" bind:checked={config.logging.enabled} on:change={scheduleSave} />
                      <span class="slider"></span>
                    </label>
                  </div>
                </div>
                <div class="setting-row sub-setting">
                  <div class="setting-label">
                    <span class="setting-name">Log Level</span>
                  </div>
                  <div class="setting-control">
                    <select class="form-select" bind:value={config.logging.level} on:change={scheduleSave}>
                      <option value="Off">Off</option>
                      <option value="Error">Error</option>
                      <option value="Warn">Warn</option>
                      <option value="Info">Info</option>
                      <option value="Debug">Debug</option>
                    </select>
                  </div>
                </div>
                <div class="setting-row sub-setting">
                  <div class="setting-label">
                    <span class="setting-name">Max Records</span>
                    <span class="setting-desc">Between 500 and 20,000</span>
                  </div>
                  <div class="setting-control">
                    <div class="number-input">
                      <input type="number" min="500" max="20000" step="500" bind:value={config.logging.max_records} on:change={scheduleSave} />
                    </div>
                  </div>
                </div>
                {#if appDataPath}
                  <p class="hint path-display" style="margin-top:12px;font-size:12px;word-break:break-all">{appDataPath}</p>
                {/if}
                {#if openFolderError}
                  <p class="hint error">Failed to open folder: {openFolderError}</p>
                {/if}
                <div class="log-actions">
                  <button type="button" class="settings-secondary-btn" on:click={downloadLog}>
                    <Icon icon="ph:download" /> Download log
                  </button>
                  <button type="button" class="settings-secondary-btn" on:click={downloadLogsCsv}>Download CSV</button>
                  <button type="button" class="settings-secondary-btn" on:click={openAppDataFolder}>
                    <Icon icon="ph:folder-open" /> Open Data Folder
                  </button>
                </div>
                <p class="hint">
                  {#if logEmpty}
                    No log entries yet. Enable logging to capture entries.
                  {:else}
                    Download current buffer or full history from the database.
                  {/if}
                </p>
                {#if logExportMessage}
                  <p class="hint error">{logExportMessage}</p>
                {/if}
              </div>
            {/if}
          </section>

          <section class="settings-section danger" class:collapsed={collapsedSections.advanced_danger}>
            <button type="button" class="section-header" on:click={() => toggleSection('advanced_danger')}>
              <h3>Danger Zone</h3>
              <Icon icon={collapsedSections.advanced_danger ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.advanced_danger}
              <div class="section-content">
                <div class="danger-item">
                  <div class="danger-info">
                    <span class="danger-title">Clear All Data</span>
                    <span class="danger-desc">Delete history, notes, tasks, reminders, snippets, and settings</span>
                  </div>
                  <button type="button" class="danger-btn" disabled={resetting} on:click={confirmAndReset}>
                    {resetting ? 'Resetting…' : 'Reset'}
                  </button>
                </div>
                {#if resetError}
                  <p class="hint error">{resetError}</p>
                {/if}
              </div>
            {/if}
          </section>
        </div>

      {:else if activeTab === 'about'}
        <About embeddedInSettings={true} />
      {/if}
    </div>
  </div>
{:else}
  <div class="page fade-in state-container">
    {#if !initialLoadDone}
      <Icon icon="ph:spinner-gap-duotone" />
      <p>Loading settings…</p>
    {:else}
      <Icon icon="ph:info" />
      <p>Settings are available in the Kalam desktop app.</p>
    {/if}
  </div>
{/if}

<style>
  /* Prototype-matching styles - using CSS custom properties from App.svelte */
  .settings-page {
    max-width: 800px;
  }

  .settings-header {
    margin-bottom: var(--space-xl);
  }

  .save-status {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-muted);
    padding: 6px 12px;
    background: var(--bg-elevated);
    border-radius: var(--radius-full);
  }

  .save-status.error {
    color: #FF3B30;
    background: rgba(255, 59, 48, 0.1);
  }

  .save-error {
    margin: 0 0 var(--space-lg);
    padding: 12px 16px;
    background: rgba(255, 59, 48, 0.1);
    border: 1px solid rgba(255, 59, 48, 0.2);
    border-radius: var(--radius-md);
    color: #FF3B30;
    font-size: 14px;
  }

  .settings-tabs-shell {
    position: relative;
    margin-bottom: var(--space-xl);
  }

  /* Edge fades: wide multi-stop gradients so the mask blends into the page (no hard cutoff). */
  .settings-tabs-fade {
    position: absolute;
    top: 0;
    bottom: 1px;
    width: 64px;
    z-index: 1;
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.28s ease-out;
  }

  .settings-tabs-fade.visible {
    opacity: 1;
  }

  .settings-tabs-fade--left {
    left: 0;
    background: linear-gradient(
      to right,
      var(--bg) 0%,
      color-mix(in srgb, var(--bg) 92%, transparent) 18%,
      color-mix(in srgb, var(--bg) 55%, transparent) 48%,
      color-mix(in srgb, var(--bg) 18%, transparent) 78%,
      transparent 100%
    );
  }

  .settings-tabs-fade--right {
    right: 0;
    background: linear-gradient(
      to left,
      var(--bg) 0%,
      color-mix(in srgb, var(--bg) 92%, transparent) 18%,
      color-mix(in srgb, var(--bg) 55%, transparent) 48%,
      color-mix(in srgb, var(--bg) 18%, transparent) 78%,
      transparent 100%
    );
  }

  .settings-tabs-edge-btn {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    padding: 0;
    border: 1px solid color-mix(in srgb, var(--border) 75%, transparent);
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--bg-elevated) 94%, var(--border-subtle));
    color: var(--text-secondary);
    cursor: pointer;
    transition: color 0.15s ease, background 0.15s ease, border-color 0.15s ease;
  }

  /* Use --bg-hover (sleek theme) — :root --bg-input-hover is light-only and breaks dark hover contrast. */
  .settings-tabs-edge-btn:hover {
    color: var(--text);
    background: var(--bg-hover);
    border-color: color-mix(in srgb, var(--border-visible) 65%, transparent);
    /* Removed shadow */
  }

  /* Match settings accordions: native button + Iconify SVG may not pick up parent color in WebView (dark mode). */
  .settings-tabs-edge-btn :global(svg) {
    display: block;
    width: 1.25em;
    height: 1.25em;
    flex-shrink: 0;
    color: inherit;
  }

  .settings-tabs-edge-btn--left {
    left: 12px;
  }

  .settings-tabs-edge-btn--right {
    right: 12px;
  }

  .settings-tabs {
    display: flex;
    gap: 4px;
    overflow-x: auto;
    overflow-y: hidden;
    scroll-padding-inline: 8px;
  }

  .settings-tab {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
    border-bottom: 2px solid transparent;
    margin-bottom: -2px;
    white-space: nowrap;
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

  .settings-content {
    min-height: 400px;
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

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: var(--space-lg);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: background 0.2s ease;
    text-align: left;
  }

  .section-header:hover {
    background: var(--bg-hover);
  }

  .section-header h3 {
    font-size: 14px;
    font-weight: 600;
    margin: 0;
    color: var(--text);
  }

  .section-content {
    padding: 0 var(--space-lg) var(--space-lg);
  }

  .settings-section.collapsed .section-content {
    display: none;
  }

  /* Let height-based slide transition run while the block is still mounted on collapse */
  .settings-section.collapsed .section-content.accordion-animated {
    display: block;
  }

  .stt-mode-selected-hint {
    margin: 0 0 var(--space-lg);
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-secondary);
  }

  .sensitive-apps-panel {
    padding: var(--space-md) 0 var(--space-lg);
    border-bottom: 1px solid var(--border-light);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .sensitive-apps-heading {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .sensitive-apps-title-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .sensitive-apps-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .sensitive-app-card {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .sensitive-app-icon {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    font-size: 18px;
  }

  .sensitive-app-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .sensitive-app-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sensitive-app-type {
    font-size: 11px;
    color: var(--text-secondary);
    text-transform: lowercase;
  }

  .sensitive-apps-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xl);
    color: var(--text-secondary);
    text-align: center;
  }

  .sensitive-apps-empty :global(svg) {
    font-size: 32px;
    opacity: 0.5;
  }

  .sensitive-apps-empty p {
    font-size: 13px;
    max-width: 280px;
    margin: 0;
  }

  /* While the picker is open, the real scroll container is `.page-content` — lock it (html class set in script). */
  :global(html.sensitive-app-picker-open .kalam-sleek .page-content) {
    overflow: hidden !important;
    overscroll-behavior: none;
    touch-action: none;
  }

  /*
   * Portaled to document.body: fixed = viewport, matches app-shell height.
   * Inset padding uses --space-lg to match settings section spacing.
   * Using :global() so these styles apply to the portaled modal (outside component scope).
   */
  :global(.modal-backdrop.sensitive-picker-backdrop) {
    position: fixed;
    inset: 0;
    z-index: 10050;
    box-sizing: border-box;
    width: 100vw;
    max-width: 100vw;
    height: 100vh;
    height: 100dvh;
    max-height: 100vh;
    max-height: 100dvh;
    margin: 0;
    padding: var(--space-lg);
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    overscroll-behavior: none;
  }

  /* Modal inherits CSS vars from .kalam-sleek class on the element */
  :global(.modal.sensitive-app-modal) {
    background: var(--bg-elevated);
    width: 100%;
    max-width: 720px;
    flex: 0 1 auto;
    min-height: 0;
    height: auto;
    max-height: calc(100vh - var(--space-lg) * 2);
    max-height: calc(100dvh - var(--space-lg) * 2);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: var(--radius-lg);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
    border: 1px solid var(--border);
    font-family: var(--font-sleek, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif);
  }

  /* Modal header matches settings section header padding */
  :global(.sensitive-app-modal .modal-header) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-lg);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  :global(.sensitive-app-modal .modal-header h3) {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text);
    font-family: inherit;
  }

  /* Fixed-height tab bar: three equal segments, no inner scroll. */
  :global(.sensitive-app-modal .modal-tabs) {
    display: flex;
    flex-direction: row;
    align-items: stretch;
    height: 52px;
    min-height: 52px;
    max-height: 52px;
    flex-shrink: 0;
    padding: 0 var(--space-sm);
    gap: 0;
    border-bottom: 1px solid var(--border);
    overflow: hidden;
  }

  :global(.sensitive-app-modal .modal-tab) {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    padding: 6px 4px;
    background: transparent;
    border: none;
    border-radius: 0;
    border-bottom: 3px solid transparent;
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.2;
    cursor: pointer;
    transition: color 0.15s ease, border-color 0.15s ease, background 0.15s ease;
    font-family: inherit;
  }

  :global(.sensitive-app-modal .modal-tab svg) {
    font-size: 18px;
    flex-shrink: 0;
  }

  :global(.sensitive-app-modal .modal-tab-label) {
    display: block;
    text-align: center;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0 2px;
  }

  :global(.sensitive-app-modal .modal-tab:hover) {
    background: var(--bg-hover);
    color: var(--text);
  }

  :global(.sensitive-app-modal .modal-tab.active) {
    background: transparent;
    color: var(--accent);
    border-bottom-color: var(--accent);
  }

  /* Modal search uses consistent spacing */
  :global(.sensitive-app-modal .modal-search) {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-md) var(--space-lg);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  :global(.sensitive-app-modal .modal-search svg) {
    color: var(--text-muted);
    font-size: 18px;
    flex-shrink: 0;
  }

  :global(.sensitive-app-modal .modal-search input) {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: none;
    color: var(--text);
    font-size: 14px;
    outline: none;
    font-family: inherit;
  }

  :global(.sensitive-app-modal .modal-search input:disabled) {
    opacity: 0.45;
    cursor: not-allowed;
  }

  :global(.sensitive-app-modal .modal-search input::placeholder) {
    color: var(--text-muted);
  }

  /* Modal content matches settings section-content padding: 0 var(--space-lg) var(--space-lg) */
  :global(.sensitive-app-modal .modal-content-scroll) {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 0 var(--space-lg) var(--space-lg);
    overscroll-behavior: contain;
    -webkit-overflow-scrolling: touch;
  }

  :global(.sensitive-app-modal .sensitive-apps-list-select) {
    list-style: none;
    margin: 0;
    padding: var(--space-sm) 0 0 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  :global(.sensitive-app-modal .sensitive-apps-list-select li) {
    margin: 0;
    padding: 0;
  }

  :global(.sensitive-app-modal .sensitive-app-select-row) {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: var(--space-md);
    width: 100%;
    padding: 10px 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: border-color 0.15s ease, background 0.15s ease;
    text-align: left;
    font-family: inherit;
  }

  :global(.sensitive-app-modal .sensitive-app-select-row:hover:not(:disabled)) {
    border-color: var(--accent);
    background: rgba(0, 122, 255, 0.04);
  }

  :global(.sensitive-app-modal .sensitive-app-select-row:disabled) {
    opacity: 0.55;
    cursor: not-allowed;
  }

  :global(.sensitive-app-modal .sensitive-app-select-row.already-added) {
    border-color: var(--border-light);
    background: var(--bg-hover);
  }

  :global(.sensitive-app-modal .sensitive-app-select-icon) {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    object-fit: contain;
    flex-shrink: 0;
  }

  :global(.sensitive-app-modal .sensitive-app-select-icon-placeholder) {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-size: 18px;
    flex-shrink: 0;
  }

  :global(.sensitive-app-modal .sensitive-app-select-row .sensitive-app-select-name) {
    flex: 1 1 auto;
    min-width: 0;
    font-size: 14px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: inherit;
  }

  :global(.sensitive-app-modal .sensitive-app-select-badge) {
    font-size: 11px;
    padding: 3px 8px;
    background: var(--accent);
    color: var(--accent-fg);
    border-radius: var(--radius-sm);
    font-weight: 600;
    flex-shrink: 0;
  }

  :global(.sensitive-app-modal .sensitive-app-select-chevron) {
    flex-shrink: 0;
    color: var(--text-muted);
    font-size: 18px;
  }

  /* Load prompt centered with consistent spacing */
  :global(.sensitive-app-modal .sensitive-apps-load-prompt) {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-lg) 0;
    text-align: center;
    max-width: 360px;
    margin: 0 auto;
  }

  :global(.sensitive-app-modal .sensitive-apps-load-prompt svg:first-child) {
    font-size: 40px;
    color: var(--text-secondary);
    opacity: 0.6;
  }

  :global(.sensitive-app-modal .sensitive-apps-load-prompt p) {
    margin: 0;
    font-size: 14px;
    line-height: 1.5;
    color: var(--text-secondary);
    font-family: inherit;
  }

  :global(.sensitive-app-modal .settings-primary-btn) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 10px 18px;
    border: none;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s ease, transform 0.1s ease;
    font-family: inherit;
  }

  :global(.sensitive-app-modal .settings-primary-btn:hover) {
    opacity: 0.92;
  }

  :global(.sensitive-app-modal .settings-primary-btn:active) {
    transform: scale(0.98);
  }

  /* Compact variant for load prompt */
  :global(.sensitive-app-modal .settings-primary-btn.compact) {
    padding: 8px 14px;
    font-size: 13px;
    font-weight: 500;
  }

  :global(.sensitive-app-modal .sensitive-apps-loading) {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-lg) 0;
    color: var(--text-secondary);
  }

  :global(.sensitive-app-modal .sensitive-apps-loading svg.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  :global(.sensitive-app-modal .sensitive-apps-empty-state) {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-lg) 0;
    color: var(--text-secondary);
    text-align: center;
  }

  :global(.sensitive-app-modal .sensitive-apps-empty-state svg) {
    font-size: 48px;
    opacity: 0.3;
  }

  :global(.sensitive-app-modal .sensitive-apps-browse) {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-lg);
    padding: var(--space-lg) 0;
    text-align: center;
  }

  :global(.sensitive-app-modal .sensitive-apps-browse p) {
    color: var(--text-secondary);
    margin: 0;
    font-family: inherit;
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

  .setting-row.checkbox-row {
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-xs);
  }

  .setting-row.audio-filter-range-row {
    flex-direction: column;
    align-items: stretch;
    gap: var(--space-xs);
  }

  .setting-row.audio-filter-range-row .setting-control input[type='range'] {
    width: 100%;
    min-height: 1.5rem;
  }

  .setting-row.row-group {
    flex-wrap: wrap;
  }

  .setting-row.local-model-row,
  .setting-row.language-row {
    flex-direction: column;
    align-items: stretch;
  }

  .setting-label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }

  .setting-label.full-width {
    flex: 1 1 100%;
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

  .setting-control.full-width {
    flex-direction: column;
    align-items: stretch;
    min-width: 200px;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 12px;
    cursor: pointer;
    font-size: 14px;
    color: var(--text);
  }

  .checkbox-label input[type="checkbox"] {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }

  input[type="text"],
  input[type="password"],
  input[type="number"],
  select {
    padding: 10px 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 14px;
    font-family: inherit;
    outline: none;
    transition: border-color 0.2s ease;
  }

  input:focus,
  select:focus {
    border-color: var(--text-muted);
  }

  select {
    cursor: pointer;
    min-width: 150px;
  }

  .input-group {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .input-group input {
    flex: 1;
  }

  .btn-secondary {
    padding: 8px 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .btn-secondary:hover {
    background: var(--bg-hover);
    border-color: var(--border-light);
  }

  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-secondary.danger:hover {
    color: #FF3B30;
    border-color: #FF3B30;
    background: rgba(255, 59, 48, 0.1);
  }

  .btn-danger {
    padding: 10px 16px;
    background: transparent;
    border: 1px solid #FF3B30;
    border-radius: var(--radius-md);
    color: #FF3B30;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .btn-danger:hover {
    background: #FF3B30;
    color: white;
  }

  .btn-refresh {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 14px;
    margin-left: 8px;
    padding: 4px;
    display: inline-flex;
    align-items: center;
  }

  .btn-refresh:hover {
    color: var(--text);
  }

  .badge {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .badge.configured {
    background: rgba(52, 199, 89, 0.15);
    color: #34C759;
  }

  .badge.muted {
    background: var(--surface-elevated, rgba(128, 128, 128, 0.12));
    color: var(--text-muted);
  }

  .badge.success {
    background: rgba(52, 199, 89, 0.15);
    color: #34C759;
  }

  .badge.error {
    background: rgba(255, 59, 48, 0.15);
    color: #FF3B30;
  }

  .hint {
    font-size: 13px;
    color: var(--text-muted);
    margin-top: 6px;
  }

  .hint.warning {
    color: #FF9500;
  }

  .hint.error {
    color: #FF3B30;
  }

  .hint a {
    color: var(--text);
    text-decoration: underline;
  }

  .validation {
    font-size: 13px;
    font-weight: 500;
    margin-top: 6px;
    display: block;
  }

  .validation.success {
    color: #34C759;
  }

  .validation.error {
    color: #FF3B30;
  }

  .mic-level-container {
    margin: var(--space-md) 0;
    padding: 16px;
    background: var(--bg);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-light);
  }

  .mic-level {
    height: 8px;
    background: var(--border);
    border-radius: var(--radius-full);
    overflow: hidden;
  }

  .mic-bar {
    height: 100%;
    background: var(--accent);
    transition: width 0.1s ease-out;
    min-width: 4px;
  }

  .mic-status {
    font-size: 13px;
    color: var(--text-secondary);
    margin-top: 8px;
    display: block;
  }

  /* Model list styles */
  .model-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 12px;
  }

  .model-item {
    padding: 16px;
    background: var(--bg);
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    transition: border-color 0.2s ease;
  }

  .model-item:hover {
    border-color: var(--border-light);
  }

  .model-item.active {
    border-color: var(--accent);
  }

  /* One row: model details left, Start/Stop/Delete right (aligned to block, not below). */
  .model-item-main {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .model-radio-row {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    cursor: pointer;
    flex: 1;
    min-width: 0;
  }

  .model-radio {
    flex-shrink: 0;
    font-size: 20px;
    color: var(--accent);
    display: flex;
    align-items: center;
  }

  .model-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }

  .model-info strong {
    font-size: 14px;
    color: var(--text);
  }

  .model-info span {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .model-info .model-ram {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted);
  }

  .model-info .warning {
    color: #FF9500;
  }

  .status-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    width: fit-content;
  }

  .status-badge.running { background: #34C759; color: white; }
  .status-badge.stopped { background: var(--border); color: var(--text-secondary); }
  .status-badge.starting { background: var(--accent); color: var(--accent-fg); }
  .status-badge.error { background: #FF3B30; color: white; }
  .status-badge.notinstalled { background: var(--bg-hover); color: var(--text-muted); }

  .model-download-progress {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    color: var(--text-secondary);
    margin-top: 8px;
  }

  .model-download-progress progress {
    flex: 1;
    max-width: 200px;
    height: 6px;
  }

  .model-error {
    font-size: 13px;
    color: #FF3B30;
    margin: 8px 0 0;
  }

  .model-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    justify-content: flex-end;
    flex-shrink: 0;
  }

  .model-item-main .model-actions {
    margin-top: 0;
  }

  .local-engine-section {
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  /* Language styles */
  .selected-languages {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 12px 0;
  }

  .lang-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 16px;
    background: var(--bg);
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
  }

  .lang-row:hover {
    border-color: var(--border-light);
  }

  .lang-row.unsupported {
    opacity: 0.7;
  }

  .lang-badge {
    font-size: 14px;
    color: var(--text);
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .default-tag {
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 10px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
    text-transform: uppercase;
  }

  .unsupported-icon {
    color: #FF9500;
  }

  .lang-actions {
    display: flex;
    gap: 4px;
  }

  .btn-icon {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    transition: all 0.2s ease;
  }

  .btn-icon:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .btn-icon.remove:hover {
    color: #FF3B30;
    border-color: #FF3B30;
    background: rgba(255, 59, 48, 0.1);
  }

  .add-language {
    margin-top: 8px;
  }

  .add-language select {
    width: 100%;
  }

  .language-toggle-hotkey {
    margin-top: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  /* Dictionary styles */
  .dictionary-list {
    list-style: none;
    padding: 0;
    margin: 12px 0 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .dictionary-list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    background: var(--bg);
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
  }

  .dictionary-term {
    font-size: 14px;
    color: var(--text);
  }

  /* Combobox styles */
  .custom-combobox {
    position: relative;
    width: 100%;
  }

  .custom-combobox.open {
    z-index: 100;
  }

  .input-wrapper {
    position: relative;
  }

  .input-wrapper input {
    width: 100%;
    padding-right: 36px;
  }

  .dropdown-icon {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted);
    pointer-events: none;
  }

  .combobox-dropdown-container {
    position: fixed;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow);
    z-index: 9999;
    max-height: 240px;
    overflow-y: auto;
  }

  .combobox-dropdown {
    list-style: none;
    padding: 6px;
    margin: 0;
  }

  .combobox-dropdown li {
    padding: 10px 12px;
    cursor: pointer;
    border-radius: var(--radius-sm);
    font-size: 14px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    transition: background 0.15s ease;
  }

  .combobox-dropdown li:hover {
    background: var(--bg-hover);
  }

  .combobox-dropdown li.selected {
    background: var(--bg-hover);
    font-weight: 500;
  }

  .combobox-empty {
    padding: 16px;
    text-align: center;
    color: var(--text-secondary);
  }

  /* Danger zone */
  .danger-zone {
    border-color: rgba(255, 59, 48, 0.3);
  }

  .danger-zone .section-header h3 {
    color: #FF3B30;
  }

  .button-row {
    display: flex;
    gap: 8px;
    margin: 8px 0;
  }

  .path-display {
    font-size: 12px;
    word-break: break-all;
    padding: 8px 12px;
    background: var(--bg);
    border-radius: var(--radius-sm);
    margin: 8px 0;
  }

  .about-tab {
    padding: 0;
  }

  code {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  }

  @media (max-width: 768px) {
    .setting-row {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--space-sm);
    }

    .setting-control {
      width: 100%;
    }

    .input-group {
      flex-direction: column;
      width: 100%;
    }

    .input-group input,
    .input-group button {
      width: 100%;
    }

    .settings-tabs-shell {
      margin-bottom: var(--space-lg);
    }

    .settings-tabs {
      gap: 0;
    }

    .settings-tabs-fade {
      width: 52px;
    }

    .settings-tabs-edge-btn {
      width: 40px;
      height: 40px;
    }

    .settings-tab {
      padding: 10px 12px;
      font-size: 13px;
    }

    .settings-tab span {
      display: none;
    }
  }
</style>
