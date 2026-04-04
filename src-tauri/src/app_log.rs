//! In-app logging: writes to in-memory buffer (for UI snapshot) and to SQLite `logs` table.
//! No PII/sensitive data should ever be logged (no transcription text, no API keys, no response bodies).

use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use chrono::{Local, NaiveTime, TimeZone, Utc};
use once_cell::sync::Lazy;

use crate::config::{LogLevel, LoggingConfig};
use crate::db;
use regex::Regex;

static INITIALIZED: AtomicBool = AtomicBool::new(false);

struct AppLogState {
    buffer: VecDeque<String>,
    config: LoggingConfig,
}

fn format_timestamp() -> String {
    let now = SystemTime::now();
    let duration = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    let mins = (secs / 60) % 60;
    let hours = (secs / 3600) % 24;
    let secs_rem = secs % 60;
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs_rem, millis)
}

static APP_LOG_STATE: Lazy<Mutex<AppLogState>> = Lazy::new(|| {
    Mutex::new(AppLogState {
        buffer: VecDeque::new(),
        config: LoggingConfig::default(),
    })
});

const STDERR_FILTER: log::LevelFilter = log::LevelFilter::Info;

/// Write directly to stderr, bypassing the log framework. Used when the
/// log mutex is poisoned or when diagnosing logger failures.
fn stderr_fallback(msg: &str) {
    let _ = writeln!(std::io::stderr(), "[app_log FALLBACK] {}", msg);
}

fn push_to_buffer_and_db(
    record_level: log::Level,
    line: &str,
    message: &std::fmt::Arguments,
    target: &str,
) {
    let mut state = match APP_LOG_STATE.lock() {
        Ok(s) => s,
        Err(poisoned) => {
            stderr_fallback(&format!("Log mutex poisoned, recovering: {}", line));
            // Recover the inner state so logging doesn't permanently die.
            poisoned.into_inner()
        }
    };
    if !state.config.enabled {
        return;
    }
    let Some(min_level) = state.config.level.to_log_filter() else {
        return;
    };
    if record_level > min_level {
        return;
    }
    state.buffer.push_back(line.to_string());
    let max = state.config.max_records as usize;
    while state.buffer.len() > max {
        state.buffer.pop_front();
    }
    // Format message and release the lock before DB I/O to reduce contention.
    let msg = message.to_string();
    drop(state);
    if let Ok(conn) = db::open_db() {
        let id = uuid::Uuid::new_v4().to_string();
        let level = record_level.to_string();
        let ts = Utc::now().to_rfc3339();
        let _ = conn.execute(
            "INSERT INTO logs (id, level, message, module, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, level, msg, target, ts],
        );
    }
}

struct AppLoggerStatic {
    stderr_filter: log::LevelFilter,
}

impl log::Log for AppLoggerStatic {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let line = format!(
            "{} {} [{}] {}",
            format_timestamp(),
            record.level(),
            record.target(),
            record.args()
        );

        if record.level() <= self.stderr_filter {
            let _ = writeln!(std::io::stderr(), "{}", line);
        }

        push_to_buffer_and_db(record.level(), &line, record.args(), record.target());
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}

static LOGGER: AppLoggerStatic = AppLoggerStatic {
    stderr_filter: STDERR_FILTER,
};

/// Try to load just the logging config from the config file on disk.
/// Falls back to `default_config` on any failure (missing file, parse error, etc.).
pub fn load_logging_config_from_disk(default_config: LoggingConfig) -> LoggingConfig {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    if home.is_empty() {
        return default_config;
    }
    let config_path = std::path::PathBuf::from(home)
        .join(".kalam")
        .join("config.json");
    let contents = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return default_config,
    };
    // Parse just enough to extract the logging section.
    let value: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(v) => v,
        Err(_) => return default_config,
    };
    let Some(logging_value) = value.get("logging") else {
        return default_config;
    };
    serde_json::from_value::<LoggingConfig>(logging_value.clone()).unwrap_or(default_config)
}

/// Initialize the app log and set it as the global logger.
/// Reads the user's config file to pick up persisted logging settings so that
/// logs captured during early startup (before full config load) are not lost.
/// Call once at startup from main().
pub fn init(default_config: LoggingConfig) {
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return;
    }
    let effective_config = load_logging_config_from_disk(default_config);
    {
        let mut state = APP_LOG_STATE.lock().unwrap();
        state.config = effective_config.clone();
        state.buffer.clear();
    }
    if log::set_logger(&LOGGER).is_err() {
        stderr_fallback(
            "CRITICAL: log::set_logger failed — another logger was already registered. \
             In-app logging will not work.",
        );
    }
    log::set_max_level(log::LevelFilter::Debug);
    // Confirm logging state at startup so it's visible in the buffer and stderr.
    if effective_config.enabled {
        log::info!(
            "App logging active at startup (level={:?}, max_records={})",
            effective_config.level,
            effective_config.max_records
        );
    } else {
        let _ = writeln!(
            std::io::stderr(),
            "[app_log] Logging disabled in config. Enable in Settings > Advanced."
        );
    }
}

