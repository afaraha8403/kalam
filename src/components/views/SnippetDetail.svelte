<script lang="ts">
  import { invoke } from '$lib/backend'
  import Icon from '@iconify/svelte'
  import { onMount } from 'svelte'
  import { selectedSnippetTrigger } from '../../lib/snippetDetailStore'
  import type { Snippet } from '../../types'

  export let navigate: (page: string) => void = () => {}

  let existingTrigger: string | null = null
  /**
   * Top-level fields (not `draft.x`) so `bind:value` reliably invalidates Save disabled state in Svelte 4.
   */
  let trigger = ''
  let expansion = ''
  let loading = true
  let saving = false
  let saveError: string | null = null

  onMount(() => {
    return selectedSnippetTrigger.subscribe((t) => {
      existingTrigger = t
      loadDraft(t)
    })
  })

  async function loadDraft(t: string | null) {
    saveError = null
    loading = true
    if (t) {
      try {
        const list = await invoke<Snippet[]>('get_snippets')
        const found = list.find((s: Snippet) => s.trigger.toLowerCase() === t.toLowerCase())
        if (found) {
          trigger = found.trigger
          expansion = found.expansion
        } else {
          trigger = ''
          expansion = ''
        }
      } catch {
        trigger = ''
        expansion = ''
      }
    } else {
      trigger = ''
      expansion = ''
    }
    loading = false
  }

  function back() {
    selectedSnippetTrigger.set(null)
    navigate('snippets')
  }

  async function save() {
    saveError = null
    const tr = trigger.trim()
    const ex = expansion.trim()
    if (!tr || !ex) {
      saveError = 'Trigger and expansion are both required.'
      return
    }
    saving = true
    try {
      if (existingTrigger && existingTrigger !== tr) {
        await invoke('remove_snippet', { trigger: existingTrigger })
      }
      await invoke('add_snippet', { trigger: tr, expansion: ex })
      back()
    } catch (e) {
      console.error(e)
      saveError = e instanceof Error ? e.message : 'Save failed.'
    } finally {
      saving = false
    }
  }

  async function deleteSnippet() {
    if (!existingTrigger) return
    if (!confirm('Delete this snippet?')) return
    saveError = null
    try {
      await invoke('remove_snippet', { trigger: existingTrigger })
      back()
    } catch (e) {
      console.error(e)
      saveError = e instanceof Error ? e.message : 'Delete failed.'
    }
  }
</script>

{#if loading}
  <div class="page fade-in state-container">
    <Icon icon="ph:spinner-gap-duotone" />
    <p>Loading…</p>
  </div>
{:else}
  <div class="page fade-in sleek-editor-page">
    <header class="sleek-header">
      <button type="button" class="sleek-back" on:click={back}>
        <Icon icon="ph:caret-left" /> Snippets
      </button>
      <div class="sleek-actions">
        {#if existingTrigger}
          <button type="button" class="sleek-icon-btn danger" on:click={deleteSnippet} title="Delete">
            <Icon icon="ph:trash" />
          </button>
        {/if}
        <button type="button" class="sleek-cancel" on:click={back}>Cancel</button>
        <button
          type="button"
          class="sleek-save"
          on:click={save}
          disabled={!trigger?.trim() || !expansion?.trim() || saving}
        >
          Save
        </button>
      </div>
    </header>

    {#if saveError}
      <p class="snippet-detail-save-error" role="alert">{saveError}</p>
    {/if}

    <div class="sleek-body">
      <div class="snippet-form-row">
        <label class="form-label" for="snippet-trigger">Trigger</label>
        <input
          id="snippet-trigger"
          type="text"
          class="sleek-title snippet-trigger-field"
          bind:value={trigger}
          placeholder="e.g. @@email or my signature phrase"
        />
        <p class="snippet-field-hint">Must match exactly what Kalam transcribes (same spelling and spacing).</p>
      </div>

      <div class="snippet-form-row">
        <label class="form-label" for="snippet-expansion">Expansion</label>
        <textarea
          id="snippet-expansion"
          class="sleek-content snippet-expansion"
          bind:value={expansion}
          placeholder="Text that replaces the trigger before it is pasted into the app…"
        ></textarea>
      </div>
    </div>
  </div>
{/if}

<style>
  .snippet-trigger-field {
    width: 100%;
    box-sizing: border-box;
  }

  .snippet-field-hint {
    margin: 8px 0 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-muted);
  }

  .snippet-detail-save-error {
    margin: 0 var(--space-lg, 16px) var(--space-sm, 8px);
    padding: var(--space-sm, 8px) var(--space-md, 12px);
    border-radius: var(--radius-md, 8px);
    background: color-mix(in srgb, var(--danger, #dc2626) 12%, transparent);
    color: var(--text, inherit);
    font-size: 0.875rem;
    line-height: 1.4;
  }
</style>
