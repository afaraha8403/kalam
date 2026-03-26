//! Unified SQLite + sqlite-vec database for entries (history, notes, tasks, reminders) and logs.
//! Data lives at ~/.kalam/data.db. The vec extension is registered before the first connection.

use once_cell::sync::Lazy;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::config::get_kalam_dir;

const DATA_DB: &str = "data.db";
const VEC_EMBEDDING_DIM: u32 = 384;

/// Full row projection for `row_to_entry` (column indices 0..=28). Keep INSERT/UPDATE aligned.
const SELECT_ENTRY_ROW: &str = "SELECT id, entry_type, created_at, updated_at, sync_status, title, content, attachments, tags, \
     color, is_pinned, priority, due_date, subtasks, is_completed, reminder_at, rrule, archived_at, deleted_at, target_app, target_app_name, duration_ms, \
     word_count, stt_latency_ms, stt_mode, dictation_language, session_mode, stt_provider, note_order";

static VEC_EXTENSION_REGISTERED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

fn ensure_vec_extension_registered() -> anyhow::Result<()> {
    let mut guard = VEC_EXTENSION_REGISTERED
        .lock()
        .map_err(|e| anyhow::anyhow!("lock: {}", e))?;
    if *guard {
        return Ok(());
    }
    unsafe {
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute::<
            *const (),
            unsafe extern "C" fn(
                *mut rusqlite::ffi::sqlite3,
                *mut *const i8,
                *const rusqlite::ffi::sqlite3_api_routines,
            ) -> i32,
        >(
            sqlite_vec::sqlite3_vec_init as *const ()
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

    // Schema version 2: add archived_at, deleted_at for notes (archive/trash)
    let has_archived_at: bool = conn.query_row(
        "SELECT COUNT(1) FROM pragma_table_info('entries') WHERE name = 'archived_at'",
        [],
        |row| row.get(0),
    )?;
    if !has_archived_at {
        conn.execute("ALTER TABLE entries ADD COLUMN archived_at TEXT", [])?;
        conn.execute("ALTER TABLE entries ADD COLUMN deleted_at TEXT", [])?;
    } else {
        let has_deleted_at: bool = conn.query_row(
            "SELECT COUNT(1) FROM pragma_table_info('entries') WHERE name = 'deleted_at'",
            [],
            |row| row.get(0),
        )?;
        if !has_deleted_at {
            conn.execute("ALTER TABLE entries ADD COLUMN deleted_at TEXT", [])?;
        }
    }

    // Schema: target app + recording duration for history rows (overview dashboard).
    let has_target_app: i64 = conn.query_row(
        "SELECT COUNT(1) FROM pragma_table_info('entries') WHERE name = 'target_app'",
        [],
        |row| row.get(0),
    )?;
    if has_target_app == 0 {
        conn.execute("ALTER TABLE entries ADD COLUMN target_app TEXT", [])?;
        conn.execute("ALTER TABLE entries ADD COLUMN duration_ms INTEGER", [])?;
    }

    // History detail: word count, STT latency, effective mode, language, dictation vs command.
    let has_word_count: i64 = conn.query_row(
        "SELECT COUNT(1) FROM pragma_table_info('entries') WHERE name = 'word_count'",
        [],
        |row| row.get(0),
    )?;
    if has_word_count == 0 {
        conn.execute("ALTER TABLE entries ADD COLUMN word_count INTEGER", [])?;
        conn.execute("ALTER TABLE entries ADD COLUMN stt_latency_ms INTEGER", [])?;
        conn.execute("ALTER TABLE entries ADD COLUMN stt_mode TEXT", [])?;
        conn.execute("ALTER TABLE entries ADD COLUMN dictation_language TEXT", [])?;
        conn.execute("ALTER TABLE entries ADD COLUMN session_mode TEXT", [])?;
    }

    let has_stt_provider: i64 = conn.query_row(
        "SELECT COUNT(1) FROM pragma_table_info('entries') WHERE name = 'stt_provider'",
        [],
        |row| row.get(0),
    )?;
    if has_stt_provider == 0 {
        conn.execute("ALTER TABLE entries ADD COLUMN stt_provider TEXT", [])?;
    }

    // Friendly target-app title + `applications` cache (deduplicated icons per process name).
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _kalam_migrations (name TEXT PRIMARY KEY)",
        [],
    )?;
    let has_target_app_name: i64 = conn.query_row(
        "SELECT COUNT(1) FROM pragma_table_info('entries') WHERE name = 'target_app_name'",
        [],
        |row| row.get(0),
    )?;
    if has_target_app_name == 0 {
        conn.execute("ALTER TABLE entries ADD COLUMN target_app_name TEXT", [])?;
    }
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS applications (
            process_name TEXT PRIMARY KEY,
            display_name TEXT NOT NULL,
            icon_png BLOB,
            last_resolved_at TEXT NOT NULL
        );
        "#,
    )?;
    let backfill_done: i64 = conn.query_row(
        "SELECT COUNT(1) FROM _kalam_migrations WHERE name = 'applications_backfill_v1'",
        [],
        |row| row.get(0),
    )?;
    let has_note_order: i64 = conn.query_row(
        "SELECT COUNT(1) FROM pragma_table_info('entries') WHERE name = 'note_order'",
        [],
        |row| row.get(0),
    )?;
    if has_note_order == 0 {
        conn.execute("ALTER TABLE entries ADD COLUMN note_order INTEGER DEFAULT 0", [])?;
    }

    if backfill_done == 0 {
        conn.execute(
            r#"
            INSERT OR IGNORE INTO applications (process_name, display_name, icon_png, last_resolved_at)
            SELECT DISTINCT
                LOWER(TRIM(target_app)),
                TRIM(target_app),
                NULL,
                datetime('now')
            FROM entries
            WHERE target_app IS NOT NULL AND TRIM(target_app) != ''
            "#,
            [],
        )?;
        conn.execute(
            "INSERT INTO _kalam_migrations (name) VALUES ('applications_backfill_v1')",
            [],
        )?;
    }

    // Dictionary table for custom vocabulary (sent as prompt to cloud STT)
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS dictionary (
            id TEXT PRIMARY KEY,
            term TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        "#,
    )?;

    // Daily stats: one row per calendar day (aggregates for streak, words, latency)
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS daily_stats (
            date TEXT PRIMARY KEY,
            transcriptions_count INTEGER NOT NULL DEFAULT 0,
            words_count INTEGER NOT NULL DEFAULT 0,
            latency_sum_ms INTEGER NOT NULL DEFAULT 0,
            latency_count INTEGER NOT NULL DEFAULT 0,
            latency_last_ms INTEGER,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;

    // Migrate legacy standalone `reminder` rows to notes (reminders are only via reminder_at on notes/tasks).
    let legacy_reminder_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entries WHERE entry_type = 'reminder'",
        [],
        |row| row.get(0),
    )?;
    if legacy_reminder_count > 0 {
        conn.execute(
            "UPDATE entries SET entry_type = 'note' WHERE entry_type = 'reminder'",
            [],
        )?;
    }

    #[cfg(debug_assertions)]
    maybe_seed_dev_sample_notes_if_empty(conn)?;

    Ok(())
}

