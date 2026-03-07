//! Unified SQLite + sqlite-vec database for entries (history, notes, tasks, reminders) and logs.
//! Data lives at ~/.kalam/data.db. The vec extension is registered before the first connection.

use once_cell::sync::Lazy;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::config::get_kalam_dir;

const DATA_DB: &str = "data.db";
const VEC_EMBEDDING_DIM: u32 = 384;

static VEC_EXTENSION_REGISTERED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

fn ensure_vec_extension_registered() -> anyhow::Result<()> {
    let mut guard = VEC_EXTENSION_REGISTERED.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
    if *guard {
        return Ok(());
    }
    unsafe {
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    }
    *guard = true;
    Ok(())
}

pub fn get_data_db_path() -> anyhow::Result<PathBuf> {
    Ok(get_kalam_dir()?.join(DATA_DB))
}

/// Open the main app database (data.db), ensuring the vec extension is loaded and migrations have run.
pub fn open_db() -> anyhow::Result<Connection> {
    ensure_vec_extension_registered()?;
    let path = get_data_db_path()?;
    let conn = Connection::open(&path)?;
    run_migrations(&conn)?;
    Ok(conn)
}

fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        r#"
        -- Main unified table for all entry types
        CREATE TABLE IF NOT EXISTS entries (
            id TEXT PRIMARY KEY,
            entry_type TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            sync_status TEXT DEFAULT 'pending',
            title TEXT,
            content TEXT NOT NULL,
            attachments TEXT,
            tags TEXT,
            color TEXT,
            is_pinned INTEGER DEFAULT 0,
            priority INTEGER,
            due_date TEXT,
            subtasks TEXT,
            is_completed INTEGER DEFAULT 0,
            reminder_at TEXT,
            rrule TEXT
        );

        -- sqlite-vec virtual table for semantic search (embedding dimension 384)
        CREATE VIRTUAL TABLE IF NOT EXISTS vec_entries USING vec0(
            entry_id TEXT PRIMARY KEY,
            embedding float[384]
        );

        -- App logs table
        CREATE TABLE IF NOT EXISTS logs (
            id TEXT PRIMARY KEY,
            level TEXT NOT NULL,
            message TEXT NOT NULL,
            module TEXT NOT NULL,
            timestamp TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

/// Embedding dimension used by vec_entries. Must match the virtual table definition.
pub fn embedding_dimension() -> u32 {
    VEC_EMBEDDING_DIM
}

/// Insert one row into the entries table. Caller must open_db() and pass the connection.
pub fn insert_entry(conn: &Connection, e: &crate::models::Entry) -> anyhow::Result<()> {
    let attachments = serde_json::to_string(&e.attachments).unwrap_or_else(|_| "[]".to_string());
    let tags = serde_json::to_string(&e.tags).unwrap_or_else(|_| "[]".to_string());
    let subtasks = e
        .subtasks
        .as_ref()
        .and_then(|s| serde_json::to_string(s).ok());
    let created_at = e.created_at.to_rfc3339();
    let updated_at = e.updated_at.to_rfc3339();
    let due_date = e.due_date.as_ref().map(|d| d.to_rfc3339());
    let reminder_at = e.reminder_at.as_ref().map(|r| r.to_rfc3339());
    conn.execute(
        r#"
        INSERT INTO entries (id, entry_type, created_at, updated_at, sync_status, title, content,
            attachments, tags, color, is_pinned, priority, due_date, subtasks, is_completed, reminder_at, rrule)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
        "#,
        rusqlite::params![
            e.id,
            e.entry_type,
            created_at,
            updated_at,
            e.sync_status,
            e.title,
            e.content,
            attachments,
            tags,
            e.color,
            e.is_pinned as i32,
            e.priority.map(|p| p as i32),
            due_date,
            subtasks,
            e.is_completed.map(|c| c as i32),
            reminder_at,
            e.rrule,
        ],
    )?;
    Ok(())
}

/// Insert a placeholder embedding (zeros) for an entry so vec_entries has a row. Used until real embedding pipeline exists.
pub fn insert_embedding_stub(conn: &Connection, entry_id: &str) -> anyhow::Result<()> {
    let dim = embedding_dimension() as usize;
    let zeros: Vec<f32> = (0..dim).map(|_| 0.0_f32).collect();
    let json_array = serde_json::to_string(&zeros).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    conn.execute(
        "INSERT OR REPLACE INTO vec_entries (entry_id, embedding) VALUES (?1, vec_f32(?2))",
        rusqlite::params![entry_id, json_array],
    )?;
    Ok(())
}

/// Read a single row from entries into an Entry. Returns None if not found.
fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<crate::models::Entry> {
    use chrono::DateTime;
    let id: String = row.get(0)?;
    let entry_type: String = row.get(1)?;
    let created_at: String = row.get(2)?;
    let updated_at: String = row.get(3)?;
    let sync_status: String = row.get(4)?;
    let title: Option<String> = row.get(5)?;
    let content: String = row.get(6)?;
    let attachments: String = row.get(7).unwrap_or_else(|_| "[]".to_string());
    let tags: String = row.get(8).unwrap_or_else(|_| "[]".to_string());
    let color: Option<String> = row.get(9)?;
    let is_pinned: i32 = row.get(10).unwrap_or(0);
    let priority: Option<i64> = row.get(11)?;
    let due_date: Option<String> = row.get(12)?;
    let subtasks: Option<String> = row.get(13)?;
    let is_completed: Option<i32> = row.get(14)?;
    let reminder_at: Option<String> = row.get(15)?;
    let rrule: Option<String> = row.get(16)?;
    let attachments: Vec<String> = serde_json::from_str(&attachments).unwrap_or_default();
    let tags: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();
    let subtasks: Option<Vec<crate::models::Subtask>> =
        subtasks.and_then(|s| serde_json::from_str(&s).ok());
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    let updated_at = DateTime::parse_from_rfc3339(&updated_at)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    let due_date = due_date
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&chrono::Utc)));
    let reminder_at = reminder_at
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&chrono::Utc)));
    Ok(crate::models::Entry {
        id,
        entry_type,
        created_at,
        updated_at,
        sync_status,
        title,
        content,
        attachments,
        tags,
        color,
        is_pinned: is_pinned != 0,
        priority: priority.map(|p| p as u8),
        due_date,
        subtasks,
        is_completed: is_completed.map(|c| c != 0),
        reminder_at,
        rrule,
    })
}

