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
    color: #666;
  }

  .add-form {
    background: #252525;
    border-radius: 12px;
    padding: 24px;
    margin-bottom: 30px;
  }

  .add-form h3 {
    font-size: 16px;
    margin-bottom: 20px;
    color: #4fc1ff;
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
    color: #e0e0e0;
  }

  input {
    width: 100%;
    padding: 12px 16px;
    background: #333;
    border: 1px solid #444;
    border-radius: 8px;
    color: #e0e0e0;
    font-size: 14px;
  }

  input:focus {
    outline: none;
    border-color: #4fc1ff;
  }

  .form-actions {
    display: flex;
    gap: 12px;
  }

  .btn-primary {
    padding: 10px 20px;
    background: #4fc1ff;
    border: none;
    border-radius: 8px;
    color: #1a1a1a;
    font-weight: 500;
    cursor: pointer;
  }

  .btn-secondary {
    padding: 10px 20px;
    background: transparent;
    border: 1px solid #444;
    border-radius: 8px;
    color: #e0e0e0;
    cursor: pointer;
  }

  .loading,
  .empty {
    text-align: center;
    padding: 60px 20px;
    color: #666;
  }

  .empty .hint {
    margin-top: 8px;
    color: #999;
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
    background: #252525;
    border-radius: 8px;
  }

  .snippet-content {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
  }

  .trigger {
    background: #4fc1ff;
    color: #1a1a1a;
    padding: 4px 10px;
    border-radius: 6px;
    font-family: monospace;
    font-size: 14px;
    font-weight: 500;
  }

  .arrow {
    color: #666;
  }

  .expansion {
    color: #e0e0e0;
    font-size: 14px;
  }

  .snippet-actions {
    display: flex;
    gap: 8px;
  }

  .snippet-actions button {
    padding: 6px 12px;
    background: transparent;
    border: 1px solid #444;
    border-radius: 6px;
    color: #999;
    font-size: 12px;
    cursor: pointer;
  }

  .snippet-actions button:hover {
    border-color: #4fc1ff;
    color: #4fc1ff;
  }
</style>
