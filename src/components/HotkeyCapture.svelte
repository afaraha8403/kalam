<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { invoke } from '$lib/backend'
  import Icon from '@iconify/svelte'
  import {
    formatHotkeyForDisplay,
    modifierSortIndex,
    superKeyLabel,
  } from '$lib/platformHotkey'

  export let value = ''
  export let onChange: (hotkey: string) => void
  /** From `get_platform` (windows | macos | linux). If empty, resolved once via invoke. */
  export let platform = ''

  let isCapturing = false
  let currentKeys: Set<string> = new Set()
  let keysPressed: Set<string> = new Set()
  let containerElement: HTMLDivElement
  let fetchedOs = ''

  const keyDisplayMap: Record<string, string> = {
    'Control': 'Ctrl',
    'ControlLeft': 'Ctrl',
    'ControlRight': 'Ctrl',
    'Shift': 'Shift',
    'ShiftLeft': 'Shift',
    'ShiftRight': 'Shift',
    'Alt': 'Alt',
    'AltLeft': 'Alt',
    'AltRight': 'Alt',
    'CapsLock': 'Caps',
    'Tab': 'Tab',
    'Escape': 'Esc',
    'Enter': 'Enter',
    'Space': 'Space',
    'Backspace': 'Backspace',
    'Delete': 'Delete',
    'ArrowUp': '↑',
    'ArrowDown': '↓',
    'ArrowLeft': '←',
    'ArrowRight': '→',
    'Home': 'Home',
    'End': 'End',
    'PageUp': 'PgUp',
    'PageDown': 'PgDn',
    'Insert': 'Ins',
    'F1': 'F1', 'F2': 'F2', 'F3': 'F3', 'F4': 'F4',
    'F5': 'F5', 'F6': 'F6', 'F7': 'F7', 'F8': 'F8',
    'F9': 'F9', 'F10': 'F10', 'F11': 'F11', 'F12': 'F12',
  }

  $: os = platform || fetchedOs || 'windows'

  function getDisplayName(key: string, code: string): string {
    if (
      code === 'Meta' ||
      code === 'MetaLeft' ||
      code === 'MetaRight' ||
      code === 'OS' ||
      key === 'Meta'
    ) {
      return superKeyLabel(os)
    }
    if (keyDisplayMap[code]) {
      return keyDisplayMap[code]
    }
    if (keyDisplayMap[key]) {
      return keyDisplayMap[key]
    }
    if (key.length === 1) {
      return key.toUpperCase()
    }
    return key
  }

  function isModifier(key: string): boolean {
    return modifierSortIndex(key) < 4
  }

  function sortKeys(keys: string[]): string[] {
    return [...keys].sort((a, b) => {
      const ai = modifierSortIndex(a)
      const bi = modifierSortIndex(b)
      if (ai !== bi) return ai - bi
      return a.localeCompare(b)
    })
  }

  /** Stored hotkey string with meta segments shown for this OS. */
  $: displayValue = value ? formatHotkeyForDisplay(value, os) : ''

  function startCapture(e?: MouseEvent) {
    if (e) {
      e.preventDefault()
      e.stopPropagation()
    }
    invoke('set_hotkeys_paused', { paused: true }).catch(() => {})
    isCapturing = true
    currentKeys = new Set()
    keysPressed = new Set()
  }

  function handleCaptureAreaKeydown(event: KeyboardEvent) {
    if (isCapturing) return
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault()
      startCapture()
    }
  }

  function stopCapture() {
    invoke('set_hotkeys_paused', { paused: false }).catch(() => {})
    isCapturing = false
    currentKeys = new Set()
    keysPressed = new Set()
  }

  function clearHotkey(e: MouseEvent) {
    e.preventDefault()
    e.stopPropagation()
    onChange('')
    stopCapture()
  }

  function confirmHotkey() {
    if (keysPressed.size > 0) {
      const sortedKeys = sortKeys(Array.from(keysPressed))
      const hotkeyString = sortedKeys.join('+')
      value = hotkeyString
      onChange(hotkeyString)
    }
    stopCapture()
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (!isCapturing) return

    event.preventDefault()
    event.stopPropagation()

    const displayName = getDisplayName(event.key, event.code)
    currentKeys = new Set([...currentKeys, displayName])
    keysPressed = new Set([...keysPressed, displayName])

    if (!isModifier(displayName) && !['Tab', 'Esc', 'Enter'].includes(displayName)) {
      setTimeout(() => {
        if (isCapturing) {
          confirmHotkey()
        }
      }, 200)
    }
  }

  function handleKeyUp(event: KeyboardEvent) {
    if (!isCapturing) return

    event.preventDefault()
    event.stopPropagation()

    const displayName = getDisplayName(event.key, event.code)

    currentKeys = new Set([...currentKeys].filter(k => k !== displayName))

    if (currentKeys.size === 0 && keysPressed.size > 0) {
      setTimeout(() => {
        if (isCapturing) {
          confirmHotkey()
        }
      }, 50)
    }
  }

  function handleDocumentClick(event: MouseEvent) {
    if (isCapturing && containerElement && !containerElement.contains(event.target as Node)) {
      stopCapture()
    }
  }

  onMount(() => {
    if (!platform) {
      invoke('get_platform')
        .then((p) => {
          fetchedOs = typeof p === 'string' ? p : 'windows'
        })
        .catch(() => {
          fetchedOs = 'windows'
        })
    }
    document.addEventListener('click', handleDocumentClick)
    document.addEventListener('keydown', handleKeyDown)
    document.addEventListener('keyup', handleKeyUp)
  })

  onDestroy(() => {
    document.removeEventListener('click', handleDocumentClick)
    document.removeEventListener('keydown', handleKeyDown)
    document.removeEventListener('keyup', handleKeyUp)
  })
