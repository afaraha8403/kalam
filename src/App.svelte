<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { initTelemetry } from './lib/telemetry'
  import { sidebarDictationStore, displayHotkey } from './lib/sidebarDictation'
  import Settings from './pages/Settings.svelte'
  import History from './pages/History.svelte'
  import Snippets from './pages/Snippets.svelte'
  import Onboarding from './pages/Onboarding.svelte'
  import Overlay from './components/Overlay.svelte'
  import type { AppConfig } from './types'

  const isOverlay =
    (() => {
      try {
        return getCurrentWebviewWindow().label === 'overlay'
      } catch {
        return false
      }
    })()

  // Must run synchronously before first paint to prevent dark flash
  if (isOverlay) {
    document.documentElement.style.background = 'transparent'
    document.body.style.background = 'transparent'
    document.body.style.overflow = 'hidden'
  }

  let currentPage = 'settings'
  let isFirstRun = true

  onMount(async () => {
    if (isOverlay) return
    try {
      const config = (await invoke('get_settings')) as AppConfig
      isFirstRun = !config.onboarding_complete
      initTelemetry(config.privacy?.telemetry_enabled ?? false)
      const platform = (await invoke('get_platform')) as string
      sidebarDictationStore.updateFromConfig(config, platform)
    } catch {
      isFirstRun = true
    }
  })

  function navigate(page: string) {
    currentPage = page
  }

  async function handleOnboardingComplete() {
    isFirstRun = false
    try {
      const config = (await invoke('get_settings')) as AppConfig
      const platform = (await invoke('get_platform')) as string
      sidebarDictationStore.updateFromConfig(config, platform)
    } catch {
      // keep store as-is
    }
  }
</script>

{#if isOverlay}
  <Overlay />
{:else if isFirstRun}
  <Onboarding on:complete={handleOnboardingComplete} />
{:else}
  <main class="app">
    <nav class="sidebar">
      <div class="logo">
        <img src="/logo/kalam-logo-icon.svg" alt="Kalam" class="logo-icon" />
        <h1>Kalam</h1>
      </div>
      <ul class="nav-links">
        <li class:active={currentPage === 'settings'}>
          <button on:click={() => navigate('settings')}>Settings</button>
        </li>
        <li class:active={currentPage === 'history'}>
          <button on:click={() => navigate('history')}>History</button>
        </li>
        <li class:active={currentPage === 'snippets'}>
          <button on:click={() => navigate('snippets')}>Snippets</button>
        </li>
      </ul>
      <div class="footer">
        <p>Press <kbd>{$sidebarDictationStore ? displayHotkey($sidebarDictationStore.hotkey, $sidebarDictationStore.platform) : 'Ctrl+Win'}</kbd> to dictate</p>
        {#if $sidebarDictationStore && $sidebarDictationStore.languages.length >= 2 && $sidebarDictationStore.languageToggleHotkey}
          <p>Press <kbd>{displayHotkey($sidebarDictationStore.languageToggleHotkey, $sidebarDictationStore.platform)}</kbd> to switch language</p>
        {/if}
      </div>
    </nav>

    <div class="content">
      {#if currentPage === 'settings'}
        <Settings />
      {:else if currentPage === 'history'}
        <History />
      {:else if currentPage === 'snippets'}
        <Snippets />
      {/if}
    </div>
  </main>
{/if}

<style>
  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Inter', 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    background: var(--bg-dark);
    color: var(--text-primary);
  }

  .app {
    display: flex;
    min-height: 100vh;
    height: 100vh;
    background: var(--bg-dark);
  }

  .sidebar {
    width: 240px;
    background: var(--bg-dark);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 24px;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 32px;
    padding-bottom: 20px;
  }

  .logo-icon {
    width: 32px;
    height: 32px;
    flex-shrink: 0;
  }

  .logo h1 {
    font-size: 20px;
    font-weight: 700;
    color: var(--navy-deep);
    margin: 0;
    letter-spacing: -0.5px;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 40px 48px;
    background: var(--bg-content);
  }

  .nav-links {
    list-style: none;
    flex: 1;
  }

  .nav-links li {
    margin-bottom: 4px;
  }

  .nav-links button {
    width: 100%;
    padding: 10px 14px;
    background: transparent;
    border: none;
    border-radius: 8px;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .nav-links button:hover {
    background: var(--bg-input);
    color: var(--text-primary);
  }

  .nav-links li.active button {
    background: var(--primary-alpha);
    color: var(--primary-dark);
    font-weight: 600;
  }

  .footer {
    padding-top: 20px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .footer p + p {
    margin-top: 12px;
  }

  .footer kbd {
    background: var(--bg-content);
    padding: 3px 6px;
    border-radius: 4px;
    font-family: monospace;
    font-size: 11px;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    box-shadow: 0 1px 2px rgba(0,0,0,0.05);
  }

</style>