/// Debug / `tauri dev` only: if there are no notes yet, insert a small set so the Notes UI is testable.
/// Release builds (`--release`) never run this. Clearing all notes during dev will re-seed on next DB open.
#[cfg(debug_assertions)]
fn maybe_seed_dev_sample_notes_if_empty(conn: &Connection) -> anyhow::Result<()> {
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    let note_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entries WHERE entry_type = 'note'",
        [],
        |row| row.get(0),
    )?;
    if note_count > 0 {
        return Ok(());
    }

    let tomorrow = Utc::now() + Duration::days(1);
    let specs: Vec<crate::models::Entry> = vec![
        crate::models::Entry {
            id: Uuid::new_v4().to_string(),
            entry_type: "note".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            sync_status: "pending".into(),
            title: Some("Pinned recap".into()),
            content: "**Actions**\n- Sync with design on voice UX\n- Draft changelog bullets\n\nSample content for the masonry layout.".into(),
            attachments: vec![],
            tags: vec!["work".into(), "meetings".into()],
            color: None,
            is_pinned: true,
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
        },
        crate::models::Entry {
            id: Uuid::new_v4().to_string(),
            entry_type: "note".into(),
            created_at: Utc::now() - Duration::minutes(5),
            updated_at: Utc::now() - Duration::minutes(5),
            sync_status: "pending".into(),
            title: Some("Reading list".into()),
            content: "Essays on calm software, local-first sync, and accessibility for dictation.".into(),
            attachments: vec![],
            tags: vec!["personal".into(), "ideas".into()],
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
        },
        crate::models::Entry {
            id: Uuid::new_v4().to_string(),
            entry_type: "note".into(),
            created_at: Utc::now() - Duration::minutes(10),
            updated_at: Utc::now() - Duration::minutes(10),
            sync_status: "pending".into(),
            title: Some("Groceries".into()),
            content: "- Oats\n- Berries\n- Yogurt".into(),
            attachments: vec![],
            tags: vec!["personal".into()],
            color: Some("#FFF9C4".into()),
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
        },
        crate::models::Entry {
            id: Uuid::new_v4().to_string(),
            entry_type: "note".into(),
            created_at: Utc::now() - Duration::minutes(15),
            updated_at: Utc::now() - Duration::minutes(15),
            sync_status: "pending".into(),
            title: Some("Label filter demo".into()),
            content: "Use the chips to filter by work or personal.".into(),
            attachments: vec![],
            tags: vec!["work".into()],
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
        },
        crate::models::Entry {
            id: Uuid::new_v4().to_string(),
            entry_type: "note".into(),
            created_at: Utc::now() - Duration::minutes(20),
            updated_at: Utc::now() - Duration::minutes(20),
            sync_status: "pending".into(),
            title: Some("Reminder sample".into()),
            content: "Shows the bell row when reminder_at is set.".into(),
            attachments: vec![],
            tags: vec!["demo".into()],
            color: None,
            is_pinned: false,
            priority: None,
            due_date: None,
            subtasks: None,
            is_completed: None,
            reminder_at: Some(tomorrow),
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
        },
    ];

    for e in specs {
        insert_entry(conn, &e)?;
        insert_embedding_stub(conn, &e.id)?;
    }

    Ok(())
}

