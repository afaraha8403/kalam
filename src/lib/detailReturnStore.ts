import { writable } from 'svelte/store'

/** When opening task/note detail from history, Cancel/back should return to that dictation. */
export type DetailReturnToHistory = { type: 'history-detail'; entryId: string }

/** When opening from the Reminders list, back should return there instead of Notes/Tasks. */
export type DetailReturnToReminders = { type: 'reminders' }

export type DetailReturnTarget = DetailReturnToHistory | DetailReturnToReminders

export const taskDetailReturnTo = writable<DetailReturnTarget | null>(null)
export const noteDetailReturnTo = writable<DetailReturnTarget | null>(null)
