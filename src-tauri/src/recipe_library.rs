//! HTTP helpers for Phase 8 community recipe library (kalam-website Worker API).

use std::time::Duration;

const USER_AGENT: &str = concat!("Kalam/", env!("CARGO_PKG_VERSION"));

/// Strip whitespace and trailing slashes so we can append `/api/...` safely.
pub fn normalize_recipe_library_base(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

/// Shared client for recipe list + download (low frequency; no connection pooling requirement).
pub fn recipe_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .user_agent(USER_AGENT)
        .build()
        .expect("recipe library reqwest client")
}

/// GET `/api/recipes` — returns full JSON body (`recipes`, `total`, `page`).
pub async fn fetch_recipe_list(
    client: &reqwest::Client,
    base: &str,
    query: Option<&str>,
    category: Option<&str>,
    sort: &str,
    page: u32,
    limit: u32,
) -> Result<serde_json::Value, String> {
    let base = normalize_recipe_library_base(base);
    let url = format!("{}/api/recipes", base);
    let sort = if sort == "popular" {
        "popular"
    } else {
        "newest"
    };
    let mut pairs: Vec<(&str, String)> = vec![
        ("page", page.to_string()),
        ("limit", limit.to_string()),
        ("sort", sort.to_string()),
    ];
    if let Some(q) = query {
        let t = q.trim();
        if !t.is_empty() {
            pairs.push(("q", t.to_string()));
        }
    }
    if let Some(c) = category {
        let t = c.trim();
        if !t.is_empty() && t != "all" {
            pairs.push(("category", t.to_string()));
        }
    }
    let qref: Vec<(&str, &str)> = pairs.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let resp = client
        .get(&url)
        .query(&qref)
        .send()
        .await
        .map_err(|e| format!("recipe library request failed: {}", e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("recipe library error {}: {}", status, body));
    }
    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("recipe library bad JSON: {}", e))
}

/// POST `/api/recipes/{slug}/download` — increments server counter and returns `recipe_json` string.
pub async fn post_recipe_download_json(
    client: &reqwest::Client,
    base: &str,
    slug: &str,
) -> Result<String, String> {
    let base = normalize_recipe_library_base(base);
    let slug = slug.trim();
    if slug.is_empty() {
        return Err("missing slug".to_string());
    }
    let url = format!("{}/api/recipes/{}/download", base, slug);
    let resp = client
        .post(&url)
        .header("content-type", "application/json")
        .body("{}")
        .send()
        .await
        .map_err(|e| format!("recipe download failed: {}", e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("recipe download error {}: {}", status, body));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("recipe download bad JSON: {}", e))?;
    v.get("recipe_json")
        .and_then(|x| x.as_str())
        .map(String::from)
        .ok_or_else(|| "missing recipe_json in response".to_string())
}
