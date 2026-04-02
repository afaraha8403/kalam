//! Phase 9 — multi-PC sync (Pro): pull from Worker, merge locally, push pending rows/config.

mod blobs;
mod client;
mod crypto;
mod diff;
mod merge;

use std::sync::Mutex;

use once_cell::sync::Lazy;
use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::config::ConfigManager;
use crate::db;
use crate::AppState;

const PULL_EPOCH: &str = "1970-01-01T00:00:00.000Z";
const PUSH_CHUNK: usize = 500;

static SYNC_UI: Lazy<Mutex<SyncUiState>> = Lazy::new(|| {
    Mutex::new(SyncUiState {
        syncing: false,
        last_error: None,
    })
});

#[derive(Debug, Default, Clone)]
struct SyncUiState {
    syncing: bool,
    last_error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusDto {
    pub enabled: bool,
    pub last_sync_at: Option<String>,
    pub syncing: bool,
    pub error: Option<String>,
    pub device_id: Option<String>,
    pub has_license_key: bool,
}

fn set_sync_ui(syncing: bool, err: Option<String>) {
    if let Ok(mut g) = SYNC_UI.lock() {
        g.syncing = syncing;
        g.last_error = err;
    }
}

pub fn get_sync_status_dto(cfg: &crate::config::AppConfig) -> SyncStatusDto {
    let ui = SYNC_UI.lock().map(|g| g.clone()).unwrap_or_default();
    SyncStatusDto {
        enabled: cfg.sync_enabled,
        last_sync_at: cfg.sync_last_at.clone(),
        syncing: ui.syncing,
        error: ui.last_error.clone(),
        device_id: cfg.sync_device_id.clone(),
        has_license_key: cfg
            .license_key
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false),
    }
}

/// Full pull → merge → push cycle. Updates config on disk when merges or pushes succeed.
pub async fn run_sync_cycle(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let enabled = {
        let cm = state.config.lock().await;
        cm.get_all().sync_enabled
    };
    if !enabled {
        return Ok(());
    }
    set_sync_ui(true, None);
    let res = run_sync_cycle_inner(app).await;
    match &res {
        Ok(()) => set_sync_ui(false, None),
        Err(e) => set_sync_ui(false, Some(e.clone())),
    }
    res
}

async fn run_sync_cycle_inner(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let (sync_on, key, base, since_start) = {
        let cm = state.config.lock().await;
        let c = cm.get_all();
        if !c.sync_enabled {
            return Ok(());
        }
        let key = c
            .license_key
            .clone()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| "Enter your Pro license key in Settings to use sync.".to_string())?;
        let base = client::api_base_url(&c.recipe_library_url);
        let since = c
            .sync_last_at
            .clone()
            .unwrap_or_else(|| PULL_EPOCH.to_string());
        (true, key, base, since)
    };
    if !sync_on {
        return Ok(());
    }

    if !client::validate_license_ok(&http, &base, &key)
        .await
        .map_err(|e| e.to_string())?
    {
        return Err("License is not valid for sync (need active Pro or trial).".to_string());
    }

    let mut pulled = Vec::new();
    let mut since = since_start;
    let mut after_id = String::new();
    let mut last_server_time;
    loop {
        let page = client::pull_page(&http, &base, &key, &since, &after_id).await?;
        last_server_time = page.server_time.clone();
        let n = page.items.len();
        if n == 0 && !page.has_more {
            break;
        }
        for it in page.items {
            pulled.push(it);
        }
        if !page.has_more {
            break;
        }
        // Avoid spinning if the server reports more pages but returns no rows.
        if page.has_more && n == 0 {
            break;
        }
        if let Some(last) = pulled.last() {
            since = last.updated_at.clone();
            after_id = last.id.clone();
        } else {
            break;
        }
    }

    let conn = db::open_db().map_err(|e| e.to_string())?;
    {
        let mut cm = state.config.lock().await;
        let mut cfg = cm.get_all();
        if !pulled.is_empty() {
            merge::apply_pull_batch(&conn, &mut cfg, &pulled, &key)?;
        }
        cfg.sync_last_at = Some(last_server_time.clone());
        cm.save(cfg).map_err(|e| e.to_string())?;
    }

    let items = {
        let cm = state.config.lock().await;
        let cfg = cm.get_all();
        diff::collect_push_items(&conn, &cfg, &key)?
    };

    if !items.is_empty() {
        for chunk in items.chunks(PUSH_CHUNK) {
            let chunk_vec = chunk.to_vec();
            let resp = client::push_batch(&http, &base, &key, chunk_vec.clone()).await?;
            let _ = (resp.accepted, resp.conflicts);
            let st = resp.server_time;
            let mut cm = state.config.lock().await;
            let mut cfg2 = cm.get_all();
            diff::finalize_after_push(&conn, &mut cfg2, &chunk_vec)?;
            cfg2.sync_last_at = Some(st.clone());
            cm.save(cfg2).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

pub async fn reset_remote_and_local_meta(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let (key, base) = {
        let cm = state.config.lock().await;
        let c = cm.get_all();
        let key = c
            .license_key
            .clone()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| "missing_license_key".to_string())?;
        (key, client::api_base_url(&c.recipe_library_url))
    };

    client::reset_remote(&http, &base, &key).await?;

    let mut cm = state.config.lock().await;
    let mut cfg = cm.get_all();
    cfg.sync_last_at = None;
    cfg.sync_last_merged_settings_at = None;
    cfg.sync_last_merged_keys_at = None;
    cfg.sync_config_dirty = true;
    cm.save(cfg).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn enable_sync(cm: &mut ConfigManager) -> Result<(), String> {
    let mut cfg = cm.get_all();
    cfg.sync_enabled = true;
    if cfg.sync_device_id.is_none() {
        cfg.sync_device_id = Some(uuid::Uuid::new_v4().to_string());
    }
    cfg.sync_config_dirty = true;
    cm.save(cfg).map_err(|e| e.to_string())
}

pub fn disable_sync(cm: &mut ConfigManager) -> Result<(), String> {
    let mut cfg = cm.get_all();
    cfg.sync_enabled = false;
    cm.save(cfg).map_err(|e| e.to_string())
}
