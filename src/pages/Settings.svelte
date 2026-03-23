<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import { invoke, listenSafe } from '$lib/backend'
  import Icon from '@iconify/svelte'
  import { initTelemetry, optOut } from '../lib/telemetry'
  import { sidebarDictationStore } from '../lib/sidebarDictation'
  import { LANGUAGE_OPTIONS, languageLabel, isLanguageSupportedByProvider } from '../lib/languages'
  import type { AppConfig, AudioDevice, DictionaryEntry } from '../types'
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
    { id: 'advanced', label: 'Advanced', icon: 'ph:gear-fine' },
    { id: 'about', label: 'About', icon: 'ph:info' },
  ]

  let commandApiKeyInput = ''
  let llmModels: string[] = []
  let loadingLlmModels = false
  let hasCommandApiKey = false

  let dictionaryEntries: DictionaryEntry[] = []
  let dictionaryNewTerm = ''
  let dictionaryLoading = false

  // Collapsible sections state
  let collapsedSections: Record<string, boolean> = {
    general_hotkeys: false,
    general_startup: true,
    general_overlay: true,
    dictation_audio: false,
    dictation_stt: false,
    dictation_formatting: true,
    dictionary: false,
    command: false,
    privacy: false,
    advanced: false,
    advanced_danger: false
  }

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
      sidebarDictationStore.updateFromConfig(settings, platform)

      if (config) {
        if (config.audio_device == null || config.audio_device === 'default' || config.audio_device === '') {
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
    if (statusPollInterval) clearInterval(statusPollInterval)
    if (unlistenDownloadProgress) unlistenDownloadProgress()
    if (unlistenEngineDownloadProgress) unlistenEngineDownloadProgress()
  })

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

      if (config?.stt_config?.mode === 'Local' || config?.stt_config?.mode === 'Hybrid') {
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

    saveError = null
    try {
      await invoke('save_settings', { newConfig: config })
      const platform = (await invoke('get_platform')) as string
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

    <div class="settings-tabs">
      {#each tabs as tab}
        <button
          type="button"
          class="settings-tab"
          class:active={activeTab === tab.id}
          on:click={() => {
            activeTab = tab.id
            if (tab.id !== 'advanced') logExportMessage = null
            if (tab.id === 'advanced') checkLogEmpty()
            if (tab.id === 'dictionary') loadDictionaryEntries()
          }}
        >
          <Icon icon={tab.icon} />
          <span>{tab.label}</span>
        </button>
      {/each}
    </div>

    <div class="settings-content">
      {#if activeTab === 'general'}
        <section class="settings-section" class:collapsed={collapsedSections.general_hotkeys}>
          <button type="button" class="section-header" on:click={() => toggleSection('general_hotkeys')}>
            <h3>Dictation Hotkeys</h3>
            <Icon icon={collapsedSections.general_hotkeys ? 'ph:caret-right' : 'ph:caret-down'} />
          </button>
          <div class="section-content">
            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">Hold to Dictate</span>
                <span class="setting-desc">Press and hold this hotkey to dictate, release to stop</span>
              </div>
              <div class="setting-control">
                <HotkeyCapture value={config.hotkey ?? ''} onChange={setHotkey} />
              </div>
            </div>
            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">Short-press threshold (ms)</span>
                <span class="setting-desc">Releases earlier than this are treated as short presses and cancelled</span>
              </div>
              <div class="setting-control">
                <input type="number" min="0" max="2000" step="50" bind:value={config.min_hold_ms} on:change={scheduleSave} />
              </div>
            </div>
            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">Toggle Dictation</span>
                <span class="setting-desc">Press once to start, again to stop</span>
              </div>
              <div class="setting-control">
                <HotkeyCapture value={config.toggle_dictation_hotkey ?? ''} onChange={setToggleHotkey} />
              </div>
            </div>
          </div>
        </section>

        <section class="settings-section" class:collapsed={collapsedSections.general_startup}>
          <button type="button" class="section-header" on:click={() => toggleSection('general_startup')}>
            <h3>Startup</h3>
            <Icon icon={collapsedSections.general_startup ? 'ph:caret-right' : 'ph:caret-down'} />
          </button>
          <div class="section-content">
            <div class="setting-row checkbox-row">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={config.auto_start} on:change={scheduleSave} />
                <span>Start on login</span>
              </label>
            </div>
            <div class="setting-row checkbox-row">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={config.start_in_focus} on:change={scheduleSave} />
                <span>Start in focus (show window on startup)</span>
              </label>
              <span class="setting-desc">If disabled, app starts minimized to tray</span>
            </div>
          </div>
        </section>

        <section class="settings-section" class:collapsed={collapsedSections.general_overlay}>
          <button type="button" class="section-header" on:click={() => toggleSection('general_overlay')}>
            <h3>Overlay Appearance</h3>
            <Icon icon={collapsedSections.general_overlay ? 'ph:caret-right' : 'ph:caret-down'} />
          </button>
          <div class="section-content">
            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">Waveform Style</span>
                <span class="setting-desc">Choose how your voice is visualized</span>
              </div>
              <div class="setting-control">
                <select bind:value={config.waveform_style} on:change={scheduleSave}>
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
                <span class="setting-desc">Which direction the pill expands</span>
              </div>
              <div class="setting-control">
                <select bind:value={config.overlay_expand_direction} on:change={scheduleSave}>
                  <option value="Up">Upwards</option>
                  <option value="Down">Downwards</option>
                  <option value="Center">Center</option>
                </select>
              </div>
            </div>
            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">Screen Position</span>
                <span class="setting-desc">Where the pill appears on your primary monitor</span>
              </div>
              <div class="setting-control">
                <select bind:value={config.overlay_position} on:change={scheduleSave}>
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
            <div class="setting-row row-group">
              <div class="setting-label">
                <span class="setting-name">X Offset (px)</span>
              </div>
              <div class="setting-control">
                <input type="number" bind:value={config.overlay_offset_x} on:input={scheduleSave} />
              </div>
              <div class="setting-label">
                <span class="setting-name">Y Offset (px)</span>
              </div>
              <div class="setting-control">
                <input type="number" bind:value={config.overlay_offset_y} on:input={scheduleSave} />
              </div>
            </div>
          </div>
        </section>

      {:else if activeTab === 'dictation'}
        <section class="settings-section" class:collapsed={collapsedSections.dictation_audio}>
          <button type="button" class="section-header" on:click={() => toggleSection('dictation_audio')}>
            <h3>Audio Input</h3>
            <Icon icon={collapsedSections.dictation_audio ? 'ph:caret-right' : 'ph:caret-down'} />
          </button>
          <div class="section-content">
            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">Microphone</span>
                <button class="btn-refresh" on:click={refreshAudioDevices} title="Refresh">
                  <Icon icon="ph:arrow-clockwise" />
                </button>
              </div>
              <div class="setting-control full-width">
                <select bind:value={config.audio_device} on:change={scheduleSave}>
                  {#if audioDevices.length === 0}
                    <option value="">No devices found</option>
                  {:else}
                    {#each audioDevices as device}
                      <option value={device.id === 'default' ? '' : device.id}>
                        {device.is_default ? 'Default — ' + device.name : device.name}
                      </option>
                    {/each}
                  {/if}
                </select>
                {#if audioDevices.length === 0}
                  <span class="hint warning">No audio devices found. Try refreshing.</span>
                {:else}
                  <span class="hint">{audioDevices.length} audio device(s) available</span>
                {/if}
              </div>
            </div>

            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">Test Microphone</span>
                <span class="setting-desc">Record and hear playback</span>
              </div>
              <div class="setting-control">
                {#if testingMic}
                  <button class="btn-secondary" on:click={stopTestRecording}>Stop</button>
                {:else}
                  <button class="btn-secondary" on:click={startTestRecording}>Start</button>
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
                <span class="setting-desc">Voice Activity Detection sensitivity</span>
              </div>
              <div class="setting-control">
                <select bind:value={config.stt_config.vad_preset} on:change={scheduleSave}>
                  <option value="Fast">Fast (0.8s silence)</option>
                  <option value="Balanced">Balanced (1.5s silence)</option>
                  <option value="Accurate">Accurate (2.5s silence)</option>
                </select>
              </div>
            </div>
          </div>
        </section>

        <section class="settings-section" class:collapsed={collapsedSections.dictation_stt}>
          <button type="button" class="section-header" on:click={() => toggleSection('dictation_stt')}>
            <h3>Speech-to-Text Mode</h3>
            <Icon icon={collapsedSections.dictation_stt ? 'ph:caret-right' : 'ph:caret-down'} />
          </button>
          <div class="section-content">
            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">Mode</span>
              </div>
              <div class="setting-control">
                <select bind:value={config.stt_config.mode} on:change={onSttModeChange}>
                  <option value="Cloud">Cloud</option>
                  <option value="Local">Local (SenseVoice)</option>
                  <option value="Hybrid">Hybrid (Auto-switch)</option>
                </select>
              </div>
            </div>

            {#if config.stt_config.mode === 'Cloud' || config.stt_config.mode === 'Hybrid'}
              <div class="setting-row">
                <div class="setting-label">
                  <span class="setting-name">Cloud Provider</span>
                </div>
                <div class="setting-control">
                  <select bind:value={config.stt_config.provider} on:change={onCloudProviderChange}>
                    <option value="groq">Groq</option>
                    <option value="openai">OpenAI</option>
                  </select>
                </div>
              </div>

              <div class="setting-row">
                <div class="setting-label">
                  <span class="setting-name">API Key</span>
                  {#if hasApiKey && !apiKeyInput}
                    <span class="badge configured">✓ Configured</span>
                  {/if}
                </div>
                <div class="setting-control full-width">
                  <div class="input-group">
                    <input
                      type="password"
                      bind:value={apiKeyInput}
                      on:input={scheduleSave}
                      placeholder={hasApiKey ? "Enter new key to change" : "Enter API key"}
                    />
                    <button class="btn-secondary" on:click={checkApiKey}>Validate</button>
                    {#if hasApiKey && !apiKeyInput}
                      <button class="btn-secondary danger" on:click={clearApiKey}>Clear</button>
                    {/if}
                  </div>
                  {#if apiKeyValid !== null}
                    <span class="validation {apiKeyValid ? 'success' : 'error'}">
                      {apiKeyValid ? '✓ Valid' : '✗ Invalid'}
                    </span>
                  {/if}
                  <p class="hint">
                    {#if config.stt_config.provider === 'openai'}
                      <a href="https://platform.openai.com/api-keys" target="_blank">Get your API key from OpenAI →</a>
                    {:else}
                      <a href="https://console.groq.com" target="_blank">Get your API key from Groq →</a>
                    {/if}
                  </p>
                </div>
              </div>
            {/if}

            {#if config.stt_config.mode === 'Local' || config.stt_config.mode === 'Hybrid'}
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
                        <div class="model-radio-row" role="button" tabindex="0" on:click={() => setActiveLocalModel(modelId)} on:keydown={(ev) => ev.key === 'Enter' && setActiveLocalModel(modelId)}>
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
                        <div class="model-actions">
                          {#if !status?.installed}
                            <button class="btn-secondary" disabled={(hardwareReqs[modelId] && !hardwareReqs[modelId].can_run) || sidecarAvailable[modelId] === false} on:click|stopPropagation={() => downloadModel(modelId)}>Download</button>
                          {:else}
                            {#if status.status === 'Stopped' || status.status === 'Error'}
                              <button class="btn-secondary" disabled={sidecarAvailable[modelId] === false} on:click|stopPropagation={() => startModel(modelId)}>Start</button>
                            {:else if status.status === 'Running'}
                              <button class="btn-secondary" on:click|stopPropagation={() => stopModel(modelId)}>Stop</button>
                              <button class="btn-secondary" on:click|stopPropagation={() => restartModel(modelId)}>Restart</button>
                            {/if}
                            <button class="btn-secondary danger" on:click|stopPropagation={() => deleteModel(modelId)}>Delete</button>
                          {/if}
                        </div>
                      </div>
                    {/each}
                  </div>

                  {#if sidecarInstalled}
                    <div class="local-engine-section">
                      <span class="setting-desc">Local STT engine is installed</span>
                      <button class="btn-secondary danger" on:click={() => uninstallEngine(config?.stt_config?.local_model ?? 'sensevoice')}>Uninstall engine</button>
                    </div>
                  {/if}
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
                  <select bind:value={addLanguageCode} on:change={addSelectedLanguage}>
                    <option value="">Add a language…</option>
                    {#each LANGUAGE_OPTIONS as opt}
                      <option value={opt.code} disabled={(config?.languages ?? []).includes(opt.code)}>{opt.label}</option>
                    {/each}
                  </select>
                </div>

                {#if (config?.languages?.length ?? 0) >= 2}
                  <div class="language-toggle-hotkey">
                    <span class="setting-desc">Language toggle hotkey</span>
                    <HotkeyCapture value={config.language_toggle_hotkey ?? ''} onChange={setLanguageToggleHotkey} />
                  </div>
                {/if}
              </div>
            </div>
          </div>
        </section>

        <section class="settings-section" class:collapsed={collapsedSections.dictation_formatting}>
          <button type="button" class="section-header" on:click={() => toggleSection('dictation_formatting')}>
            <h3>Text Formatting</h3>
            <Icon icon={collapsedSections.dictation_formatting ? 'ph:caret-right' : 'ph:caret-down'} />
          </button>
          <div class="section-content">
            <div class="setting-row checkbox-row">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={config.formatting.voice_commands} on:change={scheduleSave} />
                <span>Enable voice commands ("period", "new line", etc.)</span>
              </label>
            </div>
            <div class="setting-row checkbox-row">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={config.formatting.filler_word_removal} on:change={scheduleSave} />
                <span>Remove filler words ("um", "uh", "like")</span>
              </label>
            </div>
            <div class="setting-row checkbox-row">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={config.formatting.auto_punctuation} on:change={scheduleSave} />
                <span>Auto-punctuation</span>
              </label>
            </div>
            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">Text Injection Method</span>
              </div>
              <div class="setting-control">
                <select bind:value={config.formatting.injection_method} on:change={scheduleSave}>
                  <option value="Auto">Auto (recommended)</option>
                  <option value="Keystrokes">Keystrokes only</option>
                  <option value="Clipboard">Clipboard only</option>
                </select>
              </div>
            </div>
          </div>
        </section>

      {:else if activeTab === 'dictionary'}
        <section class="settings-section" class:collapsed={collapsedSections.dictionary}>
          <button type="button" class="section-header" on:click={() => toggleSection('dictionary')}>
            <h3>Custom Vocabulary</h3>
            <Icon icon={collapsedSections.dictionary ? 'ph:caret-right' : 'ph:caret-down'} />
          </button>
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
                  <button type="button" class="btn-secondary" disabled={!dictionaryNewTerm.trim() || dictionaryLoading} on:click={addDictionaryTerm}>
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
        </section>

      {:else if activeTab === 'command'}
        <section class="settings-section" class:collapsed={collapsedSections.command}>
          <button type="button" class="section-header" on:click={() => toggleSection('command')}>
            <h3>Command Mode</h3>
            <Icon icon={collapsedSections.command ? 'ph:caret-right' : 'ph:caret-down'} />
          </button>
          <div class="section-content">
            <p class="hint">
              Use a dedicated hotkey to speak commands. Creates notes, tasks, or reminders.
            </p>
            <div class="setting-row checkbox-row">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={config.command_config.enabled} on:change={scheduleSave} />
                <span>Enable Command Mode</span>
              </label>
            </div>

            {#if config.command_config.enabled}
              <div class="setting-row">
                <div class="setting-label">
                  <span class="setting-name">Command Hotkey</span>
                  <span class="setting-desc">Press, then speak naturally</span>
                </div>
                <div class="setting-control">
                  <HotkeyCapture value={config.command_config.hotkey ?? ''} onChange={setCommandHotkey} />
                </div>
              </div>

              <div class="setting-row">
                <div class="setting-label">
                  <span class="setting-name">LLM Provider</span>
                  <span class="setting-desc">Optional: enables natural language understanding</span>
                </div>
                <div class="setting-control">
                  <select value={config.command_config.provider ?? ''} on:change={setCommandProvider}>
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
                      <button class="btn-secondary" disabled={loadingLlmModels || (!commandApiKeyInput.trim() && !config.command_config.api_keys?.[config.command_config.provider])} on:click={fetchCommandLlmModels}>
                        {loadingLlmModels ? 'Testing…' : 'Test & Load'}
                      </button>
                      {#if hasCommandApiKey && !commandApiKeyInput}
                        <button class="btn-secondary danger" on:click={clearCommandApiKey}>Clear</button>
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
        </section>

      {:else if activeTab === 'privacy'}
        <section class="settings-section" class:collapsed={collapsedSections.privacy}>
          <button type="button" class="section-header" on:click={() => toggleSection('privacy')}>
            <h3>Privacy</h3>
            <Icon icon={collapsedSections.privacy ? 'ph:caret-right' : 'ph:caret-down'} />
          </button>
          <div class="section-content">
            <p class="hint">
              <a href="https://kalam.stream/privacy.html" target="_blank">Read our Privacy Policy →</a>
            </p>
            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">History Retention</span>
              </div>
              <div class="setting-control">
                <select bind:value={config.privacy.history_retention_days} on:change={scheduleSave}>
                  <option value={7}>7 days</option>
                  <option value={30}>30 days</option>
                  <option value={90}>90 days</option>
                  <option value={365}>1 year</option>
                  <option value={0}>Forever</option>
                </select>
              </div>
            </div>
            <div class="setting-row checkbox-row">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={config.privacy.sensitive_app_detection} on:change={scheduleSave} />
                <span>Auto-switch to local mode in sensitive apps</span>
              </label>
            </div>
            <div class="setting-row checkbox-row">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={config.privacy.telemetry_enabled} on:change={scheduleSave} />
                <span>Send anonymous usage data to improve Kalam</span>
              </label>
              <span class="setting-desc">No audio or text is ever sent</span>
            </div>
          </div>
        </section>

      {:else if activeTab === 'advanced'}
        <section class="settings-section" class:collapsed={collapsedSections.advanced}>
          <button type="button" class="section-header" on:click={() => toggleSection('advanced')}>
            <h3>App Data & Logging</h3>
            <Icon icon={collapsedSections.advanced ? 'ph:caret-right' : 'ph:caret-down'} />
          </button>
          <div class="section-content">
            <p class="hint">Logs are stored in memory and database—no separate .log files.</p>

            <div class="setting-row">
              <div class="setting-label full-width">
                <span class="setting-name">App Data Folder</span>
                {#if appDataPath}
                  <code class="path-display">{appDataPath}</code>
                {/if}
                {#if openFolderError}
                  <p class="hint error">Failed to open: {openFolderError}</p>
                {/if}
                <button type="button" class="btn-secondary" on:click={openAppDataFolder}>
                  Open folder
                </button>
              </div>
            </div>

            <div class="setting-row checkbox-row">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={config.logging.enabled} on:change={scheduleSave} />
                <span>Enable in-app logging</span>
              </label>
            </div>

            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">Log Level</span>
              </div>
              <div class="setting-control">
                <select bind:value={config.logging.level} on:change={scheduleSave}>
                  <option value="Off">Off</option>
                  <option value="Error">Error</option>
                  <option value="Warn">Warn</option>
                  <option value="Info">Info</option>
                  <option value="Debug">Debug</option>
                </select>
              </div>
            </div>

            <div class="setting-row">
              <div class="setting-label">
                <span class="setting-name">Max Records</span>
                <span class="setting-desc">Between 500 and 20,000</span>
              </div>
              <div class="setting-control">
                <input type="number" min="500" max="20000" step="500" bind:value={config.logging.max_records} on:change={scheduleSave} />
              </div>
            </div>

            <div class="setting-row">
              <div class="setting-label full-width">
                <span class="setting-name">Export Log</span>
                <div class="button-row">
                  <button class="btn-secondary" on:click={downloadLog}>Download log</button>
                  <button class="btn-secondary" on:click={downloadLogsCsv}>Download CSV</button>
                </div>
                <p class="hint">
                  {#if logEmpty}
                    No log entries yet. Enable logging to capture entries.
                  {:else}
                    Download current buffer or full history from database.
                  {/if}
                </p>
                {#if logExportMessage}
                  <p class="hint error">{logExportMessage}</p>
                {/if}
              </div>
            </div>
          </div>
        </section>

        <section class="settings-section danger-zone" class:collapsed={collapsedSections.advanced_danger}>
          <button type="button" class="section-header" on:click={() => toggleSection('advanced_danger')}>
            <h3>Danger Zone</h3>
            <Icon icon={collapsedSections.advanced_danger ? 'ph:caret-right' : 'ph:caret-down'} />
          </button>
          <div class="section-content">
            <p class="hint">Reset removes all configuration, history, and data.</p>
            {#if resetError}
              <p class="hint error">{resetError}</p>
            {/if}
            <button class="btn-danger" disabled={resetting} on:click={confirmAndReset}>
              {resetting ? 'Resetting…' : 'Reset entire application'}
            </button>
          </div>
        </section>

      {:else if activeTab === 'about'}
        <div class="about-tab">
          <About />
        </div>
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

  .settings-tabs {
    display: flex;
    gap: 4px;
    margin-bottom: var(--space-xl);
    border-bottom: 1px solid var(--border);
    padding-bottom: 1px;
    overflow-x: auto;
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

  .model-radio-row {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    cursor: pointer;
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
    gap: 8px;
    margin-top: 12px;
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

    .settings-tabs {
      gap: 0;
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
