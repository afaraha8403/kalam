#![allow(dead_code)]

pub mod groq;
pub mod openai;
pub mod models;
pub mod provider;
pub mod sensevoice;
pub mod lifecycle;

use crate::audio::vad::{VADConfig, VADProcessor};
use crate::stt::provider::STTProvider;

/// Overlap between consecutive chunks in samples (0.5s at 16kHz).
const CHUNK_OVERLAP_SAMPLES: usize = 8000;

#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f32,
    pub language: String,
}

pub struct STTManager;

impl STTManager {
    pub fn new() -> Self {
        Self
    }
}

/// Transcribe audio using VAD-based chunking and prompt chaining for context.
/// Chunks are built from VAD segments with 0.5s overlap; each chunk is sent with
/// the previous chunk's text as prompt for consistent capitalization/context.
/// If `language_hint` is Some (e.g. "en"), the provider may use it for the API.
pub fn transcribe_chunked(
    provider: &dyn STTProvider,
    audio: &[f32],
    sample_rate: u32,
    vad_config: &VADConfig,
    language_hint: Option<&str>,
) -> anyhow::Result<TranscriptionResult> {
    let mut vad = VADProcessor::new(vad_config.clone())?;
    let segments = vad.process(audio);

    if segments.is_empty() {
        return provider.transcribe_blocking(audio, sample_rate, None, language_hint);
    }

    let mut combined_text = String::new();
    let mut prev_text: Option<String> = None;
    let mut confidence_sum = 0.0f32;
    let mut count = 0usize;
    let mut language = String::new();

    for (start, end) in segments {
        let chunk_start = start.saturating_sub(CHUNK_OVERLAP_SAMPLES);
        let chunk = &audio[chunk_start..end];
        if chunk.len() < 1600 {
            continue;
        }
        let prompt = prev_text.as_deref();
        let result = provider.transcribe_blocking(chunk, sample_rate, prompt, language_hint)?;
        let t = result.text.trim();
        if !t.is_empty() {
            if !combined_text.is_empty() {
                combined_text.push(' ');
            }
            combined_text.push_str(t);
            confidence_sum += result.confidence;
            count += 1;
            if !result.language.is_empty() && language.is_empty() {
                language = result.language;
            }
            prev_text = Some(result.text);
        }
    }

    Ok(TranscriptionResult {
        text: combined_text,
        confidence: if count > 0 { confidence_sum / count as f32 } else { 0.9 },
        language: if language.is_empty() { "unknown".to_string() } else { language },
    })
}

pub async fn validate_api_key(provider: &str, api_key: &str) -> anyhow::Result<bool> {
    match provider {
        "groq" => groq::validate_key(api_key).await,
        "openai" => openai::validate_key(api_key).await,
        _ => Ok(false),
    }
}
