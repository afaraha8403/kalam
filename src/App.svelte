<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '$lib/backend'
  import { listen } from '@tauri-apps/api/event'
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { initTelemetry } from './lib/telemetry'
  import { sidebarDictationStore } from './lib/sidebarDictation'
  import { applyOverlayBroadcast } from './lib/dictationState'
  import Settings from './pages/Settings.svelte'
  import Dictation from './pages/Dictation.svelte'
  import Home from './pages/Home.svelte'
  import Snippets from './pages/Snippets.svelte'
  import Dictionary from './pages/Dictionary.svelte'
  import Onboarding from './pages/Onboarding.svelte'
  import Overlay from './components/Overlay.svelte'
  import History from './components/views/History.svelte'
  import HistoryDetail from './components/views/HistoryDetail.svelte'
  import Notes from './components/views/Notes.svelte'
  import NoteDetail from './components/views/NoteDetail.svelte'
  import Tasks from './components/views/Tasks.svelte'
  import TaskDetail from './components/views/TaskDetail.svelte'
  import Reminders from './components/views/Reminders.svelte'
  import StatusBar from './components/StatusBar.svelte'
  import Prototype from './pages/Prototype.svelte'
  import Icon from '@iconify/svelte'
  import type { AppConfig, ThemePreference } from './types'

  /** Show alternate prototype UI when URL has ?page=prototype (main interface unchanged). */
  const showPrototype =
    typeof window !== 'undefined' &&
    new URLSearchParams(window.location.search).get('page') === 'prototype'

  /** True when running in a normal browser (e.g. Vite dev server), not inside Tauri. */
  const inTauri = typeof window !== 'undefined' && !!(window as { __TAURI__?: unknown }).__TAURI__
  /** When set, force-skip onboarding in Tauri (for testing). In browser we always skip. */
  const skipOnboardingParam =
    typeof window !== 'undefined' && new URLSearchParams(window.location.search).get('skipOnboarding') === '1'

  /** Cypress / browser: show onboarding when dev bridge (or mocks) back invoke. */
  const e2eOnboarding =
    typeof window !== 'undefined' &&
    new URLSearchParams(window.location.search).get('e2eOnboarding') === '1'

  /** Minimal config for rendering the main app in browser (no backend). */
  function defaultBrowserConfig(): AppConfig {
    return {
      onboarding_complete: true,
      hotkey: null,
      toggle_dictation_hotkey: null,
      dictation_enabled: true,
      sidebar_collapsed: false,
      audio_device: null,
      stt_config: {
        mode: 'Cloud',
        provider: 'groq',
        api_keys: {},
        local_model: null,
        vad_preset: 'Balanced',
        audio_filter: {
          enabled: false,
          preset: 'Off',
          highpass_cutoff_hz: 80,
          noise_gate_threshold_db: -45,
          compressor_ratio: 3,
          compressor_threshold_db: -18,
          normalize_target_db: -6,
        },
      },
      formatting: {
        voice_commands: true,
        filler_word_removal: true,
        auto_punctuation: true,
        custom_rules: [],
        injection_method: 'Auto',
        keystroke_delay_ms: 10,
        clipboard_threshold: 200,
        force_clipboard_apps: [],
        app_injection_rules: [
          {
            process_name: 'notepad.exe',
            display_name: 'Notepad',
            method: 'Clipboard',
            keystroke_delay_ms: null,
            clipboard_threshold: null,
          },
        ],
      },
      privacy: {
        history_retention_days: 90,
        telemetry_enabled: false,
        sensitive_app_detection: false,
        sensitive_app_patterns: []
      },
      notifications: {
        show_completion: true,
        show_errors: true,
        show_updates: true,
        sound_enabled: false
      },
      logging: { enabled: false, level: 'Info', max_records: 2000 },
      snippets: [],
      auto_start: false,
      languages: ['en'],
      language_toggle_hotkey: null,
      start_in_focus: true,
      min_hold_ms: 300,
      command_config: {
        enabled: false,
        hotkey: null,
      },
      provider_keys: {},
      default_llm_provider: null,
      default_llm_model: null,
      custom_openai_endpoint: null,
      theme_preference: 'Auto',
      modes: [
        {
          id: 'default',
          name: 'Default',
          icon: 'ph:microphone',
          accent_color: 'oklch(68% 0.1 240)',
          ai_instructions: '',
          voice_model: { provider: 'groq', model_id: '' },
          language_model: { provider: '', model_id: '' },
          polish: false,
          context: {
            enabled: false,
            read_app: false,
            read_clipboard: false,
            read_selection: false,
            include_system_info: false
          },
          auto_activate_rules: [],
          is_builtin: true,
          is_deletable: false,
          created_at: '',
          updated_at: ''
        }
      ],
      active_mode_id: 'default',
      polish_enabled: false,
      polish_config: {
        fix_grammar: true,
        remove_filler: true,
        fix_punctuation: true,
        smart_formatting: true,
        self_correction: true
      },
      context_awareness_enabled: false,
      mode_cycle_hotkey: 'Ctrl+Shift+M'
    }
  }

  function normalizeThemePreference(v: unknown): ThemePreference {
    if (v === 'Light' || v === 'Dark' || v === 'Auto') return v
    return 'Auto'
  }

  function readSystemPrefersDark(): boolean {
    if (typeof window === 'undefined') return true
    return window.matchMedia('(prefers-color-scheme: dark)').matches
  }

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
  /** null = config not yet loaded; avoids flashing onboarding for returning users. */
  let isFirstRun: boolean | null = null
  let dictationEnabled = true
  let sidebarCollapsed = false
  let statusBarConfig: AppConfig | null = null
  let dbStatus: { ok: boolean } | null = null
  let statusBarPlatform = ''
  let lastLatencyMs: number | null = null

  /** Persisted via `theme_preference`; when `Auto`, tracks OS `prefers-color-scheme`. */
  let themePreference: ThemePreference = 'Auto'
  let systemPrefersDark = readSystemPrefersDark()

  $: darkMode =
    themePreference === 'Dark' ? true : themePreference === 'Light' ? false : systemPrefersDark

  async function setThemePreference(next: ThemePreference) {
    if (themePreference === next) return
    const prev = themePreference
    themePreference = next
    if (!inTauri) {
      if (statusBarConfig) statusBarConfig = { ...statusBarConfig, theme_preference: next }
      return
    }
    try {
      const config = (await invoke('get_settings')) as AppConfig
      const merged = { ...config, theme_preference: next }
      await invoke('save_settings', { newConfig: merged })
      statusBarConfig = merged
    } catch (e) {
      console.error('Failed to save theme preference:', e)
      themePreference = prev
    }
  }

  /** One control cycles Light → Dark → Auto (saved in app config via `setThemePreference`). */
  function cycleThemePreference() {
    const order: ThemePreference[] = ['Light', 'Dark', 'Auto']
    const i = order.indexOf(themePreference)
    const next = order[(i + 1) % order.length]
    void setThemePreference(next)
  }

  // Icon reflects the selected mode (not “what you see” when Auto); Auto uses combined light/dark glyph, not a display/device.
  $: themeToggleIcon =
    themePreference === 'Light' ? 'ph:sun' : themePreference === 'Dark' ? 'ph:moon' : 'mdi:theme-light-dark'

  $: themeToggleTitle =
    themePreference === 'Light'
      ? 'Light — always light. Click to toggle (next: Dark).'
      : themePreference === 'Dark'
        ? 'Dark — always dark. Click to toggle (next: Auto).'
        : `Auto — follows system (looks ${darkMode ? 'dark' : 'light'} now). Click to toggle (next: Light).`

  /** ApexCharts tooltips render on `body`; CSS vars live under `.kalam-sleek` so we mirror theme on `html`. */
  $: if (typeof document !== 'undefined' && !isOverlay) {
    document.documentElement.setAttribute('data-theme', darkMode ? 'dark' : 'light')
  }

  onMount(() => {
    if (isOverlay) return

    let unlistenThemeMq: (() => void) | null = null
    if (typeof window !== 'undefined') {
      const mq = window.matchMedia('(prefers-color-scheme: dark)')
      const onSchemeChange = () => {
        systemPrefersDark = mq.matches
      }
      mq.addEventListener('change', onSchemeChange)
      unlistenThemeMq = () => mq.removeEventListener('change', onSchemeChange)
    }

    // Browser (no Tauri): skip onboarding and use defaults so the main app can be debugged.
    if (!inTauri) {
      if (e2eOnboarding) {
        isFirstRun = true
        return () => {
          unlistenThemeMq?.()
        }
      }
      isFirstRun = false
      const browserCfg = defaultBrowserConfig()
      statusBarConfig = browserCfg
      statusBarPlatform = 'web'
      sidebarDictationStore.updateFromConfig(browserCfg, 'web')
      themePreference = normalizeThemePreference(browserCfg.theme_preference)
      return () => {
        unlistenThemeMq?.()
      }
    }

    let unlistenReset: (() => void) | null = null
    let unlistenTrayNavigate: (() => void) | null = null
    let unlistenSettings: (() => void) | null = null
    let unlistenTranscription: (() => void) | null = null
    let unlistenOverlayBroadcast: (() => void) | null = null
    let dbPollId: ReturnType<typeof setInterval> | null = null
    ;(async () => {
      unlistenReset = await listen('app_reset', () => {
        isFirstRun = true
      })
      unlistenTrayNavigate = await listen<string>('tray-navigate', (e) => {
        if (
          e.payload &&
          [
            'settings',
            'history',
            'dictionary',
            'snippets',
            'dictation',
          ].includes(e.payload)
        ) {
          navigate(e.payload)
        }
      })
      unlistenSettings = await listen<AppConfig>('settings_updated', (e) => {
        if (!e.payload) return
        statusBarConfig = e.payload
        if (e.payload.sidebar_collapsed != null) sidebarCollapsed = e.payload.sidebar_collapsed
        if (e.payload.dictation_enabled != null) dictationEnabled = e.payload.dictation_enabled
        if (e.payload.theme_preference != null) {
          themePreference = normalizeThemePreference(e.payload.theme_preference)
        }
        // Keep dictation hotkey labels in sync with Settings (same source as initial load).
        const syncSidebar = async () => {
          let p = statusBarPlatform
          if (!p) {
            try {
              p = (await invoke('get_platform')) as string
            } catch {
              p = 'windows'
            }
          }
          sidebarDictationStore.updateFromConfig(e.payload!, p)
        }
        void syncSidebar()
      })
      unlistenTranscription = await listen<{ latency_ms?: number }>('transcription-saved', (e) => {
        if (e.payload?.latency_ms != null) lastLatencyMs = e.payload.latency_ms
      })
      unlistenOverlayBroadcast = await listen<unknown>('overlay-state-broadcast', (e) => {
        applyOverlayBroadcast(e.payload)
      })
      try {
        const config = (await invoke('get_settings')) as AppConfig
        isFirstRun = skipOnboardingParam ? false : !config.onboarding_complete
        dictationEnabled = config.dictation_enabled ?? true
        sidebarCollapsed = config.sidebar_collapsed ?? false
        themePreference = normalizeThemePreference(config.theme_preference)
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
      unlistenThemeMq?.()
      if (unlistenReset) unlistenReset()
      if (unlistenTrayNavigate) unlistenTrayNavigate()
      if (unlistenSettings) unlistenSettings()
      if (unlistenTranscription) unlistenTranscription()
      if (unlistenOverlayBroadcast) unlistenOverlayBroadcast()
      if (dbPollId != null) clearInterval(dbPollId)
    }
  })

  async function refreshDbStatus() {
    try {
      dbStatus = (await invoke('get_db_status')) as { ok: boolean }
    } catch {
      dbStatus = { ok: false }
    }
  }

  function navigate(page: string) {
    currentPage = page
  }

  async function handleOnboardingComplete() {
    isFirstRun = false
    try {
      const config = (await invoke('get_settings')) as AppConfig
      dictationEnabled = config.dictation_enabled ?? true
      sidebarCollapsed = config.sidebar_collapsed ?? false
      themePreference = normalizeThemePreference(config.theme_preference)
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
{:else if isFirstRun === null}
  <!-- Config still loading — blank themed shell prevents onboarding flash for returning users. -->
  <div class="kalam-sleek" class:dark={darkMode} class:light={!darkMode}></div>
{:else if isFirstRun}
  <div class="kalam-sleek onboarding-shell" class:dark={darkMode} class:light={!darkMode}>
    <Onboarding on:complete={handleOnboardingComplete} />
  </div>
{:else if showPrototype}
  <Prototype
    currentPage={currentPage}
    navigate={navigate}
    dictationEnabled={dictationEnabled}
    setDictation={setDictation}
    statusBarConfig={statusBarConfig}
    dbStatus={dbStatus}
    statusBarPlatform={statusBarPlatform}
    lastLatencyMs={lastLatencyMs}
    onRetryDb={refreshDbStatus}
  />
{:else}
  <div class="app-shell kalam-sleek" class:dark={darkMode} class:light={!darkMode}>
      <div class="sleek-body-row">
      <aside class="sidebar" class:collapsed={sidebarCollapsed}>
        <button
          type="button"
          class="sidebar-toggle"
          title={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          on:click={() => setSidebarCollapsed(!sidebarCollapsed)}
          aria-label={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          <Icon icon={sidebarCollapsed ? 'ph:caret-right-duotone' : 'ph:caret-left-duotone'} />
        </button>
        <div class="sidebar-content">
          <div class="logo-section">
            <div class="logo">
              <!-- Same asset as index.html favicon; full wordmark stays in .logo-text when sidebar expanded. -->
              <img
                src="/logo/kalam-logo-icon.svg"
                alt={sidebarCollapsed ? 'Kalam' : ''}
                class="logo-img"
                width="28"
                height="28"
              />
              <span class="logo-text">Kalam</span>
            </div>
          </div>

          <nav class="main-nav">
            <button class="nav-link" class:active={currentPage === 'home'} on:click={() => navigate('home')} title="Overview">
              <Icon icon={currentPage === 'home' ? 'ph:squares-four-fill' : 'ph:squares-four'} />
              <span class="nav-text">Overview</span>
            </button>
            <button class="nav-link" class:active={currentPage === 'dictation'} on:click={() => navigate('dictation')} title="Dictation modes">
              <Icon icon={currentPage === 'dictation' ? 'ph:microphone-stage-fill' : 'ph:microphone-stage'} />
              <span class="nav-text">Dictation</span>
            </button>
            <button class="nav-link" class:active={currentPage === 'history'} on:click={() => navigate('history')} title="History">
              <Icon icon={currentPage === 'history' ? 'ph:clock-fill' : 'ph:clock'} />
              <span class="nav-text">History</span>
            </button>
            <button class="nav-link" class:active={currentPage === 'notes'} on:click={() => navigate('notes')} title="Notes">
              <Icon icon={currentPage === 'notes' ? 'ph:notebook-fill' : 'ph:notebook'} />
              <span class="nav-text">Notes</span>
            </button>
            <button class="nav-link" class:active={currentPage === 'tasks'} on:click={() => navigate('tasks')} title="Tasks">
              <Icon icon={currentPage === 'tasks' ? 'ph:check-circle-fill' : 'ph:check-circle'} />
              <span class="nav-text">Tasks</span>
            </button>
            <button class="nav-link" class:active={currentPage === 'reminders'} on:click={() => navigate('reminders')} title="Reminders">
              <Icon icon={currentPage === 'reminders' ? 'ph:bell-fill' : 'ph:bell'} />
              <span class="nav-text">Reminders</span>
            </button>
            <button class="nav-link" class:active={currentPage === 'dictionary'} on:click={() => navigate('dictionary')} title="Dictionary">
              <Icon icon={currentPage === 'dictionary' ? 'ph:book-open-fill' : 'ph:book-open'} />
              <span class="nav-text">Dictionary</span>
            </button>
            <button class="nav-link" class:active={currentPage === 'snippets'} on:click={() => navigate('snippets')} title="Snippets">
              <Icon icon={currentPage === 'snippets' ? 'ph:text-aa-fill' : 'ph:text-aa'} />
              <span class="nav-text">Snippets</span>
            </button>
          </nav>

          <div class="sidebar-bottom">
            <div class="bottom-links">
              <button
                type="button"
                class="icon-btn dictation-toggle"
                class:dictation-off={!dictationEnabled}
                title={dictationEnabled ? 'Microphone — dictation on. Click to turn off.' : 'Microphone — dictation off. Click to turn on.'}
                aria-label={dictationEnabled ? 'Dictation on, click to turn off' : 'Dictation off, click to turn on'}
                aria-pressed={dictationEnabled}
                on:click={() => setDictation(!dictationEnabled)}
              >
                <Icon icon={dictationEnabled ? 'ph:microphone-fill' : 'ph:microphone'} />
              </button>
              <button
                type="button"
                class="icon-btn"
                title={themeToggleTitle}
                aria-label={themeToggleTitle}
                on:click={cycleThemePreference}
              >
                <Icon icon={themeToggleIcon} />
              </button>
              <button
                type="button"
                class="icon-btn"
                class:active={currentPage === 'settings'}
                title="Settings — click to open."
                aria-label="Settings, click to open"
                on:click={() => navigate('settings')}
              >
                <Icon icon="ph:gear" />
              </button>
            </div>
          </div>
        </div>
      </aside>

      <main class="main">
        <div class="page-content">
          {#if currentPage === 'home'}
            <Home navigate={navigate} darkMode={darkMode} />
          {:else if currentPage === 'settings'}
            <Settings />
          {:else if currentPage === 'dictation'}
            <Dictation />
          {:else if currentPage === 'dictionary'}
            <Dictionary />
          {:else if currentPage === 'snippets'}
            <Snippets />
          {:else if currentPage === 'history'}
            <History navigate={navigate} />
          {:else if currentPage === 'history-detail'}
            <HistoryDetail navigate={navigate} />
          {:else if currentPage === 'notes'}
            <Notes navigate={navigate} />
          {:else if currentPage === 'note-detail'}
            <NoteDetail navigate={navigate} />
          {:else if currentPage === 'tasks'}
            <Tasks navigate={navigate} />
          {:else if currentPage === 'task-detail'}
            <TaskDetail navigate={navigate} />
          {:else if currentPage === 'reminders'}
            <Reminders navigate={navigate} />
          {/if}
        </div>
      </main>
      </div>
      <StatusBar
        config={statusBarConfig}
        dbStatus={dbStatus}
        platform={statusBarPlatform}
        lastLatencyMs={lastLatencyMs}
        dictationEnabled={dictationEnabled}
        onRetryDb={refreshDbStatus}
      />
  </div>
{/if}

<style>
  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(body) {
    font-family: var(--font-sleek, 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif);
    background: var(--bg-app);
    color: var(--text-primary);
  }

  /* When main app uses sleek theme, body must use same bg (body is not inside .kalam-sleek). */
  :global(body:has(.kalam-sleek.light)) {
    background: #ffffff;
    color: #1d1d1f;
  }
  :global(body:has(.kalam-sleek.dark)) {
    background: #000000;
    color: #f5f5f7;
  }

  .app-shell {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    height: 100vh;
    overflow: hidden;
  }

  .onboarding-shell {
    min-height: 100vh;
    height: 100vh;
    overflow: hidden;
  }

  /* === Sleek design system (match prototype) === */
  /* Base variables - global so portaled modals can access them */
  :global(.kalam-sleek) {
    --font-sleek: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
    --font: var(--font-sleek);
    --transition: var(--transition-sleek);
    --space-xs: 4px;
    --space-sm: 8px;
    --space-md: 16px;
    --space-lg: 24px;
    --space-xl: 32px;
    --space-2xl: 48px;
    --space-3xl: 64px;
    --radius-sm: 8px;
    --radius-md: 12px;
    --radius-lg: 20px;
    --radius-full: 9999px;
    --transition-sleek: 200ms cubic-bezier(0.4, 0, 0.2, 1);
  }

  /* Light theme variables - global */
  :global(.kalam-sleek.light) {
    --bg: #ffffff;
    --bg-elevated: #f5f5f7;
    --bg-card: #ffffff;
    --bg-input: #f5f5f7;
    --bg-hover: rgba(0, 0, 0, 0.04);
    --border: rgba(0, 0, 0, 0.08);
    --border-light: rgba(0, 0, 0, 0.04);
    --border-subtle: rgba(0, 0, 0, 0.06);
    --text: #1d1d1f;
    --text-secondary: #86868b;
    --text-muted: #a1a1a6;
    --accent: #000000;
    --accent-fg: #ffffff;
    --shadow: 0 2px 12px rgba(0, 0, 0, 0.03);
    --bg-app: #ffffff;
    --bg-content: #ffffff;
    --text-primary: #1d1d1f;
    --navy-deep: #1d1d1f;
    --primary: #000000;
    --primary-alpha: rgba(0, 0, 0, 0.08);
    --primary-alpha-subtle: rgba(0, 0, 0, 0.04);
  }

  /* Dark theme variables - global */
  :global(.kalam-sleek.dark) {
    --bg: #000000;
    --bg-elevated: #1c1c1e;
    --bg-card: #1c1c1e;
    --bg-input: #2c2c2e;
    --bg-hover: rgba(255, 255, 255, 0.08);
    --border: rgba(255, 255, 255, 0.12);
    --border-light: rgba(255, 255, 255, 0.06);
    --border-subtle: rgba(255, 255, 255, 0.08);
    --text: #f5f5f7;
    --text-secondary: #a1a1a6;
    --text-muted: #6e6e73;
    --accent: #ffffff;
    --accent-fg: #000000;
    --shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
    --bg-app: #000000;
    --bg-content: #000000;
    --text-primary: #f5f5f7;
    --navy-deep: #f5f5f7;
    --primary: #ffffff;
    --primary-alpha: rgba(255, 255, 255, 0.12);
    --primary-alpha-subtle: rgba(255, 255, 255, 0.06);
  }

  /* Component-scoped kalam-sleek styles */
  .kalam-sleek {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
    font-family: var(--font-sleek);
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    background: var(--bg);
    color: var(--text);
    flex-direction: column;
  }

  .sleek-body-row {
    flex: 1;
    min-height: 0;
    display: flex;
    overflow: hidden;
  }

  /* Sidebar — prototype layout */
  .sidebar {
    width: 240px;
    flex-shrink: 0;
    background: var(--bg-elevated);
    display: flex;
    flex-direction: column;
    position: relative;
    transition: width var(--transition-sleek);
  }

  .sidebar.collapsed {
    width: 80px;
  }

  .sidebar-content {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: var(--space-lg) var(--space-md);
    position: relative;
    z-index: 0;
  }

  .sidebar.collapsed .sidebar-content {
    padding: var(--space-lg) var(--space-sm);
    padding-right: 20px; /* keep content left of toggle so Settings/theme buttons are clickable */
    align-items: center;
  }

  /* When collapsed, stack theme + Settings so they fit and don't sit under the toggle */
  .sidebar.collapsed .bottom-links {
    flex-direction: column;
  }

  .sidebar-toggle {
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    width: 20px;
    /* Above .sidebar-content: content is full-width and would otherwise capture all clicks on this strip. */
    z-index: 2;
    border: none;
    background: transparent;
    color: var(--text-muted);
    opacity: 0.5;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: opacity 0.2s, color 0.2s, background 0.2s;
  }

  .sidebar-toggle:hover {
    opacity: 1;
    color: var(--text);
    background: var(--bg-hover);
  }

  .sidebar-toggle :global(svg) {
    font-size: 16px;
  }

  .logo-section {
    padding: 0 var(--space-sm);
    margin-bottom: var(--space-xl);
  }

  .sidebar.collapsed .logo-section {
    padding: 0;
    margin-bottom: var(--space-lg);
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .logo-img {
    /* Sidebar mark: 28px base ×1.2, then ×1.15 */
    width: calc(28px * 1.2 * 1.15);
    height: calc(28px * 1.2 * 1.15);
    flex-shrink: 0;
    object-fit: contain;
    display: block;
  }

  .logo-text {
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.03em;
  }

  .sidebar.collapsed .logo-text {
    display: none;
  }

  .sidebar.collapsed .logo {
    justify-content: center;
  }

  .main-nav {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .sidebar.collapsed .main-nav {
    width: 100%;
    align-items: center;
  }

  .nav-link {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition-sleek);
    text-align: left;
    width: 100%;
  }

  .sidebar.collapsed .nav-link {
    width: 48px;
    height: 48px;
    padding: 0;
    justify-content: center;
  }

  .nav-link:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .nav-link.active {
    background: var(--bg-card);
    color: var(--text);
    box-shadow: var(--shadow);
    font-weight: 600;
  }

  .nav-link :global(svg) {
    font-size: 18px;
  }

  .sidebar.collapsed .nav-text {
    display: none;
  }

  .sidebar-bottom {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .sidebar.collapsed .sidebar-bottom {
    align-items: center;
  }

  /* Muted pill when dictation is off; when on, matches theme/settings icon buttons (transparent). */
  .dictation-toggle.dictation-off {
    background: var(--bg-input);
    color: var(--text-muted);
    border: 1px solid var(--border-subtle);
  }

  .dictation-toggle.dictation-off:hover {
    background: var(--bg-input);
    color: var(--text-secondary);
    border-color: var(--border);
  }

  .bottom-links {
    display: flex;
    justify-content: center;
    gap: var(--space-sm);
  }

  .icon-btn {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: var(--transition-sleek);
  }

  .icon-btn:hover,
  .icon-btn.active {
    background: var(--bg-hover);
    color: var(--text);
  }

  .icon-btn :global(svg) {
    font-size: 18px;
  }

  /* Main content — prototype padding and scroll */
  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  .page-content {
    flex: 1;
    overflow-y: auto;
    overflow-x: clip;
    /* Allow shrinking inside flex .main so wide chart SVGs don’t force horizontal overflow */
    min-width: 0;
    padding: var(--space-3xl) var(--space-2xl);
    max-width: 900px;
    margin: 0 auto;
    width: 100%;
    background: var(--bg);
  }

  /* ~min window width (845) minus sidebar: content column is narrow; stack toolbars and soften type scale. */
  @media (max-width: 960px) {
    .page-content {
      padding: var(--space-xl) var(--space-lg);
    }
  }

  @media (max-width: 768px) {
    .sidebar {
      width: 80px;
    }

    .sidebar:not(.collapsed) {
      width: 240px;
    }

    .sidebar.collapsed .sidebar-content {
      padding: var(--space-md) var(--space-sm);
    }

    .page-content {
      padding: var(--space-lg) var(--space-md);
    }
  }

  @media (max-width: 960px) {
    :global(.kalam-sleek .page-content .page-title) {
      font-size: 28px;
      letter-spacing: -0.03em;
    }
    :global(.kalam-sleek .page-content .page-subtitle) {
      font-size: 14px;
    }
    :global(.kalam-sleek .page-content .notes-header),
    :global(.kalam-sleek .page-content .page-header) {
      flex-direction: row;
      align-items: flex-start;
    }
    :global(.kalam-sleek .page-content .notes-header > div:first-child),
    :global(.kalam-sleek .page-content .page-header > div:first-child) {
      min-width: 0;
      flex: 1 1 auto;
    }
    :global(.kalam-sleek .page-content .notes-header .btn-primary),
    :global(.kalam-sleek .page-content .notes-header .btn-danger-outline),
    :global(.kalam-sleek .page-content .page-header .btn-primary) {
      flex-shrink: 0;
      align-self: flex-start;
    }
    :global(.kalam-sleek .page-content .dash-columns) {
      grid-template-columns: 1fr;
    }
    :global(.kalam-sleek .page-content .notes-toolbar-scope-dropdown) {
      flex: 0 0 auto;
      min-width: 48px;
      max-width: 100%;
    }
    :global(.kalam-sleek .page-content .notes-sort-cycle),
    :global(.kalam-sleek .page-content .notes-label-filter-dropdown) {
      flex: 0 0 auto;
    }
    :global(.kalam-sleek .page-content .sleek-header) {
      flex-wrap: wrap;
      align-items: flex-start;
      gap: var(--space-md);
    }
    :global(.kalam-sleek .page-content .sleek-back) {
      min-width: 0;
    }
    :global(.kalam-sleek .page-content .sleek-actions) {
      flex-wrap: wrap;
      justify-content: flex-end;
      row-gap: 8px;
      max-width: 100%;
    }
    :global(.kalam-sleek .page-content .sleek-cancel) {
      padding: 8px 14px;
      font-size: 13px;
    }
    :global(.kalam-sleek .page-content .sleek-save) {
      padding: 8px 14px;
      font-size: 13px;
    }
    :global(.kalam-sleek .page-content .btn-primary),
    :global(.kalam-sleek .page-content .btn-danger-outline) {
      padding: 8px 14px;
      font-size: 13px;
    }
    :global(.kalam-sleek .page-content .list-item) {
      min-width: 0;
    }
    :global(.kalam-sleek .page-content .item-content) {
      min-width: 0;
    }
    :global(.kalam-sleek .page-content .setting-row) {
      flex-wrap: wrap;
      align-items: flex-start;
    }
    :global(.kalam-sleek .page-content .sleek-title) {
      font-size: 26px;
    }
  }

  /* Very narrow viewport only: stack list toolbar (search / actions). Default stays one row at app min width (~846). */
  @media (max-width: 560px) {
    :global(.kalam-sleek .page-content .notes-toolbar) {
      flex-direction: column;
      align-items: stretch;
    }
    :global(.kalam-sleek .page-content .notes-toolbar .notes-search-bar) {
      flex: none;
      width: 100%;
      max-width: none;
    }
    :global(.kalam-sleek .page-content .notes-toolbar-actions) {
      flex: none;
      width: 100%;
      flex-wrap: nowrap;
      align-items: stretch;
      gap: var(--space-sm);
      min-width: 0;
    }
    /* Sort-only strip: keep content-sized (scope is a sibling, not inside scroll). */
    :global(.kalam-sleek .page-content .notes-toolbar-actions-scroll) {
      flex: 0 0 auto;
      justify-content: flex-end;
    }
  }

  /* Narrow toolbar: icon-only scope trigger label via @container list-toolbar (max-width: 480px) below. */

  /* === Prototype-matching page content (so Home/History/etc. can use same classes) === */
  :global(.kalam-sleek .page-content .page),
  :global(.kalam-sleek .page-content .fade-in) {
    animation: fadeInPage 0.4s ease-out forwards;
    min-width: 0;
    max-width: 100%;
  }
  /* Container queries: tune layouts down to the window min width (~845px) minus sidebar. */
  :global(.kalam-sleek .page-content .page.fade-in) {
    container-type: inline-size;
    container-name: sleek-page;
  }
  @keyframes fadeInPage {
    from { opacity: 0; transform: translateY(8px); }
    /* Final frame must be `transform: none` (not translateY(0)): any non-none transform on an
       ancestor makes position:fixed relative to that ancestor and clips inside overflow:auto
       (.page-content), which broke the History “Clear all” modal in the main window. */
    to { opacity: 1; transform: none; }
  }
  :global(.kalam-sleek .page-content .page-header) {
    margin-bottom: var(--space-2xl);
  }
  :global(.kalam-sleek .page-content .page-title) {
    font-size: 36px;
    font-weight: 600;
    letter-spacing: -0.04em;
    margin: 0 0 8px 0;
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .page-subtitle) {
    font-size: 16px;
    color: var(--text-secondary);
    margin: 0;
  }
  :global(.kalam-sleek .page-content .stats-row) {
    display: flex;
    gap: var(--space-lg);
    margin-bottom: var(--space-3xl);
  }
  :global(.kalam-sleek .page-content .stat-box) {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: var(--space-lg);
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    min-width: 0;
    max-width: 100%;
    box-sizing: border-box;
  }
  :global(.kalam-sleek .page-content .stat-label) {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  :global(.kalam-sleek .page-content .stat-num) {
    font-size: 32px;
    font-weight: 600;
    letter-spacing: -0.03em;
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .dashboard-grid) {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xl);
  }
  :global(.kalam-sleek .page-content .dash-columns) {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-2xl);
  }
  :global(.kalam-sleek .page-content .dash-section.wide) {
    grid-column: 1 / -1;
  }
  :global(.kalam-sleek .page-content .section-header) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-md);
    padding-bottom: var(--space-sm);
    border-bottom: 1px solid var(--border-light);
  }
  :global(.kalam-sleek .page-content .section-header h3) {
    font-size: 16px;
    font-weight: 600;
    margin: 0;
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .text-btn) {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
  }
  :global(.kalam-sleek .page-content .text-btn:hover) {
    color: var(--text);
  }
  /* Primary button — prototype: filled accent, used for New Note/Task/Reminder/Snippet */
  :global(.kalam-sleek .page-content .btn-primary) {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 18px;
    border: none;
    border-radius: var(--radius-full);
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .btn-primary:hover:not(:disabled)) {
    opacity: 0.9;
    transform: translateY(-1px);
  }
  :global(.kalam-sleek .page-content .btn-primary:disabled) {
    opacity: 0.5;
    cursor: not-allowed;
  }
  :global(.kalam-sleek .page-content .btn-primary :global(svg)) {
    font-size: 18px;
  }
  /* Destructive outline — same footprint as .btn-primary (e.g. Empty trash in Notes header). */
  :global(.kalam-sleek .page-content .btn-danger-outline) {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 18px;
    border: 1px solid color-mix(in srgb, #ff3b30 55%, var(--border));
    border-radius: var(--radius-full);
    background: transparent;
    color: #ff3b30;
    font-size: 14px;
    font-weight: 600;
    font-family: var(--font-sleek);
    cursor: pointer;
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .btn-danger-outline:hover:not(:disabled)) {
    background: color-mix(in srgb, #ff3b30 10%, transparent);
    transform: translateY(-1px);
  }
  :global(.kalam-sleek .page-content .btn-danger-outline:disabled) {
    opacity: 0.45;
    cursor: not-allowed;
  }
  :global(.kalam-sleek .page-content .btn-danger-outline :global(svg)) {
    font-size: 18px;
  }
  :global(.kalam-sleek .page-content .history-list) {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }
  :global(.kalam-sleek .page-content .list-item) {
    display: flex;
    gap: var(--space-md);
    padding: var(--space-md);
    background: var(--bg-elevated);
    border-radius: var(--radius-md);
    transition: var(--transition-sleek);
    cursor: pointer;
  }
  :global(.kalam-sleek .page-content .list-item:hover) {
    background: var(--bg-hover);
  }
  :global(.kalam-sleek .page-content .item-icon) {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--bg);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    flex-shrink: 0;
  }
  :global(.kalam-sleek .page-content .item-content) {
    flex: 1;
  }
  :global(.kalam-sleek .page-content .item-text) {
    font-size: 14px;
    line-height: 1.5;
    margin: 0 0 6px 0;
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .item-meta-row) {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    min-width: 0;
    max-width: 100%;
  }
  /* When meta row has copy button, use entry-actions spacing (12px) even if class is missing in DOM */
  :global(.kalam-sleek .page-content .item-meta-row:has(.icon-btn)) {
    gap: 12px;
  }
  :global(.kalam-sleek .page-content .item-meta) {
    font-size: 12px;
    color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .simple-list) {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  :global(.kalam-sleek .page-content .simple-item) {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 14px;
    padding: 8px 0;
    color: var(--text);
    cursor: pointer;
  }
  :global(.kalam-sleek .page-content .priority-dot) {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  :global(.kalam-sleek .page-content .muted-icon) {
    color: var(--text-muted);
    font-size: 16px;
  }
  :global(.kalam-sleek .page-content .simple-text) {
    color: var(--text);
  }
  /* Empty state for dashboard Tasks/Reminders when none due — matches prototype tone */
  :global(.kalam-sleek .page-content .simple-list-empty) {
    font-size: 13px;
    color: var(--text-muted);
    margin: 0;
    padding: 8px 0;
  }
  :global(.kalam-sleek .page-content .chip) {
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 4px 8px;
    border-radius: 6px;
  }
  :global(.kalam-sleek .page-content .chip.small) {
    font-size: 10px;
    padding: 2px 6px;
  }
  :global(.kalam-sleek .page-content .chip.dictation) {
    background: rgba(52, 199, 89, 0.15);
    color: #34C759;
  }
  :global(.kalam-sleek .page-content .chip.command) {
    background: rgba(175, 82, 222, 0.15);
    color: #AF52DE;
  }
  :global(.kalam-sleek .page-content .chip.chip-mode.dictation) {
    background: rgba(52, 199, 89, 0.15);
    color: #34C759;
  }
  :global(.kalam-sleek .page-content .chip.chip-mode.command) {
    background: rgba(175, 82, 222, 0.15);
    color: #AF52DE;
  }
  :global(.kalam-sleek .page-content .chip.chip-stt.cloud) {
    background: rgba(10, 132, 255, 0.15);
    color: #0A84FF;
  }
  :global(.kalam-sleek .page-content .chip.chip-stt.local) {
    background: rgba(48, 209, 88, 0.15);
    color: #30D158;
  }
  :global(.kalam-sleek .page-content .chip.chip-stt.hybrid) {
    background: rgba(255, 159, 10, 0.15);
    color: #FF9F0A;
  }
  :global(.kalam-sleek .page-content .chip.chip-stt.auto) {
    background: rgba(142, 142, 147, 0.18);
    color: var(--text-secondary, #8e8e93);
  }
  :global(.kalam-sleek .page-content .chip.chip-stt.unknown) {
    background: rgba(142, 142, 147, 0.12);
    color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .chip.chip-lang) {
    background: rgba(100, 210, 255, 0.12);
    color: var(--text-secondary, #8e8e93);
  }
  :global(.kalam-sleek .page-content .state-container) {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-3xl);
    color: var(--text-muted);
    gap: var(--space-md);
  }
  :global(.kalam-sleek .page-content .state-container :global(svg)) {
    font-size: 32px;
    animation: spinPage 0.8s linear infinite;
  }
  @keyframes spinPage {
    to { transform: rotate(360deg); }
  }

  /* === HISTORY PAGE (prototype) === */
  :global(.kalam-sleek .page-content .search-bar) {
    position: relative;
    margin-bottom: var(--space-3xl);
  }
  /* Wrap Iconify root so centering matches Prototype (flex), avoids svg+parent transform quirks. */
  :global(.kalam-sleek .page-content .search-bar .search-bar-icon) {
    position: absolute;
    left: 16px;
    top: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .search-bar .search-bar-icon :global(svg)) {
    font-size: 18px;
    width: 1em;
    height: 1em;
    display: block;
  }
  :global(.kalam-sleek .page-content .search-bar input) {
    width: 100%;
    padding: 16px 16px 16px 48px;
    background: var(--bg-elevated);
    border: 1px solid transparent;
    border-radius: var(--radius-lg);
    color: var(--text);
    font-size: 15px;
    font-family: var(--font-sleek);
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .search-bar input:focus) {
    outline: none;
    background: var(--bg);
    border-color: var(--border);
    box-shadow: var(--shadow);
  }
  :global(.kalam-sleek .page-content .timeline) {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xl);
  }
  :global(.kalam-sleek .page-content .day-group) {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
  }
  :global(.kalam-sleek .page-content .day-label) {
    font-size: 18px;
    font-weight: 600;
    margin: 0;
    display: flex;
    align-items: baseline;
    gap: 12px;
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .day-sub) {
    font-size: 13px;
    font-weight: 400;
    color: var(--text-secondary);
  }
  :global(.kalam-sleek .page-content .entries) {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }
  :global(.kalam-sleek .page-content .entry-row) {
    display: flex;
    gap: var(--space-lg);
    padding: var(--space-lg);
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    transition: var(--transition-sleek);
    cursor: pointer;
  }
  :global(.kalam-sleek .page-content .entry-row:hover) {
    background: var(--bg-hover);
  }
  :global(.kalam-sleek .page-content .entry-time) {
    width: 60px;
    font-size: 13px;
    color: var(--text-secondary);
    flex-shrink: 0;
    padding-top: 2px;
  }
  :global(.kalam-sleek .page-content .entry-content) {
    flex: 1;
  }
  :global(.kalam-sleek .page-content .entry-text) {
    font-size: 15px;
    line-height: 1.6;
    margin: 0 0 12px 0;
    color: var(--text);
  }
  /* History search: all substring hits (same semantics as Rust search_history). */
  :global(.kalam-sleek .page-content .entry-text mark.history-search-hit) {
    background: rgba(255, 159, 10, 0.38);
    color: inherit;
    padding: 0 0.1em;
    border-radius: 3px;
    font-weight: 500;
  }
  :global(.kalam-sleek.light .page-content .entry-text mark.history-search-hit) {
    background: rgba(255, 159, 10, 0.45);
  }
  :global(.kalam-sleek .page-content .entry-actions) {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
    min-width: 0;
    overflow: hidden;
  }
  :global(.kalam-sleek .page-content .entry-duration) {
    font-size: 12px;
    color: var(--text-muted);
  }
  /* Copy button in Recent/History: match prototype .icon-btn and .icon-btn.small exactly */
  :global(.kalam-sleek .page-content .icon-btn) {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .icon-btn:hover),
  :global(.kalam-sleek .page-content .icon-btn.active) {
    background: var(--bg-hover);
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .icon-btn :global(svg)) {
    font-size: 18px;
  }
  :global(.kalam-sleek .page-content .icon-btn.small) {
    width: 28px;
    height: 28px;
    margin-left: auto;
  }
  @keyframes historyCopyPop {
    0% {
      transform: scale(1);
    }
    40% {
      transform: scale(1.14);
    }
    100% {
      transform: scale(1);
    }
  }
  :global(.kalam-sleek .page-content .icon-btn.small.copied) {
    animation: historyCopyPop 0.38s cubic-bezier(0.34, 1.2, 0.64, 1);
    color: var(--text);
    background: var(--bg-hover);
  }
  :global(.kalam-sleek .page-content .empty-state) {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 64px 0;
    color: var(--text-muted);
    gap: 12px;
  }
  :global(.kalam-sleek .page-content .empty-state :global(svg)) {
    font-size: 48px;
    opacity: 0.5;
  }
  :global(.kalam-sleek .page-content .empty-state :global(span.spin-icon)) {
    animation: spinPage 0.8s linear infinite;
  }
  :global(.kalam-sleek .page-content .btn-ghost) {
    padding: 8px 16px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-weight: 500;
    cursor: pointer;
    color: var(--text);
    font-size: 14px;
  }
  :global(.kalam-sleek .page-content .btn-ghost:hover) {
    background: var(--bg-hover);
  }

  /* === SETTINGS PAGE (prototype) === */
  :global(.kalam-sleek .page-content .settings-page) {
    max-width: 800px;
  }
  :global(.kalam-sleek .page-content .settings-header) {
    margin-bottom: var(--space-xl);
  }
  :global(.kalam-sleek .page-content .settings-tabs) {
    display: flex;
    gap: 4px;
    margin-bottom: var(--space-xl);
    border-bottom: 1px solid var(--border);
    padding-bottom: 1px;
  }
  :global(.kalam-sleek .page-content .settings-tab) {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 20px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition-sleek);
    border-bottom: 2px solid transparent;
    margin-bottom: -2px;
  }
  :global(.kalam-sleek .page-content .settings-tab:hover) {
    color: var(--text);
    background: var(--bg-hover);
  }
  :global(.kalam-sleek .page-content .settings-tab.active) {
    color: var(--text);
    border-bottom-color: var(--accent);
    font-weight: 600;
  }
  :global(.kalam-sleek .page-content .settings-tab :global(svg)) {
    flex-shrink: 0;
    display: block;
    width: 1.1em;
    height: 1.1em;
    color: inherit;
  }
  :global(.kalam-sleek .page-content .settings-content) {
    min-height: 400px;
  }
  :global(.kalam-sleek .page-content .settings-section) {
    margin-bottom: var(--space-lg);
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    overflow: hidden;
  }
  :global(.kalam-sleek .page-content .settings-section.collapsed) {
    margin-bottom: var(--space-xs);
  }
  :global(.kalam-sleek .page-content .settings-section .section-header) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: var(--space-lg);
    margin: 0;
    background: transparent;
    border: none;
    cursor: pointer;
    transition: var(--transition-sleek);
    text-align: left;
    /* WebView / native <button> often ignores inherited color; without this, Iconify carets stay dark on dark surfaces. */
    color: var(--text);
    -webkit-appearance: none;
    appearance: none;
  }
  :global(.kalam-sleek .page-content .settings-section .section-header :global(svg)) {
    flex-shrink: 0;
    display: block;
    width: 1.15em;
    height: 1.15em;
    color: var(--text-secondary);
  }
  :global(.kalam-sleek .page-content .settings-section .section-header:hover :global(svg)) {
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .section-header:hover) {
    background: var(--bg-hover);
  }
  :global(.kalam-sleek .page-content .settings-section .section-header h3) {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
    margin: 0;
  }
  :global(.kalam-sleek .page-content .section-content) {
    padding: 0 var(--space-lg) var(--space-lg);
  }
  :global(.kalam-sleek .page-content .setting-row) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-md) 0;
    gap: var(--space-lg);
    border-bottom: 1px solid var(--border-light);
  }
  :global(.kalam-sleek .page-content .setting-row:last-child) {
    border-bottom: none;
  }
  :global(.kalam-sleek .page-content .setting-label) {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }
  :global(.kalam-sleek .page-content .setting-name) {
    font-size: 14px;
    font-weight: 500;
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .setting-desc) {
    font-size: 13px;
    color: var(--text-secondary);
  }
  :global(.kalam-sleek .page-content .setting-control) {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }
  :global(.kalam-sleek .page-content .setting-control.full-width) {
    flex-direction: column;
    align-items: stretch;
    min-width: 200px;
  }
  :global(.kalam-sleek .page-content .setting-label.full-width) {
    flex: 1 1 100%;
  }
  :global(.kalam-sleek .page-content .settings-tab-content) {
    animation: settingsTabFade 0.2s ease;
  }
  @keyframes settingsTabFade {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  :global(.kalam-sleek .page-content .setting-row.sub-setting) {
    padding-left: var(--space-xl);
    border-left: 2px solid var(--border);
    margin-left: var(--space-md);
  }
  :global(.kalam-sleek .page-content .setting-row.checkbox-row) {
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-xs);
  }
  :global(.kalam-sleek .page-content .setting-row.local-model-row),
  :global(.kalam-sleek .page-content .setting-row.language-row) {
    flex-direction: column;
    align-items: stretch;
  }
  :global(.kalam-sleek .page-content .setting-row.row-group) {
    flex-wrap: wrap;
  }

  /* Hotkey capture (prototype) */
  :global(.kalam-sleek .page-content .hotkey-capture-area) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-width: 160px;
    min-height: 40px;
    padding: 8px 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
    position: relative;
    gap: 8px;
  }
  :global(.kalam-sleek .page-content .hotkey-capture-area:hover) {
    background: var(--bg-hover);
    border-color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .hotkey-capture-area.capturing) {
    border-color: var(--accent);
    background: rgba(0, 122, 255, 0.05);
    box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.15);
    cursor: default;
  }
  :global(.kalam-sleek.dark .page-content .hotkey-capture-area.capturing) {
    background: rgba(255, 255, 255, 0.06);
    box-shadow: 0 0 0 3px rgba(255, 255, 255, 0.12);
  }
  :global(.kalam-sleek .page-content .hotkey-pills) {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1;
    flex-wrap: wrap;
    /* Clicks should start/change capture like the rest of the row (see .hotkey-placeholder). */
    pointer-events: none;
  }
  :global(.kalam-sleek .page-content .hotkey-pill) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 4px 8px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
    text-transform: uppercase;
    letter-spacing: 0.3px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
    min-width: 28px;
    height: 24px;
  }
  :global(.kalam-sleek .page-content .hotkey-pill.modifier) {
    background: rgba(0, 122, 255, 0.1);
    border-color: rgba(0, 122, 255, 0.3);
    color: var(--accent);
  }
  :global(.kalam-sleek.dark .page-content .hotkey-pill.modifier) {
    background: rgba(255, 255, 255, 0.1);
    border-color: rgba(255, 255, 255, 0.25);
  }
  :global(.kalam-sleek .page-content .hotkey-placeholder) {
    color: var(--text-muted);
    font-size: 13px;
    flex: 1;
    min-width: 0;
    line-height: 1.35;
    /* Let clicks reach the capture surface (text/SVG sometimes steal hits in WebView2). */
    pointer-events: none;
  }
  :global(.kalam-sleek .page-content .hotkey-capture-area .hotkey-edit-icon) {
    font-size: 14px;
    color: var(--text-muted);
    flex-shrink: 0;
    pointer-events: none;
  }
  :global(.kalam-sleek .page-content .hotkey-clear),
  :global(.kalam-sleek .page-content .hotkey-cancel) {
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 28px;
    min-height: 28px;
    width: 28px;
    height: 28px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s;
    padding: 0;
    margin-left: 4px;
    flex-shrink: 0;
    pointer-events: auto;
    position: relative;
    z-index: 1;
  }
  :global(.kalam-sleek .page-content .hotkey-clear:hover) {
    background: rgba(255, 59, 48, 0.1);
    color: #ff3b30;
  }
  :global(.kalam-sleek .page-content .hotkey-cancel:hover) {
    background: rgba(0, 122, 255, 0.1);
    color: var(--accent);
  }

  :global(.kalam-sleek .page-content .toggle-switch) {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 26px;
    flex-shrink: 0;
  }
  :global(.kalam-sleek .page-content .toggle-switch input) {
    opacity: 0;
    width: 0;
    height: 0;
  }
  :global(.kalam-sleek .page-content .toggle-switch .slider) {
    position: absolute;
    cursor: pointer;
    inset: 0;
    background: var(--border);
    border-radius: var(--radius-full);
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .toggle-switch .slider::before) {
    position: absolute;
    content: '';
    height: 20px;
    width: 20px;
    left: 3px;
    bottom: 3px;
    background: white;
    border-radius: 50%;
    transition: var(--transition-sleek);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }
  :global(.kalam-sleek .page-content .toggle-switch input:checked + .slider) {
    background: var(--accent);
  }
  :global(.kalam-sleek .page-content .toggle-switch input:checked + .slider::before) {
    transform: translateX(18px);
    background: var(--accent-fg);
  }

  :global(.kalam-sleek .page-content .segmented-control) {
    display: flex;
    gap: 2px;
    padding: 3px;
    background: var(--bg);
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
  }
  :global(.kalam-sleek .page-content .segmented-control button) {
    padding: 6px 16px;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .segmented-control button.active) {
    background: var(--accent);
    color: var(--accent-fg);
  }

  :global(.kalam-sleek .page-content select.form-select),
  :global(.kalam-sleek .page-content .form-select) {
    width: 100%;
    max-width: 320px;
    padding: 12px 16px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-light);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 15px;
    font-family: var(--font-sleek, inherit);
    cursor: pointer;
    transition: all 0.2s ease;
  }
  :global(.kalam-sleek .page-content .form-select:focus) {
    outline: none;
    border-color: var(--text-muted);
    background: var(--bg);
  }

  :global(.kalam-sleek .page-content .stt-mode-cards.stt-mode-row) {
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    gap: var(--space-sm);
    margin-bottom: var(--space-sm);
  }
  :global(.kalam-sleek .page-content .stt-mode-row .stt-mode-card) {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: center;
    gap: 10px;
    flex: 1 1 0;
    min-width: 5.5rem;
    padding: 10px 12px;
    background: var(--bg);
    border: 2px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: var(--transition-sleek);
    text-align: left;
  }
  :global(.kalam-sleek .page-content .stt-mode-card:hover) {
    border-color: var(--text-muted);
    background: var(--bg-hover);
  }
  :global(.kalam-sleek .page-content .stt-mode-card.active) {
    border-color: var(--accent);
    background: rgba(0, 122, 255, 0.05);
  }
  :global(.kalam-sleek.dark .page-content .stt-mode-card.active) {
    background: rgba(255, 255, 255, 0.06);
  }
  :global(.kalam-sleek .page-content .stt-mode-row .mode-icon) {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--bg-elevated);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    font-size: 18px;
    flex-shrink: 0;
  }
  :global(.kalam-sleek .page-content .stt-mode-row .mode-name) {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    white-space: nowrap;
  }

  /* Settings: Cloud provider + API key share one left accent (see `.stt-cloud-group`). Prototype still uses `.api-key-section` alone. */
  :global(.kalam-sleek .page-content .stt-cloud-group) {
    margin-left: var(--space-md);
    padding-left: var(--space-xl);
    border-left: 2px solid var(--border);
    margin-top: var(--space-md);
    padding-bottom: var(--space-md);
    border-bottom: 1px solid var(--border-light);
  }
  :global(.kalam-sleek .page-content .stt-cloud-group .setting-row) {
    border-bottom: none;
  }
  :global(.kalam-sleek .page-content .stt-cloud-group .api-key-hint) {
    margin: var(--space-xs) 0 0 0;
  }
  :global(.kalam-sleek .page-content .stt-cloud-group .api-key-row) {
    margin-bottom: 0;
  }
  :global(.kalam-sleek .page-content .api-key-section) {
    padding-left: var(--space-xl);
    border-left: 2px solid var(--border);
    margin-left: var(--space-md);
    margin-top: var(--space-md);
  }
  :global(.kalam-sleek .page-content .api-key-row) {
    display: flex;
    gap: var(--space-sm);
    margin-bottom: var(--space-sm);
    flex-wrap: wrap;
  }
  :global(.kalam-sleek .page-content .api-key-input) {
    flex: 1;
    min-width: 160px;
    padding: 10px 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 14px;
  }
  :global(.kalam-sleek .page-content .validation-badge) {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 13px;
    font-weight: 500;
    padding: 4px 8px;
    border-radius: var(--radius-md);
    background: rgba(255, 59, 48, 0.1);
    color: #ff3b30;
  }
  :global(.kalam-sleek .page-content .validation-badge.valid) {
    background: rgba(52, 199, 89, 0.1);
    color: #34c759;
  }
  :global(.kalam-sleek .page-content .api-key-hint) {
    font-size: 13px;
    margin: var(--space-sm) 0 0 0;
  }
  :global(.kalam-sleek .page-content .api-key-hint a) {
    color: var(--accent);
    text-decoration: none;
  }
  :global(.kalam-sleek .page-content .api-key-hint a:hover) {
    text-decoration: underline;
  }

  :global(.kalam-sleek .page-content .local-models-section) {
    padding-left: var(--space-xl);
    border-left: 2px solid var(--border);
    margin-left: var(--space-md);
    margin-top: var(--space-md);
  }
  :global(.kalam-sleek .page-content .local-models-hint) {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0 0 var(--space-md) 0;
  }

  :global(.kalam-sleek .page-content .number-input) {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  :global(.kalam-sleek .page-content .number-input input) {
    width: 80px;
    padding: 8px 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 14px;
    text-align: center;
  }
  :global(.kalam-sleek .page-content .number-input .unit) {
    font-size: 13px;
    color: var(--text-secondary);
  }

  :global(.kalam-sleek .page-content .privacy-info) {
    display: flex;
    align-items: flex-start;
    gap: var(--space-md);
    padding: var(--space-lg);
    background: rgba(0, 122, 255, 0.05);
    border-radius: var(--radius-md);
    margin-top: var(--space-md);
  }
  :global(.kalam-sleek.dark .page-content .privacy-info) {
    background: rgba(255, 255, 255, 0.06);
  }
  :global(.kalam-sleek .page-content .privacy-info :global(svg)) {
    font-size: 24px;
    color: var(--accent);
    flex-shrink: 0;
  }
  :global(.kalam-sleek .page-content .privacy-info p) {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.5;
  }

  :global(.kalam-sleek .page-content .log-actions) {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-md);
    margin-top: var(--space-md);
    padding-top: var(--space-md);
    border-top: 1px solid var(--border-light);
  }

  :global(.kalam-sleek .page-content .settings-secondary-btn) {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .settings-secondary-btn:hover) {
    background: var(--bg-hover);
    border-color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .settings-secondary-btn.danger) {
    color: #ff3b30;
    border-color: rgba(255, 59, 48, 0.3);
  }
  :global(.kalam-sleek .page-content .settings-secondary-btn.danger:hover) {
    background: rgba(255, 59, 48, 0.1);
    border-color: rgba(255, 59, 48, 0.5);
  }
  :global(.kalam-sleek .page-content .settings-secondary-btn:disabled) {
    opacity: 0.5;
    cursor: not-allowed;
  }

  :global(.kalam-sleek .page-content .settings-section.danger) {
    border-color: rgba(255, 59, 48, 0.3);
  }
  :global(.kalam-sleek .page-content .settings-section.danger .section-header h3) {
    color: #ff3b30;
  }
  :global(.kalam-sleek .page-content .danger-item) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-md) 0;
    gap: var(--space-md);
    border-bottom: 1px solid var(--border-light);
  }
  :global(.kalam-sleek .page-content .danger-item:last-child) {
    border-bottom: none;
  }
  :global(.kalam-sleek .page-content .danger-info) {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  :global(.kalam-sleek .page-content .danger-title) {
    font-size: 14px;
    font-weight: 500;
    color: #ff3b30;
  }
  :global(.kalam-sleek .page-content .danger-desc) {
    font-size: 13px;
    color: var(--text-secondary);
  }
  :global(.kalam-sleek .page-content .danger-btn) {
    padding: 8px 16px;
    background: transparent;
    border: 1px solid #ff3b30;
    border-radius: var(--radius-md);
    color: #ff3b30;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition-sleek);
    flex-shrink: 0;
  }
  :global(.kalam-sleek .page-content .danger-btn:hover) {
    background: rgba(255, 59, 48, 0.1);
  }

  /* About tab inside settings: section spacing matches other settings tabs (.settings-section margin-bottom) */
  :global(.kalam-sleek .page-content .about-content) {
    display: flex;
    flex-direction: column;
    gap: 0;
    padding: 0;
  }
  :global(.kalam-sleek .page-content .about-top-section) {
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    padding: var(--space-lg) var(--space-xl);
  }
  :global(.kalam-sleek .page-content .about-top-content) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-xl);
    flex-wrap: wrap;
  }
  :global(.kalam-sleek .page-content .about-top-content .version-label) {
    font-size: 15px;
    color: var(--text-secondary);
  }
  :global(.kalam-sleek .page-content .about-top-content .version-label strong) {
    color: var(--accent);
    font-weight: 600;
    font-family: ui-monospace, monospace;
  }
  :global(.kalam-sleek .page-content .about-top-content .updates-block) {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    flex-wrap: wrap;
  }
  :global(.kalam-sleek .page-content .about-top-content .channel-select) {
    min-width: 200px;
    padding: 10px 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 14px;
    cursor: pointer;
  }
  :global(.kalam-sleek .page-content .about-top-content .btn-check) {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .about-top-content .btn-check:hover:not(:disabled)) {
    background: var(--bg-hover);
  }
  :global(.kalam-sleek .page-content .about-grid.two-col) {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--space-lg);
  }
  @media (max-width: 700px) {
    :global(.kalam-sleek .page-content .about-grid.two-col) {
      grid-template-columns: 1fr;
    }
  }
  :global(.kalam-sleek .page-content .about-content .about-card) {
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    padding: var(--space-xl);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    transition: transform 0.3s ease, box-shadow 0.3s ease;
  }
  :global(.kalam-sleek .page-content .about-content .about-card:hover) {
    transform: translateY(-2px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
  }
  :global(.kalam-sleek .page-content .about-content .about-card.highlight) {
    background: linear-gradient(to bottom right, var(--bg-elevated), rgba(0, 122, 255, 0.05));
    border-color: rgba(0, 122, 255, 0.3);
  }
  :global(.kalam-sleek .page-content .about-content .about-card .card-icon) {
    width: 40px;
    height: 40px;
    background: var(--bg);
    color: var(--accent);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 22px;
    margin-bottom: 4px;
  }
  :global(.kalam-sleek .page-content .about-content .about-card.highlight .card-icon) {
    background: var(--accent);
    color: white;
    box-shadow: 0 4px 12px rgba(0, 122, 255, 0.3);
  }
  :global(.kalam-sleek .page-content .about-content .about-card h3) {
    font-size: 18px;
    font-weight: 700;
    color: var(--text);
    margin: 0;
  }
  :global(.kalam-sleek .page-content .about-content .byline),
  :global(.kalam-sleek .page-content .about-content .card-text) {
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-secondary);
    margin: 0;
    flex-grow: 1;
  }
  :global(.kalam-sleek .page-content .about-content .byline a) {
    color: var(--accent);
    font-weight: 600;
    text-decoration: none;
    border-bottom: 1px solid transparent;
    transition: border-color 0.2s;
  }
  :global(.kalam-sleek .page-content .about-content .byline a:hover) {
    border-color: var(--accent);
  }
  :global(.kalam-sleek .page-content .about-content .action-group) {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    margin-top: auto;
  }
  :global(.kalam-sleek .page-content .about-content .action-link) {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 500;
    color: var(--text);
    text-decoration: none;
    padding: 10px 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    transition: all 0.2s ease;
  }
  :global(.kalam-sleek .page-content .about-content .action-link:hover) {
    background: var(--bg-hover);
  }
  :global(.kalam-sleek .page-content .about-content .action-link.secondary) {
    background: transparent;
    color: var(--text-secondary);
  }
  :global(.kalam-sleek .page-content .about-content .btn-primary.about-btn-primary) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 12px 20px;
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 14px;
    font-weight: 600;
    border-radius: var(--radius-md);
    text-decoration: none;
    border: none;
    cursor: pointer;
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .about-content .btn-primary.about-btn-primary:hover) {
    opacity: 0.92;
  }
  :global(.kalam-sleek .page-content .about-content .license-section) {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
    background: var(--bg-elevated);
  }
  :global(.kalam-sleek .page-content .about-content .license-section .accordion) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: var(--space-lg);
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--text);
    font-size: 14px;
    font-weight: 500;
  }
  :global(.kalam-sleek .page-content .about-content .license-section .accordion:hover) {
    background: var(--bg-hover);
  }
  :global(.kalam-sleek .page-content .about-content .license-section .accordion-title) {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  :global(.kalam-sleek .page-content .about-content .license-content) {
    padding: 0 var(--space-lg) var(--space-lg);
  }
  :global(.kalam-sleek .page-content .about-content .license-text) {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-secondary);
    white-space: pre-wrap;
    margin: 0;
    max-height: 240px;
    overflow: auto;
  }

  /* === NOTES PAGE (prototype) === */
  :global(.kalam-sleek .page-content .notes-header) {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: var(--space-md);
  }
  @container sleek-page (max-width: 520px) {
    :global(.kalam-sleek .page-content .notes-header > div:first-child),
    :global(.kalam-sleek .page-content .page-header > div:first-child) {
      flex: 1 1 100%;
      min-width: 0;
    }
    :global(.kalam-sleek .page-content .notes-header .btn-primary),
    :global(.kalam-sleek .page-content .page-header .btn-primary) {
      flex: 1 1 auto;
      width: 100%;
      justify-content: center;
    }
  }
  :global(.kalam-sleek .page-content .notes-subnav) {
    display: flex;
    gap: 4px;
    margin-bottom: var(--space-lg);
  }
  :global(.kalam-sleek .page-content .subnav-btn) {
    padding: 8px 16px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .subnav-btn:hover) {
    background: var(--bg-hover);
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .subnav-btn.active) {
    background: var(--bg-elevated);
    color: var(--text);
  }

  /* Notes / Tasks / Reminders: shared toolbar; search + `.notes-toolbar-actions` (scope, sort, Tasks tag filter). */
  :global(.kalam-sleek .page-content .notes-toolbar) {
    display: flex;
    flex-wrap: nowrap;
    align-items: stretch;
    gap: var(--space-sm);
    margin-bottom: var(--space-md);
    container-type: inline-size;
    container-name: list-toolbar;
  }
  /* Row: search grows; actions column is only as wide as tabs/sort/tag (scroll strip does not steal width). */
  :global(.kalam-sleek .page-content .notes-toolbar-actions) {
    display: flex;
    flex-wrap: nowrap;
    align-items: stretch;
    align-content: stretch;
    gap: var(--space-sm);
    flex: 0 0 auto;
    min-width: 0;
    justify-content: flex-end;
    overflow: visible;
  }
  /* Right-align the action cluster when list chrome hides search (`.notes-toolbar` has only this child). */
  :global(.kalam-sleek .page-content .notes-toolbar-actions:only-child) {
    margin-left: auto;
  }
  :global(.kalam-sleek .page-content .notes-toolbar-actions-scroll) {
    display: flex;
    flex-wrap: nowrap;
    align-items: stretch;
    gap: var(--space-sm);
    flex: 0 1 auto;
    min-width: 0;
    justify-content: flex-end;
    overflow-x: auto;
    overflow-y: visible;
    scrollbar-width: thin;
    -webkit-overflow-scrolling: touch;
  }
  :global(.kalam-sleek .page-content .notes-toolbar-actions .notes-toolbar-scope-dropdown) {
    flex: 0 0 auto;
    /* Keep scope from collapsing when a sibling (e.g. sort strip) uses large flex-basis. */
    min-width: 48px;
  }
  :global(.kalam-sleek .page-content .notes-toolbar .notes-search-bar) {
    flex: 1 1 0;
    min-width: 0;
    margin-bottom: 0;
    align-self: stretch;
    display: flex;
    align-items: stretch;
  }
  /* Only when the toolbar itself is very narrow: stack search + actions (not at ~600px content width). */
  @container list-toolbar (max-width: 520px) {
    :global(.kalam-sleek .page-content .notes-toolbar) {
      flex-wrap: wrap;
    }
    :global(.kalam-sleek .page-content .notes-toolbar .notes-search-bar) {
      flex: 1 1 100%;
      max-width: 100%;
    }
    :global(.kalam-sleek .page-content .notes-toolbar-actions) {
      flex: 1 1 100%;
      max-width: 100%;
      justify-content: flex-end;
      flex-wrap: nowrap;
      min-width: 0;
      row-gap: var(--space-sm);
    }
    /* Sort-only strip: must not use `flex-basis: 100%` — scope is a *sibling* now; 100% was for when scope lived inside scroll. */
    :global(.kalam-sleek .page-content .notes-toolbar-actions-scroll) {
      flex: 0 0 auto;
      max-width: 100%;
      justify-content: flex-end;
    }
    :global(.kalam-sleek .page-content .notes-toolbar-actions .notes-toolbar-scope-dropdown) {
      flex: 0 0 auto;
      min-width: 48px;
      width: auto;
    }
  }
  @container list-toolbar (max-width: 480px) {
    /* Icon-only scope trigger (title still on button). */
    :global(.kalam-sleek .page-content .notes-toolbar-scope-trigger .notes-toolbar-scope-trigger-label) {
      position: absolute;
      width: 1px;
      height: 1px;
      padding: 0;
      margin: -1px;
      overflow: hidden;
      clip: rect(0, 0, 0, 0);
      white-space: nowrap;
      border: 0;
    }
    :global(.kalam-sleek .page-content .notes-toolbar .notes-search-bar input) {
      padding: 10px 12px 10px 40px;
      font-size: 13px;
    }
    :global(.kalam-sleek .page-content .notes-toolbar .notes-search-bar .notes-search-bar-icon) {
      left: 12px;
    }
    :global(.kalam-sleek .page-content .notes-toolbar .notes-search-bar > span:not(.notes-search-bar-icon)) {
      left: 12px;
    }
  }
  /* Notes & Tasks: cycle sort mode (icon button; same four orders as former select). */
  :global(.kalam-sleek .page-content .notes-sort-cycle) {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 48px;
    min-width: 48px;
    align-self: stretch;
    min-height: 48px;
    padding: 0;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    cursor: pointer;
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .notes-sort-cycle:hover) {
    background: var(--bg-hover);
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .notes-sort-cycle:focus-visible) {
    outline: none;
    border-color: var(--border);
  }
  :global(.kalam-sleek .page-content .notes-sort-cycle :global(svg)) {
    font-size: 22px;
    width: 1em;
    height: 1em;
    display: block;
  }
  /* Notes: label filter — same 48px control as sort; popover aligns to the right under the button. */
  :global(.kalam-sleek .page-content .notes-label-filter-dropdown) {
    position: relative;
    flex-shrink: 0;
    align-self: stretch;
  }
  /* Notes / Tasks: one scope control (icon + label) opens popover matching label-filter shell. */
  :global(.kalam-sleek .page-content .notes-toolbar-scope-dropdown) {
    position: relative;
    flex-shrink: 0;
    align-self: stretch;
  }
  :global(.kalam-sleek .page-content .notes-toolbar-scope-trigger) {
    position: relative;
    width: auto !important;
    min-width: 48px;
    padding: 0 12px !important;
    gap: 8px;
  }
  :global(.kalam-sleek .page-content .notes-toolbar-scope-trigger .notes-toolbar-scope-trigger-label) {
    font-size: 13px;
    font-weight: 500;
    letter-spacing: -0.02em;
    color: var(--text);
    max-width: 92px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  :global(.kalam-sleek .page-content .notes-scope-menu.notes-label-filter-popover.notes-toolbar-scope-popover) {
    min-width: 240px;
    max-width: min(320px, 94vw);
  }
  :global(.kalam-sleek .page-content .notes-toolbar-scope-popover .notes-label-filter-popover-head) {
    border-bottom: 1px solid var(--border-light);
  }
  :global(.kalam-sleek .page-content .notes-toolbar-scope-popover .notes-label-filter-popover-meta) {
    margin-bottom: 0;
  }
  :global(.kalam-sleek .page-content .notes-toolbar-scope-menu-scroll) {
    min-height: 0 !important;
    padding: 4px 6px 8px;
  }
  :global(.kalam-sleek .page-content .notes-toolbar-scope-option-icon) {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }
  :global(.kalam-sleek .page-content .notes-toolbar-scope-option-icon :global(svg)) {
    font-size: 18px;
    width: 1em;
    height: 1em;
    display: block;
  }
  :global(.kalam-sleek .page-content .notes-toolbar-scope-popover .notes-scope-option) {
    gap: 10px;
    justify-content: flex-start;
  }
  :global(.kalam-sleek .page-content .notes-toolbar-scope-popover .notes-scope-option-label) {
    flex: 1 1 auto;
    min-width: 0;
    text-align: left;
  }
  :global(.kalam-sleek .page-content .notes-toolbar-scope-popover .notes-scope-option-count) {
    margin-left: auto;
    flex-shrink: 0;
    padding-left: 6px;
    font-size: 13px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .notes-label-filter-toggle) {
    position: relative;
  }
  :global(.kalam-sleek .page-content .notes-label-filter-toggle.has-filter) {
    border-color: var(--border);
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .notes-label-filter-count) {
    position: absolute;
    top: 3px;
    right: 3px;
    min-width: 17px;
    height: 17px;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--accent, #0a84ff);
    color: #fff;
    font-size: 10px;
    font-weight: 600;
    line-height: 17px;
    text-align: center;
    pointer-events: none;
    box-sizing: border-box;
  }
  :global(.kalam-sleek .page-content .notes-label-filter-search) {
    position: relative;
    flex-shrink: 0;
    margin-bottom: 6px;
  }
  :global(.kalam-sleek .page-content .notes-label-filter-search-icon) {
    position: absolute;
    left: 10px;
    top: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .notes-label-filter-search-icon :global(svg)) {
    font-size: 16px;
    width: 1em;
    height: 1em;
    display: block;
  }
  :global(.kalam-sleek .page-content .notes-label-filter-search input) {
    width: 100%;
    box-sizing: border-box;
    margin: 0;
    padding: 8px 10px 8px 34px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    font-size: 13px;
    font-family: var(--font-sleek);
  }
  :global(.kalam-sleek .page-content .notes-label-filter-search input:focus) {
    outline: none;
    border-color: var(--accent, #0a84ff);
  }
  :global(.kalam-sleek .page-content .notes-label-filter-search input::placeholder) {
    color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .notes-label-filter-clear-all) {
    flex-shrink: 0;
    align-self: flex-start;
    margin: 0 0 6px 2px;
    padding: 4px 6px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--accent, #0a84ff);
    font-size: 13px;
    font-weight: 600;
    font-family: var(--font-sleek);
    cursor: pointer;
  }
  :global(.kalam-sleek .page-content .notes-label-filter-clear-all:hover) {
    text-decoration: underline;
  }
  :global(.kalam-sleek .page-content .notes-label-filter-menu-scroll) {
    flex: 1 1 auto;
    min-height: 112px;
    overflow-y: auto;
    overflow-x: hidden;
    margin: 0;
    padding: 6px 6px 8px;
  }
  :global(.kalam-sleek .page-content .notes-label-filter-no-match) {
    padding: 16px 10px;
    font-size: 13px;
    color: var(--text-muted);
    font-weight: 500;
    font-family: var(--font-sleek);
    text-align: center;
  }
  :global(.kalam-sleek .page-content .notes-scope-dropdown) {
    position: relative;
    flex-shrink: 0;
  }
  /* Match .notes-search-bar input: elevated surface, md radius, no visible border until focus. */
  :global(.kalam-sleek .page-content .notes-scope-trigger) {
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-width: 168px;
    padding: 12px 14px;
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    font-family: var(--font-sleek);
    cursor: pointer;
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .notes-scope-trigger:hover) {
    background: var(--bg-hover);
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .notes-scope-trigger:focus-visible) {
    outline: none;
    border-color: var(--border);
  }
  :global(.kalam-sleek .page-content .notes-scope-caret) {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    transition: transform 0.2s ease;
    color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .notes-scope-caret.open) {
    transform: rotate(180deg);
  }
  :global(.kalam-sleek .page-content .notes-scope-caret :global(svg)) {
    font-size: 16px;
    display: block;
  }
  :global(.kalam-sleek .page-content .notes-scope-menu) {
    position: absolute;
    left: 0;
    top: calc(100% + 6px);
    min-width: 180px;
    padding: 6px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    box-shadow: var(--shadow);
    z-index: 30;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  /* Label filter: same family as `.notes-scope-menu` — elevated shell, inset list reads like the segmented control. */
  :global(.kalam-sleek .page-content .notes-scope-menu.notes-label-filter-popover) {
    left: auto;
    right: 0;
    z-index: 50;
    min-width: 272px;
    max-width: min(340px, 94vw);
    max-height: min(400px, 78vh);
    min-height: 0;
    padding: 0;
    gap: 0;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    box-shadow: var(--shadow);
    overflow: hidden;
  }
  :global(.kalam-sleek .page-content .notes-label-filter-popover-head) {
    padding: 10px 10px 10px;
    flex-shrink: 0;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border-light);
  }
  :global(.kalam-sleek .page-content .notes-label-filter-popover-meta) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
    margin-bottom: 8px;
  }
  :global(.kalam-sleek .page-content .notes-label-filter-popover-heading) {
    font-size: 13px;
    font-weight: 600;
    letter-spacing: -0.02em;
    color: var(--text);
    font-family: var(--font-sleek);
  }
  :global(.kalam-sleek .page-content .notes-label-filter-popover-head .notes-label-filter-search) {
    margin-bottom: 0;
  }
  :global(.kalam-sleek .page-content .notes-label-filter-popover-head .notes-label-filter-clear-all) {
    margin: 0;
    padding: 4px 8px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    border-radius: var(--radius-sm);
  }
  :global(.kalam-sleek .page-content .notes-label-filter-popover-head .notes-label-filter-clear-all:hover) {
    color: var(--text);
    background: var(--bg-hover);
    text-decoration: none;
  }
  :global(.kalam-sleek .page-content .notes-label-filter-popover-head .notes-label-filter-search input) {
    background: var(--bg-input);
    border: 1px solid transparent;
    border-radius: var(--radius-md);
  }
  :global(.kalam-sleek .page-content .notes-label-filter-popover-head .notes-label-filter-search input:focus) {
    border-color: var(--border);
    box-shadow: 0 0 0 3px var(--primary-alpha-subtle);
  }
  :global(.kalam-sleek .page-content .notes-label-filter-popover-body) {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    overflow: hidden;
  }
  :global(.kalam-sleek .page-content .notes-scope-option) {
    display: flex;
    align-items: center;
    width: 100%;
    padding: 10px 12px;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    font-family: var(--font-sleek);
    text-align: left;
    cursor: pointer;
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .notes-scope-option:hover) {
    background: var(--bg-hover);
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .notes-scope-option.active) {
    background: var(--bg-hover);
    color: var(--text);
    font-weight: 500;
  }
  /* Notes: scrollable label list inside popover (search + clear sit above). */
  :global(.kalam-sleek .page-content .notes-label-filter-menu) {
    min-width: 0;
  }
  :global(.kalam-sleek .page-content .notes-label-filter-row) {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 8px;
    margin: 0;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    font-family: var(--font-sleek);
    color: var(--text);
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .notes-label-filter-row:hover) {
    background: var(--bg-hover);
  }
  :global(.kalam-sleek .page-content .notes-label-filter-row input) {
    width: 16px;
    height: 16px;
    margin: 0;
    flex-shrink: 0;
    accent-color: var(--accent);
    cursor: pointer;
  }
  :global(.kalam-sleek .page-content .notes-sort-select) {
    flex-shrink: 0;
    padding: 12px 32px 12px 14px;
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    font-family: var(--font-sleek);
    cursor: pointer;
    transition: var(--transition-sleek);
    appearance: none;
    background: var(--bg-elevated)
      url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 256 256' fill='%236e6e73'%3E%3Cpath d='M216.49 104.49l-80 80a12 12 0 0 1-17 0l-80-80a12 12 0 0 1 17-17L128 159l71.51-71.52a12 12 0 0 1 17.17 17Z'/%3E%3C/svg%3E")
      no-repeat right 12px center;
  }
  :global(.kalam-sleek .page-content .notes-sort-select:hover) {
    background: var(--bg-hover)
      url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 256 256' fill='%236e6e73'%3E%3Cpath d='M216.49 104.49l-80 80a12 12 0 0 1-17 0l-80-80a12 12 0 0 1 17-17L128 159l71.51-71.52a12 12 0 0 1 17.17 17Z'/%3E%3C/svg%3E")
      no-repeat right 12px center;
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .notes-sort-select:focus) {
    outline: none;
    border-color: var(--border);
  }

  :global(.kalam-sleek .page-content .notes-search-bar) {
    position: relative;
    margin-bottom: var(--space-md);
  }
  /* Same as History .search-bar-icon: Iconify root centered inside the field (avoids stray transform / wrong fill). */
  :global(.kalam-sleek .page-content .notes-search-bar .notes-search-bar-icon) {
    position: absolute;
    left: 14px;
    top: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .notes-search-bar .notes-search-bar-icon :global(svg)) {
    font-size: 18px;
    width: 1em;
    height: 1em;
    display: block;
  }
  /* Prototype: Iconify root as direct child (no .notes-search-bar-icon), same geometry as Prototype.svelte .notes-search-bar :global(svg). */
  :global(.kalam-sleek .page-content .notes-search-bar > span:not(.notes-search-bar-icon)) {
    position: absolute;
    left: 14px;
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .notes-search-bar > span:not(.notes-search-bar-icon) :global(svg)) {
    font-size: 18px;
    width: 1em;
    height: 1em;
    display: block;
  }
  :global(.kalam-sleek .page-content .notes-search-bar input) {
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
    padding: 12px 14px 12px 44px;
    background: var(--bg-elevated);
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 14px;
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .notes-search-bar input:focus) {
    outline: none;
    border-color: var(--border);
  }
  :global(.kalam-sleek .page-content .notes-label-filters) {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: var(--space-lg);
  }
  /* Sibling of .notes-toolbar; uses sleek-page container (same width as the page column). */
  @container sleek-page (max-width: 480px) {
    :global(.kalam-sleek .page-content .notes-label-filters) {
      flex-wrap: nowrap;
      overflow-x: auto;
      overflow-y: hidden;
      padding-bottom: 6px;
      margin-bottom: var(--space-md);
      -webkit-overflow-scrolling: touch;
      scrollbar-width: thin;
    }
    :global(.kalam-sleek .page-content .notes-label-filters .label-chip) {
      flex-shrink: 0;
    }
  }
  :global(.kalam-sleek .page-content .label-chip) {
    padding: 6px 12px;
    border-radius: var(--radius-full);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: pointer;
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .label-chip:hover),
  :global(.kalam-sleek .page-content .label-chip.active) {
    background: var(--bg-hover);
    color: var(--text);
    border-color: var(--border);
  }
  :global(.kalam-sleek .page-content .notes-empty) {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-3xl);
    color: var(--text-muted);
    gap: var(--space-md);
  }
  :global(.kalam-sleek .page-content .notes-empty :global(svg)) {
    font-size: 48px;
  }
  :global(.kalam-sleek .page-content .notes-lists-container) {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xl);
  }
  :global(.kalam-sleek .page-content .notes-section-title) {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0 0 var(--space-lg) 0;
  }
  /* Auto-fill grid: card width grows with the window (up to max) instead of 3 skinny columns + giant 1:1 tiles when narrow. */
  :global(.kalam-sleek .page-content .notes-masonry) {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 252px), 1fr));
    gap: var(--space-lg);
    align-items: start;
  }
  /* Note cards: same token model as prototype — readable text on custom colors, shared footer border. */
  :global(.kalam-sleek .page-content .note-card) {
    width: min(100%, 400px);
    justify-self: center;
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    position: relative;
    text-align: left;
    border: none;
    transition: transform 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    cursor: pointer;
    height: auto;
    --note-fg: var(--text);
    --note-fg-secondary: var(--text-secondary);
    --note-fg-muted: var(--text-muted);
    --note-border: var(--border-light);
    --note-bg-hover: var(--bg-hover);
  }
  :global(.kalam-sleek .page-content .note-card.has-custom-color) {
    --note-fg: #1d1d1f;
    --note-fg-secondary: #424245;
    --note-fg-muted: #6e6e73;
    --note-border: rgba(0, 0, 0, 0.12);
    --note-bg-hover: rgba(0, 0, 0, 0.06);
  }
  :global(.kalam-sleek .page-content .note-card:hover) {
    transform: translateY(-2px);
    box-shadow: var(--shadow);
  }
  /* Pointer reorder: floating note that follows the cursor.
     Position is set via inline styles (fixed + left/top) in the component.
     Hit-testing must see through the lifted card to the drop target below. */
  :global(.kalam-sleek .page-content .note-card.dragging) {
    opacity: 0.92;
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.18), 0 0 0 1px rgba(0, 0, 0, 0.04);
    pointer-events: none;
    transition: none;
  }
  :global(.kalam-sleek .page-content .note-card.note-card--reorder .note-inner) {
    user-select: none;
    -webkit-user-select: none;
  }
  /* Wrapper for each card in the masonry to allow spacer + card together */
  :global(.kalam-sleek .page-content .note-card-wrapper) {
    break-inside: avoid;
    margin-bottom: 16px;
  }
  :global(.kalam-sleek .page-content .note-card-wrapper .note-card) {
    margin-bottom: 0;
  }
  /* Drop spacer - shows where the dragged card will land */
  :global(.kalam-sleek .page-content .note-drop-spacer) {
    width: 100%;
    height: 8px;
    margin: 4px 0;
    background: var(--accent-primary, #3b82f6);
    border-radius: 4px;
    opacity: 0.6;
    animation: note-drop-spacer-pulse 0.8s ease-in-out infinite;
    grid-column: 1 / -1;
  }
  @keyframes note-drop-spacer-pulse {
    0%, 100% { opacity: 0.4; transform: scaleX(0.95); }
    50% { opacity: 0.8; transform: scaleX(1); }
  }
  :global(.kalam-sleek .page-content .notes-masonry--reorder) {
    min-height: 3rem;
  }
  :global(.kalam-sleek .page-content .notes-masonry--empty-pinned),
  :global(.kalam-sleek .page-content .notes-masonry--empty-others) {
    min-height: 4.5rem;
    align-content: start;
  }
  :global(.kalam-sleek .page-content .notes-pin-drop-hint),
  :global(.kalam-sleek .page-content .notes-unpin-drop-hint) {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 3.5rem;
    padding: var(--space-md);
    border: 1px dashed var(--border-visible);
    border-radius: var(--radius-md);
    color: var(--text-muted);
    font-size: 0.875rem;
    background: var(--bg-input);
  }
  :global(.kalam-sleek .page-content .note-card.pinned::before) {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: var(--accent);
  }
  :global(.kalam-sleek .page-content .note-card.has-custom-color.pinned::before) {
    background: var(--note-fg-muted);
  }
  :global(.kalam-sleek .page-content .note-inner) {
    padding: var(--space-lg);
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  :global(.kalam-sleek .page-content .note-card .note-title) {
    font-size: 16px;
    font-weight: 600;
    line-height: 1.35;
    margin: 0;
    color: var(--note-fg);
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    word-break: break-word;
  }
  :global(.kalam-sleek .page-content .note-card .note-title.note-title-placeholder) {
    color: var(--note-fg-muted);
    font-weight: 500;
  }
  :global(.kalam-sleek .page-content .note-card .note-preview) {
    font-size: 12px;
    line-height: 1.45;
    font-weight: 400;
    margin: 0;
    color: var(--note-fg-secondary);
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    word-break: break-word;
  }
  :global(.kalam-sleek .page-content .note-card-meta) {
    margin-top: auto;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  :global(.kalam-sleek .page-content .note-tags-row) {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  :global(.kalam-sleek .page-content .note-tag) {
    font-size: 11px;
    padding: 2px 8px;
    background: var(--note-bg-hover);
    color: var(--note-fg-secondary);
    border-radius: 4px;
  }
  :global(.kalam-sleek .page-content .note-reminder-row) {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--note-fg-muted);
    flex-shrink: 0;
  }
  :global(.kalam-sleek .page-content .note-reminder-row :global(svg)) {
    font-size: 14px;
  }
  :global(.kalam-sleek .page-content .note-footer) {
    padding: 12px var(--space-lg);
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid var(--note-border);
    flex-shrink: 0;
    background: inherit;
  }
  :global(.kalam-sleek .page-content .note-date) {
    font-size: 12px;
    color: var(--note-fg-muted);
  }
  :global(.kalam-sleek .page-content .note-actions) {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    justify-content: flex-end;
    max-width: 100%;
  }
  :global(.kalam-sleek .page-content .note-action-btn) {
    width: 21px;
    height: 21px;
    border-radius: 5px;
    border: none;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    color: var(--note-fg-muted);
    transition: var(--transition-sleek);
  }
  /* Archive / trash scopes: readable labels next to icons on note cards. */
  :global(.kalam-sleek .page-content .note-action-btn.note-action-btn--labeled) {
    width: auto;
    min-height: 21px;
    height: auto;
    padding: 4px 7px;
    gap: 5px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-sleek);
    line-height: 1.2;
  }
  :global(.kalam-sleek .page-content .note-action-btn.note-action-btn--labeled :global(svg)) {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
  }
  :global(.kalam-sleek .page-content .note-action-btn :global(svg)) {
    width: 14px;
    height: 14px;
  }
  :global(.kalam-sleek .page-content .note-action-btn:hover) {
    background: var(--note-bg-hover);
    color: var(--note-fg);
  }
  :global(.kalam-sleek .page-content .note-action-btn.delete:hover) {
    color: #FF3B30;
    background: rgba(255, 59, 48, 0.1);
  }
  :global(.kalam-sleek .page-content .pin-icon) {
    position: absolute;
    top: var(--space-md);
    right: var(--space-md);
    color: var(--note-fg-muted);
    font-size: 14px;
  }

  /* Sleek editor (note/task/reminder/snippet detail) — from prototype */
  :global(.kalam-sleek .page-content .sleek-header) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-2xl);
  }
  :global(.kalam-sleek .page-content .sleek-back) {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 15px;
    font-weight: 500;
    cursor: pointer;
    padding: 8px 0;
    transition: var(--transition);
  }
  :global(.kalam-sleek .page-content .sleek-back:hover) {
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .sleek-actions) {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }
  /* Match .btn-primary (New Note, etc.): same height, weight, and radius for detail headers. */
  :global(.kalam-sleek .page-content .sleek-cancel) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 10px 18px;
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    background: var(--bg);
    color: var(--text);
    font-size: 14px;
    font-weight: 600;
    font-family: var(--font-sleek);
    cursor: pointer;
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .sleek-cancel:hover) {
    background: var(--bg-hover);
    transform: translateY(-1px);
  }
  :global(.kalam-sleek .page-content .sleek-save) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 10px 18px;
    border: none;
    border-radius: var(--radius-full);
    background: var(--text);
    color: var(--bg);
    font-size: 14px;
    font-weight: 600;
    font-family: var(--font-sleek);
    cursor: pointer;
    transition: var(--transition-sleek);
  }
  :global(.kalam-sleek .page-content .sleek-save:hover:not(:disabled)) {
    opacity: 0.9;
    transform: translateY(-1px);
  }
  :global(.kalam-sleek .page-content .sleek-save:disabled) {
    opacity: 0.5;
    cursor: not-allowed;
  }
  :global(.kalam-sleek .page-content .sleek-cancel :global(svg)),
  :global(.kalam-sleek .page-content .sleek-save :global(svg)) {
    font-size: 18px;
    width: 1em;
    height: 1em;
    flex-shrink: 0;
  }
  :global(.kalam-sleek .page-content .sleek-icon-btn) {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: var(--transition);
  }
  :global(.kalam-sleek .page-content .sleek-icon-btn:hover:not(:disabled)) {
    color: var(--text);
    background: var(--bg-hover);
  }
  :global(.kalam-sleek .page-content .sleek-icon-btn.danger:hover) {
    color: #FF3B30;
    background: rgba(255, 59, 48, 0.1);
  }
  /* Icon buttons in editor headers sit next to .sleek-cancel / .sleek-save (New Note–sized pills). */
  :global(.kalam-sleek .page-content .sleek-header .sleek-actions > .sleek-icon-btn) {
    width: 36px;
    height: 36px;
    flex-shrink: 0;
  }
  :global(.kalam-sleek .page-content .sleek-header .sleek-actions > .sleek-icon-btn :global(svg)) {
    width: 18px;
    height: 18px;
  }
  :global(.kalam-sleek .page-content .sleek-body) {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }
  :global(.kalam-sleek .page-content .sleek-title) {
    width: 100%;
    font-size: 36px;
    font-weight: 700;
    letter-spacing: -0.03em;
    border: 1px solid var(--border-light);
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    color: var(--text);
    outline: none;
    padding: 16px 20px;
    margin-bottom: var(--space-md);
    font-family: var(--font);
    transition: all 0.2s ease;
  }
  :global(.kalam-sleek .page-content .sleek-title:hover) {
    border-color: var(--border);
  }
  :global(.kalam-sleek .page-content .sleek-title:focus) {
    border-color: var(--text-muted);
    background: var(--bg);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.08);
  }
  :global(.kalam-sleek .page-content .sleek-title::placeholder) {
    color: var(--text-muted);
    opacity: 0.6;
  }
  @media (max-width: 960px) {
    :global(.kalam-sleek .page-content .sleek-title) {
      font-size: 26px;
      padding: 12px 14px;
    }
  }
  :global(.kalam-sleek .page-content .sleek-labels) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-top: var(--space-xl);
    margin-bottom: var(--space-md);
    color: var(--text-muted);
    padding: 0 4px;
  }
  :global(.kalam-sleek .page-content .sleek-label-chip) {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 14px;
    color: var(--text-secondary);
    background: var(--bg-elevated);
    padding: 4px 10px;
    border-radius: var(--radius-full);
    border: 1px solid var(--border-light);
  }
  :global(.kalam-sleek .page-content .sleek-label-chip button) {
    background: transparent;
    border: none;
    padding: 0;
    color: inherit;
    cursor: pointer;
    display: flex;
    opacity: 0.6;
  }
  :global(.kalam-sleek .page-content .sleek-label-chip button:hover) {
    opacity: 1;
    color: #FF3B30;
  }
  :global(.kalam-sleek .page-content .sleek-label-input) {
    border: 1px solid var(--border-light);
    background: var(--bg-elevated);
    border-radius: var(--radius-full);
    color: var(--text);
    font-size: 14px;
    outline: none;
    width: 140px;
    padding: 6px 12px;
    font-family: var(--font);
    transition: all 0.2s ease;
  }
  :global(.kalam-sleek .page-content .sleek-label-input:focus) {
    border-color: var(--text-muted);
    background: var(--bg);
  }
  :global(.kalam-sleek .page-content .sleek-label-input::placeholder) {
    color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .sleek-content) {
    width: 100%;
    flex: 1;
    font-size: 16px;
    line-height: 1.6;
    border: 1px solid var(--border-light);
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    color: var(--text);
    outline: none;
    padding: 20px;
    resize: none;
    font-family: var(--font);
    min-height: 300px;
    transition: all 0.2s ease;
  }
  :global(.kalam-sleek .page-content .sleek-content:hover) {
    border-color: var(--border);
  }
  :global(.kalam-sleek .page-content .sleek-content:focus) {
    border-color: var(--text-muted);
    background: var(--bg);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.08);
  }
  :global(.kalam-sleek .page-content .sleek-content::placeholder) {
    color: var(--text-muted);
    opacity: 0.6;
  }
  :global(.kalam-sleek .page-content .sleek-footer) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: var(--space-xl);
    margin-top: var(--space-xl);
  }
  :global(.kalam-sleek .page-content .sleek-meta) {
    font-size: 13px;
    color: var(--text-muted);
    font-weight: 500;
  }
  :global(.kalam-sleek .page-content .sleek-tools) {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  :global(.kalam-sleek .page-content .color-dropdown-container),
  :global(.kalam-sleek .page-content .reminder-dropdown-container) {
    position: relative;
  }
  :global(.kalam-sleek .page-content .current-color-indicator) {
    display: block;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1px solid var(--border);
    position: relative;
    overflow: hidden;
  }
  :global(.kalam-sleek .page-content .sleek-popover) {
    position: absolute;
    bottom: calc(100% + 12px);
    left: 50%;
    transform: translateX(-50%);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 12px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
    z-index: 100;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  :global(.kalam-sleek .page-content .color-popover) {
    width: 160px;
  }
  :global(.kalam-sleek .page-content .sleek-colors-grid) {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: center;
  }
  :global(.kalam-sleek .page-content .sleek-datetime-input) {
    border: 1px solid var(--border-light);
    background: var(--bg);
    color: var(--text);
    /* Match sleek row fields: subtask-input, add-subtask-input, sleek-label-input (14px). */
    font-size: 14px;
    font-family: var(--font);
    outline: none;
    cursor: pointer;
    padding: 8px 12px;
    border-radius: var(--radius-md);
  }
  :global(.kalam-sleek .page-content .sleek-clear-btn) {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-muted);
    padding: 6px 12px;
    border-radius: var(--radius-md);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  :global(.kalam-sleek .page-content .sleek-clear-btn:hover) {
    background: rgba(255, 59, 48, 0.1);
    color: #FF3B30;
    border-color: rgba(255, 59, 48, 0.2);
  }
  :global(.kalam-sleek .page-content .sleek-color-dot) {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 1px solid var(--border);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.15s ease, border-color 0.15s ease;
  }
  :global(.kalam-sleek .page-content .sleek-color-dot[title="default"]) {
    position: relative;
    overflow: hidden;
  }
  :global(.kalam-sleek .page-content .sleek-color-dot[title="default"]::after) {
    content: '';
    position: absolute;
    top: 50%;
    left: -20%;
    width: 140%;
    height: 1px;
    background-color: #ff3b30;
    transform: rotate(-45deg);
    opacity: 0.7;
  }
  :global(.kalam-sleek .page-content .sleek-color-dot:hover) {
    transform: scale(1.15);
  }
  :global(.kalam-sleek .page-content .sleek-color-dot.selected) {
    transform: scale(1.15);
    border-color: var(--text);
    border-width: 2px;
  }
  :global(.kalam-sleek .page-content .sleek-tool-btn) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: var(--transition);
  }
  :global(.kalam-sleek .page-content .sleek-tool-btn:hover),
  :global(.kalam-sleek .page-content .sleek-tool-btn.active) {
    color: var(--text);
    background: var(--bg-elevated);
  }
  :global(.kalam-sleek .page-content .sleek-tool-btn :global(svg)) {
    font-size: 24px;
  }
  :global(.kalam-sleek .page-content .sleek-tool-btn.complete-toggle) {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    width: auto;
    height: auto;
    border-radius: var(--radius-full);
    font-size: 13px;
    font-weight: 500;
  }
  :global(.kalam-sleek .page-content .sleek-tool-btn.complete-toggle span) {
    font-size: 13px;
  }
  :global(.kalam-sleek .page-content .sleek-tool-btn.complete-toggle.completed) {
    color: #34C759;
    background: rgba(52, 199, 89, 0.1);
  }
  :global(.kalam-sleek .page-content .sleek-tool-btn.complete-toggle.completed:hover) {
    background: rgba(52, 199, 89, 0.15);
  }
  :global(.kalam-sleek .page-content .sleek-tool-btn.complete-toggle.compact) {
    width: 36px;
    height: 36px;
    padding: 0;
  }
  :global(.kalam-sleek .page-content .sleek-tool-btn.complete-toggle.compact :global(svg)) {
    font-size: 20px;
  }
  /* Complete control removed from here, styled locally in TaskDetail */
  :global(.kalam-sleek .page-content .task-desc) {
    min-height: 120px;
    margin-bottom: var(--space-xl);
  }
  :global(.kalam-sleek .page-content .section-title) {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: var(--space-md);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  :global(.kalam-sleek .page-content .due-date-section) {
    margin-bottom: var(--space-2xl);
  }
  :global(.kalam-sleek .page-content .due-date-input-row) {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-light);
    border-radius: var(--radius-md);
    transition: all 0.2s ease;
  }
  :global(.kalam-sleek .page-content .due-date-input-row:focus-within) {
    border-color: var(--text-muted);
    background: var(--bg);
  }
  /* Svelty field: no white inset; surface matches the row so :focus-within fade applies as one unit */
  :global(.kalam-sleek .page-content .due-date-input-row .sleek-datetime-input) {
    background: transparent;
    border-color: transparent;
    box-shadow: none;
    transition: color 0.2s ease;
  }
  :global(.kalam-sleek .page-content .due-date-input-row :global(svg)) {
    color: var(--text-muted);
    font-size: 20px;
  }
  :global(.kalam-sleek .page-content .subtasks-section) {
    margin-bottom: var(--space-2xl);
  }
  :global(.kalam-sleek .page-content .subtasks-list) {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  :global(.kalam-sleek .page-content .subtask-row) {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-light);
    border-radius: var(--radius-md);
    transition: all 0.2s ease;
  }
  :global(.kalam-sleek .page-content .subtask-row:focus-within) {
    border-color: var(--text-muted);
    background: var(--bg);
  }
  :global(.kalam-sleek .page-content .subtask-row.completed) {
    opacity: 0.6;
  }
  :global(.kalam-sleek .page-content .subtask-row.completed .subtask-input) {
    text-decoration: line-through;
  }
  /* Drag grip chip on elevated rows (tasks list + subtasks): border + bg so the handle reads clearly. */
  :global(.kalam-sleek .page-content .task-row .drag-handle),
  :global(.kalam-sleek .page-content .subtask-row .drag-handle) {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 28px;
    min-height: 28px;
    padding: 4px 6px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-light);
    background: var(--bg);
    color: var(--text-muted);
    cursor: grab;
    font-size: 16px;
    opacity: 0.65;
    transition: var(--transition);
  }
  :global(.kalam-sleek .page-content .task-row:hover .drag-handle),
  :global(.kalam-sleek .page-content .subtask-row:hover .drag-handle) {
    opacity: 0.9;
    border-color: var(--border);
    color: var(--text-secondary);
  }
  :global(.kalam-sleek .page-content .task-row .drag-handle:hover),
  :global(.kalam-sleek .page-content .subtask-row .drag-handle:hover) {
    background: var(--bg-hover);
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .task-row .drag-handle:active),
  :global(.kalam-sleek .page-content .subtask-row .drag-handle:active) {
    cursor: grabbing;
  }
  :global(.kalam-sleek .page-content .subtask-input) {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 14px;
    font-family: var(--font);
    outline: none;
  }
  :global(.kalam-sleek .page-content .remove-subtask) {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: opacity 0.2s ease;
  }
  :global(.kalam-sleek .page-content .remove-subtask:hover) {
    color: #FF3B30;
  }
  :global(.kalam-sleek .page-content .add-subtask-row) {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .add-subtask-input) {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 14px;
    font-family: var(--font);
    outline: none;
  }
  :global(.kalam-sleek .page-content .add-subtask-input::placeholder) {
    color: var(--text-muted);
  }
  /* Compact priority: same footprint as old segmented control; menu is custom-styled (not native select). */
  :global(.kalam-sleek .page-content .task-priority-popover) {
    position: relative;
    flex-shrink: 0;
  }
  :global(.kalam-sleek .page-content .task-priority-trigger) {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 36px;
    min-width: 118px;
    max-width: 132px;
    padding: 0 10px;
    margin: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-family: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: border-color 0.2s ease, background 0.2s ease;
  }
  :global(.kalam-sleek .page-content .task-priority-trigger:hover) {
    border-color: var(--text-muted);
    background: var(--bg);
  }
  :global(.kalam-sleek .page-content .task-priority-trigger:focus-visible) {
    outline: 2px solid var(--text-muted);
    outline-offset: 2px;
  }
  :global(.kalam-sleek .page-content .task-priority-trigger-flag) {
    display: flex;
    font-size: 16px;
    flex-shrink: 0;
  }
  :global(.kalam-sleek .page-content .task-priority-trigger-flag.p0 :global(svg)) {
    color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .task-priority-trigger-flag.p1 :global(svg)) {
    color: #34c759;
  }
  :global(.kalam-sleek .page-content .task-priority-trigger-flag.p2 :global(svg)) {
    color: #ff9500;
  }
  :global(.kalam-sleek .page-content .task-priority-trigger-flag.p3 :global(svg)) {
    color: #ff3b30;
  }
  :global(.kalam-sleek .page-content .task-priority-trigger-text) {
    flex: 1;
    min-width: 0;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  :global(.kalam-sleek .page-content .task-priority-trigger-caret) {
    display: flex;
    font-size: 14px;
    flex-shrink: 0;
    color: var(--text-muted);
    transition: transform 0.2s ease;
  }
  :global(.kalam-sleek .page-content .task-priority-trigger[aria-expanded='true'] .task-priority-trigger-caret) {
    transform: rotate(180deg);
  }
  :global(.kalam-sleek .page-content .task-priority-menu) {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 40;
    min-width: 220px;
    margin: 0;
    padding: 6px;
    list-style: none;
    background: var(--bg-elevated);
    border: 1px solid var(--border-light);
    border-radius: var(--radius-lg);
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.12);
  }
  :global(.kalam-sleek.dark .page-content .task-priority-menu) {
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.45);
  }
  :global(.kalam-sleek .page-content .task-priority-option) {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 10px 12px;
    margin: 0;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text);
    font-family: inherit;
    font-size: 14px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s ease;
  }
  :global(.kalam-sleek .page-content .task-priority-option:hover) {
    background: var(--bg-hover);
  }
  :global(.kalam-sleek .page-content .task-priority-option.selected) {
    background: var(--bg);
    font-weight: 600;
  }
  :global(.kalam-sleek .page-content .task-priority-option-flag) {
    display: flex;
    font-size: 18px;
    flex-shrink: 0;
  }
  :global(.kalam-sleek .page-content .task-priority-option-flag.p0 :global(svg)) {
    color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .task-priority-option-flag.p1 :global(svg)) {
    color: #34c759;
  }
  :global(.kalam-sleek .page-content .task-priority-option-flag.p2 :global(svg)) {
    color: #ff9500;
  }
  :global(.kalam-sleek .page-content .task-priority-option-flag.p3 :global(svg)) {
    color: #ff3b30;
  }
  :global(.kalam-sleek .page-content .task-priority-option-label) {
    flex: 1;
    min-width: 0;
  }
  :global(.kalam-sleek .page-content .task-priority-option-check) {
    display: flex;
    font-size: 18px;
    flex-shrink: 0;
    color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .sleek-editor-page .checkbox.small) {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 1px solid var(--border);
    background: var(--bg);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    color: var(--text);
    padding: 0;
  }
  :global(.kalam-sleek .page-content .subtask-row.completed .checkbox.small) {
    background: var(--text);
    color: var(--bg);
    border-color: var(--text);
  }

  /* Tasks list — from prototype; section gap matches `.notes-lists-container` (Pinned / Others). */
  :global(.kalam-sleek .page-content .task-list-large) {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xl);
  }
  :global(.kalam-sleek .page-content .task-list-large .task-list-section-rows) {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  :global(.kalam-sleek .page-content .task-row) {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    background: var(--bg-elevated);
    border-radius: var(--radius-md);
    transition: var(--transition);
    cursor: pointer;
    border: 1px solid transparent;
  }
  /* Keep pointer (click) cursor — whole row drags without a handle; grab/grabbing would imply a different affordance. */
  :global(.kalam-sleek .page-content .task-row.task-row--reorder) {
    cursor: pointer;
    user-select: none;
    -webkit-user-select: none;
  }
  :global(.kalam-sleek .page-content .task-row.task-row--reorder .checkbox),
  :global(.kalam-sleek .page-content .task-row.task-row--reorder .task-row-actions) {
    cursor: pointer;
    user-select: none;
  }
  :global(.kalam-sleek .page-content .task-row:hover) {
    background: var(--bg-hover);
    border-color: var(--border-light);
  }
  :global(.kalam-sleek .page-content .task-list-large .task-row-wrapper) {
    width: 100%;
  }
  :global(.kalam-sleek .page-content .task-drop-spacer) {
    width: 100%;
    height: 6px;
    margin: 2px 0;
    background: var(--accent-primary, #3b82f6);
    border-radius: 3px;
    opacity: 0.6;
    animation: note-drop-spacer-pulse 0.8s ease-in-out infinite;
    flex-shrink: 0;
  }
  :global(.kalam-sleek .page-content .task-row.task-row--reorder.dragging) {
    cursor: pointer;
  }
  :global(.kalam-sleek .page-content .task-row.dragging) {
    opacity: 0.92;
    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.14), 0 0 0 1px rgba(0, 0, 0, 0.04);
    pointer-events: none;
    transition: none;
  }
  :global(.kalam-sleek .page-content .task-row.completed) {
    opacity: 0.6;
  }
  :global(.kalam-sleek .page-content .task-row.completed .task-title) {
    text-decoration: line-through;
    color: var(--text-secondary);
  }
  :global(.kalam-sleek .page-content .drag-handle:hover) {
    background: var(--bg);
    color: var(--text);
  }
  :global(.kalam-sleek .page-content .drag-handle:active) {
    cursor: grabbing;
  }
  :global(.kalam-sleek .page-content .task-row .checkbox) {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 1px solid var(--border);
    background: var(--bg);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    color: var(--text);
    padding: 0;
    transition: all 0.2s ease;
  }
  :global(.kalam-sleek .page-content .task-row .checkbox:hover) {
    border-color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .task-row.completed .checkbox) {
    background: var(--text);
    color: var(--bg);
    border-color: var(--text);
  }
  :global(.kalam-sleek .page-content .task-info) {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  :global(.kalam-sleek .page-content .task-title) {
    font-size: 15px;
    font-weight: 500;
  }
  :global(.kalam-sleek .page-content .task-meta) {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  :global(.kalam-sleek .page-content .task-due) {
    font-size: 12px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 4px;
  }
  :global(.kalam-sleek .page-content .task-due.urgent) {
    color: #FF3B30;
  }
  :global(.kalam-sleek .page-content .task-subtasks-count) {
    font-size: 12px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 4px;
  }
  :global(.kalam-sleek .page-content .task-tags) {
    display: flex;
    gap: 6px;
  }
  :global(.kalam-sleek .page-content .task-tag-pill) {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }
  :global(.kalam-sleek .page-content .task-row-actions) {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    margin-left: auto;
  }
  :global(.kalam-sleek .page-content .task-row--trash .task-row-actions) {
    margin-left: auto;
  }
  /* Narrow page: keep one row per task — wrapping pushed `.task-row-actions` to a full-width line and doubled row height. */
  @container sleek-page (max-width: 720px) {
    :global(.kalam-sleek .page-content .task-row) {
      flex-wrap: nowrap;
      align-items: center;
    }
    :global(.kalam-sleek .page-content .task-info) {
      flex: 1 1 auto;
      min-width: 0;
    }
    :global(.kalam-sleek .page-content .task-row .task-title) {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    :global(.kalam-sleek .page-content .task-meta) {
      flex-wrap: nowrap;
      min-width: 0;
      overflow: hidden;
    }
    :global(.kalam-sleek .page-content .task-tags) {
      flex: 0 1 auto;
      min-width: 0;
      flex-wrap: nowrap;
      overflow-x: auto;
    }
  }
  @container sleek-page (max-width: 640px) {
    :global(.kalam-sleek .page-content .sleek-header) {
      flex-wrap: wrap;
      align-items: flex-start;
      gap: var(--space-md);
    }
    :global(.kalam-sleek .page-content .sleek-actions) {
      flex-wrap: wrap;
      justify-content: flex-end;
      width: 100%;
      max-width: 100%;
    }
  }
  :global(.kalam-sleek .page-content .priority-indicator) {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  /* Reminders list — from prototype */
  :global(.kalam-sleek .page-content .reminder-list-large) {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  :global(.kalam-sleek .page-content .reminder-row) {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 20px;
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    transition: var(--transition);
    cursor: pointer;
    border: 1px solid transparent;
  }
  :global(.kalam-sleek .page-content .reminder-row:hover) {
    background: var(--bg-hover);
    border-color: var(--border-light);
  }
  :global(.kalam-sleek .page-content .reminder-icon-large) {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: var(--bg);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 20px;
    color: var(--text-secondary);
  }
  :global(.kalam-sleek .page-content .reminder-icon-large.recurring) {
    color: var(--accent);
    background: rgba(0, 122, 255, 0.1);
  }
  :global(.kalam-sleek .page-content .reminder-info) {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  :global(.kalam-sleek .page-content .reminder-title-row) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  :global(.kalam-sleek .page-content .reminder-text) {
    font-size: 15px;
    font-weight: 500;
  }
  /* Match History row `.chip` (e.g. chip-stt): radius, type scale, weight */
  :global(.kalam-sleek .page-content .reminder-source-badge) {
    font-size: 11px;
    font-weight: 500;
    padding: 4px 8px;
    border-radius: 6px;
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  :global(.kalam-sleek .page-content .reminder-source-badge.note) {
    background: #fef08a;
    color: #854d0e;
    border-color: #fde047;
  }
  :global(.kalam-sleek .page-content .reminder-source-badge.task) {
    background: #bfdbfe;
    color: #1e40af;
    border-color: #93c5fd;
  }
  :global(.kalam-sleek .page-content .reminder-meta) {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  :global(.kalam-sleek .page-content .reminder-time) {
    font-size: 13px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 4px;
  }
  :global(.kalam-sleek .page-content .reminder-recurring-badge) {
    font-size: 12px;
    color: var(--accent);
    display: flex;
    align-items: center;
    gap: 4px;
    font-weight: 500;
  }
  :global(.kalam-sleek .page-content .reminder-tags) {
    display: flex;
    gap: 6px;
  }
  :global(.kalam-sleek .page-content .reminder-tag) {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }
  :global(.kalam-sleek .page-content .reminder-form-row) {
    margin-bottom: var(--space-xl);
  }
  :global(.kalam-sleek .page-content .form-label) {
    display: block;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: var(--space-sm);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  :global(.kalam-sleek .page-content .form-select) {
    width: 100%;
    padding: 12px 16px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-light);
    border-radius: var(--radius-md);
    color: var(--text);
    font-size: 15px;
    font-family: var(--font);
    cursor: pointer;
    transition: all 0.2s ease;
  }
  :global(.kalam-sleek .page-content .form-select:focus) {
    outline: none;
    border-color: var(--text-muted);
    background: var(--bg);
  }
  :global(.kalam-sleek .page-content .sleek-datetime-input.full-width) {
    width: 100%;
    box-sizing: border-box;
  }

  /* Snippets list — from prototype */
  :global(.kalam-sleek .page-content .snippets-grid) {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: var(--space-lg);
  }
  :global(.kalam-sleek .page-content .snippet-card) {
    padding: var(--space-lg);
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    transition: var(--transition);
    cursor: pointer;
    border: 1px solid transparent;
  }
  :global(.kalam-sleek .page-content .snippet-card:hover) {
    background: var(--bg-hover);
    border-color: var(--border-light);
  }
  :global(.kalam-sleek .page-content .snippet-header) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }
  :global(.kalam-sleek .page-content .trigger-code) {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    background: var(--bg);
    padding: 4px 8px;
    border-radius: 6px;
    border: 1px solid var(--border-light);
  }
  :global(.kalam-sleek .page-content .uses-count) {
    font-size: 12px;
    color: var(--text-muted);
  }
  :global(.kalam-sleek .page-content .expansion-text) {
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-secondary);
    margin: 0;
    white-space: pre-line;
  }
  :global(.kalam-sleek .page-content .snippet-tags) {
    display: flex;
    gap: 6px;
    margin-top: 12px;
    flex-wrap: wrap;
  }
  :global(.kalam-sleek .page-content .snippet-tag) {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }
  :global(.kalam-sleek .page-content .snippet-form-row) {
    margin-bottom: var(--space-xl);
  }
  :global(.kalam-sleek .page-content .trigger-input-wrapper) {
    position: relative;
    display: flex;
    align-items: center;
  }
  :global(.kalam-sleek .page-content .trigger-prefix) {
    position: absolute;
    left: 16px;
    font-size: 24px;
    color: var(--text-muted);
    font-weight: 500;
  }
  :global(.kalam-sleek .page-content .trigger-input) {
    padding-left: 32px;
  }
  :global(.kalam-sleek .page-content .snippet-expansion) {
    min-height: 200px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  }

</style>
