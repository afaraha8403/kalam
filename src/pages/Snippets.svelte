<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import Icon from '@iconify/svelte'
  import type { Snippet } from '../types'
  import SidePanel from '../components/ui/SidePanel.svelte'
  import SearchFilterBar from '../components/ui/SearchFilterBar.svelte'

  let snippets: Snippet[] = []
  let newTrigger = ''
  let newExpansion = ''
  let loading = true
  let editing: string | null = null
  let isPanelOpen = false
  let searchQuery = ''

  $: filteredSnippets = snippets.filter(s => 
    s.trigger.toLowerCase().includes(searchQuery.toLowerCase()) || 
    s.expansion.toLowerCase().includes(searchQuery.toLowerCase())
  )

  onMount(async () => {
    await loadSnippets()
  })

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

  function openAddPanel() {
    editing = null
    newTrigger = ''
    newExpansion = ''
    isPanelOpen = true
  }

  async function addSnippet() {
    if (!newTrigger.trim() || !newExpansion.trim()) return

    try {
      await invoke('add_snippet', {
        trigger: newTrigger,
        expansion: newExpansion
      })
      await loadSnippets()
      isPanelOpen = false
    } catch (e) {
      console.error('Failed to add snippet:', e)
    }
  }

  async function removeSnippet(trigger: string) {
    try {
      await invoke('remove_snippet', { trigger })
      await loadSnippets()
    } catch (e) {
      console.error('Failed to remove snippet:', e)
    }
  }

  function startEdit(snippet: Snippet) {
    editing = snippet.trigger
    newTrigger = snippet.trigger
    newExpansion = snippet.expansion
    isPanelOpen = true
  }

  async function saveEdit() {
    if (!editing) return
    
    try {
      await invoke('remove_snippet', { trigger: editing })
      await invoke('add_snippet', {
        trigger: newTrigger,
        expansion: newExpansion
      })
      await loadSnippets()
      isPanelOpen = false
    } catch (e) {
      console.error('Failed to save edit:', e)
    }
  }

  function closePanel() {
    isPanelOpen = false
  }
</script>

