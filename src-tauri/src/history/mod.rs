#![allow(dead_code)]

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm,
};
use chrono::{DateTime, Utc};
use rand::RngCore;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::db;
use crate::models::Entry;

const HISTORY_DB: &str = "history.db";
const KEY_FILE: &str = ".key";
const MAX_HISTORY_ENTRIES: usize = 1000;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;

fn get_legacy_db_path() -> anyhow::Result<PathBuf> {
    Ok(crate::config::get_kalam_dir()?.join(HISTORY_DB))
}

fn get_key_path() -> anyhow::Result<PathBuf> {
    Ok(crate::config::get_kalam_dir()?.join(KEY_FILE))
}

fn ensure_key() -> anyhow::Result<[u8; 32]> {
    let path = get_key_path()?;
    if path.exists() {
        let key = fs::read(&path)?;
        if key.len() == 32 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&key);
            return Ok(arr);
        }
    }
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    fs::write(&path, key)?;
    Ok(key)
}

fn encrypt(plaintext: &str, key: &[u8; 32]) -> anyhow::Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let mut nonce = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt((&nonce).into(), plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let mut out = nonce.to_vec();
    out.extend(ciphertext);
    Ok(out)
}

fn decrypt(blob: &[u8], key: &[u8; 32]) -> anyhow::Result<String> {
    if blob.len() < NONCE_LEN + TAG_LEN {
        return Err(anyhow::anyhow!("Blob too short"));
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let (nonce, ct) = blob.split_at(NONCE_LEN);
    let plain = cipher
        .decrypt(nonce.into(), ct)
        .map_err(|_| anyhow::anyhow!("Decrypt failed"))?;
    String::from_utf8(plain).map_err(|e| anyhow::anyhow!("{:?}", e))
}

fn open_legacy_db() -> anyhow::Result<Connection> {
    let path = get_legacy_db_path()?;
    let conn = Connection::open(&path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS history (
            id TEXT PRIMARY KEY,
            text_blob BLOB NOT NULL,
            created_at TEXT NOT NULL,
            mode TEXT NOT NULL,
            language TEXT NOT NULL,
            duration_ms INTEGER
        )",
        [],
    )?;
    Ok(conn)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
    pub mode: String,
    pub language: String,
    pub duration_ms: Option<u32>,
}

pub async fn save_transcription(text: &str) -> anyhow::Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();
    let entry = Entry {
        id: id.clone(),
        entry_type: "history".to_string(),
        created_at: now,
        updated_at: now,
        sync_status: "pending".to_string(),
        title: None,
        content: text.to_string(),
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
    };
    let conn = db::open_db()?;
    db::insert_entry(&conn, &entry)?;
    db::insert_embedding_stub(&conn, &id)?;
    trim_history_if_needed(&conn)?;
    Ok(())
}

fn trim_history_if_needed(conn: &rusqlite::Connection) -> anyhow::Result<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entries WHERE entry_type = 'history'",
        [],
        |r| r.get(0),
    )?;
    if count > MAX_HISTORY_ENTRIES as i64 {
        let to_delete = count - MAX_HISTORY_ENTRIES as i64;
        conn.execute(
            "DELETE FROM entries WHERE id IN (
                SELECT id FROM entries WHERE entry_type = 'history' ORDER BY created_at ASC LIMIT ?1
            )",
            [to_delete],
        )?;
        conn.execute(
            "DELETE FROM vec_entries WHERE entry_id NOT IN (SELECT id FROM entries)",
            [],
        )?;
    }
    Ok(())
}

