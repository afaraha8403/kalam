<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { initTelemetry, optOut } from '../lib/telemetry'
  import { LANGUAGE_OPTIONS, languageLabel, getSupportedLanguagesForProvider, isLanguageSupportedByProvider } from '../lib/languages'
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

  const tabs = [
    { id: 'general', label: 'General' },
    { id: 'audio', label: 'Audio' },
    { id: 'stt', label: 'STT Provider' },
    { id: 'formatting', label: 'Formatting' },
    { id: 'privacy', label: 'Privacy' },
    { id: 'logging', label: 'Logging' },
  ]

  onMount(async () => {
    try {
      // Load settings and audio devices in parallel
      const [settings, devices] = await Promise.all([
        invoke('get_settings') as Promise<AppConfig>,
        invoke('get_audio_devices') as Promise<AudioDevice[]>
      ])
      
      config = settings
      audioDevices = devices
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
      }
      
      // Check if API key is already configured
      hasApiKey = !!config?.stt_config?.api_key
      // Don't populate the input with the actual key for security
      apiKeyInput = ''
      
      console.log('Loaded audio devices:', devices)
    } catch (e) {
      console.error('Failed to load settings:', e)
    } finally {
      initialLoadDone = true
    }
  })

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
    ? (config.stt_config.mode === 'Local' ? 'sensevoice' : (config.stt_config.provider || 'groq'))
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
          <h3>Hotkey</h3>
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

      {:else if activeTab === 'audio'}
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

      {:else if activeTab === 'stt'}
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
                  placeholder={hasApiKey ? "•••••••••••••••• (enter new key to change)" : "Enter your Groq API key"}
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
                <a href="https://console.groq.com" target="_blank">Get your API key from Groq →</a>
              </p>
            </div>
          {/if}

          {#if config.stt_config.mode === 'Local' || config.stt_config.mode === 'Hybrid'}
            <div class="form-group">
              <span class="label-text">Local Models</span>
              <div class="model-list">
                <div class="model-item">
                  <div class="model-info">
                    <strong>SenseVoice Small</strong>
                    <span>200 MB • Fast • 50+ languages</span>
                  </div>
                  <button class="btn-secondary">Download</button>
                </div>
                <div class="model-item">
                  <div class="model-info">
                    <strong>Whisper Base</strong>
                    <span>142 MB • Good quality</span>
                  </div>
                  <button class="btn-secondary">Download</button>
                </div>
              </div>
            </div>
          {/if}

          <div class="form-group language-multiselect">
            <label>Recognition languages</label>
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

      {:else if activeTab === 'formatting'}
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

          <div class="danger-zone">
            <h4>Danger Zone</h4>
            <button class="btn-danger">Clear All History</button>
            <button class="btn-danger">Reset All Settings</button>
          </div>
        </section>

      {:else if activeTab === 'logging'}
        <section>
          <h3>Logging</h3>
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
      {/if}
    </div>
  </div>
{/if}

<style>
  .settings {
    max-width: 800px;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 30px;
  }

  h2 {
    font-size: 28px;
    font-weight: 600;
  }

  .tabs {
    display: flex;
    gap: 8px;
    margin-bottom: 30px;
    border-bottom: 1px solid #333;
    padding-bottom: 16px;
  }

  .tab {
    padding: 10px 20px;
    background: transparent;
    border: none;
    border-radius: 8px;
    color: #999;
    font-size: 14px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .tab:hover {
    background: #333;
    color: #fff;
  }

  .tab.active {
    background: #4fc1ff;
    color: #1a1a1a;
  }

  section {
    background: #252525;
    border-radius: 12px;
    padding: 24px;
    margin-bottom: 24px;
  }

  section h3 {
    font-size: 18px;
    margin-bottom: 20px;
    color: #4fc1ff;
  }

  .form-group {
    margin-bottom: 20px;
  }

  .form-group:last-child {
    margin-bottom: 0;
  }

  label,
  .label-text {
    display: block;
    font-size: 14px;
    font-weight: 500;
    margin-bottom: 8px;
    color: #e0e0e0;
  }

  .form-group.checkbox label {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
  }

  input[type="text"],
  input[type="password"],
  select {
    width: 100%;
    padding: 12px 16px;
    background: #333;
    border: 1px solid #444;
    border-radius: 8px;
    color: #e0e0e0;
    font-size: 14px;
  }

  input:focus,
  select:focus {
    outline: none;
    border-color: #4fc1ff;
  }

  .save-error {
    margin: 0 0 16px;
    padding: 12px 16px;
    background: rgba(255, 80, 80, 0.15);
    border: 1px solid #e05555;
    border-radius: 8px;
    color: #ff9999;
    font-size: 14px;
  }

  .hint {
    font-size: 12px;
    color: #666;
    margin-top: 6px;
  }

  .hint a {
    color: #4fc1ff;
    text-decoration: none;
  }

  .hint.warning {
    color: #ff9800;
  }

  .input-group {
    display: flex;
    gap: 8px;
  }

  .input-group input {
    flex: 1;
  }

  .language-multiselect .selected-languages {
    margin-bottom: 12px;
  }

  .language-multiselect .lang-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 12px;
    background: #333;
    border-radius: 8px;
    margin-bottom: 6px;
  }

  .language-multiselect .lang-badge {
    font-size: 14px;
    color: #e0e0e0;
  }

  .language-multiselect .default-tag {
    display: inline-block;
    background: #4fc1ff;
    color: #1a1a1a;
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
    margin-right: 6px;
  }

  .language-multiselect .lang-actions {
    display: flex;
    gap: 4px;
  }

  .language-multiselect .btn-icon {
    background: #444;
    border: none;
    color: #e0e0e0;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
  }

  .language-multiselect .btn-icon:hover {
    background: #555;
  }

  .language-multiselect .btn-icon.remove {
    color: #e74c3c;
  }

  .language-multiselect .lang-row.unsupported {
    opacity: 0.65;
  }

  .language-multiselect .lang-row.unsupported .lang-badge {
    color: #888;
  }

  .language-multiselect .unsupported-icon {
    margin-left: 6px;
    color: #ff9800;
    font-size: 12px;
    vertical-align: middle;
  }

  .language-multiselect .add-language select {
    width: 100%;
  }

  .validation {
    font-size: 12px;
    margin-top: 6px;
    display: block;
  }

  .validation.success {
    color: #4caf50;
  }

  .validation.error {
    color: #f44336;
  }

  .btn-primary {
    padding: 10px 20px;
    background: #4fc1ff;
    border: none;
    border-radius: 8px;
    color: #1a1a1a;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-primary:hover {
    background: #3ba8e6;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .save-status {
    font-size: 13px;
    color: #999;
  }

  .save-status.error {
    color: #e74c3c;
  }

  .btn-secondary {
    padding: 10px 20px;
    background: #333;
    border: 1px solid #444;
    border-radius: 8px;
    color: #e0e0e0;
    font-size: 14px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-secondary:hover {
    background: #444;
  }

  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-secondary.btn-link {
    color: #4fc1ff;
    border-color: transparent;
    background: transparent;
    text-decoration: underline;
  }

  .btn-secondary.btn-link:hover {
    background: rgba(79, 193, 255, 0.1);
  }

  .mic-level-container {
    margin-top: 10px;
  }

  .mic-level {
    height: 8px;
    background: #333;
    border-radius: 4px;
    overflow: hidden;
  }

  .mic-bar {
    height: 100%;
    background: linear-gradient(90deg, #4caf50, #8bc34a);
    border-radius: 4px;
    transition: width 0.1s ease-out;
    min-width: 2px;
  }

  .mic-status {
    font-size: 12px;
    color: #999;
    margin-top: 4px;
    display: block;
  }

  .btn-refresh {
    background: transparent;
    border: none;
    color: #4fc1ff;
    cursor: pointer;
    font-size: 14px;
    margin-left: 8px;
    padding: 2px 6px;
    border-radius: 4px;
    transition: background 0.2s;
  }

  .btn-refresh:hover {
    background: #333;
  }

  .model-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .model-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px;
    background: #333;
    border-radius: 8px;
  }

  .model-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .model-info span {
    font-size: 12px;
    color: #999;
  }

  .danger-zone {
    margin-top: 30px;
    padding: 20px;
    border: 1px solid #f44336;
    border-radius: 8px;
  }

  .danger-zone h4 {
    color: #f44336;
    margin-bottom: 16px;
  }

  .btn-danger {
    padding: 10px 20px;
    background: transparent;
    border: 1px solid #f44336;
    border-radius: 8px;
    color: #f44336;
    font-size: 14px;
    cursor: pointer;
    margin-right: 10px;
    transition: all 0.2s;
  }

  .btn-danger:hover {
    background: #f44336;
    color: #fff;
  }
  
  .badge {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 4px;
    margin-left: 8px;
    font-weight: 500;
  }
  
  .badge.configured {
    background: #4caf50;
    color: #fff;
  }
  
  .input-group .btn-danger {
    margin-left: 8px;
    border-color: #f44336;
    color: #f44336;
  }
  
  .input-group .btn-danger:hover {
    background: #f44336;
    color: #fff;
  }
  
  .hint.warning {
    color: #ff9800;
  }
</style>