/// Entry for the personal dictionary (custom words/phrases for cloud transcription).
#[derive(Debug, Clone, serde::Serialize)]
pub struct DictionaryEntry {
    pub id: String,
    pub term: String,
    pub created_at: String,
}

/// Get all dictionary entries ordered by created_at ascending.
pub fn get_dictionary_entries(conn: &Connection) -> anyhow::Result<Vec<DictionaryEntry>> {
    let mut stmt =
        conn.prepare("SELECT id, term, created_at FROM dictionary ORDER BY created_at ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(DictionaryEntry {
            id: row.get(0)?,
            term: row.get(1)?,
            created_at: row.get(2)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.into())
}

/// Add a dictionary term; returns the new entry id.
pub fn add_dictionary_entry(conn: &Connection, term: &str) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO dictionary (id, term, created_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![id, term, created_at],
    )?;
    Ok(id)
}

/// Remove a dictionary entry by id.
pub fn delete_dictionary_entry(conn: &Connection, id: &str) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM dictionary WHERE id = ?1",
        rusqlite::params![id],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Daily stats (one row per calendar day)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct DailyStatsRow {
    pub date: String,
    pub transcriptions_count: i64,
    pub words_count: i64,
    pub latency_avg_ms: Option<i64>,
    pub latency_last_ms: Option<i64>,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AggregateStats {
    pub streak_days: i64,
    pub total_words: i64,
    pub time_saved_hours: f64,
    pub last_latency_ms: Option<u32>,
    pub today_avg_latency_ms: Option<u32>,
}

/// Upsert daily_stats for the given date: increment transcriptions, add words, update latency (running avg + last).
pub fn record_transcription_stats(
    conn: &Connection,
    date: &str,
    words_count: u32,
    latency_ms: u32,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        r#"
        INSERT INTO daily_stats (date, transcriptions_count, words_count, latency_sum_ms, latency_count, latency_last_ms, updated_at)
        VALUES (?1, 1, ?2, ?3, 1, ?4, ?5)
        ON CONFLICT(date) DO UPDATE SET
            transcriptions_count = transcriptions_count + 1,
            words_count = words_count + ?2,
            latency_sum_ms = latency_sum_ms + ?3,
            latency_count = latency_count + 1,
            latency_last_ms = ?4,
            updated_at = ?5
        "#,
        rusqlite::params![date, words_count as i64, latency_ms as i64, latency_ms as i64, now],
    )?;
    Ok(())
}

