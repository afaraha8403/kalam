use crate::db;

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
