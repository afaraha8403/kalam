import { writable } from 'svelte/store'

/** Standalone reminder id to open in full-page reminder-detail. null = new reminder. */
export const selectedReminderId = writable<string | null>(null)