pub async fn get_history(
    limit: Option<u32>,
    offset: Option<u32>,
) -> anyhow::Result<Vec<HistoryEntry>> {
    let conn = db::open_db()?;
    let limit = limit.unwrap_or(50) as i64;
    let offset = offset.unwrap_or(0) as i64;
    let mut stmt = conn.prepare(
        "SELECT id, content, created_at FROM entries WHERE entry_type = 'history'
         ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
    )?;
    let rows = stmt.query_map(rusqlite::params![limit, offset], |row| {
        let id: String = row.get(0)?;
        let text: String = row.get(1)?;
        let created_at: String = row.get(2)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        Ok(HistoryEntry {
            id,
            text,
            created_at,
            mode: "cloud".to_string(),
            language: "auto".to_string(),
            duration_ms: None,
        })
    })?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

pub async fn search(query: &str) -> anyhow::Result<Vec<HistoryEntry>> {
    let all = get_history(Some(1000), None).await?;
    let q = query.to_lowercase();
    Ok(all
        .into_iter()
        .filter(|e| e.text.to_lowercase().contains(&q))
        .collect())
}

pub async fn clear() -> anyhow::Result<()> {
    let conn = db::open_db()?;
    conn.execute("DELETE FROM entries WHERE entry_type = 'history'", [])?;
    conn.execute(
        "DELETE FROM vec_entries WHERE entry_id NOT IN (SELECT id FROM entries)",
        [],
    )?;
    Ok(())
}

pub async fn export(format: &str) -> anyhow::Result<String> {
    let entries = get_history(None, None).await?;
    match format {
        "json" => Ok(serde_json::to_string_pretty(&entries)?),
        "csv" => {
            let mut csv = String::from("id,text,created_at,mode,language\n");
            for entry in entries {
                csv.push_str(&format!(
                    "{},{},{},{},{}\n",
                    entry.id,
                    entry.text.replace(',', "\\,"),
                    entry.created_at.to_rfc3339(),
                    entry.mode,
                    entry.language
                ));
            }
            Ok(csv)
        }
        "txt" => {
            let mut txt = String::new();
            for entry in entries {
                txt.push_str(&format!("{}\n---\n", entry.text));
            }
            Ok(txt)
        }
        _ => Err(anyhow::anyhow!("Unknown format: {}", format)),
    }
}

/// Migrate from legacy history.json into legacy history.db if it exists and legacy DB is empty.
fn migrate_json_to_legacy_if_needed() -> anyhow::Result<()> {
    let kalam = crate::config::get_kalam_dir()?;
    let json_path = kalam.join("history.json");
    if !json_path.exists() {
        return Ok(());
    }
    let conn = open_legacy_db()?;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }
    let contents = fs::read_to_string(&json_path)?;
    let store: serde_json::Value =
        serde_json::from_str(&contents).unwrap_or(serde_json::Value::Null);
    let entries = store
        .get("entries")
        .and_then(|e| e.as_array())
        .cloned()
        .unwrap_or_default();
    let key = ensure_key()?;
    for e in entries {
        let text = e.get("text").and_then(|t| t.as_str()).unwrap_or("");
        let created_at: String = e
            .get("created_at")
            .and_then(|c| c.as_str())
            .map(String::from)
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        let id: String = e
            .get("id")
            .and_then(|i| i.as_str())
            .map(String::from)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mode = e.get("mode").and_then(|m| m.as_str()).unwrap_or("cloud");
        let language = e.get("language").and_then(|l| l.as_str()).unwrap_or("auto");
        let blob = encrypt(text, &key)?;
        conn.execute(
            "INSERT OR IGNORE INTO history (id, text_blob, created_at, mode, language, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
            rusqlite::params![id, blob, created_at, mode, language],
        )?;
    }
    fs::remove_file(&json_path).ok();
    log::info!("Migrated history from JSON to legacy SQLite");
    Ok(())
}

/// Migrate from legacy history.db (encrypted) into unified data.db entries table. Run once at startup.
pub fn migrate_legacy_to_unified() -> anyhow::Result<()> {
    let legacy_path = get_legacy_db_path()?;
    if !legacy_path.exists() {
        return Ok(());
    }
    let unified = db::open_db()?;
    let count: i64 = unified.query_row(
        "SELECT COUNT(*) FROM entries WHERE entry_type = 'history'",
        [],
        |r| r.get(0),
    )?;
    if count > 0 {
        return Ok(());
    }
    let key = ensure_key()?;
    let legacy_conn = open_legacy_db()?;
    let rows: Vec<_> = {
        let mut stmt = legacy_conn
            .prepare("SELECT id, text_blob, created_at FROM history ORDER BY created_at ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    drop(legacy_conn);
    for (id, blob, created_at_str) in rows {
        let text = decrypt(&blob, &key).unwrap_or_default();
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let entry = Entry {
            id: id.clone(),
            entry_type: "history".to_string(),
            created_at,
            updated_at: created_at,
            sync_status: "pending".to_string(),
            title: None,
            content: text,
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
        };
        db::insert_entry(&unified, &entry)?;
        db::insert_embedding_stub(&unified, &id)?;
    }
    fs::remove_file(&legacy_path).ok();
    fs::remove_file(get_key_path()?).ok();
    log::info!("Migrated legacy history.db to unified data.db");
    Ok(())
}

/// Run all history migrations: JSON -> legacy, then legacy -> unified. Call once at startup.
pub fn migrate_from_json_if_needed() -> anyhow::Result<()> {
    migrate_json_to_legacy_if_needed()?;
    migrate_legacy_to_unified()?;
    Ok(())
}

/// Remove all persisted history data (unified DB, legacy DB, key, legacy JSON). Used for full app reset.
pub fn delete_all_persisted_data() -> anyhow::Result<()> {
    let kalam = crate::config::get_kalam_dir()?;
    let _ = fs::remove_file(kalam.join("data.db"));
    let _ = fs::remove_file(kalam.join(HISTORY_DB));
    let _ = fs::remove_file(kalam.join(KEY_FILE));
    let _ = fs::remove_file(kalam.join("history.json"));
    log::info!("Deleted all history data (unified, legacy, key, JSON)");
    Ok(())
}
