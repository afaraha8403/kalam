<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import type { Snippet } from '../types'

  let snippets: Snippet[] = []
  let newTrigger = ''
  let newExpansion = ''
  let loading = true
  let editing: string | null = null

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

  async function addSnippet() {
    if (!newTrigger.trim() || !newExpansion.trim()) return

    try {
      await invoke('add_snippet', {
        trigger: newTrigger,
        expansion: newExpansion
      })
      await loadSnippets()
      newTrigger = ''
      newExpansion = ''
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
      editing = null
      newTrigger = ''
      newExpansion = ''
    } catch (e) {
      console.error('Failed to save edit:', e)
    }
  }

  function cancelEdit() {
    editing = null
    newTrigger = ''
    newExpansion = ''
  }
</script>

<div class="snippets">
  <header>
    <h2>Snippets</h2>
    <p class="subtitle">Create shortcuts for frequently used text</p>
  </header>

  <div class="add-form">
    <h3>{editing ? 'Edit Snippet' : 'Add New Snippet'}</h3>
    <div class="form-row">
      <div class="form-group">
        <label for="trigger-phrase">Trigger phrase</label>
        <input
          id="trigger-phrase"
          type="text"
          bind:value={newTrigger}
          placeholder="e.g., @@email"
        />
      </div>
      <div class="form-group flex-1">
        <label for="expanded-text">Expanded text</label>
        <input
          id="expanded-text"
          type="text"
          bind:value={newExpansion}
          placeholder="e.g., myemail@example.com"
        />
      </div>
    </div>
    <div class="form-actions">
      {#if editing}
        <button class="btn-primary" on:click={saveEdit}>Save Changes</button>
        <button class="btn-secondary" on:click={cancelEdit}>Cancel</button>
      {:else}
        <button class="btn-primary" on:click={addSnippet}>Add Snippet</button>
      {/if}
    </div>
  </div>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else if snippets.length === 0}
    <div class="empty">
      <p>No snippets yet.</p>
      <p class="hint">Create shortcuts for email signatures, common phrases, code templates, etc.</p>
    </div>
  {:else}
    <div class="snippet-list">
      {#each snippets as snippet}
        <div class="snippet-item">
          <div class="snippet-content">
            <span class="trigger">{snippet.trigger}</span>
            <span class="arrow">→</span>
            <span class="expansion">{snippet.expansion}</span>
          </div>
          <div class="snippet-actions">
            <button on:click={() => startEdit(snippet)}>Edit</button>
            <button on:click={() => removeSnippet(snippet.trigger)}>Delete</button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .snippets {
    max-width: 800px;
  }

  header {
    margin-bottom: 30px;
  }

  h2 {
    font-size: 28px;
    font-weight: 600;
    margin-bottom: 8px;
  }

  .subtitle {
    color: var(--text-muted);
  }

  .add-form {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 24px;
    margin-bottom: 30px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
  }

  .add-form h3 {
    font-size: 16px;
    margin-bottom: 20px;
    color: var(--navy-deep);
  }

  .form-row {
    display: flex;
    gap: 16px;
    margin-bottom: 16px;
  }

  .form-group {
    flex: 0 0 200px;
  }

  .form-group.flex-1 {
    flex: 1;
  }

  label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    margin-bottom: 8px;
    color: var(--text-primary);
  }

  input {
    width: 100%;
    padding: 12px 16px;
    background: var(--bg-input);
    border: 1px solid var(--border-visible);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 14px;
  }

  input:focus {
    outline: none;
    border-color: var(--primary);
  }

  .form-actions {
    display: flex;
    gap: 12px;
  }

  .btn-primary {
    padding: 10px 20px;
    background: var(--primary);
    border: none;
    border-radius: 8px;
    color: var(--white);
    font-weight: 500;
    cursor: pointer;
  }

  .btn-secondary {
    padding: 10px 20px;
    background: transparent;
    border: 1px solid var(--border-visible);
    border-radius: 8px;
    color: var(--text-primary);
    cursor: pointer;
  }

  .loading,
  .empty {
    text-align: center;
    padding: 60px 20px;
    color: var(--text-muted);
  }

  .empty .hint {
    margin-top: 8px;
    color: var(--text-secondary);
  }

  .snippet-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .snippet-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.02);
  }

  .snippet-content {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
  }

  .trigger {
    background: var(--primary-alpha);
    color: var(--primary-dark);
    border: 1px solid var(--primary);
    padding: 4px 10px;
    border-radius: 6px;
    font-family: monospace;
    font-size: 14px;
    font-weight: 500;
  }

  .arrow {
    color: var(--text-muted);
  }

  .expansion {
    color: var(--text-primary);
    font-size: 14px;
  }

  .snippet-actions {
    display: flex;
    gap: 8px;
  }

  .snippet-actions button {
    padding: 6px 12px;
    background: transparent;
    border: 1px solid var(--border-visible);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
  }

  .snippet-actions button:hover {
    border-color: var(--primary);
    color: var(--primary);
  }
</style>
