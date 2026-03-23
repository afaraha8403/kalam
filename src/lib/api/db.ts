import { invoke } from '$lib/backend'
import type { Entry, Subtask } from '../../types'

/** Unify snake_case and camelCase from IPC/JSON so history detail stats stay populated. */
function normalizeUnifiedEntry(raw: Entry | null): Entry | null {
  if (!raw || typeof raw !== 'object') return raw
  const o = raw as Record<string, unknown>
  if (o.stt_latency_ms == null && typeof o.sttLatencyMs === 'number') o.stt_latency_ms = o.sttLatencyMs
  if (o.word_count == null && typeof o.wordCount === 'number') o.word_count = o.wordCount
  if (o.stt_mode == null && typeof o.sttMode === 'string') o.stt_mode = o.sttMode
  if (o.stt_provider == null && typeof o.sttProvider === 'string') o.stt_provider = o.sttProvider
  if (o.duration_ms == null && typeof o.durationMs === 'number') o.duration_ms = o.durationMs
  return raw
}

export async function createEntry(entry: Entry): Promise<void> {
  await invoke('create_entry', { entry })
}

export async function getEntriesByType(
  entryType: 'history' | 'note' | 'task' | 'reminder',
  limit?: number,
  offset?: number,
  scope?: 'active' | 'archived' | 'trash'
): Promise<Entry[]> {
  return invoke<Entry[]>('get_entries_by_type', {
    args: {
      entryType,
      scope: scope ?? undefined,
      limit: limit ?? 100,
      offset: offset ?? 0
    }
  })
}

/** Get notes in a scope (active, archived, or trash). Uses get_entries_by_type with scope. */
export async function getNotes(
  scope: 'active' | 'archived' | 'trash',
  limit?: number,
  offset?: number
): Promise<Entry[]> {
  return getEntriesByType('note', limit ?? 100, offset ?? 0, scope)
}

/** Search notes with optional query and label filter. */
export async function searchNotes(params: {
  query?: string
  label?: string
  scope: 'active' | 'archived' | 'trash'
  limit?: number
  offset?: number
}): Promise<Entry[]> {
  return invoke<Entry[]>('search_notes', {
    args: {
      query: params.query ?? undefined,
      label: params.label ?? undefined,
      scope: params.scope,
      limit: params.limit ?? 100,
      offset: params.offset ?? 0
    }
  })
}

/** Entries that have a reminder: reminders + notes with reminder_at (not trashed). For Reminders view. */
export async function getEntriesWithReminder(limit?: number, offset?: number): Promise<Entry[]> {
  return invoke<Entry[]>('get_entries_with_reminder', {
    args: {
      limit: limit ?? 200,
      offset: offset ?? 0
    }
  })
}

/** Distinct label (tag) strings from notes. Optional scope: active (default), archived, trash, or all. */
export async function getNoteLabels(scope?: 'active' | 'archived' | 'trash' | 'all'): Promise<string[]> {
  return invoke<string[]>('get_note_labels', { scope: scope ?? undefined })
}

/** Permanently delete all trashed notes. Returns count deleted. */
export async function emptyTrash(): Promise<number> {
  return invoke<number>('empty_trash')
}

/** Hard delete one entry (e.g. "Delete permanently" from Trash). For "move to trash" use updateEntry with deleted_at set. */
export function permanentlyDeleteEntry(id: string): Promise<boolean> {
  return deleteEntry(id)
}

export async function getEntry(id: string): Promise<Entry | null> {
  const e = await invoke<Entry | null>('get_entry', { id })
  return normalizeUnifiedEntry(e)
}

export async function updateEntry(entry: Entry): Promise<boolean> {
  return invoke<boolean>('update_entry', { entry })
}

export async function deleteEntry(id: string): Promise<boolean> {
  return invoke<boolean>('delete_entry', { id })
}

export async function saveAttachment(bytes: number[], filename: string): Promise<string> {
  return invoke<string>('save_attachment', { bytes, filename })
}

export async function searchSimilar(queryEmbedding: number[], limit?: number): Promise<string[]> {
  return invoke<string[]>('search_similar', {
    queryEmbedding,
    limit: limit ?? 10
  })
}

export async function exportLogsCsv(): Promise<{ csv: string; filename: string }> {
  const [csv, filename] = await invoke<[string, string]>('export_logs_csv')
  return { csv, filename }
}

/** Build a new Entry for create/update. Caller sets type and content. */
export function newEntry(
  entryType: Entry['entry_type'],
  content: string,
  opts: Partial<{
    title: string | null
    attachments: string[]
    tags: string[]
    color: string | null
    is_pinned: boolean
    priority: number | null
    due_date: string | null
    subtasks: Subtask[] | null
    is_completed: boolean | null
    reminder_at: string | null
    rrule: string | null
    archived_at: string | null
    deleted_at: string | null
    target_app?: string | null
    duration_ms?: number | null
    word_count?: number | null
    stt_latency_ms?: number | null
    stt_mode?: string | null
    dictation_language?: string | null
    session_mode?: string | null
    stt_provider?: string | null
  }> = {}
): Entry {
  const now = new Date().toISOString()
  return {
    id: crypto.randomUUID(),
    entry_type: entryType,
    created_at: now,
    updated_at: now,
    sync_status: 'pending',
    title: opts.title ?? null,
    content,
    attachments: opts.attachments ?? [],
    tags: opts.tags ?? [],
    color: opts.color ?? null,
    is_pinned: opts.is_pinned ?? false,
    priority: opts.priority ?? null,
    due_date: opts.due_date ?? null,
    subtasks: opts.subtasks ?? null,
    is_completed: opts.is_completed ?? null,
    reminder_at: opts.reminder_at ?? null,
    rrule: opts.rrule ?? null,
    archived_at: opts.archived_at ?? null,
    deleted_at: opts.deleted_at ?? null,
    target_app: opts.target_app ?? undefined,
    duration_ms: opts.duration_ms ?? undefined,
    word_count: opts.word_count ?? undefined,
    stt_latency_ms: opts.stt_latency_ms ?? undefined,
    stt_mode: opts.stt_mode ?? undefined,
    dictation_language: opts.dictation_language ?? undefined,
    session_mode: opts.session_mode ?? undefined,
    stt_provider: opts.stt_provider ?? undefined
  }
}
