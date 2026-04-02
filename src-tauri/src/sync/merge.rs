//! Apply server rows to local SQLite + config (last-write-wins using `updated_at`).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::{AppConfig, DictationMode};
use crate::db;
use crate::models::Entry;
use crate::sync::blobs::SyncSettingsBlob;
use crate::sync::client::PullItem;
use crate::sync::crypto::{decrypt_provider_keys_json, envelope_updated_at};
use rusqlite::Connection;

#[derive(Debug, Serialize, Deserialize)]
struct DictionaryPayload {
    id: String,
    term: String,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    deleted_at: Option<String>,
}

fn parse_dt(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

fn rank_bucket(b: &str) -> u8 {
    match b {
        "modes" => 0,
        "settings" => 1,
        "provider_keys" => 2,
        "entries" => 3,
        "dictionary" => 4,
        _ => 5,
    }
}

pub fn sort_pull_items(mut items: Vec<PullItem>) -> Vec<PullItem> {
    items.sort_by(|a, b| {
        rank_bucket(&a.bucket)
            .cmp(&rank_bucket(&b.bucket))
            .then_with(|| a.updated_at.cmp(&b.updated_at))
            .then_with(|| a.id.cmp(&b.id))
    });
    items
}

pub fn apply_dictionary_pull(conn: &Connection, item: &PullItem) -> Result<(), String> {
    if item.deleted == 1 {
        let _ = conn.execute("DELETE FROM dictionary WHERE id = ?1", [&item.id]);
        return Ok(());
    }
    let p: DictionaryPayload = serde_json::from_str(&item.payload).map_err(|e| e.to_string())?;
    if p.deleted_at.is_some() {
        let _ = conn.execute("DELETE FROM dictionary WHERE id = ?1", [&p.id]);
        return Ok(());
    }
    let local_updated: Option<String> = match conn.query_row(
        "SELECT updated_at FROM dictionary WHERE id = ?1 AND deleted_at IS NULL",
        [&p.id],
        |row| row.get(0),
    ) {
        Ok(s) => Some(s),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(e.to_string()),
    };
    if let Some(lu) = local_updated {
        if lu.as_str() >= p.updated_at.as_str() {
            return Ok(());
        }
    }
    conn.execute(
        "INSERT INTO dictionary (id, term, created_at, updated_at, sync_status, deleted_at) VALUES (?1, ?2, ?3, ?4, 'synced', NULL)
         ON CONFLICT(id) DO UPDATE SET
           term = excluded.term,
           created_at = excluded.created_at,
           updated_at = excluded.updated_at,
           sync_status = 'synced',
           deleted_at = NULL",
        rusqlite::params![p.id, p.term, p.created_at, p.updated_at],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn apply_entry_pull(conn: &Connection, item: &PullItem) -> Result<(), String> {
    if item.deleted == 1 {
        let _ = db::delete_entry_with_options(conn, &item.id, false);
        let _ = db::remove_sync_pending_deletes(conn, &[item.id.clone()]);
        return Ok(());
    }
    let mut remote: Entry = serde_json::from_str(&item.payload).map_err(|e| e.to_string())?;
    if !matches!(remote.entry_type.as_str(), "note" | "task" | "snippet") {
        return Ok(());
    }
    remote.sync_status = "synced".to_string();
    let r_ts = remote.updated_at;
    if let Some(local) = db::get_entry(conn, &remote.id).map_err(|e| e.to_string())? {
        if local.updated_at >= r_ts {
            return Ok(());
        }
    }
    if db::get_entry(conn, &remote.id)
        .map_err(|e| e.to_string())?
        .is_some()
    {
        db::update_entry_from_sync_merge(conn, &remote).map_err(|e| e.to_string())?;
    } else {
        db::insert_entry(conn, &remote).map_err(|e| e.to_string())?;
        let _ = db::insert_embedding_stub(conn, &remote.id);
    }
    Ok(())
}

pub fn apply_mode_pull(cfg: &mut AppConfig, item: &PullItem) -> Result<(), String> {
    if (item.id == "default" || item.id == "voice") && item.deleted == 1 {
        return Ok(());
    }
    if item.deleted == 1 {
        cfg.modes.retain(|m| m.id != item.id);
        return Ok(());
    }
    let remote: DictationMode = serde_json::from_str(&item.payload).map_err(|e| e.to_string())?;
    let rt = parse_dt(&remote.updated_at);
    if let Some(idx) = cfg.modes.iter().position(|m| m.id == remote.id) {
        let local = &cfg.modes[idx];
        let lt = parse_dt(&local.updated_at);
        if let (Some(lt), Some(rt)) = (lt, rt) {
            if lt > rt {
                return Ok(());
            }
        }
        cfg.modes[idx] = remote;
    } else {
        cfg.modes.push(remote);
    }
    Ok(())
}

pub fn apply_settings_pull(cfg: &mut AppConfig, item: &PullItem) -> Result<(), String> {
    let blob: SyncSettingsBlob = serde_json::from_str(&item.payload).map_err(|e| e.to_string())?;
    let remote_t = &blob.updated_at;
    let skip = match &cfg.sync_last_merged_settings_at {
        Some(prev) => remote_t.as_str() <= prev.as_str(),
        None => false,
    };
    if skip {
        return Ok(());
    }
    blob.apply_to(cfg);
    cfg.sync_last_merged_settings_at = Some(remote_t.clone());
    crate::config::merge_provider_keys_from_nested_maps(cfg);
    Ok(())
}

pub fn apply_provider_keys_pull(
    cfg: &mut AppConfig,
    item: &PullItem,
    license_key: &str,
) -> Result<(), String> {
    let env_ts = envelope_updated_at(&item.payload).map_err(|e| e.to_string())?;
    let skip = match &cfg.sync_last_merged_keys_at {
        Some(prev) => env_ts.as_str() <= prev.as_str(),
        None => false,
    };
    if skip {
        return Ok(());
    }
    let json = decrypt_provider_keys_json(license_key, &item.payload)?;
    let map: HashMap<String, String> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    cfg.provider_keys = map;
    cfg.sync_last_merged_keys_at = Some(env_ts.clone());
    crate::config::merge_provider_keys_from_nested_maps(cfg);
    Ok(())
}

pub fn apply_pull_batch(
    conn: &Connection,
    cfg: &mut AppConfig,
    items: &[PullItem],
    license_key: &str,
) -> Result<(), String> {
    let sorted = sort_pull_items(items.to_vec());
    for item in sorted {
        match item.bucket.as_str() {
            "dictionary" => apply_dictionary_pull(conn, &item)?,
            "entries" => apply_entry_pull(conn, &item)?,
            "modes" => apply_mode_pull(cfg, &item)?,
            "settings" => apply_settings_pull(cfg, &item)?,
            "provider_keys" => apply_provider_keys_pull(cfg, &item, license_key)?,
            _ => {}
        }
    }
    Ok(())
}
