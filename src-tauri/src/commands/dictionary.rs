use crate::db;
use regex::RegexBuilder;

#[tauri::command]
pub fn get_dictionary_entries() -> Result<Vec<db::DictionaryEntry>, String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::get_dictionary_entries(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_dictionary_entry(term: String) -> Result<String, String> {
    let term = term.trim();
    if term.is_empty() {
        return Err("Term cannot be empty".to_string());
    }
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::add_dictionary_entry(&conn, term).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_dictionary_entry(id: String) -> Result<(), String> {
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::delete_dictionary_entry(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_dictionary_entry(id: String, term: String) -> Result<(), String> {
    let term = term.trim();
    if term.is_empty() {
        return Err("Term cannot be empty".to_string());
    }
    let conn = db::open_db().map_err(|e| e.to_string())?;
    db::update_dictionary_entry(&conn, &id, term).map_err(|e| e.to_string())
}

/// Ensures `pattern` is non-empty; if `is_regex`, compiles with the same flags as the formatter (case-insensitive).
#[tauri::command]
pub fn validate_replacement_pattern(pattern: String, is_regex: bool) -> Result<(), String> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return Err("Word or phrase cannot be empty".to_string());
    }
    if !is_regex {
        return Ok(());
    }
    RegexBuilder::new(pattern)
        .case_insensitive(true)
        .build()
        .map(|_| ())
        .map_err(|e| format!("Invalid pattern: {e}"))
}
