//! In-app logging: writes to in-memory buffer (for UI snapshot) and to SQLite `logs` table.
//! No PII/sensitive data should ever be logged (no transcription text, no API keys, no response bodies).

use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use chrono::Utc;
use once_cell::sync::Lazy;

use crate::config::LoggingConfig;
use crate::db;

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

fn push_to_buffer_and_db(
    record_level: log::Level,
    line: &str,
    message: &std::fmt::Arguments,
    target: &str,
) {
    let mut state = match APP_LOG_STATE.lock() {
        Ok(s) => s,
        Err(_) => return,
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

/// Initialize the app log with default config and set it as the global logger.
/// Call once at startup (e.g. from main or start of run()).
pub fn init(default_config: LoggingConfig) {
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return;
    }
    {
        let mut state = APP_LOG_STATE.lock().unwrap();
        state.config = default_config;
        state.buffer.clear();
    }
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log::LevelFilter::Debug);
}

/// Reconfigure in-app logging (e.g. after user changes settings).
/// Trims the buffer if max_records is reduced.
pub fn reconfigure(config: LoggingConfig) {
    let mut state = match APP_LOG_STATE.lock() {
        Ok(s) => s,
        Err(_) => return,
    };
    state.config = config;
    let max = state.config.max_records as usize;
    while state.buffer.len() > max {
        state.buffer.pop_front();
    }
}

/// Return whether the log buffer is empty (no entries to download).
pub fn is_empty() -> bool {
    let state = match APP_LOG_STATE.lock() {
        Ok(s) => s,
        Err(_) => return true,
    };
    state.buffer.is_empty()
}

/// Return the current log buffer as a single string (newline-separated lines).
pub fn get_snapshot() -> String {
    let state = match APP_LOG_STATE.lock() {
        Ok(s) => s,
        Err(_) => return String::new(),
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
