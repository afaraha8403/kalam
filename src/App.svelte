<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { initTelemetry } from './lib/telemetry'
  import { sidebarDictationStore, displayHotkey } from './lib/sidebarDictation'
  import Settings from './pages/Settings.svelte'
  import Home from './pages/Home.svelte'
  import Snippets from './pages/Snippets.svelte'
  import About from './pages/About.svelte'
  import Onboarding from './pages/Onboarding.svelte'
  import Overlay from './components/Overlay.svelte'
  import History from './components/views/History.svelte'
  import Notes from './components/views/Notes.svelte'
  import Tasks from './components/views/Tasks.svelte'
  import Reminders from './components/views/Reminders.svelte'
  import Icon from '@iconify/svelte'
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

  let currentPage = 'home'
  let isFirstRun = true
  let dictationEnabled = true

  onMount(() => {
    if (isOverlay) return
    let unlistenReset: (() => void) | null = null
    ;(async () => {
      unlistenReset = await listen('app_reset', () => {
        isFirstRun = true
      })
      try {
        const config = (await invoke('get_settings')) as AppConfig
        isFirstRun = !config.onboarding_complete
        dictationEnabled = config.dictation_enabled ?? true
        initTelemetry(config.privacy?.telemetry_enabled ?? false)
        const platform = (await invoke('get_platform')) as string
        sidebarDictationStore.updateFromConfig(config, platform)
      } catch {
        isFirstRun = true
      }
    })()
    return () => {
      if (unlistenReset) unlistenReset()
    }
  })

  function navigate(page: string) {
    currentPage = page
  }

  async function handleOnboardingComplete() {
    isFirstRun = false
    try {
      const config = (await invoke('get_settings')) as AppConfig
      dictationEnabled = config.dictation_enabled ?? true
      const platform = (await invoke('get_platform')) as string
      sidebarDictationStore.updateFromConfig(config, platform)
    } catch {
      // keep store as-is
    }
  }

  async function setDictation(next: boolean) {
    if (dictationEnabled === next) return;
    dictationEnabled = next;
    try {
      const config = (await invoke('get_settings')) as AppConfig;
      await invoke('save_settings', { newConfig: { ...config, dictation_enabled: next } });
    } catch (e) {
      console.error('Failed to save dictation state:', e);
      dictationEnabled = !next;
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
        <li class:active={currentPage === 'home'}>
          <button on:click={() => navigate('home')} title="Home">
            <Icon icon="ph:house-duotone" class="nav-icon" />
            <span class="nav-text">Home</span>
          </button>
        </li>
        <li class:active={currentPage === 'snippets'}>
          <button on:click={() => navigate('snippets')} title="Snippets">
            <Icon icon="ph:scissors-duotone" class="nav-icon" />
            <span class="nav-text">Snippets</span>
          </button>
        </li>
        <li class:active={currentPage === 'history'}>
          <button on:click={() => navigate('history')} title="History">
            <Icon icon="ph:clock-counter-clockwise-duotone" class="nav-icon" />
            <span class="nav-text">History</span>
          </button>
        </li>
        <li class:active={currentPage === 'notes'}>
          <button on:click={() => navigate('notes')} title="Notes">
            <Icon icon="ph:note-duotone" class="nav-icon" />
            <span class="nav-text">Notes</span>
          </button>
        </li>
        <li class:active={currentPage === 'tasks'}>
          <button on:click={() => navigate('tasks')} title="Tasks">
            <Icon icon="ph:check-square-duotone" class="nav-icon" />
            <span class="nav-text">Tasks</span>
          </button>
        </li>
        <li class:active={currentPage === 'reminders'}>
          <button on:click={() => navigate('reminders')} title="Reminders">
            <Icon icon="ph:bell-duotone" class="nav-icon" />
            <span class="nav-text">Reminders</span>
          </button>
        </li>
      </ul>
      
      <div class="sidebar-bottom">
        <div class="footer">
          <p>Press <kbd>{$sidebarDictationStore ? displayHotkey($sidebarDictationStore.hotkey, $sidebarDictationStore.platform) : 'Ctrl+Win'}</kbd> to dictate</p>
          {#if $sidebarDictationStore && $sidebarDictationStore.languages.length >= 2 && $sidebarDictationStore.languageToggleHotkey}
            <p>Press <kbd>{displayHotkey($sidebarDictationStore.languageToggleHotkey, $sidebarDictationStore.platform)}</kbd> to switch language</p>
          {/if}
        </div>

        <div class="dictation-control" title="Turn dictation and hotkeys on or off">
          <div class="dictation-info">
            <Icon icon="ph:microphone-stage-duotone" class="nav-icon" />
            <span class="nav-text">Dictation</span>
          </div>
          <div class="tab-selector">
            <button type="button" class="tab" class:active={dictationEnabled} on:click={() => setDictation(true)}>On</button>
            <button type="button" class="tab" class:active={!dictationEnabled} on:click={() => setDictation(false)}>Off</button>
          </div>
        </div>
        
        <ul class="nav-links settings-link">
          <li class:active={currentPage === 'about'}>
            <button on:click={() => navigate('about')} title="About">
              <Icon icon="ph:info-duotone" class="nav-icon" />
              <span class="nav-text">About</span>
            </button>
          </li>
          <li class:active={currentPage === 'settings'}>
            <button on:click={() => navigate('settings')} title="Settings">
              <Icon icon="ph:gear-duotone" class="nav-icon" />
              <span class="nav-text">Settings</span>
            </button>
          </li>
        </ul>
      </div>
    </nav>

    <div class="content">
      {#if currentPage === 'home'}
        <Home />
      {:else if currentPage === 'settings'}
        <Settings />
      {:else if currentPage === 'snippets'}
        <Snippets />
      {:else if currentPage === 'about'}
        <About />
      {:else if currentPage === 'history'}
        <History />
      {:else if currentPage === 'notes'}
        <Notes />
      {:else if currentPage === 'tasks'}
        <Tasks />
      {:else if currentPage === 'reminders'}
        <Reminders />
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
    font-family: 'DM Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    background: var(--bg-app);
    color: var(--text-primary);
  }

  .app {
    display: flex;
    min-height: 100vh;
    height: 100vh;
    background: var(--bg-app);
    padding: 16px;
    gap: 16px;
  }

  .sidebar {
    width: 260px;
    background: var(--bg-card);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-sm);
    display: flex;
    flex-direction: column;
    padding: 28px 24px;
    position: relative;
    overflow: hidden;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 40px;
    padding-bottom: 0;
  }

  .logo-icon {
    width: 36px;
    height: 36px;
    flex-shrink: 0;
  }

  .logo h1 {
    font-family: 'Syne', sans-serif;
    font-size: 24px;
    font-weight: 700;
    color: var(--navy-deep);
    margin: 0;
    letter-spacing: -0.03em;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    background: var(--bg-content);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-md);
    padding: 48px 56px;
    position: relative;
  }

  .nav-links {
    list-style: none;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .nav-links li {
    margin-bottom: 0;
  }

  .nav-links button {
    width: 100%;
    padding: 12px 16px;
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: 15px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  :global(.nav-icon) {
    font-size: 20px;
    color: var(--text-muted);
    transition: color 0.2s;
  }

  .nav-links button:hover {
    background: var(--bg-input);
    color: var(--navy-deep);
    transform: translateX(4px);
  }

  .nav-links button:hover :global(.nav-icon) {
    color: var(--navy-deep);
  }

  .nav-links li.active button {
    background: var(--primary-alpha);
    color: var(--primary-dark);
    font-weight: 600;
  }

  .nav-links li.active button :global(.nav-icon) {
    color: var(--primary-dark);
  }

  .nav-links li.active button::before {
    content: '';
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    height: 60%;
    width: 3px;
    background: var(--primary);
    border-radius: 0 4px 4px 0;
  }

  .sidebar-bottom {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .dictation-control {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px 8px 16px;
    background: transparent;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    transition: background 0.2s;
  }

  .dictation-control:hover {
    background: var(--bg-input);
  }

  .dictation-info {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 15px;
    font-weight: 500;
  }

  .tab-selector {
    display: flex;
    background: var(--bg-app);
    border-radius: var(--radius-sm);
    padding: 3px;
    gap: 2px;
    border: 1px solid var(--border-subtle);
  }

  .tab-selector .tab {
    border: none;
    background: transparent;
    padding: 4px 10px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    border-radius: calc(var(--radius-sm) - 2px);
    cursor: pointer;
    transition: all 0.2s;
  }

  .tab-selector .tab:hover:not(.active) {
    color: var(--text-primary);
  }

  .tab-selector .tab.active {
    background: var(--bg-card);
    color: var(--navy-deep);
    box-shadow: var(--shadow-sm);
  }

  .settings-link {
    flex: none;
  }

  .footer {
    padding-top: 24px;
    font-size: 13px;
    color: var(--text-muted);
    border-top: 1px solid var(--border-subtle);
  }

  .footer p + p {
    margin-top: 14px;
  }

  .footer kbd {
    background: var(--bg-input);
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    font-family: 'DM Sans', monospace;
    font-size: 12px;
    font-weight: 600;
    border: 1px solid var(--border);
    color: var(--navy-deep);
    box-shadow: var(--shadow-inner);
  }

  @media (max-width: 768px) {
    .app {
      padding: 12px;
      gap: 12px;
    }

    .sidebar {
      width: 80px;
      padding: 24px 12px;
      align-items: center;
    }

    .logo {
      margin-bottom: 32px;
      margin-right: 0;
      justify-content: center;
    }

    .logo h1 {
      display: none; /* Hide text on small screens to save space */
    }

    .nav-links {
      width: 100%;
    }

    .nav-links button {
      padding: 12px;
      justify-content: center;
    }

    .nav-text {
      display: none;
    }

    :global(.nav-icon) {
      font-size: 24px;
      margin: 0;
    }

    .nav-links li.active button::before {
      height: 40%;
      width: 4px;
      border-radius: 0 4px 4px 0;
    }

    .sidebar-bottom {
      width: 100%;
      align-items: center;
    }

    .footer {
      display: none; /* Hide hotkey hints on small screens */
    }

    .dictation-control {
      padding: 8px 4px;
      flex-direction: column;
      gap: 10px;
      margin-bottom: 12px;
      width: 100%;
    }

    .dictation-info .nav-text {
      display: none;
    }

    .dictation-info {
      gap: 0;
    }

    .tab-selector {
      padding: 2px;
      width: 100%;
      flex-direction: column;
    }

    .tab-selector .tab {
      padding: 6px 4px;
      font-size: 11px;
      text-align: center;
    }

    .content {
      padding: 24px 20px;
    }
  }

</style>