/// Get entries by type, ordered by updated_at DESC.
pub fn get_entries_by_type(
    conn: &Connection,
    entry_type: &str,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<crate::models::Entry>> {
    let mut stmt = conn.prepare(
        "SELECT id, entry_type, created_at, updated_at, sync_status, title, content, attachments, tags,
         color, is_pinned, priority, due_date, subtasks, is_completed, reminder_at, rrule
         FROM entries WHERE entry_type = ?1 ORDER BY updated_at DESC LIMIT ?2 OFFSET ?3",
    )?;
    let rows = stmt.query_map(rusqlite::params![entry_type, limit, offset], |row| row_to_entry(row))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Get a single entry by id.
pub fn get_entry(conn: &Connection, id: &str) -> anyhow::Result<Option<crate::models::Entry>> {
    let mut stmt = conn.prepare(
        "SELECT id, entry_type, created_at, updated_at, sync_status, title, content, attachments, tags,
         color, is_pinned, priority, due_date, subtasks, is_completed, reminder_at, rrule
         FROM entries WHERE id = ?1",
    )?;
    let mut rows = stmt.query(rusqlite::params![id])?;
    Ok(rows.next()?.map(|row| row_to_entry(&row)).transpose()?)
}

/// Update an existing entry. Returns true if a row was updated.
pub fn update_entry(conn: &Connection, e: &crate::models::Entry) -> anyhow::Result<bool> {
    let attachments = serde_json::to_string(&e.attachments).unwrap_or_else(|_| "[]".to_string());
    let tags = serde_json::to_string(&e.tags).unwrap_or_else(|_| "[]".to_string());
    let subtasks = e.subtasks.as_ref().and_then(|s| serde_json::to_string(s).ok());
    let created_at = e.created_at.to_rfc3339();
    let updated_at = e.updated_at.to_rfc3339();
    let due_date = e.due_date.as_ref().map(|d| d.to_rfc3339());
    let reminder_at = e.reminder_at.as_ref().map(|r| r.to_rfc3339());
    let n = conn.execute(
        r#"
        UPDATE entries SET entry_type = ?1, created_at = ?2, updated_at = ?3, sync_status = ?4,
            title = ?5, content = ?6, attachments = ?7, tags = ?8, color = ?9, is_pinned = ?10,
            priority = ?11, due_date = ?12, subtasks = ?13, is_completed = ?14, reminder_at = ?15, rrule = ?16
        WHERE id = ?17
        "#,
        rusqlite::params![
            e.entry_type,
            created_at,
            updated_at,
            e.sync_status,
            e.title,
            e.content,
            attachments,
            tags,
            e.color,
            e.is_pinned as i32,
            e.priority.map(|p| p as i32),
            due_date,
            subtasks,
            e.is_completed.map(|c| c as i32),
            reminder_at,
            e.rrule,
            e.id,
        ],
    )?;
    Ok(n > 0)
}

/// Delete an entry and its embedding.
pub fn delete_entry(conn: &Connection, id: &str) -> anyhow::Result<bool> {
    let n = conn.execute("DELETE FROM entries WHERE id = ?1", rusqlite::params![id])?;
    let _ = conn.execute("DELETE FROM vec_entries WHERE entry_id = ?1", rusqlite::params![id]);
    Ok(n > 0)
}

/// Semantic search: return entry IDs ordered by similarity to the query embedding. For now uses stub embeddings.
pub fn search_similar(conn: &Connection, _query_embedding: &[f32], limit: i64) -> anyhow::Result<Vec<String>> {
    if _query_embedding.is_empty() {
        let mut stmt = conn.prepare("SELECT entry_id FROM vec_entries LIMIT ?1")?;
        let rows = stmt.query_map(rusqlite::params![limit], |row| row.get::<_, String>(0))?;
        return Ok(rows.collect::<Result<Vec<_>, _>>()?);
    }
    let json_array = serde_json::to_string(_query_embedding).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let mut stmt = conn.prepare(
        "SELECT entry_id FROM vec_entries WHERE embedding MATCH vec_f32(?1) ORDER BY distance LIMIT ?2",
    )?;
    let rows = stmt.query_map(rusqlite::params![json_array, limit], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// One-time migration: if DB has no snippet entries and config had snippets, insert them and return true.
pub fn migrate_snippets_from_config(snippets: &[crate::config::Snippet]) -> anyhow::Result<bool> {
    if snippets.is_empty() {
        return Ok(false);
    }
    let conn = open_db()?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entries WHERE entry_type = 'snippet'",
        [],
        |row| row.get(0),
    )?;
    if count > 0 {
        return Ok(false);
    }
    let now = chrono::Utc::now();
    for s in snippets {
        let id = uuid::Uuid::new_v4().to_string();
        let entry = crate::models::Entry {
            id: id.clone(),
            entry_type: "snippet".to_string(),
            created_at: now,
            updated_at: now,
            sync_status: "pending".to_string(),
            title: Some(s.trigger.clone()),
            content: s.expansion.clone(),
            attachments: vec![],
            tags: vec![],
            color: None,
            is_pinned: false,
            priority: None,
            due_date: None,
            subtasks: None,
            is_completed: None,
            reminder_at: None,
            rrule: None,
        };
        insert_entry(&conn, &entry)?;
        insert_embedding_stub(&conn, &id)?;
    }
    log::info!("Migrated {} snippets from config to DB", snippets.len());
    Ok(true)
}
