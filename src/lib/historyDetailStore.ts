import { writable } from 'svelte/store'

/** History entry id for full-page detail; null when not viewing a row. */
export const selectedHistoryId = writable<string | null>(null)
