//! Dev-only HTTP server so the browser (Vite dev at localhost:5173) can read DB/settings
//! when the Tauri app is running. Enable with: cargo tauri dev --features dev-bridge
//! Frontend uses fetch to http://localhost:1430 when !window.__TAURI__.

use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

const DEFAULT_PORT: u16 = 1430;

#[derive(Debug, Deserialize)]
struct InvokeRequest {
    cmd: String,
    #[serde(default)]
    args: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct InvokeError {
    error: String,
}

/// Run the dev bridge server (blocking). Call from a spawned thread.
pub async fn run() {
    let port = std::env::var("KALAM_DEV_BRIDGE_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/health", get(|| async { (StatusCode::OK, "ok") }))
        .route("/api/invoke", post(handle_invoke))
        .layer(cors);

    log::info!(
        "Dev bridge listening on http://{} (browser can use this for data)",
        addr
    );
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            log::warn!(
                "Dev bridge bind failed: {}. Browser dev will not get DB data.",
                e
            );
            panic!("Dev bridge bind failed: {}", e);
        });
    axum::serve(listener, app)
        .await
        .expect("Dev bridge server error");
}

/// Response headers to prevent browser caching of API results (tasks/reminders must stay fresh).
fn no_cache_headers() -> [(axum::http::HeaderName, &'static str); 2] {
    [
        (axum::http::header::CACHE_CONTROL, "no-store, no-cache"),
        (axum::http::header::PRAGMA, "no-cache"),
    ]
}

async fn handle_invoke(Json(req): Json<InvokeRequest>) -> impl IntoResponse {
    let result = dispatch(&req.cmd, &req.args).await;
    let headers = no_cache_headers();
    match result {
        Ok(value) => {
            let mut res = (StatusCode::OK, axum::Json(value)).into_response();
            for (k, v) in headers {
                let h = axum::http::HeaderValue::from_static(v);
                res.headers_mut().insert(k, h);
            }
            res
        }
        Err(e) => {
            let mut res = (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(InvokeError {
                    error: e.to_string(),
                }),
            )
                .into_response();
            for (k, v) in headers {
                let h = axum::http::HeaderValue::from_static(v);
                res.headers_mut().insert(k, h);
            }
            res
        }
    }
}

/// Get a key from args, or from args.args (frontend sometimes sends { args: { ... } }).
fn arg_get<'a>(args: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    args.get(key)
        .or_else(|| args.get("args").and_then(|a| a.get(key)))
}

