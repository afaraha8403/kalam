#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const HISTORY_FILE: &str = "history.json";
const MAX_HISTORY_ENTRIES: usize = 1000;

fn get_kalam_dir() -> anyhow::Result<PathBuf> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| anyhow::anyhow!("Could not find home directory"))?;
    
    let kalam_dir = PathBuf::from(home).join(".kalam");
    fs::create_dir_all(&kalam_dir)?;
    Ok(kalam_dir)
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

#[derive(Debug, Serialize, Deserialize)]
struct HistoryStore {
    entries: Vec<HistoryEntry>,
}

impl Default for HistoryStore {
    fn default() -> Self {
        Self { entries: Vec::new() }
    }
}

fn get_history_path() -> anyhow::Result<PathBuf> {
    let kalam_dir = get_kalam_dir()?;
    Ok(kalam_dir.join(HISTORY_FILE))
}

fn load_history() -> anyhow::Result<HistoryStore> {
    let path = get_history_path()?;
    
    if path.exists() {
        let contents = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&contents).unwrap_or_default())
    } else {
        Ok(HistoryStore::default())
    }
}

fn save_history(store: &HistoryStore) -> anyhow::Result<()> {
    let path = get_history_path()?;
    let json = serde_json::to_string_pretty(store)?;
    fs::write(&path, json)?;
    Ok(())
}

pub async fn save_transcription(text: &str) -> anyhow::Result<()> {
    let mut store = load_history()?;
    
    let entry = HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        text: text.to_string(),
        created_at: Utc::now(),
        mode: "cloud".to_string(),
        language: "auto".to_string(),
        duration_ms: None,
    };
    
    store.entries.insert(0, entry);
    
    // Keep only the most recent entries
    if store.entries.len() > MAX_HISTORY_ENTRIES {
        store.entries.truncate(MAX_HISTORY_ENTRIES);
    }
    
    save_history(&store)
}

pub async fn get_history(
    limit: Option<u32>,
    offset: Option<u32>,
) -> anyhow::Result<Vec<HistoryEntry>> {
    let store = load_history()?;
    let limit = limit.unwrap_or(50) as usize;
    let offset = offset.unwrap_or(0) as usize;
    
    let entries: Vec<HistoryEntry> = store.entries
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    
    Ok(entries)
}

pub async fn search(query: &str) -> anyhow::Result<Vec<HistoryEntry>> {
    let store = load_history()?;
    let query_lower = query.to_lowercase();
    
    let entries: Vec<HistoryEntry> = store.entries
        .into_iter()
        .filter(|e| e.text.to_lowercase().contains(&query_lower))
        .collect();
    
    Ok(entries)
}

pub async fn clear() -> anyhow::Result<()> {
    let store = HistoryStore::default();
    save_history(&store)
}

pub async fn export(format: &str) -> anyhow::Result<String> {
    let entries = get_history(None, None).await?;

    match format {
        "json" => {
            Ok(serde_json::to_string_pretty(&entries)?)
        }
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
