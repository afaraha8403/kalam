<script lang="ts">
  import Icon from '@iconify/svelte'

  export let searchQuery = ''
  export let placeholder = 'Search...'
</script>

<div class="search-filter-bar">
  <div class="search-wrap">
    <Icon icon="ph:magnifying-glass-duotone" class="search-icon" />
    <input 
      type="text" 
      class="search-input" 
      bind:value={searchQuery} 
      {placeholder} 
    />
  </div>
  {#if $$slots.filters}
    <div class="filter-chips">
      <slot name="filters"></slot>
    </div>
  {/if}
</div>

<style>
  .search-filter-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    margin-top: 12px;
  }

  .search-wrap {
    display: flex;
    align-items: center;
    background: var(--bg-input);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    padding: 8px 12px;
    flex: 1;
    min-width: 200px;
    transition: all 0.2s;
  }

  .search-wrap:focus-within {
    border-color: var(--primary);
    box-shadow: 0 0 0 3px var(--primary-alpha);
  }

  .search-icon {
    font-size: 18px;
    color: var(--text-muted);
    margin-right: 8px;
  }

  .search-input {
    border: none;
    background: transparent;
    font-size: 14px;
    color: var(--text-primary);
    flex: 1;
  }

  .search-input:focus {
    outline: none;
  }

  .filter-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
</style>
