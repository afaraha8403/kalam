<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import { sidebarDictationStore, displayHotkey } from '../lib/sidebarDictation'
  import { superKeyLabel } from '../lib/platformHotkey'
  import { dictationRuntimeStore } from '../lib/dictationState'
  import { languageLabel } from '../lib/languages'
  import { invoke, isTauriRuntime } from '../lib/backend'
  import type { AppConfig, AudioDevice } from '../types'

  type SttMode = AppConfig['stt_config']['mode']

  /** Menu entries typed as SttMode so the template needs no assertions. */
  const STT_MENU_MODES: SttMode[] = ['Cloud', 'Hybrid', 'Local', 'Auto']

  export let config: AppConfig | null = null
  export let dbStatus: { ok: boolean } | null = null
  export let platform: string = ''
  /** Last transcription latency in ms (from backend or transcription-saved event). */
  export let lastLatencyMs: number | null = null
  /** Master dictation switch; when false, status bar shows off state and does not start capture. */
  export let dictationEnabled = true
  /** Parent refreshes DB status after invoke (e.g. App updates `dbStatus`). */
  export let onRetryDb: () => void | Promise<void> = () => {}

  let sttMenuOpen = false
  let sttWrapEl: HTMLDivElement | null = null

  let audioDevices: AudioDevice[] = []
  /** Bound to mic `<select>`; synced from `config` when settings change. */
  let micSelection = ''

  $: rt = $dictationRuntimeStore

  $: if (config) {
    micSelection =
      config.audio_device == null || config.audio_device === '' ? '' : String(config.audio_device)
  } else {
    micSelection = ''
  }

  async function loadAudioDevices() {
    if (!isTauriRuntime()) return
    try {
      audioDevices = (await invoke('get_audio_devices')) as AudioDevice[]
    } catch (e) {
      console.error('get_audio_devices failed:', e)
      audioDevices = []
    }
  }

  async function onMicChange() {
    const value = micSelection
    if (!isTauriRuntime()) return
    try {
      const current = (await invoke('get_settings')) as AppConfig
      const next: AppConfig = {
        ...current,
        audio_device: value === '' ? null : value,
      }
      await invoke('save_settings', { newConfig: next })
    } catch (err) {
      console.error('save_settings (audio device) failed:', err)
    }
  }

  function formatMmSs(totalSec: number): string {
    const s = Math.max(0, Math.floor(totalSec))
    const m = Math.floor(s / 60)
    const r = s % 60
    return `${m}:${r.toString().padStart(2, '0')}`
  }

  function truncate(s: string, max: number): string {
    const t = s.trim()
    if (t.length <= max) return t
    return `${t.slice(0, max - 1)}…`
  }

  /**
   * One leading emoji per state (primary line only — no icon font).
   * 🔄 boot · 🔕 muted · ✅ idle · 🎧 listening · 🔴 recording · ⏳ transcribing · ⚠️ error · 💬 status · ⏹️ cancelling
   */
  $: primaryLabel = (() => {
    if (config === null) return '🔄 Starting…'
    if (!dictationEnabled) return '🔕 Dictation off'
    switch (rt.phase) {
      case 'idle':
        return '✅ Ready'
      case 'listening':
        return '🎧 Listening…'
      case 'recording':
        return `🔴 Recording ${formatMmSs(rt.recordingDurationSec)}`
      case 'processing':
        return `⏳ Processing ${rt.processingElapsedSec}s`
      case 'error':
        return `⚠️ ${truncate(rt.errorMessage ?? 'Error', 38)}`
      case 'status':
        return `💬 ${truncate(rt.statusMessage ?? '', 38)}`
      case 'cancelling':
        return '⏹️ Cancelling…'
      default:
        return '✅ Ready'
    }
  })()

  $: latencyTitle =
    lastLatencyMs != null
      ? `⚡ Last transcription: ${lastLatencyMs} ms`
      : '⚡ Latency — shown after your first transcription in this session'

  $: sttMode = config?.stt_config?.mode ?? 'Cloud'
  $: sttProvider = config?.stt_config?.provider ?? 'groq'
  $: localModel = config?.stt_config?.local_model ?? 'sensevoice'
  $: localModelLabel =
    localModel === 'whisper_base' ? 'Whisper Base' : localModel === 'sensevoice' ? 'SenseVoice' : localModel
  $: providerLabel =
    sttProvider === 'openai' ? 'OpenAI' : sttProvider === 'groq' ? 'Groq' : sttProvider
  $: sttShortLabel =
    sttMode === 'Local' ? 'Local' : sttMode === 'Hybrid' ? 'Hybrid' : sttMode === 'Auto' ? 'Auto' : 'Cloud'
  $: sttDetailTitle = `🌐 ${sttMode} · ${providerLabel}${
    sttMode === 'Local' || sttMode === 'Hybrid' ? ` · ${localModelLabel}` : ''
  }`

  $: dictateHotkey =
    $sidebarDictationStore?.hotkey != null
      ? displayHotkey($sidebarDictationStore.hotkey, $sidebarDictationStore.platform)
      : $sidebarDictationStore?.toggleDictationHotkey != null
        ? displayHotkey($sidebarDictationStore.toggleDictationHotkey, $sidebarDictationStore.platform)
        : `Ctrl+${superKeyLabel(platform)}`
  $: showLang =
    ($sidebarDictationStore?.languages?.length ?? 0) >= 2 &&
    $sidebarDictationStore?.languageToggleHotkey
  $: langHotkey =
    showLang && $sidebarDictationStore?.languageToggleHotkey
      ? displayHotkey($sidebarDictationStore.languageToggleHotkey, $sidebarDictationStore.platform)
      : ''
  $: cmdConfig = config?.command_config
  $: showCmd = !!(cmdConfig?.enabled && cmdConfig?.hotkey?.trim())
  $: cmdHotkey =
    showCmd && cmdConfig?.hotkey ? displayHotkey(cmdConfig.hotkey, platform) : ''

  $: langs = config?.languages ?? []
  $: activeLangCode = langs[0] ?? 'en'
  $: activeLangShort = activeLangCode.slice(0, 2).toUpperCase()
  $: showLangBadge = langs.length >= 2

  $: readyTitle =
    config && lastLatencyMs != null
      ? `✅ Ready · ${sttDetailTitle} · Last latency ${lastLatencyMs} ms`
      : config
        ? `✅ Ready · ${sttDetailTitle}`
        : '🔄 Loading app configuration…'

  $: dbTitle =
    dbStatus === null
      ? '💾 Checking database…'
      : dbStatus.ok
        ? '💾 Database connected'
        : '💾 Database disconnected — click to retry'

  /** Emoji only for DB row (aligned with title). */
  $: dbEmoji = dbStatus === null ? '⏳' : dbStatus.ok ? '✅' : '⚠️'

  async function onPrimaryClick() {
    if (!isTauriRuntime() || !dictationEnabled || config === null) return
    try {
      await invoke('ui_toggle_dictation')
    } catch (e) {
      console.error('ui_toggle_dictation failed:', e)
    }
  }

  async function onCancelProcessing() {
    if (!isTauriRuntime()) return
    try {
      await invoke('cancel_transcription')
    } catch (e) {
      console.error('cancel_transcription failed:', e)
    }
  }

  async function onDbClick() {
    if (dbStatus?.ok !== false) return
    await Promise.resolve(onRetryDb())
  }

  async function setSttModeFromMenu(mode: SttMode) {
    if (!config || !isTauriRuntime()) {
      sttMenuOpen = false
      return
    }
    try {
      const current = (await invoke('get_settings')) as AppConfig
      const next: AppConfig = {
        ...current,
        stt_config: { ...current.stt_config, mode },
      }
      await invoke('save_settings', { newConfig: next })
      if (mode === 'Cloud') {
        try {
          await invoke('stop_all_local_models')
        } catch {
          /* best-effort */
        }
      }
    } catch (e) {
      console.error('save_settings (STT mode) failed:', e)
    }
    sttMenuOpen = false
  }

  function onDocClick(e: MouseEvent) {
    if (!sttMenuOpen || !sttWrapEl) return
    const t = e.target as Node
    if (!sttWrapEl.contains(t)) sttMenuOpen = false
  }

  onMount(() => {
    document.addEventListener('click', onDocClick, true)
    if (isTauriRuntime()) void loadAudioDevices()
  })
  onDestroy(() => {
    document.removeEventListener('click', onDocClick, true)
  })