/// Reconfigure in-app logging (e.g. after user changes settings).
/// Trims the buffer if max_records is reduced.
pub fn reconfigure(config: LoggingConfig) {
    let mut state = match APP_LOG_STATE.lock() {
        Ok(s) => s,
        Err(poisoned) => {
            stderr_fallback("Log mutex poisoned during reconfigure, recovering");
            poisoned.into_inner()
        }
    };
    let was_enabled = state.config.enabled;
    state.config = config;
    let max = state.config.max_records as usize;
    while state.buffer.len() > max {
        state.buffer.pop_front();
    }
    // Emit a confirmation so the first log entry proves reconfigure worked.
    if state.config.enabled && !was_enabled {
        let line = format!(
            "{} INFO [app_log] Logging enabled (level={:?}, max_records={})",
            format_timestamp(),
            state.config.level,
            state.config.max_records
        );
        state.buffer.push_back(line);
    }
}

/// Return the current effective logging config (for diagnostic UI).
pub fn current_config() -> LoggingConfig {
    match APP_LOG_STATE.lock() {
        Ok(s) => s.config.clone(),
        Err(poisoned) => poisoned.into_inner().config.clone(),
    }
}

/// Return whether the log buffer is empty (no entries to download).
pub fn is_empty() -> bool {
    let state = match APP_LOG_STATE.lock() {
        Ok(s) => s,
        Err(poisoned) => poisoned.into_inner(),
    };
    state.buffer.is_empty()
}

/// Return the current log buffer as a single string (newline-separated lines).
pub fn get_snapshot() -> String {
    let state = match APP_LOG_STATE.lock() {
        Ok(s) => s,
        Err(poisoned) => poisoned.into_inner(),
    };
    state.buffer.iter().cloned().collect::<Vec<_>>().join("\n")
}

/// Export all logs from the database as CSV. Returns the CSV string and the suggested filename.
pub fn export_logs_csv() -> anyhow::Result<(String, String)> {
    let conn = db::open_db()?;
    let mut stmt = conn
        .prepare("SELECT id, level, message, module, timestamp FROM logs ORDER BY timestamp ASC")?;
    let mut csv = String::from("id,level,message,module,timestamp\n");
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;
    for row in rows {
        let (id, level, message, module, timestamp) = row?;
        let message_escaped = message.replace('"', "\"\"");
        csv.push_str(&format!(
            "{},\"{}\",\"{}\",\"{}\",\"{}\"\n",
            id, level, message_escaped, module, timestamp
        ));
    }
    let filename = format!(
        "kalam-logs-{}.csv",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    Ok((csv, filename))
}

/// Temporarily force in-app logging to enabled + Debug (diagnostic test runs).
/// Pair with [`deescalate`] so the user's normal logging preference is restored in memory
/// (persisted `config.json` is unchanged unless they save settings).
pub fn escalate() -> LoggingConfig {
    let prev = current_config();
    reconfigure(LoggingConfig {
        enabled: true,
        level: LogLevel::Debug,
        max_records: prev.max_records.max(2000),
    });
    prev
}

/// Restore logging after [`escalate`].
pub fn deescalate(prev: LoggingConfig) {
    reconfigure(prev);
}

static LOG_LINE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\d{2}:\d{2}:\d{2}\.\d{3})\s+(\w+)\s+\[([^\]]*)\]\s*(.*)$")
        .expect("log line regex")
});

/// Last `max` lines from the in-memory buffer as structured rows for markdown reports.
/// Matches the format written by [`AppLoggerStatic::log`].
pub fn recent_log_entries(max: usize) -> Vec<crate::diagnostics::LogEntry> {
    let state = match APP_LOG_STATE.lock() {
        Ok(s) => s,
        Err(poisoned) => poisoned.into_inner(),
    };
    let mut out = Vec::new();
    for line in state.buffer.iter().rev().take(max).rev() {
        if let Some(entry) = parse_log_buffer_line(line) {
            out.push(entry);
        }
    }
    out
}

fn parse_log_buffer_line(line: &str) -> Option<crate::diagnostics::LogEntry> {
    let caps = LOG_LINE_RE.captures(line)?;
    let time_s = caps.get(1)?.as_str();
    let level = caps.get(2)?.as_str().to_string();
    let message = caps.get(4)?.as_str().to_string();
    let naive_time = NaiveTime::parse_from_str(time_s, "%H:%M:%S%.3f").ok()?;
    let date = Local::now().date_naive();
    let naive_dt = date.and_time(naive_time);
    let timestamp = match Local.from_local_datetime(&naive_dt) {
        chrono::LocalResult::Single(dt) => dt,
        chrono::LocalResult::Ambiguous(dt, _) => dt,
        chrono::LocalResult::None => Local::now(),
    };
    Some(crate::diagnostics::LogEntry {
        timestamp,
        level,
        message,
    })
}
