<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import { fade, fly } from 'svelte/transition'
  import { invoke } from '$lib/backend'
  import Icon from '@iconify/svelte'
  import { sidebarDictationStore } from '../lib/sidebarDictation'
  import type { AppConfig, DictionaryEntry, FormattingRule } from '../types'

  /** Unified item for display - combines vocabulary and replacements */
  type DictItem =
    | { type: 'vocab'; id: string; term: string; sourceId: string }
    | { type: 'replacement'; id: string; pattern: string; replacement: string; rule: RuleRow }

  /** Stable key for each rule row */
  type RuleRow = {
    clientId: string
    pattern: string
    replacement: string
    enabled: boolean
    is_regex: boolean
  }

  let config: AppConfig | null = null
  let initialLoadDone = false
  let saveDebounceId: ReturnType<typeof setTimeout> | null = null
  let saving = false
  let saveError: string | null = null

  let ruleRows: RuleRow[] = []

  let dictionaryEntries: DictionaryEntry[] = []
  let dictionaryLoading = false
  /** User-visible errors from dictionary DB ops or panel validation */
  let dictionaryError: string | null = null

  /** Side panel state (Snippets-style: panel for both create and edit) */
  let editingItem: DictItem | null = null
  /** True when panel is open for a new entry (editingItem stays null until saved). */
  let panelCreating = false
  let isPanelOpen = false

  /** Panel form values */
  let editPattern = ''
  let editReplacement = ''
  let editIsRegex = false

  function ruleRowFromServer(r: FormattingRule): RuleRow {
    return {
      pattern: r.pattern ?? '',
      replacement: r.replacement ?? '',
      enabled: r.enabled !== false,
      is_regex: r.is_regex ?? true,
      clientId: crypto.randomUUID(),
    }
  }

  function rowsToFormattingRules(rows: RuleRow[]): FormattingRule[] {
    return rows.map(({ clientId, ...r }) => ({
      pattern: r.pattern,
      replacement: r.replacement,
      enabled: r.enabled,
      is_regex: r.is_regex,
    }))
  }

  function syncRuleRowsToConfig() {
    if (!config) return
    config.formatting.custom_rules = rowsToFormattingRules(ruleRows)
  }

  function scheduleSave() {
    if (!initialLoadDone || !config) return
    if (saveDebounceId != null) clearTimeout(saveDebounceId)
    saveDebounceId = setTimeout(() => {
      saveDebounceId = null
      void saveSettings()
    }, 400)
  }

  async function saveSettings() {
    if (!config) return
    saving = true
    saveError = null
    syncRuleRowsToConfig()
    try {
      await invoke('save_settings', { newConfig: config })
      const platform = (await invoke('get_platform')) as string
      sidebarDictationStore.updateFromConfig(config, platform)
    } catch (e) {
      saveError = String(e)
      console.error('Dictionary: save_settings failed:', e)
    } finally {
      saving = false
    }
  }

  $: allItems = [
    ...dictionaryEntries.map((e) => ({
      type: 'vocab' as const,
      id: `vocab-${e.id}`,
      term: e.term,
      sourceId: e.id,
    })),
    ...ruleRows.map((r) => ({
      type: 'replacement' as const,
      id: `rule-${r.clientId}`,
      pattern: r.pattern,
      replacement: r.replacement,
      rule: r,
    })),
  ]

  async function loadDictionaryEntries() {
    dictionaryLoading = true
    try {
      dictionaryEntries = (await invoke('get_dictionary_entries')) as DictionaryEntry[]
      dictionaryError = null
    } catch (e) {
      dictionaryEntries = []
      dictionaryError = String(e)
      console.error('Failed to load dictionary:', e)
    } finally {
      dictionaryLoading = false
    }
  }

  async function addDictionaryTerm(term: string): Promise<boolean> {
    const t = term.trim()
    if (!t) return false
    try {
      await invoke('add_dictionary_entry', { term: t })
      await loadDictionaryEntries()
      return true
    } catch (e) {
      dictionaryError = String(e)
      console.error('Failed to add dictionary term:', e)
      return false
    }
  }

  async function deleteDictionaryEntry(id: string, skipPanelClose = false): Promise<boolean> {
    try {
      await invoke('delete_dictionary_entry', { id })
      await loadDictionaryEntries()
      if (
        !skipPanelClose &&
        editingItem?.type === 'vocab' &&
        editingItem.sourceId === id
      ) {
        closePanel()
      }
      return true
    } catch (e) {
      dictionaryError = String(e)
      console.error('Failed to delete dictionary entry:', e)
      return false
    }
  }

  async function updateDictionaryEntry(id: string, term: string): Promise<boolean> {
    const t = term.trim()
    if (!t) return false
    try {
      await invoke('update_dictionary_entry', { id, term: t })
      await loadDictionaryEntries()
      return true
    } catch (e) {
      dictionaryError = String(e)
      console.error('Failed to update dictionary entry:', e)
      return false
    }
  }

  /** Drop a replacement rule and persist. When `skipPanelClose`, caller handles closing (e.g. after save). */
  function removeReplacementRow(clientId: string, skipPanelClose = false) {
    ruleRows = ruleRows.filter((r) => r.clientId !== clientId)
    if (
      !skipPanelClose &&
      editingItem?.type === 'replacement' &&
      editingItem.rule.clientId === clientId
    ) {
      closePanel()
    }
    syncRuleRowsToConfig()
    scheduleSave()
  }

  /** Portal action to move element to app-shell so it inherits themes */
  function portal(node: HTMLElement) {
    const target = document.querySelector('.app-shell') || document.body
    target.appendChild(node)
    return {
      destroy() {
        if (node.parentNode) {
          node.parentNode.removeChild(node)
        }
      }
    }
  }

  /** Open panel to add an entry (same flow as Snippets → New snippet). */
  function openNewPanel() {
    panelCreating = true
    editingItem = null
    isPanelOpen = true
    dictionaryError = null
    editPattern = ''
    editReplacement = ''
    editIsRegex = false
  }

  /** Open side panel for editing */
  function openEditPanel(item: DictItem) {
    panelCreating = false
    editingItem = item
    isPanelOpen = true
    dictionaryError = null

    if (item.type === 'vocab') {
      editPattern = item.term
      editReplacement = ''
      editIsRegex = false
    } else {
      editPattern = item.pattern
      editReplacement = item.replacement
      editIsRegex = item.rule.is_regex
    }
  }

  function closePanel() {
    isPanelOpen = false
    setTimeout(() => {
      editingItem = null
      panelCreating = false
    }, 200)
  }

  /** Save from side panel (create or edit). */
  async function savePanel() {
    const pattern = editPattern.trim()
    if (!pattern) return

    const replacement = editReplacement.trim()
    dictionaryError = null

    if (panelCreating) {
      if (dictionaryLoading) return
      if (replacement) {
        try {
          await invoke('validate_replacement_pattern', {
            pattern,
            isRegex: editIsRegex,
          })
        } catch (e) {
          dictionaryError = String(e)
          return
        }
      }
      if (!replacement) {
        const ok = await addDictionaryTerm(pattern)
        if (ok) closePanel()
        return
      }
      if (!config) return
      ruleRows = [
        ...ruleRows,
        {
          clientId: crypto.randomUUID(),
          pattern,
          replacement,
          enabled: true,
          is_regex: false,
        },
      ]
      syncRuleRowsToConfig()
      scheduleSave()
      closePanel()
      return
    }

    const item = editingItem
    if (!item) return

    const row =
      item.type === 'replacement'
        ? ruleRows.find((r) => r.clientId === item.rule.clientId)
        : null
    if (item.type === 'replacement' && !row) return

    // Converting a replacement row to vocabulary-only: pattern is stored as plain text, not regex.
    const convertingReplacementToVocab = item.type === 'replacement' && !replacement

    if (!convertingReplacementToVocab && (replacement || item.type === 'replacement')) {
      const isRegex =
        item.type === 'vocab'
          ? Boolean(replacement) && editIsRegex
          : Boolean(replacement) && (row?.is_regex ?? false)
      try {
        await invoke('validate_replacement_pattern', {
          pattern,
          isRegex,
        })
      } catch (e) {
        dictionaryError = String(e)
        return
      }
    }

    if (item.type === 'vocab') {
      if (!replacement) {
        const ok = await updateDictionaryEntry(item.sourceId, pattern)
        if (!ok) return
      } else {
        const delOk = await deleteDictionaryEntry(item.sourceId, true)
        if (!delOk) return
        ruleRows = [
          ...ruleRows,
          {
            clientId: crypto.randomUUID(),
            pattern,
            replacement,
            enabled: true,
            is_regex: editIsRegex,
          },
        ]
        syncRuleRowsToConfig()
        scheduleSave()
      }
    } else {
      const editingRow = row
      if (!editingRow) return
      if (!replacement) {
        const added = await addDictionaryTerm(pattern)
        if (!added) return
        removeReplacementRow(editingRow.clientId, true)
      } else {
        editingRow.pattern = pattern
        editingRow.replacement = replacement
        editingRow.is_regex = editIsRegex
        ruleRows = [...ruleRows]
        syncRuleRowsToConfig()
        scheduleSave()
      }
    }

    closePanel()
  }

  function deleteCurrentItem() {
    if (panelCreating || !editingItem) return
    const item = editingItem
    if (item.type === 'vocab') {
      deleteDictionaryEntry(item.sourceId)
    } else {
      removeReplacementRow(item.rule.clientId)
    }
  }

  onMount(async () => {
    try {
      const settings = (await invoke('get_settings')) as AppConfig
      config = settings
      if (!config.formatting.custom_rules) config.formatting.custom_rules = []
      ruleRows = config.formatting.custom_rules.map(ruleRowFromServer)
      await loadDictionaryEntries()
    } catch (e) {
      console.error('Dictionary page load failed:', e)
      config = null
    } finally {
      initialLoadDone = true
    }
  })

  onDestroy(() => {
    if (saveDebounceId != null) clearTimeout(saveDebounceId)
  })
