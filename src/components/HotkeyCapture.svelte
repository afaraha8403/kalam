<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { invoke } from '$lib/backend'
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
  let keysPressed: Set<string> = new Set() // Track all keys that were pressed
  let containerElement: HTMLDivElement
  let fetchedOs = ''

  // Map of key codes to display names (Meta/OS use superKeyLabel at runtime)
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
    e.stopPropagation()
    value = ''
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

    // If it's a regular key (not a modifier), auto-confirm after short delay
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
    
    // Remove the released key from current keys
    currentKeys = new Set([...currentKeys].filter(k => k !== displayName))
    
    // If all keys are released and we captured some keys, confirm
    if (currentKeys.size === 0 && keysPressed.size > 0) {
      // Small delay to ensure we captured everything
      setTimeout(() => {
        if (isCapturing) {
          confirmHotkey()
        }
      }, 50)
    }
  }

  // Handle clicks outside to cancel
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

<div class="hotkey-capture" bind:this={containerElement}>
  <div
    class="capture-area"
    class:capturing={isCapturing}
    role="button"
    tabindex={isCapturing ? 0 : -1}
    on:click={!isCapturing ? startCapture : null}
    on:keydown={handleCaptureAreaKeydown}
    aria-label={value ? `Hotkey: ${displayValue}. Click to change.` : 'Click to set hotkey'}
  >
    {#if isCapturing}
      {#if keysPressed.size === 0}
        <span class="placeholder">Press keys...</span>
      {:else}
        <div class="keys-container">
          {#each sortKeys(Array.from(keysPressed)) as key, index}
            <span class="key-pill" class:modifier={isModifier(key)}>{key}</span>
            {#if index < keysPressed.size - 1}
              <span class="plus">+</span>
            {/if}
          {/each}
        </div>
      {/if}
      <button class="cancel-btn" on:click|stopPropagation={stopCapture} title="Cancel">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="18" y1="6" x2="6" y2="18"></line>
          <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
      </button>
    {:else}
      {#if value}
        <div class="keys-container">
          {#each displayValue.split('+') as key, index}
            <span class="key-pill" class:modifier={isModifier(key)}>{key}</span>
            {#if index < displayValue.split('+').length - 1}
              <span class="plus">+</span>
            {/if}
          {/each}
        </div>
        <button class="clear-btn" on:click={clearHotkey} title="Clear">
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      {:else}
        <span class="placeholder">Click to set hotkey</span>
        <button class="edit-btn">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path>
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path>
          </svg>
        </button>
      {/if}
    {/if}
  </div>
  {#if isCapturing}
    <span class="hint">Press your key combination, then release to confirm</span>
  {/if}
</div>

<style>
  .hotkey-capture {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .capture-area {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 48px;
    padding: 10px 14px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.2s ease;
    position: relative;
  }

  .capture-area:hover {
    border-color: var(--border-visible);
    background: var(--bg-input);
  }

  .capture-area.capturing {
    border-color: var(--primary);
    background: var(--white);
    box-shadow: 0 0 0 3px var(--primary-alpha);
    cursor: default;
    animation: none;
  }

  @keyframes pulse {
    0%, 100% {
      box-shadow: 0 0 0 0 rgba(79, 193, 255, 0.3);
    }
    50% {
      box-shadow: 0 0 0 4px rgba(79, 193, 255, 0.1);
    }
  }

  .placeholder {
    color: var(--text-muted);
    font-size: 14px;
    flex: 1;
  }

  .keys-container {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    flex-wrap: wrap;
  }

  .key-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 6px 12px;
    background: var(--bg-content);
    border: 1px solid var(--border-visible);
    border-radius: 6px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    box-shadow: 0 2px 0 var(--border), 0 1px 2px rgba(0, 0, 0, 0.05);
    min-width: 36px;
    height: 32px;
    transition: all 0.15s ease;
  }

  .key-pill.modifier {
    background: var(--primary-alpha);
    border-color: var(--primary);
    color: var(--primary-dark);
    box-shadow: none;
  }

  .plus {
    color: var(--text-muted);
    font-size: 14px;
    font-weight: 500;
    margin: 0 2px;
  }

  .clear-btn,
  .edit-btn,
  .cancel-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s;
    margin-left: 8px;
  }

  .clear-btn:hover {
    background: rgba(244, 67, 54, 0.15);
    color: var(--error);
  }

  .edit-btn:hover,
  .cancel-btn:hover {
    background: rgba(79, 193, 255, 0.15);
    color: var(--primary);
  }

  .hint {
    font-size: 12px;
    color: var(--primary);
    margin-top: 2px;
  }
</style>
