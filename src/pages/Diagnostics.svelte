<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke, isTauriRuntime } from '$lib/backend'
  import Icon from '@iconify/svelte'
  import type { AppConfig } from '../types'

  type SystemInfo = {
    os_name: string
    os_version: string
    architecture: string
    kalam_config_path: string
    kalam_config_exists: boolean
  }

  let systemInfo: SystemInfo | null = null
  let loadErr = ''
  let busy: string | null = null
  let lastMessage = ''
  let lastDetail = ''

  let captureSecs = 8
  let matchHotkey = 'Ctrl+Win'

  async function refreshSystemInfo() {
    if (!isTauriRuntime()) {
      loadErr = 'Diagnostics run inside the desktop app only.'
      return
    }
    loadErr = ''
    try {
      systemInfo = (await invoke('get_diagnostic_system_info')) as SystemInfo
    } catch (e) {
      loadErr = e instanceof Error ? e.message : String(e)
    }
  }

  onMount(() => {
    void refreshSystemInfo()
    void (async () => {
      if (!isTauriRuntime()) return
      try {
        const cfg = (await invoke('get_settings')) as AppConfig
        const h = cfg.hotkey || cfg.toggle_dictation_hotkey
        if (h) matchHotkey = h
      } catch {
        // keep default
      }
    })()
  })

  function setBusy(id: string | null) {
    busy = id
    lastMessage = ''
    lastDetail = ''
  }

  async function run(id: string, fn: () => Promise<unknown>) {
    if (!isTauriRuntime()) return
    setBusy(id)
    try {
      const out = await fn()
      lastMessage = 'Done.'
      lastDetail = JSON.stringify(out, null, 2)
    } catch (e) {
      lastMessage = 'Failed.'
      lastDetail = e instanceof Error ? e.message : String(e)
    } finally {
      busy = null
    }
  }

  function runHook() {
    return run('hook', () => invoke('run_hook_installation_test'))
  }

  function runCapture() {
    const s = Math.min(120, Math.max(1, Math.floor(Number(captureSecs)) || 8))
    captureSecs = s
    return run('capture', () => invoke('run_key_capture_test', { durationSecs: s }))
  }

  function runMatch() {
    const hk = matchHotkey.trim()
    return run('match', () => invoke('run_hotkey_matching_test', { hotkeyStr: hk }))
  }

  function runConfig() {
    return run('config', () => invoke('analyze_kalam_config_diagnostic'))
  }

  function runHealth() {
    return run('health', () => invoke('run_system_health_check'))
  }

  async function getModifierState() {
    if (!isTauriRuntime()) return
    setBusy('modifier')
    try {
      const [ctrl, alt, shift, meta] = (await invoke('get_modifier_state')) as [boolean, boolean, boolean, boolean]
      lastMessage = 'Internal Modifier State'
      lastDetail = JSON.stringify({ ctrl, alt, shift, meta }, null, 2)
    } catch (e) {
      lastMessage = 'Failed to get state.'
      lastDetail = e instanceof Error ? e.message : String(e)
    } finally {
      busy = null
    }
  }

  async function saveReport() {
    if (!isTauriRuntime()) return
    setBusy('save')
    try {
      const path = (await invoke('save_diagnostics_report_to_file')) as string
      lastMessage = 'Report saved.'
      lastDetail = path
    } catch (e) {
      lastMessage = 'Save failed.'
      lastDetail = e instanceof Error ? e.message : String(e)
    } finally {
      busy = null
    }
  }
</script>

