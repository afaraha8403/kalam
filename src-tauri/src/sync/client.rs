use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

pub fn api_base_url(recipe_library_url: &str) -> String {
    recipe_library_url.trim_end_matches('/').to_string()
}

#[derive(Debug, Serialize, Clone)]
pub struct PushItem {
    pub bucket: String,
    pub id: String,
    pub payload: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct PushResponse {
    pub accepted: u32,
    pub conflicts: u32,
    pub server_time: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PullItem {
    pub id: String,
    pub bucket: String,
    pub payload: String,
    pub updated_at: String,
    #[serde(default)]
    pub deleted: i32,
}

#[derive(Debug, Deserialize)]
pub struct PullResponse {
    pub items: Vec<PullItem>,
    pub has_more: bool,
    pub server_time: String,
}

#[derive(Debug, Deserialize)]
pub struct LicenseValidateBody {
    pub valid: bool,
    #[serde(default)]
    pub plan: Option<String>,
}

pub async fn validate_license_ok(
    client: &reqwest::Client,
    base: &str,
    key: &str,
) -> Result<bool, String> {
    let url = format!(
        "{}/api/license/validate?key={}",
        base,
        urlencoding::encode(key)
    );
    let res = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("license_validate_http_{}", res.status()));
    }
    let body: LicenseValidateBody = res.json().await.map_err(|e| e.to_string())?;
    let plan_ok = matches!(
        body.plan.as_deref(),
        Some("trial") | Some("pro_monthly") | Some("pro_yearly")
    );
    Ok(body.valid && plan_ok)
}

pub async fn push_batch(
    client: &reqwest::Client,
    base: &str,
    key: &str,
    items: Vec<PushItem>,
) -> Result<PushResponse, String> {
    let url = format!("{}/api/sync/push", base);
    let res = client
        .post(&url)
        .header(AUTHORIZATION, format!("Bearer {}", key.trim()))
        .header(CONTENT_TYPE, "application/json")
        .json(&serde_json::json!({ "items": items }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("push_{}: {}", status, text));
    }
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub async fn pull_page(
    client: &reqwest::Client,
    base: &str,
    key: &str,
    since: &str,
    after_id: &str,
) -> Result<PullResponse, String> {
    let url = format!(
        "{}/api/sync/pull?since={}&after_id={}",
        base,
        urlencoding::encode(since),
        urlencoding::encode(after_id)
    );
    let res = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {}", key.trim()))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("pull_{}: {}", status, text));
    }
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub async fn reset_remote(client: &reqwest::Client, base: &str, key: &str) -> Result<(), String> {
    let url = format!("{}/api/sync/reset", base);
    let res = client
        .post(&url)
        .header(AUTHORIZATION, format!("Bearer {}", key.trim()))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("reset_{}: {}", status, text));
    }
    Ok(())
}