async fn dispatch(cmd: &str, args: &serde_json::Value) -> Result<serde_json::Value, String> {
    match cmd {
        "get_settings" => {
            let config = crate::config::ConfigManager::new().map_err(|e| e.to_string())?;
            let cfg = config.get_all();
            serde_json::to_value(cfg).map_err(|e| e.to_string())
        }
        "get_platform" => Ok(serde_json::Value::String(platform())),
        "get_os_release_info" => {
            serde_json::to_value(crate::read_os_release_info()).map_err(|e| e.to_string())
        }
        "get_db_status" => {
            let ok = crate::db::open_db().is_ok();
            serde_json::to_value(serde_json::json!({ "ok": ok })).map_err(|e| e.to_string())
        }
        "get_aggregate_stats" => {
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            let stats = crate::db::get_aggregate_stats(&conn).map_err(|e| e.to_string())?;
            serde_json::to_value(stats).map_err(|e| e.to_string())
        }
        "get_dashboard_stats" => {
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            let stats = crate::db::get_dashboard_stats(&conn).map_err(|e| e.to_string())?;
            serde_json::to_value(stats).map_err(|e| e.to_string())
        }
        "get_history" => {
            let limit = arg_get(args, "limit")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);
            let offset = arg_get(args, "offset")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);
            let entries = crate::history::get_history(limit, offset)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_value(entries).map_err(|e| e.to_string())
        }
        "search_history" => {
            let query = arg_get(args, "query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let entries = crate::history::search(&query)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_value(entries).map_err(|e| e.to_string())
        }
        "clear_history" => {
            crate::history::clear().await.map_err(|e| e.to_string())?;
            Ok(serde_json::Value::Null)
        }
        "get_tasks_due_on" => {
            let day_start = arg_get(args, "dayStart")
                .and_then(|v| v.as_str())
                .map(String::from)
                .ok_or_else(|| "missing dayStart".to_string())?;
            let day_end = arg_get(args, "dayEnd")
                .and_then(|v| v.as_str())
                .map(String::from)
                .ok_or_else(|| "missing dayEnd".to_string())?;
            let limit = arg_get(args, "limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(50) as i64;
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            let entries = crate::db::get_tasks_due_on(&conn, &day_start, &day_end, limit)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(entries).map_err(|e| e.to_string())
        }
        "get_reminders_due_on" => {
            let day_start = arg_get(args, "dayStart")
                .and_then(|v| v.as_str())
                .map(String::from)
                .ok_or_else(|| "missing dayStart".to_string())?;
            let day_end = arg_get(args, "dayEnd")
                .and_then(|v| v.as_str())
                .map(String::from)
                .ok_or_else(|| "missing dayEnd".to_string())?;
            let limit = arg_get(args, "limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(50) as i64;
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            let entries = crate::db::get_reminders_due_on(&conn, &day_start, &day_end, limit)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(entries).map_err(|e| e.to_string())
        }
        "get_snippets" => {
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            let entries = crate::db::get_entries_by_type(&conn, "snippet", None, 500, 0)
                .map_err(|e| e.to_string())?;
            let snippets: Vec<crate::config::Snippet> = entries
                .into_iter()
                .map(|e| crate::config::Snippet {
                    trigger: e.title.unwrap_or_default(),
                    expansion: e.content,
                })
                .collect();
            serde_json::to_value(snippets).map_err(|e| e.to_string())
        }
        "get_entries_by_type" => {
            let entry_type = arg_get(args, "entryType")
                .or_else(|| arg_get(args, "entry_type"))
                .and_then(|v| v.as_str())
                .map(String::from)
                .ok_or_else(|| "missing entryType".to_string())?;
            let scope = arg_get(args, "scope")
                .and_then(|v| v.as_str())
                .map(String::from);
            let limit = arg_get(args, "limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(100);
            let offset = arg_get(args, "offset")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            let entries =
                crate::db::get_entries_by_type(&conn, &entry_type, scope.as_deref(), limit, offset)
                    .map_err(|e| e.to_string())?;
            serde_json::to_value(entries).map_err(|e| e.to_string())
        }
        "get_entries_with_reminder" => {
            let limit = arg_get(args, "limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(200);
            let offset = arg_get(args, "offset")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            let entries = crate::db::get_entries_with_reminder(&conn, limit, offset)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(entries).map_err(|e| e.to_string())
        }
        "get_entries_for_reminders" => {
            let limit = arg_get(args, "limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(200);
            let offset = arg_get(args, "offset")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            let entries = crate::db::get_entries_for_reminders_view(&conn, limit, offset)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(entries).map_err(|e| e.to_string())
        }
        "search_notes" => {
            let query = arg_get(args, "query")
                .and_then(|v| v.as_str())
                .map(String::from);
            let label = arg_get(args, "label")
                .and_then(|v| v.as_str())
                .map(String::from);
            let scope = arg_get(args, "scope")
                .and_then(|v| v.as_str())
                .unwrap_or("active")
                .to_string();
            let limit = arg_get(args, "limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(100);
            let offset = arg_get(args, "offset")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            let entries = crate::db::search_notes(
                &conn,
                query.as_deref(),
                label.as_deref(),
                &scope,
                limit,
                offset,
            )
            .map_err(|e| e.to_string())?;
            serde_json::to_value(entries).map_err(|e| e.to_string())
        }
        "get_note_labels" => {
            let scope = arg_get(args, "scope")
                .and_then(|v| v.as_str())
                .map(String::from);
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            let labels =
                crate::db::get_note_labels(&conn, scope.as_deref()).map_err(|e| e.to_string())?;
            serde_json::to_value(labels).map_err(|e| e.to_string())
        }
        "get_note_scope_counts" => {
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            let (active, archived, trash) =
                crate::db::count_notes_by_scope(&conn).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "active": active, "archived": archived, "trash": trash }))
        }
        "empty_task_trash" => {
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            let n = crate::db::empty_task_trash(&conn).map_err(|e| e.to_string())?;
            serde_json::to_value(n).map_err(|e| e.to_string())
        }
        "get_dictionary_entries" => {
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            let entries = crate::db::get_dictionary_entries(&conn).map_err(|e| e.to_string())?;
            serde_json::to_value(entries).map_err(|e| e.to_string())
        }
        "update_dictionary_entry" => {
            let id = arg_get(args, "id")
                .and_then(|v| v.as_str())
                .map(String::from)
                .ok_or_else(|| "missing id".to_string())?;
            let term = arg_get(args, "term")
                .and_then(|v| v.as_str())
                .map(String::from)
                .ok_or_else(|| "missing term".to_string())?;
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            crate::db::update_dictionary_entry(&conn, &id, &term).map_err(|e| e.to_string())?;
            Ok(serde_json::Value::Null)
        }
        "focus_main_window" => Ok(serde_json::Value::Null),
        "get_entry" => {
            let id = arg_get(args, "id")
                .and_then(|v| v.as_str())
                .map(String::from)
                .ok_or_else(|| "missing id".to_string())?;
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            let entry = crate::db::get_entry(&conn, &id).map_err(|e| e.to_string())?;
            serde_json::to_value(entry).map_err(|e| e.to_string())
        }
        "update_entry" => {
            let entry_val = args
                .get("entry")
                .ok_or_else(|| "missing entry".to_string())?;
            let entry: crate::models::Entry = serde_json::from_value(entry_val.clone())
                .map_err(|e| format!("update_entry: {e}"))?;
            let conn = crate::db::open_db().map_err(|e| e.to_string())?;
            let ok = crate::db::update_entry(&conn, &entry).map_err(|e| e.to_string())?;
            serde_json::to_value(ok).map_err(|e| e.to_string())
        }
        _ => Err(format!("Dev bridge does not implement command: {}", cmd)),
    }
}

fn platform() -> String {
    #[cfg(target_os = "windows")]
    return "windows".to_string();
    #[cfg(target_os = "macos")]
    return "macos".to_string();
    #[cfg(target_os = "linux")]
    return "linux".to_string();
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    "unknown".to_string()
}
