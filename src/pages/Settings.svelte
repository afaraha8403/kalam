<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte'
  import { cubicOut } from 'svelte/easing'
  import { fade, fly, slide } from 'svelte/transition'
  import { invoke, isTauriRuntime, listenSafe } from '$lib/backend'
  import Icon from '@iconify/svelte'
  import { initTelemetry, optOut } from '../lib/telemetry'
  import { sidebarDictationStore } from '../lib/sidebarDictation'
  import { LANGUAGE_OPTIONS, languageLabel, isLanguageSupportedByProvider } from '../lib/languages'
  import type {
    AppConfig,
    AppInjectionRule,
    AudioDevice,
    AudioFilterPreset,
    CatalogProvider,
    SensitiveAppPattern,
    SyncStatusDto,
    ThemePreference,
  } from '../types'
  import HotkeyCapture from '../components/HotkeyCapture.svelte'
  import FilterPreview from '../components/FilterPreview.svelte'
  import About from './About.svelte'
  import { isProcessInSensitiveList } from '../lib/sensitiveAppPatterns'

  let config: AppConfig | null = null
  let audioDevices: AudioDevice[] = []
  let activeTab = 'general'
  let saving = false
  let micLevel = 0
  let testingMic = false
  /** Last mic test capture for optional replay (Play). */
  let micTestSamples: number[] = []
  let micTestSampleRate = 0
  let levelPollIntervalId: ReturnType<typeof setInterval> | null = null
  let audioCtx: AudioContext | null = null
  let apiKeyValid: boolean | null = null
  let hasApiKey = false
  let apiKeyInput = ''
  let addLanguageCode = ''
  let logEmpty = true
  let logBackendMismatch = false
  let logExportMessage: string | null = null
  let saveError: string | null = null
  let appDataPath: string | null = null
  let openFolderError: string | null = null
  let initialLoadDone = false
  let saveDebounceId: ReturnType<typeof setTimeout> | null = null
  let resetting = false
  let resetError: string | null = null

  type DiagnosticSystemInfo = {
    os_name: string
    os_version: string
    architecture: string
    kalam_config_path: string
    kalam_config_exists: boolean
  }

  let diagSystemInfo: DiagnosticSystemInfo | null = null
  let diagLoadErr = ''
  let diagBusy: string | null = null
  let diagLastMessage = ''
  let diagLastDetail = ''
  let diagCaptureSecs = 8
  let diagMatchHotkey = 'Ctrl+Win'

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

  /** Phase 9: multi-PC sync status from Rust (also reflects `config` for last sync time). */
  let syncUi: SyncStatusDto = {
    enabled: false,
    lastSyncAt: null,
    syncing: false,
    error: null,
    deviceId: null,
    hasLicenseKey: false,
  }
  let syncActionError: string | null = null

  async function refreshSyncStatus() {
    try {
      syncUi = (await invoke('get_sync_status')) as SyncStatusDto
    } catch (e) {
      console.error('get_sync_status failed:', e)
    }
  }

  async function toggleSyncEnabled(checked: boolean) {
    if (!config) return
    syncActionError = null
    try {
      if (checked) {
        await invoke('enable_sync')
      } else {
        await invoke('disable_sync')
      }
      config = (await invoke('get_settings')) as AppConfig
      await refreshSyncStatus()
    } catch (e) {
      syncActionError = String(e)
    }
  }

  function onSyncToggleChange(e: Event) {
    const el = e.target as HTMLInputElement
    void toggleSyncEnabled(el.checked)
  }

  async function doSyncNow() {
    syncActionError = null
    try {
      await invoke('sync_now')
      if (config) config = (await invoke('get_settings')) as AppConfig
      await refreshSyncStatus()
    } catch (e) {
      syncActionError = String(e)
    }
  }

  async function doResetServerSync() {
    if (!confirm('Delete all sync data on the server for this license? Notes and settings on this PC stay.')) return
    syncActionError = null
    try {
      await invoke('reset_sync')
      if (config) config = (await invoke('get_settings')) as AppConfig
      await refreshSyncStatus()
    } catch (e) {
      syncActionError = String(e)
    }
  }

  function onLicenseKeyInput(e: Event) {
    if (!config) return
    const el = e.target as HTMLInputElement
    config.license_key = el.value
    scheduleSave()
  }

  function modelIdToSidecarId(modelId: string): string | null {
    if (modelId === 'sensevoice') return 'sherpa-onnx'
    if (modelId === 'whisper_base') return 'whisper-cpp'
    return null
  }

  const tabs = [
    { id: 'general', label: 'General', icon: 'ph:sliders-horizontal' },
    { id: 'dictation', label: 'Voice Input', icon: 'ph:microphone' },
    { id: 'connections', label: 'AI & Models', icon: 'ph:brain' },
    { id: 'insertion', label: 'Text Output', icon: 'ph:text-aa' },
    { id: 'account', label: 'Account & Sync', icon: 'ph:user-circle' },
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
    general_overlay: false,
    general_notifications: false,
    dictation_microphone: false,
    dictation_processing: false,
    dictation_languages: false,
    ai_stt: false,
    model_library: false,
    default_llm: false,
    account_sync: true,
    privacy_data: false,
    advanced_logs: false,
    advanced_diagnostics: true,
    advanced_danger: false,
    insertion_formatting: true,
    insertion_default: false,
    insertion_per_app: false,
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

  /** Curated provider list from Rust (Phase 2). */
  let modelCatalog: CatalogProvider[] = []
  let modelCatalogLoaded = false
  /** Draft API keys per provider id before Save from library cards. */
  let catalogKeyDraft: Record<string, string> = {}
  let catalogKeyTestStatus: Record<string, 'idle' | 'testing' | 'ok' | 'err'> = {}
  let catalogKeyTestError: Record<string, string> = {}
  let selectedProviderToAdd = ''
  let editingProviderKey: string | null = null

  /** Non-local catalog rows for provider key UI (avoids repeated filters in markup). */
  $: remoteCatalogProviders = modelCatalog.filter((p) => p.id !== 'local')
  $: connectedCatalogProviders = remoteCatalogProviders.filter((p) => !!(config?.provider_keys?.[p.id]?.trim()))
  $: addableCatalogProviders = remoteCatalogProviders.filter((p) => !config?.provider_keys?.[p.id]?.trim())

  /** Changing the “Add a provider” pick closes the key form until Continue is pressed again. */
  function onAddProviderSelectChange() {
    editingProviderKey = null
  }

  /** Short capability line for AI provider rows (STT vs LLM). */
  function providerCapabilityLabel(prov: CatalogProvider): string {
    const stt = prov.capabilities.includes('STT')
    const llm = prov.capabilities.includes('LLM')
    if (stt && llm) return 'Transcription + LLM'
    if (llm) return 'LLM only'
    if (stt) return 'Transcription only'
    return ''
  }

  /** `get_platform`: windows | macos | linux — for hotkey Meta label (Win / Cmd / Super). */
  let appPlatform = ''

  type PermissionStatusItem = { state: string; actionable: boolean; message: string }
  type PermissionStatusPayload = {
    platform: string
    microphone: PermissionStatusItem
    accessibility: PermissionStatusItem
    input_monitoring: PermissionStatusItem
  }
  type RuntimeCapabilitiesPayload = {
    can_capture_audio: boolean
    can_text_inject: boolean
    can_global_hotkey: boolean
    capture_audio_state: string
    text_inject_state: string
    global_hotkey_state: string
    next_steps: string[]
    permission_status: PermissionStatusPayload
  }

  let runtimeCaps: RuntimeCapabilitiesPayload | null = null
  let permCapsLoading = false
  let micTestStartError: string | null = null
  let micTestStarting = false

  function badgeLabelForState(state: string): string {
    if (state === 'granted') return 'Granted'
    if (state === 'needs_action') return 'Needs action'
    return 'Unknown'
  }

  async function loadRuntimeCapabilities() {
    permCapsLoading = true
    try {
      runtimeCaps = (await invoke('get_runtime_capabilities')) as RuntimeCapabilitiesPayload
    } catch (e) {
      console.error('get_runtime_capabilities failed:', e)
      runtimeCaps = null
    } finally {
      permCapsLoading = false
    }
  }

  function openPermissionPage(permission: 'microphone' | 'accessibility') {
    invoke('open_system_permission_page', { permission }).catch((e) => console.error(e))
  }

  function requestPermission(permission: 'microphone' | 'accessibility') {
    invoke('request_system_permission', { permission }).catch((e) => console.error(e))
  }

  $: if (activeTab === 'dictation' || activeTab === 'insertion') {
    void loadRuntimeCapabilities()
  }

  $: if (activeTab === 'account') {
    void refreshSyncStatus()
  }

  function toggleSection(section: string) {
    collapsedSections[section] = !collapsedSections[section]
    collapsedSections = { ...collapsedSections }
  }

  function getCurrentSttProvider(): string {
    return config?.stt_config?.provider || 'groq'
  }

  function getProviderKey(provider: string): string | null {
    if (!config) return null
    const p = provider.trim()
    const fromUnified = config.provider_keys?.[p]?.trim()
    if (fromUnified) return fromUnified
    return config.stt_config?.api_keys?.[p] ?? null
  }

  function getStoredSttApiKey(): string | null {
    if (!config?.stt_config) return null
    const provider = getCurrentSttProvider()
    return getProviderKey(provider) ?? config.stt_config.api_key ?? null
  }

  async function ensureModelCatalogLoaded() {
    if (modelCatalogLoaded) return
    modelCatalogLoaded = true
    try {
      modelCatalog = (await invoke('get_model_catalog')) as CatalogProvider[]
    } catch (e) {
      console.error('get_model_catalog failed:', e)
      modelCatalog = []
    }
  }

  $: if (activeTab === 'connections') void ensureModelCatalogLoaded()

  function maskProviderKey(k: string): string {
    const t = k.trim()
    if (t.length <= 8) return '••••••••'
    return `${t.slice(0, 4)}…${t.slice(-4)}`
  }

  /** Read input/textarea value from an event (no TS `as` casts in Svelte templates — svelte-check rejects them). */
  function eventInputValue(e: Event): string {
    const t = e.currentTarget
    return t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement ? t.value : ''
  }

  function onCatalogKeyInput(e: Event, providerId: string) {
    catalogKeyDraft = { ...catalogKeyDraft, [providerId]: eventInputValue(e) }
    catalogKeyTestStatus = { ...catalogKeyTestStatus, [providerId]: 'idle' }
  }

  /** Custom OpenAI-compatible endpoint fields: avoid TS `!` in markup (breaks svelte-check). */
  function onCustomOpenAiBaseUrlInput(e: Event) {
    if (!config?.custom_openai_endpoint) return
    const cur = config.custom_openai_endpoint
    config.custom_openai_endpoint = { ...cur, base_url: eventInputValue(e) }
    scheduleSave()
  }

  function onCustomOpenAiKeyInput(e: Event) {
    if (!config?.custom_openai_endpoint) return
    const cur = config.custom_openai_endpoint
    const v = eventInputValue(e)
    config.custom_openai_endpoint = { ...cur, api_key: v }
    scheduleSave()
  }

  function onCustomOpenAiModelInput(e: Event) {
    if (!config?.custom_openai_endpoint) return
    const cur = config.custom_openai_endpoint
    const v = eventInputValue(e)
    config.custom_openai_endpoint = { ...cur, model_id: v }
    scheduleSave()
  }

  async function testCatalogProviderKey(providerId: string) {
    const draft = catalogKeyDraft[providerId]?.trim()
    const stored = getProviderKey(providerId)
    const apiKey = draft || stored || ''
    if (!apiKey) return
    catalogKeyTestStatus = { ...catalogKeyTestStatus, [providerId]: 'testing' }
    catalogKeyTestError = { ...catalogKeyTestError, [providerId]: '' }
    try {
      const ok = (await invoke('test_provider_key', { provider: providerId, apiKey })) as boolean
      catalogKeyTestStatus = { ...catalogKeyTestStatus, [providerId]: ok ? 'ok' : 'err' }
      if (!ok) catalogKeyTestError = { ...catalogKeyTestError, [providerId]: 'Validation failed' }
    } catch (e) {
      catalogKeyTestStatus = { ...catalogKeyTestStatus, [providerId]: 'err' }
      catalogKeyTestError = { ...catalogKeyTestError, [providerId]: String(e) }
    }
  }

  async function persistCatalogProviderKey(providerId: string) {
    const raw = catalogKeyDraft[providerId]?.trim() ?? ''
    if (!raw) return
    try {
      const updated = (await invoke('set_provider_key', { provider: providerId, apiKey: raw })) as AppConfig
      config = updated
      catalogKeyDraft = { ...catalogKeyDraft, [providerId]: '' }
      catalogKeyTestStatus = { ...catalogKeyTestStatus, [providerId]: 'idle' }
      const platform = (await invoke('get_platform')) as string
      appPlatform = platform
      sidebarDictationStore.updateFromConfig(updated, platform)
      hasApiKey = !!getStoredSttApiKey()
    } catch (e) {
      console.error('set_provider_key failed:', e)
      catalogKeyTestError = { ...catalogKeyTestError, [providerId]: String(e) }
    }
  }

  async function removeCatalogProviderKey(providerId: string) {
    try {
      const updated = (await invoke('remove_provider_key', { provider: providerId })) as AppConfig
      config = updated
      catalogKeyDraft = { ...catalogKeyDraft, [providerId]: '' }
      const platform = (await invoke('get_platform')) as string
      sidebarDictationStore.updateFromConfig(updated, platform)
      hasApiKey = !!getStoredSttApiKey()
    } catch (e) {
      console.error('remove_provider_key failed:', e)
    }
  }

  async function rerunOnboardingWizard() {
    if (!config) return
    if (!confirm('Reopen the setup wizard? You can skip or finish anytime.')) return
    config.onboarding_complete = false
    await saveSettingsImmediate()
    window.location.reload()
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
        if (!config.overlay_active_preference) config.overlay_active_preference = 'Mini'
        if (config.overlay_always_visible == null) config.overlay_always_visible = false
        if (!config.command_config) {
          config.command_config = { enabled: false, hotkey: null }
        }
        if (config.command_config.hotkey === undefined) config.command_config.hotkey = null

        if (!Array.isArray(config.formatting.force_clipboard_apps)) {
          config.formatting.force_clipboard_apps = []
        }
        if (!Array.isArray(config.formatting.app_injection_rules)) {
          config.formatting.app_injection_rules = []
        }
        if (config.formatting.retry_attempts == null) config.formatting.retry_attempts = 3
        if (config.formatting.retry_delay_ms == null) config.formatting.retry_delay_ms = 100

        const dhk = config.hotkey || config.toggle_dictation_hotkey
        if (dhk) diagMatchHotkey = dhk
      }

      if (!config.provider_keys) config.provider_keys = {}

      void refreshDiagnosticSystemInfo()

      await checkLogEmpty()
      await refreshSyncStatus()

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
  let installedApps: AppListEntry[] = []
  let sensitiveAppsLoading = false
  let sensitiveAppsSearch = ''
  /** Invalidate in-flight scans when the panel closes or a new open starts. */
  let sensitiveAppsLoadGeneration = 0

  /** Portals to `.app-shell` like Dictionary — fixed panel inherits theme vars. */
  function portalAppShell(node: HTMLElement) {
    const target = document.querySelector('.app-shell') || document.body
    target.appendChild(node)
    return {
      destroy() {
        node.remove()
      },
    }
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
    sensitiveAppsLoadGeneration += 1
    sensitiveAppPickerOpen = true
    sensitiveAppsSearch = ''
    installedApps = []
    void loadInstalledApps()
  }

  function closeSensitiveAppPicker() {
    sensitiveAppsLoadGeneration += 1
    sensitiveAppPickerOpen = false
    installedApps = []
    sensitiveAppsSearch = ''
    sensitiveAppsLoading = false
  }

  async function loadInstalledApps() {
    const gen = sensitiveAppsLoadGeneration
    sensitiveAppsLoading = true
    try {
      const list = (await invoke('get_installed_apps')) as AppListEntry[]
      if (gen !== sensitiveAppsLoadGeneration) return
      installedApps = list
    } catch (e) {
      if (gen !== sensitiveAppsLoadGeneration) return
      console.error('Failed to load installed apps:', e)
      installedApps = []
    } finally {
      if (gen === sensitiveAppsLoadGeneration) {
        sensitiveAppsLoading = false
      }
    }
  }

  function escapeRegex(str: string): string {
    return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  }

  function isAppAlreadyAdded(processName: string): boolean {
    return isProcessInSensitiveList(config?.privacy.sensitive_app_patterns, processName)
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

  $: filteredInstalledApps = sensitiveAppsSearch
    ? installedApps.filter((a) =>
        a.display_name.toLowerCase().includes(sensitiveAppsSearch.toLowerCase()) ||
        a.process_name.toLowerCase().includes(sensitiveAppsSearch.toLowerCase())
      )
    : installedApps

  // --- Text insertion: per-app rules (same installed-app scan as Privacy) ---
  let injectionAppPickerOpen = false
  let injectionInstalledApps: AppListEntry[] = []
  let injectionAppsLoading = false
  let injectionAppsSearch = ''
  let injectionAppsLoadGeneration = 0

  function ensureAppInjectionRulesArray() {
    if (!config) return
    if (!Array.isArray(config.formatting.app_injection_rules)) {
      config.formatting.app_injection_rules = []
    }
    if (!Array.isArray(config.formatting.force_clipboard_apps)) {
      config.formatting.force_clipboard_apps = []
    }
  }

  function removeAppInjectionRule(index: number) {
    if (!config) return
    ensureAppInjectionRulesArray()
    config.formatting.app_injection_rules = config.formatting.app_injection_rules.filter((_, i) => i !== index)
    scheduleSave()
  }

  async function openInjectionAppPicker() {
    injectionAppsLoadGeneration += 1
    injectionAppPickerOpen = true
    injectionAppsSearch = ''
    injectionInstalledApps = []
    void loadInjectionInstalledApps()
  }

  function closeInjectionAppPicker() {
    injectionAppsLoadGeneration += 1
    injectionAppPickerOpen = false
    injectionInstalledApps = []
    injectionAppsSearch = ''
    injectionAppsLoading = false
  }

  async function loadInjectionInstalledApps() {
    const gen = injectionAppsLoadGeneration
    injectionAppsLoading = true
    try {
      const list = (await invoke('get_installed_apps')) as AppListEntry[]
      if (gen !== injectionAppsLoadGeneration) return
      injectionInstalledApps = list
    } catch (e) {
      if (gen !== injectionAppsLoadGeneration) return
      console.error('Failed to load installed apps (injection picker):', e)
      injectionInstalledApps = []
    } finally {
      if (gen === injectionAppsLoadGeneration) {
        injectionAppsLoading = false
      }
    }
  }

  function isInjectionAppAlreadyAdded(processName: string): boolean {
    return !!config?.formatting.app_injection_rules?.some(
      (r) => r.process_name.toLowerCase() === processName.toLowerCase()
    )
  }

  function addInjectionAppFromPicker(processName: string, displayName: string) {
    if (!config) return
    if (isInjectionAppAlreadyAdded(processName)) {
      closeInjectionAppPicker()
      return
    }
    ensureAppInjectionRulesArray()
    const next: AppInjectionRule = {
      process_name: processName,
      display_name: displayName?.trim() ? displayName.trim() : null,
      method: 'Clipboard',
      keystroke_delay_ms: null,
      clipboard_threshold: null,
    }
    config.formatting.app_injection_rules = [...config.formatting.app_injection_rules, next]
    scheduleSave()
    closeInjectionAppPicker()
  }

  function injectionRuleDisplayName(rule: AppInjectionRule): string {
    if (rule.display_name?.trim()) return rule.display_name.trim()
    return rule.process_name.replace(/\.exe$/i, '').replace(/\.app$/i, '')
  }

  /** Global defaults: “length before paste” only applies to Automatic on Windows/Linux. */
  $: insertionShowThreshold =
    !!config &&
    config.formatting.injection_method === 'Auto' &&
    appPlatform !== 'macos'
  $: insertionShowTypingSpeed =
    !!config &&
    (config.formatting.injection_method === 'Keystrokes' ||
      (config.formatting.injection_method === 'Auto' && appPlatform !== 'macos'))

  /** macOS Automatic / Accessibility API benefit from Accessibility permission (see inline note). */
  $: insertionShowAccessibilityNote =
    appPlatform === 'macos' &&
    !!config &&
    (config.formatting.injection_method === 'Auto' ||
      config.formatting.injection_method === 'AccessibilityAPI')

  /** Max pause between typed characters (ms); higher values make long text impractical. */
  const INSERTION_KEYSTROKE_PAUSE_MAX_MS = 100

  function injectionRuleShowsSpeed(rule: AppInjectionRule): boolean {
    return rule.method === 'Keystrokes' || (rule.method === 'Auto' && appPlatform !== 'macos')
  }

  function injectionRuleShowsThreshold(rule: AppInjectionRule): boolean {
    return rule.method === 'Auto' && appPlatform !== 'macos'
  }

  $: filteredInjectionInstalledApps = injectionAppsSearch
    ? injectionInstalledApps.filter(
        (a) =>
          a.display_name.toLowerCase().includes(injectionAppsSearch.toLowerCase()) ||
          a.process_name.toLowerCase().includes(injectionAppsSearch.toLowerCase())
      )
    : injectionInstalledApps

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

    if (!config.provider_keys) config.provider_keys = {}
    if (apiKeyInput.trim()) {
      config.provider_keys[getCurrentSttProvider()] = apiKeyInput.trim()
    }
    if (config.logging) {
      config.logging.max_records = Math.min(20000, Math.max(500, config.logging.max_records || 2000))
    }
    if (config.language_toggle_hotkey === '') config.language_toggle_hotkey = null
    if (!Array.isArray(config.languages) || config.languages.length === 0) config.languages = ['en']
    if (!Array.isArray(config.privacy.sensitive_app_patterns)) config.privacy.sensitive_app_patterns = []
    if (!Array.isArray(config.formatting.force_clipboard_apps)) config.formatting.force_clipboard_apps = []
    if (!Array.isArray(config.formatting.app_injection_rules)) config.formatting.app_injection_rules = []
    if (config.formatting.retry_attempts == null) config.formatting.retry_attempts = 3
    if (config.formatting.retry_delay_ms == null) config.formatting.retry_delay_ms = 100

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
    // Cross-check: verify the backend's effective logging config matches
    // what the UI shows, so we can warn on desync.
    try {
      const backendState = await invoke('get_logging_state') as { enabled: boolean; level: string }
      if (config?.logging) {
        logBackendMismatch =
          backendState.enabled !== config.logging.enabled ||
          backendState.level !== config.logging.level
      }
    } catch {
      logBackendMismatch = false
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

  async function refreshDiagnosticSystemInfo() {
    if (!isTauriRuntime()) {
      diagLoadErr = 'Diagnostics run inside the desktop app only.'
      return
    }
    diagLoadErr = ''
    try {
      diagSystemInfo = (await invoke('get_diagnostic_system_info')) as DiagnosticSystemInfo
    } catch (e) {
      diagLoadErr = e instanceof Error ? e.message : String(e)
    }
  }

  function setDiagBusy(id: string | null) {
    diagBusy = id
    diagLastMessage = ''
    diagLastDetail = ''
  }

  async function diagRun(id: string, fn: () => Promise<unknown>) {
    if (!isTauriRuntime()) return
    setDiagBusy(id)
    try {
      const out = await fn()
      diagLastMessage = 'Done.'
      diagLastDetail = JSON.stringify(out, null, 2)
    } catch (e) {
      diagLastMessage = 'Failed.'
      diagLastDetail = e instanceof Error ? e.message : String(e)
    } finally {
      diagBusy = null
    }
  }

  function diagRunHook() {
    return diagRun('hook', () => invoke('run_hook_installation_test'))
  }

  function diagRunCapture() {
    const s = Math.min(120, Math.max(1, Math.floor(Number(diagCaptureSecs)) || 8))
    diagCaptureSecs = s
    return diagRun('capture', () => invoke('run_key_capture_test', { durationSecs: s }))
  }

  function diagRunMatch() {
    const hk = diagMatchHotkey.trim()
    return diagRun('match', () => invoke('run_hotkey_matching_test', { hotkeyStr: hk }))
  }

  function diagRunConfig() {
    return diagRun('config', () => invoke('analyze_kalam_config_diagnostic'))
  }

  function diagRunHealth() {
    return diagRun('health', () => invoke('run_system_health_check'))
  }

  async function diagGetModifierState() {
    if (!isTauriRuntime()) return
    setDiagBusy('modifier')
    try {
      const [ctrl, alt, shift, meta] = (await invoke('get_modifier_state')) as [boolean, boolean, boolean, boolean]
      diagLastMessage = 'Internal Modifier State'
      diagLastDetail = JSON.stringify({ ctrl, alt, shift, meta }, null, 2)
    } catch (e) {
      diagLastMessage = 'Failed to get state.'
      diagLastDetail = e instanceof Error ? e.message : String(e)
    } finally {
      diagBusy = null
    }
  }

  async function diagSaveReport() {
    if (!isTauriRuntime()) return
    setDiagBusy('save')
    try {
      const path = (await invoke('save_diagnostics_report_to_file')) as string
      diagLastMessage = 'Report saved.'
      diagLastDetail = path
    } catch (e) {
      diagLastMessage = 'Save failed.'
      diagLastDetail = e instanceof Error ? e.message : String(e)
    } finally {
      diagBusy = null
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
    micTestStartError = null
    micTestStarting = true
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
      micTestStartError = e instanceof Error ? e.message : String(e)
    } finally {
      micTestStarting = false
    }
  }

  async function stopTestRecording() {
    if (!testingMic) return
    micTestStartError = null
    if (levelPollIntervalId != null) {
      clearInterval(levelPollIntervalId)
      levelPollIntervalId = null
    }
    try {
      const result = await invoke('test_microphone_stop') as { level: number; samples: number[]; sample_rate: number }
      micLevel = result.level
      if (result.samples?.length && result.sample_rate) {
        micTestSamples = result.samples
        micTestSampleRate = result.sample_rate
      }
    } catch (e) {
      micLevel = 0
    } finally {
      testingMic = false
    }
  }

  async function playMicTestRecording() {
    if (!micTestSamples.length || !micTestSampleRate) return
    await playTestAudio(micTestSamples, micTestSampleRate)
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
      if (!config.provider_keys) config.provider_keys = {}
      delete config.provider_keys[provider]
      if (config.stt_config.api_keys) delete config.stt_config.api_keys[provider]
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
      .finally(() => {
        if (activeTab === 'dictation') void loadRuntimeCapabilities()
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

  function setCommandHotkey(hotkey: string) {
    if (config?.command_config) {
      config = {
        ...config,
        command_config: { ...config.command_config, hotkey: hotkey === '' ? null : hotkey }
      }
      scheduleSave()
    }
  }

  function setVoiceEditHotkey(hotkey: string) {
    if (!config) return
    config = {
      ...config,
      voice_edit_hotkey: hotkey === '' ? null : hotkey
    }
    scheduleSave()
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
      <span
        class="save-status"
        class:visible={saving || !!saveError}
        class:error={!!saveError}
        aria-live="polite"
      >
        {#if saving}
          Saving…
        {:else if saveError}
          Save failed
        {:else}
          &#8203;
        {/if}
      </span>
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
              <h3>Keyboard shortcuts</h3>
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
                    <span class="setting-name">Recording mode</span>
                    <span class="setting-desc">Which dictation shortcuts are active (you can still assign both keys above)</span>
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
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Minimum hold time</span>
                    <span class="setting-desc">How long to hold the dictation shortcut before recording starts</span>
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
                    <span class="setting-name">Toggle Language</span>
                    <span class="setting-desc">Switch between recognition languages</span>
                  </div>
                  <div class="setting-control">
                    <HotkeyCapture value={config.language_toggle_hotkey ?? ''} platform={appPlatform} onChange={setLanguageToggleHotkey} />
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Command mode shortcut</span>
                    <span class="setting-desc">Create notes, tasks, and reminders by voice</span>
                  </div>
                  <div class="setting-control">
                    <HotkeyCapture value={config.command_config?.hotkey ?? ''} platform={appPlatform} onChange={setCommandHotkey} />
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Enable command mode</span>
                    <span class="setting-desc">Allow voice-triggered notes, tasks, and reminders from any dictation mode</span>
                  </div>
                  <div class="setting-control">
                    <label class="toggle-switch">
                      <input type="checkbox" bind:checked={config.command_config.enabled} on:change={scheduleSave} />
                      <span class="slider"></span>
                    </label>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Voice editing</span>
                    <span class="setting-desc"
                      >Hold to record an instruction for highlighted text; uses your active mode’s LLM (Windows only)</span
                    >
                  </div>
                  <div class="setting-control">
                    <HotkeyCapture value={config.voice_edit_hotkey ?? ''} platform={appPlatform} onChange={setVoiceEditHotkey} />
                  </div>
                </div>
              </div>
            {/if}
          </section>

          <section class="settings-section" class:collapsed={collapsedSections.general_startup}>
            <button type="button" class="section-header" on:click={() => toggleSection('general_startup')}>
              <h3>Startup</h3>
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
              </div>
            {/if}
          </section>

          <section class="settings-section" class:collapsed={collapsedSections.general_overlay}>
            <button type="button" class="section-header" on:click={() => toggleSection('general_overlay')}>
              <h3>Overlay</h3>
              <Icon icon={collapsedSections.general_overlay ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.general_overlay}
              <div class="section-content">
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Waveform style</span>
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
                    <span class="setting-name">Expand direction</span>
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
                    <span class="setting-name">Overlay position</span>
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
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Active overlay style</span>
                    <span class="setting-desc">Compact pill or full panel when dictation is active</span>
                  </div>
                  <div class="setting-control">
                    <div class="segmented-control">
                      <button
                        type="button"
                        class:active={config.overlay_active_preference === 'Mini'}
                        on:click={() => {
                          if (!config) return
                          config.overlay_active_preference = 'Mini'
                          scheduleSave()
                        }}>Mini</button
                      >
                      <button
                        type="button"
                        class:active={config.overlay_active_preference === 'Full'}
                        on:click={() => {
                          if (!config) return
                          config.overlay_active_preference = 'Full'
                          scheduleSave()
                        }}>Full</button
                      >
                    </div>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Always show overlay</span>
                    <span class="setting-desc">Keep the idle overlay pill fully opaque instead of faded</span>
                  </div>
                  <div class="setting-control">
                    <label class="toggle-switch">
                      <input type="checkbox" bind:checked={config.overlay_always_visible} on:change={scheduleSave} />
                      <span class="slider"></span>
                    </label>
                  </div>
                </div>
              </div>
            {/if}
          </section>

          <section class="settings-section" class:collapsed={collapsedSections.general_notifications}>
            <button type="button" class="section-header" on:click={() => toggleSection('general_notifications')}>
              <h3>Notifications &amp; sounds</h3>
              <Icon icon={collapsedSections.general_notifications ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.general_notifications}
              <div class="section-content">
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Sound effects</span>
                    <span class="setting-desc">Tones when dictation starts and stops, plus the background startup chime</span>
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
                    <span class="setting-name">Error notifications</span>
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
                    <span class="setting-name">Completion notifications</span>
                    <span class="setting-desc">Brief notice when dictated text is inserted successfully</span>
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
                    <span class="setting-name">Update available</span>
                    <span class="setting-desc">Notify after startup when a new version is found (install from About)</span>
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

      {:else if activeTab === 'dictation'}
        <div class="settings-tab-content">
          <section class="settings-section" class:collapsed={collapsedSections.dictation_microphone}>
            <button type="button" class="section-header" on:click={() => toggleSection('dictation_microphone')}>
              <h3>Microphone</h3>
              <Icon icon={collapsedSections.dictation_microphone ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.dictation_microphone}
              <div class="section-content">
                <div class="perm-cap-panel" aria-live="polite">
                  <p class="perm-cap-panel-title">Permissions &amp; capabilities</p>
                  {#if permCapsLoading}
                    <p class="hint">Checking permissions…</p>
                  {:else if runtimeCaps}
                    <div class="perm-cap-row">
                      <div class="perm-cap-row-head">
                        <span class="perm-cap-name">Microphone</span>
                        <span
                          class="perm-cap-badge"
                          class:perm-cap-badge--granted={runtimeCaps.capture_audio_state === 'granted'}
                          class:perm-cap-badge--needs={runtimeCaps.capture_audio_state === 'needs_action'}
                          class:perm-cap-badge--unknown={runtimeCaps.capture_audio_state === 'unknown'}
                          >{badgeLabelForState(runtimeCaps.capture_audio_state)}</span
                        >
                      </div>
                      <p class="hint perm-cap-msg">{runtimeCaps.permission_status.microphone.message}</p>
                      <div class="perm-cap-actions">
                        {#if appPlatform === 'macos'}
                          <button type="button" class="settings-secondary-btn" on:click={() => openPermissionPage('microphone')}>
                            Open Microphone settings
                          </button>
                          <span class="perm-cap-inline-hint">Use <strong>Record</strong> below to trigger the prompt if needed.</span>
                        {:else if appPlatform === 'windows'}
                          {#if runtimeCaps.permission_status.microphone.actionable}
                            <button type="button" class="settings-secondary-btn" on:click={() => openPermissionPage('microphone')}>
                              Open microphone privacy settings
                            </button>
                          {/if}
                        {:else}
                          <span class="perm-cap-inline-hint">Use the mic test below; Linux audio permissions vary by distro.</span>
                        {/if}
                      </div>
                    </div>
                    <div class="perm-cap-row">
                      <div class="perm-cap-row-head">
                        <span class="perm-cap-name">Text insertion</span>
                        <span
                          class="perm-cap-badge"
                          class:perm-cap-badge--granted={runtimeCaps.text_inject_state === 'granted'}
                          class:perm-cap-badge--needs={runtimeCaps.text_inject_state === 'needs_action'}
                          class:perm-cap-badge--unknown={runtimeCaps.text_inject_state === 'unknown'}
                          >{badgeLabelForState(runtimeCaps.text_inject_state)}</span
                        >
                      </div>
                      <p class="hint perm-cap-msg">{runtimeCaps.permission_status.accessibility.message}</p>
                      <div class="perm-cap-actions">
                        {#if appPlatform === 'macos'}
                          {#if runtimeCaps.text_inject_state !== 'granted'}
                            <button type="button" class="settings-secondary-btn" on:click={() => requestPermission('accessibility')}>
                              Request accessibility prompt
                            </button>
                            <button type="button" class="settings-secondary-btn" on:click={() => openPermissionPage('accessibility')}>
                              Open Accessibility settings
                            </button>
                          {/if}
                        {:else if appPlatform === 'windows'}
                          <span class="perm-cap-inline-hint">No separate accessibility toggle is usually required on Windows.</span>
                        {:else}
                          <p class="hint perm-cap-linux">Text injection depends on your desktop; there isn’t one universal Linux settings link.</p>
                        {/if}
                      </div>
                    </div>
                    <div class="perm-cap-row">
                      <div class="perm-cap-row-head">
                        <span class="perm-cap-name">Global hotkey</span>
                        <span
                          class="perm-cap-badge"
                          class:perm-cap-badge--granted={runtimeCaps.global_hotkey_state === 'granted'}
                          class:perm-cap-badge--needs={runtimeCaps.global_hotkey_state === 'needs_action'}
                          class:perm-cap-badge--unknown={runtimeCaps.global_hotkey_state === 'unknown'}
                          >{badgeLabelForState(runtimeCaps.global_hotkey_state)}</span
                        >
                      </div>
                      <p class="hint perm-cap-msg">{runtimeCaps.permission_status.input_monitoring.message}</p>
                    </div>
                  {/if}
                </div>

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
                    <span class="setting-name">Test microphone</span>
                    <span class="setting-desc">Record a short sample, then play it back to check levels</span>
                  </div>
                  <div class="setting-control mic-test-actions">
                    {#if testingMic}
                      <button type="button" class="settings-secondary-btn" on:click={stopTestRecording}>Stop</button>
                    {:else}
                      <button
                        type="button"
                        class="settings-secondary-btn"
                        disabled={micTestStarting}
                        aria-busy={micTestStarting}
                        on:click={startTestRecording}
                        >{micTestStarting ? 'Starting…' : 'Record'}</button
                      >
                      <button
                        type="button"
                        class="settings-secondary-btn"
                        disabled={micTestSamples.length === 0}
                        on:click={playMicTestRecording}
                        title="Play last recording"
                      >
                        Play
                      </button>
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

                {#if micTestStartError}
                  <p class="hint error" role="alert">{micTestStartError}</p>
                  <div class="mic-test-remediation">
                    {#if appPlatform === 'windows' || appPlatform === 'macos'}
                      <button type="button" class="settings-secondary-btn" on:click={() => openPermissionPage('microphone')}>
                        Open microphone settings
                      </button>
                    {/if}
                    <button type="button" class="settings-secondary-btn" on:click={refreshAudioDevices}>
                      Refresh device list
                    </button>
                  </div>
                {/if}
              </div>
            {/if}
          </section>

          <section class="settings-section" class:collapsed={collapsedSections.dictation_processing}>
            <button type="button" class="section-header" on:click={() => toggleSection('dictation_processing')}>
              <h3>Audio processing</h3>
              <Icon icon={collapsedSections.dictation_processing ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.dictation_processing}
              <div class="section-content">
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Silence detection</span>
                    <span class="setting-desc">How long Kalam waits for silence before ending the recording</span>
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
                    <span class="setting-name">Audio cleanup</span>
                    <span class="setting-desc"
                      >Pre-process audio before transcription (same chain as dictation and mic test). Off uses raw input.</span
                    >
                  </div>
                  <div class="setting-control">
                    <select
                      class="form-select"
                      value={config.stt_config.audio_filter.preset}
                      on:change={onAudioFilterPresetSelect}
                    >
                      <option value="Off">Off — no processing</option>
                      <option value="Light">Light — gentle cleanup for most rooms</option>
                      <option value="Moderate">Moderate — stronger noise reduction</option>
                      <option value="Custom">Custom — tune each step</option>
                    </select>
                  </div>
                </div>

                {#if config.stt_config.audio_filter.preset !== 'Off'}
                  <p class="hint audio-filter-hint">
                    {#if config.stt_config.audio_filter.preset === 'Light'}
                      Best default for most microphones: trims low rumble, light compression, consistent volume.
                    {:else if config.stt_config.audio_filter.preset === 'Moderate'}
                      Use in noisier environments; may sound more “processed” than Light.
                    {:else if config.stt_config.audio_filter.preset === 'Custom'}
                      Adjust sliders below; the chart shows an approximate shape (illustration, not a lab measurement).
                    {/if}
                    Long passages and quiet speech often benefit from Light or Moderate so levels stay even.
                  </p>
                  <FilterPreview filter={config.stt_config.audio_filter} />
                {/if}

                {#if config.stt_config.audio_filter.preset === 'Custom'}
                    <div class="setting-row audio-filter-range-row">
                      <div class="setting-label">
                        <span class="setting-name">Bass roll-off</span>
                        <span class="setting-desc">{Math.round(config.stt_config.audio_filter.highpass_cutoff_hz)} Hz — removes low rumble</span>
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
                        <span class="setting-name">Background noise reduction</span>
                        <span class="setting-desc"
                          >{Math.round(config.stt_config.audio_filter.noise_gate_threshold_db)} dB (quieter = cut more noise)</span
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
                        <span class="setting-name">Volume leveling (ratio)</span>
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
                        <span class="setting-name">Output loudness (normalize)</span>
                        <span class="setting-desc"
                          >Peak target {Math.round(config.stt_config.audio_filter.normalize_target_db)} dBFS</span
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
                        <span class="setting-name">Volume leveling (threshold)</span>
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

          <section class="settings-section" class:collapsed={collapsedSections.dictation_languages}>
            <button type="button" class="section-header" on:click={() => toggleSection('dictation_languages')}>
              <h3>Recognition languages</h3>
              <Icon icon={collapsedSections.dictation_languages ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.dictation_languages}
              <div class="section-content">
                <p class="hint">Default speech-to-text mode and API keys are under <strong>AI &amp; Models</strong>.</p>
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

        </div>

      {:else if activeTab === 'connections'}
        <div class="settings-tab-content">
          <section class="settings-section" class:collapsed={collapsedSections.model_library}>
            <button type="button" class="section-header" on:click={() => toggleSection('model_library')}>
              <h3>Providers & API Keys</h3>
              <Icon icon={collapsedSections.model_library ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.model_library}
              <div class="section-content">
                {#if connectedCatalogProviders.length === 0}
                  <p class="hint">No providers connected yet. Use the dropdown below to add one.</p>
                {:else}
                  {#each connectedCatalogProviders as prov}
                    {@const draft = catalogKeyDraft[prov.id] ?? ''}
                    {@const isEditing = editingProviderKey === prov.id}
                    <div class="setting-row prov-row" class:prov-row--editing={isEditing}>
                      <div class="setting-label">
                        <span class="setting-name prov-name">
                          <Icon icon={prov.icon} class="prov-icon" />
                          {prov.name}
                        </span>
                        <span class="setting-desc">{providerCapabilityLabel(prov)}</span>
                        {#if !isEditing}
                          <span class="setting-desc prov-masked-key"><code>{maskProviderKey(config.provider_keys?.[prov.id] ?? '')}</code></span>
                        {/if}
                      </div>
                      <div class="setting-control prov-controls">
                        {#if !isEditing}
                          <button type="button" class="settings-secondary-btn" on:click={() => (editingProviderKey = prov.id)}>Replace key</button>
                          <button type="button" class="settings-secondary-btn danger" on:click={() => removeCatalogProviderKey(prov.id)}>Remove</button>
                        {:else}
                          <button type="button" class="settings-secondary-btn" on:click={() => (editingProviderKey = null)}>Cancel</button>
                        {/if}
                      </div>
                    </div>
                    {#if isEditing}
                      <div class="prov-key-editor">
                        <div class="api-key-row">
                          <input
                            type="password"
                            class="api-key-input"
                            placeholder="Paste new API key"
                            value={draft}
                            on:input={(e) => onCatalogKeyInput(e, prov.id)}
                            aria-label={`New API key for ${prov.name}`}
                          />
                          <button
                            type="button"
                            class="settings-secondary-btn"
                            disabled={!draft.trim()}
                            on:click={() => testCatalogProviderKey(prov.id)}
                          >
                            {catalogKeyTestStatus[prov.id] === 'testing' ? 'Testing…' : 'Test'}
                          </button>
                          <button
                            type="button"
                            class="settings-secondary-btn"
                            disabled={!draft.trim()}
                            on:click={() => {
                              persistCatalogProviderKey(prov.id)
                              editingProviderKey = null
                            }}
                          >
                            Save
                          </button>
                        </div>
                        {#if catalogKeyTestStatus[prov.id] === 'ok'}
                          <p class="hint success">Key looks valid.</p>
                        {:else if catalogKeyTestStatus[prov.id] === 'err'}
                          <p class="hint error">{catalogKeyTestError[prov.id] ?? 'Check failed'}</p>
                        {/if}
                        {#if prov.get_api_key_url}
                          <p class="api-key-hint">
                            <a href={prov.get_api_key_url} target="_blank" rel="noopener noreferrer">Get API key ↗</a>
                          </p>
                        {/if}
                      </div>
                    {/if}
                  {/each}
                {/if}

                <div class="setting-row prov-add-row">
                  <div class="setting-label">
                    <span class="setting-name">Add provider</span>
                    <span class="setting-desc">Connect a new AI service</span>
                  </div>
                  <div class="setting-control prov-add-control">
                    <select
                      class="form-select"
                      bind:value={selectedProviderToAdd}
                      on:change={onAddProviderSelectChange}
                    >
                      <option value="" disabled>Select…</option>
                      {#each addableCatalogProviders as prov}
                        <option value={prov.id}>{prov.name}</option>
                      {/each}
                    </select>
                    <button
                      type="button"
                      class="settings-secondary-btn"
                      disabled={!selectedProviderToAdd}
                      on:click={() => {
                        editingProviderKey = selectedProviderToAdd
                      }}
                    >
                      Add
                    </button>
                  </div>
                </div>

                {#if selectedProviderToAdd && editingProviderKey === selectedProviderToAdd && !(config.provider_keys?.[selectedProviderToAdd]?.trim())}
                  {@const prov = modelCatalog.find((p) => p.id === selectedProviderToAdd)}
                  {#if prov}
                    {@const draft = catalogKeyDraft[prov.id] ?? ''}
                    <div class="prov-key-editor prov-key-editor--new">
                      <p class="prov-new-head">
                        <Icon icon={prov.icon} class="prov-icon" />
                        <strong>{prov.name}</strong>
                        <span class="setting-desc">{providerCapabilityLabel(prov)}</span>
                      </p>
                      <div class="api-key-row">
                        <input
                          type="password"
                          class="api-key-input"
                          placeholder="Paste API key"
                          value={draft}
                          on:input={(e) => onCatalogKeyInput(e, prov.id)}
                          aria-label={`API key for ${prov.name}`}
                        />
                        <button
                          type="button"
                          class="settings-secondary-btn"
                          disabled={!draft.trim()}
                          on:click={() => testCatalogProviderKey(prov.id)}
                        >
                          {catalogKeyTestStatus[prov.id] === 'testing' ? 'Testing…' : 'Test'}
                        </button>
                        <button
                          type="button"
                          class="settings-secondary-btn"
                          disabled={!draft.trim()}
                          on:click={() => {
                            persistCatalogProviderKey(prov.id)
                            editingProviderKey = null
                            selectedProviderToAdd = ''
                          }}
                        >
                          Save
                        </button>
                        <button
                          type="button"
                          class="settings-secondary-btn"
                          on:click={() => {
                            editingProviderKey = null
                            selectedProviderToAdd = ''
                          }}>Cancel</button>
                      </div>
                      {#if catalogKeyTestStatus[prov.id] === 'ok'}
                        <p class="hint success">Key looks valid.</p>
                      {:else if catalogKeyTestStatus[prov.id] === 'err'}
                        <p class="hint error">{catalogKeyTestError[prov.id] ?? 'Check failed'}</p>
                      {/if}
                      {#if prov.get_api_key_url}
                        <p class="api-key-hint">
                          <a href={prov.get_api_key_url} target="_blank" rel="noopener noreferrer">Get API key ↗</a>
                        </p>
                      {/if}
                    </div>
                  {/if}
                {/if}
              </div>
            {/if}
          </section>

          <section class="settings-section" class:collapsed={collapsedSections.default_llm}>
            <button type="button" class="section-header" on:click={() => toggleSection('default_llm')}>
              <h3>Default AI model</h3>
              <Icon icon={collapsedSections.default_llm ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.default_llm}
              <div class="section-content">
                <p class="hint">
                  Fallback model for command mode, polish, and modes that don’t set their own LLM. Turn on command mode under <strong>General → Keyboard shortcuts</strong>. Per-mode overrides on the <strong>Dictation</strong> page take priority.
                </p>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Provider</span>
                    <span class="setting-desc">Used for commands, polish, and AI formatting</span>
                  </div>
                  <div class="setting-control">
                    <select
                      class="form-select"
                      value={config.default_llm_provider ?? ''}
                      on:change={(e) => {
                        const c = config
                        if (!c) return
                        const v = e.currentTarget.value
                        c.default_llm_provider = v || null
                        if (v === 'custom_openai' && !c.custom_openai_endpoint) {
                          c.custom_openai_endpoint = { base_url: '', api_key: '', model_id: '' }
                        }
                        scheduleSave()
                      }}
                    >
                      <option value="">None (basic parsing for commands)</option>
                      {#each modelCatalog.filter(p => p.id !== 'local' && p.capabilities.includes('LLM')) as prov}
                        <option value={prov.id}>{prov.name}{config.provider_keys?.[prov.id]?.trim() ? '' : ' — add key'}</option>
                      {/each}
                      <option value="custom_openai">Custom (OpenAI-compatible URL)</option>
                    </select>
                  </div>
                </div>

                {#if config.default_llm_provider === 'custom_openai' && config.custom_openai_endpoint}
                  <div class="setting-row">
                    <div class="setting-label">
                      <span class="setting-name">Base URL</span>
                      <span class="setting-desc">OpenAI-compatible root, e.g. https://api.example.com/v1</span>
                    </div>
                    <div class="setting-control full-width">
                      <input type="url" class="form-select" value={config.custom_openai_endpoint.base_url} on:input={onCustomOpenAiBaseUrlInput} />
                    </div>
                  </div>
                  <div class="setting-row">
                    <div class="setting-label"><span class="setting-name">API key</span></div>
                    <div class="setting-control full-width">
                      <input type="password" value={config.custom_openai_endpoint.api_key} on:input={onCustomOpenAiKeyInput} placeholder="API key for this endpoint" />
                    </div>
                  </div>
                  <div class="setting-row">
                    <div class="setting-label"><span class="setting-name">Model id</span></div>
                    <div class="setting-control full-width">
                      <input type="text" class="form-select" value={config.custom_openai_endpoint.model_id} on:input={onCustomOpenAiModelInput} placeholder="e.g. gpt-4o-mini" />
                    </div>
                  </div>
                {:else if config != null && config.default_llm_provider}
                  {@const llmProviderId = config.default_llm_provider}
                  {@const llmProv = modelCatalog.find((p) => p.id === llmProviderId)}
                  {@const defaultModel = llmProv?.models?.find((m) => m.capability === 'LLM' && m.is_default)}
                  <div class="setting-row">
                    <div class="setting-label">
                      <span class="setting-name">Model</span>
                      <span class="setting-desc">{defaultModel ? `Default: ${defaultModel.name}` : 'Enter a model id'}</span>
                    </div>
                    <div class="setting-control full-width">
                      <input
                        type="text"
                        class="form-select"
                        value={config.default_llm_model ?? ''}
                        placeholder={defaultModel?.id ?? 'Model id'}
                        on:input={(e) => {
                          const c = config
                          if (!c) return
                          c.default_llm_model = e.currentTarget.value || null
                          scheduleSave()
                        }}
                      />
                    </div>
                  </div>
                {/if}
              </div>
            {/if}
          </section>

          <section class="settings-section" class:collapsed={collapsedSections.ai_stt}>
            <button type="button" class="section-header" on:click={() => toggleSection('ai_stt')}>
              <h3>Speech-to-text</h3>
              <Icon icon={collapsedSections.ai_stt ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.ai_stt}
              <div class="section-content">
                <p class="hint">
                  Global default when a mode’s voice model is set to <strong>Inherit</strong>. Per-mode overrides are on the <strong>Dictation</strong> page.
                </p>
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

                    <p class="hint stt-cloud-key-hint">
                      Add your <strong>{config.stt_config.provider === 'openai' ? 'OpenAI' : 'Groq'}</strong> key under
                      <strong>Providers & API Keys</strong> above. One key covers cloud transcription (when this provider is selected) and LLM features if you use the same provider as your default AI model.
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
                      <span class="setting-desc">Local speech engine is installed</span>
                      <button type="button" class="settings-secondary-btn danger" on:click={() => uninstallEngine(config?.stt_config?.local_model ?? 'sensevoice')}>Uninstall engine</button>
                    </div>
                  {/if}
                </div>
              </div>
              </div>
            {/if}
              </div>
            {/if}
          </section>
        </div>

      {:else if activeTab === 'account'}
        <div class="settings-tab-content">
          <section class="settings-section" class:collapsed={collapsedSections.account_sync}>
            <button type="button" class="section-header" on:click={() => toggleSection('account_sync')}>
              <h3>Multi-PC sync (Pro)</h3>
              <Icon icon={collapsedSections.account_sync ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.account_sync}
              <div class="section-content">
                <p class="setting-desc" style="margin-bottom: 12px;">
                  Sync notes, tasks, snippets, dictionary, dictation modes, and settings across computers. Requires an
                  <strong>active Pro or trial</strong> license. Uses the same API host as the recipe library (<code
                    >{config?.recipe_library_url ?? '—'}</code
                  >). Dictation history stays on each device.
                </p>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Pro license key</span>
                    <span class="setting-desc">Used for Bearer authentication (validate + sync)</span>
                  </div>
                  <div class="setting-control" style="min-width: 220px;">
                    <input
                      type="text"
                      class="form-input"
                      placeholder="KALAM-XXXX-…"
                      value={config?.license_key ?? ''}
                      disabled={!config}
                      on:input={onLicenseKeyInput}
                    />
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Enable sync</span>
                    <span class="setting-desc">Pull and push in the background when online</span>
                  </div>
                  <div class="setting-control">
                    <label class="toggle-switch">
                      <input
                        type="checkbox"
                        checked={!!config?.sync_enabled}
                        disabled={!config || syncUi.syncing}
                        on:change={onSyncToggleChange}
                      />
                      <span class="slider"></span>
                    </label>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Last synced</span>
                    <span class="setting-desc">{syncUi.syncing ? 'Syncing…' : syncUi.lastSyncAt ?? config?.sync_last_at ?? '—'}</span>
                  </div>
                  <div class="setting-control" style="display: flex; gap: 8px; flex-wrap: wrap;">
                    <button type="button" class="settings-secondary-btn" disabled={syncUi.syncing} on:click={doSyncNow}
                      >Sync now</button
                    >
                  </div>
                </div>
                {#if syncUi.deviceId}
                  <div class="setting-row">
                    <div class="setting-label">
                      <span class="setting-name">Device ID</span>
                      <span class="setting-desc mono">{syncUi.deviceId}</span>
                    </div>
                  </div>
                {/if}
                {#if syncUi.error || syncActionError}
                  <p class="hint" style="color: var(--danger, #c62828);">{syncUi.error ?? syncActionError}</p>
                {/if}
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Reset server sync data</span>
                    <span class="setting-desc">Clears the cloud copy for this license; local data unchanged</span>
                  </div>
                  <div class="setting-control">
                    <button type="button" class="settings-secondary-btn danger" on:click={doResetServerSync}
                      >Reset sync</button
                    >
                  </div>
                </div>
              </div>
            {/if}
          </section>
        </div>

      {:else if activeTab === 'insertion'}
        <div class="settings-tab-content">
          <section class="settings-section" class:collapsed={collapsedSections.insertion_formatting}>
            <button type="button" class="section-header" on:click={() => toggleSection('insertion_formatting')}>
              <h3>Transcribed text</h3>
              <Icon icon={collapsedSections.insertion_formatting ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.insertion_formatting}
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
                    <span class="setting-name">Voice commands</span>
                    <span class="setting-desc">Say “new line”, “delete”, etc. to control text</span>
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
                    <span class="setting-name">Filler word removal</span>
                    <span class="setting-desc">Remove “um”, “uh”, “like”, etc.</span>
                  </div>
                  <div class="setting-control">
                    <label class="toggle-switch">
                      <input type="checkbox" bind:checked={config.formatting.filler_word_removal} on:change={scheduleSave} />
                      <span class="slider"></span>
                    </label>
                  </div>
                </div>
                <p class="hint hint--after-toggles">
                  How text is delivered into the active app (typing vs paste, timing, per-app rules) is configured in
                  Insertion below.
                </p>
              </div>
            {/if}
          </section>

          <section class="settings-section" class:collapsed={collapsedSections.insertion_default}>
            <button type="button" class="section-header" on:click={() => toggleSection('insertion_default')}>
              <h3>Insertion</h3>
              <Icon icon={collapsedSections.insertion_default ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.insertion_default}
              <div class="section-content">
                <p class="hint">
                  Choose how Kalam delivers transcribed text into the app in focus.
                </p>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Insertion method</span>
                    <span class="setting-desc">Pick what works best in the apps you dictate into</span>
                  </div>
                  <div class="setting-control">
                    <select class="form-select" bind:value={config.formatting.injection_method} on:change={scheduleSave}>
                      <option value="Auto">Automatic</option>
                      <option value="Keystrokes">Type each character</option>
                      <option value="Clipboard">Paste</option>
                      {#if appPlatform === 'macos'}
                        <option value="AccessibilityAPI">Accessibility API</option>
                      {:else if config.formatting.injection_method === 'AccessibilityAPI'}
                        <option value="AccessibilityAPI">Accessibility API (uses Paste on this system)</option>
                      {/if}
                    </select>
                  </div>
                </div>
                {#if insertionShowAccessibilityNote}
                  <p class="hint insertion-accessibility-hint" role="status">
                    <strong>Accessibility</strong> permission is required for direct text insertion on macOS (System
                    Settings → Privacy &amp; Security → Accessibility).
                    {#if runtimeCaps}
                      <span class="insertion-perm-inline">
                        Status:
                        <span
                          class="perm-cap-badge perm-cap-badge--inline"
                          class:perm-cap-badge--granted={runtimeCaps.text_inject_state === 'granted'}
                          class:perm-cap-badge--needs={runtimeCaps.text_inject_state === 'needs_action'}
                          class:perm-cap-badge--unknown={runtimeCaps.text_inject_state === 'unknown'}
                          >{badgeLabelForState(runtimeCaps.text_inject_state)}</span
                        >
                        {#if runtimeCaps.text_inject_state !== 'granted'}
                          <button
                            type="button"
                            class="settings-secondary-btn insertion-perm-btn"
                            on:click={() => requestPermission('accessibility')}>Request prompt</button
                          >
                          <button
                            type="button"
                            class="settings-secondary-btn insertion-perm-btn"
                            on:click={() => openPermissionPage('accessibility')}>Open settings</button
                          >
                        {/if}
                      </span>
                    {:else if permCapsLoading}
                      <span class="insertion-perm-inline">Checking permission…</span>
                    {/if}
                    Without it, Kalam falls back to <strong>Paste</strong> (clipboard is used briefly).
                  </p>
                {/if}
                {#if config.formatting.injection_method === 'Auto' && appPlatform === 'macos'}
                  <p class="hint">
                    With <strong>Automatic</strong> on macOS, Kalam uses the Accessibility API when possible, then
                    <strong>Paste</strong> if the target does not support it.
                  </p>
                {/if}
                {#if insertionShowThreshold}
                  <p class="hint">
                    With <strong>Automatic</strong>, short phrases are typed and longer passages are pasted. The value
                    below is roughly how long the text can get before Kalam switches to paste.
                  </p>
                  <div class="setting-row">
                    <div class="setting-label">
                      <span class="setting-name">Auto-paste threshold</span>
                      <span class="setting-desc">
                        When using Automatic, paste is used when your text exceeds this many characters. Lower means
                        paste sooner.
                      </span>
                    </div>
                    <div class="setting-control">
                      <div class="number-input">
                        <input
                          type="number"
                          min="1"
                          max="100000"
                          bind:value={config.formatting.clipboard_threshold}
                          on:change={scheduleSave}
                          aria-label="Character count before automatic mode uses paste"
                        />
                      </div>
                    </div>
                  </div>
                {/if}
                {#if insertionShowTypingSpeed}
                  <div class="setting-row">
                    <div class="setting-label">
                      <span class="setting-name">Keystroke delay</span>
                      <span class="setting-desc">
                        Extra wait after each typed character, in milliseconds (0–{INSERTION_KEYSTROKE_PAUSE_MAX_MS}).
                        Leave at 0 for fastest insertion. Increase slightly only if the target app drops or jumbles
                        characters—high values make long dictation very slow.
                      </span>
                    </div>
                    <div class="setting-control">
                      <div class="number-input">
                        <input
                          type="number"
                          min="0"
                          max={INSERTION_KEYSTROKE_PAUSE_MAX_MS}
                          step="1"
                          bind:value={config.formatting.keystroke_delay_ms}
                          on:change={scheduleSave}
                          aria-label="Pause in milliseconds between each typed character"
                        />
                      </div>
                    </div>
                  </div>
                {/if}
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Retry attempts</span>
                    <span class="setting-desc">How many times Kalam retries if insertion fails</span>
                  </div>
                  <div class="setting-control">
                    <div class="number-input">
                      <input
                        type="number"
                        min="1"
                        max="20"
                        bind:value={config.formatting.retry_attempts}
                        on:change={scheduleSave}
                        aria-label="Number of times to retry inserting text"
                      />
                    </div>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Retry delay</span>
                    <span class="setting-desc">Pause before each retry (milliseconds)</span>
                  </div>
                  <div class="setting-control">
                    <div class="number-input">
                      <input
                        type="number"
                        min="0"
                        max="5000"
                        bind:value={config.formatting.retry_delay_ms}
                        on:change={scheduleSave}
                        aria-label="Milliseconds to wait between retry attempts"
                      />
                    </div>
                  </div>
                </div>
              </div>
            {/if}
          </section>

          <section class="settings-section" class:collapsed={collapsedSections.insertion_per_app}>
            <button type="button" class="section-header" on:click={() => toggleSection('insertion_per_app')}>
              <h3>Per-app rules</h3>
              <Icon icon={collapsedSections.insertion_per_app ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.insertion_per_app}
              <div class="section-content">
                <p class="hint">
                  Override defaults for a single program when typing or pasting misbehaves there. Kalam matches the
                  running app by its file name (for example <code class="injection-picker-code">notepad.exe</code>).
                </p>

                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Per-app rules</span>
                    <span class="setting-desc">Each rule uses the same options as above, scoped to one application</span>
                  </div>
                  <div class="setting-control">
                    <button type="button" class="settings-secondary-btn" on:click={openInjectionAppPicker}>
                      <Icon icon="ph:plus" />
                      Add application
                    </button>
                  </div>
                </div>

                {#if config.formatting.app_injection_rules?.length > 0}
                  <div class="injection-rules-stack">
                    {#each config.formatting.app_injection_rules as rule, i (i)}
                      <article
                        class="injection-rule-card"
                        aria-labelledby={`injection-rule-heading-${i}`}
                      >
                        <div class="injection-rule-card-head">
                          <div class="injection-rule-card-identity" id={`injection-rule-heading-${i}`}>
                            <div class="injection-rule-card-icon" aria-hidden="true">
                              <Icon icon="ph:app-window" />
                            </div>
                            <div class="injection-rule-card-titles">
                              <span class="injection-rule-card-name">{injectionRuleDisplayName(rule)}</span>
                              <span class="injection-rule-card-process">{rule.process_name}</span>
                            </div>
                          </div>
                          <button
                            type="button"
                            class="btn-icon remove"
                            title={`Remove rule for ${injectionRuleDisplayName(rule)}`}
                            aria-label={`Remove rule for ${injectionRuleDisplayName(rule)}`}
                            on:click={() => removeAppInjectionRule(i)}
                          >
                            <Icon icon="ph:x" />
                          </button>
                        </div>

                        <div class="injection-rule-card-body">
                          <div class="setting-row">
                            <div class="setting-label">
                              <span class="setting-name">Insertion method</span>
                              <span class="setting-desc">For this app only</span>
                            </div>
                            <div class="setting-control">
                              <select
                                class="form-select"
                                bind:value={rule.method}
                                on:change={scheduleSave}
                                aria-label={`Insertion method for ${injectionRuleDisplayName(rule)}`}
                              >
                                <option value="Auto">Automatic</option>
                                <option value="Keystrokes">Type each character</option>
                                <option value="Clipboard">Paste</option>
                                {#if appPlatform === 'macos'}
                                  <option value="AccessibilityAPI">Accessibility API</option>
                                {:else if rule.method === 'AccessibilityAPI'}
                                  <option value="AccessibilityAPI">Accessibility API (uses Paste on this system)</option>
                                {/if}
                              </select>
                            </div>
                          </div>

                          {#if injectionRuleShowsSpeed(rule)}
                            <div class="setting-row">
                              <div class="setting-label">
                                <span class="setting-name">Keystroke delay</span>
                                <span class="setting-desc">
                                  Leave blank to use the default. 0–{INSERTION_KEYSTROKE_PAUSE_MAX_MS} ms; raise only if
                                  this app drops or jumbles typed text.
                                </span>
                              </div>
                              <div class="setting-control">
                                <div class="number-input">
                                  <input
                                    type="number"
                                    min="0"
                                    max={INSERTION_KEYSTROKE_PAUSE_MAX_MS}
                                    step="1"
                                    placeholder="Default"
                                    title="Leave blank to use the default pause between letters"
                                    value={rule.keystroke_delay_ms ?? ''}
                                    aria-label={`Keystroke delay in milliseconds for ${injectionRuleDisplayName(rule)}`}
                                    on:input={(e) => {
                                      const raw = e.currentTarget.value
                                      rule.keystroke_delay_ms = raw === '' ? null : Number(raw)
                                      scheduleSave()
                                    }}
                                  />
                                </div>
                              </div>
                            </div>
                          {/if}

                          {#if injectionRuleShowsThreshold(rule)}
                            <div class="setting-row">
                              <div class="setting-label">
                                <span class="setting-name">Auto-paste threshold</span>
                                <span class="setting-desc">
                                  Leave blank to use the default. Used when this rule is set to Automatic.
                                </span>
                              </div>
                              <div class="setting-control">
                                <div class="number-input">
                                  <input
                                    type="number"
                                    min="1"
                                    max="100000"
                                    placeholder="Default"
                                    title="Leave blank to use the default length before pasting"
                                    value={rule.clipboard_threshold ?? ''}
                                    aria-label={`Character count before pasting for ${injectionRuleDisplayName(rule)}`}
                                    on:input={(e) => {
                                      const raw = e.currentTarget.value
                                      rule.clipboard_threshold = raw === '' ? null : Number(raw)
                                      scheduleSave()
                                    }}
                                  />
                                </div>
                              </div>
                            </div>
                          {/if}
                        </div>
                      </article>
                    {/each}
                  </div>
                {:else}
                  <div class="injection-rules-empty">
                    <Icon icon="ph:text-aa" />
                    <p>No rules yet. Add an app here if the defaults work everywhere except one program.</p>
                  </div>
                {/if}

                {#if injectionAppPickerOpen}
                  <!-- svelte-ignore a11y-click-events-have-key-events -->
                  <!-- svelte-ignore a11y-no-static-element-interactions -->
                  <div
                    class="sensitive-apps-picker-overlay"
                    role="button"
                    tabindex="0"
                    aria-label="Close panel"
                    on:click={closeInjectionAppPicker}
                    on:keydown={(e) => e.key === 'Enter' && closeInjectionAppPicker()}
                    transition:fade
                    use:portalAppShell
                  ></div>
                  <aside
                    class="sensitive-apps-picker-panel kalam-sleek"
                    transition:fly={{ x: 420, duration: 250, opacity: 1 }}
                    use:portalAppShell
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="injection-apps-picker-title"
                  >
                    <div class="sensitive-apps-picker-header">
                      <h3 id="injection-apps-picker-title">Add application</h3>
                      <button type="button" class="btn-icon" on:click={closeInjectionAppPicker} aria-label="Close">
                        <Icon icon="ph:x" />
                      </button>
                    </div>
                    <p class="sensitive-apps-picker-intro">
                      Pick from applications installed on this device. Rules match the <strong>process name</strong> (for example
                      <code class="injection-picker-code">notepad.exe</code>).
                    </p>
                    <div class="sensitive-apps-picker-search">
                      <Icon icon="ph:magnifying-glass" />
                      <input
                        type="text"
                        placeholder="Search installed apps…"
                        bind:value={injectionAppsSearch}
                        disabled={injectionAppsLoading}
                      />
                    </div>
                    <div class="sensitive-apps-picker-scroll">
                      {#if injectionAppsLoading}
                        <div class="sensitive-apps-loading">
                          <Icon icon="ph:spinner" class="spin" />
                          <span>Scanning installed apps…</span>
                        </div>
                      {:else if filteredInjectionInstalledApps.length > 0}
                        <ul class="sensitive-apps-list-select">
                          {#each filteredInjectionInstalledApps as app}
                            <li>
                              <button
                                type="button"
                                class="sensitive-app-select-row"
                                class:already-added={isInjectionAppAlreadyAdded(app.process_name)}
                                on:click={() => addInjectionAppFromPicker(app.process_name, app.display_name)}
                                disabled={isInjectionAppAlreadyAdded(app.process_name)}
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
                                {#if isInjectionAppAlreadyAdded(app.process_name)}
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
                          <p>{injectionAppsSearch ? 'No installed apps match your search.' : 'No installed apps found.'}</p>
                        </div>
                      {/if}
                    </div>
                  </aside>
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
                      >When speech-to-text is Hybrid or Auto (<strong>AI &amp; Models</strong>), use on-device transcription if the focused app matches a pattern below</span
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
                    class="sensitive-apps-picker-overlay"
                    role="button"
                    tabindex="0"
                    aria-label="Close panel"
                    on:click={closeSensitiveAppPicker}
                    on:keydown={(e) => e.key === 'Enter' && closeSensitiveAppPicker()}
                    transition:fade
                    use:portalAppShell
                  ></div>
                  <aside
                    class="sensitive-apps-picker-panel kalam-sleek"
                    transition:fly={{ x: 420, duration: 250, opacity: 1 }}
                    use:portalAppShell
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="sensitive-apps-picker-title"
                  >
                    <div class="sensitive-apps-picker-header">
                      <h3 id="sensitive-apps-picker-title">Add sensitive app</h3>
                      <button type="button" class="btn-icon" on:click={closeSensitiveAppPicker} aria-label="Close">
                        <Icon icon="ph:x" />
                      </button>
                    </div>
                    <p class="sensitive-apps-picker-intro">
                      Choose from applications installed on this device. Matching uses process names.
                    </p>
                    <div class="sensitive-apps-picker-search">
                      <Icon icon="ph:magnifying-glass" />
                      <input
                        type="text"
                        placeholder="Search installed apps…"
                        bind:value={sensitiveAppsSearch}
                        disabled={sensitiveAppsLoading}
                      />
                    </div>
                    <div class="sensitive-apps-picker-scroll">
                      {#if sensitiveAppsLoading}
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
                    </div>
                  </aside>
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
        </div>

      {:else if activeTab === 'advanced'}
        <div class="settings-tab-content">
          <section class="settings-section" class:collapsed={collapsedSections.advanced_logs}>
            <button type="button" class="section-header" on:click={() => toggleSection('advanced_logs')}>
              <h3>Logging</h3>
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
                  {#if logBackendMismatch}
                    <span style="color:var(--error,#e74c3c)">Warning: Backend logging state differs from settings. Try saving again or restart the app.</span>
                  {:else if logEmpty}
                    No log entries yet. {#if config?.logging?.enabled}Logs will appear after app activity.{:else}Enable logging to capture entries.{/if}
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

          <section class="settings-section" class:collapsed={collapsedSections.advanced_diagnostics}>
            <button type="button" class="section-header" on:click={() => toggleSection('advanced_diagnostics')}>
              <h3>Diagnostics</h3>
              <Icon icon={collapsedSections.advanced_diagnostics ? 'ph:caret-down' : 'ph:caret-up'} />
            </button>
            {#if !collapsedSections.advanced_diagnostics}
              <div class="section-content adv-diag">
                <p class="adv-diag-lede">
                  Hotkey and config checks use a short listener alongside Kalam’s own. Logging is raised to <strong>Debug</strong> only while a test runs.
                  <strong>Save report</strong> attaches structured rows from runs since your last save, plus a log excerpt.
                </p>
                {#if diagLoadErr}
                  <p class="hint error" role="alert">{diagLoadErr}</p>
                {/if}
                {#if diagSystemInfo}
                  <div class="adv-diag-env">
                    <p class="adv-diag-eyebrow" id="adv-diag-env-h">Environment</p>
                    <dl class="adv-diag-dl" aria-labelledby="adv-diag-env-h">
                      <dt>OS</dt>
                      <dd>{diagSystemInfo.os_name} <span class="adv-diag-muted">({diagSystemInfo.architecture})</span></dd>
                      <dt>Version</dt>
                      <dd>{diagSystemInfo.os_version}</dd>
                      <dt>Config</dt>
                      <dd class="adv-diag-path">{diagSystemInfo.kalam_config_path}</dd>
                      <dt>Config file</dt>
                      <dd>{diagSystemInfo.kalam_config_exists ? 'Found' : 'Missing'}</dd>
                    </dl>
                  </div>
                {/if}

                <div class="adv-diag-stack">
                  <div class="adv-diag-block">
                    <p class="adv-diag-eyebrow" id="adv-diag-kb-h">Keyboard and hooks</p>
                    <div class="adv-diag-block-inner" role="group" aria-labelledby="adv-diag-kb-h">
                      <div class="adv-diag-tool">
                        <span class="adv-diag-tool-name">Low-level hook</span>
                        <div class="adv-diag-tool-ctrl">
                          <button
                            type="button"
                            class="settings-secondary-btn"
                            disabled={!isTauriRuntime() || diagBusy !== null}
                            on:click={() => void diagRunHook()}
                          >
                            {#if diagBusy === 'hook'}
                              <span class="adv-diag-spin"><Icon icon="ph:spinner" /></span>
                            {:else}
                              <Icon icon="ph:plug" />
                            {/if}
                            Install probe
                          </button>
                        </div>
                      </div>
                      <div class="adv-diag-tool">
                        <span class="adv-diag-tool-name">Key capture</span>
                        <div class="adv-diag-tool-ctrl">
                          <label class="adv-diag-sr-only" for="adv-diag-cap-secs">Duration in seconds</label>
                          <div class="adv-diag-field">
                            <span class="adv-diag-field-label" aria-hidden="true">Seconds</span>
                            <input
                              id="adv-diag-cap-secs"
                              type="number"
                              min="1"
                              max="120"
                              bind:value={diagCaptureSecs}
                              class="adv-diag-input adv-diag-input-narrow"
                            />
                          </div>
                          <button
                            type="button"
                            class="settings-secondary-btn"
                            disabled={!isTauriRuntime() || diagBusy !== null}
                            on:click={() => void diagRunCapture()}
                          >
                            {#if diagBusy === 'capture'}
                              <span class="adv-diag-spin"><Icon icon="ph:spinner" /></span>
                            {:else}
                              <Icon icon="ph:keyboard" />
                            {/if}
                            Run capture
                          </button>
                        </div>
                      </div>
                      <div class="adv-diag-tool">
                        <span class="adv-diag-tool-name">Hotkey match</span>
                        <div class="adv-diag-tool-ctrl">
                          <label class="adv-diag-sr-only" for="adv-diag-match-hk">Hotkey string to test</label>
                          <input
                            id="adv-diag-match-hk"
                            type="text"
                            bind:value={diagMatchHotkey}
                            class="adv-diag-input adv-diag-input-hotkey"
                            placeholder="e.g. Ctrl+Win"
                            autocomplete="off"
                          />
                          <button
                            type="button"
                            class="settings-secondary-btn"
                            disabled={!isTauriRuntime() || diagBusy !== null}
                            on:click={() => void diagRunMatch()}
                          >
                            {#if diagBusy === 'match'}
                              <span class="adv-diag-spin"><Icon icon="ph:spinner" /></span>
                            {:else}
                              <Icon icon="ph:crosshair" />
                            {/if}
                            10s window
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>

                  <div class="adv-diag-block">
                    <p class="adv-diag-eyebrow" id="adv-diag-sys-h">Config and system</p>
                    <div class="adv-diag-btn-cluster" role="group" aria-labelledby="adv-diag-sys-h">
                      <button
                        type="button"
                        class="settings-secondary-btn"
                        disabled={!isTauriRuntime() || diagBusy !== null}
                        on:click={() => void diagRunConfig()}
                      >
                        {#if diagBusy === 'config'}
                          <span class="adv-diag-spin"><Icon icon="ph:spinner" /></span>
                        {:else}
                          <Icon icon="ph:file-json" />
                        {/if}
                        Analyze config
                      </button>
                      <button
                        type="button"
                        class="settings-secondary-btn"
                        disabled={!isTauriRuntime() || diagBusy !== null}
                        on:click={() => void diagRunHealth()}
                      >
                        {#if diagBusy === 'health'}
                          <span class="adv-diag-spin"><Icon icon="ph:spinner" /></span>
                        {:else}
                          <Icon icon="ph:heartbeat" />
                        {/if}
                        DISM health
                      </button>
                      <button
                        type="button"
                        class="settings-secondary-btn"
                        disabled={!isTauriRuntime() || diagBusy !== null}
                        on:click={() => void diagGetModifierState()}
                      >
                        {#if diagBusy === 'modifier'}
                          <span class="adv-diag-spin"><Icon icon="ph:spinner" /></span>
                        {:else}
                          <Icon icon="ph:keyboard" />
                        {/if}
                        Modifier snapshot
                      </button>
                    </div>
                  </div>

                  <div class="adv-diag-export">
                    <button
                      type="button"
                      class="btn-primary adv-diag-save"
                      disabled={!isTauriRuntime() || diagBusy !== null}
                      on:click={() => void diagSaveReport()}
                    >
                      {#if diagBusy === 'save'}
                        <span class="adv-diag-spin"><Icon icon="ph:spinner" /></span>
                      {:else}
                        <Icon icon="ph:floppy-disk" />
                      {/if}
                      Save diagnostic report
                    </button>
                    <p class="hint adv-diag-export-hint">Writes markdown under <code class="adv-diag-code">.kalam/diagnostics/</code></p>
                  </div>
                </div>

                {#if diagLastMessage || diagLastDetail}
                  <div class="adv-diag-result">
                    <p class="adv-diag-result-eyebrow">Last output</p>
                    <p class="adv-diag-result-title">{diagLastMessage}</p>
                    {#if diagLastDetail}
                      <pre class="adv-diag-pre" tabindex="0">{diagLastDetail}</pre>
                    {/if}
                  </div>
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
    min-height: 32px;
    min-width: 7rem;
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    visibility: hidden;
    transition: opacity 0.15s ease;
  }

  .save-status.visible {
    opacity: 1;
    visibility: visible;
  }

  .save-status.error {
    color: #ff3b30;
    background: rgba(255, 59, 48, 0.1);
  }

  .mic-test-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .audio-filter-hint {
    margin-top: var(--space-sm, 8px);
    margin-bottom: 0;
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

  .hint--after-toggles {
    margin: var(--space-md) 0 0;
  }

  /* Per-app text insertion: same setting-row pattern as defaults; stacks cleanly on narrow widths */
  .injection-rules-stack {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    margin-top: var(--space-md);
  }

  .injection-rule-card {
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg);
    overflow: hidden;
  }

  .injection-rule-card-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-md);
    padding: var(--space-md);
    border-bottom: 1px solid var(--border-light);
    background: color-mix(in oklch, var(--bg) 92%, var(--text) 8%);
  }

  .injection-rule-card-identity {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    min-width: 0;
    flex: 1;
  }

  .injection-rule-card-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    background: color-mix(in oklch, var(--bg) 88%, var(--text) 12%);
    color: var(--text-secondary);
  }

  .injection-rule-card-icon :global(svg) {
    font-size: 1.25rem;
  }

  .injection-rule-card-titles {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .injection-rule-card-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .injection-rule-card-process {
    font-size: 12px;
    color: var(--text-secondary);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .injection-rule-card-body {
    padding: 0 var(--space-md);
  }

  .injection-rule-card-body .setting-row:last-child {
    border-bottom: none;
  }

  .injection-rules-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-sm);
    margin-top: var(--space-md);
    padding: var(--space-xl) var(--space-md);
    color: var(--text-secondary);
    text-align: center;
    border: 1px dashed var(--border);
    border-radius: var(--radius-md);
    background: color-mix(in oklch, var(--bg) 96%, var(--text) 4%);
  }

  .injection-rules-empty :global(svg) {
    font-size: 2rem;
    opacity: 0.45;
  }

  .injection-rules-empty p {
    margin: 0;
    max-width: 22rem;
    font-size: 14px;
    line-height: 1.45;
  }

  :global(code.injection-picker-code) {
    font-size: 0.9em;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    background: var(--bg);
    border: 1px solid var(--border);
    font-family: ui-monospace, 'Cascadia Code', monospace;
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

  /* Right-side picker — same interaction model as Dictionary (portaled to .app-shell). */
  :global(.sensitive-apps-picker-overlay) {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(2px);
    z-index: 9998;
    cursor: pointer;
  }

  :global(aside.sensitive-apps-picker-panel) {
    position: fixed;
    top: 0;
    right: 0;
    width: 100%;
    max-width: 420px;
    height: 100vh;
    height: 100dvh;
    max-height: 100vh;
    max-height: 100dvh;
    background: var(--bg-elevated);
    border-left: 1px solid var(--border);
    color: var(--text);
    z-index: 9999;
    display: flex;
    flex-direction: column;
    box-shadow: -4px 0 24px rgba(0, 0, 0, 0.15);
    font-family: var(--font-sleek, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif);
  }

  :global(.sensitive-apps-picker-header) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  :global(.sensitive-apps-picker-header h3) {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text);
  }

  :global(.sensitive-apps-picker-intro) {
    margin: 0;
    padding: 12px 20px 0;
    font-size: 13px;
    line-height: 1.45;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  :global(.sensitive-apps-picker-search) {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: 14px 20px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  :global(.sensitive-apps-picker-search svg) {
    color: var(--text-muted);
    font-size: 18px;
    flex-shrink: 0;
  }

  :global(.sensitive-apps-picker-search input) {
    flex: 1;
    min-width: 0;
    padding: 8px 10px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    font-size: 14px;
    outline: none;
    font-family: inherit;
  }

  :global(.sensitive-apps-picker-search input:focus) {
    border-color: var(--border-subtle);
    box-shadow: 0 0 0 3px var(--primary-alpha-subtle, rgba(0, 122, 255, 0.15));
  }

  :global(.sensitive-apps-picker-search input:disabled) {
    opacity: 0.45;
    cursor: not-allowed;
  }

  :global(.sensitive-apps-picker-search input::placeholder) {
    color: var(--text-muted);
  }

  :global(.sensitive-apps-picker-scroll) {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 0 20px 20px;
    overscroll-behavior: contain;
    -webkit-overflow-scrolling: touch;
  }

  :global(.sensitive-apps-picker-panel .sensitive-apps-list-select) {
    list-style: none;
    margin: 0;
    padding: 12px 0 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  :global(.sensitive-apps-picker-panel .sensitive-apps-list-select li) {
    margin: 0;
    padding: 0;
  }

  :global(.sensitive-apps-picker-panel .sensitive-app-select-row) {
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

  :global(.sensitive-apps-picker-panel .sensitive-app-select-row:hover:not(:disabled)) {
    border-color: var(--accent);
    background: rgba(0, 122, 255, 0.04);
  }

  :global(.sensitive-apps-picker-panel .sensitive-app-select-row:disabled) {
    opacity: 0.55;
    cursor: not-allowed;
  }

  :global(.sensitive-apps-picker-panel .sensitive-app-select-row.already-added) {
    border-color: var(--border-light);
    background: var(--bg-hover);
  }

  :global(.sensitive-apps-picker-panel .sensitive-app-select-icon) {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    object-fit: contain;
    flex-shrink: 0;
  }

  :global(.sensitive-apps-picker-panel .sensitive-app-select-icon-placeholder) {
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

  :global(.sensitive-apps-picker-panel .sensitive-app-select-row .sensitive-app-select-name) {
    flex: 1 1 auto;
    min-width: 0;
    font-size: 14px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: inherit;
  }

  :global(.sensitive-apps-picker-panel .sensitive-app-select-badge) {
    font-size: 11px;
    padding: 3px 8px;
    background: var(--accent);
    color: var(--accent-fg);
    border-radius: var(--radius-sm);
    font-weight: 600;
    flex-shrink: 0;
  }

  :global(.sensitive-apps-picker-panel .sensitive-app-select-chevron) {
    flex-shrink: 0;
    color: var(--text-muted);
    font-size: 18px;
  }

  :global(.sensitive-apps-picker-panel .sensitive-apps-loading) {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-lg) 0;
    color: var(--text-secondary);
  }

  :global(.sensitive-apps-picker-panel .sensitive-apps-loading svg.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  :global(.sensitive-apps-picker-panel .sensitive-apps-empty-state) {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-lg) 0;
    color: var(--text-secondary);
    text-align: center;
  }

  :global(.sensitive-apps-picker-panel .sensitive-apps-empty-state svg) {
    font-size: 48px;
    opacity: 0.3;
  }

  @media (max-width: 480px) {
    :global(aside.sensitive-apps-picker-panel) {
      max-width: 100%;
    }
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

  .perm-cap-panel {
    margin-bottom: var(--space-lg, 20px);
    padding: 16px;
    background: var(--bg);
    border: 1px solid var(--border-light);
    border-radius: var(--radius-md);
  }

  .perm-cap-panel-title {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    margin: 0 0 12px;
  }

  .perm-cap-row {
    padding-bottom: 14px;
    margin-bottom: 14px;
    border-bottom: 1px solid var(--border-light);
  }

  .perm-cap-row:last-child {
    padding-bottom: 0;
    margin-bottom: 0;
    border-bottom: none;
  }

  .perm-cap-row-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
    margin-bottom: 6px;
  }

  .perm-cap-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }

  .perm-cap-badge {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 3px 8px;
    border-radius: var(--radius-full);
    background: var(--surface-elevated, rgba(128, 128, 128, 0.12));
    color: var(--text-muted);
    border: 1px solid var(--border);
  }

  .perm-cap-badge.perm-cap-badge--granted {
    background: rgba(52, 199, 89, 0.15);
    color: #34c759;
    border-color: rgba(52, 199, 89, 0.35);
  }

  .perm-cap-badge.perm-cap-badge--needs {
    background: rgba(255, 149, 0, 0.12);
    color: #ff9500;
    border-color: rgba(255, 149, 0, 0.35);
  }

  .perm-cap-badge.perm-cap-badge--unknown {
    color: var(--text-secondary);
  }

  .perm-cap-badge.perm-cap-badge--inline {
    display: inline-block;
    vertical-align: middle;
    margin: 0 4px;
  }

  .insertion-accessibility-hint .insertion-perm-inline {
    display: block;
    margin-top: 6px;
  }

  button.insertion-perm-btn {
    margin-right: 6px;
    margin-top: 4px;
  }

  .perm-cap-msg {
    margin: 0 0 8px !important;
    color: var(--text-secondary) !important;
  }

  .perm-cap-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .perm-cap-inline-hint {
    font-size: 12px;
    color: var(--text-muted);
    flex: 1 1 200px;
    line-height: 1.4;
  }

  .perm-cap-linux {
    margin: 0 !important;
    font-size: 13px;
    line-height: 1.45;
  }

  .mic-test-remediation {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 8px 0 16px;
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

  /* Advanced → Diagnostics: grouped layout, env snapshot, tool rows */
  .adv-diag {
    container-type: inline-size;
    container-name: adv-diag;
  }

  .adv-diag-lede {
    margin: 0 0 clamp(1rem, 2.5vw, 1.35rem);
    max-width: 52ch;
    font-size: clamp(0.8125rem, 0.8rem + 0.2vw, 0.9rem);
    line-height: 1.55;
    color: color-mix(in srgb, var(--text, #e8e8e8) 78%, transparent);
  }

  .adv-diag-lede strong {
    color: color-mix(in srgb, var(--text, #e8e8e8) 92%, transparent);
    font-weight: 600;
  }

  .adv-diag-eyebrow {
    margin: 0 0 0.65rem;
    font-size: 0.6875rem;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--text-muted, #999) 92%, var(--accent, #6b9) 8%);
  }

  .adv-diag-env {
    margin-bottom: clamp(1.1rem, 3vw, 1.5rem);
    padding: 0.75rem 1rem 0.85rem 1rem;
    border-radius: 0 var(--radius-md, 10px) var(--radius-md, 10px) 0;
    border-left: 3px solid color-mix(in srgb, var(--accent, #5a8) 55%, var(--border, #333));
    background: color-mix(in srgb, var(--accent, #5a8) 7%, var(--bg-elevated, rgba(255, 255, 255, 0.04)));
  }

  .adv-diag-env .adv-diag-eyebrow {
    margin-bottom: 0.5rem;
  }

  .adv-diag-dl {
    display: grid;
    grid-template-columns: minmax(5.5rem, auto) 1fr;
    gap: 0.35rem 1rem;
    margin: 0;
    font-size: 0.8125rem;
    line-height: 1.45;
  }

  .adv-diag-dl dt {
    margin: 0;
    font-weight: 600;
    color: color-mix(in srgb, var(--text, #fff) 72%, transparent);
  }

  .adv-diag-dl dd {
    margin: 0;
    word-break: break-word;
  }

  .adv-diag-muted {
    font-weight: 400;
    color: color-mix(in srgb, var(--text-muted, #aaa) 95%, transparent);
  }

  .adv-diag-path {
    font-size: 0.78rem;
    line-height: 1.4;
  }

  .adv-diag-stack {
    display: flex;
    flex-direction: column;
    gap: clamp(1.15rem, 3vw, 1.65rem);
  }

  .adv-diag-block-inner {
    border: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.08));
    border-radius: var(--radius-md, 10px);
    background: color-mix(in srgb, var(--bg-elevated, #1a1a1c) 88%, transparent);
    overflow: hidden;
  }

  .adv-diag-tool {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.65rem 1rem;
    padding: 0.65rem 0.85rem;
    border-bottom: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.06));
  }

  .adv-diag-tool:last-child {
    border-bottom: none;
  }

  .adv-diag-tool-name {
    flex: 0 1 auto;
    min-width: min(100%, 7.5rem);
    font-size: 0.8125rem;
    font-weight: 600;
    color: color-mix(in srgb, var(--text, #fff) 88%, transparent);
  }

  .adv-diag-tool-ctrl {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem 0.65rem;
    justify-content: flex-end;
    flex: 1 1 auto;
    min-width: min(100%, 12rem);
  }

  @container adv-diag (min-width: 440px) {
    .adv-diag-tool {
      flex-wrap: nowrap;
    }
    .adv-diag-tool-ctrl {
      justify-content: flex-end;
    }
  }

  .adv-diag-field {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }

  .adv-diag-field-label {
    font-size: 0.75rem;
    color: color-mix(in srgb, var(--text-muted, #999) 95%, transparent);
  }

  .adv-diag-sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .adv-diag-input {
    padding: 0.4rem 0.55rem;
    border-radius: var(--radius-sm, 8px);
    border: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.12));
    background: color-mix(in srgb, var(--bg, #0f0f10) 92%, var(--accent, #5a8) 4%);
    color: inherit;
    font-size: 0.8125rem;
  }

  .adv-diag-input:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--accent, #5a8) 65%, transparent);
    outline-offset: 1px;
  }

  .adv-diag-input-narrow {
    width: 3.5rem;
    text-align: center;
  }

  .adv-diag-input-hotkey {
    min-width: 8rem;
    flex: 1 1 10rem;
    max-width: 16rem;
  }

  .adv-diag-btn-cluster {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
  }

  .adv-diag-export {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.35rem;
    padding-top: 0.25rem;
    border-top: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.1));
    margin-top: 0.15rem;
  }

  .adv-diag-save.btn-primary {
    margin-top: 0.15rem;
  }

  .adv-diag-export-hint {
    margin: 0;
    font-size: 0.75rem;
    max-width: 48ch;
  }

  .adv-diag-code {
    font-size: 0.85em;
    padding: 0.1em 0.35em;
    border-radius: 4px;
    background: color-mix(in srgb, var(--text, #fff) 8%, transparent);
  }

  .adv-diag-result {
    margin-top: clamp(1rem, 2.5vw, 1.35rem);
    padding-top: 1rem;
    border-top: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.1));
  }

  .adv-diag-result-eyebrow {
    margin: 0 0 0.35rem;
    font-size: 0.6875rem;
    font-weight: 600;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--text-muted, #999) 88%, transparent);
  }

  .adv-diag-result-title {
    margin: 0 0 0.5rem;
    font-size: 0.9375rem;
    font-weight: 600;
    color: color-mix(in srgb, var(--text, #fff) 92%, transparent);
  }

  .adv-diag-pre {
    margin: 0;
    font-size: 0.75rem;
    line-height: 1.5;
    max-height: min(40vh, 320px);
    overflow: auto;
    padding: 0.75rem 0.85rem;
    border-radius: var(--radius-sm, 8px);
    border: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.08));
    background: color-mix(in srgb, var(--bg, #000) 94%, var(--accent, #5a8) 6%);
    white-space: pre-wrap;
    word-break: break-word;
    font-variant-ligatures: none;
  }

  .adv-diag-spin {
    display: inline-flex;
    animation: adv-diag-spin 0.75s linear infinite;
  }

  @keyframes adv-diag-spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .adv-diag-spin {
      animation: none;
    }
  }

  .settings-secondary-btn.small {
    padding: 6px 12px;
    font-size: 13px;
  }

  .btn-ghost.small {
    padding: 6px 12px;
    font-size: 13px;
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

  /* Providers section — extends .setting-row pattern */
  .prov-name {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .prov-name :global(.prov-icon) {
    font-size: 18px;
    color: var(--accent);
    flex-shrink: 0;
  }

  .prov-masked-key code {
    font-size: 12px;
    padding: 1px 5px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--text) 6%, transparent);
  }

  .prov-controls {
    flex-wrap: wrap;
  }

  .prov-key-editor {
    padding: 0 0 var(--space-md);
    border-bottom: 1px solid var(--border-light);
  }

  .prov-key-editor--new {
    padding-top: var(--space-sm);
  }

  .prov-new-head {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 var(--space-sm);
    font-size: 14px;
    color: var(--text);
  }

  .prov-new-head :global(.prov-icon) {
    font-size: 18px;
    color: var(--accent);
  }

  .prov-new-head .setting-desc {
    margin-left: 4px;
  }

  .prov-add-row {
    border-bottom: none;
  }

  .prov-add-control {
    flex-wrap: wrap;
  }

  .prov-add-control .form-select {
    min-width: 160px;
  }

  .model-pill {
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
  }

  .model-pill.default {
    border-color: var(--accent);
  }

  .hint.success {
    color: var(--accent, #0d9488);
  }

  .stt-cloud-key-hint {
    margin-top: var(--space-sm);
  }
</style>
