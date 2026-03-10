<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { initTelemetry } from './lib/telemetry'
  import { sidebarDictationStore } from './lib/sidebarDictation'
  import Settings from './pages/Settings.svelte'
  import Home from './pages/Home.svelte'
  import Snippets from './pages/Snippets.svelte'
  import Onboarding from './pages/Onboarding.svelte'
  import Overlay from './components/Overlay.svelte'
  import History from './components/views/History.svelte'
  import Notes from './components/views/Notes.svelte'
  import Tasks from './components/views/Tasks.svelte'
  import Reminders from './components/views/Reminders.svelte'
  import StatusBar from './components/StatusBar.svelte'
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
  let sidebarCollapsed = false
  let statusBarConfig: AppConfig | null = null
  let dbStatus: { ok: boolean } | null = null
  let statusBarPlatform = ''
  let lastLatencyMs: number | null = null

  onMount(() => {
    if (isOverlay) return
    let unlistenReset: (() => void) | null = null
    let unlistenTrayNavigate: (() => void) | null = null
    let unlistenSettings: (() => void) | null = null
    let unlistenTranscription: (() => void) | null = null
    let dbPollId: ReturnType<typeof setInterval> | null = null
    ;(async () => {
      unlistenReset = await listen('app_reset', () => {
        isFirstRun = true
      })
      unlistenTrayNavigate = await listen<string>('tray-navigate', (e) => {
        if (e.payload && ['settings', 'history', 'snippets'].includes(e.payload)) {
          navigate(e.payload)
        }
      })
      unlistenSettings = await listen<AppConfig>('settings_updated', (e) => {
        if (e.payload) {
          statusBarConfig = e.payload
          if (e.payload.sidebar_collapsed != null) sidebarCollapsed = e.payload.sidebar_collapsed
        }
      })
      unlistenTranscription = await listen<{ latency_ms?: number }>('transcription-saved', (e) => {
        if (e.payload?.latency_ms != null) lastLatencyMs = e.payload.latency_ms
      })
      try {
        const config = (await invoke('get_settings')) as AppConfig
        isFirstRun = !config.onboarding_complete
        dictationEnabled = config.dictation_enabled ?? true
        sidebarCollapsed = config.sidebar_collapsed ?? false
        initTelemetry(config.privacy?.telemetry_enabled ?? false)
        const platform = (await invoke('get_platform')) as string
        statusBarConfig = config
        statusBarPlatform = platform
        sidebarDictationStore.updateFromConfig(config, platform)
        try {
          dbStatus = (await invoke('get_db_status')) as { ok: boolean }
        } catch {
          dbStatus = { ok: false }
        }
        try {
          const stats = (await invoke('get_aggregate_stats')) as { last_latency_ms?: number | null }
          if (stats.last_latency_ms != null) lastLatencyMs = stats.last_latency_ms
        } catch {
          // ignore
        }
        dbPollId = setInterval(async () => {
          try {
            dbStatus = (await invoke('get_db_status')) as { ok: boolean }
          } catch {
            dbStatus = { ok: false }
          }
        }, 30000)
      } catch {
        isFirstRun = true
      }
    })()
    return () => {
      if (unlistenReset) unlistenReset()
      if (unlistenTrayNavigate) unlistenTrayNavigate()
      if (unlistenSettings) unlistenSettings()
      if (unlistenTranscription) unlistenTranscription()
      if (dbPollId != null) clearInterval(dbPollId)
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
      sidebarCollapsed = config.sidebar_collapsed ?? false
      const platform = (await invoke('get_platform')) as string
      statusBarConfig = config
      statusBarPlatform = platform
      sidebarDictationStore.updateFromConfig(config, platform)
      try {
        dbStatus = (await invoke('get_db_status')) as { ok: boolean }
      } catch {
        dbStatus = { ok: false }
      }
    } catch {
      // keep store as-is
    }
  }

  async function setSidebarCollapsed(next: boolean) {
    if (sidebarCollapsed === next) return
    sidebarCollapsed = next
    try {
      const config = (await invoke('get_settings')) as AppConfig
      await invoke('save_settings', { newConfig: { ...config, sidebar_collapsed: next } })
    } catch (e) {
      console.error('Failed to save sidebar state:', e)
      sidebarCollapsed = !next
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
  <div class="app-shell">
    <main class="app">
    <nav class="sidebar" class:collapsed={sidebarCollapsed}>
      <div class="sidebar-header">
        <div class="logo">
          <img src="/logo/kalam-logo-icon.svg" alt="Kalam" class="logo-icon" />
          <h1>Kalam</h1>
        </div>
      </div>
      <button
        type="button"
        class="sidebar-toggle"
        title={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        on:click={() => setSidebarCollapsed(!sidebarCollapsed)}
        aria-label={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
      >
        <Icon icon={sidebarCollapsed ? 'ph:caret-right-duotone' : 'ph:caret-left-duotone'} />
      </button>
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
        <div
          class="dictation-control"
          class:collapsed-toggle={sidebarCollapsed}
          title={sidebarCollapsed ? (dictationEnabled ? 'Dictation on — click to turn off' : 'Dictation off — click to turn on') : 'Turn dictation and hotkeys on or off'}
          role={sidebarCollapsed ? 'button' : undefined}
          tabindex={sidebarCollapsed ? 0 : undefined}
          on:click={sidebarCollapsed ? () => setDictation(!dictationEnabled) : undefined}
          on:keydown={sidebarCollapsed ? (e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setDictation(!dictationEnabled); } } : undefined}
        >
          <div class="dictation-info" class:dictation-off={!dictationEnabled}>
            <Icon icon="ph:microphone-stage-duotone" class="nav-icon" />
            <span class="nav-text">Dictation</span>
          </div>
          <div class="tab-selector">
            <button type="button" class="tab" class:active={dictationEnabled} on:click={() => setDictation(true)}>On</button>
            <button type="button" class="tab" class:active={!dictationEnabled} on:click={() => setDictation(false)}>Off</button>
          </div>
        </div>
        
        <ul class="nav-links settings-link">
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
        <Home navigate={navigate} />
      {:else if currentPage === 'settings'}
        <Settings />
      {:else if currentPage === 'snippets'}
        <Snippets />
      {:else if currentPage === 'history'}
        <History />
      {:else if currentPage === 'notes'}
        <Notes />
      {:else if currentPage === 'tasks'}
        <Tasks navigate={navigate} />
      {:else if currentPage === 'reminders'}
        <Reminders onNavigateToPage={navigate} />
      {/if}
    </div>
  </main>
  <StatusBar config={statusBarConfig} dbStatus={dbStatus} platform={statusBarPlatform} lastLatencyMs={lastLatencyMs} />
  </div>
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

  .app-shell {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    height: 100vh;
    background: var(--bg-app);
  }

  .app {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
    gap: 16px;
    padding: 16px;
    background: var(--bg-app);
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
    transition: width 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .sidebar.collapsed {
    width: 80px;
    padding: 24px 12px;
    align-items: center;
  }

  .sidebar.collapsed .sidebar-header {
    margin-bottom: 32px;
    justify-content: center;
    width: 100%;
  }

  .sidebar.collapsed .logo {
    gap: 0;
    justify-content: center;
    width: 100%;
  }

  .sidebar.collapsed .logo h1,
  .sidebar.collapsed .nav-text,
  .sidebar.collapsed .dictation-info .nav-text,
  .sidebar.collapsed .tab-selector {
    display: none;
  }

  .sidebar.collapsed .nav-links {
    width: 100%;
    align-items: center;
  }

  .sidebar.collapsed .nav-links li {
    width: 100%;
    display: flex;
    justify-content: center;
  }

  .sidebar.collapsed .nav-links button,
  .sidebar.collapsed .dictation-control,
  .sidebar.collapsed .settings-link button {
    justify-content: center;
    padding: 12px;
    width: 48px;
    height: 48px;
    border-radius: 12px;
  }

  .sidebar.collapsed .nav-links button :global(.nav-icon) {
    margin: 0;
  }

  .sidebar.collapsed .dictation-control {
    flex-direction: column;
    padding: 12px;
    width: 48px;
    height: 48px;
    justify-content: center;
    align-items: center;
  }

  .sidebar.collapsed .dictation-info {
    gap: 0;
  }

  .dictation-control.collapsed-toggle {
    cursor: pointer;
  }

  .dictation-control.collapsed-toggle:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: 2px;
  }

  .dictation-control.collapsed-toggle:not(:focus-visible) {
    outline: none;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 40px;
  }

  .sidebar-toggle {
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    width: 20px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    opacity: 0.45;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: opacity 0.2s, color 0.2s, background 0.2s;
    border-radius: 0 var(--radius-xl) var(--radius-xl) 0;
  }

  .sidebar-toggle:hover {
    opacity: 1;
    color: var(--navy-deep);
    background: var(--bg-input);
  }

  .sidebar-toggle :global(svg) {
    font-size: 16px;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 14px;
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
    padding: 20px 24px;
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
    width: 100%;
  }

  .sidebar.collapsed .sidebar-bottom {
    align-items: center;
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

  .dictation-info.dictation-off :global(.nav-icon),
  .dictation-info.dictation-off {
    opacity: 0.45;
    color: var(--text-muted);
  }

  .dictation-info.dictation-off :global(.nav-icon) {
    filter: grayscale(0.6);
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

    .sidebar.collapsed {
      width: 72px;
      padding: 20px 12px;
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
      padding: 12px 16px;
    }
  }

</style>
