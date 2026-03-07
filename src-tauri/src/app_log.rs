//! In-app log buffer for user-exportable logs. No PII/sensitive data should ever be logged
//! to this buffer (see plan: no transcription text, no API keys, no response bodies).

use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use crate::config::LoggingConfig;

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

use once_cell::sync::Lazy;

static APP_LOG_STATE: Lazy<Mutex<AppLogState>> = Lazy::new(|| {
    Mutex::new(AppLogState {
        buffer: VecDeque::new(),
        config: LoggingConfig::default(),
    })
});

const STDERR_FILTER: log::LevelFilter = log::LevelFilter::Info;

fn push_to_buffer_if_enabled(record_level: log::Level, line: &str) {
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

        push_to_buffer_if_enabled(record.level(), &line);
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
