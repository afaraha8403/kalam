use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    History,
    Note,
    Task,
    Reminder,
    Snippet,
}

#[allow(dead_code)]
impl EntryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntryType::History => "history",
            EntryType::Note => "note",
            EntryType::Task => "task",
            EntryType::Reminder => "reminder",
            EntryType::Snippet => "snippet",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "note" => EntryType::Note,
            "task" => EntryType::Task,
            "reminder" => EntryType::Reminder,
            "snippet" => EntryType::Snippet,
            _ => EntryType::History,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    pub title: String,
    pub is_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub entry_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub sync_status: String,
    pub title: Option<String>,
    pub content: String,
    pub attachments: Vec<String>,
    pub tags: Vec<String>,
    pub color: Option<String>,
    pub is_pinned: bool,
    pub priority: Option<u8>,
    pub due_date: Option<DateTime<Utc>>,
    pub subtasks: Option<Vec<Subtask>>,
    pub is_completed: Option<bool>,
    pub reminder_at: Option<DateTime<Utc>>,
    pub rrule: Option<String>,
    pub archived_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    /// Process name (e.g. notepad.exe) of the foreground app when dictation completed; dashboard only.
    pub target_app: Option<String>,
    /// Recorded audio length in milliseconds (dictation sessions).
    #[serde(default, alias = "durationMs")]
    pub duration_ms: Option<u32>,
    /// Word count at save time (history rows); avoids recount drift if formatting rules change.
    #[serde(default, alias = "wordCount")]
    pub word_count: Option<u32>,
    /// Wall-clock STT latency for this session (ms).
    #[serde(default, alias = "sttLatencyMs")]
    pub stt_latency_ms: Option<u32>,
    /// Effective STT mode label: Cloud, Local, Hybrid, Auto.
    #[serde(default, alias = "sttMode")]
    pub stt_mode: Option<String>,
    /// Primary recognition language code(s) from settings at capture time.
    pub dictation_language: Option<String>,
    /// `dictation` vs `command` session (command transcripts are not injected as dictation).
    pub session_mode: Option<String>,
    /// STT provider id at capture time (e.g. groq, sensevoice); pairs with `stt_mode`.
    #[serde(default, alias = "sttProvider")]
    pub stt_provider: Option<String>,
}
