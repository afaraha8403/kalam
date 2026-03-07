import { writable } from 'svelte/store'

/** Task id to open in the Tasks view detail panel. Set by Reminders when user clicks a task; Tasks clears after opening. */
export const selectedTaskId = writable<string | null>(null)
