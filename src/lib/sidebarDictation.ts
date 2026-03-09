import { writable } from 'svelte/store'

export interface SidebarDictationState {
  hotkey: string | null
  toggleDictationHotkey: string | null
  languageToggleHotkey: string | null
  languages: string[]
  platform: string
}

function createStore() {
  const { subscribe, set, update } = writable<SidebarDictationState | null>(null)
  return {
    subscribe,
    set,
    updateFromConfig: (
      config: { hotkey: string | null; toggle_dictation_hotkey: string | null; language_toggle_hotkey: string | null; languages: string[] },
      platform: string
    ) => {
      set({
        hotkey: config.hotkey,
        toggleDictationHotkey: config.toggle_dictation_hotkey,
        languageToggleHotkey: config.language_toggle_hotkey,
        languages: config.languages ?? [],
        platform,
      })
    },
  }
}

export const sidebarDictationStore = createStore()

/** Display hotkey for UI (e.g. Ctrl+Super → Ctrl+Win on Windows). */
export function displayHotkey(hotkey: string, platform: string): string {
  return platform === 'windows' && hotkey === 'Ctrl+Super' ? 'Ctrl+Win' : hotkey
}
