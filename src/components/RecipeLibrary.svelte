<script lang="ts">
  /**
   * Phase 8: in-app browser for the community recipe library (Worker API via Rust IPC).
   */
  import { onDestroy } from 'svelte'
  import { invoke, isTauriRuntime } from '$lib/backend'
  import Icon from '@iconify/svelte'
  import type { RecipeLibrarySummary } from '../types'

  export let open = false
  export let onClose: () => void
  /** Called after a recipe is installed so the parent can refresh and select the mode. */
  export let onInstalled: (modeId: string) => void

  let loading = false
  let installBusy = false
  let errorMsg = ''
  let search = ''
  let category: string = 'all'
  let sort: 'newest' | 'popular' = 'newest'
  let page = 1
  let total = 0
  let recipes: RecipeLibrarySummary[] = []
  let selected: RecipeLibrarySummary | null = null

  let debounceTimer: ReturnType<typeof setTimeout> | null = null

  const CATEGORIES: { id: string; label: string }[] = [
    { id: 'all', label: 'All' },
    { id: 'work', label: 'Work' },
    { id: 'creative', label: 'Creative' },
    { id: 'developer', label: 'Developer' },
    { id: 'languages', label: 'Languages' },
    { id: 'general', label: 'General' },
  ]

  async function loadList() {
    if (!isTauriRuntime() || !open) return
    loading = true
    errorMsg = ''
    try {
      const q = search.trim() || undefined
      const cat = category === 'all' ? undefined : category
      const res = (await invoke('fetch_recipe_library', {
        query: q,
        category: cat,
        sort,
        page,
        limit: 24,
      })) as { recipes?: RecipeLibrarySummary[]; total?: number }
      recipes = res.recipes ?? []
      total = res.total ?? 0
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : String(e)
      recipes = []
    } finally {
      loading = false
    }
  }

  function scheduleLoad() {
    if (debounceTimer) clearTimeout(debounceTimer)
    debounceTimer = setTimeout(() => {
      page = 1
      void loadList()
    }, 320)
  }

  /** When the panel opens, fetch the list (deps: `open` only — keep separate from `selected` to avoid refetch on card pick). */
  $: if (open) {
    void loadList()
  }

  $: if (!open) {
    selected = null
  }

  function onSearchInput() {
    scheduleLoad()
  }

  function pickCategory(id: string) {
    category = id
    page = 1
    if (open) void loadList()
  }

  function onSortChange() {
    page = 1
    if (open) void loadList()
  }

  async function installSelected() {
    if (!selected || !isTauriRuntime() || installBusy) return
    installBusy = true
    errorMsg = ''
    try {
      const mode = (await invoke('install_recipe_from_library', {
        slug: selected.slug,
      })) as { id: string }
      onInstalled(mode.id)
      onClose()
      selected = null
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : String(e)
    } finally {
      installBusy = false
    }
  }

  function backdropClick(ev: MouseEvent) {
    if (ev.target === ev.currentTarget) onClose()
  }

  onDestroy(() => {
    if (debounceTimer) clearTimeout(debounceTimer)
  })
</script>