/// Get daily stats for a date (ISO date string YYYY-MM-DD). None = today.
pub fn get_daily_stats(
    conn: &Connection,
    date: Option<&str>,
) -> anyhow::Result<Option<DailyStatsRow>> {
    let date = date
        .map(|s| s.to_string())
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
    let row = conn.query_row(
        "SELECT date, transcriptions_count, words_count, latency_sum_ms, latency_count, latency_last_ms, updated_at FROM daily_stats WHERE date = ?1",
        rusqlite::params![date],
        |row| {
            let latency_sum: i64 = row.get(3)?;
            let latency_count: i64 = row.get(4)?;
            Ok(DailyStatsRow {
                date: row.get(0)?,
                transcriptions_count: row.get(1)?,
                words_count: row.get(2)?,
                latency_avg_ms: if latency_count > 0 { Some(latency_sum / latency_count) } else { None },
                latency_last_ms: row.get(5)?,
                updated_at: row.get(6)?,
            })
        },
    );
    match row {
        Ok(r) => Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Streak = consecutive days (including today) with at least one transcription, going backward from today.
fn streak_from_daily_stats(conn: &Connection) -> anyhow::Result<i64> {
    let mut stmt = conn.prepare("SELECT date FROM daily_stats ORDER BY date DESC")?;
    let dates: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut streak: i64 = 0;
    let mut expect = today;
    for d in &dates {
        if d != &expect {
            break;
        }
        streak += 1;
        expect = chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
            .ok()
            .and_then(|dt| dt.pred_opt())
            .map(|prev| prev.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        if expect.is_empty() {
            break;
        }
    }
    Ok(streak)
}

/// Aggregate stats for dashboard: streak, total words, time saved (40 WPM), last and today avg latency.
pub fn get_aggregate_stats(conn: &Connection) -> anyhow::Result<AggregateStats> {
    let total_words: i64 = conn.query_row(
        "SELECT COALESCE(SUM(words_count), 0) FROM daily_stats",
        [],
        |row| row.get(0),
    )?;
    let streak_days = streak_from_daily_stats(conn)?;
    let time_saved_hours = (total_words as f64) / 40.0 / 60.0;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let (last_latency_ms, today_avg_latency_ms) = match get_daily_stats(conn, Some(&today))? {
        Some(row) => (
            row.latency_last_ms.map(|n| n as u32),
            row.latency_avg_ms.map(|n| n as u32),
        ),
        None => (None, None),
    };
    Ok(AggregateStats {
        streak_days,
        total_words,
        time_saved_hours,
        last_latency_ms,
        today_avg_latency_ms,
    })
}

// ---------------------------------------------------------------------------
// Overview dashboard aggregates (UTC date bucketing; matches daily_stats dates)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct WordsByDay {
    pub date: String,
    pub words: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AppCount {
    pub app: String,
    pub count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FlowPoint {
    pub created_at: String,
    pub target_app: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ActivityDay {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DashboardStats {
    pub words_dictated_7d: Vec<WordsByDay>,
    pub top_apps_7d: Vec<AppCount>,
    pub total_time_dictating_7d_ms: i64,
    pub dictation_flow_7d: Vec<FlowPoint>,
    pub session_lengths_7d_ms: Vec<i64>,
    /// Mean word count per history row among the same timed sessions as `session_lengths_7d_ms` (last 7d, `duration_ms` > 0).
    pub avg_words_per_dictation_7d: Option<f64>,
    pub activity_heatmap_14d: Vec<ActivityDay>,
    pub streak_days: i64,
    pub total_words: i64,
}

/// JSON-friendly dashboard payload for the Overview page charts.
pub fn get_dashboard_stats(conn: &Connection) -> anyhow::Result<DashboardStats> {
    use chrono::Duration;
    use std::collections::HashMap;

    let agg = get_aggregate_stats(conn)?;
    let today_naive = chrono::Utc::now().date_naive();
    let cutoff_7d = (chrono::Utc::now() - Duration::days(7)).to_rfc3339();

    // Last 7 UTC calendar days, words from daily_stats (zeros if no row).
    let mut words_dictated_7d = Vec::with_capacity(7);
    for i in (0..7).rev() {
        let d = today_naive - Duration::days(i);
        let ds = d.format("%Y-%m-%d").to_string();
        let words: i64 = match conn.query_row(
            "SELECT COALESCE(words_count, 0) FROM daily_stats WHERE date = ?1",
            rusqlite::params![&ds],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(w) => w,
            Err(rusqlite::Error::QueryReturnedNoRows) => 0,
            Err(e) => return Err(e.into()),
        };
        words_dictated_7d.push(WordsByDay { date: ds, words });
    }

    let mut stmt = conn.prepare(
        "SELECT COALESCE(NULLIF(TRIM(target_app), ''), '(unknown)'), COUNT(*) FROM entries \
         WHERE entry_type = 'history' AND created_at >= ?1 \
         GROUP BY 1 ORDER BY COUNT(*) DESC LIMIT 8",
    )?;
    let rows = stmt.query_map(rusqlite::params![&cutoff_7d], |row| {
        Ok(AppCount {
            app: row.get(0)?,
            count: row.get(1)?,
        })
    })?;
    let top_apps_7d: Vec<AppCount> = rows.collect::<rusqlite::Result<Vec<_>>>()?;

    let total_time_dictating_7d_ms: i64 = conn.query_row(
        "SELECT COALESCE(SUM(duration_ms), 0) FROM entries \
         WHERE entry_type = 'history' AND created_at >= ?1 AND duration_ms IS NOT NULL AND duration_ms > 0",
        rusqlite::params![&cutoff_7d],
        |row| row.get(0),
    )?;

    let flow_start = cutoff_7d.clone();
    let flow_end = chrono::Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "SELECT created_at, target_app FROM entries \
         WHERE entry_type = 'history' AND created_at >= ?1 AND created_at <= ?2 \
         ORDER BY created_at ASC",
    )?;
    let flow_rows = stmt.query_map(rusqlite::params![&flow_start, &flow_end], |row| {
        let created_at: String = row.get(0)?;
        let target_app: Option<String> = row.get(1)?;
        Ok(FlowPoint {
            created_at,
            target_app,
        })
    })?;
    let dictation_flow_7d: Vec<FlowPoint> = flow_rows.collect::<rusqlite::Result<Vec<_>>>()?;

    // Timed dictations only: same rows drive avg duration (frontend) and avg words below.
    let mut stmt = conn.prepare(
        "SELECT duration_ms, content FROM entries \
         WHERE entry_type = 'history' AND created_at >= ?1 AND duration_ms IS NOT NULL AND duration_ms > 0 \
         ORDER BY duration_ms ASC LIMIT 2000",
    )?;
    let sl_rows = stmt.query_map(rusqlite::params![&cutoff_7d], |row| {
        let ms: i64 = row.get(0)?;
        let content: String = row.get(1)?;
        Ok((ms, content))
    })?;
    let mut session_lengths_7d_ms: Vec<i64> = Vec::new();
    let mut words_sum: i64 = 0;
    let mut timed_n: i64 = 0;
    for r in sl_rows {
        let (ms, content) = r?;
        session_lengths_7d_ms.push(ms);
        words_sum += content.split_whitespace().count() as i64;
        timed_n += 1;
    }
    let avg_words_per_dictation_7d = if timed_n > 0 {
        Some((words_sum as f64) / (timed_n as f64))
    } else {
        None
    };

    let start_14 = today_naive - Duration::days(13);
    let heatmap_cutoff = start_14
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("invalid heatmap start"))?
        .and_utc()
        .to_rfc3339();
    let mut stmt = conn.prepare(
        "SELECT substr(created_at, 1, 10) AS d, COUNT(*) FROM entries \
         WHERE entry_type = 'history' AND created_at >= ?1 \
         GROUP BY d",
    )?;
    let hm_rows = stmt.query_map(rusqlite::params![&heatmap_cutoff], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    let mut hm_map: HashMap<String, i64> = HashMap::new();
    for r in hm_rows {
        let (d, c) = r?;
        hm_map.insert(d, c);
    }
    let mut activity_heatmap_14d = Vec::with_capacity(14);
    let mut d = start_14;
    while d <= today_naive {
        let key = d.format("%Y-%m-%d").to_string();
        activity_heatmap_14d.push(ActivityDay {
            count: *hm_map.get(&key).unwrap_or(&0),
            date: key,
        });
        d = d
            .succ_opt()
            .ok_or_else(|| anyhow::anyhow!("heatmap date overflow"))?;
    }

    Ok(DashboardStats {
        words_dictated_7d,
        top_apps_7d,
        total_time_dictating_7d_ms,
        dictation_flow_7d,
        session_lengths_7d_ms,
        avg_words_per_dictation_7d,
        activity_heatmap_14d,
        streak_days: agg.streak_days,
        total_words: agg.total_words,
    })
}

/// Embedding dimension used by vec_entries. Must match the virtual table definition.
pub fn embedding_dimension() -> u32 {
    VEC_EMBEDDING_DIM
}

/// Look up or populate `applications` for a process name; returns display name and cached icon bytes.
pub fn get_or_resolve_application(
    conn: &Connection,
    process_name: &str,
    exe_path_hint: Option<&str>,
) -> anyhow::Result<(String, Option<Vec<u8>>)> {
    let normalized = crate::app_info::normalize_process_name(process_name);
    let now = chrono::Utc::now().to_rfc3339();

    if let Ok(row) = conn.query_row(
        "SELECT display_name, icon_png FROM applications WHERE process_name = ?1",
        [&normalized],
        |row| {
            let name: String = row.get(0)?;
            let icon: Option<Vec<u8>> = row.get(1)?;
            Ok((name, icon))
        },
    ) {
        return Ok(row);
    }

    let resolved = exe_path_hint
        .and_then(crate::app_info::resolve)
        .or_else(|| crate::app_info::resolve(&normalized));

    let (display_name, icon_png) = if let Some(info) = resolved {
        (info.display_name, info.icon_png)
    } else {
        (
            crate::app_info::capitalize_process_name(&normalized),
            None,
        )
    };

    conn.execute(
        "INSERT OR REPLACE INTO applications (process_name, display_name, icon_png, last_resolved_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![&normalized, &display_name, &icon_png, &now],
    )?;

    Ok((display_name, icon_png))
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
    let archived_at = e.archived_at.as_ref().map(|a| a.to_rfc3339());
    let deleted_at = e.deleted_at.as_ref().map(|d| d.to_rfc3339());
    conn.execute(
        r#"
        INSERT INTO entries (id, entry_type, created_at, updated_at, sync_status, title, content,
            attachments, tags, color, is_pinned, priority, due_date, subtasks, is_completed, reminder_at, rrule, archived_at, deleted_at,
            target_app, target_app_name, duration_ms, word_count, stt_latency_ms, stt_mode, dictation_language, session_mode, stt_provider, note_order)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29)
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
            archived_at,
            deleted_at,
            e.target_app,
            e.target_app_name,
            e.duration_ms.map(|n| n as i64),
            e.word_count.map(|n| n as i64),
            e.stt_latency_ms.map(|n| n as i64),
            e.stt_mode,
            e.dictation_language,
            e.session_mode,
            e.stt_provider,
            e.note_order,
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
    let archived_at: Option<String> = row.get::<_, Option<String>>(17).ok().flatten();
    let deleted_at: Option<String> = row.get::<_, Option<String>>(18).ok().flatten();
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
    let due_date = due_date.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|d| d.with_timezone(&chrono::Utc))
    });
    let reminder_at = reminder_at.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|d| d.with_timezone(&chrono::Utc))
    });
    let archived_at = archived_at.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|d| d.with_timezone(&chrono::Utc))
    });
    let deleted_at = deleted_at.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|d| d.with_timezone(&chrono::Utc))
    });
    let target_app: Option<String> = row.get::<_, Option<String>>(19)?;
    let target_app_name: Option<String> = row.get::<_, Option<String>>(20)?;
    let duration_raw: Option<i64> = row.get::<_, Option<i64>>(21)?;
    let duration_ms = duration_raw.and_then(|n| u32::try_from(n).ok());
    let word_count_raw: Option<i64> = row.get::<_, Option<i64>>(22)?;
    let word_count = word_count_raw.and_then(|n| u32::try_from(n).ok());
    let stt_latency_raw: Option<i64> = row.get::<_, Option<i64>>(23)?;
    let stt_latency_ms = stt_latency_raw.and_then(|n| u32::try_from(n).ok());
    let stt_mode: Option<String> = row.get(24)?;
    let dictation_language: Option<String> = row.get(25)?;
    let session_mode: Option<String> = row.get(26)?;
    let stt_provider: Option<String> = row.get(27)?;
    let note_order: i64 = row
        .get::<_, Option<i64>>(28)
        .ok()
        .flatten()
        .unwrap_or(0);
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
        archived_at,
        deleted_at,
        target_app,
        target_app_name,
        duration_ms,
        word_count,
        stt_latency_ms,
        stt_mode,
        dictation_language,
        session_mode,
        stt_provider,
        note_order,
    })
}

