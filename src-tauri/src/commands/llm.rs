//! LLM provider integration for Command Mode: list models and generate structured JSON.

use serde::{Deserialize, Serialize};

const GROQ_BASE: &str = "https://api.groq.com/openai/v1";
const OPENROUTER_BASE: &str = "https://openrouter.ai/api/v1";
const OPENAI_BASE: &str = "https://api.openai.com/v1";
const ANTHROPIC_BASE: &str = "https://api.anthropic.com/v1";
const GEMINI_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

/// List model IDs for the given provider using its API key.
#[tauri::command]
pub async fn fetch_llm_models(provider: String, api_key: String) -> Result<Vec<String>, String> {
    if api_key.trim().is_empty() {
        return Err("API key is required".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    match provider.to_lowercase().as_str() {
        "groq" => fetch_openai_style_models(&client, GROQ_BASE, &api_key).await,
        "openrouter" => fetch_openrouter_models(&client, &api_key).await,
        "openai" => fetch_openai_style_models(&client, OPENAI_BASE, &api_key).await,
        "anthropic" => fetch_anthropic_models(&client, &api_key).await,
        "gemini" => fetch_gemini_models(&client, &api_key).await,
        _ => Err(format!("Unknown LLM provider: {}", provider)),
    }
}

#[derive(Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Deserialize)]
struct OpenAIModel {
    id: String,
}

async fn fetch_openai_style_models(
    client: &reqwest::Client,
    base: &str,
    api_key: &str,
) -> Result<Vec<String>, String> {
    let url = format!("{}/models", base);
    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, body));
    }

    let body: OpenAIModelsResponse = res.json().await.map_err(|e| e.to_string())?;
    Ok(body.data.into_iter().map(|m| m.id).collect())
}

#[derive(Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModel>,
}

#[derive(Deserialize)]
struct OpenRouterModel {
    id: String,
}

async fn fetch_openrouter_models(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<Vec<String>, String> {
    let url = format!("{}/models", OPENROUTER_BASE);
    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, body));
    }

    let body: OpenRouterModelsResponse = res.json().await.map_err(|e| e.to_string())?;
    Ok(body.data.into_iter().map(|m| m.id).collect())
}

#[derive(Deserialize)]
struct AnthropicModelsResponse {
    data: Vec<AnthropicModel>,
}

#[derive(Deserialize)]
struct AnthropicModel {
    id: String,
}

async fn fetch_anthropic_models(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<Vec<String>, String> {
    let url = format!("{}/models", ANTHROPIC_BASE);
    let res = client
        .get(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, body));
    }

    let body: AnthropicModelsResponse = res.json().await.map_err(|e| e.to_string())?;
    Ok(body.data.into_iter().map(|m| m.id).collect())
}

#[derive(Deserialize)]
struct GeminiModelsResponse {
    models: Option<Vec<GeminiModel>>,
}

#[derive(Deserialize)]
struct GeminiModel {
    name: String,
}

async fn fetch_gemini_models(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<Vec<String>, String> {
    let url = format!("{}/models?key={}", GEMINI_BASE, api_key);
    let res = client.get(&url).send().await.map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, body));
    }

    let body: GeminiModelsResponse = res.json().await.map_err(|e| e.to_string())?;
    let models = body.models.unwrap_or_default();
    Ok(models
        .into_iter()
        .map(|m| {
            if m.name.starts_with("models/") {
                m.name
            } else {
                format!("models/{}", m.name)
            }
        })
        .collect())
}

// --- Chat / generate structured data ---

/// Test if the model and API key are valid by sending a tiny prompt.
#[tauri::command]
pub async fn test_llm_model(
    provider: String,
    api_key: String,
    model: String,
) -> Result<String, String> {
    generate_structured_data(
        provider,
        api_key,
        model,
        "You are a test bot. Reply with {\"status\": \"ok\"} in JSON.".to_string(),
        "test".to_string(),
    )
    .await
}

