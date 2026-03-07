<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { initTelemetry, optOut } from '../lib/telemetry'
  import { sidebarDictationStore } from '../lib/sidebarDictation'
  import { LANGUAGE_OPTIONS, languageLabel, isLanguageSupportedByProvider } from '../lib/languages'
  import type { AppConfig, AudioDevice } from '../types'
  import HotkeyCapture from '../components/HotkeyCapture.svelte'

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
  let saveError: string | null = null
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
  }

  let hardwareReqs: Record<string, ModelRequirement> = {}
  let modelStatuses: Record<string, ModelStatusEntry> = {}
  let statusPollInterval: ReturnType<typeof setInterval> | null = null

  const tabs = [
    { id: 'general', label: 'General' },
    { id: 'dictation', label: 'Audio & Dictation' },
    { id: 'privacy', label: 'Privacy' },
    { id: 'advanced', label: 'Advanced' },
  ]

  onMount(async () => {
    try {
      // Load settings and audio devices in parallel
      const [settings, devices, platform, sensevoiceReqs, whisperReqs] = await Promise.all([
        invoke('get_settings') as Promise<AppConfig>,
        invoke('get_audio_devices') as Promise<AudioDevice[]>,
        invoke('get_platform') as Promise<string>,
        invoke('check_model_requirements', { modelId: 'sensevoice' }),
        invoke('check_model_requirements', { modelId: 'whisper_base' }),
      ])
      
      hardwareReqs['sensevoice'] = sensevoiceReqs as ModelRequirement
      hardwareReqs['whisper_base'] = whisperReqs as ModelRequirement
      
      config = settings
      audioDevices = devices
      sidebarDictationStore.updateFromConfig(settings, platform)
      // Normalize so UI never sees missing or invalid shape
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
        if (!config.waveform_style) config.waveform_style = 'Heartbeat'
        if (!config.overlay_position) config.overlay_position = 'BottomCenter'
        if (config.overlay_offset_x == null) config.overlay_offset_x = 0
        if (config.overlay_offset_y == null) config.overlay_offset_y = 0
        if (!config.overlay_expand_direction) config.overlay_expand_direction = 'Up'
      }
      
      // Check if API key is already configured
      hasApiKey = !!config?.stt_config?.api_key
      // Don't populate the input with the actual key for security
      apiKeyInput = ''
      
      await refreshModelStatuses()
      statusPollInterval = setInterval(refreshModelStatuses, 2000)
      
      console.log('Loaded audio devices:', devices)
    } catch (e) {
      console.error('Failed to load settings:', e)
    } finally {
      initialLoadDone = true
    }
  })

  onDestroy(() => {
    if (statusPollInterval) clearInterval(statusPollInterval)
  })

  async function refreshModelStatuses() {
    try {
      modelStatuses = await invoke('get_model_status') as Record<string, ModelStatusEntry>
    } catch (e) {
      console.error('Failed to get model statuses:', e)
    }
  }

  async function startModel(modelId: string) {
    try {
      await invoke('start_local_model', { modelId })
      await refreshModelStatuses()
    } catch (e) {
      console.error('Failed to start model:', e)
    }
  }

  async function stopModel(modelId: string) {
    try {
      await invoke('stop_local_model', { modelId })
      await refreshModelStatuses()
    } catch (e) {
      console.error('Failed to stop model:', e)
    }
  }

  async function restartModel(modelId: string) {
    try {
      await invoke('restart_local_model', { modelId })
      await refreshModelStatuses()
    } catch (e) {
      console.error('Failed to restart model:', e)
    }
  }

  async function deleteModel(modelId: string) {
    if (!confirm('Are you sure you want to delete this model? You will need to download it again.')) return
    try {
      await invoke('delete_local_model', { modelId })
      await refreshModelStatuses()
    } catch (e) {
      console.error('Failed to delete model:', e)
    }
  }

  async function downloadModel(modelId: string) {
    try {
      await invoke('download_model', { modelType: modelId })
      await refreshModelStatuses()
    } catch (e) {
      console.error('Failed to download model:', e)
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

  async function saveSettings() {
    console.log('saveSettings called')
    if (!config) {
      console.log('No config to save')
      return
    }
    saving = true
    
    // If user entered a new API key, update it
    if (apiKeyInput.trim()) {
      config.stt_config.api_key = apiKeyInput.trim()
    }
    // Clamp logging max_records to valid range
    if (config.logging) {
      config.logging.max_records = Math.min(20000, Math.max(500, config.logging.max_records || 2000))
    }
    if (config.language_toggle_hotkey === '') config.language_toggle_hotkey = null
    if (!Array.isArray(config.languages) || config.languages.length === 0) config.languages = ['en']

    console.log('Config object:', JSON.stringify(config, null, 2))
    console.log('Saving config with api_key:', config.stt_config.api_key ? 'present' : 'missing')
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
      hasApiKey = !!config.stt_config.api_key
      apiKeyInput = '' // Clear input after save
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
    try {
      const content = await invoke('get_app_log') as string
      if (!content || content.trim() === '') return
      const blob = new Blob([content], { type: 'text/plain' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `kalam-log-${new Date().toISOString().slice(0, 19).replace(/:/g, '-')}.log`
      a.click()
      URL.revokeObjectURL(url)
      await checkLogEmpty()
    } catch (e) {
      console.error('Failed to download log:', e)
    }
  }

  async function openAppDataFolder() {
    try {
      await invoke('open_app_data_folder')
    } catch (e) {
      console.error('Failed to open app data folder:', e)
    }
  }

  async function confirmAndReset() {
    if (!confirm('Reset the entire application? This will delete all settings, history, and data. You will see the onboarding again. This cannot be undone.')) return
    resetError = null
    resetting = true
    try {
      await invoke('reset_application')
    } catch (e) {
      console.error('Reset failed:', e)
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
      console.log('Microphone test result:', result.level, 'samples:', result.samples?.length)
      if (result.samples?.length && result.sample_rate) {
        await playTestAudio(result.samples, result.sample_rate)
      }
    } catch (e) {
      console.error('Microphone test failed:', e)
      micLevel = 0
    } finally {
      testingMic = false
    }
  }

  async function checkApiKey() {
    console.log('checkApiKey called')
    // Use the input value if present, otherwise use the stored key
    const keyToCheck = apiKeyInput.trim() || config?.stt_config?.api_key
    
    if (!keyToCheck) {
      console.log('No API key to check')
      return
    }
    console.log('Calling check_api_key with provider:', config?.stt_config?.provider)
    console.log('API key length:', keyToCheck.length)
    try {
      apiKeyValid = await invoke('check_api_key', {
        provider: config?.stt_config?.provider || 'groq',
        apiKey: keyToCheck
      })
      console.log('API validation result:', apiKeyValid)
    } catch (e) {
      console.error('API validation error:', e)
      apiKeyValid = false
    }
  }
  
  function clearApiKey() {
    if (config) {
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
        console.log('Refreshed audio devices:', audioDevices)
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
      config = { ...config, hotkey }
      scheduleSave()
    }
  }

  function setLanguageToggleHotkey(hotkey: string) {
    if (config) {
      config = { ...config, language_toggle_hotkey: hotkey === '' ? null : hotkey }
      scheduleSave()
    }
  }

  $: langProviderKey = config
    ? (
        config.stt_config.mode === 'Local'
          ? (config.stt_config.local_model || 'sensevoice')
          : (config.stt_config.provider || 'groq')
      )
    : 'groq'
</script>

{#if config}
  <div class="settings">
    <header>
      <h2>Settings</h2>
      {#if saving}
        <span class="save-status">Saving…</span>
      {:else if saveError}
        <span class="save-status error">Save failed</span>
      {/if}
    </header>
    {#if saveError}
      <p class="save-error" role="alert">{saveError}</p>
    {/if}

    <div class="tabs">
      {#each tabs as tab}
        <button
          class="tab"
          class:active={activeTab === tab.id}
          on:click={() => {
            activeTab = tab.id
            if (tab.id === 'logging') checkLogEmpty()
          }}
        >
          {tab.label}
        </button>
      {/each}
    </div>

    <div class="tab-content">
      {#if activeTab === 'general'}
        <section>
          <h3>Hotkey & Recording</h3>
          <div class="form-group">
            <label for="hotkey">Activation Hotkey</label>
            <HotkeyCapture 
              value={config.hotkey} 
              onChange={setHotkey}
            />
          </div>

          <div class="form-group">
            <label for="recording-mode">Recording Mode</label>
            <select id="recording-mode" bind:value={config.recording_mode} on:change={scheduleSave}>
              <option value="Hold">Hold to record</option>
              <option value="Toggle">Toggle mode</option>
            </select>
            {#if config.recording_mode === 'Hold'}
              <p class="hint">Press and hold the hotkey to record, release to stop</p>
              
              <div class="sub-setting" style="margin-top: 12px;">
                <label for="min-hold-ms">Short-press threshold (ms)</label>
                <input 
                  id="min-hold-ms" 
                  type="number" 
                  min="0" 
                  max="2000" 
                  step="50"
                  bind:value={config.min_hold_ms}
                  on:change={scheduleSave}
                />
                <p class="hint">If you hold the key for less than this time, recording is cancelled.</p>
              </div>
            {:else}
              <p class="hint">Press hotkey once to start, press again to stop</p>
            {/if}
          </div>
        </section>

        <section>
          <h3>Startup</h3>
          <div class="form-group checkbox">
            <label>
              <input type="checkbox" bind:checked={config.auto_start} on:change={scheduleSave} />
              Start on login
            </label>
          </div>

          <div class="form-group checkbox">
            <label>
              <input type="checkbox" bind:checked={config.start_in_focus} on:change={scheduleSave} />
              Start in focus (show window on startup)
            </label>
            <p class="hint">If disabled, app starts minimized to tray and plays a sound</p>
          </div>
        </section>

        <section>
          <h3>Overlay Appearance</h3>
          
          <div class="form-group">
            <label for="waveform-style">Waveform Style</label>
            <select id="waveform-style" bind:value={config.waveform_style} on:change={scheduleSave}>
              <option value="Line">Line</option>
              <option value="Symmetric">Symmetric Wave</option>
              <option value="Heartbeat">Heartbeat</option>
              <option value="Snake">Snake</option>
              <option value="DoubleHelix">Double Helix</option>
              <option value="Liquid">Liquid</option>
              <option value="Waves">Waves</option>
              <option value="Glitch">Glitch</option>
              <option value="Bars">Bars</option>
              <option value="CenterSplit">Center Split</option>
            </select>
            <p class="hint">Choose how your voice is visualized in the overlay pill.</p>
          </div>

          <div class="form-group">
            <label for="overlay-expand-direction">Expand Direction</label>
            <select id="overlay-expand-direction" bind:value={config.overlay_expand_direction} on:change={scheduleSave}>
              <option value="Up">Upwards (Default)</option>
              <option value="Down">Downwards</option>
              <option value="Center">Center</option>
            </select>
            <p class="hint">Which direction the pill expands when you start dictating.</p>
          </div>

          <div class="form-group">
            <label for="overlay-position">Screen Position</label>
            <select id="overlay-position" bind:value={config.overlay_position} on:change={scheduleSave}>
              <option value="BottomCenter">Bottom Center (Default)</option>
              <option value="BottomLeft">Bottom Left</option>
              <option value="BottomRight">Bottom Right</option>
              <option value="TopCenter">Top Center</option>
              <option value="TopLeft">Top Left</option>
              <option value="TopRight">Top Right</option>
              <option value="CenterLeft">Center Left</option>
              <option value="CenterRight">Center Right</option>
              <option value="Center">Center</option>
            </select>
            <p class="hint">Where the pill appears on your primary monitor.</p>
          </div>

          <div class="form-group row-group">
            <div class="sub-setting">
              <label for="overlay-offset-x">X Offset (px)</label>
              <input
                id="overlay-offset-x"
                type="number"
                bind:value={config.overlay_offset_x}
                on:input={scheduleSave}
              />
            </div>
            <div class="sub-setting">
              <label for="overlay-offset-y">Y Offset (px)</label>
              <input
                id="overlay-offset-y"
                type="number"
                bind:value={config.overlay_offset_y}
                on:input={scheduleSave}
              />
            </div>
          </div>
          <p class="hint">Fine-tune the position by adding horizontal (X) or vertical (Y) pixel offsets.</p>
        </section>

      {:else if activeTab === 'dictation'}
        <section>
          <h3>Audio Input</h3>
          <div class="form-group">
            <label for="microphone">
              Microphone
              <button class="btn-refresh" on:click={refreshAudioDevices} title="Refresh device list">
                ↻
              </button>
            </label>
            <select id="microphone" bind:value={config.audio_device} on:change={scheduleSave}>
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
              <p class="hint warning">No audio devices found. Try refreshing the list.</p>
            {:else}
              <p class="hint">{audioDevices.length} audio device(s) available</p>
            {/if}
          </div>

          <div class="form-group">
            <span class="label-text">Test Microphone</span>
            {#if testingMic}
              <button class="btn-secondary" on:click={stopTestRecording}>
                Stop
              </button>
            {:else}
              <button class="btn-secondary" on:click={startTestRecording}>
                Start
              </button>
            {/if}
            <p class="hint">Record with Start/Stop, then hear playback. The bar shows how loud your mic picked up the recording.</p>
            <div class="mic-level-container">
              <div class="mic-level" role="meter" aria-label="Microphone input level" aria-valuenow={Math.round(micLevel * 100)} aria-valuemin={0} aria-valuemax={100}>
                <div class="mic-bar" style="width: {micLevel * 100}%"></div>
              </div>
              {#if testingMic}
                <span class="mic-status">Recording… Click Stop when done.</span>
              {:else if micLevel > 0}
                <span class="mic-status">Volume captured: {Math.round(micLevel * 100)}% — higher means your mic heard you louder</span>
              {/if}
            </div>
          </div>

          <div class="form-group">
            <label for="vad-preset">VAD Sensitivity</label>
            <select id="vad-preset" bind:value={config.stt_config.vad_preset} on:change={scheduleSave}>
              <option value="Fast">Fast (0.8s silence)</option>
              <option value="Balanced">Balanced (1.5s silence)</option>
              <option value="Accurate">Accurate (2.5s silence)</option>
            </select>
          </div>
        </section>

        <section>
          <h3>Speech-to-Text Mode</h3>
          <div class="form-group">
            <label for="stt-mode">Mode</label>
            <select id="stt-mode" bind:value={config.stt_config.mode} on:change={scheduleSave}>
              <option value="Cloud">Cloud (Groq API)</option>
              <option value="Local">Local (SenseVoice)</option>
              <option value="Hybrid">Hybrid (Auto-switch)</option>
            </select>
          </div>

          {#if config.stt_config.mode === 'Cloud' || config.stt_config.mode === 'Hybrid'}
            <div class="form-group">
              <label for="cloud-provider">Cloud Provider</label>
              <select id="cloud-provider" bind:value={config.stt_config.provider} on:change={scheduleSave}>
                <option value="groq">Groq (whisper-large-v3-turbo)</option>
                <option value="openai">OpenAI (whisper-1)</option>
              </select>
            </div>

            <div class="form-group">
              <label for="api-key">
                API Key
                {#if hasApiKey && !apiKeyInput}
                  <span class="badge configured">✓ Configured</span>
                {/if}
              </label>
              <div class="input-group">
                <input
                  id="api-key"
                  type="password"
                  bind:value={apiKeyInput}
                  on:input={scheduleSave}
                  placeholder={hasApiKey ? "•••••••••••••••• (enter new key to change)" : `Enter your ${config.stt_config.provider === 'openai' ? 'OpenAI' : 'Groq'} API key`}
                />
                <button class="btn-secondary" on:click={checkApiKey}>
                  Validate
                </button>
                {#if hasApiKey && !apiKeyInput}
                  <button class="btn-secondary btn-danger" on:click={clearApiKey}>
                    Clear
                  </button>
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
          {/if}

          {#if config.stt_config.mode === 'Local' || config.stt_config.mode === 'Hybrid'}
            <div class="form-group">
              <label for="local-model">Active local model</label>
              <select id="local-model" bind:value={config.stt_config.local_model} on:change={scheduleSave}>
                <option value="sensevoice">SenseVoice Small</option>
                <option value="whisper_base">Whisper Base</option>
              </select>
              <p class="hint">This model is used when mode is Local.</p>
            </div>

            <div class="form-group">
              <span class="label-text">Local Models</span>
              <div class="model-list">
                <div class="model-item">
                  <div class="model-info">
                    <strong>SenseVoice Small</strong>
                    <span>200 MB • Fast • 50+ languages</span>
                    {#if hardwareReqs['sensevoice'] && !hardwareReqs['sensevoice'].can_run}
                      <span class="warning">⚠️ {hardwareReqs['sensevoice'].reason}</span>
                    {/if}
                    {#if modelStatuses['sensevoice']}
                      <span class="status-badge {modelStatuses['sensevoice'].status.toLowerCase()}">{modelStatuses['sensevoice'].status}</span>
                    {/if}
                  </div>
                  <div class="model-actions">
                    {#if !modelStatuses['sensevoice']?.installed}
                      <button class="btn-secondary" disabled={hardwareReqs['sensevoice'] && !hardwareReqs['sensevoice'].can_run} on:click={() => downloadModel('sensevoice')}>Download</button>
                    {:else}
                      {#if modelStatuses['sensevoice'].status === 'Stopped' || modelStatuses['sensevoice'].status === 'Error'}
                        <button class="btn-secondary" on:click={() => startModel('sensevoice')}>Start</button>
                      {:else if modelStatuses['sensevoice'].status === 'Running'}
                        <button class="btn-secondary" on:click={() => stopModel('sensevoice')}>Stop</button>
                        <button class="btn-secondary" on:click={() => restartModel('sensevoice')}>Restart</button>
                      {/if}
                      <button class="btn-secondary btn-danger" title="Delete model" on:click={() => deleteModel('sensevoice')}>Delete</button>
                    {/if}
                  </div>
                </div>

                <div class="model-item">
                  <div class="model-info">
                    <strong>Whisper Base</strong>
                    <span>142 MB • Good quality</span>
                    {#if hardwareReqs['whisper_base'] && !hardwareReqs['whisper_base'].can_run}
                      <span class="warning">⚠️ {hardwareReqs['whisper_base'].reason}</span>
                    {/if}
                    {#if modelStatuses['whisper_base']}
                      <span class="status-badge {modelStatuses['whisper_base'].status.toLowerCase()}">{modelStatuses['whisper_base'].status}</span>
                    {/if}
                  </div>
                  <div class="model-actions">
                    {#if !modelStatuses['whisper_base']?.installed}
                      <button class="btn-secondary" disabled={hardwareReqs['whisper_base'] && !hardwareReqs['whisper_base'].can_run} on:click={() => downloadModel('whisper_base')}>Download</button>
                    {:else}
                      {#if modelStatuses['whisper_base'].status === 'Stopped' || modelStatuses['whisper_base'].status === 'Error'}
                        <button class="btn-secondary" on:click={() => startModel('whisper_base')}>Start</button>
                      {:else if modelStatuses['whisper_base'].status === 'Running'}
                        <button class="btn-secondary" on:click={() => stopModel('whisper_base')}>Stop</button>
                        <button class="btn-secondary" on:click={() => restartModel('whisper_base')}>Restart</button>
                      {/if}
                      <button class="btn-secondary btn-danger" title="Delete model" on:click={() => deleteModel('whisper_base')}>Delete</button>
                    {/if}
                  </div>
                </div>
              </div>
            </div>
          {/if}

          <div class="form-group language-multiselect">
            <label for="recognition-languages">Recognition languages</label>
            <p class="hint">First language is the default. Support depends on the current provider above; unsupported languages are dimmed.</p>
            <div class="selected-languages">
              {#each (config.languages ?? ['en']) as code, i}
                {@const supported = isLanguageSupportedByProvider(code, langProviderKey)}
                <div class="lang-row" class:unsupported={!supported}>
                  <span class="lang-badge">
                    {#if i === 0}<span class="default-tag">Default</span> {/if}
                    {languageLabel(code)}
                    {#if !supported}
                      <span class="unsupported-icon" title="Not supported by current STT provider">⚠</span>
                    {/if}
                  </span>
                  <div class="lang-actions">
                    {#if i > 0}
                      <button type="button" class="btn-icon" title="Move up" on:click={() => moveLanguageUp(i)}>↑</button>
                    {/if}
                    {#if i < (config.languages?.length ?? 1) - 1}
                      <button type="button" class="btn-icon" title="Move down" on:click={() => moveLanguageDown(i)}>↓</button>
                    {/if}
                    <button type="button" class="btn-icon remove" title="Remove" on:click={() => removeLanguage(i)}>×</button>
                  </div>
                </div>
              {/each}
            </div>
            <div class="add-language">
              <select
                id="recognition-languages"
                bind:value={addLanguageCode}
                on:change={addSelectedLanguage}
              >
                <option value="">Add a language…</option>
                {#each LANGUAGE_OPTIONS as opt}
                  <option value={opt.code} disabled={(config?.languages ?? []).includes(opt.code)}>{opt.label}</option>
                {/each}
              </select>
            </div>
          </div>

          {#if (config?.languages?.length ?? 0) >= 2}
            <div class="form-group">
              <label for="language-toggle-hotkey">Language toggle hotkey</label>
              <HotkeyCapture
                value={config.language_toggle_hotkey ?? ''}
                onChange={setLanguageToggleHotkey}
              />
              <p class="hint">Press to switch between the first two languages. A notification is shown on toggle.</p>
            </div>
          {/if}
        </section>

        <section>
          <h3>Text Formatting</h3>
          <div class="form-group checkbox">
            <label>
              <input type="checkbox" bind:checked={config.formatting.voice_commands} on:change={scheduleSave} />
              Enable voice commands ("period", "new line", etc.)
            </label>
          </div>

          <div class="form-group checkbox">
            <label>
              <input type="checkbox" bind:checked={config.formatting.filler_word_removal} on:change={scheduleSave} />
              Remove filler words ("um", "uh", "like")
            </label>
          </div>

          <div class="form-group checkbox">
            <label>
              <input type="checkbox" bind:checked={config.formatting.auto_punctuation} on:change={scheduleSave} />
              Auto-punctuation
            </label>
          </div>

          <div class="form-group">
            <label for="injection-method">Text Injection Method</label>
            <select id="injection-method" bind:value={config.formatting.injection_method} on:change={scheduleSave}>
              <option value="Auto">Auto (recommended)</option>
              <option value="Keystrokes">Keystrokes only</option>
              <option value="Clipboard">Clipboard only</option>
            </select>
          </div>
        </section>

      {:else if activeTab === 'privacy'}
        <section>
          <h3>Privacy</h3>
          <div class="form-group">
            <label for="history-retention">History Retention</label>
            <select id="history-retention" bind:value={config.privacy.history_retention_days} on:change={scheduleSave}>
              <option value={7}>7 days</option>
              <option value={30}>30 days</option>
              <option value={90}>90 days</option>
              <option value={365}>1 year</option>
              <option value={0}>Forever</option>
            </select>
          </div>

          <div class="form-group checkbox">
            <label>
              <input type="checkbox" bind:checked={config.privacy.sensitive_app_detection} on:change={scheduleSave} />
              Auto-switch to local mode in sensitive apps (password managers, etc.)
            </label>
          </div>

          <div class="form-group checkbox">
            <label>
              <input type="checkbox" bind:checked={config.privacy.telemetry_enabled} on:change={scheduleSave} />
              Help improve Kalam by sending anonymous usage data
            </label>
            <p class="hint">No audio or text is ever sent. Only metrics like session duration.</p>
          </div>
        </section>

      {:else if activeTab === 'advanced'}
        <section>
          <h3>App Data & Logging</h3>
          <p class="hint" style="margin-bottom: 16px;">
            When enabled, the app keeps a bounded in-memory log (no transcription or personal data).
            Use it to export a log file for support if something goes wrong.
          </p>
          <div class="form-group">
            <span class="label-text">App data folder</span>
            <p class="hint">Config and app data are stored here. Click to open the folder.</p>
            <button
              type="button"
              class="btn-secondary btn-link"
              on:click={openAppDataFolder}
              title="Open app data folder in file manager"
            >
              Open app data folder
            </button>
          </div>
          <div class="form-group checkbox">
            <label>
              <input type="checkbox" bind:checked={config.logging.enabled} on:change={scheduleSave} />
              Enable in-app logging
            </label>
          </div>
          <div class="form-group">
            <label for="log-level">Log level</label>
            <select id="log-level" bind:value={config.logging.level} on:change={scheduleSave}>
              <option value="Off">Off</option>
              <option value="Error">Error</option>
              <option value="Warn">Warn</option>
              <option value="Info">Info</option>
              <option value="Debug">Debug</option>
            </select>
          </div>
          <div class="form-group">
            <label for="log-max-records">Max records to keep</label>
            <input
              id="log-max-records"
              type="number"
              min="500"
              max="20000"
              step="500"
              bind:value={config.logging.max_records}
              on:change={scheduleSave}
            />
            <p class="hint">Between 500 and 20,000. Oldest entries are dropped when the limit is reached.</p>
          </div>
          <div class="form-group">
            <span class="label-text">Export log</span>
            <button
              class="btn-secondary"
              on:click={downloadLog}
              disabled={logEmpty}
              title={logEmpty ? 'No log entries yet' : 'Download current log buffer as a file'}
            >
              Download log
            </button>
            <p class="hint">
              {#if logEmpty}
                No log entries yet. Enable logging and use the app to capture entries, then download.
              {:else}
                Saves the current in-memory log to a .log file. No transcription or sensitive data is included.
              {/if}
            </p>
          </div>
        </section>

        <section class="danger-zone">
          <h4>Danger Zone</h4>
          <p class="hint" style="margin-bottom: 12px;">Reset removes all configuration, history, and data. You will see the onboarding again as if the app were newly installed.</p>
          {#if resetError}
            <p class="save-error" role="alert" style="margin-bottom: 12px;">{resetError}</p>
          {/if}
          <button
            class="btn-danger"
            disabled={resetting}
            on:click={confirmAndReset}
          >
            {resetting ? 'Resetting…' : 'Reset entire application'}
          </button>
        </section>
      {/if}
    </div>
  </div>
{/if}

<style>
  .settings {
    max-width: 900px;
    padding: 8px;
    animation: fadeIn 0.4s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(10px); }
    to { opacity: 1; transform: translateY(0); }
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 32px;
  }

  h2 {
    font-size: 32px;
    font-weight: 700;
  }

  .tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 32px;
    background: var(--bg-app);
    padding: 6px;
    border-radius: var(--radius-pill);
    overflow-x: auto;
    scrollbar-width: none; /* Firefox */
  }
  
  .tabs::-webkit-scrollbar {
    display: none; /* Chrome/Safari */
  }

  .tab {
    padding: 10px 20px;
    background: transparent;
    border: none;
    border-radius: var(--radius-pill);
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    white-space: nowrap;
  }

  .tab:hover {
    color: var(--navy-deep);
  }

  .tab.active {
    background: var(--bg-card);
    color: var(--primary-dark);
    box-shadow: var(--shadow-sm);
  }

  section {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 32px;
    margin-bottom: 24px;
    box-shadow: var(--shadow-sm);
    transition: box-shadow 0.3s ease;
  }
  
  section:hover {
    box-shadow: var(--shadow-md);
  }

  section h3 {
    font-size: 18px;
    font-weight: 700;
    margin-bottom: 24px;
    color: var(--navy-deep);
    border-bottom: 1px solid var(--border-subtle);
    padding-bottom: 16px;
  }

  .form-group {
    margin-bottom: 24px;
  }

  .form-group.row-group {
    display: flex;
    gap: 20px;
    margin-bottom: 12px;
  }
  
  .form-group.row-group .sub-setting {
    flex: 1;
    margin-top: 0;
  }

  .form-group:last-child {
    margin-bottom: 0;
  }

  label,
  .label-text {
    display: block;
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 10px;
    color: var(--navy-deep);
  }

  .form-group.checkbox label {
    display: flex;
    align-items: center;
    gap: 12px;
    cursor: pointer;
    font-weight: 500;
    color: var(--text-primary);
    user-select: none;
  }

  input[type="checkbox"] {
    appearance: none;
    -webkit-appearance: none;
    width: 20px;
    height: 20px;
    margin: 0;
    background: var(--bg-input);
    border: 2px solid var(--border-visible);
    border-radius: 6px;
    cursor: pointer;
    position: relative;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: var(--shadow-inner);
    flex-shrink: 0;
  }

  input[type="checkbox"]:hover {
    border-color: var(--primary);
    background: var(--bg-card);
  }

  input[type="checkbox"]:checked {
    background: var(--primary);
    border-color: var(--primary);
    box-shadow: 0 2px 8px var(--primary-alpha);
  }

  input[type="checkbox"]:checked::after {
    content: '';
    position: absolute;
    left: 5px;
    top: 1px;
    width: 4px;
    height: 9px;
    border: solid var(--white);
    border-width: 0 2px 2px 0;
    transform: rotate(45deg);
    animation: checkmark 0.2s cubic-bezier(0.4, 0, 0.2, 1) forwards;
  }

  @keyframes checkmark {
    0% { height: 0; width: 0; opacity: 0; }
    100% { height: 9px; width: 4px; opacity: 1; }
  }

  select {
    appearance: none;
    -webkit-appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='%2364748B' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 16px center;
    background-size: 16px;
    padding-right: 48px !important;
  }

  input[type="password"],
  input[type="number"],
  select {
    width: 100%;
    padding: 14px 16px;
    background: var(--bg-input);
    border: 2px solid transparent;
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font-size: 15px;
    font-weight: 500;
    transition: all 0.2s ease;
    box-shadow: var(--shadow-inner);
  }

  input[type="password"]:focus,
  input[type="number"]:focus,
  select:focus {
    outline: none;
    background: var(--bg-card);
    border-color: var(--primary);
    box-shadow: 0 4px 12px var(--primary-alpha);
  }
  
  input[type="password"]:hover,
  input[type="number"]:hover,
  select:hover {
    background: var(--bg-input-hover);
  }

  .save-error {
    margin: 0 0 20px;
    padding: 14px 16px;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid var(--error);
    border-radius: var(--radius-md);
    color: var(--error);
    font-size: 14px;
    font-weight: 500;
  }

  .hint {
    font-size: 13px;
    color: var(--text-muted);
    margin-top: 8px;
    line-height: 1.5;
  }

  .hint a {
    color: var(--primary);
    text-decoration: none;
    font-weight: 500;
  }
  
  .hint a:hover {
    text-decoration: underline;
  }

  .hint.warning {
    color: var(--warning);
  }

  .input-group {
    display: flex;
    gap: 12px;
  }

  .input-group input {
    flex: 1;
  }

  .language-multiselect .selected-languages {
    margin-bottom: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .language-multiselect .lang-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 16px;
    background: var(--bg-input);
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    transition: all 0.2s ease;
  }
  
  .language-multiselect .lang-row:hover {
    background: var(--bg-card);
    border-color: var(--border);
    box-shadow: var(--shadow-sm);
  }

  .language-multiselect .lang-badge {
    font-size: 15px;
    font-weight: 500;
    color: var(--navy-deep);
  }

  .language-multiselect .default-tag {
    display: inline-block;
    background: var(--primary);
    color: var(--white);
    font-size: 11px;
    font-weight: 700;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    margin-right: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .language-multiselect .lang-actions {
    display: flex;
    gap: 6px;
  }

  .language-multiselect .btn-icon {
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
  }

  .language-multiselect .btn-icon:hover {
    background: var(--bg-input);
    color: var(--navy-deep);
    border-color: var(--border-visible);
  }

  .language-multiselect .btn-icon.remove {
    color: var(--error);
  }
  
  .language-multiselect .btn-icon.remove:hover {
    background: rgba(239, 68, 68, 0.1);
    border-color: var(--error);
  }

  .language-multiselect .lang-row.unsupported {
    opacity: 0.7;
    background: var(--bg-app);
  }

  .language-multiselect .lang-row.unsupported .lang-badge {
    color: var(--text-muted);
  }

  .language-multiselect .unsupported-icon {
    margin-left: 8px;
    color: var(--warning);
    font-size: 14px;
  }

  .language-multiselect .add-language select {
    width: 100%;
  }

  .validation {
    font-size: 13px;
    font-weight: 500;
    margin-top: 8px;
    display: block;
  }

  .validation.success {
    color: var(--success);
  }

  .validation.error {
    color: var(--error);
  }

  .btn-primary {
    padding: 12px 24px;
    background: var(--primary);
    border: none;
    border-radius: var(--radius-md);
    color: var(--white);
    font-weight: 600;
    font-size: 14px;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 4px 12px var(--primary-alpha);
  }

  .btn-primary:hover {
    background: var(--primary-dark);
    transform: translateY(-1px);
    box-shadow: 0 6px 16px var(--primary-alpha);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
  }

  .save-status {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
    background: var(--bg-input);
    padding: 6px 12px;
    border-radius: var(--radius-pill);
  }

  .save-status.error {
    color: var(--error);
    background: rgba(239, 68, 68, 0.1);
  }

  .btn-secondary {
    padding: 12px 20px;
    background: var(--bg-card);
    border: 1px solid var(--border-visible);
    border-radius: var(--radius-md);
    color: var(--navy-deep);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: var(--shadow-sm);
  }

  .btn-secondary:hover {
    background: var(--bg-input);
    border-color: var(--navy-deep);
  }

  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    box-shadow: none;
  }

  .btn-secondary.btn-link {
    color: var(--primary);
    border-color: transparent;
    background: transparent;
    box-shadow: none;
    padding: 8px 0;
  }

  .btn-secondary.btn-link:hover {
    background: transparent;
    color: var(--primary-dark);
    text-decoration: underline;
  }

  .mic-level-container {
    margin-top: 16px;
    background: var(--bg-input);
    padding: 16px;
    border-radius: var(--radius-md);
  }

  .mic-level {
    height: 12px;
    background: var(--bg-card);
    border-radius: var(--radius-pill);
    overflow: hidden;
    box-shadow: var(--shadow-inner);
  }

  .mic-bar {
    height: 100%;
    background: linear-gradient(90deg, var(--primary), var(--primary-light));
    border-radius: var(--radius-pill);
    transition: width 0.1s ease-out;
    min-width: 4px;
  }

  .mic-status {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-top: 10px;
    display: block;
    text-align: center;
  }

  .btn-refresh {
    background: var(--bg-input);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 14px;
    margin-left: 12px;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    transition: all 0.2s ease;
  }

  .btn-refresh:hover {
    background: var(--bg-card);
    color: var(--navy-deep);
    border-color: var(--border-visible);
    box-shadow: var(--shadow-sm);
  }

  .model-list {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .model-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px;
    background: var(--bg-input);
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    transition: all 0.2s ease;
  }
  
  .model-item:hover {
    background: var(--bg-card);
    border-color: var(--border);
    box-shadow: var(--shadow-sm);
  }

  .model-info {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  
  .model-info strong {
    font-size: 15px;
    color: var(--navy-deep);
  }

  .model-info span {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .model-info span.warning {
    color: var(--warning);
    margin-top: 4px;
  }

  .status-badge {
    display: inline-block;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    margin-top: 4px;
  }
  
  .status-badge.running { background: var(--success); color: white; }
  .status-badge.stopped { background: var(--bg-app); color: var(--text-secondary); }
  .status-badge.starting { background: var(--primary); color: white; }
  .status-badge.error { background: var(--error); color: white; }
  .status-badge.notinstalled { background: var(--bg-app); color: var(--text-muted); }

  .model-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .danger-zone {
    margin-top: 40px;
    padding: 24px;
    background: rgba(239, 68, 68, 0.02);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: var(--radius-lg);
  }

  .danger-zone h4 {
    color: var(--error);
    margin-bottom: 12px;
    font-size: 16px;
  }

  .btn-danger {
    padding: 12px 24px;
    background: var(--white);
    border: 1px solid var(--error);
    border-radius: var(--radius-md);
    color: var(--error);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .btn-danger:hover {
    background: var(--error);
    color: var(--white);
    box-shadow: 0 4px 12px rgba(239, 68, 68, 0.2);
  }
  
  .badge {
    font-size: 11px;
    padding: 4px 10px;
    border-radius: var(--radius-pill);
    margin-left: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  
  .badge.configured {
    background: var(--success);
    color: var(--white);
  }
  
  .input-group .btn-danger {
    margin-left: 0;
  }

  @media (max-width: 768px) {
    .settings {
      padding: 0;
    }

    header {
      flex-direction: column;
      align-items: flex-start;
      gap: 12px;
      margin-bottom: 24px;
    }

    .tabs {
      padding: 4px;
    }

    .tab {
      padding: 8px 16px;
      font-size: 13px;
    }

    section {
      padding: 24px 20px;
    }

    .form-group.row-group {
      flex-direction: column;
      gap: 16px;
    }

    .input-group {
      flex-direction: column;
    }

    .input-group button {
      width: 100%;
      margin-left: 0 !important;
    }

    .language-multiselect .lang-row {
      flex-direction: column;
      align-items: flex-start;
      gap: 12px;
    }

    .language-multiselect .lang-actions {
      width: 100%;
      justify-content: flex-end;
    }
  }
</style>