<div class="page fade-in">
  <header class="page-header notes-header">
    <div>
      <h1 class="page-title">Diagnostics</h1>
      <p class="page-subtitle">
        Troubleshoot global hotkeys and config. Capture tests use a short, separate listener — safe to run while Kalam is open.
      </p>
    </div>
  </header>

  {#if loadErr}
    <p class="diag-error" role="alert">{loadErr}</p>
  {/if}

  {#if systemInfo}
    <section class="diag-card">
      <h2 class="diag-h2">System</h2>
      <ul class="diag-list">
        <li><strong>OS</strong> — {systemInfo.os_name} ({systemInfo.architecture})</li>
        <li><strong>Version</strong> — {systemInfo.os_version}</li>
        <li><strong>Config</strong> — {systemInfo.kalam_config_path}</li>
        <li><strong>Config file</strong> — {systemInfo.kalam_config_exists ? 'found' : 'missing'}</li>
      </ul>
    </section>
  {/if}

  <section class="diag-card">
    <h2 class="diag-h2">Tests</h2>
    <div class="diag-actions">
      <button
        type="button"
        class="btn-secondary"
        disabled={!isTauriRuntime() || busy !== null}
        on:click={() => void runHook()}
      >
        {#if busy === 'hook'}
          <span class="spin-wrap"><Icon icon="ph:spinner" /></span>
        {:else}
          <Icon icon="ph:plug" />
        {/if}
        Hook install probe
      </button>

      <div class="diag-inline">
        <label class="diag-label" for="cap-secs">Seconds</label>
        <input id="cap-secs" type="number" min="1" max="120" bind:value={captureSecs} class="diag-input" />
        <button
          type="button"
          class="btn-secondary"
          disabled={!isTauriRuntime() || busy !== null}
          on:click={() => void runCapture()}
        >
          {#if busy === 'capture'}
            <span class="spin-wrap"><Icon icon="ph:spinner" /></span>
          {:else}
            <Icon icon="ph:keyboard" />
          {/if}
          Key capture
        </button>
      </div>

      <div class="diag-inline diag-inline-grow">
        <label class="diag-label" for="match-hk">Hotkey</label>
        <input
          id="match-hk"
          type="text"
          bind:value={matchHotkey}
          class="diag-input diag-input-wide"
          placeholder="Ctrl+Win"
          autocomplete="off"
        />
        <button
          type="button"
          class="btn-secondary"
          disabled={!isTauriRuntime() || busy !== null}
          on:click={() => void runMatch()}
        >
          {#if busy === 'match'}
            <span class="spin-wrap"><Icon icon="ph:spinner" /></span>
          {:else}
            <Icon icon="ph:crosshair" />
          {/if}
          Match test (10s)
        </button>
      </div>

      <button
        type="button"
        class="btn-secondary"
        disabled={!isTauriRuntime() || busy !== null}
        on:click={() => void runConfig()}
      >
        {#if busy === 'config'}
          <span class="spin-wrap"><Icon icon="ph:spinner" /></span>
        {:else}
          <Icon icon="ph:file-json" />
        {/if}
        Analyze config
      </button>

      <button
        type="button"
        class="btn-secondary"
        disabled={!isTauriRuntime() || busy !== null}
        on:click={() => void runHealth()}
      >
        {#if busy === 'health'}
          <span class="spin-wrap"><Icon icon="ph:spinner" /></span>
        {:else}
          <Icon icon="ph:heartbeat" />
        {/if}
        System health (DISM)
      </button>

      <button
        type="button"
        class="btn-secondary"
        disabled={!isTauriRuntime() || busy !== null}
        on:click={() => void getModifierState()}
      >
        {#if busy === 'modifier'}
          <span class="spin-wrap"><Icon icon="ph:spinner" /></span>
        {:else}
          <Icon icon="ph:keyboard" />
        {/if}
        Get internal modifier state
      </button>

      <button
        type="button"
        class="btn-primary"
        disabled={!isTauriRuntime() || busy !== null}
        on:click={() => void saveReport()}
      >
        {#if busy === 'save'}
          <span class="spin-wrap"><Icon icon="ph:spinner" /></span>
        {:else}
          <Icon icon="ph:floppy-disk" />
        {/if}
        Save report (.md)
      </button>
    </div>
  </section>

  {#if lastMessage || lastDetail}
    <section class="diag-card diag-result">
      <h2 class="diag-h2">{lastMessage}</h2>
      {#if lastDetail}
        <pre class="diag-pre">{lastDetail}</pre>
      {/if}
    </section>
  {/if}
</div>

<style>
  .diag-error {
    color: var(--danger, #c62828);
    margin: 0 0 1rem;
  }

  .diag-card {
    background: var(--surface-raised, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.08));
    border-radius: 12px;
    padding: 1rem 1.25rem;
    margin-bottom: 1rem;
  }

  .diag-h2 {
    font-size: 0.95rem;
    font-weight: 600;
    margin: 0 0 0.75rem;
    opacity: 0.92;
  }

  .diag-list {
    margin: 0;
    padding-left: 1.2rem;
    line-height: 1.6;
    font-size: 0.9rem;
  }

  .diag-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.65rem;
    align-items: center;
  }

  .diag-inline {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
  }

  .diag-inline-grow .diag-input-wide {
    min-width: 140px;
    flex: 1;
    max-width: 220px;
  }

  .diag-label {
    font-size: 0.8rem;
    opacity: 0.75;
  }

  .diag-input {
    width: 4rem;
    padding: 0.35rem 0.5rem;
    border-radius: 8px;
    border: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.12));
    background: var(--surface, rgba(0, 0, 0, 0.2));
    color: inherit;
  }

  .diag-result .diag-h2 {
    margin-bottom: 0.5rem;
  }

  .diag-pre {
    margin: 0;
    font-size: 0.78rem;
    line-height: 1.45;
    max-height: 420px;
    overflow: auto;
    padding: 0.75rem;
    border-radius: 8px;
    background: rgba(0, 0, 0, 0.25);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .spin-wrap {
    display: inline-flex;
    animation: diag-spin 0.8s linear infinite;
  }

  @keyframes diag-spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
