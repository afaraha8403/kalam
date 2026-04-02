//! Build outbound `PushItem`s from local DB + config.

use serde::Serialize;

use crate::config::AppConfig;
use crate::db;
use crate::sync::blobs::SyncSettingsBlob;
use crate::sync::client::PushItem;
use crate::sync::crypto::encrypt_provider_keys_json;
use rusqlite::Connection;

#[derive(Serialize)]
struct DictionaryPayloadOut<'a> {
    id: &'a str,
    term: &'a str,
    created_at: &'a str,
    updated_at: &'a str,
    deleted_at: Option<&'a str>,
}

pub fn collect_push_items(
    conn: &Connection,
    cfg: &AppConfig,
    license_key: &str,
) -> Result<Vec<PushItem>, String> {
    let mut items: Vec<PushItem> = Vec::new();

    for (id, updated_at) in db::list_sync_pending_deletes(conn).map_err(|e| e.to_string())? {
        items.push(PushItem {
            bucket: "entries".into(),
            id,
            payload: "{}".into(),
            updated_at,
            deleted: Some(1),
        });
    }

    for e in db::list_entries_pending_sync(conn).map_err(|e| e.to_string())? {
        let payload = serde_json::to_string(&e).map_err(|e| e.to_string())?;
        items.push(PushItem {
            bucket: "entries".into(),
            id: e.id.clone(),
            payload,
            updated_at: e.updated_at.to_rfc3339(),
            deleted: None,
        });
    }

    for row in db::list_dictionary_for_sync_push(conn).map_err(|e| e.to_string())? {
        let deleted = row.deleted_at.is_some();
        let payload = if deleted {
            "{}".to_string()
        } else {
            let p = DictionaryPayloadOut {
                id: &row.id,
                term: &row.term,
                created_at: &row.created_at,
                updated_at: &row.updated_at,
                deleted_at: None,
            };
            serde_json::to_string(&p).map_err(|e| e.to_string())?
        };
        items.push(PushItem {
            bucket: "dictionary".into(),
            id: row.id.clone(),
            payload,
            updated_at: row.updated_at.clone(),
            deleted: if deleted { Some(1) } else { None },
        });
    }

    if cfg.sync_config_dirty {
        for m in &cfg.modes {
            let payload = serde_json::to_string(m).map_err(|e| e.to_string())?;
            items.push(PushItem {
                bucket: "modes".into(),
                id: m.id.clone(),
                payload,
                updated_at: m.updated_at.clone(),
                deleted: None,
            });
        }

        let rev = cfg
            .sync_settings_rev
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
        let blob = SyncSettingsBlob::from_config(cfg, &rev);
        let payload = serde_json::to_string(&blob).map_err(|e| e.to_string())?;
        items.push(PushItem {
            bucket: "settings".into(),
            id: "_settings".into(),
            payload,
            updated_at: rev.clone(),
            deleted: None,
        });

        let keys_json = serde_json::to_string(&cfg.provider_keys).map_err(|e| e.to_string())?;
        let enc = encrypt_provider_keys_json(license_key, &rev, &keys_json)?;
        items.push(PushItem {
            bucket: "provider_keys".into(),
            id: "_keys".into(),
            payload: enc,
            updated_at: rev,
            deleted: None,
        });
    }

    Ok(items)
}

/// After a successful push, mark local rows synced / cleanup tombstones.
pub fn finalize_after_push(
    conn: &Connection,
    cfg: &mut AppConfig,
    pushed: &[PushItem],
) -> Result<(), String> {
    let mut entry_ids: Vec<String> = Vec::new();
    let mut dict_mark: Vec<String> = Vec::new();
    let mut dict_purge: Vec<String> = Vec::new();
    let mut tomb_ids: Vec<String> = Vec::new();

    for it in pushed {
        if it.bucket == "entries" {
            if it.deleted == Some(1) {
                tomb_ids.push(it.id.clone());
            } else {
                entry_ids.push(it.id.clone());
            }
        }
        if it.bucket == "dictionary" {
            if it.deleted == Some(1) {
                dict_purge.push(it.id.clone());
            } else {
                dict_mark.push(it.id.clone());
            }
        }
    }

    db::remove_sync_pending_deletes(conn, &tomb_ids).map_err(|e| e.to_string())?;
    db::mark_entries_synced(conn, &entry_ids).map_err(|e| e.to_string())?;
    db::mark_dictionary_synced(conn, &dict_mark).map_err(|e| e.to_string())?;
    db::purge_dictionary_tombstones(conn, &dict_purge).map_err(|e| e.to_string())?;

    let had_config = pushed
        .iter()
        .any(|p| p.bucket == "settings" || p.bucket == "provider_keys" || p.bucket == "modes");
    if had_config {
        cfg.sync_config_dirty = false;
    }

    Ok(())
}
