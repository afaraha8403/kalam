<script lang="ts">
  import Icon from '@iconify/svelte'
  import { createEventDispatcher } from 'svelte'
  import { fade, fly } from 'svelte/transition'

  export let isOpen = false
  export let title = ''

  const dispatch = createEventDispatcher()

  function close() {
    isOpen = false
    dispatch('close')
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && isOpen) {
      close()
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if isOpen}
  <div 
    class="panel-backdrop" 
    on:click={close} 
    role="presentation" 
    transition:fade={{ duration: 200 }}
  ></div>
  <div 
    class="side-panel" 
    role="dialog" 
    aria-labelledby="panel-title"
    transition:fly={{ x: 400, duration: 300, opacity: 1 }}
  >
    <div class="panel-header">
      <h3 id="panel-title">{title}</h3>
      <button class="panel-close" on:click={close} title="Close" aria-label="Close">
        <Icon icon="ph:x-bold" />
      </button>
    </div>
    <div class="panel-body">
      <slot name="body"></slot>
    </div>
    {#if $$slots.footer}
      <div class="panel-footer">
        <slot name="footer"></slot>
      </div>
    {/if}
  </div>
{/if}

<style>
  .panel-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.3);
    z-index: 100;
    cursor: pointer;
  }

  .side-panel {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(420px, 100vw);
    background: var(--bg-card);
    border-left: 1px solid var(--border-subtle);
    z-index: 101;
    display: flex;
    flex-direction: column;
    box-shadow: -8px 0 24px rgba(0, 0, 0, 0.08);
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .panel-header h3 {
    font-size: 18px;
    font-weight: 700;
    color: var(--navy-deep);
    margin: 0;
  }

  .panel-close {
    width: 36px;
    height: 36px;
    border: none;
    background: transparent;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    color: var(--text-muted);
    transition: all 0.2s;
  }

  .panel-close:hover {
    background: var(--bg-input);
    color: var(--navy-deep);
  }

  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .panel-footer {
    padding: 16px 20px;
    border-top: 1px solid var(--border-subtle);
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    background: var(--bg-app);
  }
</style>
