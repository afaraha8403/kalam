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

  $: filteredSnippets = snippets.filter(s =>
    s.trigger.toLowerCase().includes(searchQuery.toLowerCase()) ||
    s.expansion.toLowerCase().includes(searchQuery.toLowerCase())
  )

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

  async function removeSnippet(trigger: string) {
    try {
      await invoke('remove_snippet', { trigger })
      await loadSnippets()
    } catch (e) {
      console.error('Failed to remove snippet:', e)
    }
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

  <div class="notes-search-bar">
    <span class="notes-search-bar-icon" aria-hidden="true">
      <Icon icon="ph:magnifying-glass" />
    </span>
    <input type="text" placeholder="Search snippets..." bind:value={searchQuery} />
  </div>

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
            <code class="trigger-code">/{snippet.trigger}</code>
            <span class="uses-count">0 uses</span>
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
