<script lang="ts">
  import { onMount } from 'svelte'
  import { fade, fly } from 'svelte/transition'
  import { invoke } from '$lib/backend'
  import Icon from '@iconify/svelte'
  import type { Snippet } from '../types'

  let snippets: Snippet[] = []
  let loading = true
  let searchQuery = ''
  /** Collapsed by default so the list stays primary. */
  let howSnippetsOpen = false

  /** Side panel — same interaction model as Dictionary */
  let isPanelOpen = false
  /** Original trigger when editing; null when creating */
  let existingTrigger: string | null = null
  let panelTrigger = ''
  let panelExpansion = ''
  let panelSaving = false
  let panelSaveError: string | null = null

  $: q = searchQuery.trim().toLowerCase()
  $: filteredSnippets = snippets.filter((s) => {
    if (!q) return true
    return s.trigger.toLowerCase().includes(q) || s.expansion.toLowerCase().includes(q)
  })

  onMount(loadSnippets)

  async function loadSnippets() {
    loading = true
    try {
      snippets = await invoke('get_snippets')
    } catch (e) {
      console.error('Failed to load snippets:', e)
    } finally {
      loading = false
    }
  }

  /** Portal action to move overlay/panel to app-shell so it inherits themes */
  function portal(node: HTMLElement) {
    const target = document.querySelector('.app-shell') || document.body
    target.appendChild(node)
    return {
      destroy() {
        if (node.parentNode) {
          node.parentNode.removeChild(node)
        }
      },
    }
  }

  function openEditPanel(snippet: Snippet | null) {
    panelSaveError = null
    existingTrigger = snippet ? snippet.trigger : null
    panelTrigger = snippet?.trigger ?? ''
    panelExpansion = snippet?.expansion ?? ''
    isPanelOpen = true
  }

  function closePanel() {
    isPanelOpen = false
    /* Fields reset on next openEditPanel; avoid clearing mid-fly-out (250ms). */
  }

  async function savePanel() {
    panelSaveError = null
    const tr = panelTrigger.trim()
    const ex = panelExpansion.trim()
    if (!tr || !ex) {
      panelSaveError = 'Trigger and expansion are both required.'
      return
    }
    panelSaving = true
    try {
      if (existingTrigger && existingTrigger !== tr) {
        await invoke('remove_snippet', { trigger: existingTrigger })
      }
      await invoke('add_snippet', { trigger: tr, expansion: ex })
      await loadSnippets()
      closePanel()
    } catch (e) {
      console.error(e)
      panelSaveError = e instanceof Error ? e.message : 'Save failed.'
    } finally {
      panelSaving = false
    }
  }

  async function deletePanelSnippet() {
    if (!existingTrigger) return
    if (!confirm('Delete this snippet?')) return
    panelSaveError = null
    try {
      await invoke('remove_snippet', { trigger: existingTrigger })
      await loadSnippets()
      closePanel()
    } catch (e) {
      console.error(e)
      panelSaveError = e instanceof Error ? e.message : 'Delete failed.'
    }
  }

  function toggleHowSnippets() {
    howSnippetsOpen = !howSnippetsOpen
  }
</script>

