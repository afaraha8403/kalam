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
    animation: fadeIn 0.4s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(10px); }
    to { opacity: 1; transform: translateY(0); }
  }

  header {
    margin-bottom: 32px;
  }

  h2 {
    font-size: 32px;
    font-weight: 700;
    margin-bottom: 8px;
  }

  .subtitle {
    color: var(--text-muted);
    font-size: 15px;
  }

  .add-form {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 32px;
    margin-bottom: 32px;
    box-shadow: var(--shadow-sm);
    transition: box-shadow 0.3s ease;
  }
  
  .add-form:hover {
    box-shadow: var(--shadow-md);
  }

  .add-form h3 {
    font-size: 18px;
    font-weight: 700;
    margin-bottom: 24px;
    color: var(--navy-deep);
    border-bottom: 1px solid var(--border-subtle);
    padding-bottom: 16px;
  }

  .form-row {
    display: flex;
    gap: 20px;
    margin-bottom: 24px;
  }

  .form-group {
    flex: 0 0 220px;
  }

  .form-group.flex-1 {
    flex: 1;
  }

  label {
    display: block;
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 10px;
    color: var(--navy-deep);
  }

  input {
    width: 100%;
    padding: 14px 16px;
    background: var(--bg-input);
    border: 2px solid transparent;
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font-size: 15px;
    font-weight: 500;
    transition: all 0.2s ease;
    box-shadow: var(--shadow-inner);
  }

  input:focus {
    outline: none;
    background: var(--bg-card);
    border-color: var(--primary);
    box-shadow: 0 4px 12px var(--primary-alpha);
  }

  input:hover {
    background: var(--bg-input-hover);
  }

  .form-actions {
    display: flex;
    gap: 12px;
  }

  .btn-primary {
    padding: 12px 24px;
    background: var(--primary);
    border: none;
    border-radius: var(--radius-md);
    color: var(--white);
    font-weight: 600;
    font-size: 14px;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 4px 12px var(--primary-alpha);
  }
  
  .btn-primary:hover {
    background: var(--primary-dark);
    transform: translateY(-1px);
    box-shadow: 0 6px 16px var(--primary-alpha);
  }

  .btn-secondary {
    padding: 12px 20px;
    background: var(--bg-card);
    border: 1px solid var(--border-visible);
    border-radius: var(--radius-md);
    color: var(--navy-deep);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: var(--shadow-sm);
  }
  
  .btn-secondary:hover {
    background: var(--bg-input);
    border-color: var(--navy-deep);
  }

  .loading,
  .empty {
    text-align: center;
    padding: 80px 20px;
    color: var(--text-muted);
    background: var(--bg-card);
    border-radius: var(--radius-lg);
    border: 1px dashed var(--border);
  }

  .empty p {
    font-size: 16px;
    font-weight: 500;
  }

  .empty .hint {
    margin-top: 12px;
    color: var(--primary-dark);
    font-size: 14px;
  }

  .snippet-list {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .snippet-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px 24px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }
  
  .snippet-item:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow-md);
    border-color: var(--border-visible);
  }

  .snippet-content {
    display: flex;
    align-items: center;
    gap: 16px;
    flex: 1;
  }

  .trigger {
    background: var(--primary-alpha-light);
    color: var(--primary-dark);
    border: 1px solid var(--primary-alpha);
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    font-family: 'DM Sans', monospace;
    font-size: 14px;
    font-weight: 600;
  }

  .arrow {
    color: var(--border-visible);
    font-size: 18px;
  }

  .expansion {
    color: var(--navy-deep);
    font-size: 15px;
    font-weight: 500;
  }

  .snippet-actions {
    display: flex;
    gap: 8px;
  }

  .snippet-actions button {
    padding: 8px 16px;
    background: var(--bg-input);
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .snippet-actions button:hover {
    background: var(--bg-card);
    border-color: var(--border-visible);
    color: var(--navy-deep);
    box-shadow: var(--shadow-sm);
  }

  @media (max-width: 768px) {
    .form-row {
      flex-direction: column;
      gap: 16px;
    }

    .form-group {
      flex: 1;
    }

    .form-actions {
      flex-direction: column;
    }

    .form-actions button {
      width: 100%;
    }

    .snippet-item {
      flex-direction: column;
      align-items: flex-start;
      gap: 16px;
      padding: 16px;
    }

    .snippet-content {
      flex-direction: column;
      align-items: flex-start;
      gap: 8px;
    }

    .arrow {
      display: none;
    }

    .snippet-actions {
      width: 100%;
      justify-content: flex-end;
    }
  }
</style>
