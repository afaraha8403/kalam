<script lang="ts">
  import { onMount } from 'svelte'
  import { getEntriesByType, createEntry, deleteEntry, newEntry } from '../../lib/api/db'
  import type { Entry } from '../../types'
  import Icon from '@iconify/svelte'

  let entries: Entry[] = []
  let loading = true
  let error: string | null = null
  let newTitle = ''
  let newContent = ''
  let isComposerExpanded = false

  async function load() {
    loading = true
    error = null
    try {
      const result = await getEntriesByType('note')
      entries = Array.isArray(result) ? result : []
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    } finally {
      loading = false
    }
  }

  async function addNote() {
    const content = newContent.trim() || newTitle.trim()
    if (!content) return
    const entry = newEntry('note', content, { title: newTitle.trim() || null })
    try {
      await createEntry(entry)
      newTitle = ''
      newContent = ''
      isComposerExpanded = false
      entries = [entry, ...entries]
      error = null
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  async function remove(id: string) {
    try {
      await deleteEntry(id)
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    }
  }

  function formatDate(iso: string) {
    try {
      const d = new Date(iso)
      const today = new Date()
      if (d.toDateString() === today.toDateString()) {
        return `Today, ${d.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })}`
      }
      return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
    } catch {
      return iso
    }
  }

  function expandComposer() {
    isComposerExpanded = true
  }

  function collapseComposer() {
    if (!newTitle.trim() && !newContent.trim()) {
      isComposerExpanded = false
    }
  }

  onMount(() => load())
</script>

<div class="view notes-view">
  <header class="page-header">
    <div class="header-content">
      <div class="title-wrapper">
        <Icon icon="ph:note-duotone" class="header-icon" />
        <h2>Notes</h2>
      </div>
      <p class="subtitle">Jot down your thoughts, ideas, and transcriptions.</p>
    </div>
  </header>

  <div class="composer-container" class:expanded={isComposerExpanded}>
    <div class="composer-card">
      {#if isComposerExpanded}
        <input 
          type="text" 
          bind:value={newTitle} 
          placeholder="Title" 
          class="title-input" 
        />
      {/if}
      <textarea 
        bind:value={newContent} 
        placeholder="Take a note..." 
        rows={isComposerExpanded ? 3 : 1}
        class="content-input"
        on:focus={expandComposer}
        on:blur={collapseComposer}
      ></textarea>
      
      {#if isComposerExpanded}
        <div class="composer-actions">
          <button type="button" class="btn-ghost" on:click={() => { newTitle=''; newContent=''; isComposerExpanded=false; }}>Close</button>
          <button type="button" class="btn-primary" on:click={addNote} disabled={!newTitle.trim() && !newContent.trim()}>
            <Icon icon="ph:plus-bold" /> Add Note
          </button>
        </div>
      {/if}
    </div>
  </div>

  {#if error}
    <div class="state-container error-state">
      <Icon icon="ph:warning-circle-duotone" class="error-icon" />
      <p>{error}</p>
    </div>
  {/if}

  {#if loading && entries.length === 0}
    <div class="state-container">
      <Icon icon="ph:spinner-gap-duotone" class="spin-icon" />
      <p>Loading notes...</p>
    </div>
  {:else if entries.length === 0}
    <div class="state-container empty-state">
      <div class="empty-icon-wrapper">
        <Icon icon="ph:notebook-duotone" class="empty-icon" />
      </div>
      <h3>No notes yet</h3>
      <p>Your captured thoughts will appear here.</p>
    </div>
  {:else}
    <div class="notes-grid">
      {#each entries as entry (entry.id)}
        <article class="note-card" class:pinned={entry.is_pinned} style:background-color={entry.color ?? 'var(--bg-card)'}>
          <div class="note-inner">
            {#if entry.title}
              <h3 class="note-title">{entry.title}</h3>
            {/if}
            <p class="note-content" class:no-title={!entry.title}>{entry.content || '(empty)'}</p>
          </div>
          <div class="note-footer">
            <span class="note-date">{formatDate(entry.updated_at)}</span>
            <div class="note-actions">
              <button type="button" class="action-btn delete" on:click={() => remove(entry.id)} title="Delete note">
                <Icon icon="ph:trash-duotone" />
              </button>
            </div>
          </div>
        </article>
      {/each}
    </div>
  {/if}
</div>

<style>
  .view {
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
    position: relative;
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

  /* Composer */
  .composer-container {
    max-width: 600px;
    margin: 0 auto;
    width: 100%;
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .composer-card {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.03), 0 1px 3px rgba(0, 0, 0, 0.02);
    overflow: hidden;
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    display: flex;
    flex-direction: column;
  }

  .composer-container.expanded .composer-card {
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.08), 0 1px 3px rgba(0, 0, 0, 0.02);
    border-color: var(--border-visible);
    transform: translateY(-2px);
  }

  .title-input {
    width: 100%;
    border: none;
    padding: 16px 20px 4px;
    font-size: 16px;
    font-weight: 700;
    color: var(--navy-deep);
    background: transparent;
    font-family: inherit;
  }

  .title-input:focus {
    outline: none;
  }

  .title-input::placeholder {
    color: var(--text-muted);
    font-weight: 600;
  }

  .content-input {
    width: 100%;
    border: none;
    padding: 16px 20px;
    font-size: 15px;
    color: var(--text-primary);
    background: transparent;
    resize: none;
    font-family: inherit;
    line-height: 1.5;
  }

  .content-input:focus {
    outline: none;
  }

  .composer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    background: var(--bg-app);
    border-top: 1px solid var(--border-subtle);
  }

  .btn-primary {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    background: var(--primary);
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--primary-dark);
    transform: translateY(-1px);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-ghost {
    padding: 8px 16px;
    background: transparent;
    color: var(--text-secondary);
    border: none;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-ghost:hover {
    background: var(--bg-input);
    color: var(--navy-deep);
  }

  /* Grid */
  .notes-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 20px;
    align-items: start;
  }

  .note-card {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    display: flex;
    flex-direction: column;
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    position: relative;
    overflow: hidden;
  }

  .note-card:hover {
    transform: translateY(-4px);
    box-shadow: 0 12px 24px rgba(0, 0, 0, 0.06);
    border-color: var(--border-visible);
  }

  .note-card.pinned::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 4px;
    background: var(--primary);
  }

  .note-inner {
    padding: 20px;
    flex: 1;
  }

  .note-title {
    font-size: 16px;
    font-weight: 700;
    color: var(--navy-deep);
    margin: 0 0 8px 0;
    line-height: 1.3;
  }

  .note-content {
    font-size: 15px;
    line-height: 1.6;
    color: var(--text-primary);
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    display: -webkit-box;
    -webkit-line-clamp: 8;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .note-content.no-title {
    font-size: 16px;
    color: var(--navy-deep);
  }

  .note-footer {
    padding: 12px 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px dashed var(--border-subtle);
    background: rgba(0,0,0,0.01);
  }

  .note-date {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .note-actions {
    display: flex;
    gap: 4px;
    opacity: 0;
    transition: opacity 0.2s;
  }

  .note-card:hover .note-actions {
    opacity: 1;
  }

  .action-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: none;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    color: var(--text-muted);
    transition: all 0.2s;
  }

  .action-btn:hover {
    background: var(--bg-input);
    color: var(--navy-deep);
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
    margin: 0;
  }

  .error-state {
    border-color: rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.02);
    padding: 24px;
    flex-direction: row;
    color: var(--error);
  }

  @media (max-width: 768px) {
    .note-actions {
      opacity: 1;
    }
    .subtitle {
      padding-left: 0;
    }
  }
</style>
