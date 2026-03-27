<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '$lib/backend'
  import Icon from '@iconify/svelte'
  import type { Snippet } from '../types'
  import { selectedSnippetTrigger } from '../lib/snippetDetailStore'

  export let navigate: (page: string) => void = () => {}

  let snippets: Snippet[] = []
  let loading = true
  let searchQuery = ''
  /** Collapsed by default so the list stays primary. */
  let howSnippetsOpen = false

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

  function openNewSnippet() {
    selectedSnippetTrigger.set(null)
    navigate('snippet-detail')
  }

  function openSnippet(snippet: Snippet) {
    selectedSnippetTrigger.set(snippet.trigger)
    navigate('snippet-detail')
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
    <button type="button" class="btn-primary" on:click={openNewSnippet}>
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
        <div
          class="snippet-card"
          role="button"
          tabindex="0"
          on:click={() => openSnippet(snippet)}
          on:keydown={(e) => e.key === 'Enter' && openSnippet(snippet)}
        >
          <div class="snippet-header">
            <code class="trigger-code">{snippet.trigger}</code>
          </div>
          <p class="expansion-text">{snippet.expansion}</p>
        </div>
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

  /* Empty / no-results row sits in a multi-column grid; span all columns so flex centering uses full width. */
  .snippets-grid > .empty-state {
    grid-column: 1 / -1;
  }
</style>