/// Call the LLM with a system prompt and user text; return the assistant message content (e.g. JSON string).
#[tauri::command]
pub async fn generate_structured_data(
    provider: String,
    api_key: String,
    model: String,
    system_prompt: String,
    user_text: String,
) -> Result<String, String> {
    if api_key.trim().is_empty() || model.trim().is_empty() {
        return Err("API key and model are required".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    match provider.to_lowercase().as_str() {
        "groq" | "openrouter" | "openai" => {
            chat_openai_style(
                &client,
                &provider,
                &api_key,
                &model,
                &system_prompt,
                &user_text,
            )
            .await
        }
        "anthropic" => chat_anthropic(&client, &api_key, &model, &system_prompt, &user_text).await,
        "gemini" => chat_gemini(&client, &api_key, &model, &system_prompt, &user_text).await,
        _ => Err(format!("Unknown LLM provider: {}", provider)),
    }
}

/// Dictation polish / mode instructions: one LLM call returning plain text (no JSON schema).
pub async fn complete_plain_text(
    provider: String,
    api_key: String,
    model: String,
    system_prompt: String,
    user_text: String,
) -> Result<String, String> {
    if api_key.trim().is_empty() || model.trim().is_empty() {
        return Err("API key and model are required".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    match provider.to_lowercase().as_str() {
        "groq" | "openrouter" | "openai" => {
            chat_openai_style_plain(
                &client,
                &provider,
                &api_key,
                &model,
                &system_prompt,
                &user_text,
            )
            .await
        }
        "anthropic" => {
            chat_anthropic_plain(&client, &api_key, &model, &system_prompt, &user_text).await
        }
        "gemini" => chat_gemini_plain(&client, &api_key, &model, &system_prompt, &user_text).await,
        _ => Err(format!("Unknown LLM provider: {}", provider)),
    }
}

#[tauri::command]
pub async fn complete_plain_text_command(
    provider: String,
    api_key: String,
    model: String,
    system_prompt: String,
    user_text: String,
) -> Result<String, String> {
    complete_plain_text(provider, api_key, model, system_prompt, user_text).await
}

fn openai_style_base(provider: &str) -> &'static str {
    match provider.to_lowercase().as_str() {
        "groq" => GROQ_BASE,
        "openrouter" => OPENROUTER_BASE,
        _ => OPENAI_BASE,
    }
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<OpenAIResponseFormat>,
}

#[derive(Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OpenAIResponseFormat {
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Deserialize)]
struct OpenAICompletionResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

async fn chat_openai_style(
    client: &reqwest::Client,
    provider: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_text: &str,
) -> Result<String, String> {
    let base = openai_style_base(provider);
    post_openai_chat_completion(
        client,
        base,
        api_key,
        model,
        system_prompt,
        user_text,
        Some(OpenAIResponseFormat {
            type_: "json_object".to_string(),
        }),
    )
    .await
}

/// `base` is the OpenAI-compatible root including `/v1` when applicable (e.g. `https://api.openai.com/v1`).
async fn post_openai_chat_completion(
    client: &reqwest::Client,
    base: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_text: &str,
    response_format: Option<OpenAIResponseFormat>,
) -> Result<String, String> {
    let base = base.trim().trim_end_matches('/');
    let url = format!("{}/chat/completions", base);

    let body = OpenAIRequest {
        model: model.to_string(),
        messages: vec![
            OpenAIMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            OpenAIMessage {
                role: "user".to_string(),
                content: user_text.to_string(),
            },
        ],
        response_format,
    };

    let res = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }

    let data: OpenAICompletionResponse = res.json().await.map_err(|e| e.to_string())?;
    let content = data
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_default();
    Ok(content.trim().to_string())
}

async fn chat_openai_style_plain(
    client: &reqwest::Client,
    provider: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_text: &str,
) -> Result<String, String> {
    let base = openai_style_base(provider);
    post_openai_chat_completion(client, base, api_key, model, system_prompt, user_text, None).await
}

