#![allow(dead_code)]

pub mod groq;
pub mod lifecycle;
pub mod models;
pub mod openai;
pub mod provider;
pub mod sensevoice;

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
/// If `vocabulary` is Some (e.g. dictionary terms), it is used as prompt for the first chunk only (cloud STT).
pub fn transcribe_chunked(
    provider: &dyn STTProvider,
    audio: &[f32],
    sample_rate: u32,
    vad_config: &VADConfig,
    language_hint: Option<&str>,
    vocabulary: Option<&str>,
) -> anyhow::Result<TranscriptionResult> {
    let mut vad = VADProcessor::new(vad_config.clone())?;
    let segments = vad.process(audio);

    if segments.is_empty() {
        let prompt = vocabulary;
        return provider.transcribe_blocking(audio, sample_rate, prompt, language_hint);
    }

    let mut combined_text = String::new();
    let mut prev_text: Option<String> = None;
    let mut confidence_sum = 0.0f32;
    let mut count = 0usize;
    let mut language = String::new();
    let mut combined_prompt_buf = String::new();

    for (start, end) in segments {
        let chunk_start = start.saturating_sub(CHUNK_OVERLAP_SAMPLES);
        let chunk = &audio[chunk_start..end];
        if chunk.len() < 1600 {
            continue;
        }
        // First chunk: vocabulary only. Later chunks: prev_text + vocabulary so custom words apply every segment.
        let prompt: Option<&str> = match (prev_text.as_deref(), vocabulary) {
            (None, Some(v)) => Some(v),
            (Some(prev), None) => Some(prev),
            (Some(prev), Some(v)) => {
                combined_prompt_buf.clear();
                combined_prompt_buf.push_str(prev);
                combined_prompt_buf.push_str(", ");
                combined_prompt_buf.push_str(v);
                if combined_prompt_buf.len() > 800 {
                    let keep_from = combined_prompt_buf.len().saturating_sub(800);
                    let tail: String = combined_prompt_buf.chars().skip(keep_from).collect();
                    combined_prompt_buf = tail.trim_start_matches([',', ' ']).to_string();
                }
                if combined_prompt_buf.is_empty() {
                    None
                } else {
                    Some(combined_prompt_buf.as_str())
                }
            }
            (None, None) => None,
        };
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
        confidence: if count > 0 {
            confidence_sum / count as f32
        } else {
            0.9
        },
        language: if language.is_empty() {
            "unknown".to_string()
        } else {
            language
        },
    })
}

pub async fn validate_api_key(provider: &str, api_key: &str) -> anyhow::Result<bool> {
    match provider {
        "groq" => groq::validate_key(api_key).await,
        "openai" => openai::validate_key(api_key).await,
        _ => Ok(false),
    }
}