</script>

<!-- Prototype: .hotkey-capture-area + pills (styles from App.svelte :global .kalam-sleek) -->
<div class="hotkey-capture-root" bind:this={containerElement}>
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div
    class="hotkey-capture-area"
    class:capturing={isCapturing}
    role="button"
    tabindex={isCapturing ? 0 : -1}
    on:click={() => (isCapturing ? stopCapture() : startCapture())}
    on:keydown={handleCaptureAreaKeydown}
    aria-label={value
      ? `Hotkey: ${displayValue}. Click to change.`
      : 'Click to set hotkey. Then hold two or more keys at once, then release.'}
  >
    {#if isCapturing}
      {#if keysPressed.size === 0}
        <span class="hotkey-placeholder">Hold two or more keys at once, then release</span>
      {:else}
        <div class="hotkey-pills">
          {#each sortKeys(Array.from(keysPressed)) as key}
            <span class="hotkey-pill" class:modifier={isModifier(key)}>{key}</span>
          {/each}
        </div>
      {/if}
      <button type="button" class="hotkey-cancel" on:click|stopPropagation={stopCapture} title="Cancel">
        <Icon icon="ph:x" />
      </button>
    {:else}
      {#if value}
        <div class="hotkey-pills">
          {#each displayValue.split('+') as key}
            <span class="hotkey-pill" class:modifier={isModifier(key)}>{key}</span>
          {/each}
        </div>
        <button type="button" class="hotkey-clear" on:click|stopPropagation={clearHotkey} title="Clear">
          <Icon icon="ph:x" />
        </button>
      {:else}
        <span class="hotkey-placeholder">Click, then hold two or more keys</span>
        <Icon icon="ph:pencil-simple" class="hotkey-edit-icon" aria-hidden="true" />
      {/if}
    {/if}
  </div>
</div>

<style>
  /* Layout wrapper only; visual design lives under .kalam-sleek :global rules in App.svelte */
  .hotkey-capture-root {
    width: 100%;
    max-width: 280px;
  }
</style>
