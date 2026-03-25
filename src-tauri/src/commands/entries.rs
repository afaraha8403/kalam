use crate::db;
use crate::models::Entry;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct GetEntriesByTypeArgs {
    #[serde(rename = "entryType")]
    pub entry_type: String,
    /// For notes and tasks: "active" | "archived" | "trash". Ignored for other types.
    pub scope: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct GetEntriesWithReminderArgs {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Ensure ~/.kalam/attachments exists; return its path.
fn attachments_dir() -> anyhow::Result<PathBuf> {
    let dir = crate::config::get_kalam_dir()?.join("attachments");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[tauri::command]
pub fn export_logs_csv() -> Result<(String, String), String> {
    crate::app_log::export_logs_csv().map_err(|e: anyhow::Error| e.to_string())
}

#[tauri::command]
pub fn create_entry(entry: Entry) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::insert_entry(&conn, &entry).map_err(|e| e.to_string())?;
    db::insert_embedding_stub(&conn, &entry.id).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_entries_by_type(args: GetEntriesByTypeArgs) -> Result<Vec<Entry>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let limit = args.limit.unwrap_or(100);
    let offset = args.offset.unwrap_or(0);
    db::get_entries_by_type(
        &conn,
        &args.entry_type,
        args.scope.as_deref(),
        limit,
        offset,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_entries_with_reminder(args: GetEntriesWithReminderArgs) -> Result<Vec<Entry>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let limit = args.limit.unwrap_or(200);
    let offset = args.offset.unwrap_or(0);
    db::get_entries_with_reminder(&conn, limit, offset).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct GetEntriesForRemindersArgs {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[tauri::command]
pub fn get_entries_for_reminders(args: GetEntriesForRemindersArgs) -> Result<Vec<Entry>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let limit = args.limit.unwrap_or(200);
    let offset = args.offset.unwrap_or(0);
    db::get_entries_for_reminders_view(&conn, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_aggregate_stats() -> Result<db::AggregateStats, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::get_aggregate_stats(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_dashboard_stats() -> Result<db::DashboardStats, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::get_dashboard_stats(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_daily_stats(date: Option<String>) -> Result<Option<db::DailyStatsRow>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::get_daily_stats(&conn, date.as_deref()).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTasksDueOnArgs {
    /// Inclusive start of the user's local calendar day (ISO 8601 UTC instant from the client).
    pub day_start: String,
    /// Exclusive end of that local day (next local midnight as ISO 8601 UTC).
    pub day_end: String,
    pub limit: Option<i64>,
}

#[tauri::command]
pub fn get_tasks_due_on(args: GetTasksDueOnArgs) -> Result<Vec<Entry>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let limit = args.limit.unwrap_or(50);
    db::get_tasks_due_on(&conn, &args.day_start, &args.day_end, limit).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRemindersDueOnArgs {
    pub day_start: String,
    pub day_end: String,
    pub limit: Option<i64>,
}

#[tauri::command]
pub fn get_reminders_due_on(args: GetRemindersDueOnArgs) -> Result<Vec<Entry>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let limit = args.limit.unwrap_or(50);
    db::get_reminders_due_on(&conn, &args.day_start, &args.day_end, limit).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct SearchNotesArgs {
    pub query: Option<String>,
    pub label: Option<String>,
    pub scope: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[tauri::command]
pub fn search_notes(args: SearchNotesArgs) -> Result<Vec<Entry>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let limit = args.limit.unwrap_or(100);
    let offset = args.offset.unwrap_or(0);
    db::search_notes(
        &conn,
        args.query.as_deref(),
        args.label.as_deref(),
        &args.scope,
        limit,
        offset,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_note_labels(scope: Option<String>) -> Result<Vec<String>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::get_note_labels(&conn, scope.as_deref()).map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteScopeCounts {
    pub active: i64,
    pub archived: i64,
    pub trash: i64,
}

#[tauri::command]
pub fn get_note_scope_counts() -> Result<NoteScopeCounts, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let (active, archived, trash) = db::count_notes_by_scope(&conn).map_err(|e| e.to_string())?;
    Ok(NoteScopeCounts {
        active,
        archived,
        trash,
    })
}

#[tauri::command]
pub fn empty_trash() -> Result<i64, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::empty_trash(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn empty_task_trash() -> Result<i64, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::empty_task_trash(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_entry(id: String) -> Result<Option<Entry>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::get_entry(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_entry(entry: Entry) -> Result<bool, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::update_entry(&conn, &entry).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_entry(id: String) -> Result<bool, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::delete_entry(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_attachment(bytes: Vec<u8>, filename: String) -> Result<String, String> {
    let dir = attachments_dir().map_err(|e| e.to_string())?;
    let sanitized = filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    let name = if sanitized.is_empty() {
        format!("{}.bin", uuid::Uuid::new_v4())
    } else {
        sanitized
    };
    let path = dir.join(&name);
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(name)
}

#[tauri::command]
pub fn search_similar(
    query_embedding: Vec<f32>,
    limit: Option<i64>,
) -> Result<Vec<String>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(10);
    db::search_similar(&conn, &query_embedding, limit).map_err(|e| e.to_string())
}

fn snippet_entry(trigger: String, expansion: String) -> Entry {
    let now = chrono::Utc::now();
    Entry {
        id: uuid::Uuid::new_v4().to_string(),
        entry_type: "snippet".to_string(),
        created_at: now,
        updated_at: now,
        sync_status: "pending".to_string(),
        title: Some(trigger),
        content: expansion,
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
        archived_at: None,
        deleted_at: None,
        target_app: None,
        target_app_name: None,
        duration_ms: None,
        word_count: None,
        stt_latency_ms: None,
        stt_mode: None,
        dictation_language: None,
        session_mode: None,
        stt_provider: None,
        note_order: 0,
    }
}

#[tauri::command]
pub fn get_snippets() -> Result<Vec<crate::config::Snippet>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let entries =
        db::get_entries_by_type(&conn, "snippet", None, 500, 0).map_err(|e| e.to_string())?;
    Ok(entries
        .into_iter()
        .map(|e| crate::config::Snippet {
            trigger: e.title.unwrap_or_default(),
            expansion: e.content,
        })
        .collect())
}

#[tauri::command]
pub fn add_snippet(trigger: String, expansion: String) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let existing =
        db::get_entries_by_type(&conn, "snippet", None, 500, 0).map_err(|e| e.to_string())?;
    for e in existing
        .iter()
        .filter(|e| e.title.as_deref() == Some(trigger.as_str()))
    {
        let _ = db::delete_entry(&conn, &e.id);
    }
    let entry = snippet_entry(trigger, expansion);
    let id = entry.id.clone();
    db::insert_entry(&conn, &entry).map_err(|e| e.to_string())?;
    db::insert_embedding_stub(&conn, &id).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn remove_snippet(trigger: String) -> Result<bool, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    let existing =
        db::get_entries_by_type(&conn, "snippet", None, 500, 0).map_err(|e| e.to_string())?;
    if let Some(e) = existing
        .into_iter()
        .find(|e| e.title.as_deref() == Some(trigger.as_str()))
    {
        Ok(db::delete_entry(&conn, &e.id).map_err(|e| e.to_string())?)
    } else {
        Ok(false)
    }
}
