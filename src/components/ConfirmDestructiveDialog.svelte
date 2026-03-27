<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import Icon from '@iconify/svelte'

  /** In-app confirm (native `window.confirm` is unreliable in Tauri / WebView2). */
  export let open = false
  export let title = ''
  export let description = ''
  export let confirmLabel = 'Delete'
  export let cancelLabel = 'Cancel'
  export let busy = false
  export let error: string | null = null
  /** `btn-danger-outline` for destructive; `btn-primary` for neutral confirms (e.g. move to trash). */
  export let confirmButtonClass = 'btn-danger-outline'
  /** Show trash icon on the confirm button when idle. */
  export let showTrashIcon = true
  /** Label on the confirm button while `busy` (e.g. “Clearing…”). */
  export let busyLabel = 'Please wait…'

  const dispatch = createEventDispatcher<{ confirm: void; cancel: void }>()

  function onBackdropOrCancel() {
    if (busy) return
    open = false
    dispatch('cancel')
  }

  function onDialogKeydown(e: KeyboardEvent) {
    if (!open || busy) return
    if (e.key === 'Escape') {
      e.preventDefault()
      onBackdropOrCancel()
    }
  }

  function onConfirmClick() {
    if (busy) return
    dispatch('confirm')
  }
</script>

<svelte:window on:keydown={onDialogKeydown} />

{#if open}
  <div class="d-confirm-root" aria-live="polite">
    <button
      type="button"
      class="d-confirm-backdrop"
      aria-label={cancelLabel}
      disabled={busy}
      on:click={onBackdropOrCancel}
    />
    <div
      class="d-confirm-panel"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="d-confirm-title"
      aria-describedby="d-confirm-desc"
      on:click|stopPropagation
    >
      <h2 id="d-confirm-title" class="d-confirm-title">{title}</h2>
      <p id="d-confirm-desc" class="d-confirm-desc">{description}</p>
      {#if error}
        <p class="d-confirm-error" role="alert">{error}</p>
      {/if}
      <div class="d-confirm-actions">
        <button type="button" class="btn-ghost" disabled={busy} on:click={onBackdropOrCancel}>
          {cancelLabel}
        </button>
        <button
          type="button"
          class={confirmButtonClass}
          disabled={busy}
          on:click={onConfirmClick}
        >
          {#if busy}
            <span class="d-confirm-spin" aria-hidden="true">
              <Icon icon="ph:spinner-gap-duotone" />
            </span>
            {busyLabel}
          {:else if showTrashIcon}
            <Icon icon="ph:trash" />
            {confirmLabel}
          {:else}
            {confirmLabel}
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .d-confirm-root {
    position: fixed;
    inset: 0;
    z-index: 5000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-md, 1rem);
    pointer-events: none;
  }
  .d-confirm-root > * {
    pointer-events: auto;
  }
  .d-confirm-backdrop {
    position: absolute;
    inset: 0;
    margin: 0;
    padding: 0;
    border: none;
    cursor: pointer;
    background: rgba(7, 16, 41, 0.45);
  }
  .d-confirm-backdrop:disabled {
    cursor: not-allowed;
    opacity: 0.85;
  }
  .d-confirm-panel {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 22rem;
    padding: var(--space-lg, 1.25rem);
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
  }
  .d-confirm-title {
    margin: 0 0 0.5rem;
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .d-confirm-desc {
    margin: 0 0 1rem;
    font-size: 0.9375rem;
    line-height: 1.45;
    color: var(--text-secondary);
    white-space: pre-wrap;
  }
  .d-confirm-error {
    margin: 0 0 1rem;
    font-size: 0.875rem;
    color: var(--error);
  }
  .d-confirm-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    justify-content: flex-end;
  }
  .d-confirm-spin {
    display: inline-flex;
    vertical-align: middle;
    margin-right: 0.35rem;
    animation: d-confirm-spin 0.85s linear infinite;
  }
  .d-confirm-spin :global(svg) {
    display: block;
  }
  @keyframes d-confirm-spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
