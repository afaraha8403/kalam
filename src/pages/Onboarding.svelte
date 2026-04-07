<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { onMount } from 'svelte'
  import { invoke, listenSafe } from '$lib/backend'
  import HotkeyCapture from '../components/HotkeyCapture.svelte'
  import Icon from '@iconify/svelte'
  import { formatHotkeyForDisplay, superKeyLabel } from '../lib/platformHotkey'
  import { LANGUAGE_OPTIONS, languageLabel } from '../lib/languages'
  import type { AppConfig, AudioDevice, CatalogProvider, DictationMode } from '../types'

  const dispatch = createEventDispatcher<{ complete: void }>()

  let step = 1
  const totalSteps = 7

  /** Wizard-only: chosen after Access; drives defaults on the Engine step and command LLM keys. */
  type SetupPath = 'offline' | 'one_key' | 'best_quality' | 'manual'
  let setupPath: SetupPath | null = null
  /** When `best_quality` + Anthropic, second key for LLM (STT stays OpenAI). */
  let llmApiKey = ''
  /** For best-quality path: reuse OpenAI key for LLM vs separate Anthropic key. */
  let qualityLlmChoice: 'openai' | 'anthropic' = 'openai'
  /** Shown on Try-it step; persisted with the rest of onboarding. */
  let polishDemoEnabled = false
  /** Display name for active dictation mode (config snapshot). */
  let activeModeDisplayName = 'Default'
  let termsAgreed = false
  let userEmail = ''
  let notificationsOptIn = true
  let testingMic = false
  let micLevel = 0
  let levelPollId: ReturnType<typeof setInterval> | null = null
  let noAudioRecorded = false
  let hasRecording = false
  let recordedSamples: number[] = []
  let recordedSampleRate = 0
  let isPlaying = false
  let audioCtx: AudioContext | null = null
  let apiKey = ''
  /** OpenAI API key for command/dictation LLM on the “best quality” path (STT stays Groq). */
  let openaiLlmKey = ''
  let selectedProvider: 'groq' | 'openai' = 'groq'
  let apiKeyValid: boolean | null = null
  let validating = false
  /** Curated provider list for Engine-step cards (matches Settings provider library). */
  let modelCatalog: CatalogProvider[] = []
  let onboardingCatalogLoading = false
  /** Manual path: draft keys per catalog provider id (saved into `provider_keys` on continue/finish). */
  let onboardingManualKeys: Record<string, string> = {}
  let onboardingKeyValid: Record<string, boolean | null> = {}
  let onboardingKeyValidating: Record<string, boolean> = {}
  let selectedMode: 'Cloud' | 'Hybrid' | 'Local' = 'Hybrid'
  let hotkey = ''
  let toggleHotkey = ''
  let addLanguageCode = ''
  let languages: string[] = ['en']
  let platform: 'windows' | 'macos' | 'linux' = 'windows'
  let demoTranscription = ''
  let demoTextarea: HTMLTextAreaElement
  let demoFocused = false
  let unlistenDictation: (() => void) | null = null
  let skipInProgress = false
  let skipError = ''
  let audioDevices: AudioDevice[] = []
  /** '' = system default (same as Settings → Audio Input). */
  let audioDeviceSelection = ''
  /** True while refresh re-lists devices (click feedback + prevent double-fires). */
  let micRefreshBusy = false
  /** True from Record click until `test_microphone_start` succeeds (UI feedback during async start). */
  let micRecordStarting = false
  /** User-visible error when `test_microphone_start` fails (Settings surfaces similarly). */
  let micTestStartError = ''

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
  let permCapsLoadError = ''
  /** macOS: Continue allowed if Accessibility isn’t ready and user defers to Settings. */
  let macAccessibilityDeferLater = false

  function badgeLabelForState(state: string): string {
    if (state === 'granted') return 'Granted'
    if (state === 'needs_action') return 'Needs action'
    return 'Unknown'
  }

  async function loadPermissionsAndCapabilities() {
    permCapsLoading = true
    permCapsLoadError = ''
    try {
      runtimeCaps = (await invoke('get_runtime_capabilities')) as RuntimeCapabilitiesPayload
    } catch (e) {
      permCapsLoadError = e instanceof Error ? e.message : String(e)
      runtimeCaps = null
    } finally {
      permCapsLoading = false
    }
  }

  /** Continue on step 3: mic not blocked; on macOS also need Accessibility granted or deferral checkbox. */
  $: step3ContinueEnabled =
    step !== 3 ||
    permCapsLoadError !== '' ||
    (runtimeCaps != null &&
      runtimeCaps.capture_audio_state !== 'needs_action' &&
      (platform !== 'macos' ||
        runtimeCaps.text_inject_state === 'granted' ||
        macAccessibilityDeferLater))

  /** Short nav labels — match the order of steps below. */
  const stepLabels = [
    'Welcome',
    'Email',
    'Access',
    'Setup',
    'Engine',
    'Shortcuts',
    'Try it',
  ]

  /** Single place for demo / refocus copy so hold vs toggle stays consistent. */
  $: primaryHotkeyDemo = hotkey || toggleHotkey || `Ctrl+${superKeyLabel(platform)}`

  /** Short line next to the demo shortcut for the “Try it” step. */
  $: hotkeyDemoCaption =
    hotkey && toggleHotkey
      ? 'Hold or toggle — then speak'
      : hotkey
        ? 'Hold while speaking — then release'
        : toggleHotkey
          ? 'Press once to start, again to stop — then speak'
          : 'Hold while speaking — then release'

  function isEmailValid(email: string) {
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test((email || '').trim())
  }

  /** Apply preset when user picks a setup path (Engine step reflects these choices). */
  function selectSetupPath(path: SetupPath) {
    setupPath = path
    apiKeyValid = null
    // AI-capable paths default polish on; offline has no LLM so polish stays off.
    polishDemoEnabled = path !== 'offline'
    if (path === 'offline') {
      selectedMode = 'Local'
      apiKey = ''
    } else if (path === 'one_key') {
      selectedMode = 'Hybrid'
      selectedProvider = 'groq'
    } else if (path === 'best_quality') {
      selectedMode = 'Cloud'
      // Phase 2: curated “quality” stack uses Groq STT + separate OpenAI or Anthropic LLM key.
      selectedProvider = 'groq'
      qualityLlmChoice = 'openai'
    } else {
      // manual — sensible defaults without hiding options on the next step
      selectedMode = 'Hybrid'
      if (apiKey.trim() && (selectedProvider === 'groq' || selectedProvider === 'openai')) {
        onboardingManualKeys = { ...onboardingManualKeys, [selectedProvider]: apiKey.trim() }
      }
    }
  }

  /**
   * Single merge point for onboarding fields into `config` before save.
   * Keeps command_mode keys aligned with the chosen setup path (Groq one-key, quality STT+LLM).
   */
  function mergeOnboardingWizardIntoConfig(config: AppConfig) {
    if (!config.provider_keys) config.provider_keys = {}

    if (setupPath === 'best_quality') {
      config.stt_config.provider = 'groq'
      if (apiKey.trim()) config.provider_keys.groq = apiKey.trim()
    } else if (setupPath === 'manual') {
      config.stt_config.provider = selectedProvider
      for (const [pid, k] of Object.entries(onboardingManualKeys)) {
        const t = (k ?? '').trim()
        if (t) config.provider_keys[pid] = t
      }
    } else {
      config.stt_config.provider = selectedProvider
      if (apiKey.trim()) {
        config.provider_keys[selectedProvider] = apiKey.trim()
      }
    }

    config.stt_config.mode = selectedMode
    config.hotkey = hotkey
    config.toggle_dictation_hotkey = toggleHotkey || null
    config.languages = languages.length ? [...languages] : ['en']
    config.user_email = userEmail.trim() || null
    config.marketing_opt_in = false
    config.notifications_opt_in = notificationsOptIn
    const mic = audioDeviceSelection.trim()
    config.audio_device = mic === '' ? null : mic
    // Set polish on the default mode based on onboarding choice
    const defaultMode = config.modes.find((m) => m.id === 'default')
    if (defaultMode) {
      defaultMode.polish = polishDemoEnabled
    }

    if (setupPath === 'one_key') {
      config.command_config.enabled = true
      config.default_llm_provider = 'groq'
      if (apiKey.trim()) config.provider_keys.groq = apiKey.trim()
    } else if (setupPath === 'best_quality') {
      config.command_config.enabled = true
      if (qualityLlmChoice === 'anthropic') {
        config.default_llm_provider = 'anthropic'
        if (llmApiKey.trim()) config.provider_keys.anthropic = llmApiKey.trim()
      } else {
        config.default_llm_provider = 'openai'
        if (openaiLlmKey.trim()) config.provider_keys.openai = openaiLlmKey.trim()
      }
    } else if (setupPath === 'offline') {
      config.command_config.enabled = false
      config.default_llm_provider = null
    }
  }

  async function refreshActiveModeDisplay() {
    try {
      const cfg = (await invoke('get_settings')) as AppConfig
      const modes = cfg.modes as DictationMode[]
      const aid = cfg.active_mode_id ?? 'default'
      activeModeDisplayName = modes.find((m) => m.id === aid)?.name ?? aid
    } catch {
      /* non-fatal */
    }
  }

  async function loadConfig() {
    try {
      const config = (await invoke('get_settings')) as AppConfig
      if (config.stt_config?.provider) selectedProvider = config.stt_config.provider as 'groq' | 'openai'
      if (config.stt_config?.mode) selectedMode = config.stt_config.mode as 'Cloud' | 'Hybrid' | 'Local'
      if (!config.stt_config?.api_keys) config.stt_config.api_keys = {}
      if (!config.provider_keys) config.provider_keys = {}
      if (config.stt_config?.api_key && !config.stt_config.api_keys[selectedProvider]) {
        config.stt_config.api_keys[selectedProvider] = config.stt_config.api_key
      }
      const pk = config.provider_keys[selectedProvider]
      if (pk) apiKey = pk
      else if (config.stt_config?.api_keys?.[selectedProvider]) apiKey = config.stt_config.api_keys[selectedProvider]
      llmApiKey = config.provider_keys.anthropic?.trim() ?? ''
      openaiLlmKey = config.provider_keys.openai?.trim() ?? ''
      {
        const cloudIds = ['groq', 'openai', 'anthropic', 'gemini', 'openrouter'] as const
        const next: Record<string, string> = {}
        for (const id of cloudIds) {
          const v = config.provider_keys[id]?.trim()
          if (v) next[id] = v
        }
        onboardingManualKeys = next
      }
      if (config.hotkey) {
        hotkey = formatHotkeyForDisplay(config.hotkey, platform)
      } else {
        hotkey = `Ctrl+${superKeyLabel(platform)}`
      }
      if (config.toggle_dictation_hotkey) {
        toggleHotkey = formatHotkeyForDisplay(config.toggle_dictation_hotkey, platform)
      }
      if (config.languages?.length) languages = [...config.languages]
      if (config.user_email) userEmail = config.user_email
      if (config.notifications_opt_in != null) notificationsOptIn = config.notifications_opt_in
      const modes = config.modes as DictationMode[]
      const defaultMode = modes.find((m) => m.id === 'default')
      if (defaultMode) polishDemoEnabled = defaultMode.polish
      const aid = config.active_mode_id ?? 'default'
      activeModeDisplayName = modes.find((m) => m.id === aid)?.name ?? aid
      const ad = config.audio_device
      // '' = explicit “System default” row; 'default' / 'device_N' = real ids from list_devices (never duplicate option values).
      if (ad == null || ad === '') {
        audioDeviceSelection = ''
      } else {
        audioDeviceSelection = ad
      }
      lastSavedDevice = ad == null || ad === '' ? null : ad
      deviceInitComplete = true
    } catch (e) {
      console.error('Onboarding load config failed:', e)
    }
  }

  async function loadAudioDevices() {
    try {
      audioDevices = (await invoke('get_audio_devices')) as AudioDevice[]
    } catch (e) {
      console.error('[Onboarding] list audio devices failed:', e)
      audioDevices = []
    }
  }

  async function handleRefreshMicList() {
    if (micRefreshBusy) return
    micRefreshBusy = true
    try {
      await loadAudioDevices()
    } finally {
      micRefreshBusy = false
    }
  }

  /** Debounced save for audio device changes (mirrors Settings behavior). */
  let deviceSaveTimeout: ReturnType<typeof setTimeout> | null = null
  let lastSavedDevice: string | null = null
  let deviceInitComplete = false
  async function saveAudioDevice() {
    if (deviceSaveTimeout) clearTimeout(deviceSaveTimeout)
    deviceSaveTimeout = setTimeout(async () => {
      try {
        // Skip saves until initial config load is done
        if (!deviceInitComplete) return
        const t = audioDeviceSelection.trim()
        const deviceToSave = t === '' ? null : t
        if (lastSavedDevice === deviceToSave) return
        const config = (await invoke('get_settings')) as AppConfig
        config.audio_device = deviceToSave
        await invoke('save_settings', { newConfig: config })
        lastSavedDevice = deviceToSave
      } catch (e) {
        console.error('[Onboarding] save audio device failed:', e)
      }
    }, 150)
  }

  // Reactive save trigger: fires AFTER bind:value updates the variable
  $: if (audioDeviceSelection !== undefined && !micRefreshBusy && deviceInitComplete) {
    saveAudioDevice()
  }

  type SaveOnboardingOpts = { captureOsForEmailStep?: boolean }

  async function saveOnboardingState(opts?: SaveOnboardingOpts) {
    try {
      const config = (await invoke('get_settings')) as AppConfig
      mergeOnboardingWizardIntoConfig(config)
      // Snapshot OS when leaving the email step (Continue)—for support / diagnostics alongside `user_email`.
      if (opts?.captureOsForEmailStep) {
        try {
          const rel = (await invoke('get_os_release_info')) as { name: string; version: string }
          const n = (rel.name ?? '').trim()
          const v = (rel.version ?? '').trim()
          config.onboarding_os_name = n || null
          config.onboarding_os_version = v || null
        } catch (e) {
          console.error('Onboarding: get_os_release_info failed:', e)
          config.onboarding_os_name =
            platform === 'macos' ? 'macOS' : platform === 'linux' ? 'Linux' : 'Windows'
          config.onboarding_os_version = null
        }
      }
      await invoke('save_settings', { newConfig: config })
    } catch (e) {
      console.error('Onboarding save state failed:', e)
    }
  }

  async function nextStep() {
    if (step === 2) {
      if (!termsAgreed || !isEmailValid(userEmail)) return
      await saveOnboardingState({ captureOsForEmailStep: true })
    }
    // Save when leaving Engine (step 5) and Shortcuts (step 6) so keys persist if the user bails early
    if (step === 5) await saveOnboardingState()
    if (step === 6) await saveOnboardingState()
    if (step < totalSteps) {
      step++
      if (step === 7) await refreshActiveModeDisplay()
    }
  }

  function prevStep() {
    if (step > 1) step--
  }

  function goToStep(target: number) {
    if (target < step) step = target
  }

  function removeLanguage(index: number) {
    if (languages.length <= 1) return
    languages = languages.filter((_, j) => j !== index)
  }

  function addLanguage(code: string) {
    if (!code || languages.includes(code)) return
    languages = [...languages, code]
    addLanguageCode = ''
  }

  async function startRecording() {
    if (micRecordStarting || testingMic) return
    noAudioRecorded = false
    hasRecording = false
    recordedSamples = []
    recordedSampleRate = 0
    micLevel = 0
    micRecordStarting = true
    micTestStartError = ''
    try {
      audioCtx = new (window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext)()
      await audioCtx.resume()
      await invoke('test_microphone_start')
      testingMic = true
      levelPollId = setInterval(async () => {
        try {
          micLevel = (await invoke('test_microphone_level')) as number
        } catch {
          // ignore
        }
      }, 100)
    } catch (e) {
      console.error('Microphone test start failed:', e)
      micTestStartError = e instanceof Error ? e.message : String(e)
      testingMic = false
      if (levelPollId != null) {
        clearInterval(levelPollId)
        levelPollId = null
      }
    } finally {
      micRecordStarting = false
    }
  }

  async function stopRecording() {
    if (levelPollId != null) {
      clearInterval(levelPollId)
      levelPollId = null
    }
    try {
      const result = (await invoke('test_microphone_stop')) as {
        level: number
        samples: number[]
        sample_rate: number
      }
      if (result.samples?.length && result.sample_rate) {
        recordedSamples = result.samples
        recordedSampleRate = result.sample_rate
        hasRecording = true
      } else {
        noAudioRecorded = true
      }
    } catch (e) {
      console.error(e)
      noAudioRecorded = true
    }
    testingMic = false
  }

  async function playRecording() {
    if (!recordedSamples.length || isPlaying) return
    const ctx = audioCtx ?? new (window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext)()
    try {
      if (ctx.state === 'suspended') await ctx.resume()
      const buffer = ctx.createBuffer(1, recordedSamples.length, recordedSampleRate)
      buffer.getChannelData(0).set(new Float32Array(recordedSamples))
      const source = ctx.createBufferSource()
      source.buffer = buffer
      source.connect(ctx.destination)
      isPlaying = true
      source.onended = () => { isPlaying = false }
      source.start(0)
    } catch (e) {
      console.error('Playback failed:', e)
      isPlaying = false
    }
  }

  async function loadPlatform() {
    try {
      const os = (await invoke('get_platform')) as string
      if (os === 'macos' || os === 'linux') platform = os
      else platform = 'windows'
    } catch {
      platform = 'windows'
    }
  }

  function openPermissionPage(permission: 'microphone' | 'accessibility') {
    invoke('open_system_permission_page', { permission }).catch((e) => console.error(e))
  }

  function requestPermission(permission: 'microphone' | 'accessibility') {
    invoke('request_system_permission', { permission }).catch((e) => console.error(e))
  }

  /**
   * Explicit paste handler for the API key field.
   * WebView2 on Windows can silently swallow the default paste action on certain inputs;
   * using clipboardData (synchronous, no permission prompt) ensures the value arrives.
   */
  function onApiKeyPaste(e: ClipboardEvent) {
    const text = e.clipboardData?.getData('text/plain')
    if (text) {
      e.preventDefault()
      apiKey = text.trim()
      // Reset validation state when key changes
      apiKeyValid = null
    }
  }

  /**
   * Validate the API key with the provider.
   */
  async function checkApiKey() {
    const keyToCheck = apiKey.trim()
    if (!keyToCheck) return
    validating = true
    try {
      apiKeyValid = (await invoke('test_provider_key', {
        provider: selectedProvider,
        apiKey: keyToCheck,
      })) as boolean
    } catch (e) {
      console.error('API key validation failed:', e)
      apiKeyValid = false
    } finally {
      validating = false
    }
  }

  function eventInputValue(e: Event): string {
    const t = e.currentTarget
    return t instanceof HTMLInputElement ? t.value : ''
  }

  function catalogProviderById(id: string): CatalogProvider | undefined {
    return modelCatalog.find((p) => p.id === id)
  }

  async function loadOnboardingCatalog() {
    if (onboardingCatalogLoading || modelCatalog.length > 0) return
    onboardingCatalogLoading = true
    try {
      modelCatalog = (await invoke('get_model_catalog')) as CatalogProvider[]
    } catch (e) {
      console.error('[Onboarding] get_model_catalog failed:', e)
      modelCatalog = []
    } finally {
      onboardingCatalogLoading = false
    }
  }

  $: if (step === 5) void loadOnboardingCatalog()

  async function validateOnboardingCatalogKey(providerId: string, key: string) {
    const k = key.trim()
    if (!k) return
    onboardingKeyValidating = { ...onboardingKeyValidating, [providerId]: true }
    try {
      const ok = (await invoke('test_provider_key', { provider: providerId, apiKey: k })) as boolean
      onboardingKeyValid = { ...onboardingKeyValid, [providerId]: ok }
    } catch (e) {
      console.error('[Onboarding] test_provider_key failed:', e)
      onboardingKeyValid = { ...onboardingKeyValid, [providerId]: false }
    } finally {
      onboardingKeyValidating = { ...onboardingKeyValidating, [providerId]: false }
    }
  }

  $: if (step === 3) {
    void loadPermissionsAndCapabilities()
  }

  onMount(() => {
    const setup = async () => {
      const unlisten = await listenSafe<string>('dictation-result', (e) => {
        if (typeof e.payload === 'string') {
          demoTranscription = e.payload
        }
      })
      unlistenDictation = unlisten
    }
    void setup()
    void loadPlatform()
      .then(() => loadConfig())
      .then(() => loadAudioDevices())
      .then(() => loadPermissionsAndCapabilities())
    return () => {
      unlistenDictation?.()
    }
  })

  async function finish() {
    try {
      const config = (await invoke('get_settings')) as AppConfig
      config.onboarding_complete = true
      mergeOnboardingWizardIntoConfig(config)
      await invoke('save_settings', { newConfig: config })
      dispatch('complete')
    } catch (e) {
      console.error('Onboarding save failed:', e)
    }
  }

  $: if (step === 7 && demoTextarea) {
    setTimeout(() => demoTextarea?.focus(), 100)
  }

  async function handleSkipClick(e: MouseEvent) {
    e.preventDefault()
    e.stopPropagation()
    if (skipInProgress) return
    skipError = ''
    skipInProgress = true
    try {
      await invoke('skip_onboarding_with_defaults')
      dispatch('complete')
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err)
      console.error('Skip onboarding failed:', err)
      skipError = msg
    } finally {
      skipInProgress = false
    }
  }
