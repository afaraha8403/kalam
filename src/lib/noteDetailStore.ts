import { writable } from 'svelte/store'

/** Note id to open in full-page note detail. null = new note. Set by Notes list; NoteDetail clears after save/cancel. */
export const selectedNoteId = writable<string | null>(null)
