<script lang="ts">
  import { invoke } from '$lib/backend'
  import Icon from '@iconify/svelte'
  import { onMount } from 'svelte'
  import { selectedSnippetTrigger } from '../../lib/snippetDetailStore'
  import type { Snippet } from '../../types'

  export let navigate: (page: string) => void = () => {}

  let existingTrigger: string | null = null
  let draft: { trigger: string; expansion: string } = { trigger: '', expansion: '' }
  let loading = true
  let saving = false

  onMount(() => {
    return selectedSnippetTrigger.subscribe((trigger) => {
      existingTrigger = trigger
      loadDraft(trigger)
    })
  })

  async function loadDraft(trigger: string | null) {
    loading = true
    if (trigger) {
      try {
        const list = await invoke<Snippet[]>('get_snippets')
        const found = list.find((s) => s.trigger === trigger)
        if (found) {
          draft = { trigger: found.trigger, expansion: found.expansion }
        } else {
          draft = { trigger: '', expansion: '' }
        }
      } catch {
        draft = { trigger: '', expansion: '' }
      }
    } else {
      draft = { trigger: '', expansion: '' }
    }
    loading = false
  }

  function back() {
    selectedSnippetTrigger.set(null)
    navigate('snippets')
  }

  async function save() {
    const trigger = draft.trigger.trim()
    const expansion = draft.expansion.trim()
    if (!trigger || !expansion) return
    saving = true
    try {
      if (existingTrigger && existingTrigger !== trigger) {
        await invoke('remove_snippet', { trigger: existingTrigger })
      }
      await invoke('add_snippet', { trigger, expansion })
      back()
    } catch (e) {
      console.error(e)
    } finally {
      saving = false
    }
  }

  async function deleteSnippet() {
    if (!existingTrigger) return
    try {
      await invoke('remove_snippet', { trigger: existingTrigger })
      back()
    } catch (e) {
      console.error(e)
    }
  }
</script>

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
      <button type="button" class="sleek-save" on:click={save} disabled={!draft.trigger?.trim() || !draft.expansion?.trim() || saving}>Save</button>
    </div>
  </header>

  <div class="sleek-body">
    <div class="snippet-form-row">
      <label class="form-label" for="snippet-trigger">Trigger (prefix with /)</label>
      <div class="trigger-input-wrapper">
        <span class="trigger-prefix">/</span>
        <input id="snippet-trigger" type="text" class="sleek-title trigger-input" bind:value={draft.trigger} placeholder="e.g., sig" />
      </div>
    </div>

    <div class="snippet-form-row">
      <label class="form-label" for="snippet-expansion">Expansion</label>
      <textarea
        id="snippet-expansion"
        class="sleek-content snippet-expansion"
        bind:value={draft.expansion}
        placeholder="Text that will be inserted when you type the trigger..."
      ></textarea>
    </div>
  </div>
</div>
