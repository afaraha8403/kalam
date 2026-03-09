use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppLogRow {
    pub id: String,
    pub level: String,
    pub message: String,
    pub module: String,
    pub timestamp: DateTime<Utc>,
}
