import { invoke } from '@tauri-apps/api/core'
import type { Entry, Subtask } from '../../types'

export async function createEntry(entry: Entry): Promise<void> {
  await invoke('create_entry', { entry })
}

export async function getEntriesByType(
  entryType: 'history' | 'note' | 'task' | 'reminder',
  limit?: number,
  offset?: number
): Promise<Entry[]> {
  return invoke<Entry[]>('get_entries_by_type', {
    args: {
      entryType,
      limit: limit ?? 100,
      offset: offset ?? 0
    }
  })
}

export async function getEntry(id: string): Promise<Entry | null> {
  return invoke<Entry | null>('get_entry', { id })
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
    rrule: opts.rrule ?? null
  }
}