/// Custom OpenAI-compatible HTTPS endpoint (BYO base URL).
pub async fn complete_plain_text_openai_compatible(
    base_url: String,
    api_key: String,
    model: String,
    system_prompt: String,
    user_text: String,
) -> Result<String, String> {
    if api_key.trim().is_empty() || model.trim().is_empty() {
        return Err("API key and model are required".to_string());
    }
    let base = base_url.trim().trim_end_matches('/').to_string();
    if base.is_empty() {
        return Err("Base URL is required".to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    post_openai_chat_completion(
        &client,
        &base,
        &api_key,
        &model,
        &system_prompt,
        &user_text,
        None,
    )
    .await
}

/// JSON-object response (command mode) against a custom OpenAI-compatible base URL.
pub async fn generate_structured_data_openai_compatible(
    base_url: String,
    api_key: String,
    model: String,
    system_prompt: String,
    user_text: String,
) -> Result<String, String> {
    if api_key.trim().is_empty() || model.trim().is_empty() {
        return Err("API key and model are required".to_string());
    }
    let base = base_url.trim().trim_end_matches('/').to_string();
    if base.is_empty() {
        return Err("Base URL is required".to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    post_openai_chat_completion(
        &client,
        &base,
        &api_key,
        &model,
        &system_prompt,
        &user_text,
        Some(OpenAIResponseFormat {
            type_: "json_object".to_string(),
        }),
    )
    .await
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<AnthropicMessage>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    type_: String,
    text: Option<String>,
}

async fn chat_anthropic(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_text: &str,
) -> Result<String, String> {
    let url = format!("{}/messages", ANTHROPIC_BASE);

    let body = AnthropicRequest {
        model: model.to_string(),
        max_tokens: 4096,
        system: format!(
            "{}\n\nRespond with ONLY a valid JSON object, no markdown or explanation.",
            system_prompt
        ),
        messages: vec![AnthropicMessage {
            role: "user".to_string(),
            content: user_text.to_string(),
        }],
    };

    let res = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }

    let data: AnthropicResponse = res.json().await.map_err(|e| e.to_string())?;
    let text = data
        .content
        .into_iter()
        .find(|c| c.type_ == "text")
        .and_then(|c| c.text)
        .unwrap_or_default();
    Ok(text.trim().to_string())
}

async fn chat_anthropic_plain(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_text: &str,
) -> Result<String, String> {
    let url = format!("{}/messages", ANTHROPIC_BASE);

    let body = AnthropicRequest {
        model: model.to_string(),
        max_tokens: 4096,
        system: format!(
            "{}\n\nReply with plain text only (the final dictation result). No JSON, no preamble.",
            system_prompt
        ),
        messages: vec![AnthropicMessage {
            role: "user".to_string(),
            content: user_text.to_string(),
        }],
    };

    let res = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }

    let data: AnthropicResponse = res.json().await.map_err(|e| e.to_string())?;
    let text = data
        .content
        .into_iter()
        .find(|c| c.type_ == "text")
        .and_then(|c| c.text)
        .unwrap_or_default();
    Ok(text.trim().to_string())
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    generation_config: GeminiGenerationConfig,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GeminiGenerationConfig {
    response_mime_type: String,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiCandidateContent>,
}

#[derive(Deserialize)]
struct GeminiCandidateContent {
    parts: Option<Vec<GeminiPart>>,
}

async fn chat_gemini(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_text: &str,
) -> Result<String, String> {
    // API path is /v1beta/models/{name}:generateContent where name is e.g. "gemini-1.5-flash"
    let path_name = model.strip_prefix("models/").unwrap_or(model);
    let url = format!(
        "{}/models/{}:generateContent?key={}",
        GEMINI_BASE, path_name, api_key
    );

    let combined = format!(
        "{}\n\nUser input:\n{}\n\nRespond with ONLY a valid JSON object.",
        system_prompt, user_text
    );

    let body = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart { text: combined }],
        }],
        generation_config: GeminiGenerationConfig {
            response_mime_type: "application/json".to_string(),
        },
    };

    let res = client
        .post(&url)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }

    let data: GeminiResponse = res.json().await.map_err(|e| e.to_string())?;
    let text = data
        .candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .and_then(|p| p.into_iter().next())
        .map(|p| p.text)
        .unwrap_or_default();
    Ok(text.trim().to_string())
}

async fn chat_gemini_plain(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_text: &str,
) -> Result<String, String> {
    let path_name = model.strip_prefix("models/").unwrap_or(model);
    let url = format!(
        "{}/models/{}:generateContent?key={}",
        GEMINI_BASE, path_name, api_key
    );

    let combined = format!(
        "{}\n\nTranscript to process:\n{}\n\nOutput only the final plain text result.",
        system_prompt, user_text
    );

    let body = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart { text: combined }],
        }],
        generation_config: GeminiGenerationConfig {
            response_mime_type: "text/plain".to_string(),
        },
    };

    let res = client
        .post(&url)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }

    let data: GeminiResponse = res.json().await.map_err(|e| e.to_string())?;
    let text = data
        .candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .and_then(|p| p.into_iter().next())
        .map(|p| p.text)
        .unwrap_or_default();
    Ok(text.trim().to_string())
}
