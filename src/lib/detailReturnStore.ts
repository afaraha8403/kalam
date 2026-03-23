import { writable } from 'svelte/store'

/** When opening task/note detail from history, Cancel/back should return to that dictation. */
export type DetailReturnToHistory = { type: 'history-detail'; entryId: string }

export const taskDetailReturnTo = writable<DetailReturnToHistory | null>(null)
export const noteDetailReturnTo = writable<DetailReturnToHistory | null>(null)
