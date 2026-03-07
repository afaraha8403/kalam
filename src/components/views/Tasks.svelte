<script lang="ts">
  import { onMount } from 'svelte'
  import { fade } from 'svelte/transition'
  import { getEntriesByType, createEntry, updateEntry, deleteEntry, newEntry } from '../../lib/api/db'
  import type { Entry } from '../../types'
  import Icon from '@iconify/svelte'

  let entries: Entry[] = []
  let loading = true
  let error: string | null = null
  let newTitle = ''

  async function load() {
    loading = true
    error = null
    try {
      const result = await getEntriesByType('task')
      entries = Array.isArray(result) ? result : []
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      // Keep existing entries so UI does not break if refetch fails
    } finally {
      loading = false
    }
  }

  async function addTask() {
    const title = newTitle.trim()
    if (!title) return
    const entry = newEntry('task', title, { title })
    try {
      await createEntry(entry)
      newTitle = ''
      // Optimistic update: show new task immediately so UI does not depend on refetch
      entries = [entry, ...entries]
      error = null
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load()
    }
  }

  async function toggleComplete(entry: Entry) {
    try {
      const updated = { ...entry, is_completed: !entry.is_completed, updated_at: new Date().toISOString() }
      // Optimistic update
      entries = entries.map(e => e.id === entry.id ? updated : e)
      await updateEntry(updated)
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load() // Revert on error
    }
  }

  async function remove(id: string) {
    try {
      // Optimistic update
      entries = entries.filter(e => e.id !== id)
      await deleteEntry(id)
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
      await load() // Revert on error
    }
  }

  function formatDate(iso: string | null) {
    if (!iso) return ''
    try {
      return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
    } catch {
      return iso
    }
  }

  $: activeTasks = entries.filter(e => !e.is_completed)
  $: completedTasks = entries.filter(e => e.is_completed)

  onMount(() => load())
</script>

<div class="view tasks-view">
  <header class="page-header">
    <div class="header-content">
      <div class="title-wrapper">
        <Icon icon="ph:check-square-offset-duotone" class="header-icon" />
        <h2>Tasks</h2>
      </div>
      <p class="subtitle">Stay on top of what needs to be done.</p>
    </div>
  </header>

  <div class="add-task-container">
    <div class="input-wrapper">
      <Icon icon="ph:plus-bold" class="input-icon" />
      <input 
        type="text" 
        bind:value={newTitle} 
        placeholder="Add a new task..." 
        on:keydown={(e) => e.key === 'Enter' && addTask()}
      />
      {#if newTitle.trim()}
        <button class="add-btn" on:click={addTask} transition:fade>
          Add
        </button>
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
      <p>Loading tasks...</p>
    </div>
  {:else if entries.length === 0}
    <div class="state-container empty-state">
      <div class="empty-icon-wrapper">
        <Icon icon="ph:check-circle-duotone" class="empty-icon" />
      </div>
      <h3>All caught up!</h3>
      <p>You have no tasks. Enjoy your day or add a new one above.</p>
    </div>
  {:else}
    <div class="task-sections">
      {#if activeTasks.length > 0}
        <div class="task-list">
          {#each activeTasks as entry (entry.id)}
            <div class="task-item">
              <button class="checkbox" on:click={() => toggleComplete(entry)}>
                <div class="check-circle">
                  <Icon icon="ph:check-bold" class="check-icon" />
                </div>
              </button>
              <div class="task-content">
                <span class="task-title">{entry.title || entry.content}</span>
                {#if entry.due_date}
                  <span class="task-due">
                    <Icon icon="ph:calendar-blank-duotone" />
                    {formatDate(entry.due_date)}
                  </span>
                {/if}
              </div>
              <div class="task-actions">
                <button class="action-btn delete" on:click={() => remove(entry.id)} title="Delete task">
                  <Icon icon="ph:trash-duotone" />
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}

      {#if completedTasks.length > 0}
        <div class="completed-section">
          <div class="section-header">
            <h3>Completed</h3>
            <span class="count">{completedTasks.length}</span>
          </div>
          <div class="task-list completed">
            {#each completedTasks as entry (entry.id)}
              <div class="task-item is-completed">
                <button class="checkbox" on:click={() => toggleComplete(entry)}>
                  <div class="check-circle checked">
                    <Icon icon="ph:check-bold" class="check-icon" />
                  </div>
                </button>
                <div class="task-content">
                  <span class="task-title">{entry.title || entry.content}</span>
                </div>
                <div class="task-actions">
                  <button class="action-btn delete" on:click={() => remove(entry.id)} title="Delete task">
                    <Icon icon="ph:trash-duotone" />
                  </button>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .view {
    max-width: 800px;
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

  /* Add Task Input */
  .add-task-container {
    position: relative;
  }

  .input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    padding: 4px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.02);
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .input-wrapper:focus-within {
    border-color: var(--primary);
    box-shadow: 0 0 0 4px var(--primary-alpha), 0 8px 24px rgba(0, 0, 0, 0.04);
    transform: translateY(-2px);
  }

  .input-icon {
    position: absolute;
    left: 16px;
    font-size: 20px;
    color: var(--primary);
    pointer-events: none;
  }

  .input-wrapper input {
    width: 100%;
    padding: 16px 20px 16px 48px;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 16px;
    font-family: inherit;
  }

  .input-wrapper input:focus {
    outline: none;
  }

  .add-btn {
    position: absolute;
    right: 8px;
    background: var(--primary);
    color: white;
    border: none;
    padding: 8px 16px;
    border-radius: 10px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .add-btn:hover {
    background: var(--primary-dark);
  }

  /* Task List */
  .task-sections {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  .task-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .task-item {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    transition: all 0.2s ease;
  }

  .task-item:hover {
    border-color: var(--border-visible);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.03);
  }

  .checkbox {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .check-circle {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 2px solid var(--border-visible);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
    color: transparent;
  }

  .checkbox:hover .check-circle {
    border-color: var(--primary);
    background: var(--primary-alpha);
  }

  .check-circle.checked {
    background: var(--primary);
    border-color: var(--primary);
    color: white;
  }

  .check-icon {
    font-size: 14px;
  }

  .task-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .task-title {
    font-size: 16px;
    color: var(--navy-deep);
    font-weight: 500;
    transition: all 0.2s;
  }

  .task-due {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--text-muted);
    font-weight: 600;
  }

  .task-actions {
    opacity: 0;
    transition: opacity 0.2s;
  }

  .task-item:hover .task-actions {
    opacity: 1;
  }

  .action-btn {
    width: 32px;
    height: 32px;
    border-radius: 8px;
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

  /* Completed Section */
  .completed-section {
    opacity: 0.8;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
    padding-left: 16px;
  }

  .section-header h3 {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0;
  }

  .count {
    background: var(--bg-input);
    color: var(--text-secondary);
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 700;
  }

  .task-item.is-completed {
    background: transparent;
    border-color: transparent;
    box-shadow: none;
  }

  .task-item.is-completed .task-title {
    text-decoration: line-through;
    color: var(--text-muted);
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
    .task-actions {
      opacity: 1;
    }
    .subtitle {
      padding-left: 0;
    }
  }
</style>
