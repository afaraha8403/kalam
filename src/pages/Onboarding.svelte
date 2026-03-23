<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { onMount } from 'svelte'
  import { invoke, listenSafe } from '$lib/backend'
  import HotkeyCapture from '../components/HotkeyCapture.svelte'
  import { LANGUAGE_OPTIONS, languageLabel } from '../lib/languages'
  import type { AppConfig } from '../types'

  const dispatch = createEventDispatcher<{ complete: void }>()

  let step = 1
  const totalSteps = 6
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
  let selectedProvider: 'groq' | 'openai' = 'groq'
  let selectedMode: 'Cloud' | 'Hybrid' | 'Local' = 'Hybrid'
  let hotkey = 'Ctrl+Win'
  let toggleHotkey = ''
  let addLanguageCode = ''
  let languages: string[] = ['en']
  let platform: 'windows' | 'darwin' | 'linux' = 'windows'
  let demoTranscription = ''
  let demoTextarea: HTMLTextAreaElement
  let demoFocused = false
  let unlistenDictation: (() => void) | null = null
  let skipInProgress = false
  let skipError = ''

  const stepLabels = [
    'Welcome',
    'Account',
    'Permissions',
    'Mode',
    'Controls',
    'Ready',
  ]

  onMount(() => {
    const setup = async () => {
      const unlisten = await listenSafe<string>('dictation-result', (e) => {
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

  function isEmailValid(email: string) {
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test((email || '').trim())
  }

  async function loadConfig() {
    try {
      const config = (await invoke('get_settings')) as AppConfig
      if (config.stt_config?.provider) selectedProvider = config.stt_config.provider as 'groq' | 'openai'
      if (config.stt_config?.mode) selectedMode = config.stt_config.mode as 'Cloud' | 'Hybrid' | 'Local'
      if (!config.stt_config?.api_keys) config.stt_config.api_keys = {}
      if (config.stt_config?.api_key && !config.stt_config.api_keys[selectedProvider]) {
        config.stt_config.api_keys[selectedProvider] = config.stt_config.api_key
      }
      if (config.stt_config?.api_keys?.[selectedProvider]) apiKey = config.stt_config.api_keys[selectedProvider]
      if (config.hotkey) {
        hotkey = platform === 'windows' && config.hotkey === 'Ctrl+Super' ? 'Ctrl+Win' : config.hotkey
      } else {
        hotkey = platform === 'windows' ? 'Ctrl+Win' : 'Ctrl+Super'
      }
      if (config.toggle_dictation_hotkey) {
        toggleHotkey = config.toggle_dictation_hotkey
      }
      if (config.languages?.length) languages = [...config.languages]
      if (config.user_email) userEmail = config.user_email
      if (config.notifications_opt_in != null) notificationsOptIn = config.notifications_opt_in
    } catch (e) {
      console.error('Onboarding load config failed:', e)
    }
  }

  async function saveOnboardingState() {
    try {
      const config = (await invoke('get_settings')) as AppConfig
      config.stt_config.provider = selectedProvider
      if (!config.stt_config.api_keys) config.stt_config.api_keys = {}
      if (apiKey) config.stt_config.api_keys[selectedProvider] = apiKey
      config.stt_config.mode = selectedMode
      config.hotkey = hotkey
      config.toggle_dictation_hotkey = toggleHotkey || null
      config.languages = languages.length ? [...languages] : ['en']
      config.user_email = userEmail.trim() || null
      config.marketing_opt_in = false
      config.notifications_opt_in = notificationsOptIn
      await invoke('save_settings', { newConfig: config })
    } catch (e) {
      console.error('Onboarding save state failed:', e)
    }
  }

  async function nextStep() {
    if (step === 2) {
      if (!termsAgreed || !isEmailValid(userEmail)) return
      await saveOnboardingState()
    }
    if (step === 5) await saveOnboardingState()
    if (step < totalSteps) step++
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
    noAudioRecorded = false
    hasRecording = false
    recordedSamples = []
    recordedSampleRate = 0
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
      config.stt_config.provider = selectedProvider
      if (!config.stt_config.api_keys) config.stt_config.api_keys = {}
      if (apiKey) config.stt_config.api_keys[selectedProvider] = apiKey
      config.stt_config.mode = selectedMode
      config.hotkey = hotkey
      config.toggle_dictation_hotkey = toggleHotkey || null
      config.languages = languages.length ? [...languages] : ['en']
      config.user_email = userEmail.trim() || null
      config.marketing_opt_in = false
      config.notifications_opt_in = notificationsOptIn
      await invoke('save_settings', { newConfig: config })
      dispatch('complete')
    } catch (e) {
      console.error('Onboarding save failed:', e)
    }
  }

  $: if (step === 6 && demoTextarea) {
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
          {skipInProgress ? 'Skipping...' : 'Skip setup'}
        </button>
        {#if skipError}
          <p class="skip-error" role="alert">{skipError}</p>
        {/if}
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

      {#if step === 1}
        <div class="step step-welcome">
          <h1>Welcome to Kalam</h1>
          <p class="subtitle">Voice-to-text that's fast, private, and works everywhere.</p>

          <div class="features-grid">
            <div class="feat">
              <span class="feat-icon">⚡</span>
              <div>
                <strong>Lightning Fast</strong>
                <span>Speak 4x faster than typing</span>
              </div>
            </div>
            <div class="feat">
              <span class="feat-icon">🔒</span>
              <div>
                <strong>Private</strong>
                <span>Runs locally on your machine</span>
              </div>
            </div>
            <div class="feat">
              <span class="feat-icon">🎯</span>
              <div>
                <strong>Universal</strong>
                <span>Dictate into any app or text field</span>
              </div>
            </div>
            <div class="feat">
              <span class="feat-icon">🌐</span>
              <div>
                <strong>99+ Languages</strong>
                <span>Auto-punctuation built in</span>
              </div>
            </div>
          </div>
        </div>

      {:else if step === 2}
        <div class="step step-account">
          <h1>Create your account</h1>
          <p class="subtitle">So we can reach you for support and important updates.</p>

          <div class="form-card">
            <div class="field">
              <label for="onboarding-email">Email <span class="req">*</span></label>
              <input
                id="onboarding-email"
                type="email"
                placeholder="you@example.com"
                bind:value={userEmail}
              />
              {#if userEmail && !isEmailValid(userEmail)}
                <p class="field-error">Enter a valid email address.</p>
              {/if}
              <p class="privacy-note">Your email stays private and is never shared with third parties.</p>
            </div>
            <div class="checkboxes">
              <label class="check-row">
                <input type="checkbox" bind:checked={termsAgreed} />
                <span>I agree to the <a href="https://kalam.stream/terms.html" target="_blank" rel="noopener noreferrer">Terms and Conditions</a> and <a href="https://kalam.stream/privacy.html" target="_blank" rel="noopener noreferrer">Privacy Policy</a></span>
              </label>
              <label class="check-row">
                <input type="checkbox" bind:checked={notificationsOptIn} />
                <span>Send me product updates</span>
              </label>
            </div>
          </div>
        </div>

      {:else if step === 3}
        <div class="step step-permissions">
          <h1>Permissions</h1>
          <p class="subtitle">Kalam needs mic access to hear you, and accessibility to type for you.</p>

          <div class="perm-trust">
            <p>Kalam runs on your device. We use the <strong>microphone</strong> only for your voice to transcribe—we don't store raw audio. <strong>Accessibility</strong> lets us type the transcribed text into other apps. On macOS, the system may also ask for <strong>Input Monitoring</strong> (keyboard): we use it only to detect your dictation hotkey in any app; we do not record or send your keystrokes.</p>
          </div>

          <div class="perm-list">
            <div class="perm-row">
              <div class="perm-icon-wrap"><span class="perm-icon">🎤</span></div>
              <div class="perm-info">
                <strong>Microphone</strong>
                <span>Captures your voice for transcription</span>
              </div>
              <button type="button" class="btn-outline-sm" on:click={() => requestPermission('microphone')}>
                {platform === 'darwin' ? 'Allow' : 'Open Settings'}
              </button>
            </div>
            <div class="perm-row">
              <div class="perm-icon-wrap"><span class="perm-icon">⌨️</span></div>
              <div class="perm-info">
                <strong>Accessibility</strong>
                <span>Types transcribed text into apps</span>
              </div>
              {#if platform === 'darwin'}
                <button type="button" class="btn-outline-sm" on:click={() => requestPermission('accessibility')}>Allow</button>
              {:else if platform === 'windows'}
                <span class="perm-auto">Auto-enabled</span>
              {:else}
                <button type="button" class="btn-outline-sm" on:click={() => openPermissionPage('accessibility')}>Open Settings</button>
              {/if}
            </div>
            {#if platform === 'darwin'}
            <div class="perm-row">
              <div class="perm-icon-wrap"><span class="perm-icon">⌨️</span></div>
              <div class="perm-info">
                <strong>Input Monitoring</strong>
                <span>Lets your dictation hotkey work in any app. We only detect the hotkey—we don't record keystrokes.</span>
              </div>
              <span class="perm-hint">You'll be prompted when you first use the hotkey</span>
            </div>
            {/if}
          </div>

          <div class="mic-test">
            <h3 class="mic-test-title">Test your microphone</h3>
            <div class="mic-test-body">
              <div class="mic-controls">
                {#if testingMic}
                  <button class="mic-action-btn recording" on:click={stopRecording}>
                    <div class="stop-sq"></div>
                    <span>Stop</span>
                  </button>
                  <div class="mic-level-bar">
                    <div class="mic-level-fill" style="width: {Math.min(micLevel * 100, 100)}%"></div>
                  </div>
                {:else}
                  <button class="mic-action-btn" on:click={startRecording}>
                    <span class="mic-action-icon">🎤</span>
                    <span>Record</span>
                  </button>
                  {#if hasRecording}
                    <button class="mic-action-btn play" class:playing={isPlaying} on:click={playRecording} disabled={isPlaying}>
                      <span class="mic-action-icon">{isPlaying ? '🔊' : '▶️'}</span>
                      <span>{isPlaying ? 'Playing...' : 'Play back'}</span>
                    </button>
                  {/if}
                {/if}
              </div>
              <p class="mic-status">
                {#if testingMic}
                  Speak now — tap <strong>Stop</strong> when done.
                {:else if noAudioRecorded}
                  <span class="error-text">No audio detected. Check your mic is unmuted.</span>
                {:else if hasRecording}
                  Recording captured. Tap <strong>Play back</strong> to listen.
                {:else}
                  Tap <strong>Record</strong>, say a few words, then stop to hear playback.
                {/if}
              </p>
            </div>
          </div>
        </div>

      {:else if step === 4}
        <div class="step step-mode">
          <h1>Transcription mode</h1>
          <p class="subtitle">You can change this anytime in Settings.</p>

          <div class="mode-pills">
            <button
              class="mode-pill" class:active={selectedMode === 'Cloud'}
              on:click={() => (selectedMode = 'Cloud')}
            >
              <span class="mp-icon">☁️</span> Cloud
            </button>
            <button
              class="mode-pill recommended" class:active={selectedMode === 'Hybrid'}
              on:click={() => (selectedMode = 'Hybrid')}
            >
              <span class="mp-icon">🔄</span> Hybrid
              <span class="rec-dot"></span>
            </button>
            <button
              class="mode-pill" class:active={selectedMode === 'Local'}
              on:click={() => (selectedMode = 'Local')}
            >
              <span class="mp-icon">💻</span> Local
            </button>
          </div>

          <div class="mode-detail">
            {#if selectedMode === 'Cloud'}
              <div class="mode-desc">
                <div class="mode-stats">
                  <span class="stat">~300ms latency</span>
                  <span class="stat">99 languages</span>
                  <span class="stat">Requires internet</span>
                </div>
                <p>Fastest transcription using a cloud provider.</p>
              </div>
            {:else if selectedMode === 'Hybrid'}
              <div class="mode-desc">
                <div class="mode-stats">
                  <span class="stat rec">Recommended</span>
                  <span class="stat">Cloud + local fallback</span>
                  <span class="stat">Auto-switches</span>
                </div>
                <p>Uses cloud when online, falls back to local when offline. You can configure the local model later in Settings.</p>
              </div>
            {:else}
              <div class="mode-desc">
                <div class="mode-stats">
                  <span class="stat">100% offline</span>
                  <span class="stat">50+ languages</span>
                  <span class="stat">No data leaves your machine</span>
                </div>
                <p class="local-hint">You can configure the local model and download it from <strong>Settings</strong> after setup.</p>
              </div>
            {/if}

            {#if selectedMode === 'Cloud' || selectedMode === 'Hybrid'}
              <div class="provider-section">
                <div class="provider-toggle">
                  <span class="provider-label">Cloud provider</span>
                  <div class="provider-pills">
                    <button
                      class="prov-pill" class:active={selectedProvider === 'groq'}
                      on:click={() => (selectedProvider = 'groq')}
                    >Groq</button>
                    <button
                      class="prov-pill" class:active={selectedProvider === 'openai'}
                      on:click={() => (selectedProvider = 'openai')}
                    >OpenAI</button>
                  </div>
                </div>
                <div class="api-key-row">
                  <input
                    type="text"
                    placeholder="{selectedProvider === 'openai' ? 'OpenAI' : 'Groq'} API key"
                    bind:value={apiKey}
                  />
                  <a
                    href={selectedProvider === 'openai' ? 'https://platform.openai.com/api-keys' : 'https://console.groq.com'}
                    target="_blank"
                    rel="noopener noreferrer"
                  >Get key →</a>
                </div>
              </div>
            {/if}
          </div>
        </div>

      {:else if step === 5}
        <div class="step step-controls">
          <h1>Controls</h1>
          <p class="subtitle">Set your hotkey and languages.</p>

          <div class="controls-grid">
            <section class="ctrl-section">
              <h3>Dictation Hotkeys</h3>
              <div class="form-group" role="group" aria-labelledby="hotkey-hold-label">
                <span id="hotkey-hold-label" class="label-text">Hold to Dictate</span>
                <HotkeyCapture
                  value={hotkey}
                  onChange={(h) => (hotkey = h)}
                />
                <p class="rec-desc">Press and hold to dictate, release to stop</p>
              </div>
              <div class="form-group" style="margin-top: 1rem;" role="group" aria-labelledby="hotkey-toggle-label">
                <span id="hotkey-toggle-label" class="label-text">Toggle Dictation</span>
                <HotkeyCapture
                  value={toggleHotkey}
                  onChange={(h) => (toggleHotkey = h)}
                />
                <p class="rec-desc">Press once to start dictating, press again to stop</p>
              </div>
            </section>

            <section class="ctrl-section">
              <h3>Languages</h3>
              <div class="lang-tags">
                {#each languages as code, i}
                  <span class="lang-tag">
                    {#if i === 0}<span class="tag-default">Default</span>{/if}
                    {languageLabel(code)}
                    {#if languages.length > 1}
                      <button type="button" class="tag-remove" on:click={() => removeLanguage(i)}>×</button>
                    {/if}
                  </span>
                {/each}
              </div>
              <select
                class="lang-add"
                bind:value={addLanguageCode}
                on:change={() => addLanguage(addLanguageCode)}
              >
                <option value="">+ Add language</option>
                {#each LANGUAGE_OPTIONS as opt}
                  <option value={opt.code} disabled={languages.includes(opt.code)}>{opt.label}</option>
                {/each}
              </select>
            </section>
          </div>
        </div>

      {:else if step === 6}
        <div class="step step-ready">
          <div class="ready-visual">
            <div class="ready-ring r1"></div>
            <div class="ready-ring r2"></div>
            <div class="ready-ring r3"></div>
            <div class="ready-check">✓</div>
          </div>
          <h1>You're all set</h1>
          <p class="subtitle">Try dictating right now — press your dictation hotkey and speak.</p>

          <div class="demo-box" class:unfocused={!demoFocused}>
            <div class="demo-prompt">
              <kbd>{hotkey || toggleHotkey || 'Ctrl+Win'}</kbd>
              <span>then say anything</span>
            </div>
            <textarea
              bind:this={demoTextarea}
              placeholder="Your transcription will appear here..."
              readonly
              value={demoTranscription}
              on:focus={() => (demoFocused = true)}
              on:blur={() => (demoFocused = false)}
            ></textarea>
            {#if !demoFocused}
              <button class="refocus-cue" on:click={() => demoTextarea?.focus()}>
                Click here to focus — then press <kbd>{hotkey || 'Ctrl+Win'}</kbd> to dictate
              </button>
            {/if}
          </div>
        </div>
      {/if}

      <div class="actions">
        {#if step > 1}
          <button class="btn-back" on:click={prevStep}>Back</button>
        {/if}
        <div class="actions-spacer"></div>
        {#if step < totalSteps}
          <button
            class="btn-next"
            disabled={step === 2 && (!termsAgreed || !isEmailValid(userEmail))}
            on:click={nextStep}
          >Next</button>
        {:else}
          <button class="btn-next btn-finish" on:click={finish}>Get Started</button>
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
            {skipInProgress ? 'Skipping...' : 'Skip setup'}
          </button>
          {#if skipError}
            <p class="skip-error" role="alert">{skipError}</p>
          {/if}
        </div>
      {/if}

    </div>
  </div>
</div>

<style>
  /* ── Layout ── */
  .onboarding {
    position: fixed;
    inset: 0;
    background: var(--bg-app);
    display: flex;
  }

  /* ── Left stepper ── */
  .stepper {
    width: 220px;
    flex-shrink: 0;
    background: var(--bg-card);
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

  .stepper-logo {
    height: 36px;
    width: 36px;
    flex-shrink: 0;
  }

  .stepper-title {
    font-family: 'Syne', sans-serif;
    font-size: 22px;
    font-weight: 700;
    color: var(--navy-deep);
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
    transition: all 0.2s ease;
    color: var(--text-muted);
    font-size: 14px;
    font-weight: 500;
    text-align: left;
  }

  .stepper-btn:disabled {
    cursor: default;
    opacity: 0.5;
  }

  .stepper-btn:not(:disabled):hover {
    background: var(--bg-input);
  }

  .stepper-item.active .stepper-btn {
    color: var(--navy-deep);
    font-weight: 600;
    background: var(--primary-alpha);
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
    transition: all 0.25s ease;
    background: var(--bg-input);
    color: var(--text-muted);
    border: 2px solid var(--border);
  }

  .stepper-item.active .stepper-indicator {
    background: var(--primary);
    color: white;
    border-color: var(--primary);
    box-shadow: 0 2px 8px var(--primary-alpha);
  }

  .stepper-item.complete .stepper-indicator {
    background: var(--success);
    color: white;
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

  .content-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 48px 56px;
    display: flex;
    flex-direction: column;
  }

  .step {
    flex: 1;
    max-width: 600px;
    width: 100%;
    margin: 0 auto;
  }

  h1 {
    font-size: 28px;
    margin-bottom: 8px;
    color: var(--navy-deep);
  }

  .subtitle {
    font-size: 15px;
    color: var(--text-muted);
    margin-bottom: 32px;
    line-height: 1.5;
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
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    transition: border-color 0.2s;
  }

  .feat:hover {
    border-color: var(--primary);
  }

  .feat-icon {
    font-size: 22px;
    flex-shrink: 0;
    margin-top: 1px;
  }

  .feat strong {
    display: block;
    font-size: 14px;
    font-weight: 600;
    color: var(--navy-deep);
    margin-bottom: 2px;
  }

  .feat span {
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  /* ── Step 2: Account ── */
  .form-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 28px;
  }

  .field {
    margin-bottom: 20px;
  }

  .field label {
    display: block;
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 6px;
    color: var(--text-primary);
  }

  .req {
    color: var(--error);
  }

  .field input[type="email"] {
    width: 100%;
    padding: 11px 14px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 14px;
    transition: border-color 0.2s;
  }

  .field input[type="email"]:focus {
    outline: none;
    border-color: var(--primary);
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
    color: var(--text-primary);
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
    color: var(--primary);
    text-decoration: underline;
  }

  .privacy-note {
    font-size: 12px;
    color: var(--primary-dark);
    margin: 8px 0 0;
    font-weight: 500;
  }

  /* ── Step 3: Permissions ── */
  .perm-trust {
    margin-bottom: 20px;
    padding: 14px 18px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .perm-trust p {
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .perm-trust strong {
    color: var(--navy-deep);
  }

  .perm-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 24px;
  }

  .perm-row {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 20px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    transition: border-color 0.2s;
  }

  .perm-row:hover {
    border-color: var(--border-visible);
  }

  .perm-icon-wrap {
    width: 42px;
    height: 42px;
    background: var(--primary-alpha);
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .perm-icon {
    font-size: 20px;
  }

  .perm-info {
    flex: 1;
    min-width: 0;
  }

  .perm-info strong {
    display: block;
    font-size: 14px;
    font-weight: 600;
    color: var(--navy-deep);
  }

  .perm-info span {
    font-size: 13px;
    color: var(--text-muted);
  }

  .perm-auto {
    font-size: 12px;
    color: var(--success);
    font-weight: 600;
    white-space: nowrap;
  }

  .perm-hint {
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .btn-outline-sm {
    padding: 7px 14px;
    font-size: 13px;
    font-weight: 600;
    background: none;
    border: 1px solid var(--primary);
    border-radius: var(--radius-sm);
    color: var(--primary);
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.15s;
  }

  .btn-outline-sm:hover {
    background: var(--primary);
    color: white;
  }

  .mic-test {
    padding: 20px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .mic-test-title {
    font-size: 14px;
    font-weight: 700;
    color: var(--navy-deep);
    margin-bottom: 14px;
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
    border: 2px solid var(--primary);
    border-radius: var(--radius-sm);
    background: var(--primary-alpha);
    color: var(--primary-dark);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
  }

  .mic-action-btn:hover {
    background: var(--primary);
    color: white;
  }

  .mic-action-btn.recording {
    border-color: var(--error);
    background: rgba(239, 68, 68, 0.1);
    color: var(--error);
    animation: pulse-rec 1.5s ease-in-out infinite;
  }

  .mic-action-btn.recording:hover {
    background: var(--error);
    color: white;
  }

  @keyframes pulse-rec {
    0%, 100% { box-shadow: 0 0 0 0 rgba(239, 68, 68, 0.2); }
    50% { box-shadow: 0 0 0 8px rgba(239, 68, 68, 0); }
  }

  .mic-action-btn.play {
    border-color: var(--success);
    background: rgba(16, 185, 129, 0.1);
    color: var(--success);
  }

  .mic-action-btn.play:hover {
    background: var(--success);
    color: white;
  }

  .mic-action-btn.playing {
    opacity: 0.7;
    cursor: default;
  }

  .mic-action-icon {
    font-size: 16px;
  }

  .stop-sq {
    width: 14px;
    height: 14px;
    background: currentColor;
    border-radius: 3px;
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
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.5;
    margin: 0;
  }

  .mic-status strong {
    font-weight: 600;
    color: var(--text-primary);
  }

  .error-text {
    color: var(--error) !important;
  }

  /* ── Step 4: Mode ── */
  .mode-pills {
    display: flex;
    gap: 8px;
    margin-bottom: 20px;
  }

  .mode-pill {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 14px 12px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    border-radius: var(--radius-md);
    font-size: 14px;
    font-weight: 600;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s ease;
    position: relative;
  }

  .mode-pill:hover {
    border-color: var(--border-visible);
    background: var(--bg-input);
  }

  .mode-pill.active {
    border-color: var(--primary);
    background: var(--primary-alpha);
    color: var(--primary-dark);
    box-shadow: 0 2px 8px var(--primary-alpha);
  }

  .mp-icon {
    font-size: 18px;
  }

  .rec-dot {
    position: absolute;
    top: -4px;
    right: -4px;
    width: 10px;
    height: 10px;
    background: var(--primary);
    border: 2px solid var(--bg-app);
    border-radius: 50%;
  }

  .mode-detail {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 24px;
  }

  .mode-desc p {
    font-size: 14px;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.5;
  }

  .mode-stats {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 12px;
  }

  .stat {
    display: inline-block;
    padding: 4px 10px;
    background: var(--bg-input);
    border-radius: var(--radius-pill);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .stat.rec {
    background: var(--primary-alpha);
    color: var(--primary-dark);
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
    background: var(--bg-input);
    border: 2px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 14px;
    transition: border-color 0.2s;
  }

  .api-key-row input:focus {
    outline: none;
    border-color: var(--primary);
  }

  .api-key-row a {
    color: var(--primary);
    font-size: 13px;
    font-weight: 600;
    text-decoration: none;
    white-space: nowrap;
  }

  .api-key-row a:hover {
    text-decoration: underline;
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
    color: var(--text-primary);
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
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s;
  }

  .prov-pill:hover:not(.active) {
    color: var(--text-primary);
  }

  .prov-pill.active {
    background: var(--bg-card);
    color: var(--navy-deep);
    box-shadow: var(--shadow-sm);
  }

  .local-hint {
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .local-hint strong {
    color: var(--primary-dark);
    font-weight: 600;
  }

  /* ── Step 5: Controls ── */
  .controls-grid {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .ctrl-section {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 24px;
  }

  .ctrl-section h3 {
    font-size: 14px;
    font-weight: 700;
    color: var(--navy-deep);
    margin-bottom: 14px;
    letter-spacing: 0;
  }

  .rec-mode {
    display: flex;
    gap: 10px;
    margin-top: 16px;
  }

  .rec-mode label {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 12px;
    background: var(--bg-input);
    border: 2px solid transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.2s;
    text-align: center;
  }

  .rec-mode label.selected {
    border-color: var(--primary);
    background: var(--primary-alpha);
  }

  .rec-mode input[type="radio"] {
    display: none;
  }

  .rec-label {
    font-size: 14px;
    font-weight: 600;
    color: var(--navy-deep);
  }

  .form-group .label-text {
    display: block;
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 6px;
    color: var(--text-primary);
  }

  .rec-desc {
    font-size: 12px;
    color: var(--text-muted);
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
    border-radius: var(--radius-pill);
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .tag-default {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    background: var(--primary);
    color: white;
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
    appearance: none;
    -webkit-appearance: none;
    padding: 10px 14px;
    background: var(--bg-input);
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='%2364748B' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 12px center;
    background-size: 16px;
    border: 2px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: border-color 0.2s;
  }

  .lang-add:focus {
    outline: none;
    border-color: var(--primary);
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
    border: 2px solid var(--primary);
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
    font-size: 36px;
    font-weight: 700;
    color: var(--primary);
    background: var(--primary-alpha);
    border-radius: 50%;
    animation: check-pop 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
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
    border: 1px solid var(--border-visible);
    border-radius: var(--radius-sm);
    font-family: 'Google Sans', ui-monospace, monospace;
    font-size: 14px;
    font-weight: 700;
    color: var(--navy-deep);
    box-shadow: 0 2px 0 var(--border);
  }

  .demo-prompt span {
    font-size: 14px;
    color: var(--text-muted);
  }

  .demo-box textarea {
    width: 100%;
    height: 90px;
    padding: 14px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 14px;
    resize: none;
    transition: border-color 0.2s;
  }

  .demo-box textarea:focus {
    outline: none;
    border-color: var(--primary);
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
    border: 1px solid var(--border-visible);
    border-radius: 4px;
    font-family: 'Google Sans', ui-monospace, monospace;
    font-size: 12px;
    font-weight: 700;
    color: var(--navy-deep);
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
    background: var(--primary);
    border: none;
    border-radius: var(--radius-sm);
    color: white;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn-next:hover {
    background: var(--primary-dark);
  }

  .btn-next:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-finish {
    padding: 12px 36px;
    font-size: 15px;
  }

  .btn-back {
    padding: 11px 24px;
    background: none;
    border: 1px solid var(--border-visible);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn-back:hover {
    background: var(--bg-input);
    color: var(--navy-deep);
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
      background: var(--bg-card);
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
      background: var(--primary);
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

    h1 {
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

    .perm-row {
      flex-wrap: wrap;
      gap: 12px;
      padding: 14px 16px;
    }

    .perm-info {
      flex: 1;
      min-width: 120px;
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

    .ctrl-section {
      padding: 18px;
    }

    .rec-mode {
      flex-direction: column;
      gap: 8px;
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