<div class="snippets-view">
  <header class="page-header">
    <div class="header-content">
      <div class="title-wrapper">
        <Icon icon="ph:scissors-duotone" class="header-icon" />
        <h2>Snippets</h2>
      </div>
      <p class="subtitle">Create magical shortcuts for your most used phrases.</p>
    </div>
    <div class="header-actions">
      <button class="btn-primary" on:click={openAddPanel}>
        <Icon icon="ph:plus-bold" /> Add Snippet
      </button>
    </div>
  </header>

  <SearchFilterBar bind:searchQuery placeholder="Search snippets..." />

  {#if loading}
    <div class="state-container">
      <Icon icon="ph:spinner-gap-duotone" class="spin-icon" />
      <p>Loading snippets...</p>
    </div>
  {:else if snippets.length === 0}
    <div class="state-container empty-state">
      <div class="empty-icon-wrapper">
        <Icon icon="ph:magic-wand-duotone" class="empty-icon" />
      </div>
      <h3>No snippets yet</h3>
      <p>Create your first shortcut to save time typing.</p>
    </div>
  {:else if filteredSnippets.length === 0}
    <div class="state-container empty-state">
      <div class="empty-icon-wrapper">
        <Icon icon="ph:magnifying-glass-duotone" class="empty-icon" />
      </div>
      <h3>No results found</h3>
      <p>Try adjusting your search query.</p>
    </div>
  {:else}
    <div class="snippets-grid">
      {#each filteredSnippets as snippet}
        <div class="snippet-card">
          <div class="snippet-trigger">
            <span class="badge">{snippet.trigger}</span>
          </div>
          <div class="snippet-expansion">
            <p>{snippet.expansion}</p>
          </div>
          <div class="snippet-hover-actions">
            <button class="action-btn edit" on:click={() => startEdit(snippet)} title="Edit">
              <Icon icon="ph:pencil-simple-duotone" />
            </button>
            <button class="action-btn delete" on:click={() => removeSnippet(snippet.trigger)} title="Delete">
              <Icon icon="ph:trash-duotone" />
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <SidePanel 
    isOpen={isPanelOpen} 
    title={editing ? 'Edit Snippet' : 'Create New Snippet'} 
    on:close={closePanel}
  >
    <div slot="body" class="panel-form">
      <div class="input-group">
        <label for="trigger-phrase">Trigger</label>
        <div class="input-wrapper">
          <Icon icon="ph:lightning-duotone" class="input-icon" />
          <input
            id="trigger-phrase"
            type="text"
            bind:value={newTrigger}
            placeholder="e.g. @@email"
            autocomplete="off"
          />
        </div>
      </div>
      
      <div class="input-group">
        <label for="expanded-text">Expansion</label>
        <div class="input-wrapper">
          <Icon icon="ph:text-aa-duotone" class="input-icon" />
          <textarea
            id="expanded-text"
            bind:value={newExpansion}
            placeholder="e.g. hello@example.com"
            autocomplete="off"
            rows="5"
          ></textarea>
        </div>
      </div>
    </div>
    
    <div slot="footer">
      <button class="btn-ghost" on:click={closePanel}>Cancel</button>
      <button class="btn-primary" on:click={editing ? saveEdit : addSnippet} disabled={!newTrigger.trim() || !newExpansion.trim()}>
        {#if editing}
          <Icon icon="ph:check-bold" /> Save Changes
        {:else}
          <Icon icon="ph:plus-bold" /> Add Snippet
        {/if}
      </button>
    </div>
  </SidePanel>
</div>

<style>
  .snippets-view {
    max-width: 1000px;
    margin: 0 auto;
    animation: fadeSlideUp 0.5s cubic-bezier(0.16, 1, 0.3, 1);
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  @keyframes fadeSlideUp {
    from { opacity: 0; transform: translateY(20px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* Header */
  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: 16px;
  }

  .header-content {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .title-wrapper {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .header-icon {
    font-size: 24px;
    color: var(--primary);
  }

  h2 {
    font-size: 24px;
    font-weight: 700;
    color: var(--navy-deep);
    margin: 0;
  }

  .subtitle {
    color: var(--text-muted);
    font-size: 15px;
    margin: 0;
    padding-left: 34px;
  }

  .header-actions {
    display: flex;
    gap: 12px;
  }

  /* Form inside Panel */
  .panel-form {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  label {
    font-size: 13px;
    font-weight: 700;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .input-wrapper {
    position: relative;
    display: flex;
    align-items: flex-start;
  }

  .input-icon {
    position: absolute;
    left: 16px;
    top: 14px;
    font-size: 18px;
    color: var(--text-muted);
    pointer-events: none;
  }

  input, textarea {
    width: 100%;
    padding: 14px 16px 14px 44px;
    background: var(--bg-input);
    border: 2px solid transparent;
    border-radius: 12px;
    color: var(--text-primary);
    font-size: 15px;
    font-weight: 500;
    font-family: inherit;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  textarea {
    resize: vertical;
    min-height: 120px;
  }

  input:focus, textarea:focus {
    outline: none;
    background: var(--bg-card);
    border-color: var(--primary);
    box-shadow: 0 0 0 4px var(--primary-alpha);
  }

  input:hover:not(:focus), textarea:hover:not(:focus) {
    background: var(--bg-input-hover);
  }

  .btn-primary {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 24px;
    background: var(--primary);
    color: white;
    border: none;
    border-radius: 10px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
    box-shadow: 0 4px 12px var(--primary-alpha);
  }

  .btn-primary:hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow: 0 6px 16px var(--primary-alpha);
    background: var(--primary-dark);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    box-shadow: none;
  }

  .btn-ghost {
    padding: 12px 20px;
    background: transparent;
    color: var(--text-secondary);
    border: none;
    border-radius: 10px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-ghost:hover {
    background: var(--bg-input);
    color: var(--navy-deep);
  }

  /* Grid & Cards */
  .snippets-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 20px;
  }

  .snippet-card {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    position: relative;
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.02);
  }

  .snippet-card:hover {
    transform: translateY(-4px);
    box-shadow: 0 12px 24px rgba(0, 0, 0, 0.06);
    border-color: var(--border-visible);
  }

  .snippet-trigger {
    display: flex;
    align-items: center;
  }

  .badge {
    background: var(--primary-alpha-light);
    color: var(--primary-dark);
    padding: 6px 12px;
    border-radius: 8px;
    font-family: 'DM Sans', monospace;
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.02em;
    border: 1px solid var(--primary-alpha);
  }

  .snippet-expansion p {
    margin: 0;
    font-size: 15px;
    color: var(--navy-deep);
    line-height: 1.5;
    word-break: break-word;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .snippet-hover-actions {
    position: absolute;
    top: 16px;
    right: 16px;
    display: flex;
    gap: 6px;
    opacity: 0;
    transform: translateX(10px);
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .snippet-card:hover .snippet-hover-actions {
    opacity: 1;
    transform: translateX(0);
  }

  .action-btn {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    border: none;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 16px;
    transition: all 0.2s;
    background: var(--bg-card);
    box-shadow: 0 2px 8px rgba(0,0,0,0.1);
  }

  .action-btn.edit {
    color: var(--text-secondary);
  }
  .action-btn.edit:hover {
    color: var(--primary);
    background: var(--primary-alpha);
  }

  .action-btn.delete {
    color: var(--text-secondary);
  }
  .action-btn.delete:hover {
    color: var(--error);
    background: rgba(239, 68, 68, 0.1);
  }

  /* States */
  .state-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 64px 20px;
    background: var(--bg-card);
    border-radius: 20px;
    border: 1px dashed var(--border-visible);
    color: var(--text-muted);
    gap: 16px;
  }

  .spin-icon {
    font-size: 32px;
    animation: spin 1s linear infinite;
    color: var(--primary);
  }

  @keyframes spin {
    100% { transform: rotate(360deg); }
  }

  .empty-state {
    text-align: center;
  }

  .empty-icon-wrapper {
    width: 64px;
    height: 64px;
    background: var(--primary-alpha);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 8px;
  }

  .empty-icon {
    font-size: 32px;
    color: var(--primary);
  }

  .empty-state h3 {
    font-size: 20px;
    font-weight: 700;
    color: var(--navy-deep);
    margin: 0;
  }

  .empty-state p {
    font-size: 15px;
    max-width: 300px;
    margin: 0;
  }

  @media (max-width: 768px) {
    .subtitle {
      padding-left: 0;
    }

    .snippet-hover-actions {
      opacity: 1;
      transform: none;
      position: static;
      justify-content: flex-end;
      margin-top: 12px;
      padding-top: 12px;
      border-top: 1px solid var(--border-subtle);
    }

    .action-btn {
      box-shadow: none;
      background: var(--bg-input);
    }
  }
</style>