<div class="page fade-in">
  <header class="page-header notes-header">
    <div>
      <h1 class="page-title">Snippets</h1>
      <p class="page-subtitle">Text shortcuts for quick expansion.</p>
    </div>
    <button type="button" class="btn-primary" on:click={() => openEditPanel(null)}>
      <Icon icon="ph:plus" />
      New Snippet
    </button>
  </header>

  <div class="snippets-help-accordion">
    <button
      type="button"
      class="snippets-help-toggle"
      aria-expanded={howSnippetsOpen}
      aria-controls="snippets-help-panel"
      id="snippets-help-label"
      on:click={toggleHowSnippets}
    >
      <span class="snippets-help-toggle-inner">
        <span class="snippets-help-toggle-icon" aria-hidden="true">
          <Icon icon="ph:info" />
        </span>
        <span class="snippets-help-toggle-text">How snippets work</span>
      </span>
      <span class="snippets-help-caret" aria-hidden="true">
        <Icon icon={howSnippetsOpen ? 'ph:caret-up' : 'ph:caret-down'} />
      </span>
    </button>
    {#if howSnippetsOpen}
      <div
        id="snippets-help-panel"
        class="snippets-help-panel"
        role="region"
        aria-labelledby="snippets-help-label"
      >
        <ul class="snippets-help-list">
          <li>
            After dictation, Kalam looks for your <strong>trigger</strong> in the transcribed text and replaces it with
            the <strong>expansion</strong> before the result is pasted into the target app.
          </li>
          <li>
            There is <strong>no special prefix</strong>: whatever you save as the trigger is the exact substring that
            must appear in the transcript (same words, symbols, and spacing as Kalam transcribes; matching ignores case).
          </li>
          <li>
            If one trigger appears inside another, <strong>longer triggers are applied first</strong> so a short code
            does not break a longer phrase.
          </li>
        </ul>
      </div>
    {/if}
  </div>

  {#if !loading && snippets.length > 0}
    <div class="notes-search-bar">
      <span class="notes-search-bar-icon" aria-hidden="true">
        <Icon icon="ph:magnifying-glass" />
      </span>
      <input type="text" placeholder="Search snippets..." bind:value={searchQuery} />
    </div>
  {/if}

  {#if loading}
    <div class="state-container empty-state">
      <Icon icon="ph:spinner-gap" class="spin-icon" />
      <p>Loading snippets...</p>
    </div>
  {:else}
    <div class="snippets-grid">
      {#each filteredSnippets as snippet (snippet.trigger)}
        <button
          type="button"
          class="snippet-card"
          on:click={() => openEditPanel(snippet)}
        >
          <div class="snippet-header">
            <code class="trigger-code">{snippet.trigger}</code>
          </div>
          <p class="expansion-text">{snippet.expansion}</p>
        </button>
      {/each}
      {#if filteredSnippets.length === 0}
        <div class="empty-state">
          <Icon icon="ph:textbox" />
          <p>{searchQuery ? 'No snippets match your search' : 'No snippets yet'}</p>
        </div>
      {/if}
    </div>
  {/if}
</div>

{#if isPanelOpen}
  <div
    class="snippet-overlay"
    role="button"
    tabindex="0"
    aria-label="Close panel"
    on:click={closePanel}
    on:keydown={(e) => e.key === 'Enter' && closePanel()}
    transition:fade
    use:portal
  ></div>
{/if}

{#if isPanelOpen}
  <aside
    class="snippet-side-panel"
    transition:fly={{ x: 420, duration: 250, opacity: 1 }}
    use:portal
  >
    <div class="snippet-panel-header">
      <h3>{existingTrigger ? 'Edit snippet' : 'New snippet'}</h3>
      <button type="button" class="btn-icon" on:click={closePanel} aria-label="Close">
        <Icon icon="ph:x" />
      </button>
    </div>

    <div class="snippet-panel-body">
      <label class="snippet-field">
        <span>Trigger</span>
        <input
          type="text"
          class="snippet-panel-input"
          bind:value={panelTrigger}
          placeholder="Exact phrase as it appears in transcripts"
        />
        <p class="snippet-field-hint">Must match exactly what Kalam transcribes (same spelling and spacing).</p>
      </label>

      <label class="snippet-field">
        <span>Expansion</span>
        <textarea
          class="snippet-panel-textarea"
          bind:value={panelExpansion}
          placeholder="Text to insert instead of the trigger"
          rows="8"
        ></textarea>
      </label>

      {#if panelSaveError}
        <p class="snippet-panel-error" role="alert">{panelSaveError}</p>
      {/if}
    </div>

    <div class="snippet-panel-footer">
      <div>
        {#if existingTrigger}
          <button type="button" class="btn-ghost danger" on:click={deletePanelSnippet}>
            <Icon icon="ph:trash" />
            Delete
          </button>
        {/if}
      </div>
      <div class="snippet-panel-actions">
        <button type="button" class="btn-ghost" on:click={closePanel}>Cancel</button>
        <button
          type="button"
          class="btn-primary"
          on:click={savePanel}
          disabled={!panelTrigger?.trim() || !panelExpansion?.trim() || panelSaving}
        >
          Save
        </button>
      </div>
    </div>
  </aside>
{/if}

<style>
  .snippets-help-accordion {
    margin-bottom: var(--space-lg);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-light);
    background: var(--bg-elevated);
    overflow: hidden;
  }

  .snippets-help-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    margin: 0;
    padding: 10px 14px;
    border: none;
    background: transparent;
    cursor: pointer;
    font: inherit;
    text-align: left;
    color: var(--text);
    transition: background var(--transition, 0.15s ease);
  }

  .snippets-help-toggle:hover {
    background: var(--bg-hover);
  }

  .snippets-help-toggle-inner {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .snippets-help-toggle-icon {
    display: inline-flex;
    flex-shrink: 0;
    color: var(--text-secondary);
    font-size: 18px;
  }

  .snippets-help-toggle-icon :global(svg) {
    width: 1em;
    height: 1em;
  }

  .snippets-help-toggle-text {
    font-size: 13px;
    font-weight: 600;
  }

  .snippets-help-caret {
    flex-shrink: 0;
    display: inline-flex;
    font-size: 16px;
    color: var(--text-muted);
  }

  .snippets-help-caret :global(svg) {
    width: 1em;
    height: 1em;
  }

  .snippets-help-panel {
    padding: 0 14px 12px 14px;
    border-top: 1px solid var(--border-light);
  }

  .snippets-help-list {
    margin: 10px 0 0;
    padding-left: 1.25rem;
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-secondary);
  }

  .snippets-help-list li + li {
    margin-top: 0.45rem;
  }

  .snippets-help-list strong {
    color: var(--text);
    font-weight: 600;
  }

  /* Empty / no-results row sits in a multi-column grid; span all columns */
  .snippets-grid > .empty-state {
    grid-column: 1 / -1;
  }

  /* List cards as buttons: align with global .snippet-card look from App.svelte */
  .snippet-card {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    width: 100%;
    margin: 0;
    font: inherit;
    text-align: left;
    color: inherit;
    box-sizing: border-box;
  }

  .snippet-card:active {
    transform: scale(0.995);
  }

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
    transition:
      border-color 0.15s ease,
      background 0.15s ease,
      color 0.15s ease;
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
    transition:
      background 0.15s ease,
      color 0.15s ease;
  }

  .btn-icon:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  /* Side panel — mirror Dictionary overlay / aside (portaled, :global z-index) */
  :global(.snippet-overlay) {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(2px);
    z-index: 9998;
  }

  :global(aside.snippet-side-panel) {
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

  :global(.snippet-panel-header) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  :global(.snippet-panel-header h3) {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  :global(.snippet-panel-body) {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  :global(.snippet-panel-footer) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 16px 20px;
    border-top: 1px solid var(--border);
  }

  :global(.snippet-panel-actions) {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  :global(.snippet-field) {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 0;
  }

  :global(.snippet-field > span) {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
  }

  :global(.snippet-panel-input),
  :global(.snippet-panel-textarea) {
    width: 100%;
    box-sizing: border-box;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    font-family: inherit;
    font-size: 14px;
    transition:
      border-color 0.15s ease,
      box-shadow 0.15s ease;
  }

  :global(.snippet-panel-textarea) {
    min-height: 120px;
    resize: vertical;
    line-height: 1.5;
  }

  :global(.snippet-panel-input:focus),
  :global(.snippet-panel-textarea:focus) {
    outline: none;
    border-color: var(--border-subtle);
    box-shadow: 0 0 0 3px var(--primary-alpha-subtle);
  }

  :global(.snippet-field-hint) {
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-muted);
  }

  :global(.snippet-panel-error) {
    margin: 0;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--danger, #dc2626) 12%, transparent);
    color: var(--text);
    font-size: 0.875rem;
    line-height: 1.4;
  }

  @media (max-width: 480px) {
    :global(aside.snippet-side-panel) {
      max-width: 100%;
    }
  }
</style>
