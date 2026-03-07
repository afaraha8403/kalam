<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import HotkeyCapture from '../components/HotkeyCapture.svelte'
  import { LANGUAGE_OPTIONS, languageLabel } from '../lib/languages'
  import type { AppConfig } from '../types'

  const dispatch = createEventDispatcher<{ complete: void }>()

  let step = 1
  const totalSteps = 5
  let testingMic = false
  let micLevel = 0
  let levelPollId: ReturnType<typeof setInterval> | null = null
  let noAudioRecorded = false
  let audioCtx: AudioContext | null = null
  let apiKey = ''
  let selectedMode: 'Cloud' | 'Hybrid' | 'Local' = 'Hybrid'
  let hotkey = 'Ctrl+Win'
  let languages: string[] = ['en']
  let addLanguageCode = ''
  let recordingMode: 'Hold' | 'Toggle' = 'Hold'
  let platform: 'windows' | 'darwin' | 'linux' = 'windows'
  let demoTranscription = ''
  let unlistenDictation: (() => void) | null = null

  onMount(() => {
    const setup = async () => {
      const unlisten = await listen<string>('dictation-result', (e) => {
        if (typeof e.payload === 'string') {
          demoTranscription = e.payload
        }
      })
      unlistenDictation = unlisten
    }
    setup()
    return () => {
      unlistenDictation?.()
    }
  })

  async function loadConfig() {
    try {
      const config = (await invoke('get_settings')) as AppConfig
      if (config.stt_config?.api_key) apiKey = config.stt_config.api_key
      if (config.stt_config?.mode) selectedMode = config.stt_config.mode as 'Cloud' | 'Hybrid' | 'Local'
      if (config.recording_mode) recordingMode = config.recording_mode
      if (config.hotkey) {
        hotkey = platform === 'windows' && config.hotkey === 'Ctrl+Super' ? 'Ctrl+Win' : config.hotkey
      } else {
        hotkey = platform === 'windows' ? 'Ctrl+Win' : 'Ctrl+Super'
      }
      if (config.languages?.length) languages = [...config.languages]
    } catch (e) {
      console.error('Onboarding load config failed:', e)
    }
  }

  async function saveOnboardingState() {
    try {
      const config = (await invoke('get_settings')) as AppConfig
      if (apiKey) config.stt_config.api_key = apiKey
      config.stt_config.mode = selectedMode
      config.hotkey = hotkey
      config.languages = languages.length ? [...languages] : ['en']
      config.recording_mode = recordingMode
      await invoke('save_settings', { newConfig: config })
    } catch (e) {
      console.error('Onboarding save state failed:', e)
    }
  }

  async function nextStep() {
    if (step === 4) await saveOnboardingState()
    if (step < totalSteps) step++
  }

  function prevStep() {
    if (step > 1) step--
  }

  function moveLanguageUp(index: number) {
    if (index <= 0) return
    const s = [...languages]
    ;[s[index - 1], s[index]] = [s[index], s[index - 1]]
    languages = s
  }

  function moveLanguageDown(index: number) {
    if (index >= languages.length - 1) return
    const s = [...languages]
    ;[s[index], s[index + 1]] = [s[index + 1], s[index]]
    languages = s
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

  async function testMic() {
    if (testingMic) {
      if (levelPollId != null) {
        clearInterval(levelPollId)
        levelPollId = null
      }
      noAudioRecorded = false
      try {
        const result = (await invoke('test_microphone_stop')) as {
          level: number
          samples: number[]
          sample_rate: number
        }
        if (result.samples?.length && result.sample_rate) {
          await playTestAudio(result.samples, result.sample_rate)
        } else {
          noAudioRecorded = true
        }
      } catch (e) {
        console.error(e)
      }
      testingMic = false
      return
    }
    noAudioRecorded = false
    testingMic = true
    micLevel = 0
    try {
      audioCtx = new (window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext)()
      await audioCtx.resume()
      await invoke('test_microphone_start')
      levelPollId = setInterval(async () => {
        try {
          micLevel = (await invoke('test_microphone_level')) as number
        } catch {
          // ignore
        }
      }, 100)
    } catch (e) {
      console.error('Microphone test start failed:', e)
      testingMic = false
      if (levelPollId != null) {
        clearInterval(levelPollId)
        levelPollId = null
      }
    }
  }

  async function loadPlatform() {
    try {
      const os = (await invoke('get_platform')) as string
      if (os === 'darwin' || os === 'linux') platform = os
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

  onMount(() => {
    loadPlatform().then(() => loadConfig())
  })

  async function finish() {
    try {
      const config = (await invoke('get_settings')) as AppConfig
      config.onboarding_complete = true
      if (apiKey) config.stt_config.api_key = apiKey
      config.stt_config.mode = selectedMode
      config.hotkey = hotkey
      config.languages = languages.length ? [...languages] : ['en']
      config.recording_mode = recordingMode
      await invoke('save_settings', { newConfig: config })
      dispatch('complete')
    } catch (e) {
      console.error('Onboarding save failed:', e)
    }
  }
</script>

<div class="onboarding">
  <div class="progress">
    {#each Array(totalSteps) as _, i}
      <div class="dot" class:active={i + 1 === step}></div>
    {/each}
  </div>

  {#if step === 1}
    <div class="step">
      <h1>Welcome to Kalam Voice</h1>
      <p class="subtitle">Speak your thoughts freely — fast, private, and accessible anywhere.</p>
      
      <div class="features">
        <div class="feature">
          <span class="icon">⚡</span>
          <div>
            <h3>4x Faster</h3>
            <p>Than typing</p>
          </div>
        </div>
        <div class="feature">
          <span class="icon">🔒</span>
          <div>
            <h3>Privacy First</h3>
            <p>Local mode available</p>
          </div>
        </div>
        <div class="feature">
          <span class="icon">🌍</span>
          <div>
            <h3>Cross-Platform</h3>
            <p>Windows, macOS, Linux</p>
          </div>
        </div>
      </div>
    </div>

  {:else if step === 2}
    <div class="step">
      <h1>Permissions</h1>
      <p class="subtitle">Kalam needs a few permissions to work properly</p>
      
      <div class="permissions">
        <div class="permission permission-card">
          <span class="icon">🎤</span>
          <div class="permission-body">
            <h3>Microphone Access</h3>
            <p>To capture your voice</p>
            {#if platform === 'windows'}
              <p class="permission-path">Settings → Privacy & security → Microphone. Turn on <strong>Microphone access</strong> and <strong>Let desktop apps access your microphone</strong>.</p>
              <button type="button" class="btn-link" on:click={() => requestPermission('microphone')}>Open Microphone settings</button>
            {:else if platform === 'darwin'}
              <p class="permission-path">System Settings → Privacy & Security → Microphone. Add Kalam and turn it on. You may also see a system prompt the first time you use the mic.</p>
              <button type="button" class="btn-link" on:click={() => requestPermission('microphone')}>Open Microphone settings</button>
            {:else}
              <p class="permission-path">Usually allowed. On GNOME: Settings → Privacy → Microphone. Otherwise check your system’s sound or privacy settings.</p>
            {/if}
          </div>
        </div>
        <div class="permission permission-card">
          <span class="icon">⌨️</span>
          <div class="permission-body">
            <h3>Accessibility</h3>
            <p>To type text into any app</p>
            {#if platform === 'windows'}
              <p class="permission-path">On Windows, no extra permission is needed for Kalam to type into apps. If typing doesn’t work in some apps (for example, ones running as administrator), run Kalam the same way or use paste where the app supports it.</p>
              <button type="button" class="btn-link" on:click={() => requestPermission('accessibility')}>Open Settings → Accessibility</button>
            {:else if platform === 'darwin'}
              <p class="permission-path">Click below to show the system “Allow Kalam to control this computer?” dialog, or open System Settings → Privacy & Security → Accessibility and add Kalam.</p>
              <button type="button" class="btn-primary btn-small" on:click={() => requestPermission('accessibility')}>Request accessibility permission</button>
              <button type="button" class="btn-link" on:click={() => openPermissionPage('accessibility')}>Open Accessibility settings</button>
            {:else}
              <p class="permission-path">If your desktop has accessibility or input settings, allow Kalam to use assistive technology or input injection.</p>
            {/if}
          </div>
        </div>
      </div>

      <div class="test-mic">
        <p>Test your microphone:</p>
        <button class="btn-primary" on:click={testMic}>
          {testingMic ? 'Stop Test' : 'Start Test'}
        </button>
        {#if testingMic}
          <div class="mic-level-wrap">
            <div
              class="mic-level"
              role="meter"
              aria-label="Microphone level"
              aria-valuenow={Math.round(micLevel * 100)}
              aria-valuemin={0}
              aria-valuemax={100}
              style="width: {Math.min(100, Math.round(micLevel * 100))}%"
            ></div>
          </div>
          <p class="mic-hint">Speak into your microphone. Click Stop Test to hear playback of what was recorded.</p>
        {:else if noAudioRecorded}
          <p class="mic-empty">No audio recorded. Speak a bit longer, then stop.</p>
        {/if}
      </div>
    </div>

  {:else if step === 3}
    <div class="step">
      <h1>Choose Your Mode</h1>
      <p class="subtitle">How would you like to transcribe?</p>
      
      <div class="modes">
        <div class="mode-card" class:selected={selectedMode === 'Cloud'}>
          <h3>☁️ Cloud Mode</h3>
          <p>Fastest transcription using Groq API</p>
          <ul>
            <li>~300ms latency</li>
            <li>99 languages</li>
            <li>Requires internet</li>
          </ul>
          {#if selectedMode === 'Cloud'}
            <input type="text" placeholder="Enter Groq API key" bind:value={apiKey} />
            <a href="https://console.groq.com" target="_blank">Get API key →</a>
          {/if}
          <button class="btn-primary" on:click={() => (selectedMode = 'Cloud')}>Select Cloud</button>
        </div>
        
        <div class="mode-card recommended" class:selected={selectedMode === 'Hybrid'}>
          <div class="badge">Recommended</div>
          <h3>🔄 Hybrid Mode</h3>
          <p>Best of both worlds</p>
          <ul>
            <li>Cloud when available</li>
            <li>Local when offline</li>
            <li>Auto-switching</li>
          </ul>
          {#if selectedMode === 'Hybrid'}
            <input type="text" placeholder="Enter Groq API key (for cloud)" bind:value={apiKey} />
            <a href="https://console.groq.com" target="_blank">Get API key →</a>
          {/if}
          <button class="btn-primary" on:click={() => (selectedMode = 'Hybrid')}>Select Hybrid</button>
        </div>
        
        <div class="mode-card" class:selected={selectedMode === 'Local'}>
          <h3>💻 Local Mode</h3>
          <p>100% private, works offline</p>
          <ul>
            <li>No internet needed</li>
            <li>50+ languages</li>
            <li>Download required</li>
          </ul>
          <button class="btn-primary" on:click={() => (selectedMode = 'Local')}>Select Local</button>
        </div>
      </div>
    </div>

  {:else if step === 4}
    <div class="step">
      <h1>Set Your Hotkey</h1>
      <p class="subtitle">Choose how you'll activate dictation</p>
      
      <div class="hotkey-config">
        <div class="form-group language-multiselect">
          <label>Recognition languages</label>
          <p class="hint">First language is the default. Add more to toggle between them with the hotkey.</p>
          <div class="selected-languages">
            {#each languages as code, i}
              <div class="lang-row">
                <span class="lang-badge">{#if i === 0}<span class="default-tag">Default</span> {/if}{languageLabel(code)}</span>
                <div class="lang-actions">
                  {#if i > 0}
                    <button type="button" class="btn-icon" title="Move up" on:click={() => moveLanguageUp(i)}>↑</button>
                  {/if}
                  {#if i < languages.length - 1}
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
              on:change={() => addLanguage(addLanguageCode)}
            >
              <option value="">Add a language…</option>
              {#each LANGUAGE_OPTIONS as opt}
                <option value={opt.code} disabled={languages.includes(opt.code)}>{opt.label}</option>
              {/each}
            </select>
          </div>
        </div>
        <HotkeyCapture
          value={hotkey}
          onChange={(h) => (hotkey = h)}
        />
        <p class="hint">Press your desired key combination</p>
        
        <div class="mode-select">
          <label>
            <input type="radio" name="mode" value="Hold" bind:group={recordingMode} />
            Hold to record (release to stop)
          </label>
          <label>
            <input type="radio" name="mode" value="Toggle" bind:group={recordingMode} />
            Toggle mode (press to start/stop)
          </label>
        </div>
      </div>
    </div>

  {:else if step === 5}
    <div class="step">
      <h1>You're Ready!</h1>
      <p class="subtitle">Try it out now</p>
      
      <div class="demo">
        <p>Press <kbd>{hotkey || 'Ctrl+Win'}</kbd> and say:</p>
        <blockquote>"Hello, this is a test of Kalam Voice!"</blockquote>
        <textarea placeholder="Your text will appear here when you dictate..." readonly value={demoTranscription}></textarea>
      </div>
    </div>
  {/if}

  <div class="actions">
    {#if step > 1}
      <button class="btn-secondary" on:click={prevStep}>Back</button>
    {/if}
    
    {#if step < totalSteps}
      <button class="btn-primary" on:click={nextStep}>Next</button>
    {:else}
      <button class="btn-primary" on:click={finish}>Get Started</button>
    {/if}
  </div>
</div>

<style>
  .onboarding {
    position: fixed;
    inset: 0;
    background: #1a1a1a;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px;
  }

  .progress {
    display: flex;
    gap: 8px;
    margin-bottom: 40px;
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #333;
    transition: background 0.3s;
  }

  .dot.active {
    background: #4fc1ff;
  }

  .step {
    text-align: center;
    max-width: 800px;
  }

  h1 {
    font-size: 36px;
    margin-bottom: 12px;
  }

  .subtitle {
    font-size: 18px;
    color: #666;
    margin-bottom: 40px;
  }

  .features {
    display: flex;
    justify-content: center;
    gap: 40px;
  }

  .feature {
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
  }

  .feature .icon {
    font-size: 32px;
  }

  .feature h3 {
    font-size: 16px;
    margin-bottom: 4px;
  }

  .feature p {
    font-size: 14px;
    color: #666;
  }

  .permissions {
    display: flex;
    justify-content: center;
    gap: 40px;
    margin-bottom: 40px;
  }

  .permission {
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
    padding: 20px;
    background: #252525;
    border-radius: 12px;
  }

  .permission.permission-card {
    flex-direction: column;
    align-items: flex-start;
    min-width: 280px;
  }

  .permission-body {
    width: 100%;
  }

  .permission-body h3 {
    margin-bottom: 4px;
  }

  .permission-body p {
    margin-bottom: 6px;
  }

  .permission-path {
    font-size: 13px;
    color: #888;
    margin-top: 4px;
  }

  .btn-link {
    background: none;
    border: none;
    color: #4fc1ff;
    cursor: pointer;
    font-size: 14px;
    padding: 4px 0;
    margin-top: 4px;
    text-decoration: underline;
  }

  .btn-link:hover {
    color: #7ad4ff;
  }

  .btn-small {
    padding: 8px 16px;
    font-size: 14px;
    margin-right: 12px;
  }

  .permission .icon {
    font-size: 32px;
  }

  .mic-empty {
    font-size: 13px;
    color: #e67e22;
    margin: 8px 0 0;
  }

  .test-mic {
    margin-top: 24px;
  }

  .test-mic .btn-primary {
    margin-bottom: 12px;
  }

  .mic-level-wrap {
    width: 200px;
    height: 8px;
    background: #333;
    border-radius: 4px;
    overflow: hidden;
    margin: 0 auto 8px;
  }

  .mic-level {
    height: 100%;
    background: #4fc1ff;
    border-radius: 4px;
    transition: width 0.1s;
  }

  .mic-hint {
    font-size: 13px;
    color: #666;
    margin: 0;
  }

  .modes {
    display: flex;
    gap: 20px;
    justify-content: center;
  }

  .mode-card {
    width: 240px;
    padding: 24px;
    background: #252525;
    border-radius: 12px;
    text-align: left;
    position: relative;
  }

  .mode-card.recommended {
    border: 2px solid #4fc1ff;
  }

  .mode-card.selected {
    border: 2px solid #4caf50;
    box-shadow: 0 0 0 1px #4caf50;
  }

  .badge {
    position: absolute;
    top: -10px;
    left: 50%;
    transform: translateX(-50%);
    background: #4fc1ff;
    color: #1a1a1a;
    padding: 4px 12px;
    border-radius: 20px;
    font-size: 12px;
    font-weight: 600;
  }

  .mode-card h3 {
    margin-bottom: 8px;
  }

  .mode-card p {
    color: #666;
    font-size: 14px;
    margin-bottom: 16px;
  }

  .mode-card ul {
    list-style: none;
    margin-bottom: 20px;
  }

  .mode-card li {
    padding: 4px 0;
    color: #999;
    font-size: 14px;
  }

  .mode-card li::before {
    content: "✓ ";
    color: #4caf50;
  }

  .mode-card input {
    width: 100%;
    padding: 10px;
    background: #333;
    border: 1px solid #444;
    border-radius: 6px;
    color: #e0e0e0;
    margin-bottom: 8px;
  }

  .mode-card a {
    color: #4fc1ff;
    font-size: 12px;
    text-decoration: none;
  }

  .hotkey-config {
    background: #252525;
    padding: 40px;
    border-radius: 12px;
  }

  .hotkey-config .form-group {
    margin-bottom: 20px;
  }

  .hotkey-config .form-group label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    margin-bottom: 8px;
    color: #e0e0e0;
  }

  .hotkey-config .form-group select {
    width: 100%;
    padding: 12px 16px;
    background: #333;
    border: 1px solid #444;
    border-radius: 8px;
    color: #e0e0e0;
    font-size: 14px;
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

  .language-multiselect .add-language select {
    width: 100%;
  }

  .hotkey-display {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
  }

  .hotkey-display kbd {
    background: #333;
    padding: 12px 20px;
    border-radius: 8px;
    font-family: monospace;
    font-size: 18px;
  }

  .mode-select {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 24px;
  }

  .mode-select label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .demo blockquote {
    font-size: 20px;
    font-style: italic;
    color: #999;
    margin: 20px 0;
  }

  .demo textarea {
    width: 100%;
    height: 100px;
    padding: 16px;
    background: #252525;
    border: 1px solid #333;
    border-radius: 8px;
    color: #e0e0e0;
    resize: none;
  }

  .actions {
    display: flex;
    gap: 16px;
    margin-top: 40px;
  }

  .btn-primary {
    padding: 14px 32px;
    background: #4fc1ff;
    border: none;
    border-radius: 8px;
    color: #1a1a1a;
    font-size: 16px;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-secondary {
    padding: 14px 32px;
    background: transparent;
    border: 1px solid #444;
    border-radius: 8px;
    color: #e0e0e0;
    font-size: 16px;
    cursor: pointer;
  }
</style>