{#if open}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="recipe-lib-backdrop" on:click={backdropClick} role="presentation">
    <div class="recipe-lib-panel" role="dialog" aria-labelledby="recipe-lib-title">
      <header class="recipe-lib-head">
        <h2 id="recipe-lib-title">Recipe library</h2>
        <button type="button" class="recipe-lib-close" on:click={onClose} aria-label="Close">×</button>
      </header>

      <div class="recipe-lib-toolbar">
        <input
          type="search"
          class="recipe-lib-search"
          placeholder="Search recipes…"
          bind:value={search}
          on:input={onSearchInput}
        />
        <div class="recipe-lib-sort">
          <label>
            Sort
            <select bind:value={sort} on:change={onSortChange}>
              <option value="newest">Newest</option>
              <option value="popular">Popular</option>
            </select>
          </label>
        </div>
      </div>

      <div class="recipe-lib-cats">
        {#each CATEGORIES as c}
          <button
            type="button"
            class="recipe-lib-cat"
            class:active={category === c.id}
            on:click={() => pickCategory(c.id)}
          >
            {c.label}
          </button>
        {/each}
      </div>

      {#if errorMsg}
        <p class="recipe-lib-error">{errorMsg}</p>
      {/if}

      <div class="recipe-lib-body">
        <div class="recipe-lib-list">
          {#if loading}
            <p class="recipe-lib-muted">Loading…</p>
          {:else if recipes.length === 0}
            <p class="recipe-lib-muted">No recipes found.</p>
          {:else}
            {#each recipes as r}
              <button
                type="button"
                class="recipe-lib-card"
                class:sel={selected?.slug === r.slug}
                on:click={() => {
                  selected = r
                }}
              >
                {#if r.icon}<span class="recipe-lib-icon"><Icon icon={r.icon} /></span>{/if}
                <span class="recipe-lib-card-title">{r.name}</span>
                <span class="recipe-lib-meta">{r.downloads} installs · {r.category}</span>
              </button>
            {/each}
            {#if total > recipes.length}
              <p class="recipe-lib-muted recipe-lib-page-hint">Page {page} — refine search or use the website for full browse.</p>
            {/if}
          {/if}
        </div>

        <aside class="recipe-lib-detail">
          {#if selected}
            <h3>{selected.name}</h3>
            <p class="recipe-lib-desc">{selected.description || 'No description.'}</p>
            <p class="recipe-lib-muted">
              {selected.author_email ?? 'Author'} · {selected.downloads} installs
            </p>
            {#if selected.tags?.length}
              <div class="recipe-lib-tags">
                {#each selected.tags as t}
                  <span class="recipe-lib-tag">{t}</span>
                {/each}
              </div>
            {/if}
            <button
              type="button"
              class="btn primary recipe-lib-install"
              disabled={installBusy}
              on:click={() => void installSelected()}
            >
              {installBusy ? 'Installing…' : 'Install recipe'}
            </button>
          {:else}
            <p class="recipe-lib-muted">Select a recipe to preview and install.</p>
          {/if}
        </aside>
      </div>
    </div>
  </div>
{/if}

<style>
  .recipe-lib-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1200;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-lg, 16px);
  }
  .recipe-lib-panel {
    width: min(920px, 100%);
    max-height: min(88vh, 720px);
    background: var(--bg-elevated, #1a1a1c);
    color: var(--text, #eee);
    border-radius: var(--radius-lg, 12px);
    border: 1px solid var(--border-subtle, #333);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.35);
  }
  .recipe-lib-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-md, 12px) var(--space-lg, 16px);
    border-bottom: 1px solid var(--border-subtle, #333);
  }
  .recipe-lib-head h2 {
    margin: 0;
    font-size: 1.125rem;
    font-weight: 600;
  }
  .recipe-lib-close {
    border: none;
    background: transparent;
    color: inherit;
    font-size: 1.5rem;
    line-height: 1;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 6px;
  }
  .recipe-lib-close:hover {
    background: var(--bg-hover, #2a2a2e);
  }
  .recipe-lib-toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    padding: 12px 16px;
    align-items: flex-end;
  }
  .recipe-lib-search {
    flex: 1 1 200px;
    min-width: 0;
    padding: 8px 12px;
    border-radius: 8px;
    border: 1px solid var(--border-subtle, #333);
    background: var(--bg-input, #111);
    color: inherit;
  }
  .recipe-lib-sort label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.75rem;
    color: var(--text-secondary, #aaa);
  }
  .recipe-lib-sort select {
    padding: 6px 10px;
    border-radius: 8px;
    border: 1px solid var(--border-subtle, #333);
    background: var(--bg-input, #111);
    color: inherit;
  }
  .recipe-lib-cats {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 0 16px 12px;
  }
  .recipe-lib-cat {
    border: 1px solid var(--border-subtle, #333);
    background: transparent;
    color: inherit;
    padding: 6px 12px;
    border-radius: 999px;
    font-size: 0.8125rem;
    cursor: pointer;
  }
  .recipe-lib-cat.active {
    background: var(--accent-muted, rgba(45, 212, 191, 0.2));
    border-color: var(--accent, #2dd4bf);
  }
  .recipe-lib-error {
    margin: 0 16px 8px;
    color: #f87171;
    font-size: 0.875rem;
  }
  .recipe-lib-body {
    display: grid;
    grid-template-columns: 1fr 280px;
    gap: 0;
    min-height: 280px;
    flex: 1;
    overflow: hidden;
  }
  @media (max-width: 720px) {
    .recipe-lib-body {
      grid-template-columns: 1fr;
    }
  }
  .recipe-lib-list {
    overflow-y: auto;
    padding: 8px 12px 16px 16px;
    border-right: 1px solid var(--border-subtle, #333);
  }
  @media (max-width: 720px) {
    .recipe-lib-list {
      border-right: none;
      max-height: 40vh;
    }
  }
  .recipe-lib-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    text-align: left;
    width: 100%;
    padding: 10px 12px;
    margin-bottom: 8px;
    border-radius: 10px;
    border: 1px solid var(--border-subtle, #333);
    background: var(--bg-surface, #141416);
    color: inherit;
    cursor: pointer;
  }
  .recipe-lib-card.sel {
    border-color: var(--accent, #2dd4bf);
    box-shadow: 0 0 0 1px var(--accent, #2dd4bf);
  }
  .recipe-lib-icon {
    margin-bottom: 4px;
    font-size: 1.25rem;
  }
  .recipe-lib-card-title {
    font-weight: 600;
    font-size: 0.9375rem;
  }
  .recipe-lib-meta {
    font-size: 0.75rem;
    color: var(--text-secondary, #888);
    margin-top: 4px;
  }
  .recipe-lib-muted {
    color: var(--text-secondary, #888);
    font-size: 0.875rem;
    margin: 8px 0;
  }
  .recipe-lib-page-hint {
    padding-top: 8px;
  }
  .recipe-lib-detail {
    overflow-y: auto;
    padding: 12px 16px 20px;
  }
  .recipe-lib-detail h3 {
    margin: 0 0 8px;
    font-size: 1rem;
  }
  .recipe-lib-desc {
    margin: 0 0 12px;
    font-size: 0.875rem;
    line-height: 1.45;
  }
  .recipe-lib-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 16px;
  }
  .recipe-lib-tag {
    font-size: 0.75rem;
    padding: 2px 8px;
    border-radius: 6px;
    background: var(--bg-hover, #2a2a2e);
  }
  .recipe-lib-install {
    width: 100%;
    margin-top: 8px;
  }
  .btn.primary {
    padding: 10px 16px;
    border-radius: 8px;
    border: none;
    background: var(--accent, #2dd4bf);
    color: #0a0a0b;
    font-weight: 600;
    cursor: pointer;
  }
  .btn.primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