</script>

<footer class="status-bar" role="status" aria-live="polite">
  <button
    type="button"
    class="segment primary interactive"
    class:recording={rt.phase === 'recording'}
    title={rt.phase === 'idle' && config && dictationEnabled ? `🎙️ Click to start or stop dictation (${dictateHotkey})` : readyTitle}
    disabled={config === null || !dictationEnabled || !isTauriRuntime()}
    on:click={onPrimaryClick}
  >
    <span class="primary-text">{primaryLabel}</span>
  </button>

  <button
    type="button"
    class="segment db-segment interactive"
    class:error={dbStatus !== null && !dbStatus.ok}
    title={dbTitle}
    disabled={dbStatus === null || dbStatus.ok}
    aria-label={dbStatus === null
      ? 'Database: checking connection'
      : dbStatus.ok
        ? 'Database: connected'
        : 'Database: disconnected — click to retry'}
    on:click={onDbClick}
  >
    <span class="emoji-cell" aria-hidden="true">{dbEmoji}</span>
    <span class="db-label">Database</span>
  </button>

  <div class="stt-wrap" bind:this={sttWrapEl}>
    <button
      type="button"
      class="segment stt-mode interactive"
      title={sttDetailTitle}
      disabled={!config || !isTauriRuntime()}
      on:click|stopPropagation={() => {
        if (config && isTauriRuntime()) sttMenuOpen = !sttMenuOpen
      }}
    >
      <span class="emoji-cell" aria-hidden="true">🌐</span>
      <span>{sttShortLabel}</span>
      <!-- Same chevron convention as native <select> (▼ closed / ▲ open). -->
      <span class="stt-chevron" aria-hidden="true">{sttMenuOpen ? '▲' : '▼'}</span>
    </button>
    {#if sttMenuOpen && config}
      <div class="stt-menu" role="menu">
        {#each STT_MENU_MODES as m}
          <button
            type="button"
            role="menuitem"
            class:active={sttMode === m}
            on:click|stopPropagation={() => setSttModeFromMenu(m)}
          >
            {#if m === 'Cloud'}
              ☁️ Cloud
            {:else if m === 'Hybrid'}
              🔀 Hybrid
            {:else if m === 'Local'}
              🖥️ Local
            {:else}
              🤖 Auto
            {/if}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <span class="segment latency" title={latencyTitle}>
    <span class="emoji-cell" aria-hidden="true">⚡</span>
    {#if lastLatencyMs != null}
      {lastLatencyMs} ms
    {:else}
      —
    {/if}
  </span>

  {#if showLangBadge}
    <span class="segment lang-badge" title={`🌍 Recognition language: ${languageLabel(activeLangCode)} (${activeLangCode})`}>
      <span class="emoji-cell" aria-hidden="true">🌍</span>
      {activeLangShort}
    </span>
  {/if}

  <span class="segment hotkeys" title="⌨️ Dictate: {dictateHotkey}{showLang ? ` · Lang: ${langHotkey}` : ''}{showCmd ? ` · Cmd: ${cmdHotkey}` : ''}">
    <span class="emoji-cell" aria-hidden="true">⌨️</span>
    <kbd>{dictateHotkey}</kbd>
    {#if showLang}
      <span class="hotkey-extra">· <kbd>{langHotkey}</kbd></span>
    {/if}
    {#if showCmd}
      <span class="hotkey-extra">· <kbd>{cmdHotkey}</kbd></span>
    {/if}
  </span>

  {#if rt.phase === 'processing'}
    <button type="button" class="segment cancel-seg interactive" title="⏹️ Cancel transcription" on:click={onCancelProcessing}>
      ⏹️ Cancel
    </button>
  {/if}

  {#if rt.phase === 'recording'}
    <span class="audio-level" aria-hidden="true" title="🎚️ Input level">
      {#each [0, 1, 2, 3, 4] as i}
        <span
          class="bar"
          style="height: {Math.min(12, 3 + rt.audioLevel * (i + 1) * 2.2)}px"
        />
      {/each}
    </span>
  {/if}

  {#if config}
    <div class="segment mic-wrap">
      <span class="emoji-cell" aria-hidden="true">🎙️</span>
      <select
        class="mic-select"
        bind:value={micSelection}
        title="Microphone — list refreshes on focus"
        aria-label="Microphone input device"
        disabled={!isTauriRuntime()}
        on:focus={() => loadAudioDevices()}
        on:change={onMicChange}
      >
        {#if audioDevices.length === 0}
          <option value="">{isTauriRuntime() ? 'Loading mics…' : '—'}</option>
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
  {/if}
</footer>

<style>
  /*
   * Single type ramp for the whole footer so buttons, kbd, labels, and <select>
   * match (same size/weight/line-height as the shell body).
   */
  .status-bar {
    --status-font-size: 12px;
    --status-line-height: 1.35;
    flex-shrink: 0;
    width: 100%;
    min-height: 32px;
    display: flex;
    align-items: center;
    justify-content: space-evenly;
    flex-wrap: wrap;
    gap: 10px 14px;
    row-gap: 6px;
    padding: 0 16px;
    font-family: inherit;
    font-size: var(--status-font-size);
    font-weight: 400;
    line-height: var(--status-line-height);
    letter-spacing: normal;
    color: var(--text-secondary);
    background: var(--bg-card);
    border-top: 1px solid var(--border-subtle);
    box-shadow: 0 -2px 8px rgba(0, 0, 0, 0.02);
  }

  .segment {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: none;
    border: none;
    font-family: inherit;
    font-size: inherit;
    font-weight: 400;
    line-height: inherit;
    letter-spacing: inherit;
    color: inherit;
    padding: 2px 4px;
    border-radius: var(--radius-sm);
  }

  .segment.interactive:hover:not(:disabled) {
    background: var(--bg-hover, rgba(0, 0, 0, 0.06));
    cursor: pointer;
  }

  .segment.interactive:disabled {
    cursor: default;
    opacity: 0.75;
  }

  .segment.error {
    color: var(--text-primary);
  }

  .primary-text {
    font: inherit;
  }

  .primary.recording .primary-text {
    color: var(--text-primary);
  }

  .emoji-cell {
    font-size: 1em;
    line-height: 1;
    user-select: none;
    flex-shrink: 0;
  }

  /* Chevron (▼/▲) aligned with native select arrows on the mic control. */
  .stt-chevron {
    font-size: 0.7em;
    opacity: 0.8;
    margin-left: 2px;
    line-height: 1;
    user-select: none;
  }

  .db-label {
    font: inherit;
  }

  .mic-wrap {
    gap: 5px;
    min-width: 0;
    max-width: min(220px, 28vw);
  }

  .mic-select {
    flex: 1;
    min-width: 0;
    max-width: 100%;
    font-family: inherit;
    font-size: inherit;
    font-weight: inherit;
    line-height: inherit;
    letter-spacing: inherit;
    color: inherit;
    background: transparent;
    border: none;
    padding: 0 4px 0 0;
    margin: 0;
    cursor: pointer;
    text-overflow: ellipsis;
  }

  .mic-select:disabled {
    cursor: default;
    opacity: 0.75;
  }

  .mic-select:focus {
    outline: 1px solid var(--border);
    outline-offset: 2px;
  }

  /* Plain text hotkeys: no pill chrome; same font as the bar. */
  .hotkeys kbd {
    background: transparent;
    border: none;
    padding: 0;
    margin: 0;
    border-radius: 0;
    font: inherit;
  }

  .stt-wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
  }

  .stt-menu {
    position: absolute;
    bottom: 100%;
    left: 50%;
    transform: translateX(-50%);
    margin-bottom: 6px;
    min-width: 120px;
    padding: 4px;
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    border: 1px solid var(--border);
    box-shadow: 0 -4px 16px rgba(0, 0, 0, 0.12);
    z-index: 50;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .stt-menu button {
    text-align: left;
    padding: 6px 10px;
    border: none;
    background: transparent;
    border-radius: 4px;
    font: inherit;
    font-weight: 400;
    cursor: pointer;
    color: var(--text-primary);
  }

  .stt-menu button:hover {
    background: var(--bg-hover, rgba(0, 0, 0, 0.06));
  }

  .stt-menu button.active {
    font-weight: 400;
    color: var(--primary);
  }

  .cancel-seg {
    background: var(--bg-input);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    font: inherit;
    border: 1px solid var(--border);
    color: var(--text-primary);
  }

  .hotkey-extra {
    opacity: 0.9;
  }

  .latency {
    opacity: 0.9;
    font: inherit;
  }

  .lang-badge {
    opacity: 0.95;
    font: inherit;
  }

  .audio-level {
    display: inline-flex;
    gap: 2px;
    align-items: flex-end;
    height: 14px;
  }

  .audio-level .bar {
    width: 3px;
    min-height: 2px;
    background: var(--primary);
    border-radius: 1px;
    opacity: 0.85;
  }
</style>
