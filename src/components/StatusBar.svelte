<script lang="ts">
  import { sidebarDictationStore, displayHotkey } from '../lib/sidebarDictation'
  import Icon from '@iconify/svelte'
  import type { AppConfig } from '../types'

  export let config: AppConfig | null = null
  export let dbStatus: { ok: boolean } | null = null
  export let platform: string = ''
  /** Last transcription latency in ms (from backend or transcription-saved event). */
  export let lastLatencyMs: number | null = null

  $: serviceLabel = config !== null ? 'Ready' : 'Starting…'
  $: latencyLabel = lastLatencyMs != null ? `Latency ${lastLatencyMs} ms` : 'Latency —'
  $: dbLabel =
    dbStatus === null ? '…' : dbStatus.ok ? 'DB connected' : 'DB disconnected'

  $: sttMode = config?.stt_config?.mode ?? 'Cloud'
  $: sttProvider = config?.stt_config?.provider ?? 'groq'
  $: localModel = config?.stt_config?.local_model ?? 'sensevoice'
  $: localModelLabel = localModel === 'whisper_base' ? 'Whisper Base' : localModel === 'sensevoice' ? 'SenseVoice' : localModel
  $: providerLabel = sttProvider === 'openai' ? 'OpenAI' : sttProvider === 'groq' ? 'Groq' : sttProvider
  $: sttModeLabel =
    sttMode === 'Local'
      ? `Local · ${localModelLabel}`
      : sttMode === 'Hybrid'
        ? `Hybrid · ${providerLabel}`
        : `Cloud · ${providerLabel}`

  $: dictateHotkey =
    $sidebarDictationStore?.hotkey != null
      ? displayHotkey($sidebarDictationStore.hotkey, $sidebarDictationStore.platform)
      : $sidebarDictationStore?.toggleDictationHotkey != null
        ? displayHotkey($sidebarDictationStore.toggleDictationHotkey, $sidebarDictationStore.platform)
        : platform === 'windows'
          ? 'Ctrl+Win'
          : 'Ctrl+Super'
  $: showLang =
    ($sidebarDictationStore?.languages?.length ?? 0) >= 2 &&
    $sidebarDictationStore?.languageToggleHotkey
  $: langHotkey = showLang && $sidebarDictationStore?.languageToggleHotkey
    ? displayHotkey($sidebarDictationStore.languageToggleHotkey, $sidebarDictationStore.platform)
    : ''
  $: cmdConfig = config?.command_config
  $: showCmd = !!(cmdConfig?.enabled && cmdConfig?.hotkey?.trim())
  $: cmdHotkey = showCmd && cmdConfig?.hotkey
    ? displayHotkey(cmdConfig.hotkey, platform)
    : ''
</script>

<footer class="status-bar" role="status" aria-live="polite">
  <span class="segment">
    <span class="dot" class:ready={config !== null}><Icon icon="ph:circle-fill" /></span>
    {serviceLabel}
  </span>
  <span class="sep">·</span>
  <span class="segment" class:error={dbStatus !== null && !dbStatus.ok}>
    <Icon icon="ph:database-duotone" class="icon" />
    {dbLabel}
  </span>
  <span class="sep">·</span>
  <span class="segment stt-mode">
    <Icon icon="ph:microphone-duotone" class="icon" />
    {sttModeLabel}
  </span>
  <span class="sep">·</span>
  <span class="segment latency">
    <Icon icon="ph:clock-duotone" class="icon" />
    {latencyLabel}
  </span>
  <span class="sep">·</span>
  <span class="segment hotkeys">
    <Icon icon="ph:keyboard-duotone" class="icon" />
    Dictate: <kbd>{dictateHotkey}</kbd>
    {#if showLang}
      · Lang: <kbd>{langHotkey}</kbd>
    {/if}
    {#if showCmd}
      · Cmd: <kbd>{cmdHotkey}</kbd>
    {/if}
  </span>
</footer>

<style>
  .status-bar {
    flex-shrink: 0;
    width: 100%;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: space-evenly;
    padding: 0 16px;
    font-size: 12px;
    color: var(--text-secondary);
    background: var(--bg-card);
    border-top: 1px solid var(--border-subtle);
    box-shadow: 0 -2px 8px rgba(0, 0, 0, 0.02);
  }

  .segment {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .segment.error {
    color: var(--text-primary);
  }

  .dot {
    display: inline-flex;
    font-size: 6px;
    color: var(--text-muted);
  }

  .dot :global(svg) {
    width: 6px;
    height: 6px;
  }

  .dot.ready {
    color: var(--primary);
  }

  .status-bar :global(.icon) {
    font-size: 14px;
    opacity: 0.8;
  }

  .sep {
    user-select: none;
    opacity: 0.6;
  }

  .hotkeys kbd {
    background: var(--bg-input);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    font-family: 'Google Sans', ui-monospace, monospace;
    font-size: 11px;
    font-weight: 600;
    border: 1px solid var(--border);
    color: var(--navy-deep);
  }

  .latency {
    opacity: 0.8;
  }
</style>