</script>

<div class="page fade-in">
  <header class="page-header notes-header">
    <div>
      <h1 class="page-title">Dictionary</h1>
      <p class="page-subtitle">Words to recognize and automatic corrections</p>
    </div>
    {#if config}
      <button
        type="button"
        class="btn-primary"
        disabled={dictionaryLoading}
        on:click={openNewPanel}
      >
        <Icon icon="ph:plus" />
        New item
      </button>
    {/if}
  </header>

  {#if config}
    <section class="dict-panel">
      {#if dictionaryError}
        <p class="dict-save-error" role="alert">{dictionaryError}</p>
      {/if}
      {#if saveError}
        <p class="dict-save-error" role="alert">{saveError}</p>
      {/if}
      {#if saving}
        <p class="dict-saving" aria-live="polite">Saving…</p>
      {/if}

      <!-- List -->
      {#if dictionaryLoading && dictionaryEntries.length === 0 && ruleRows.length === 0}
        <div class="state-container empty-state">
          <Icon icon="ph:spinner-gap" class="spin-icon" />
          <p>Loading…</p>
        </div>
      {:else if allItems.length === 0}
        <div class="dict-empty">
          <Icon icon="ph:book-open-text" class="dict-empty-icon" />
          <p>Your dictionary is empty</p>
          <span class="dict-empty-hint">Use <strong>New item</strong> to add vocabulary or find-and-replace rules</span>
        </div>
      {:else}
        <ul class="dict-list" aria-label="Dictionary entries">
          {#each allItems as item (item.id)}
            <li class="dict-item" class:dict-item-replacement={item.type === 'replacement'}>
              <button
                type="button"
                class="dict-item-content"
                on:click={() => openEditPanel(item)}
              >
                {#if item.type === 'vocab'}
                  <span class="dict-term">{item.term}</span>
                {:else}
                  <div class="dict-replacement-preview">
                    <span class="dict-term">{item.pattern}</span>
                    <span class="dict-arrow-small" aria-hidden="true">→</span>
                    <span class="dict-replacement-target">{item.replacement}</span>
                    {#if !item.rule.enabled}
                      <span class="dict-inactive-hint">Inactive</span>
                    {/if}
                  </div>
                {/if}
              </button>
              <button
                type="button"
                class="btn-icon remove"
                title="Remove"
                aria-label="Remove"
                on:click={(e) => {
                  e.stopPropagation()
                  if (item.type === 'vocab') {
                    deleteDictionaryEntry(item.sourceId)
                  } else {
                    removeReplacementRow(item.rule.clientId)
                  }
                }}
              >
                <Icon icon="ph:trash" />
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {:else}
    <div class="state-container empty-state">
      <p>Could not load settings.</p>
    </div>
  {/if}

  <!-- Side Panel Overlay -->
  {#if isPanelOpen}
    <div
      class="dict-overlay"
      role="button"
      tabindex="0"
      aria-label="Close panel"
      on:click={closePanel}
      on:keydown={(e) => e.key === 'Enter' && closePanel()}
      transition:fade
      use:portal
    ></div>
  {/if}

  <!-- Side Panel -->
  {#if isPanelOpen}
    <aside class="dict-side-panel" transition:fly={{ x: 420, duration: 250, opacity: 1 }} use:portal>
      <div class="dict-panel-header">
        <h3>{panelCreating ? 'New item' : 'Edit entry'}</h3>
        <button type="button" class="btn-icon" on:click={closePanel} aria-label="Close">
          <Icon icon="ph:x" />
        </button>
      </div>

      <div class="dict-panel-body">
        <!-- Pattern input -->
        <label class="dict-field">
          <span>Word or phrase</span>
          <input
            type="text"
            class="dict-input"
            bind:value={editPattern}
            placeholder="What the recognizer hears"
          />
        </label>

        <!-- Replacement section - always visible, just type to enable -->
        <div class="dict-replacement-section">
          <div class="dict-field-label">
            <span>Replace with</span>
          </div>
          <!-- Grid: column 2 matches input width so the badge aligns with the field, not the full panel -->
          <div class="dict-replacement-fields">
            <span class="dict-equals" aria-hidden="true">=</span>
            <input
              type="text"
              class="dict-input"
              bind:value={editReplacement}
              placeholder="Leave empty for vocabulary-only"
            />
            {#if editReplacement.trim()}
              <div class="dict-replacement-badge-wrap" transition:fade>
                <div class="dict-replacement-badge">
                  <Icon icon="ph:arrows-left-right" />
                  <span>Will be replaced in transcript</span>
                </div>
              </div>
            {/if}
          </div>
        </div>
        {#if dictionaryError}
          <p class="dict-save-error dict-panel-inline-error" role="alert">{dictionaryError}</p>
        {/if}
      </div>

      <div class="dict-panel-footer">
        <div>
          {#if editingItem}
            <button type="button" class="btn-ghost danger" on:click={deleteCurrentItem}>
              <Icon icon="ph:trash" />
              Delete
            </button>
          {/if}
        </div>
        <div class="dict-panel-actions">
          <button type="button" class="btn-ghost" on:click={closePanel}>Cancel</button>
          <button
            type="button"
            class="btn-primary"
            disabled={!editPattern.trim() || (panelCreating && dictionaryLoading)}
            on:click={savePanel}
          >
            Save
          </button>
        </div>
      </div>
  </aside>
  {/if}
</div>

<style>
  /* Main column matches .page-header width (full .page-content); avoid a narrower max-width than other list pages. */

  .dict-input {
    flex: 1;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    font-family: inherit;
    font-size: 14px;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .dict-input:focus {
    outline: none;
    border-color: var(--border-subtle);
    box-shadow: 0 0 0 3px var(--primary-alpha-subtle);
  }

  /* Buttons */
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
    transition: opacity 0.15s ease, transform 0.1s ease;
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn-primary:active:not(:disabled) {
    transform: scale(0.98);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-ghost {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-full, 9999px);
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: border-color 0.15s ease, background 0.15s ease, color 0.15s ease;
  }

  .btn-ghost:hover {
    border-color: var(--border-hover);
    background: var(--bg-hover);
    color: var(--text);
  }

  .btn-ghost.danger {
    color: var(--danger, #ef4444);
    border-color: var(--danger-soft, rgba(239, 68, 68, 0.3));
  }

  .btn-ghost.danger:hover {
    background: var(--danger-soft, rgba(239, 68, 68, 0.1));
  }

  .btn-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;
  }

  .btn-icon:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .btn-icon.remove:hover {
    background: var(--danger-soft, rgba(239, 68, 68, 0.1));
    color: var(--danger, #ef4444);
  }

  /* Status messages */
  .dict-save-error {
    color: var(--danger, #ef4444);
    font-size: 13px;
    margin: 0 0 12px;
    padding: 8px 12px;
    background: var(--danger-soft, rgba(239, 68, 68, 0.1));
    border-radius: var(--radius-sm);
  }

  .dict-panel-inline-error {
    margin-top: 12px;
    margin-bottom: 0;
  }

  .dict-saving {
    font-size: 13px;
    color: var(--text-muted);
    margin: 0 0 12px;
  }

  /* Empty state */
  .dict-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 48px 24px;
    text-align: center;
    color: var(--text-muted);
  }

  .dict-empty :global(svg) {
    width: 48px;
    height: 48px;
    margin-bottom: 16px;
    opacity: 0.5;
  }

  .dict-empty p {
    margin: 0 0 4px;
    font-size: 16px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .dict-empty-hint {
    font-size: 13px;
    color: var(--text-muted);
  }

  .dict-empty-hint strong {
    color: var(--text);
    font-weight: 600;
  }

  /* List */
  .dict-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .dict-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 4px 4px 14px;
    background: var(--bg-elevated);
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    transition: border-color 0.15s ease, background 0.15s ease, transform 0.1s ease;
  }

  .dict-item:hover {
    border-color: var(--border-hover);
    background: var(--bg-hover);
  }

  .dict-item:active {
    transform: scale(0.995);
  }

  .dict-item-replacement {
    background: linear-gradient(to right, var(--bg-elevated), color-mix(in srgb, var(--primary) 4%, var(--bg-elevated)));
  }

  .dict-item-replacement:hover {
    background: linear-gradient(to right, var(--bg-hover), color-mix(in srgb, var(--primary) 6%, var(--bg-hover)));
  }

  .dict-item-content {
    flex: 1;
    display: flex;
    align-items: center;
    min-width: 0;
    padding: 8px 0;
    border: none;
    background: transparent;
    text-align: left;
    cursor: pointer;
    color: inherit;
    font: inherit;
  }

  .dict-term {
    font-size: 14px;
    font-weight: 500;
    color: var(--text);
    word-break: break-word;
  }

  .dict-replacement-preview {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .dict-arrow-small {
    color: var(--text-muted);
    font-size: 14px;
    font-weight: 500;
  }

  .dict-replacement-target {
    font-size: 14px;
    font-weight: 500;
    color: var(--primary);
    word-break: break-word;
  }

  .dict-inactive-hint {
    margin-left: 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }

  /* Side Panel */
  :global(.dict-overlay) {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(2px);
    z-index: 9998;
  }

  :global(aside.dict-side-panel) {
    position: fixed;
    top: 0;
    right: 0;
    width: 100%;
    max-width: 420px;
    height: 100vh;
    max-height: 100vh;
    background: var(--bg-elevated);
    border-left: 1px solid var(--border);
    color: var(--text);
    z-index: 9999;
    display: flex;
    flex-direction: column;
    box-shadow: -4px 0 24px rgba(0, 0, 0, 0.15);
  }

  :global(.dict-panel-header) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  :global(.dict-panel-header h3) {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  :global(.dict-panel-body) {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  :global(.dict-panel-footer) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 16px 20px;
    border-top: 1px solid var(--border);
  }

  :global(.dict-panel-actions) {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  /* Form fields in panel */
  :global(.dict-field) {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  :global(.dict-field span),
  :global(.dict-field-label span) {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
  }

  /* Replacement section */
  :global(.dict-replacement-section) {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  :global(.dict-replacement-fields) {
    display: grid;
    grid-template-columns: auto 1fr;
    column-gap: 12px;
    row-gap: 10px;
    align-items: center;
  }

  :global(.dict-replacement-fields > .dict-equals) {
    grid-column: 1;
    grid-row: 1;
  }

  :global(.dict-replacement-fields > .dict-input) {
    grid-column: 2;
    grid-row: 1;
    min-width: 0;
    width: 100%;
  }

  :global(.dict-replacement-badge-wrap) {
    grid-column: 2;
    grid-row: 2;
    min-width: 0;
  }

  :global(.dict-equals) {
    color: var(--text-muted);
    font-size: 18px;
    font-weight: 600;
    flex-shrink: 0;
    font-family: var(--font, inherit);
  }

  :global(.dict-replacement-badge) {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    box-sizing: border-box;
    padding: 6px 12px;
    background: var(--primary-alpha);
    border-radius: var(--radius-full);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  :global(.dict-replacement-badge :global(svg)) {
    width: 14px;
    height: 14px;
  }

  /* Responsive */
  @media (max-width: 480px) {
    :global(aside.dict-side-panel) {
      max-width: 100%;
    }

    :global(.dict-replacement-input) {
      padding-left: 0;
      flex-direction: column;
      align-items: flex-start;
    }

    :global(.dict-arrow-large) {
      display: none;
    }
  }
</style>
