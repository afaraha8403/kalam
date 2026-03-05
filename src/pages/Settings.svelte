<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import type { AppConfig, AudioDevice } from '../types'
  import HotkeyCapture from '../components/HotkeyCapture.svelte'

  let config: AppConfig | null = null
  let audioDevices: AudioDevice[] = []
  let activeTab = 'general'
  let saving = false
  let micLevel = 0
  let testingMic = false
  let apiKeyValid: boolean | null = null
  let hasApiKey = false
  let apiKeyInput = ''

  const tabs = [
    { id: 'general', label: 'General' },
    { id: 'audio', label: 'Audio' },
    { id: 'stt', label: 'STT Provider' },
    { id: 'formatting', label: 'Formatting' },
    { id: 'privacy', label: 'Privacy' },
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
      
      // Check if API key is already configured
      hasApiKey = !!config?.stt_config?.api_key
      // Don't populate the input with the actual key for security
      apiKeyInput = ''
      
      console.log('Loaded audio devices:', devices)
    } catch (e) {
      console.error('Failed to load settings:', e)
    }
  })

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
    
    console.log('Config object:', JSON.stringify(config, null, 2))
    console.log('Saving config with api_key:', config.stt_config.api_key ? 'present' : 'missing')
    try {
      const result = await invoke('save_settings', { newConfig: config })
      console.log('Settings saved successfully, result:', result)
      // Update hasApiKey status after successful save
      hasApiKey = !!config.stt_config.api_key
      apiKeyInput = '' // Clear input after save
    } catch (e) {
      console.error('Failed to save settings:', e)
    } finally {
      saving = false
    }
  }

  async function testMicrophone() {
    testingMic = true
    micLevel = 0 // Reset level before test
    try {
      const level = await invoke('test_microphone') as number
      micLevel = level
      console.log('Microphone test result:', level)
    } catch (e) {
      console.error('Microphone test failed:', e)
      micLevel = 0
    } finally {
      testingMic = false
      // Clear the level after a few seconds
      setTimeout(() => {
        micLevel = 0
      }, 3000)
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
</script>

{#if config}
  <div class="settings">
    <header>
      <h2>Settings</h2>
      <button class="btn-primary" on:click={saveSettings} disabled={saving}>
        {saving ? 'Saving...' : 'Save Changes'}
      </button>
    </header>

    <div class="tabs">
      {#each tabs as tab}
        <button
          class="tab"
          class:active={activeTab === tab.id}
          on:click={() => activeTab = tab.id}
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
              onChange={(hotkey) => {
                if (config) {
                  config.hotkey = hotkey
                  config = config // trigger reactivity
                }
              }}
            />
          </div>

          <div class="form-group">
            <label for="recording-mode">Recording Mode</label>
            <select id="recording-mode" bind:value={config.recording_mode}>
              <option value="Hold">Hold to record</option>
              <option value="Toggle">Toggle mode</option>
            </select>
            {#if config.recording_mode === 'Hold'}
              <p class="hint">Press and hold the hotkey to record, release to stop</p>
            {:else}
              <p class="hint">Press hotkey once to start, press again to stop</p>
            {/if}
          </div>

          <div class="form-group">
            <label for="language">Language</label>
            <select id="language" bind:value={config.language}>
              <option value="auto">Auto-detect</option>
              <option value="en">English</option>
              <option value="es">Spanish</option>
              <option value="fr">French</option>
              <option value="de">German</option>
              <option value="zh">Chinese</option>
            </select>
          </div>

          <div class="form-group checkbox">
            <label>
              <input type="checkbox" bind:checked={config.auto_start} />
              Start on login
            </label>
          </div>

          <div class="form-group checkbox">
            <label>
              <input type="checkbox" bind:checked={config.start_in_focus} />
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
            <select id="microphone" bind:value={config.audio_device}>
              <option value={null}>Default device</option>
              {#each audioDevices.filter(d => d.id !== 'default') as device}
                <option value={device.id}>{device.name}</option>
              {/each}
            </select>
            {#if audioDevices.length === 0}
              <p class="hint warning">No audio devices found. Try refreshing the list.</p>
            {:else}
              <p class="hint">{audioDevices.length} audio device(s) available</p>
            {/if}
          </div>

          <div class="form-group">
            <span class="label-text">Test Microphone</span>
            <button class="btn-secondary" on:click={testMicrophone} disabled={testingMic}>
              {testingMic ? 'Testing...' : 'Test'}
            </button>
            <div class="mic-level-container">
              <div class="mic-level">
                <div class="mic-bar" style="width: {micLevel * 100}%"></div>
              </div>
              {#if testingMic}
                <span class="mic-status">Recording...</span>
              {:else if micLevel > 0}
                <span class="mic-status">Level: {Math.round(micLevel * 100)}%</span>
              {/if}
            </div>
          </div>

          <div class="form-group">
            <label for="vad-preset">VAD Sensitivity</label>
            <select id="vad-preset" bind:value={config.stt_config.vad_preset}>
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
            <select id="stt-mode" bind:value={config.stt_config.mode}>
              <option value="Cloud">Cloud (Groq API)</option>
              <option value="Local">Local (SenseVoice)</option>
              <option value="Hybrid">Hybrid (Auto-switch)</option>
            </select>
          </div>

          {#if config.stt_config.mode === 'Cloud' || config.stt_config.mode === 'Hybrid'}
            <div class="form-group">
              <label for="cloud-provider">Cloud Provider</label>
              <select id="cloud-provider" bind:value={config.stt_config.provider}>
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
        </section>

      {:else if activeTab === 'formatting'}
        <section>
          <h3>Text Formatting</h3>
          <div class="form-group checkbox">
            <label>
              <input type="checkbox" bind:checked={config.formatting.voice_commands} />
              Enable voice commands ("period", "new line", etc.)
            </label>
          </div>

          <div class="form-group checkbox">
            <label>
              <input type="checkbox" bind:checked={config.formatting.filler_word_removal} />
              Remove filler words ("um", "uh", "like")
            </label>
          </div>

          <div class="form-group checkbox">
            <label>
              <input type="checkbox" bind:checked={config.formatting.auto_punctuation} />
              Auto-punctuation
            </label>
          </div>

          <div class="form-group">
            <label for="injection-method">Text Injection Method</label>
            <select id="injection-method" bind:value={config.formatting.injection_method}>
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
            <select id="history-retention" bind:value={config.privacy.history_retention_days}>
              <option value={7}>7 days</option>
              <option value={30}>30 days</option>
              <option value={90}>90 days</option>
              <option value={365}>1 year</option>
              <option value={0}>Forever</option>
            </select>
          </div>

          <div class="form-group checkbox">
            <label>
              <input type="checkbox" bind:checked={config.privacy.sensitive_app_detection} />
              Auto-switch to local mode in sensitive apps (password managers, etc.)
            </label>
          </div>

          <div class="form-group checkbox">
            <label>
              <input type="checkbox" bind:checked={config.privacy.telemetry_enabled} />
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
