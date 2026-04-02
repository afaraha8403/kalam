//! Curated STT/LLM provider and model catalog (Phase 2). Static data for the UI; config still stores provider ids + model ids.

use serde::{Deserialize, Serialize};

/// What the provider can do (voice vs language model).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Capability {
    Stt,
    Llm,
}

/// How the provider expects API keys to be sent (for docs / future use).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthStyle {
    Bearer,
    XApiKey,
    QueryParam,
}

/// One recommended model row in the library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogModel {
    pub id: String,
    pub name: String,
    pub capability: Capability,
    pub speed_rating: u8,
    pub quality_rating: u8,
    pub cost_hint: String,
    pub is_default: bool,
    /// Approximate download size in MB for local STT weights (0 when not applicable).
    #[serde(default)]
    pub size_mb: u64,
}

/// One provider card in the library (cloud or local pseudo-provider).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogProvider {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub capabilities: Vec<Capability>,
    pub get_api_key_url: String,
    pub auth_style: AuthStyle,
    pub models: Vec<CatalogModel>,
}

/// Build the full catalog for IPC / JSON. Local entries mirror `stt::models::known_models`.
pub fn curated_catalog() -> Vec<CatalogProvider> {
    vec![
        CatalogProvider {
            id: "groq".into(),
            name: "Groq".into(),
            icon: "ph:lightning".into(),
            capabilities: vec![Capability::Stt, Capability::Llm],
            get_api_key_url: "https://console.groq.com/keys".into(),
            auth_style: AuthStyle::Bearer,
            models: vec![
                CatalogModel {
                    id: "whisper-large-v3-turbo".into(),
                    name: "Whisper Large V3 Turbo".into(),
                    capability: Capability::Stt,
                    speed_rating: 5,
                    quality_rating: 4,
                    cost_hint: "Free tier".into(),
                    is_default: true,
                    size_mb: 0,
                },
                CatalogModel {
                    id: "llama-3.3-70b-versatile".into(),
                    name: "Llama 3.3 70B".into(),
                    capability: Capability::Llm,
                    speed_rating: 5,
                    quality_rating: 3,
                    cost_hint: "Free tier".into(),
                    is_default: true,
                    size_mb: 0,
                },
            ],
        },
        CatalogProvider {
            id: "openai".into(),
            name: "OpenAI".into(),
            icon: "ph:open-ai-logo".into(),
            capabilities: vec![Capability::Stt, Capability::Llm],
            get_api_key_url: "https://platform.openai.com/api-keys".into(),
            auth_style: AuthStyle::Bearer,
            models: vec![
                CatalogModel {
                    id: "whisper-1".into(),
                    name: "Whisper".into(),
                    capability: Capability::Stt,
                    speed_rating: 3,
                    quality_rating: 5,
                    cost_hint: "~$0.006/min".into(),
                    is_default: true,
                    size_mb: 0,
                },
                CatalogModel {
                    id: "gpt-4.1-mini".into(),
                    name: "GPT-4.1 mini".into(),
                    capability: Capability::Llm,
                    speed_rating: 4,
                    quality_rating: 4,
                    cost_hint: "~$0.40/M tok".into(),
                    is_default: true,
                    size_mb: 0,
                },
            ],
        },
        CatalogProvider {
            id: "anthropic".into(),
            name: "Anthropic".into(),
            icon: "ph:brain".into(),
            capabilities: vec![Capability::Llm],
            get_api_key_url: "https://console.anthropic.com/settings/keys".into(),
            auth_style: AuthStyle::XApiKey,
            models: vec![CatalogModel {
                id: "claude-sonnet-4-20250514".into(),
                name: "Claude Sonnet 4".into(),
                capability: Capability::Llm,
                speed_rating: 3,
                quality_rating: 5,
                cost_hint: "~$3/M tok".into(),
                is_default: true,
                size_mb: 0,
            }],
        },
        CatalogProvider {
            id: "gemini".into(),
            name: "Google Gemini".into(),
            icon: "ph:google-logo".into(),
            capabilities: vec![Capability::Llm],
            get_api_key_url: "https://aistudio.google.com/apikey".into(),
            auth_style: AuthStyle::QueryParam,
            models: vec![CatalogModel {
                id: "gemini-2.5-flash".into(),
                name: "Gemini 2.5 Flash".into(),
                capability: Capability::Llm,
                speed_rating: 4,
                quality_rating: 4,
                cost_hint: "Free tier / usage limits".into(),
                is_default: true,
                size_mb: 0,
            }],
        },
        CatalogProvider {
            id: "openrouter".into(),
            name: "OpenRouter".into(),
            icon: "ph:globe".into(),
            capabilities: vec![Capability::Llm],
            get_api_key_url: "https://openrouter.ai/keys".into(),
            auth_style: AuthStyle::Bearer,
            // No curated default: user picks from live model list in Settings.
            models: vec![],
        },
        CatalogProvider {
            id: "local".into(),
            name: "Local".into(),
            icon: "ph:hard-drives".into(),
            capabilities: vec![Capability::Stt],
            get_api_key_url: String::new(),
            auth_style: AuthStyle::Bearer,
            models: crate::stt::models::known_models()
                .into_iter()
                .map(|m| CatalogModel {
                    id: m.id.to_string(),
                    name: match m.id {
                        "sensevoice" => "SenseVoice".into(),
                        "whisper_base" => "Whisper Base".into(),
                        _ => m.id.to_string(),
                    },
                    capability: Capability::Stt,
                    speed_rating: if m.id == "sensevoice" { 4 } else { 3 },
                    quality_rating: if m.id == "sensevoice" { 3 } else { 2 },
                    cost_hint: "Free (on-device)".into(),
                    is_default: m.id == "sensevoice",
                    size_mb: m.size_mb,
                })
                .collect(),
        },
    ]
}

/// Default LLM model id used to validate an API key for providers that expose chat (not used for STT-only).
pub fn default_llm_model_for_provider_test(provider: &str) -> Option<&'static str> {
    match provider.to_lowercase().as_str() {
        "groq" => Some("llama-3.3-70b-versatile"),
        "openai" => Some("gpt-4.1-mini"),
        "anthropic" => Some("claude-sonnet-4-20250514"),
        "gemini" => Some("gemini-2.5-flash"),
        // OpenRouter: small widely-available model for a minimal completion test.
        "openrouter" => Some("openai/gpt-4o-mini"),
        _ => None,
    }
}

#[tauri::command]
pub fn get_model_catalog() -> Vec<CatalogProvider> {
    curated_catalog()
}
