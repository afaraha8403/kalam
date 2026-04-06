<script lang="ts">
  /**
   * Dictation modes page — unified card list.
   *
   * All modes are equal card rows; the selected mode has a left accent bar
   * indicator. Click any row to select it; gear icon opens the side panel
   * for editing. Global Polish and Context toggles live in a processing bar.
   */
  import { onDestroy, onMount } from 'svelte'
  import { fade, fly, slide } from 'svelte/transition'
  import { invoke, listenSafe, isTauriRuntime } from '$lib/backend'
  import Icon from '@iconify/svelte'
  import RecipeLibrary from '../components/RecipeLibrary.svelte'
  import { formatHotkeyForDisplay, superKeyLabel } from '$lib/platformHotkey'
  import { effectiveModeAccent, MODE_ACCENT_PRESETS } from '$lib/modeAccent'
  import type {
    AppConfig,
    AutoActivateRule,
    CatalogProvider,
    DictationMode,
    ModeModelRef,
    PolishConfig,
  } from '../types'

  type AppListEntry = {
    process_name: string
    display_name: string
    icon_base64?: string | null
    exe_path?: string | null
  }

  let config: AppConfig | null = null
  let modelCatalog: CatalogProvider[] = []
  let loadError = ''
  let platform = 'windows'
  let saving = false
  let fileInputEl: HTMLInputElement | null = null

  let isPanelOpen = false
  let editorModeId = ''

  /** Which panel sections are expanded (progressive disclosure). */
  let panelSections: Record<string, boolean> = {
    voice: false,
    ai: false,
    context: false,
    rules: false,
  }

  let autoRuleTestHint: Record<number, string> = {}
  let appPickerRuleIndex: number | null = null
  let runningAppsForPicker: AppListEntry[] = []
  let loadingRunningApps = false

  /** Curated Iconify ids + search tokens (name, synonyms) for the picker. */
  const ICON_CATALOG: { id: string; search: string }[] = [
    { id: 'ph:chat-circle', search: 'chat message talk conversation bubble' },
    { id: 'ph:envelope-simple', search: 'email mail letter inbox' },
    { id: 'ph:article', search: 'article document text writing blog' },
    { id: 'ph:code-block', search: 'code programming developer' },
    { id: 'ph:magic-wand', search: 'magic ai sparkle transform' },
    { id: 'ph:translate', search: 'translate language international' },
    { id: 'ph:list-dashes', search: 'list bullet todo tasks' },
    { id: 'ph:check-square', search: 'check task done checkbox' },
    { id: 'ph:terminal', search: 'terminal console shell command' },
    { id: 'ph:magnifying-glass', search: 'search find magnify' },
    { id: 'ph:speaker-hifi', search: 'speaker audio voice sound' },
    { id: 'ph:brain', search: 'brain think idea smart' },
    { id: 'ph:note', search: 'note memo sticky' },
    { id: 'ph:bug', search: 'bug debug issue' },
    { id: 'ph:lightbulb', search: 'lightbulb idea tip' },
    { id: 'ph:rocket', search: 'rocket launch fast ship' },
    { id: 'ph:microphone', search: 'microphone record dictation voice' },
    { id: 'ph:phone', search: 'phone call mobile' },
    { id: 'ph:video-camera', search: 'video camera meeting zoom' },
    { id: 'ph:calendar', search: 'calendar schedule date' },
    { id: 'ph:clock', search: 'clock time reminder' },
    { id: 'ph:folder', search: 'folder files project' },
    { id: 'ph:book-open', search: 'book read documentation' },
    { id: 'ph:graduation-cap', search: 'school learn teach education' },
    { id: 'ph:briefcase', search: 'briefcase work business job' },
    { id: 'ph:heart', search: 'heart favorite love' },
    { id: 'ph:star', search: 'star favorite important' },
    { id: 'ph:fire', search: 'fire hot urgent' },
    { id: 'ph:shield-check', search: 'shield security privacy safe' },
    { id: 'ph:lock', search: 'lock private secure' },
    { id: 'ph:globe', search: 'globe web internet world' },
    { id: 'ph:users', search: 'users team people group' },
    { id: 'ph:user', search: 'user person profile' },
    { id: 'ph:paper-plane-tilt', search: 'send paper plane message slack' },
    { id: 'ph:chats-circle', search: 'chats multiple threads' },
    { id: 'ph:pen-nib', search: 'pen write edit signature' },
    { id: 'ph:pencil-simple', search: 'pencil draw sketch edit' },
    { id: 'ph:quotes', search: 'quotes citation reference' },
    { id: 'ph:hash', search: 'hash tag channel slack' },
    { id: 'ph:brackets-curly', search: 'brackets json code data' },
    { id: 'ph:tree-structure', search: 'tree structure outline hierarchy' },
    { id: 'ph:clipboard-text', search: 'clipboard paste copy text' },
    { id: 'ph:file-text', search: 'file document page' },
    { id: 'ph:scissors', search: 'scissors cut trim edit' },
    { id: 'ph:wrench', search: 'wrench tools settings fix' },
    { id: 'ph:gear', search: 'gear settings configuration' },
    { id: 'ph:coffee', search: 'coffee break casual' },
    { id: 'ph:moon-stars', search: 'moon night dark mode' },
    { id: 'ph:sun', search: 'sun day bright' },
  ]

  let iconPickerOpen = false
  let iconSearchQuery = ''

  /** Approximate hex for native color input when accent is not #rrggbb. */
  function accentHexForPicker(accent: string | undefined | null): string {
    const t = accent?.trim() ?? ''
    if (/^#[0-9a-fA-F]{6}$/.test(t)) return t
    if (/^#[0-9a-fA-F]{3}$/.test(t)) {
      const r = t[1]
      const g = t[2]
      const b = t[3]
      return `#${r}${r}${g}${g}${b}${b}`.toLowerCase()
    }
    return '#5a9ec4'
  }

  $: filteredIconCatalog = (() => {
    const q = iconSearchQuery.trim().toLowerCase()
    if (!q) return ICON_CATALOG
    return ICON_CATALOG.filter(
      (row) => row.id.toLowerCase().includes(q) || row.search.includes(q)
    )
  })()

  function toggleIconPicker() {
    iconPickerOpen = !iconPickerOpen
    if (iconPickerOpen) iconSearchQuery = ''
  }

  function closeIconPicker() {
    iconPickerOpen = false
  }

  function selectModeIcon(iconId: string) {
    if (editorMode) {
      editorMode.icon = iconId
      void saveEditorFields()
    }
    closeIconPicker()
  }

  const POLISH_FLAG_ROWS: { key: keyof PolishConfig; label: string }[] = [
    { key: 'fix_grammar', label: 'Grammar' },
    { key: 'remove_filler', label: 'Filler' },
    { key: 'fix_punctuation', label: 'Punctuation' },
    { key: 'smart_formatting', label: 'Formatting' },
    { key: 'self_correction', label: 'Self-correct' },
  ]

  let unlisteners: (() => void)[] = []
  let recipeLibraryOpen = false

  function inputChecked(ev: Event): boolean {
    const el = ev.currentTarget
    return el instanceof HTMLInputElement ? el.checked : false
  }

  function inputStringValue(ev: Event): string {
    const el = ev.currentTarget
    return el instanceof HTMLInputElement ? el.value : ''
  }

  function providerHasKey(providerId: string): boolean {
    if (!config || !providerId) return true
    return !!(config.provider_keys?.[providerId]?.trim())
  }

  async function loadModelCatalog() {
    if (!isTauriRuntime()) return
    try {
      modelCatalog = (await invoke('get_model_catalog')) as CatalogProvider[]
    } catch {
      modelCatalog = []
    }
  }

  function catalogDefaultSttModel(providerId: string): string {
    const p = modelCatalog.find((x) => x.id === providerId)
    const m =
      p?.models?.find((x) => x.capability === 'STT' && x.is_default) ??
      p?.models?.find((x) => x.capability === 'STT')
    return m?.id ?? ''
  }

  function catalogDefaultLlmModel(providerId: string): string {
    const p = modelCatalog.find((x) => x.id === providerId)
    const m =
      p?.models?.find((x) => x.capability === 'LLM' && x.is_default) ??
      p?.models?.find((x) => x.capability === 'LLM')
    return m?.id ?? ''
  }

  $: voiceProviderOptions = (() => {
    const out: { id: string; label: string }[] = []
    if (!config) return out
    for (const p of modelCatalog) {
      if (p.id === 'local') continue
      if (!p.capabilities.includes('STT')) continue
      if (!providerHasKey(p.id)) continue
      out.push({ id: p.id, label: p.name })
    }
    out.push({ id: 'local', label: 'Local' })
    return out
  })()

  $: llmProviderOptions = (() => {
    const out: { id: string; label: string }[] = [{ id: '', label: 'None' }]
    if (!config) return out
    for (const p of modelCatalog) {
      if (p.id === 'local') continue
      if (!p.capabilities.includes('LLM')) continue
      if (!providerHasKey(p.id)) continue
      out.push({ id: p.id, label: p.name })
    }
    const ce = config.custom_openai_endpoint
    if (ce?.base_url?.trim() && ce?.api_key?.trim()) {
      out.push({ id: 'custom_openai', label: 'Custom endpoint' })
    }
    return out
  })()

  /** New modes get concrete defaults from global STT/LLM settings. */
  function defaultModeModelRef(kind: 'voice' | 'llm'): ModeModelRef {
    if (!config) return { provider: '', model_id: '' }
    if (kind === 'voice') {
      return {
        provider: config.stt_config?.provider || 'groq',
        model_id: config.stt_config?.cloud_transcription_model || '',
      }
    }
    return {
      provider: config.default_llm_provider || '',
      model_id: config.default_llm_model || '',
    }
  }

  $: editorMode =
    config?.modes?.find((m) => m.id === editorModeId) ?? null

  $: availableSttModels = (() => {
    if (!editorMode?.voice_model.provider) return []
    const p = modelCatalog.find(x => x.id === editorMode.voice_model.provider)
    return p?.models?.filter(x => x.capability === 'STT') || []
  })()

  $: availableLlmModels = (() => {
    if (!editorMode?.language_model.provider) return []
    const p = modelCatalog.find(x => x.id === editorMode.language_model.provider)
    return p?.models?.filter(x => x.capability === 'LLM') || []
  })()

  function portal(node: HTMLElement) {
    const target = document.querySelector('.app-shell') || document.body
    target.appendChild(node)
    return {
      destroy() {
        if (node.parentNode) node.parentNode.removeChild(node)
      }
    }
  }

  function togglePanelSection(key: string) {
    panelSections = { ...panelSections, [key]: !panelSections[key] }
  }

  function openModePanel(modeId: string) {
    editorModeId = modeId
    autoRuleTestHint = {}
    appPickerRuleIndex = null
    iconPickerOpen = false
    iconSearchQuery = ''
    panelSections = { voice: false, ai: false, context: false, rules: false }
    isPanelOpen = true
  }

  function closePanel() {
    isPanelOpen = false
    iconPickerOpen = false
    iconSearchQuery = ''
    setTimeout(() => {
      autoRuleTestHint = {}
      appPickerRuleIndex = null
    }, 250)
  }

  async function refresh() {
    if (!isTauriRuntime()) return
    loadError = ''
    try {
      config = (await invoke('get_settings')) as AppConfig
      if (!editorModeId && config.active_mode_id) {
        editorModeId = config.active_mode_id
      } else if (!editorModeId && config.modes?.[0]) {
        editorModeId = config.modes[0].id
      }
    } catch (e) {
      loadError = e instanceof Error ? e.message : String(e)
    }
  }

  async function saveFull(next: AppConfig) {
    if (!isTauriRuntime()) return
    saving = true
    try {
      await invoke('save_settings', { newConfig: next })
      config = next
    } catch (e) {
      console.error(e)
      loadError = e instanceof Error ? e.message : String(e)
    } finally {
      saving = false
    }
  }

  async function setPolishEnabled(on: boolean) {
    if (!config) return
    await saveFull({ ...config, polish_enabled: on })
  }

  async function setPolishFlagRow(row: (typeof POLISH_FLAG_ROWS)[number], ev: Event) {
    if (!config) return
    const on = inputChecked(ev)
    const polish_config: PolishConfig = { ...config.polish_config, [row.key]: on }
    await saveFull({ ...config, polish_config })
  }

  async function setContextAwarenessEnabled(on: boolean) {
    if (!config) return
    await saveFull({ ...config, context_awareness_enabled: on })
  }

  async function activateMode(modeId: string) {
    if (!isTauriRuntime()) return
    try {
      await invoke('set_active_mode', { modeId })
      await refresh()
    } catch (e) {
      console.error(e)
      loadError = e instanceof Error ? e.message : String(e)
    }
  }

  async function saveEditorFields() {
    if (!config || !editorMode) return
    const m = { ...editorMode }
    m.updated_at = new Date().toISOString()
    try {
      await invoke('update_mode', { mode: m })
      await refresh()
    } catch (e) {
      console.error(e)
      loadError = e instanceof Error ? e.message : String(e)
    }
  }

  async function createNewMode() {
    if (!isTauriRuntime()) return
    const ts = new Date().toISOString()
    const draft: DictationMode = {
      id: '',
      name: 'New mode',
      icon: 'ph:pencil',
      accent_color: '',
      ai_instructions: '',
      voice_model: defaultModeModelRef('voice'),
      language_model: defaultModeModelRef('llm'),
      polish: true,
      context: {
        enabled: false,
        read_app: false,
        read_clipboard: false,
        read_selection: false,
        include_system_info: false,
      },
      auto_activate_rules: [],
      is_builtin: false,
      is_deletable: true,
      created_at: ts,
      updated_at: ts,
    }
    try {
      const created = (await invoke('create_mode', { mode: draft })) as DictationMode
      await refresh()
      openModePanel(created.id)
    } catch (e) {
      console.error(e)
      loadError = e instanceof Error ? e.message : String(e)
    }
  }

  async function deleteEditorMode() {
    if (!config || !editorMode || !editorMode.is_deletable) return
    if (!confirm(`Delete mode "${editorMode.name}"?`)) return
    try {
      await invoke('delete_mode', { modeId: editorMode.id })
      closePanel()
      editorModeId = 'default'
      await refresh()
    } catch (e) {
      console.error(e)
      loadError = e instanceof Error ? e.message : String(e)
    }
  }

  async function exportCurrentRecipe() {
    if (!editorMode || !isTauriRuntime()) return
    try {
      const json = (await invoke('export_recipe', { modeId: editorMode.id })) as string
      const blob = new Blob([json], { type: 'application/json' })
      const a = document.createElement('a')
      a.href = URL.createObjectURL(blob)
      a.download = `${editorMode.name.replace(/\s+/g, '-')}-kalam-recipe.json`
      a.click()
      URL.revokeObjectURL(a.href)
    } catch (e) {
      console.error(e)
      loadError = e instanceof Error ? e.message : String(e)
    }
  }

  function triggerImport() {
    fileInputEl?.click()
  }

  async function onRecipeFile(e: Event) {
    const input = e.target as HTMLInputElement
    const file = input.files?.[0]
    input.value = ''
    if (!file || !isTauriRuntime()) return
    try {
      const text = await file.text()
      const created = (await invoke('import_recipe', { json: text })) as DictationMode
      await refresh()
      openModePanel(created.id)
    } catch (err) {
      console.error(err)
      loadError = err instanceof Error ? err.message : String(err)
    }
  }

  function escapeRegexMetachars(s: string): string {
    return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  }

  function addAutoActivateRule() {
    if (!editorMode) return
    const next: AutoActivateRule = { app_pattern: '', pattern_type: 'ProcessName' }
    editorMode.auto_activate_rules = [...editorMode.auto_activate_rules, next]
    if (config) config = config
    void saveEditorFields()
    const newIdx = editorMode.auto_activate_rules.length - 1
    void openRunningAppPicker(newIdx)
  }

  function removeAutoActivateRule(index: number) {
    if (!editorMode) return
    editorMode.auto_activate_rules = editorMode.auto_activate_rules.filter((_, i) => i !== index)
    autoRuleTestHint = {}
    appPickerRuleIndex = null
    if (config) config = config
    void saveEditorFields()
  }

  async function openRunningAppPicker(ruleIndex: number) {
    if (!isTauriRuntime()) return
    appPickerRuleIndex = ruleIndex
    loadingRunningApps = true
    runningAppsForPicker = []
    try {
      runningAppsForPicker = (await invoke('get_running_apps')) as AppListEntry[]
    } catch (e) {
      console.error(e)
      loadError = e instanceof Error ? e.message : String(e)
      appPickerRuleIndex = null
    } finally {
      loadingRunningApps = false
    }
  }

  function applyPickedProcess(processName: string) {
    if (!editorMode || appPickerRuleIndex === null) return
    const i = appPickerRuleIndex
    const rules = [...editorMode.auto_activate_rules]
    if (!rules[i]) return
    rules[i] = {
      ...rules[i],
      app_pattern: escapeRegexMetachars(processName),
      pattern_type: 'ProcessName',
    }
    editorMode.auto_activate_rules = rules
    appPickerRuleIndex = null
    runningAppsForPicker = []
    if (config) config = config
    void saveEditorFields()
  }

  function closeAppPicker() {
    appPickerRuleIndex = null
    runningAppsForPicker = []
  }

  async function testAutoRuleAt(index: number) {
    if (!editorMode || !isTauriRuntime()) return
    const rule = editorMode.auto_activate_rules[index]
    if (!rule?.app_pattern?.trim()) {
      autoRuleTestHint = { ...autoRuleTestHint, [index]: 'Enter a pattern first.' }
      return
    }
    let re: RegExp
    try {
      re = new RegExp(rule.app_pattern)
    } catch {
      autoRuleTestHint = { ...autoRuleTestHint, [index]: 'Invalid regex.' }
      return
    }
    try {
      const apps = (await invoke('get_running_apps')) as AppListEntry[]
      const matches = apps.filter((a) => {
        const text = rule.pattern_type === 'WindowTitle' ? a.display_name : a.process_name
        return re.test(text)
      })
      const names = matches.slice(0, 8).map((a) => a.display_name || a.process_name)
      const extra = matches.length > 8 ? ` (+${matches.length - 8} more)` : ''
      autoRuleTestHint = {
        ...autoRuleTestHint,
        [index]: matches.length === 0
          ? 'No running apps match.'
          : `Matches: ${names.join(', ')}${extra}`,
      }
    } catch (e) {
      autoRuleTestHint = {
        ...autoRuleTestHint,
        [index]: e instanceof Error ? e.message : String(e),
      }
    }
  }

  $: cycleHotkeyLabel =
    config?.mode_cycle_hotkey && platform
      ? formatHotkeyForDisplay(config.mode_cycle_hotkey, platform)
      : 'Ctrl+Shift+M'
  $: holdHotkeyLabel =
    config?.hotkey && platform
      ? formatHotkeyForDisplay(config.hotkey, platform)
      : `Ctrl+${superKeyLabel(platform)}`

  function recipeLibraryBase(): string {
    const raw = config?.recipe_library_url?.trim() || 'https://kalam.stream'
    return raw.replace(/\/+$/, '')
  }

  $: recipeSubmitUrl = `${recipeLibraryBase()}/recipes/submit`

  /** Count of enabled context sources for a mode (for the summary chip). */
  function contextSourceCount(m: DictationMode): number {
    if (!m.context.enabled) return 0
    let n = 0
    if (m.context.read_app) n++
    if (m.context.read_clipboard) n++
    if (m.context.read_selection) n++
    if (m.context.include_system_info) n++
    return n
  }

  onMount(() => {
    void refresh()
    void loadModelCatalog()
    void invoke('get_platform')
      .then((p) => { platform = String(p) })
      .catch(() => {})
    void listenSafe<AppConfig>('settings_updated', () => {
      void refresh()
    }).then((u) => unlisteners.push(u))
    void listenSafe<{ mode_id?: string }>('recipe-imported', (ev) => {
      const id = ev.payload?.mode_id
      if (id) {
        void refresh().then(() => openModePanel(id))
      }
    }).then((u) => unlisteners.push(u))
    void listenSafe<{ message?: string }>('recipe-import-failed', (ev) => {
      loadError = ev.payload?.message ?? 'Recipe import failed'
    }).then((u) => unlisteners.push(u))
  })

  onDestroy(() => {
    for (const u of unlisteners) u()
  })
</script>

<RecipeLibrary
  open={recipeLibraryOpen}
  onClose={() => { recipeLibraryOpen = false }}
  onInstalled={(modeId) => {
    void refresh().then(() => openModePanel(modeId))
  }}
/>

<input
  bind:this={fileInputEl}
  type="file"
  accept=".json,application/json"
  class="sr-only"
  aria-hidden="true"
  on:change={onRecipeFile}
/>

<div class="page fade-in">
  <header class="page-header notes-header">
    <div>
      <h1 class="page-title">Dictation</h1>
      <p class="page-subtitle">
        <kbd class="inline-kbd">{holdHotkeyLabel}</kbd> to dictate
        <span class="subtitle-sep">&middot;</span>
        <kbd class="inline-kbd">{cycleHotkeyLabel}</kbd> to cycle modes
      </p>
    </div>
    <button type="button" class="btn-primary" on:click={createNewMode} disabled={!isTauriRuntime()}>
      <Icon icon="ph:plus" />
      New mode
    </button>
  </header>

  {#if loadError}
    <p class="dp-error" role="alert">{loadError}</p>
  {/if}

  {#if config}
    <!-- ═══ Unified mode list — selected row gets accent indicator ═══ -->
    {#if config.modes.length > 0}
      <div class="mode-list">
        {#each config.modes as m (m.id)}
          <div class="mode-row" class:selected={m.id === config.active_mode_id}>
            {#if m.id === config.active_mode_id}
              <div class="mode-selected-bar"></div>
            {/if}
            <button
              type="button"
              class="mode-activate"
              on:click={() => void activateMode(m.id)}
              title={m.id === config.active_mode_id ? 'Currently selected' : 'Click to select'}
            >
              <span
                class="mode-list-accent"
                aria-hidden="true"
                style="background: {effectiveModeAccent(m)}"
                title="Mode accent"
              />
              <span class="mode-icon">
                <Icon icon={m.icon || 'ph:circle-dashed'} />
              </span>
              <span class="mode-name">{m.name}</span>
              {#if m.polish}
                <span class="mode-chip">Polish</span>
              {/if}
              {#if contextSourceCount(m) > 0}
                <span class="mode-chip">Context</span>
              {/if}
            </button>
            <button
              type="button"
              class="mode-edit"
              on:click|stopPropagation={() => openModePanel(m.id)}
              aria-label="Edit {m.name}"
            >
              <Icon icon="ph:gear" />
            </button>
          </div>
        {/each}
      </div>
    {/if}

    <!-- ═══ Global processing bar ═══ -->
    <div class="processing-bar">
      <div class="proc-group">
        <label class="proc-toggle">
          <input
            type="checkbox"
            class="toggle-switch"
            checked={config.polish_enabled ?? false}
            on:change={(e) => setPolishEnabled(inputChecked(e))}
            disabled={saving || !isTauriRuntime()}
          />
          <span class="proc-label">Polish</span>
        </label>
        {#if config.polish_enabled && config.polish_config}
          <div class="polish-chips" transition:fade={{ duration: 120 }}>
            {#each POLISH_FLAG_ROWS as row}
              <label class="polish-chip" class:on={config.polish_config[row.key]}>
                <input
                  type="checkbox"
                  checked={config.polish_config[row.key]}
                  on:change={(e) => void setPolishFlagRow(row, e)}
                  disabled={saving || !isTauriRuntime()}
                />
                {row.label}
              </label>
            {/each}
          </div>
        {/if}
      </div>
      <label class="proc-toggle">
        <input
          type="checkbox"
          class="toggle-switch"
          checked={config.context_awareness_enabled ?? false}
          on:change={(e) => setContextAwarenessEnabled(inputChecked(e))}
          disabled={saving || !isTauriRuntime()}
        />
        <span class="proc-label">Context</span>
      </label>
    </div>

    <!-- ═══ Secondary actions ═══ -->
    <div class="secondary-actions">
      <button type="button" class="btn-link" on:click={triggerImport} disabled={!isTauriRuntime()}>
        <Icon icon="ph:upload-simple" />
        Import recipe
      </button>
      <button type="button" class="btn-link" on:click={() => { recipeLibraryOpen = true }} disabled={!isTauriRuntime()}>
        <Icon icon="ph:storefront" />
        Browse library
      </button>
    </div>
  {:else}
    <div class="state-container empty-state">
      <Icon icon="ph:spinner-gap" class="spin-icon" />
      <p>Loading…</p>
    </div>
  {/if}
</div>

<!-- Overlay -->
{#if isPanelOpen}
  <div
    class="dp-overlay"
    role="button"
    tabindex="0"
    aria-label="Close panel"
    on:click={closePanel}
    on:keydown={(e) => (e.key === 'Enter' || e.key === 'Escape') && closePanel()}
    transition:fade
    use:portal
  ></div>
{/if}

<!-- Side Panel -->
{#if isPanelOpen && editorMode}
  <aside class="dp-panel" transition:fly={{ x: 460, duration: 250, opacity: 1 }} use:portal>
    <!-- Header -->
    <div class="dp-head">
      <div class="dp-head-left">
        <span
          class="dp-head-accent"
          aria-hidden="true"
          style="background: {effectiveModeAccent(editorMode)}"
        />
        {#if editorMode.icon}
          <span class="dp-head-icon"><Icon icon={editorMode.icon} /></span>
        {/if}
        <h3>{editorMode.name}</h3>
      </div>
      <div class="dp-head-right">
        {#if editorMode.id !== config?.active_mode_id}
          <button type="button" class="dp-activate-btn" on:click={() => void activateMode(editorMode.id)}>
            Activate
          </button>
        {:else}
          <span class="dp-active-badge">Selected</span>
        {/if}
        <button type="button" class="dp-close" on:click={closePanel} aria-label="Close">
          <Icon icon="ph:x" />
        </button>
      </div>
    </div>

    <!-- Body -->
    <div class="dp-body">
      <!-- Name & Icon -->
      <div class="dp-sec">
        <div class="dp-inline-fields">
          <label class="dp-f">
            <span>Name</span>
            <input type="text" class="dp-in" bind:value={editorMode.name} on:blur={saveEditorFields} />
          </label>
          <div class="dp-f dp-f-icon">
            <span>Icon</span>
            <div class="icon-picker-anchor">
              <button
                type="button"
                class="icon-picker-trigger"
                on:click={toggleIconPicker}
                aria-expanded={iconPickerOpen}
                aria-haspopup="dialog"
              >
                <span class="icon-picker-trigger-preview">
                  <Icon icon={editorMode.icon || 'ph:circle-dashed'} />
                </span>
                <span class="icon-picker-trigger-label">Choose icon</span>
                <span class="icon-picker-trigger-caret"><Icon icon="ph:caret-down" /></span>
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Accent color (status bar + overlay dormant dot) -->
      <div class="dp-sec">
        <span class="dp-sec-label">Accent color</span>
        <p class="dp-hint">Shown next to the mode in the list, status bar, and overlay. Any valid CSS color.</p>
        <div class="accent-presets" role="group" aria-label="Accent color">
          {#each MODE_ACCENT_PRESETS as p (p.value)}
            <button
              type="button"
              class="accent-swatch"
              title={p.label}
              aria-label={p.label}
              style="background: {p.value}"
              on:click={() => {
                editorMode.accent_color = p.value
                void saveEditorFields()
              }}
            />
          {/each}
          <label
            class="accent-swatch accent-swatch-custom"
            title="Pick any color"
            aria-label="Pick any color"
            style="background: {effectiveModeAccent(editorMode)}"
          >
            <input
              type="color"
              class="accent-color-native"
              value={accentHexForPicker(editorMode.accent_color)}
              on:input={(e) => {
                if (editorMode) {
                  editorMode.accent_color = e.currentTarget.value
                  void saveEditorFields()
                }
              }}
            />
          </label>
        </div>
      </div>

      <!-- AI Instructions -->
      <div class="dp-sec">
        <span class="dp-sec-label">Instructions</span>
        <textarea
          class="dp-ta"
          rows="4"
          bind:value={editorMode.ai_instructions}
          placeholder="How should the model transform the transcript?"
          on:blur={saveEditorFields}
        ></textarea>
      </div>

      <!-- Polish toggle -->
      <div class="dp-sec dp-sec-row">
        <div class="dp-toggle-info">
          <span class="dp-toggle-label">Polish</span>
          <span class="dp-toggle-desc">Automatically removes 'ums', 'ahs', and fixes basic grammar before applying your instructions.</span>
        </div>
        <input type="checkbox" class="toggle-switch" bind:checked={editorMode.polish} on:change={saveEditorFields} />
      </div>

      <!-- Voice Recognition (collapsible) -->
      <button type="button" class="dp-disclosure" on:click={() => togglePanelSection('voice')}>
        <span>Voice Recognition (Listening)</span>
        <span class="dp-disclosure-summary">
          {editorMode.voice_model.provider || 'Default'}
        </span>
        <span class="dp-disclosure-caret" class:open={panelSections.voice}>
          <Icon icon="ph:caret-down" />
        </span>
      </button>
      {#if panelSections.voice}
        <div class="dp-disclosure-body" transition:slide={{ duration: 150 }}>
          <div class="dp-grid2">
            <label class="dp-f">
              <span>Provider</span>
              <select class="dp-sel" bind:value={editorMode.voice_model.provider} on:change={saveEditorFields}>
                {#each voiceProviderOptions as vp}
                  <option value={vp.id}>{vp.label}</option>
                {/each}
                {#if editorMode.voice_model.provider && editorMode.voice_model.provider !== 'local' && !voiceProviderOptions.some((x) => x.id === editorMode.voice_model.provider)}
                  <option value={editorMode.voice_model.provider}>{editorMode.voice_model.provider} — add key</option>
                {/if}
              </select>
            </label>
            <label class="dp-f">
              <span>Model</span>
              {#if availableSttModels.length > 0}
                <select class="dp-sel" bind:value={editorMode.voice_model.model_id} on:change={saveEditorFields}>
                  <option value="">Default ({catalogDefaultSttModel(editorMode.voice_model.provider)})</option>
                  {#each availableSttModels as m}
                    <option value={m.id}>{m.name || m.id}</option>
                  {/each}
                </select>
              {:else}
                <input
                  type="text"
                  class="dp-in"
                  bind:value={editorMode.voice_model.model_id}
                  placeholder={catalogDefaultSttModel(editorMode.voice_model.provider) || 'Default'}
                  on:blur={saveEditorFields}
                />
              {/if}
            </label>
          </div>
        </div>
      {/if}

      <!-- AI Assistant (collapsible) -->
      <button type="button" class="dp-disclosure" on:click={() => togglePanelSection('ai')}>
        <span>AI Assistant (Thinking)</span>
        <span class="dp-disclosure-summary">
          {editorMode.language_model.provider || 'None'}
        </span>
        <span class="dp-disclosure-caret" class:open={panelSections.ai}>
          <Icon icon="ph:caret-down" />
        </span>
      </button>
      {#if panelSections.ai}
        <div class="dp-disclosure-body" transition:slide={{ duration: 150 }}>
          <div class="dp-grid2">
            <label class="dp-f">
              <span>Provider</span>
              <select class="dp-sel" bind:value={editorMode.language_model.provider} on:change={saveEditorFields}>
                {#each llmProviderOptions as lp}
                  <option value={lp.id}>{lp.label}</option>
                {/each}
                {#if editorMode.language_model.provider && !llmProviderOptions.some((x) => x.id === editorMode.language_model.provider)}
                  <option value={editorMode.language_model.provider}>{editorMode.language_model.provider} — add key</option>
                {/if}
              </select>
            </label>
            <label class="dp-f">
              <span>Model</span>
              {#if availableLlmModels.length > 0}
                <select class="dp-sel" bind:value={editorMode.language_model.model_id} on:change={saveEditorFields}>
                  <option value="">Default ({catalogDefaultLlmModel(editorMode.language_model.provider)})</option>
                  {#each availableLlmModels as m}
                    <option value={m.id}>{m.name || m.id}</option>
                  {/each}
                </select>
              {:else}
                <input
                  type="text"
                  class="dp-in"
                  bind:value={editorMode.language_model.model_id}
                  placeholder={catalogDefaultLlmModel(editorMode.language_model.provider) || 'Default'}
                  on:blur={saveEditorFields}
                />
              {/if}
            </label>
          </div>
        </div>
      {/if}

      <!-- Context (collapsible) -->
      <button type="button" class="dp-disclosure" on:click={() => togglePanelSection('context')}>
        <span>Context</span>
        <span class="dp-disclosure-summary">
          {#if !editorMode.context.enabled}Off{:else}{contextSourceCount(editorMode)} source{contextSourceCount(editorMode) !== 1 ? 's' : ''}{/if}
        </span>
        <span class="dp-disclosure-caret" class:open={panelSections.context}>
          <Icon icon="ph:caret-down" />
        </span>
      </button>
      {#if panelSections.context}
        <div class="dp-disclosure-body" transition:slide={{ duration: 150 }}>
          <label class="dp-toggle-row dp-nested-toggle">
            <div class="dp-toggle-info">
              <span class="dp-toggle-label">Enable context</span>
              <span class="dp-toggle-desc">Allow the AI to read information from your computer to improve accuracy.</span>
            </div>
            <input type="checkbox" class="toggle-switch" bind:checked={editorMode.context.enabled} on:change={saveEditorFields} />
          </label>
          <div class="dp-context-sources" class:disabled={!editorMode.context.enabled}>
            <label class="dp-toggle-row dp-nested-toggle">
              <div class="dp-toggle-info">
                <span class="dp-toggle-label">Active app content</span>
                <span class="dp-toggle-desc">Reads the window you are currently typing in (e.g., the email you are replying to).</span>
              </div>
              <input type="checkbox" class="toggle-switch" bind:checked={editorMode.context.read_app} disabled={!editorMode.context.enabled} on:change={saveEditorFields} />
            </label>
            <label class="dp-toggle-row dp-nested-toggle">
              <div class="dp-toggle-info">
                <span class="dp-toggle-label">Clipboard</span>
                <span class="dp-toggle-desc">Reads the last thing you copied.</span>
              </div>
              <input type="checkbox" class="toggle-switch" bind:checked={editorMode.context.read_clipboard} disabled={!editorMode.context.enabled} on:change={saveEditorFields} />
            </label>
            <label class="dp-toggle-row dp-nested-toggle">
              <div class="dp-toggle-info">
                <span class="dp-toggle-label">Selected text</span>
                <span class="dp-toggle-desc">Reads the text you currently have highlighted.</span>
              </div>
              <input type="checkbox" class="toggle-switch" bind:checked={editorMode.context.read_selection} disabled={!editorMode.context.enabled} on:change={saveEditorFields} />
            </label>
            <label class="dp-toggle-row dp-nested-toggle">
              <div class="dp-toggle-info">
                <span class="dp-toggle-label">System info</span>
                <span class="dp-toggle-desc">Includes basic details like your operating system and the current time.</span>
              </div>
              <input type="checkbox" class="toggle-switch" bind:checked={editorMode.context.include_system_info} disabled={!editorMode.context.enabled} on:change={saveEditorFields} />
            </label>
          </div>
        </div>
      {/if}

      <!-- Auto-activate rules (collapsible) -->
      <button type="button" class="dp-disclosure" on:click={() => togglePanelSection('rules')}>
        <span>Auto-activate</span>
        <span class="dp-disclosure-summary">
          {editorMode.auto_activate_rules.length === 0
            ? 'None'
            : `${editorMode.auto_activate_rules.length} rule${editorMode.auto_activate_rules.length > 1 ? 's' : ''}`}
        </span>
        <span class="dp-disclosure-caret" class:open={panelSections.rules}>
          <Icon icon="ph:caret-down" />
        </span>
      </button>
      {#if panelSections.rules}
        <div class="dp-disclosure-body" transition:slide={{ duration: 150 }}>
          {#each editorMode.auto_activate_rules as rule, ruleIdx (ruleIdx)}
            <div class="dp-rule">
              <div class="dp-rule-top">
                <div class="dp-rule-summary">
                  <span class="rule-app-name">{rule.app_pattern.replace(/^\(\?i\)/, '') || 'New rule'}</span>
                  <span class="rule-app-type">{rule.pattern_type}</span>
                </div>
                <button type="button" class="dp-close sm" on:click={() => removeAutoActivateRule(ruleIdx)} aria-label="Remove">
                  <Icon icon="ph:x" />
                </button>
              </div>
              <div class="dp-rule-advanced">
                <select class="dp-sel sm" bind:value={rule.pattern_type} on:change={saveEditorFields}>
                  <option value="ProcessName">Process</option>
                  <option value="WindowTitle">Window</option>
                  <option value="BundleId">Bundle</option>
                </select>
                <input
                  type="text"
                  class="dp-in sm"
                  bind:value={rule.app_pattern}
                  placeholder="(?i)outlook|notepad"
                  on:blur={saveEditorFields}
                />
              </div>
              <div class="dp-rule-btns">
                <button type="button" class="btn-link sm" on:click={() => void openRunningAppPicker(ruleIdx)}>Pick running app</button>
                <button type="button" class="btn-link sm" on:click={() => void testAutoRuleAt(ruleIdx)}>Test</button>
              </div>
              {#if autoRuleTestHint[ruleIdx]}
                <p class="dp-rule-result">{autoRuleTestHint[ruleIdx]}</p>
              {/if}
              {#if appPickerRuleIndex === ruleIdx}
                <div class="dp-picker">
                  {#if loadingRunningApps}
                    <span class="dp-muted">Loading…</span>
                  {:else if runningAppsForPicker.length === 0}
                    <span class="dp-muted">No apps.</span>
                  {:else}
                    <select class="dp-sel" on:change={(e) => { const v = inputStringValue(e); if (v) applyPickedProcess(v) }}>
                      <option value="">Choose…</option>
                      {#each runningAppsForPicker as a (a.process_name + (a.exe_path ?? ''))}
                        <option value={a.process_name}>{a.display_name} ({a.process_name})</option>
                      {/each}
                    </select>
                  {/if}
                  <button type="button" class="btn-link sm" on:click={closeAppPicker}>Close</button>
                </div>
              {/if}
            </div>
          {/each}
          <div class="dp-rules-actions">
            <button type="button" class="btn-link" on:click={addAutoActivateRule} disabled={!isTauriRuntime()}>
              <Icon icon="ph:plus" /> Add rule
            </button>
          </div>
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div class="dp-foot">
      <div>
        {#if editorMode.is_deletable}
          <button type="button" class="btn-link danger" on:click={deleteEditorMode}>
            <Icon icon="ph:trash" /> Delete
          </button>
        {/if}
      </div>
      <div class="dp-foot-right">
        <button type="button" class="btn-link" on:click={exportCurrentRecipe}>
          <Icon icon="ph:export" /> Export
        </button>
        <a class="btn-link" href={recipeSubmitUrl} target="_blank" rel="noopener noreferrer">
          <Icon icon="ph:share-network" /> Share
        </a>
      </div>
    </div>
  </aside>

  {#if iconPickerOpen}
    <div class="icon-picker-root" transition:fade={{ duration: 120 }} use:portal>
      <button
        type="button"
        class="icon-picker-scrim"
        aria-label="Close icon picker"
        on:click={closeIconPicker}
      />
      <div
        class="icon-picker-sheet"
        role="dialog"
        aria-modal="true"
        aria-label="Choose an icon"
      >
        <div class="icon-picker-head">
          <h4 class="icon-picker-title">Choose icon</h4>
          <button type="button" class="icon-picker-close" on:click={closeIconPicker} aria-label="Close">
            <Icon icon="ph:x" />
          </button>
        </div>
        <input
          type="search"
          class="icon-picker-search"
          placeholder="Search by name…"
          bind:value={iconSearchQuery}
        />
        <div class="icon-picker-grid">
          {#each filteredIconCatalog as row (row.id)}
            <button
              type="button"
              class="icon-picker-cell"
              class:selected={editorMode.icon === row.id}
              title={row.id}
              aria-label={row.search}
              on:click={() => selectModeIcon(row.id)}
            >
              <Icon icon={row.id} />
            </button>
          {/each}
          {#if filteredIconCatalog.length === 0}
            <p class="icon-picker-empty">No icons match “{iconSearchQuery}”.</p>
          {/if}
        </div>
      </div>
    </div>
  {/if}
{/if}

<svelte:window
  on:keydown={(e) => {
    if (iconPickerOpen && e.key === 'Escape') {
      e.preventDefault()
      closeIconPicker()
    }
  }}
/>

<style>
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    border: 0;
  }

  /* ── Subtitle ── */
  .inline-kbd {
    display: inline-block;
    padding: 1px 6px;
    border-radius: 5px;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    font-family: inherit;
    font-size: 0.85em;
    font-weight: 500;
    color: var(--text-secondary);
    vertical-align: baseline;
  }

  .subtitle-sep {
    margin: 0 4px;
    color: var(--text-muted);
  }

  /* ══════════════════════════════════════════════════
     Mode List — unified card rows with selected indicator
     ══════════════════════════════════════════════════ */
  .mode-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: var(--space-lg);
  }

  .mode-row {
    position: relative;
    display: flex;
    align-items: center;
    padding: 16px;
    background: var(--bg-elevated);
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    overflow: hidden;
    transition: background 0.15s cubic-bezier(0.16, 1, 0.3, 1), border-color 0.15s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .mode-row:hover {
    background: var(--bg-hover);
    border-color: var(--border-light);
  }

  .mode-row.selected {
    border-color: var(--border-light);
  }

  /* Accent bar on the left edge of the selected row */
  .mode-selected-bar {
    position: absolute;
    top: 0;
    left: 0;
    width: 3px;
    height: 100%;
    background: var(--accent);
    border-radius: 3px 0 0 3px;
  }

  .mode-activate {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
    min-width: 0;
    padding: 0;
    border: none;
    background: transparent;
    cursor: pointer;
    color: inherit;
    font: inherit;
    text-align: left;
    border-radius: var(--radius-sm);
  }

  .mode-activate:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .mode-list-accent {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
    box-shadow: 0 0 0 1px color-mix(in oklch, var(--border) 55%, transparent);
  }

  .mode-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background: var(--primary-alpha, rgba(0, 0, 0, 0.06));
    flex-shrink: 0;
  }

  .mode-icon :global(svg) {
    width: 16px;
    height: 16px;
    color: var(--text);
  }

  @media (prefers-reduced-motion: reduce) {
    .mode-row { transition: none; }
    .mode-edit { transition: none; }
    .toggle-switch,
    .toggle-switch::after { transition: none; }
    .polish-chip { transition: none; }
  }

  .mode-name {
    font-size: 15px;
    font-weight: 500;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mode-chip {
    display: inline-flex;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: transparent;
    border: 1px solid var(--border);
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .mode-edit {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s cubic-bezier(0.16, 1, 0.3, 1), color 0.15s cubic-bezier(0.16, 1, 0.3, 1), background 0.15s cubic-bezier(0.16, 1, 0.3, 1);
    flex-shrink: 0;
    margin-left: auto;
  }

  .mode-row:hover .mode-edit,
  .mode-edit:focus-visible {
    opacity: 1;
  }

  .mode-edit:hover {
    color: var(--text);
    background: var(--bg-hover);
  }

  .mode-edit:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .mode-edit :global(svg) {
    width: 16px;
    height: 16px;
  }

  /* ══════════════════════════════════════════════════
     Processing Bar — global Polish + Context toggles
     ══════════════════════════════════════════════════ */
  .processing-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px 20px;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-light);
    margin-bottom: var(--space-lg);
  }

  .proc-group {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px 12px;
    flex: 1;
    min-width: 0;
  }

  .proc-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    flex-shrink: 0;
  }

  .proc-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary, var(--text));
  }

  /* ── Toggle switch ── */
  .toggle-switch {
    appearance: none;
    -webkit-appearance: none;
    width: 34px;
    height: 20px;
    border-radius: 999px;
    background: var(--border);
    position: relative;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    border: none;
  }

  .toggle-switch::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--bg);
    transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
  }

  .toggle-switch:checked {
    background: var(--accent);
  }

  .toggle-switch:checked::after {
    transform: translateX(14px);
    background: var(--accent-fg);
  }

  .toggle-switch:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .toggle-switch:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .polish-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .polish-chip {
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    border: 1px solid var(--border-light);
    background: transparent;
    font-size: 10px;
    font-weight: 500;
    color: var(--text-muted);
    cursor: pointer;
    transition: background 0.12s cubic-bezier(0.16, 1, 0.3, 1), border-color 0.12s cubic-bezier(0.16, 1, 0.3, 1), color 0.12s cubic-bezier(0.16, 1, 0.3, 1);
    user-select: none;
  }

  .polish-chip input { display: none; }

  .polish-chip.on {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    border-color: color-mix(in srgb, var(--accent) 25%, var(--border));
    color: var(--text);
  }

  .polish-chip:hover {
    background: var(--bg-hover);
  }

  .polish-chip:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  /* ── Secondary actions ── */
  .secondary-actions {
    display: flex;
    gap: 16px;
    padding-top: var(--space-sm);
  }

  /* ── Buttons ── */
  .btn-primary {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    border: none;
    border-radius: var(--radius-full, 9999px);
    background: var(--accent);
    color: var(--accent-fg);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.12s ease;
  }

  .btn-primary:hover:not(:disabled) { opacity: 0.85; }
  .btn-primary:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  .btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }

  .btn-link {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    text-decoration: none;
    transition: color 0.12s ease;
  }

  .btn-link:hover { color: var(--text); }
  .btn-link:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  .btn-link:disabled { opacity: 0.4; cursor: not-allowed; }
  .btn-link.danger { color: var(--danger, #ef4444); }
  .btn-link.danger:hover { color: #dc2626; }
  .btn-link.sm { font-size: 12px; }

  .dp-error {
    color: var(--danger, #ef4444);
    font-size: 13px;
    margin: 0 0 12px;
    padding: 8px 12px;
    background: color-mix(in srgb, var(--danger, #ef4444) 8%, transparent);
    border-radius: var(--radius-sm);
  }

  /* ═══════════════════════════════════════════════
     Side Panel (portaled → :global)
     ═══════════════════════════════════════════════ */
  :global(.dp-overlay) {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    backdrop-filter: blur(2px);
    z-index: 9998;
  }

  :global(aside.dp-panel) {
    position: fixed;
    top: 0;
    right: 0;
    width: 100%;
    max-width: 440px;
    height: 100vh;
    background: var(--bg-elevated);
    border-left: 1px solid var(--border);
    color: var(--text);
    z-index: 9999;
    display: flex;
    flex-direction: column;
    box-shadow: -4px 0 24px rgba(0, 0, 0, 0.12);
  }

  /* Header */
  :global(.dp-head) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 20px;
    border-bottom: 1px solid var(--border-light);
    gap: 12px;
    flex-shrink: 0;
  }

  :global(.dp-head-left) {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  :global(.dp-head-accent) {
    width: 11px;
    height: 11px;
    border-radius: 50%;
    flex-shrink: 0;
    box-shadow: 0 0 0 1px color-mix(in oklch, var(--border) 55%, transparent);
  }

  :global(.dp-head-icon) {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: var(--radius-sm, 8px);
    background: var(--primary-alpha, rgba(0, 0, 0, 0.06));
    flex-shrink: 0;
  }

  :global(.dp-head-icon svg) {
    width: 16px;
    height: 16px;
  }

  :global(.dp-head h3) {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  :global(.dp-head-right) {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  :global(.dp-activate-btn) {
    padding: 4px 12px;
    border-radius: var(--radius-full, 9999px);
    border: 1px solid var(--border, rgba(0, 0, 0, 0.08));
    background: transparent;
    color: var(--text-secondary, #86868b);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.12s ease;
  }

  :global(.dp-activate-btn:hover) {
    background: var(--bg-hover, rgba(0, 0, 0, 0.04));
    color: var(--text, #1d1d1f);
  }

  :global(.dp-activate-btn:focus-visible) {
    outline: 2px solid var(--accent, #000);
    outline-offset: 2px;
  }

  :global(.dp-active-badge) {
    padding: 4px 10px;
    border-radius: var(--radius-full, 9999px);
    background: var(--accent, #000);
    color: var(--accent-fg, #fff);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  :global(.dp-close) {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: var(--radius-sm, 8px);
    background: transparent;
    color: var(--text-muted, #6e6e73);
    cursor: pointer;
    transition: background 0.12s ease;
  }

  :global(.dp-close:hover) {
    background: var(--bg-hover, rgba(0, 0, 0, 0.04));
    color: var(--text, #1d1d1f);
  }

  :global(.dp-close:focus-visible) {
    outline: 2px solid var(--accent, #000);
    outline-offset: 2px;
  }

  :global(.dp-close.sm) {
    width: 24px;
    height: 24px;
  }

  :global(.dp-close.sm svg) {
    width: 14px;
    height: 14px;
  }

  /* Body */
  :global(.dp-body) {
    flex: 1;
    overflow-y: auto;
    padding: 0;
  }

  :global(.dp-sec) {
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-light, rgba(0, 0, 0, 0.04));
  }

  :global(.dp-sec:last-child) {
    border-bottom: none;
  }

  :global(.dp-sec-row) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text, #1d1d1f);
  }

  /** Space between accordion header row and fields inside — avoids labels hugging the divider. */
  :global(.dp-disclosure-body) {
    padding: 14px 20px 18px;
    border-bottom: 1px solid var(--border-light, rgba(0, 0, 0, 0.04));
  }

  :global(.dp-sec-label) {
    display: block;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted, #6e6e73);
    margin-bottom: 10px;
  }

  :global(.dp-hint) {
    margin: 0 0 12px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-muted, #6e6e73);
  }

  :global(.accent-presets) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-bottom: 0;
  }

  :global(.accent-swatch) {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 1px solid var(--border, rgba(0, 0, 0, 0.12));
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
    transition: transform 0.12s ease, box-shadow 0.12s ease;
  }

  :global(.accent-swatch:hover) {
    transform: scale(1.08);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
  }

  :global(.accent-swatch:focus-visible) {
    outline: 2px solid var(--accent, #000);
    outline-offset: 2px;
  }

  /* Last swatch opens native color picker; dashed ring reads as “custom” vs preset solids. */
  :global(.accent-swatch-custom) {
    position: relative;
    overflow: hidden;
    box-shadow: inset 0 0 0 2px color-mix(in srgb, var(--bg-elevated, #fff) 65%, transparent);
    border: 2px dashed color-mix(in srgb, var(--border) 70%, transparent);
  }

  :global(.accent-swatch-custom:hover) {
    transform: scale(1.08);
    box-shadow: inset 0 0 0 2px color-mix(in srgb, var(--bg-elevated, #fff) 65%, transparent),
      0 2px 8px rgba(0, 0, 0, 0.12);
  }

  :global(.accent-color-native) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    padding: 0;
    border: none;
    opacity: 0;
    cursor: pointer;
  }

  /* Disclosure buttons */
  :global(.dp-disclosure) {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 12px 20px;
    border: none;
    border-bottom: 1px solid var(--border-light, rgba(0, 0, 0, 0.04));
    background: transparent;
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    color: var(--text, #1d1d1f);
    text-align: left;
    transition: background 0.12s ease;
  }

  :global(.dp-disclosure:hover) {
    background: var(--bg-hover, rgba(0, 0, 0, 0.04));
  }

  :global(.dp-disclosure-summary) {
    margin-left: auto;
    font-size: 12px;
    font-weight: 400;
    color: var(--text-muted, #6e6e73);
  }

  :global(.dp-disclosure-caret) {
    display: flex;
    color: var(--text-muted, #6e6e73);
    transition: transform 0.15s ease;
  }

  :global(.dp-disclosure-caret.open) {
    transform: rotate(180deg);
  }

  :global(.dp-disclosure-caret svg) {
    width: 14px;
    height: 14px;
  }

  /* Fields */
  :global(.dp-inline-fields) {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  :global(.dp-f) {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  :global(.dp-f > span) {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-muted, #6e6e73);
  }

  :global(.dp-grid2) {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    margin-bottom: 10px;
  }

  :global(.dp-grid2:last-child) {
    margin-bottom: 0;
  }

  :global(.dp-in),
  :global(.dp-sel),
  :global(.dp-ta) {
    width: 100%;
    box-sizing: border-box;
    padding: 8px 10px;
    border-radius: var(--radius-sm, 8px);
    border: 1px solid var(--border, rgba(0, 0, 0, 0.08));
    background: var(--bg, #fff);
    color: var(--text, #1d1d1f);
    font-family: inherit;
    font-size: 13px;
    transition: border-color 0.12s ease;
  }

  :global(.dp-in:focus),
  :global(.dp-sel:focus),
  :global(.dp-ta:focus) {
    outline: none;
    border-color: var(--accent, #000);
  }

  :global(.dp-in.sm),
  :global(.dp-sel.sm) {
    padding: 5px 8px;
    font-size: 12px;
  }

  :global(.dp-ta) {
    min-height: 72px;
    resize: vertical;
    line-height: 1.5;
  }

  :global(.dp-toggle-row) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 0;
    font-size: 13px;
    color: var(--text, #1d1d1f);
  }

  :global(.dp-toggle-row.dp-nested-toggle) {
    padding: 6px 0;
  }

  :global(.dp-toggle-info) {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    padding-right: 16px;
  }

  :global(.dp-toggle-label) {
    font-weight: 500;
    color: var(--text);
  }

  :global(.dp-toggle-desc) {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.3;
  }

  /* Lighter than accordion titles — avoids competing with “Voice recognition”, “Context”, etc. */
  :global(.dp-nested-toggle .dp-toggle-label) {
    font-size: 12px;
    font-weight: 400;
    color: var(--text-secondary, var(--text-muted));
  }

  :global(.dp-nested-toggle .dp-toggle-desc) {
    font-size: 10px;
    line-height: 1.35;
  }

  :global(.dp-context-sources) {
    margin-top: 10px;
    padding-top: 12px;
    border-top: 1px dashed var(--border-light);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  :global(.dp-context-sources.disabled) {
    opacity: 0.5;
    pointer-events: none;
  }

  :global(.dp-rules-actions) {
    margin-top: 14px;
    padding-top: 14px;
    border-top: 1px dashed var(--border-light);
  }

  :global(.icon-picker-anchor) {
    position: relative;
  }

  :global(.icon-picker-trigger) {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 12px;
    border-radius: var(--radius-sm, 8px);
    border: 1px solid var(--border, rgba(0, 0, 0, 0.08));
    background: var(--bg, #fff);
    color: var(--text-secondary);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    text-align: left;
    transition: border-color 0.12s ease, background 0.12s ease;
  }

  :global(.icon-picker-trigger:hover) {
    background: var(--bg-hover);
    border-color: var(--border-light);
    color: var(--text);
  }

  :global(.icon-picker-trigger:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  :global(.icon-picker-trigger-preview) {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background: var(--primary-alpha, rgba(0, 0, 0, 0.06));
    color: var(--text);
    flex-shrink: 0;
  }

  :global(.icon-picker-trigger-preview) :global(svg) {
    width: 18px;
    height: 18px;
  }

  :global(.icon-picker-trigger-label) {
    flex: 1;
    font-weight: 500;
    color: var(--text);
  }

  :global(.icon-picker-trigger-caret) {
    display: flex;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  :global(.icon-picker-trigger-caret) :global(svg) {
    width: 14px;
    height: 14px;
  }

  .icon-picker-root {
    position: fixed;
    inset: 0;
    z-index: 10050;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    pointer-events: none;
  }

  .icon-picker-scrim {
    position: absolute;
    inset: 0;
    margin: 0;
    padding: 0;
    border: none;
    background: rgba(0, 0, 0, 0.4);
    cursor: pointer;
    pointer-events: auto;
  }

  .icon-picker-sheet {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 340px;
    max-height: min(72vh, 520px);
    display: flex;
    flex-direction: column;
    border-radius: 14px;
    border: 1px solid var(--border, rgba(0, 0, 0, 0.1));
    background: var(--bg-elevated, #fff);
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.18);
    pointer-events: auto;
    overflow: hidden;
  }

  .icon-picker-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 14px 16px 10px;
    border-bottom: 1px solid var(--border-light, rgba(0, 0, 0, 0.06));
    flex-shrink: 0;
  }

  .icon-picker-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.02em;
    color: var(--text);
  }

  .icon-picker-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .icon-picker-close:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .icon-picker-search {
    margin: 12px 16px 8px;
    padding: 9px 12px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 14px;
    flex-shrink: 0;
  }

  .icon-picker-search:focus {
    outline: none;
    border-color: var(--accent);
  }

  .icon-picker-grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 6px;
    padding: 8px 16px 16px;
    overflow-y: auto;
    flex: 1;
    min-height: 120px;
  }

  .icon-picker-cell {
    display: flex;
    align-items: center;
    justify-content: center;
    aspect-ratio: 1;
    border-radius: 10px;
    border: 1px solid var(--border-light);
    background: var(--bg-hover);
    color: var(--text-secondary);
    cursor: pointer;
    transition: background 0.12s ease, border-color 0.12s ease, color 0.12s ease;
  }

  .icon-picker-cell:hover {
    border-color: var(--border);
    color: var(--text);
  }

  .icon-picker-cell.selected {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent);
  }

  .icon-picker-cell :global(svg) {
    width: 22px;
    height: 22px;
  }

  .icon-picker-empty {
    grid-column: 1 / -1;
    margin: 0;
    padding: 20px 8px;
    text-align: center;
    font-size: 13px;
    color: var(--text-muted);
  }

  /* Auto-activate rules */
  :global(.dp-rule) {
    padding: 10px;
    border-radius: var(--radius-sm, 8px);
    background: var(--bg-hover, rgba(0, 0, 0, 0.03));
    margin-bottom: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  :global(.dp-rule-top) {
    display: flex;
    gap: 6px;
    align-items: flex-start;
    justify-content: space-between;
  }

  :global(.dp-rule-summary) {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  :global(.rule-app-name) {
    font-weight: 600;
    font-size: 13px;
    color: var(--text);
    word-break: break-all;
  }

  :global(.rule-app-type) {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  :global(.dp-rule-advanced) {
    display: flex;
    gap: 6px;
    align-items: center;
    margin-top: 4px;
    padding-top: 8px;
    border-top: 1px dashed var(--border-light);
  }

  :global(.dp-rule-advanced .dp-sel) { flex: 0 0 auto; min-width: 80px; }
  :global(.dp-rule-advanced .dp-in) { flex: 1; min-width: 0; }

  :global(.dp-rule-btns) {
    display: flex;
    gap: 10px;
  }

  :global(.dp-rule-result) {
    margin: 0;
    padding: 4px 8px;
    border-radius: 6px;
    background: var(--bg-elevated, #fff);
    font-size: 11px;
    color: var(--text-muted, #6e6e73);
    line-height: 1.4;
  }

  :global(.dp-picker) {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    padding-top: 6px;
    border-top: 1px dashed var(--border-light, rgba(0, 0, 0, 0.06));
  }

  :global(.dp-picker .dp-sel) { flex: 1; min-width: 0; }

  :global(.dp-muted) {
    font-size: 12px;
    color: var(--text-muted, #6e6e73);
  }

  /* Footer */
  :global(.dp-foot) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 20px;
    border-top: 1px solid var(--border-light, rgba(0, 0, 0, 0.04));
    flex-shrink: 0;
  }

  :global(.dp-foot-right) {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  @media (max-width: 480px) {
    :global(aside.dp-panel) { max-width: 100%; }
    :global(.dp-grid2),
    :global(.dp-inline-fields) { grid-template-columns: 1fr; }
  }
</style>