/// Reminders view: notes with `reminder_at`, and open tasks with a time (`reminder_at` and/or `due_date`).
/// Task sort time is COALESCE(reminder_at, due_date). Completed tasks are omitted.
pub fn get_entries_with_reminder(
    conn: &Connection,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<crate::models::Entry>> {
    let mut stmt = conn.prepare(&format!(
        "{} FROM entries
         WHERE (entry_type = 'note' AND reminder_at IS NOT NULL AND deleted_at IS NULL)
            OR (entry_type = 'task' AND deleted_at IS NULL AND COALESCE(is_completed, 0) = 0
                AND (reminder_at IS NOT NULL OR due_date IS NOT NULL))
         ORDER BY CASE entry_type
             WHEN 'task' THEN COALESCE(reminder_at, due_date)
             ELSE reminder_at
           END ASC
         LIMIT ?1 OFFSET ?2",
        SELECT_ENTRY_ROW
    ))?;
    let rows = stmt.query_map(rusqlite::params![limit, offset], row_to_entry)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Same dataset as [`get_entries_with_reminder`] (kept for IPC / dev bridge callers).
pub fn get_entries_for_reminders_view(
    conn: &Connection,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<crate::models::Entry>> {
    get_entries_with_reminder(conn, limit, offset)
}

/// Get entries by type. For notes and tasks, optional scope: "active" | "archived" | "trash".
/// Order: for notes, is_pinned DESC, note_order ASC, updated_at DESC; for tasks, note_order ASC, updated_at DESC; otherwise updated_at DESC.
pub fn get_entries_by_type(
    conn: &Connection,
    entry_type: &str,
    scope: Option<&str>,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<crate::models::Entry>> {
    let (where_extra, order_by) = if entry_type == "note" {
        let scope = scope.unwrap_or("active");
        let where_scope = match scope {
            "archived" => " AND deleted_at IS NULL AND archived_at IS NOT NULL",
            "trash" => " AND deleted_at IS NOT NULL",
            _ => " AND deleted_at IS NULL AND archived_at IS NULL",
        };
        (where_scope, " ORDER BY is_pinned DESC, note_order ASC, updated_at DESC")
    } else if entry_type == "task" {
        let scope = scope.unwrap_or("active");
        let where_scope = match scope {
            "archived" => " AND deleted_at IS NULL AND archived_at IS NOT NULL",
            "trash" => " AND deleted_at IS NOT NULL",
            _ => " AND deleted_at IS NULL AND archived_at IS NULL",
        };
        // `note_order` is manual list order for tasks (same column as notes; tasks ignore pin fields).
        (where_scope, " ORDER BY note_order ASC, updated_at DESC")
    } else {
        ("", " ORDER BY updated_at DESC")
    };
    let sql = format!(
        "{} FROM entries WHERE entry_type = ?1{}{} LIMIT ?2 OFFSET ?3",
        SELECT_ENTRY_ROW, where_extra, order_by
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params![entry_type, limit, offset], row_to_entry)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Count notes in each list scope (same predicates as [`get_entries_by_type`] for `entry_type = note`).
pub fn count_notes_by_scope(conn: &Connection) -> anyhow::Result<(i64, i64, i64)> {
    conn.query_row(
        "SELECT
           (SELECT COUNT(*) FROM entries WHERE entry_type = 'note' AND deleted_at IS NULL AND archived_at IS NULL),
           (SELECT COUNT(*) FROM entries WHERE entry_type = 'note' AND deleted_at IS NULL AND archived_at IS NOT NULL),
           (SELECT COUNT(*) FROM entries WHERE entry_type = 'note' AND deleted_at IS NOT NULL)",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .map_err(Into::into)
}

/// Tasks whose `due_date` falls in `[day_start_iso, day_end_iso)` (half-open).
/// Bounds should be the user's local calendar day expressed as UTC instants (from the frontend).
/// Incomplete, active tasks only (not archived/deleted).
pub fn get_tasks_due_on(
    conn: &Connection,
    day_start_iso: &str,
    day_end_iso: &str,
    limit: i64,
) -> anyhow::Result<Vec<crate::models::Entry>> {
    let mut stmt = conn.prepare(&format!(
        "{} FROM entries WHERE entry_type = 'task' AND COALESCE(is_completed, 0) = 0 AND deleted_at IS NULL AND archived_at IS NULL
         AND due_date IS NOT NULL AND due_date >= ?1 AND due_date < ?2
         ORDER BY due_date ASC LIMIT ?3",
        SELECT_ENTRY_ROW
    ))?;
    let rows = stmt.query_map(rusqlite::params![day_start_iso, day_end_iso, limit], row_to_entry)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.into())
}

/// Reminders (or notes/tasks with `reminder_at`) firing in `[day_start_iso, day_end_iso)` (half-open).
/// Same local-day semantics as tasks.
pub fn get_reminders_due_on(
    conn: &Connection,
    day_start_iso: &str,
    day_end_iso: &str,
    limit: i64,
) -> anyhow::Result<Vec<crate::models::Entry>> {
    let mut stmt = conn.prepare(&format!(
        "{} FROM entries
         WHERE entry_type IN ('note','task') AND reminder_at IS NOT NULL
         AND deleted_at IS NULL AND reminder_at >= ?1 AND reminder_at < ?2
         ORDER BY reminder_at ASC LIMIT ?3",
        SELECT_ENTRY_ROW
    ))?;
    let rows = stmt.query_map(rusqlite::params![day_start_iso, day_end_iso, limit], row_to_entry)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.into())
}

/// Search notes with optional text query and label filter. Scope: "active" | "archived" | "trash".
pub fn search_notes(
    conn: &Connection,
    query: Option<&str>,
    label: Option<&str>,
    scope: &str,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<crate::models::Entry>> {
    let where_scope = match scope {
        "archived" => "deleted_at IS NULL AND archived_at IS NOT NULL",
        "trash" => "deleted_at IS NOT NULL",
        _ => "deleted_at IS NULL AND archived_at IS NULL",
    };
    let pattern = format!("%{}%", query.unwrap_or(""));
    let has_label = label.map(|l| !l.is_empty()).unwrap_or(false);

    let out = if has_label {
        let label = label.unwrap().to_string();
        let sql = format!(
            "{} FROM entries WHERE entry_type = 'note' AND {} AND (title LIKE ?1 OR content LIKE ?2)
             AND EXISTS (SELECT 1 FROM json_each(entries.tags) WHERE json_each.value = ?3)
             ORDER BY is_pinned DESC, note_order ASC, updated_at DESC LIMIT ?4 OFFSET ?5",
            SELECT_ENTRY_ROW, where_scope
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(
            rusqlite::params![&pattern, &pattern, &label, limit, offset],
            row_to_entry,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    } else {
        let sql = format!(
            "{} FROM entries WHERE entry_type = 'note' AND {} AND (title LIKE ?1 OR content LIKE ?2)
             ORDER BY is_pinned DESC, note_order ASC, updated_at DESC LIMIT ?3 OFFSET ?4",
            SELECT_ENTRY_ROW, where_scope
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(
            rusqlite::params![&pattern, &pattern, limit, offset],
            row_to_entry,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    Ok(out)
}

/// Get distinct tag strings from notes in the given scope. Scope: "active" | "archived" | "trash" (or "all" for no filter).
pub fn get_note_labels(conn: &Connection, scope: Option<&str>) -> anyhow::Result<Vec<String>> {
    let where_scope = match scope {
        Some("archived") => "deleted_at IS NULL AND archived_at IS NOT NULL",
        Some("trash") => "deleted_at IS NOT NULL",
        Some("active") | None => "deleted_at IS NULL AND archived_at IS NULL",
        _ => "", // "all" or unknown: no scope filter
    };
    let sql = if where_scope.is_empty() {
        "SELECT DISTINCT value FROM entries, json_each(entries.tags) WHERE entry_type = 'note'"
            .to_string()
    } else {
        format!(
            "SELECT DISTINCT value FROM entries, json_each(entries.tags) WHERE entry_type = 'note' AND {}",
            where_scope
        )
    };
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    out.sort();
    Ok(out)
}

/// Permanently delete all trashed notes (entry_type = 'note' AND deleted_at IS NOT NULL). Returns count deleted.
pub fn empty_trash(conn: &Connection) -> anyhow::Result<i64> {
    let ids: Vec<String> = conn
        .prepare("SELECT id FROM entries WHERE entry_type = 'note' AND deleted_at IS NOT NULL")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let count = ids.len() as i64;
    for id in &ids {
        let _ = conn.execute(
            "DELETE FROM vec_entries WHERE entry_id = ?1",
            rusqlite::params![id],
        );
        conn.execute("DELETE FROM entries WHERE id = ?1", rusqlite::params![id])?;
    }
    Ok(count)
}

/// Permanently delete all trashed tasks (entry_type = 'task' AND deleted_at IS NOT NULL). Returns count deleted.
pub fn empty_task_trash(conn: &Connection) -> anyhow::Result<i64> {
    let ids: Vec<String> = conn
        .prepare("SELECT id FROM entries WHERE entry_type = 'task' AND deleted_at IS NOT NULL")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let count = ids.len() as i64;
    for id in &ids {
        let _ = conn.execute(
            "DELETE FROM vec_entries WHERE entry_id = ?1",
            rusqlite::params![id],
        );
        conn.execute("DELETE FROM entries WHERE id = ?1", rusqlite::params![id])?;
    }
    Ok(count)
}

/// Get a single entry by id.
pub fn get_entry(conn: &Connection, id: &str) -> anyhow::Result<Option<crate::models::Entry>> {
    let mut stmt = conn.prepare(&format!(
        "{} FROM entries WHERE id = ?1",
        SELECT_ENTRY_ROW
    ))?;
    let mut rows = stmt.query(rusqlite::params![id])?;
    Ok(rows.next()?.map(|row| row_to_entry(row)).transpose()?)
}

/// Update an existing entry. Returns true if a row was updated.
pub fn update_entry(conn: &Connection, e: &crate::models::Entry) -> anyhow::Result<bool> {
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
    let archived_at = e.archived_at.as_ref().map(|a| a.to_rfc3339());
    let deleted_at = e.deleted_at.as_ref().map(|d| d.to_rfc3339());
    let n = conn.execute(
        r#"
        UPDATE entries SET entry_type = ?1, created_at = ?2, updated_at = ?3, sync_status = ?4,
            title = ?5, content = ?6, attachments = ?7, tags = ?8, color = ?9, is_pinned = ?10,
            priority = ?11, due_date = ?12, subtasks = ?13, is_completed = ?14, reminder_at = ?15, rrule = ?16,
            archived_at = ?17, deleted_at = ?18, target_app = ?19, target_app_name = ?20, duration_ms = ?21,
            word_count = ?22, stt_latency_ms = ?23,             stt_mode = ?24, dictation_language = ?25, session_mode = ?26,
            stt_provider = ?27, note_order = ?28
        WHERE id = ?29
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
            archived_at,
            deleted_at,
            e.target_app,
            e.target_app_name,
            e.duration_ms.map(|n| n as i64),
            e.word_count.map(|n| n as i64),
            e.stt_latency_ms.map(|n| n as i64),
            e.stt_mode,
            e.dictation_language,
            e.session_mode,
            e.stt_provider,
            e.note_order,
            e.id,
        ],
    )?;
    Ok(n > 0)
}

/// Delete an entry and its embedding.
pub fn delete_entry(conn: &Connection, id: &str) -> anyhow::Result<bool> {
    let n = conn.execute("DELETE FROM entries WHERE id = ?1", rusqlite::params![id])?;
    let _ = conn.execute(
        "DELETE FROM vec_entries WHERE entry_id = ?1",
        rusqlite::params![id],
    );
    Ok(n > 0)
}

/// Semantic search: return entry IDs ordered by similarity to the query embedding. For now uses stub embeddings.
pub fn search_similar(
    conn: &Connection,
    _query_embedding: &[f32],
    limit: i64,
) -> anyhow::Result<Vec<String>> {
    if _query_embedding.is_empty() {
        let mut stmt = conn.prepare("SELECT entry_id FROM vec_entries LIMIT ?1")?;
        let rows = stmt.query_map(rusqlite::params![limit], |row| row.get::<_, String>(0))?;
        return Ok(rows.collect::<Result<Vec<_>, _>>()?);
    }
    let json_array =
        serde_json::to_string(_query_embedding).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let mut stmt = conn.prepare(
        "SELECT entry_id FROM vec_entries WHERE embedding MATCH vec_f32(?1) ORDER BY distance LIMIT ?2",
    )?;
    let rows = stmt.query_map(rusqlite::params![json_array, limit], |row| {
        row.get::<_, String>(0)
    })?;
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
        };
        insert_entry(&conn, &entry)?;
        insert_embedding_stub(&conn, &id)?;
    }
    log::info!("Migrated {} snippets from config to DB", snippets.len());
    Ok(true)
}
