import { writable } from 'svelte/store'

/** Trigger of the snippet to open in full-page snippet-detail. null = new snippet. */
export const selectedSnippetTrigger = writable<string | null>(null)