</script>

<div class="onboarding">
  <!-- Left stepper nav -->
  <nav class="stepper" aria-label="Onboarding steps">
    <div class="stepper-brand">
      <img src="/logo/kalam-logo-icon.svg" alt="Kalam" class="stepper-logo" />
      <h1 class="stepper-title">Kalam</h1>
    </div>
    <ol class="stepper-list">
      {#each stepLabels as label, i}
        {@const num = i + 1}
        {@const isComplete = num < step}
        {@const isActive = num === step}
        <li class="stepper-item" class:complete={isComplete} class:active={isActive}>
          <button
            class="stepper-btn"
            disabled={num > step}
            on:click={() => goToStep(num)}
            aria-current={isActive ? 'step' : undefined}
          >
            <span class="stepper-indicator">
              {#if isComplete}
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
              {:else}
                {num}
              {/if}
            </span>
            <span class="stepper-label">{label}</span>
          </button>
          {#if i < totalSteps - 1}
            <div class="stepper-line" class:filled={isComplete}></div>
          {/if}
        </li>
      {/each}
    </ol>

    {#if step >= 3}
      <div class="stepper-skip">
        <button
          type="button"
          class="btn-skip"
          disabled={skipInProgress}
          on:click={handleSkipClick}
        >
          {skipInProgress ? 'Skipping…' : 'Use defaults & skip'}
        </button>
        {#if skipError}
          <p class="skip-error" role="alert">{skipError}</p>
        {/if}
        <p class="skip-consequence">
          Skipping applies defaults and may leave permissions unset—you can fix microphone and accessibility in Settings → Audio &amp; Dictation.
        </p>
      </div>
    {/if}
  </nav>

  <!-- Mobile stepper (horizontal) -->
  <div class="stepper-mobile" aria-hidden="true">
    <div class="stepper-mobile-brand">
      <img src="/logo/kalam-logo-icon.svg" alt="Kalam" class="stepper-logo-sm" />
    </div>
    <div class="stepper-dots">
      {#each stepLabels as _, i}
        {@const num = i + 1}
        <span
          class="dot"
          class:complete={num < step}
          class:active={num === step}
        ></span>
      {/each}
    </div>
    <span class="stepper-mobile-text">{step}/{totalSteps}</span>
  </div>

  <!-- Right content area -->
  <div class="content-area">
    <div class="content-scroll">
      {#key step}
      <div class="step-shell">
      {#if step === 1}
        <div class="step step-welcome">
          <p class="step-eyebrow" aria-hidden="true">Step {step} of {totalSteps}</p>
          <h1>Welcome to Kalam</h1>
          <p class="subtitle">Dictate into any app with a global shortcut—fast when you’re online, still usable offline with the right setup.</p>
          <p class="step-meta">Most people finish setup in under two minutes. You can change almost everything later in Settings.</p>

          <div class="features-grid">
            <div class="feat">
              <span class="feat-icon" aria-hidden="true"><Icon icon="ph:lightning" /></span>
              <div>
                <strong>Low-friction flow</strong>
                <span>Hold a shortcut to talk, release to insert text—no copy-paste dance.</span>
              </div>
            </div>
            <div class="feat">
              <span class="feat-icon" aria-hidden="true"><Icon icon="ph:lock-key" /></span>
              <div>
                <strong>You stay in control</strong>
                <span>Choose cloud, hybrid, or local so audio and data follow your comfort level.</span>
              </div>
            </div>
            <div class="feat">
              <span class="feat-icon" aria-hidden="true"><Icon icon="ph:cursor-click" /></span>
              <div>
                <strong>Works system-wide</strong>
                <span>Type into browsers, docs, chat—wherever the cursor is.</span>
              </div>
            </div>
            <div class="feat">
              <span class="feat-icon" aria-hidden="true"><Icon icon="ph:globe-hemisphere-west" /></span>
              <div>
                <strong>Many languages</strong>
                <span>Multilingual lists and auto-punctuation help polished output.</span>
              </div>
            </div>
          </div>
        </div>

      {:else if step === 2}
        <div class="step step-account">
          <p class="step-eyebrow" aria-hidden="true">Step {step} of {totalSteps}</p>
          <h1>Email &amp; terms</h1>
          <p class="subtitle">We need a valid email for support and account-related notices. Optional emails are only sent if you opt in below.</p>

          <div class="form-card">
            <div class="field">
              <label for="onboarding-email">Email address <span class="req">*</span></label>
              <input
                id="onboarding-email"
                type="email"
                placeholder="you@example.com"
                autocomplete="email"
                bind:value={userEmail}
              />
              {#if userEmail && !isEmailValid(userEmail)}
                <p class="field-error">Use a valid email (we’ll use it only as described).</p>
              {/if}
              <p class="privacy-note">We don’t sell your email. Third-party marketing lists aren’t part of this step.</p>
            </div>
            <div class="checkboxes">
              <label class="check-row">
                <input type="checkbox" bind:checked={termsAgreed} />
                <span>I’ve read and agree to the <a href="https://kalam.stream/terms.html" target="_blank" rel="noopener noreferrer">Terms</a> and <a href="https://kalam.stream/privacy.html" target="_blank" rel="noopener noreferrer">Privacy Policy</a>.</span>
              </label>
              <label class="check-row">
                <input type="checkbox" bind:checked={notificationsOptIn} />
                <span>Email me product updates and tips (optional).</span>
              </label>
            </div>
            <p class="step-footnote">Continue is enabled when your email is valid and terms are accepted.</p>
          </div>
        </div>

      {:else if step === 3}
        <div class="step step-permissions">
          <p class="step-eyebrow" aria-hidden="true">Step {step} of {totalSteps}</p>
          <h1>Access &amp; microphone</h1>
          <p class="subtitle">
            Check status below, then confirm the mic picks you up. Kalam needs microphone access for dictation.
            {#if platform === 'macos'}
              On this Mac, <strong>Accessibility</strong> is also required so Kalam can insert text directly into other
              apps (without relying on your clipboard).
            {:else}
              Use the status rows below for microphone, text insertion, and global hotkey where applicable.
            {/if}
          </p>

          <div class="perm-status-strip" aria-live="polite">
            {#if permCapsLoading}
              <p class="perm-strip-loading">Checking permissions…</p>
            {:else if permCapsLoadError}
              <p class="error-text" role="alert">{permCapsLoadError}</p>
            {:else if runtimeCaps}
              <div class="perm-status-row">
                <div class="perm-status-head">
                  <span class="perm-status-title">Microphone</span>
                  <span
                    class="perm-badge"
                    class:perm-badge--granted={runtimeCaps.capture_audio_state === 'granted'}
                    class:perm-badge--needs={runtimeCaps.capture_audio_state === 'needs_action'}
                    class:perm-badge--unknown={runtimeCaps.capture_audio_state === 'unknown'}
                    >{badgeLabelForState(runtimeCaps.capture_audio_state)}</span
                  >
                </div>
                <p class="perm-status-msg">{runtimeCaps.permission_status.microphone.message}</p>
                <div class="perm-status-actions">
                  {#if platform === 'macos'}
                    <button type="button" class="btn-outline-sm" on:click={() => openPermissionPage('microphone')}>
                      Open Microphone settings
                    </button>
                    <span class="perm-status-hint-inline"
                      >Starting <strong>Record sample</strong> below triggers the system prompt if needed.</span
                    >
                  {:else if platform === 'windows'}
                    {#if runtimeCaps.permission_status.microphone.actionable}
                      <button type="button" class="btn-outline-sm" on:click={() => openPermissionPage('microphone')}>
                        Open microphone privacy settings
                      </button>
                    {/if}
                  {:else}
                    <span class="perm-status-hint-inline"
                      >Use the mic test below; PipeWire/PulseAudio and device access vary by distro.</span
                    >
                  {/if}
                </div>
              </div>

              <div class="perm-status-row">
                <div class="perm-status-head">
                  <span class="perm-status-title">Text insertion</span>
                  <span
                    class="perm-badge"
                    class:perm-badge--granted={runtimeCaps.text_inject_state === 'granted'}
                    class:perm-badge--needs={runtimeCaps.text_inject_state === 'needs_action'}
                    class:perm-badge--unknown={runtimeCaps.text_inject_state === 'unknown'}
                    >{badgeLabelForState(runtimeCaps.text_inject_state)}</span
                  >
                </div>
                <p class="perm-status-msg">{runtimeCaps.permission_status.accessibility.message}</p>
                <div class="perm-status-actions">
                  {#if platform === 'macos'}
                    {#if runtimeCaps.text_inject_state !== 'granted'}
                      <button type="button" class="btn-outline-sm" on:click={() => requestPermission('accessibility')}>
                        Request accessibility prompt
                      </button>
                      <button type="button" class="btn-outline-sm" on:click={() => openPermissionPage('accessibility')}>
                        Open Accessibility settings
                      </button>
                    {/if}
                  {:else if platform === 'windows'}
                    <span class="perm-auto">No separate accessibility toggle is usually required on Windows.</span>
                  {:else}
                    <p class="perm-linux-msg">
                      Text injection depends on your desktop environment; there isn’t a single settings link for all Linux setups.
                    </p>
                  {/if}
                </div>
              </div>

              <div class="perm-status-row">
                <div class="perm-status-head">
                  <span class="perm-status-title">Global hotkey</span>
                  <span
                    class="perm-badge"
                    class:perm-badge--granted={runtimeCaps.global_hotkey_state === 'granted'}
                    class:perm-badge--needs={runtimeCaps.global_hotkey_state === 'needs_action'}
                    class:perm-badge--unknown={runtimeCaps.global_hotkey_state === 'unknown'}
                    >{badgeLabelForState(runtimeCaps.global_hotkey_state)}</span
                  >
                </div>
                <p class="perm-status-msg">{runtimeCaps.permission_status.input_monitoring.message}</p>
              </div>

              {#if platform === 'macos' && runtimeCaps.text_inject_state !== 'granted'}
                <label class="check-row perm-defer-check">
                  <input type="checkbox" bind:checked={macAccessibilityDeferLater} />
                  <span>I’ll enable Accessibility later in System Settings</span>
                </label>
                <p class="perm-defer-consequence hint">
                  Without Accessibility, Kalam will use <strong>clipboard paste</strong> to insert text, which
                  temporarily replaces your clipboard contents.
                </p>
              {/if}
            {/if}
          </div>

          <p class="step-meta perm-privacy-footnote">
            The sample you record below is only used to verify levels during onboarding; it isn’t kept as a file by this step.
          </p>

          <div class="mic-test">
            <div class="mic-test-header">
              <h3 class="mic-test-title">Microphone check</h3>
              <button
                type="button"
                class="mic-info-btn"
                title="Same list as Settings → Audio. Record a short clip, then play it back to confirm levels."
                aria-label="Details: same microphone list as Settings under Audio. Record a short clip, then use Play back to confirm levels."
              >
                <Icon icon="ph:info" />
              </button>
            </div>
            <div class="mic-device-row">
              <div class="mic-device-controls">
                <select
                  id="onboarding-mic-device"
                  class="mic-device-select"
                  aria-label="Microphone input. Same list as Settings; saves immediately for this test and dictation after setup."
                  bind:value={audioDeviceSelection}
                >
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
                <button
                  type="button"
                  class="mic-refresh-btn"
                  class:mic-refresh-btn--busy={micRefreshBusy}
                  title="Refresh device list"
                  aria-label="Refresh microphone list"
                  aria-busy={micRefreshBusy}
                  disabled={micRefreshBusy}
                  on:click={() => handleRefreshMicList()}
                >
                  <Icon icon="ph:arrow-clockwise" />
                </button>
              </div>
            </div>
            {#if audioDevices.length === 0}
              <p class="mic-device-warning" role="status">
                <span>No input devices found.</span>
                <button
                  type="button"
                  class="mic-info-btn"
                  title="Try Refresh. You can set the mic later in Settings → Audio &amp; Dictation."
                  aria-label="Details: try Refresh to scan again. You can set the microphone later under Settings, Audio and Dictation."
                >
                  <Icon icon="ph:info" />
                </button>
              </p>
            {/if}
            {#if micTestStartError}
              <p class="mic-test-start-error" role="alert">{micTestStartError}</p>
            {/if}
            <div class="mic-test-body">
              <div class="mic-controls">
                {#if testingMic}
                  <button class="mic-action-btn recording" on:click={stopRecording}>
                    <span class="mic-action-icon" aria-hidden="true"><Icon icon="ph:stop-fill" /></span>
                    <span>Stop</span>
                  </button>
                  <div class="mic-level-bar">
                    <div class="mic-level-fill" style="width: {Math.min(micLevel * 100, 100)}%"></div>
                  </div>
                {:else}
                  <button
                    type="button"
                    class="mic-action-btn"
                    class:mic-action-btn--starting={micRecordStarting}
                    disabled={micRecordStarting}
                    aria-busy={micRecordStarting}
                    on:click={startRecording}
                  >
                    <span class="mic-action-icon" aria-hidden="true">
                      <Icon icon={micRecordStarting ? 'ph:circle-notch' : 'ph:microphone'} />
                    </span>
                    <span>{micRecordStarting ? 'Starting…' : 'Record sample'}</span>
                  </button>
                  {#if hasRecording}
                    <button class="mic-action-btn play" class:playing={isPlaying} on:click={playRecording} disabled={isPlaying}>
                      <span class="mic-action-icon" aria-hidden="true">
                        <Icon icon={isPlaying ? 'ph:speaker-high-fill' : 'ph:play-fill'} />
                      </span>
                      <span>{isPlaying ? 'Playing…' : 'Play back'}</span>
                    </button>
                  {/if}
                {/if}
              </div>
              <p class="mic-status">
                {#if testingMic}
                  <span>Recording—tap <strong>Stop</strong> when done.</span>
                  <button
                    type="button"
                    class="mic-info-btn"
                    title="Speak at a normal volume so the level meter can confirm the mic is working."
                    aria-label="Details: speak at a normal volume so the level meter can confirm the microphone is working."
                  >
                    <Icon icon="ph:info" />
                  </button>
                {:else if noAudioRecorded}
                  <span class="error-text">No audio captured.</span>
                  <button
                    type="button"
                    class="mic-info-btn mic-info-btn--error-context"
                    title="Check the mic isn’t muted, try another input above, or open system privacy settings for the microphone."
                    aria-label="Details: check the microphone is not muted, try another input device, or open system privacy settings for microphone access."
                  >
                    <Icon icon="ph:info" />
                  </button>
                {:else if hasRecording}
                  <span>Tap <strong>Play back</strong> to verify.</span>
                  <button
                    type="button"
                    class="mic-info-btn"
                    title="If you hear yourself clearly, you’re set. Use Play back again any time."
                    aria-label="Details: if you hear yourself clearly, you are all set. You can use Play back again any time."
                  >
                    <Icon icon="ph:info" />
                  </button>
                {:else}
                  <span>Record a sample, then play it back.</span>
                  <button
                    type="button"
                    class="mic-info-btn"
                    title="Tap Record sample, speak briefly, then Stop. Use Play back to listen."
                    aria-label="Details: tap Record sample, speak briefly, then Stop. Use Play back to listen to the recording."
                  >
                    <Icon icon="ph:info" />
                  </button>
                {/if}
              </p>
            </div>
          </div>
        </div>

      {:else if step === 4}
        <div class="step step-setup-path">
          <p class="step-eyebrow" aria-hidden="true">Step {step} of {totalSteps}</p>
          <h1>How do you want to set up?</h1>
          <p class="subtitle">Pick a path—we’ll tune the next step. You can change everything later in Settings and the Dictation page.</p>

          <div class="setup-path-grid" role="radiogroup" aria-label="Setup path">
            <button
              type="button"
              class="setup-path-card"
              class:active={setupPath === 'offline'}
              aria-checked={setupPath === 'offline'}
              role="radio"
              on:click={() => selectSetupPath('offline')}
            >
              <span class="setup-path-icon" aria-hidden="true"><Icon icon="ph:wifi-slash" /></span>
              <span class="setup-path-title">Fully offline</span>
              <span class="setup-path-desc">Local speech-to-text only. Download a model after setup—no API keys.</span>
            </button>
            <button
              type="button"
              class="setup-path-card"
              class:active={setupPath === 'one_key'}
              aria-checked={setupPath === 'one_key'}
              role="radio"
              on:click={() => selectSetupPath('one_key')}
            >
              <span class="setup-path-icon" aria-hidden="true"><Icon icon="ph:key" /></span>
              <span class="setup-path-title">One key (Groq)</span>
              <span class="setup-path-desc">Single free-tier key for cloud STT and command/AI features when you’re online.</span>
            </button>
            <button
              type="button"
              class="setup-path-card"
              class:active={setupPath === 'best_quality'}
              aria-checked={setupPath === 'best_quality'}
              role="radio"
              on:click={() => selectSetupPath('best_quality')}
            >
              <span class="setup-path-icon" aria-hidden="true"><Icon icon="ph:sparkle" /></span>
              <span class="setup-path-title">Best quality</span>
              <span class="setup-path-desc">Groq for transcription; add OpenAI or Anthropic for AI / command modes.</span>
            </button>
            <button
              type="button"
              class="setup-path-card"
              class:active={setupPath === 'manual'}
              aria-checked={setupPath === 'manual'}
              role="radio"
              on:click={() => selectSetupPath('manual')}
            >
              <span class="setup-path-icon" aria-hidden="true"><Icon icon="ph:sliders-horizontal" /></span>
              <span class="setup-path-title">I already have keys</span>
              <span class="setup-path-desc">Configure cloud/local mix and providers yourself on the next screen.</span>
            </button>
          </div>
        </div>

      {:else if step === 5}
        <div class="step step-mode">
          <p class="step-eyebrow" aria-hidden="true">Step {step} of {totalSteps}</p>
          <h1>Speech engine</h1>
          <p class="subtitle">Choose how your voice becomes text. Change anytime in Settings → AI providers.</p>

          {#if setupPath === 'offline'}
            <p class="setup-path-banner">
              <Icon icon="ph:info" />
              <span><strong>Offline path:</strong> you’ll use a local model. After onboarding, open <strong>Settings → AI providers</strong> to download and start an engine.</span>
            </p>
          {:else if setupPath === 'one_key'}
            <p class="setup-path-banner">
              <Icon icon="ph:info" />
              <span><strong>Groq path:</strong> Hybrid mode is selected so you can fall back when offline once a local model is set up. Paste your Groq key below for cloud STT and AI.</span>
            </p>
          {:else if setupPath === 'best_quality'}
            <p class="setup-path-banner">
              <Icon icon="ph:info" />
              <span><strong>Quality path:</strong> Groq for cloud transcription plus a separate key for OpenAI or Anthropic to power command mode and recipes.</span>
            </p>
            <div class="quality-llm-row">
              <span class="quality-llm-label">AI / command LLM</span>
              <div class="quality-llm-pills">
                <button
                  type="button"
                  class="prov-pill"
                  class:active={qualityLlmChoice === 'openai'}
                  on:click={() => (qualityLlmChoice = 'openai')}
                >OpenAI</button>
                <button
                  type="button"
                  class="prov-pill"
                  class:active={qualityLlmChoice === 'anthropic'}
                  on:click={() => (qualityLlmChoice = 'anthropic')}
                >Anthropic</button>
              </div>
            </div>
          {/if}

          <div class="stt-tabs" role="tablist" aria-label="Speech processing mode">
            <button
              type="button"
              role="tab"
              class="stt-tab"
              class:active={selectedMode === 'Cloud'}
              aria-selected={selectedMode === 'Cloud'}
              disabled={setupPath != null && setupPath !== 'manual'}
              on:click={() => (selectedMode = 'Cloud')}
            >
              <span class="stt-tab-icon" aria-hidden="true"><Icon icon="ph:cloud" /></span>
              <span class="stt-tab-label">Cloud</span>
            </button>
            <button
              type="button"
              role="tab"
              class="stt-tab recommended"
              class:active={selectedMode === 'Hybrid'}
              aria-selected={selectedMode === 'Hybrid'}
              disabled={setupPath != null && setupPath !== 'manual'}
              on:click={() => (selectedMode = 'Hybrid')}
            >
              <span class="stt-tab-icon" aria-hidden="true"><Icon icon="ph:arrows-clockwise" /></span>
              <span class="stt-tab-label">Hybrid</span>
              <span class="stt-rec-badge" aria-hidden="true">Recommended</span>
            </button>
            <button
              type="button"
              role="tab"
              class="stt-tab"
              class:active={selectedMode === 'Local'}
              aria-selected={selectedMode === 'Local'}
              disabled={setupPath != null && setupPath !== 'manual'}
              on:click={() => (selectedMode = 'Local')}
            >
              <span class="stt-tab-icon" aria-hidden="true"><Icon icon="ph:laptop" /></span>
              <span class="stt-tab-label">Local</span>
            </button>
          </div>

          <div class="mode-detail">
            {#if selectedMode === 'Cloud'}
              <div class="mode-chips">
                <span class="mode-chip fast">Fastest transcription</span>
                <span class="mode-chip">Many languages</span>
                <span class="mode-chip needs-key">Requires API key</span>
              </div>
              <p class="mode-blurb">Audio is sent to the provider you choose. Paste your key now or add it later in Settings.</p>
            {:else if selectedMode === 'Hybrid'}
              <div class="mode-chips">
                <span class="mode-chip rec">Best default</span>
                <span class="mode-chip fast">Cloud online</span>
                <span class="mode-chip offline">Local offline</span>
              </div>
              <p class="mode-blurb">Fast when you are connected, keeps working when you are not.</p>
              <div class="mode-info-box">
                <span class="mode-info-icon" aria-hidden="true"><Icon icon="ph:info" /></span>
                <span class="mode-info-text">You can set up your local model from <strong>Settings → AI providers</strong> after you finish onboarding.</span>
              </div>
            {:else}
              <div class="mode-chips">
                <span class="mode-chip">Fully offline</span>
                <span class="mode-chip privacy">No cloud upload</span>
                <span class="mode-chip disk">Uses disk and CPU</span>
              </div>
              <p class="mode-blurb">Download a model from Settings → AI providers after onboarding. Dictation will be limited until you do.</p>
            {/if}

            {#if selectedMode === 'Cloud' || selectedMode === 'Hybrid'}
              {#if setupPath === 'one_key'}
                {#if onboardingCatalogLoading && modelCatalog.length === 0}
                  <p class="hint">Loading providers…</p>
                {:else}
                  {@const groq = catalogProviderById('groq')}
                  {#if groq}
                    <div class="onboarding-catalog-grid onboarding-catalog-grid--single">
                      <div class="onboarding-provider-card">
                        <div class="onboarding-provider-card-head">
                          <Icon icon={groq.icon} class="onboarding-provider-icon" />
                          <div>
                            <strong>{groq.name}</strong>
                            <div class="provider-cap-badges">
                              {#each groq.capabilities as cap}
                                <span class="cap-badge">{cap}</span>
                              {/each}
                            </div>
                          </div>
                        </div>
                        <p class="provider-explainer">One Groq key covers cloud STT and Groq LLM when you select Groq in Settings.</p>
                        <div class="api-key-row">
                          <input
                            type="password"
                            autocomplete="off"
                            spellcheck="false"
                            placeholder="Groq API key"
                            bind:value={apiKey}
                            on:paste={onApiKeyPaste}
                            on:input={() => (apiKeyValid = null)}
                            aria-label="Groq API key"
                          />
                          <button
                            type="button"
                            class="btn-validate"
                            disabled={!apiKey.trim() || validating}
                            on:click={() => {
                              selectedProvider = 'groq'
                              void checkApiKey()
                            }}
                          >
                            {validating ? 'Checking...' : 'Validate'}
                          </button>
                        </div>
                        <div class="api-key-meta">
                          {#if apiKeyValid !== null}
                            <span class="validation-badge" class:valid={apiKeyValid}>
                              {apiKeyValid ? '✓ Valid' : '✗ Invalid'}
                            </span>
                          {/if}
                          {#if groq.get_api_key_url}
                            <p class="api-key-hint">
                              <a href={groq.get_api_key_url} target="_blank" rel="noopener noreferrer">Get API key ↗</a>
                            </p>
                          {/if}
                        </div>
                      </div>
                    </div>
                  {/if}
                {/if}
              {:else if setupPath === 'best_quality'}
                {#if onboardingCatalogLoading && modelCatalog.length === 0}
                  <p class="hint">Loading providers…</p>
                {:else}
                  {@const groq = catalogProviderById('groq')}
                  {@const llmP = qualityLlmChoice === 'anthropic' ? catalogProviderById('anthropic') : catalogProviderById('openai')}
                  <div class="onboarding-catalog-grid">
                    {#if groq}
                      <div class="onboarding-provider-card">
                        <div class="onboarding-provider-card-head">
                          <Icon icon={groq.icon} class="onboarding-provider-icon" />
                          <div>
                            <strong>{groq.name}</strong>
                            <span class="onboarding-card-sub">Transcription</span>
                            <div class="provider-cap-badges">
                              {#each groq.capabilities as cap}
                                <span class="cap-badge">{cap}</span>
                              {/each}
                            </div>
                          </div>
                        </div>
                        <div class="api-key-row">
                          <input
                            type="password"
                            autocomplete="off"
                            spellcheck="false"
                            placeholder="Groq API key"
                            bind:value={apiKey}
                            on:paste={onApiKeyPaste}
                            on:input={() => (apiKeyValid = null)}
                            aria-label="Groq API key for transcription"
                          />
                          <button
                            type="button"
                            class="btn-validate"
                            disabled={!apiKey.trim() || validating}
                            on:click={() => {
                              selectedProvider = 'groq'
                              void checkApiKey()
                            }}
                          >
                            {validating ? 'Checking...' : 'Validate'}
                          </button>
                        </div>
                        <div class="api-key-meta">
                          {#if apiKeyValid !== null}
                            <span class="validation-badge" class:valid={apiKeyValid}>
                              {apiKeyValid ? '✓ Valid' : '✗ Invalid'}
                            </span>
                          {/if}
                          {#if groq.get_api_key_url}
                            <p class="api-key-hint">
                              <a href={groq.get_api_key_url} target="_blank" rel="noopener noreferrer">Get API key ↗</a>
                            </p>
                          {/if}
                        </div>
                      </div>
                    {/if}
                    {#if llmP}
                      <div class="onboarding-provider-card">
                        <div class="onboarding-provider-card-head">
                          <Icon icon={llmP.icon} class="onboarding-provider-icon" />
                          <div>
                            <strong>{llmP.name}</strong>
                            <span class="onboarding-card-sub">Command &amp; AI</span>
                            <div class="provider-cap-badges">
                              {#each llmP.capabilities as cap}
                                <span class="cap-badge">{cap}</span>
                              {/each}
                            </div>
                          </div>
                        </div>
                        <div class="api-key-row">
                          {#if qualityLlmChoice === 'anthropic'}
                            <input
                              type="password"
                              autocomplete="off"
                              spellcheck="false"
                              placeholder="Anthropic API key"
                              bind:value={llmApiKey}
                              on:input={() => (onboardingKeyValid = { ...onboardingKeyValid, anthropic: null })}
                              aria-label="Anthropic API key"
                            />
                            <button
                              type="button"
                              class="btn-validate"
                              disabled={!llmApiKey.trim() || onboardingKeyValidating['anthropic']}
                              on:click={() => validateOnboardingCatalogKey('anthropic', llmApiKey)}
                            >
                              {onboardingKeyValidating['anthropic'] ? 'Checking...' : 'Validate'}
                            </button>
                          {:else}
                            <input
                              type="password"
                              autocomplete="off"
                              spellcheck="false"
                              placeholder="OpenAI API key"
                              bind:value={openaiLlmKey}
                              on:input={() => (onboardingKeyValid = { ...onboardingKeyValid, openai: null })}
                              aria-label="OpenAI API key for LLM"
                            />
                            <button
                              type="button"
                              class="btn-validate"
                              disabled={!openaiLlmKey.trim() || onboardingKeyValidating['openai']}
                              on:click={() => validateOnboardingCatalogKey('openai', openaiLlmKey)}
                            >
                              {onboardingKeyValidating['openai'] ? 'Checking...' : 'Validate'}
                            </button>
                          {/if}
                        </div>
                        <div class="api-key-meta">
                          {#if qualityLlmChoice === 'anthropic'}
                            {#if onboardingKeyValid['anthropic'] != null}
                              <span class="validation-badge" class:valid={!!onboardingKeyValid['anthropic']}>
                                {onboardingKeyValid['anthropic'] ? '✓ Valid' : '✗ Invalid'}
                              </span>
                            {/if}
                          {:else if onboardingKeyValid['openai'] != null}
                            <span class="validation-badge" class:valid={!!onboardingKeyValid['openai']}>
                              {onboardingKeyValid['openai'] ? '✓ Valid' : '✗ Invalid'}
                            </span>
                          {/if}
                          {#if llmP.get_api_key_url}
                            <p class="api-key-hint">
                              <a href={llmP.get_api_key_url} target="_blank" rel="noopener noreferrer">Get API key ↗</a>
                            </p>
                          {/if}
                        </div>
                      </div>
                    {/if}
                  </div>
                {/if}
              {:else if setupPath === 'manual'}
                <p class="provider-explainer">
                  Choose your default cloud STT engine, then paste keys for any providers you use (same curated list as Settings).
                </p>
                <div class="onboarding-stt-default-row">
                  <label class="onboarding-stt-label" for="onb-stt-default">Default cloud transcription</label>
                  <select id="onb-stt-default" class="onboarding-stt-select" bind:value={selectedProvider}>
                    <option value="groq">Groq</option>
                    <option value="openai">OpenAI</option>
                  </select>
                </div>
                {#if onboardingCatalogLoading && modelCatalog.length === 0}
                  <p class="hint">Loading providers…</p>
                {:else}
                  <div class="onboarding-catalog-grid">
                    {#each modelCatalog.filter((p) => p.id !== 'local') as prov (prov.id)}
                      <div class="onboarding-provider-card">
                        <div class="onboarding-provider-card-head">
                          <Icon icon={prov.icon} class="onboarding-provider-icon" />
                          <div>
                            <strong>{prov.name}</strong>
                            <div class="provider-cap-badges">
                              {#each prov.capabilities as cap}
                                <span class="cap-badge">{cap}</span>
                              {/each}
                            </div>
                          </div>
                        </div>
                        <div class="api-key-row">
                          <input
                            type="password"
                            autocomplete="off"
                            spellcheck="false"
                            placeholder={`${prov.name} API key`}
                            value={onboardingManualKeys[prov.id] ?? ''}
                            on:input={(e) => {
                              const v = eventInputValue(e)
                              onboardingManualKeys = { ...onboardingManualKeys, [prov.id]: v }
                              onboardingKeyValid = { ...onboardingKeyValid, [prov.id]: null }
                            }}
                            aria-label={`API key for ${prov.name}`}
                          />
                          <button
                            type="button"
                            class="btn-validate"
                            disabled={!(onboardingManualKeys[prov.id] ?? '').trim() || onboardingKeyValidating[prov.id]}
                            on:click={() => validateOnboardingCatalogKey(prov.id, onboardingManualKeys[prov.id] ?? '')}
                          >
                            {onboardingKeyValidating[prov.id] ? 'Checking...' : 'Validate'}
                          </button>
                        </div>
                        <div class="api-key-meta">
                          {#if onboardingKeyValid[prov.id] != null}
                            <span class="validation-badge" class:valid={!!onboardingKeyValid[prov.id]}>
                              {onboardingKeyValid[prov.id] ? '✓ Valid' : '✗ Invalid'}
                            </span>
                          {/if}
                          {#if prov.get_api_key_url}
                            <p class="api-key-hint">
                              <a href={prov.get_api_key_url} target="_blank" rel="noopener noreferrer">Get API key ↗</a>
                            </p>
                          {/if}
                        </div>
                      </div>
                    {/each}
                  </div>
                {/if}
              {:else}
                <div class="provider-section">
                  <div class="provider-toggle">
                    <span class="provider-label">Cloud provider</span>
                    <div class="provider-pills">
                      <button
                        type="button"
                        class="prov-pill"
                        class:active={selectedProvider === 'groq'}
                        on:click={() => (selectedProvider = 'groq')}
                      >Groq</button>
                      <button
                        type="button"
                        class="prov-pill"
                        class:active={selectedProvider === 'openai'}
                        on:click={() => (selectedProvider = 'openai')}
                      >OpenAI</button>
                    </div>
                  </div>
                  <p class="provider-explainer">Paste your key now or add it later in Settings.</p>
                  <div class="api-key-row">
                    <input
                      type="password"
                      autocomplete="off"
                      spellcheck="false"
                      placeholder={selectedProvider === 'openai' ? 'OpenAI API key' : 'Groq API key'}
                      bind:value={apiKey}
                      on:paste={onApiKeyPaste}
                      on:input={() => (apiKeyValid = null)}
                    />
                    <button
                      type="button"
                      class="btn-validate"
                      disabled={!apiKey.trim() || validating}
                      on:click={checkApiKey}
                    >
                      {validating ? 'Checking...' : 'Validate'}
                    </button>
                  </div>
                  <div class="api-key-meta">
                    {#if apiKeyValid !== null}
                      <span class="validation-badge" class:valid={apiKeyValid}>
                        {apiKeyValid ? '✓ Valid' : '✗ Invalid'}
                      </span>
                    {/if}
                    <p class="api-key-hint">
                      {#if selectedProvider === 'openai'}
                        <a href="https://platform.openai.com/api-keys" target="_blank" rel="noopener noreferrer">Get your API key from OpenAI →</a>
                      {:else}
                        <a href="https://console.groq.com" target="_blank" rel="noopener noreferrer">Get your API key from Groq →</a>
                      {/if}
                    </p>
                  </div>
                </div>
              {/if}
            {/if}
          </div>
        </div>

      {:else if step === 6}
        <div class="step step-controls">
          <p class="step-eyebrow" aria-hidden="true">Step {step} of {totalSteps}</p>
          <h1>Shortcuts &amp; languages</h1>
          <p class="subtitle">Set how you start dictation, then tell Kalam which languages you speak. Everything here can be edited in Settings.</p>

          <div class="controls-grid">
            <section class="ctrl-section">
              <h3>Dictation Hotkeys</h3>
              <p class="ctrl-section-lead">Click a field, press the keys you want, then release to save. At least one shortcut should feel natural—you’ll use it constantly.</p>
              <!-- `page-content` ancestor enables App.svelte global styles for `.hotkey-capture-area` (same as Settings). -->
              <div class="page-content onboarding-hotkeys-block">
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Hold to Dictate</span>
                    <span class="setting-desc">Press and hold to start dictating</span>
                  </div>
                  <div class="setting-control">
                    <HotkeyCapture
                      value={hotkey}
                      platform={platform}
                      onChange={(h) => (hotkey = h)}
                    />
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-label">
                    <span class="setting-name">Toggle Dictation</span>
                    <span class="setting-desc">Press to start/stop dictating</span>
                  </div>
                  <div class="setting-control">
                    <HotkeyCapture
                      value={toggleHotkey}
                      platform={platform}
                      onChange={(h) => (toggleHotkey = h)}
                    />
                  </div>
                </div>
              </div>
            </section>

            <section class="ctrl-section">
              <h3>Languages you dictate in</h3>
              <p class="ctrl-section-lead">The first language is the default. Add more if you dictate in several languages—order matters for how Kalam prioritizes recognition.</p>
              <div class="lang-tags">
                {#each languages as code, i}
                  <span class="lang-tag">
                    {#if i === 0}<span class="tag-default">Default</span>{/if}
                    {languageLabel(code)}
                    {#if languages.length > 1}
                      <button type="button" class="tag-remove" aria-label="Remove {languageLabel(code)}" on:click={() => removeLanguage(i)}>×</button>
                    {/if}
                  </span>
                {/each}
              </div>
              <label class="visually-hidden" for="onboarding-lang-add">Add a language</label>
              <select
                id="onboarding-lang-add"
                class="lang-add"
                bind:value={addLanguageCode}
                on:change={() => addLanguage(addLanguageCode)}
              >
                <option value="">Add another language…</option>
                {#each LANGUAGE_OPTIONS as opt}
                  <option value={opt.code} disabled={languages.includes(opt.code)}>{opt.label}</option>
                {/each}
              </select>
            </section>
          </div>
        </div>

      {:else if step === 7}
        <div class="step step-ready">
          <p class="step-eyebrow" aria-hidden="true">Step {step} of {totalSteps}</p>
          <div class="ready-visual">
            <div class="ready-ring r1"></div>
            <div class="ready-ring r2"></div>
            <div class="ready-ring r3"></div>
            <div class="ready-check" aria-hidden="true"><Icon icon="ph:check-bold" /></div>
          </div>
          <h1>Try dictation once</h1>
          <p class="subtitle">
            Active mode: <strong>{activeModeDisplayName}</strong>. Sanity-check your shortcut and engine: focus the box, dictate a short phrase, and confirm text appears. Refine modes on the <strong>Dictation</strong> page.
          </p>

          <label class="polish-demo-toggle">
            <input type="checkbox" bind:checked={polishDemoEnabled} />
            <span><strong>Polish</strong> — use AI to clean up grammar and formatting after STT (uses your LLM when the active mode needs it).</span>
          </label>

          <ol class="ready-steps">
            <li>Click inside the text area (or use the focus button under it).</li>
            <li>
              {#if hotkey && toggleHotkey}
                Press <kbd class="ready-kbd-inline">{hotkey}</kbd><span class="ready-or"> or </span><kbd class="ready-kbd-inline">{toggleHotkey}</kbd>—whichever you set up.
              {:else}
                Press <kbd class="ready-kbd-inline">{primaryHotkeyDemo}</kbd>.
              {/if}
            </li>
            <li>Speak a few words, then release the keys or toggle off. Transcription should show up here when the backend responds.</li>
          </ol>

          <div class="demo-box" class:unfocused={!demoFocused}>
            <div class="demo-prompt">
              <kbd>{primaryHotkeyDemo}</kbd>
              <span>{hotkeyDemoCaption}</span>
            </div>
            <!-- bind:value so Svelte keeps the field in sync when dictation events update demoTranscription (value= alone does not update after mount in Svelte 4). -->
            <textarea
              bind:this={demoTextarea}
              bind:value={demoTranscription}
              placeholder="Transcription appears here after you dictate…"
              readonly
              on:focus={() => (demoFocused = true)}
              on:blur={() => (demoFocused = false)}
            ></textarea>
            {#if !demoFocused}
              <button type="button" class="refocus-cue" on:click={() => demoTextarea?.focus()}>
                {#if hotkey && toggleHotkey}
                  Click to focus, then <kbd>{hotkey}</kbd> or <kbd>{toggleHotkey}</kbd>
                {:else}
                  Click to focus, then <kbd>{primaryHotkeyDemo}</kbd>
                {/if}
              </button>
            {/if}
          </div>
          <p class="step-footnote step-footnote-center">When you’re happy, tap <strong>Enter Kalam</strong> to open the app. You can revisit any of this in Settings.</p>
        </div>
      {/if}
      </div>
      {/key}

      <div class="actions">
        {#if step > 1}
          <button class="btn-back" on:click={prevStep}>Back</button>
        {/if}
        <div class="actions-spacer"></div>
        {#if step < totalSteps}
          <button
            class="btn-next"
            disabled={(step === 2 && (!termsAgreed || !isEmailValid(userEmail))) ||
              (step === 3 && !step3ContinueEnabled) ||
              (step === 4 && setupPath == null)}
            on:click={nextStep}
          >Continue</button>
        {:else}
          <button class="btn-next btn-finish" on:click={finish}>Enter Kalam</button>
        {/if}
      </div>

      {#if step >= 3}
        <div class="skip-mobile">
          <button
            type="button"
            class="btn-skip"
            disabled={skipInProgress}
            on:click={handleSkipClick}
          >
            {skipInProgress ? 'Skipping…' : 'Use defaults & skip'}
          </button>
          {#if skipError}
            <p class="skip-error" role="alert">{skipError}</p>
          {/if}
          <p class="skip-consequence">
            Skipping applies defaults and may leave permissions unset—you can fix microphone and accessibility in Settings → Audio and AI providers.
          </p>
        </div>
      {/if}

    </div>
  </div>
</div>

<style>
  /* Inherits .kalam-sleek from App.svelte wrapper — use same tokens as main shell (Inter, --accent, surfaces). */
  .onboarding {
    position: fixed;
    inset: 0;
    font-family: var(--font-sleek, 'Inter', system-ui, sans-serif);
    background: var(--bg);
    color: var(--text);
    display: flex;
  }

  /* ── Left stepper (match main sidebar: 240px elevated strip + nav-like steps) ── */
  .stepper {
    width: 240px;
    flex-shrink: 0;
    background: var(--bg-elevated);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 32px 20px;
  }

  .stepper-brand {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 40px;
    padding: 0 4px;
  }

  /* Match App.svelte `.logo-img` (28 × 1.2 × 1.15) */
  .stepper-logo {
    width: calc(28px * 1.2 * 1.15);
    height: calc(28px * 1.2 * 1.15);
    flex-shrink: 0;
    object-fit: contain;
    display: block;
  }

  .stepper-title {
    font-family: var(--font-sleek, 'Inter', system-ui, sans-serif);
    font-size: 18px;
    font-weight: 600;
    color: var(--text);
    letter-spacing: -0.03em;
    margin: 0;
  }

  .stepper-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    flex: 1;
  }

  .stepper-item {
    display: flex;
    flex-direction: column;
  }

  .stepper-btn {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 10px 12px;
    background: none;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: var(--transition-sleek, 200ms ease);
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    text-align: left;
  }

  .stepper-btn:disabled {
    cursor: default;
    opacity: 0.5;
  }

  .stepper-btn:not(:disabled):hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .stepper-item.active .stepper-btn {
    color: var(--text);
    font-weight: 600;
    background: var(--bg-card);
    box-shadow: var(--shadow);
  }

  .stepper-item.complete .stepper-btn {
    color: var(--text-secondary);
  }

  .stepper-indicator {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 700;
    flex-shrink: 0;
    transition: var(--transition-sleek, 200ms ease);
    background: var(--bg-input);
    color: var(--text-muted);
    border: 1px solid var(--border);
  }

  .stepper-item.active .stepper-indicator {
    background: var(--accent);
    color: var(--accent-fg);
    border-color: var(--accent);
    box-shadow: var(--shadow);
  }

  .stepper-item.complete .stepper-indicator {
    background: var(--success);
    color: #fff;
    border-color: var(--success);
  }

  .stepper-label {
    white-space: nowrap;
  }

  .stepper-line {
    width: 2px;
    height: 16px;
    background: var(--border);
    margin: 2px 0 2px 25px;
    border-radius: 1px;
    transition: background 0.3s ease;
  }

  .stepper-line.filled {
    background: var(--success);
  }

  .stepper-skip {
    margin-top: auto;
    padding-top: 20px;
    border-top: 1px solid var(--border);
    text-align: center;
  }

  /* ── Mobile stepper (hidden on desktop) ── */
  .stepper-mobile {
    display: none;
  }

  /* ── Right content ── */
  .content-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* Match App.svelte `.page-content`: same padding and centered column width. */
  .content-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-3xl, 64px) var(--space-2xl, 48px);
    display: flex;
    flex-direction: column;
    background: var(--bg);
    max-width: 900px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
  }

  .step {
    max-width: 600px;
    width: 100%;
    margin: 0 auto;
  }

  /* Subtle step transition (respects reduced motion). */
  .step-shell {
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    animation: step-enter 320ms cubic-bezier(0.4, 0, 0.2, 1) both;
  }

  @keyframes step-enter {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .step-shell {
      animation: none;
    }

    .mic-refresh-btn--busy :global(svg),
    .mic-action-btn--starting .mic-action-icon :global(svg) {
      animation: none;
    }
  }

  .step-eyebrow {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
    margin: 0 0 10px;
  }

  .onboarding h1 {
    font-family: var(--font-sleek, 'Inter', system-ui, sans-serif);
    font-size: 28px;
    margin-bottom: 8px;
    color: var(--text);
    letter-spacing: -0.02em;
  }

  .subtitle {
    font-size: 15px;
    color: var(--text-secondary);
    margin-bottom: 20px;
    line-height: 1.55;
  }

  .step-meta {
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.55;
    margin: -4px 0 28px;
  }

  .step-footnote {
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.45;
    margin-top: 16px;
  }

  .step-footnote-center {
    text-align: center;
    max-width: 440px;
    margin-left: auto;
    margin-right: auto;
  }

  .perm-status-strip {
    display: flex;
    flex-direction: column;
    gap: 14px;
    margin-bottom: 20px;
  }

  .perm-strip-loading {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0;
  }

  .perm-status-row {
    padding: 14px 16px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .perm-status-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    margin-bottom: 8px;
  }

  .perm-status-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }

  .perm-badge {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 4px 10px;
    border-radius: var(--radius-full);
    background: var(--bg-input);
    color: var(--text-muted);
    border: 1px solid var(--border);
  }

  .perm-badge.perm-badge--granted {
    background: color-mix(in srgb, var(--success) 14%, var(--bg-elevated));
    color: var(--success);
    border-color: color-mix(in srgb, var(--success) 35%, var(--border));
  }

  .perm-badge.perm-badge--needs {
    background: color-mix(in srgb, var(--warning) 14%, var(--bg-elevated));
    color: var(--warning);
    border-color: color-mix(in srgb, var(--warning) 35%, var(--border));
  }

  .perm-badge.perm-badge--unknown {
    background: var(--bg-input);
    color: var(--text-secondary);
  }

  .perm-status-msg {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin: 0 0 10px;
  }

  .perm-status-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  .perm-status-hint-inline {
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.45;
    flex: 1 1 180px;
  }

  .perm-linux-msg {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.45;
    margin: 0;
  }

  .perm-defer-check {
    margin-top: 4px;
    padding: 12px 14px;
    background: var(--bg-input);
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
  }

  .perm-defer-consequence {
    margin: 8px 0 0;
    font-size: 13px;
    line-height: 1.45;
    color: var(--text-secondary);
  }

  .perm-privacy-footnote {
    margin: -8px 0 20px;
  }

  .mic-test-start-error {
    font-size: 13px;
    color: var(--error);
    margin: 0 0 10px;
    line-height: 1.45;
  }

  .skip-consequence {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.4;
    margin: 10px 0 0;
    max-width: 220px;
    margin-left: auto;
    margin-right: auto;
  }

  .visually-hidden {
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

  .mic-device-row {
    margin-bottom: 16px;
    padding: 14px 16px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .mic-device-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-width: 0;
  }

  .mic-refresh-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    cursor: pointer;
    transition: var(--transition-sleek, 200ms ease);
  }

  .mic-refresh-btn:hover {
    background: var(--bg-hover);
    color: var(--text);
    border-color: var(--border-light);
  }

  .mic-refresh-btn :global(svg) {
    width: 18px;
    height: 18px;
  }

  @keyframes mic-icon-spin {
    to {
      transform: rotate(360deg);
    }
  }

  .mic-refresh-btn--busy {
    cursor: wait;
    opacity: 0.92;
  }

  .mic-refresh-btn--busy :global(svg) {
    animation: mic-icon-spin 0.55s linear infinite;
  }

  .mic-action-btn--starting {
    cursor: wait;
    opacity: 0.95;
  }

  .mic-action-btn--starting .mic-action-icon :global(svg) {
    animation: mic-icon-spin 0.7s linear infinite;
  }

  .mic-device-select {
    flex: 1;
    min-width: 0;
    width: 100%;
    max-width: none;
    padding: 10px 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 14px;
    font-weight: 500;
    font-family: var(--font-sleek, inherit);
    cursor: pointer;
    transition: var(--transition-sleek, 200ms ease);
  }

  .mic-device-select:focus {
    outline: none;
    border-color: var(--text-muted);
  }

  .mic-device-warning {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.45;
    margin: 0 0 14px;
  }


  .provider-explainer {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.45;
    margin: 0 0 12px;
  }

  /* Curated provider cards on the Engine step (mirrors Settings provider library styling). */
  .provider-cap-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 6px;
  }

  .cap-badge {
    font-size: 11px;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    background: var(--bg-input);
    color: var(--text-muted);
    letter-spacing: 0.02em;
  }

  .onboarding-catalog-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
    margin-top: 16px;
  }

  .onboarding-catalog-grid--single {
    max-width: 440px;
  }

  .onboarding-provider-card {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 16px;
  }

  .onboarding-provider-card-head {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    margin-bottom: 12px;
  }

  .onboarding-provider-icon :global(svg) {
    width: 28px;
    height: 28px;
  }

  .onboarding-card-sub {
    display: block;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    margin-top: 2px;
  }

  .onboarding-stt-default-row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }

  .onboarding-stt-label {
    font-size: 13px;
    font-weight: 600;
  }

  .onboarding-stt-select {
    padding: 8px 12px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--bg-input);
    color: var(--text);
    font-family: inherit;
  }

  .ctrl-section-lead {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.45;
    margin: -6px 0 16px;
  }

  .ready-steps {
    margin: 0 0 22px;
    padding-left: 1.35rem;
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.55;
    max-width: 520px;
  }

  .ready-steps li {
    margin-bottom: 10px;
  }

  .ready-steps kbd,
  .ready-kbd-inline {
    padding: 2px 8px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-family: var(--font-sleek, 'Inter', ui-monospace, monospace);
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
  }

  .ready-or {
    font-weight: 500;
    color: var(--text-muted);
  }

  .step-ready .step-eyebrow {
    margin-bottom: 16px;
  }

  /* ── Step 1: Welcome ── */
  .features-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .feat {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 18px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    transition: var(--transition-sleek, 200ms ease);
  }

  .feat:hover {
    border-color: var(--border-light);
    background: var(--bg-hover);
  }

  .feat-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    margin-top: 1px;
    color: var(--text);
  }

  .feat-icon :global(svg) {
    width: 22px;
    height: 22px;
  }

  .feat strong {
    display: block;
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
    margin-bottom: 2px;
  }

  .feat span {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  /* ── Step 2: Account ── */
  .form-card {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 28px;
  }

  .form-card .step-footnote {
    margin-top: 20px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }

  .field {
    margin-bottom: 20px;
  }

  .field label {
    display: block;
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 6px;
    color: var(--text);
  }

  .req {
    color: var(--error);
  }

  .field input[type="email"] {
    width: 100%;
    padding: 11px 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 14px;
    font-family: var(--font-sleek, inherit);
    transition: var(--transition-sleek, 200ms ease);
  }

  .field input[type="email"]:focus {
    outline: none;
    border-color: var(--text-muted);
  }

  .field-error {
    font-size: 12px;
    color: var(--error);
    margin: 4px 0 0;
  }

  .checkboxes {
    padding-top: 16px;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .check-row {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    cursor: pointer;
    font-size: 14px;
    color: var(--text);
    user-select: none;
  }

  .check-row input[type="checkbox"] {
    width: 18px;
    height: 18px;
    margin: 0;
    flex-shrink: 0;
    margin-top: 1px;
  }

  .check-row a {
    color: var(--text);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .privacy-note {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 8px 0 0;
    font-weight: 500;
  }

  .perm-auto {
    font-size: 12px;
    color: var(--success);
    font-weight: 600;
    white-space: nowrap;
  }

  .btn-outline-sm {
    padding: 7px 14px;
    font-size: 13px;
    font-weight: 600;
    font-family: var(--font-sleek, inherit);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    color: var(--text);
    cursor: pointer;
    white-space: nowrap;
    transition: var(--transition-sleek, 200ms ease);
  }

  .btn-outline-sm:hover {
    background: var(--bg-hover);
    border-color: var(--border-light);
  }

  .mic-test {
    padding: 20px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .mic-test-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 14px;
  }

  .mic-test-title {
    font-size: 14px;
    font-weight: 700;
    color: var(--text);
    margin: 0;
    letter-spacing: -0.02em;
  }

  /* Details live in title + aria-label; keeps the block visually quiet. */
  .mic-info-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    padding: 0;
    margin: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: var(--transition-sleek, 200ms ease);
  }

  .mic-info-btn:hover {
    background: var(--bg-hover);
    color: var(--text-secondary);
  }

  .mic-info-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .mic-info-btn :global(svg) {
    width: 18px;
    height: 18px;
  }

  .mic-info-btn--error-context:hover {
    color: var(--error);
  }

  .mic-test-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .mic-controls {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .mic-action-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    background: var(--bg);
    color: var(--text);
    font-size: 14px;
    font-weight: 600;
    font-family: var(--font-sleek, inherit);
    cursor: pointer;
    transition: var(--transition-sleek, 200ms ease);
  }

  .mic-action-btn:hover {
    background: var(--bg-hover);
    border-color: var(--border-light);
  }

  .mic-action-btn:disabled {
    cursor: wait;
  }

  .mic-action-btn:disabled:hover {
    background: var(--bg);
    border-color: var(--border);
  }

  .mic-action-btn.recording {
    border-color: var(--error);
    background: color-mix(in srgb, var(--error) 12%, var(--bg-elevated));
    color: var(--error);
    animation: pulse-rec 1.5s ease-in-out infinite;
  }

  .mic-action-btn.recording:hover {
    background: var(--error);
    color: #fff;
  }

  @keyframes pulse-rec {
    0%, 100% { box-shadow: 0 0 0 0 rgba(239, 68, 68, 0.2); }
    50% { box-shadow: 0 0 0 8px rgba(239, 68, 68, 0); }
  }

  .mic-action-btn.play {
    border-color: var(--success);
    background: color-mix(in srgb, var(--success) 12%, var(--bg-elevated));
    color: var(--success);
  }

  .mic-action-btn.play:hover {
    background: var(--success);
    color: #fff;
  }

  .mic-action-btn.playing {
    opacity: 0.7;
    cursor: default;
  }

  .mic-action-icon {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .mic-action-icon :global(svg) {
    width: 18px;
    height: 18px;
  }

  .mic-level-bar {
    flex: 1;
    height: 8px;
    background: var(--bg-input);
    border-radius: 4px;
    overflow: hidden;
  }

  .mic-level-fill {
    height: 100%;
    background: var(--error);
    border-radius: 4px;
    transition: width 0.1s ease-out;
  }

  .mic-status {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin: 0;
  }

  .mic-status strong {
    font-weight: 600;
    color: var(--text);
  }

  .error-text {
    color: var(--error) !important;
  }

  /* ── Setup path (step 4) ── */
  .setup-path-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
    margin-top: 8px;
  }

  @media (max-width: 640px) {
    .setup-path-grid {
      grid-template-columns: 1fr;
    }
  }

  .setup-path-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    padding: 16px;
    text-align: left;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    color: var(--text);
    font-family: var(--font-sleek, inherit);
    cursor: pointer;
    transition: var(--transition-sleek, 200ms ease);
  }

  .setup-path-card:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
  }

  .setup-path-card.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }

  .setup-path-icon :global(svg) {
    width: 22px;
    height: 22px;
    color: var(--accent);
  }

  .setup-path-title {
    font-size: 15px;
    font-weight: 700;
  }

  .setup-path-desc {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.45;
  }

  .setup-path-banner {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 12px 14px;
    margin-bottom: 16px;
    border-radius: var(--radius-md);
    background: var(--primary-alpha);
    color: var(--text);
    font-size: 13px;
    line-height: 1.5;
  }

  .setup-path-banner :global(svg) {
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    margin-top: 2px;
    color: var(--accent);
  }

  .quality-llm-row {
    margin-bottom: 12px;
  }

  .quality-llm-label {
    display: block;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .quality-llm-pills {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .polish-demo-toggle {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    margin: 16px 0 20px;
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    font-size: 13px;
    line-height: 1.45;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .polish-demo-toggle input {
    margin-top: 3px;
    flex-shrink: 0;
  }

  /* ── Engine step (step 5): Mode ── */
  .stt-tabs {
    display: flex;
    min-height: 48px;
    background: var(--bg-elevated);
    border-radius: var(--radius-md);
    padding: 3px;
    gap: 2px;
    margin-bottom: 20px;
  }

  .stt-tab {
    flex: 1 1 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-width: 0;
    padding: 9px 10px;
    border: none;
    border-radius: calc(var(--radius-md) - 2px);
    background: transparent;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    font-family: var(--font-sleek, inherit);
    cursor: pointer;
    transition: var(--transition-sleek, 200ms ease);
    white-space: nowrap;
    position: relative;
  }

  .stt-tab:hover {
    color: var(--text);
    background: var(--bg-hover);
  }

  .stt-tab.active {
    background: var(--bg-hover);
    color: var(--text);
    font-weight: 600;
  }

  .stt-tab:disabled {
    opacity: 0.45;
    cursor: not-allowed;
    pointer-events: none;
  }

  .stt-tab-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .stt-tab-icon :global(svg) {
    width: 18px;
    height: 18px;
  }

  .stt-rec-badge {
    position: absolute;
    top: -6px;
    right: 4px;
    padding: 2px 6px;
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    border-radius: var(--radius-pill);
    white-space: nowrap;
  }

  .mode-detail {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 24px;
  }

  .mode-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 14px;
  }

  .mode-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    background: var(--bg-input);
    border-radius: var(--radius-pill);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .mode-chip.rec {
    background: var(--primary-alpha);
    color: var(--text);
  }

  .mode-chip.fast::before {
    content: '';
    display: inline-block;
    width: 6px;
    height: 6px;
    background: var(--success);
    border-radius: 50%;
  }

  .mode-chip.offline::before {
    content: '';
    display: inline-block;
    width: 6px;
    height: 6px;
    background: var(--text-muted);
    border-radius: 50%;
  }

  .mode-chip.privacy::before {
    content: '';
    display: inline-block;
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
  }

  .mode-chip.disk::before {
    content: '';
    display: inline-block;
    width: 6px;
    height: 6px;
    background: var(--warning);
    border-radius: 50%;
  }

  .mode-chip.needs-key::before {
    content: '';
    display: inline-block;
    width: 6px;
    height: 6px;
    background: var(--info);
    border-radius: 50%;
  }

  .mode-blurb {
    font-size: 14px;
    color: var(--text-secondary);
    margin: 0 0 14px;
    line-height: 1.5;
  }

  .mode-info-box {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 14px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    margin-top: 4px;
  }

  .mode-info-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 20px;
    height: 20px;
    color: var(--accent);
  }

  .mode-info-icon :global(svg) {
    width: 18px;
    height: 18px;
  }

  .mode-info-text {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .mode-info-text strong {
    color: var(--text);
    font-weight: 600;
  }

  .api-key-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }

  .api-key-row input {
    flex: 1;
    padding: 10px 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 14px;
    font-family: var(--font-sleek, inherit);
    transition: var(--transition-sleek, 200ms ease);
  }

  .api-key-row input:focus {
    outline: none;
    border-color: var(--text-muted);
  }

  .api-key-row a {
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
    text-decoration: none;
    white-space: nowrap;
  }

  .btn-validate {
    padding: 10px 18px;
    background: var(--accent);
    border: none;
    border-radius: var(--radius-md);
    color: var(--accent-fg);
    font-size: 13px;
    font-weight: 600;
    font-family: var(--font-sleek, inherit);
    cursor: pointer;
    transition: var(--transition-sleek, 200ms ease);
    white-space: nowrap;
  }

  .btn-validate:hover:not(:disabled) {
    opacity: 0.92;
  }

  .btn-validate:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .api-key-meta {
    margin-top: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .validation-badge {
    font-size: 13px;
    font-weight: 600;
    color: var(--error);
  }

  .validation-badge.valid {
    color: var(--success);
  }

  .api-key-hint {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0;
  }

  .api-key-hint a {
    color: var(--text-secondary);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .api-key-hint a:hover {
    color: var(--text);
  }

  .provider-section {
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .provider-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .provider-label {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .provider-pills {
    display: flex;
    background: var(--bg-input);
    border-radius: var(--radius-sm);
    padding: 3px;
    gap: 2px;
  }

  .prov-pill {
    padding: 6px 16px;
    border: none;
    background: transparent;
    border-radius: calc(var(--radius-sm) - 2px);
    font-size: 13px;
    font-weight: 600;
    font-family: var(--font-sleek, inherit);
    color: var(--text-muted);
    cursor: pointer;
    transition: var(--transition-sleek, 200ms ease);
  }

  .prov-pill:hover:not(.active) {
    color: var(--text);
  }

  .prov-pill.active {
    background: var(--bg-card);
    color: var(--text);
    box-shadow: var(--shadow);
  }

  .prov-pill:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .local-hint {
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .local-hint strong {
    color: var(--text);
    font-weight: 600;
  }

  /* ── Step 6: Controls ── */
  .controls-grid {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .ctrl-section {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 24px;
  }

  .ctrl-section h3 {
    font-size: 14px;
    font-weight: 700;
    color: var(--text);
    margin-bottom: 14px;
    letter-spacing: 0;
  }

  /* Match Settings → General → Dictation Hotkeys row layout (Settings scopes these classes to its page only). */
  .onboarding-hotkeys-block {
    padding: 0;
    margin: 0;
    max-width: none;
    width: 100%;
    background: transparent;
    flex: unset;
    overflow: visible;
  }

  .onboarding-hotkeys-block .setting-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-md) 0;
    gap: var(--space-lg);
    border-bottom: 1px solid var(--border-light);
  }

  .onboarding-hotkeys-block .setting-row:last-child {
    border-bottom: none;
  }

  .onboarding-hotkeys-block .setting-label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-width: 0;
  }

  .onboarding-hotkeys-block .setting-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text);
  }

  .onboarding-hotkeys-block .setting-desc {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .onboarding-hotkeys-block .setting-control {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-shrink: 0;
  }

  @media (max-width: 768px) {
    .onboarding-hotkeys-block .setting-row {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--space-sm);
    }

    .onboarding-hotkeys-block .setting-control {
      width: 100%;
    }
  }

  .lang-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 12px;
  }

  .lang-tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: var(--bg-input);
    border-radius: var(--radius-full);
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
  }

  .tag-default {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    background: var(--accent);
    color: var(--accent-fg);
    padding: 2px 6px;
    border-radius: 4px;
    letter-spacing: 0.03em;
  }

  .tag-remove {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    padding: 0 2px;
    transition: color 0.15s;
  }

  .tag-remove:hover {
    color: var(--error);
  }

  .lang-add {
    width: 100%;
    padding: 10px 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 14px;
    font-weight: 500;
    font-family: var(--font-sleek, inherit);
    cursor: pointer;
    transition: var(--transition-sleek, 200ms ease);
  }

  .lang-add:focus {
    outline: none;
    border-color: var(--text-muted);
  }

  /* ── Step 6: Ready ── */
  .step-ready {
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .ready-visual {
    position: relative;
    width: 100px;
    height: 100px;
    margin-bottom: 28px;
  }

  .ready-ring {
    position: absolute;
    border-radius: 50%;
    border: 2px solid var(--accent);
  }

  .ready-ring.r1 {
    inset: 0;
    opacity: 0.15;
    animation: ready-expand 2.5s ease-out infinite;
  }

  .ready-ring.r2 {
    inset: 10px;
    opacity: 0.25;
    animation: ready-expand 2.5s ease-out 0.4s infinite;
  }

  .ready-ring.r3 {
    inset: 20px;
    opacity: 0.4;
    animation: ready-expand 2.5s ease-out 0.8s infinite;
  }

  @keyframes ready-expand {
    0% { transform: scale(0.95); opacity: 0.5; }
    50% { transform: scale(1.1); opacity: 0.2; }
    100% { transform: scale(0.95); opacity: 0.5; }
  }

  .ready-check {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    background: var(--primary-alpha);
    border-radius: 50%;
    animation: check-pop 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
  }

  .ready-check :global(svg) {
    width: 40px;
    height: 40px;
  }

  @keyframes check-pop {
    0% { transform: scale(0); opacity: 0; }
    100% { transform: scale(1); opacity: 1; }
  }

  .demo-box {
    width: 100%;
    max-width: 440px;
    text-align: left;
  }

  .demo-prompt {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 14px;
  }

  .demo-prompt kbd {
    padding: 6px 14px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-family: var(--font-sleek, 'Inter', ui-monospace, monospace);
    font-size: 14px;
    font-weight: 700;
    color: var(--text);
  }

  .demo-prompt span {
    font-size: 14px;
    color: var(--text-secondary);
  }

  .demo-box textarea {
    width: 100%;
    height: 90px;
    padding: 14px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 14px;
    font-family: var(--font-sleek, inherit);
    resize: none;
    transition: var(--transition-sleek, 200ms ease);
  }

  .demo-box textarea:focus {
    outline: none;
    border-color: var(--text-muted);
  }

  .demo-box.unfocused {
    position: relative;
  }

  .demo-box.unfocused textarea {
    border-color: var(--warning);
    opacity: 0.6;
  }

  .refocus-cue {
    display: block;
    width: 100%;
    margin-top: 8px;
    padding: 10px 16px;
    background: rgba(245, 158, 11, 0.08);
    border: 1px dashed var(--warning);
    border-radius: var(--radius-sm);
    color: var(--warning);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    text-align: center;
  }

  .refocus-cue:hover {
    background: rgba(245, 158, 11, 0.15);
  }

  .refocus-cue kbd {
    padding: 2px 8px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-family: var(--font-sleek, 'Inter', ui-monospace, monospace);
    font-size: 12px;
    font-weight: 700;
    color: var(--text);
  }

  /* ── Actions ── */
  .actions {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 36px;
    padding-top: 24px;
    border-top: 1px solid var(--border);
  }

  .actions-spacer {
    flex: 1;
  }

  .btn-next {
    padding: 11px 28px;
    background: var(--accent);
    border: none;
    border-radius: var(--radius-full);
    color: var(--accent-fg);
    font-size: 14px;
    font-weight: 600;
    font-family: var(--font-sleek, inherit);
    cursor: pointer;
    transition: var(--transition-sleek, 200ms ease);
  }

  .btn-next:hover:not(:disabled) {
    opacity: 0.92;
    transform: translateY(-1px);
  }

  .btn-next:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-finish {
    padding: 12px 36px;
    font-size: 15px;
  }

  .btn-back {
    padding: 11px 24px;
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    font-family: var(--font-sleek, inherit);
    cursor: pointer;
    transition: var(--transition-sleek, 200ms ease);
  }

  .btn-back:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  /* ── Skip ── */
  .btn-skip {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 13px;
    cursor: pointer;
    text-decoration: underline;
    padding: 6px 10px;
    transition: color 0.15s;
  }

  .btn-skip:hover {
    color: var(--text-secondary);
  }

  .btn-skip:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .skip-error {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--error);
    text-align: center;
  }

  .skip-mobile {
    display: none;
  }

  /* ── Responsive ── */
  @media (max-width: 700px) {
    .onboarding {
      flex-direction: column;
    }

    .stepper {
      display: none;
    }

    .stepper-mobile {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 14px 20px;
      background: var(--bg-elevated);
      border-bottom: 1px solid var(--border);
      flex-shrink: 0;
    }

    .stepper-logo-sm {
      width: 28px;
      height: 28px;
    }

    .stepper-dots {
      display: flex;
      gap: 6px;
      flex: 1;
      justify-content: center;
    }

    .dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--border);
      transition: all 0.2s;
    }

    .dot.active {
      background: var(--accent);
      transform: scale(1.3);
    }

    .dot.complete {
      background: var(--success);
    }

    .stepper-mobile-text {
      font-size: 12px;
      font-weight: 600;
      color: var(--text-muted);
      white-space: nowrap;
    }

    .content-scroll {
      padding: 24px 20px;
    }

    .onboarding h1 {
      font-size: 22px;
    }

    .subtitle {
      font-size: 14px;
      margin-bottom: 24px;
    }

    .features-grid {
      grid-template-columns: 1fr;
      gap: 10px;
    }

    .feat {
      padding: 14px;
    }

    .form-card {
      padding: 20px;
    }

    .mode-pills {
      flex-direction: column;
      gap: 8px;
    }

    .mode-detail {
      padding: 18px;
    }

    .api-key-row {
      flex-direction: column;
      align-items: stretch;
      gap: 8px;
    }

    .btn-validate {
      width: 100%;
    }

    .ctrl-section {
      padding: 18px;
    }

    .demo-box {
      max-width: 100%;
    }

    .actions {
      margin-top: 24px;
      padding-top: 16px;
    }

    .skip-mobile {
      display: block;
      text-align: center;
      margin-top: 16px;
      padding-bottom: 20px;
    }

    .stepper-skip {
      display: none;
    }
  }
</style>
