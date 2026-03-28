#![allow(dead_code)]

pub mod groq;
pub mod lifecycle;
pub mod models;
pub mod openai;
pub mod provider;
pub mod sensevoice;
pub mod sidecars;

use crate::audio::vad::{VADConfig, VADProcessor};
use crate::stt::provider::STTProvider;
use std::fmt;

/// Structured error for transcription failures so callers can decide retry.
#[derive(Debug)]
pub enum TranscriptionError {
    /// Network/connection/timeout; retriable when transient.
    Network { retriable: bool },
    /// API returned HTTP status; 429 and 5xx are retriable.
    Api { status: u16, retriable: bool },
    /// Local server (e.g. SenseVoice/Whisper) issue.
    Local { server_down: bool },
    /// Unknown or non-classified error.
    Unknown(String),
}

impl fmt::Display for TranscriptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TranscriptionError::Network { retriable } => {
                write!(f, "Network error (retriable: {})", retriable)
            }
            TranscriptionError::Api { status, retriable } => {
                write!(f, "API error status {} (retriable: {})", status, retriable)
            }
            TranscriptionError::Local { server_down } => {
                write!(f, "Local server error (server_down: {})", server_down)
            }
            TranscriptionError::Unknown(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for TranscriptionError {}

impl TranscriptionError {
    /// True if the caller should retry (e.g. 429, 5xx, timeout).
    pub fn is_retriable(&self) -> bool {
        match self {
            TranscriptionError::Network { retriable } => *retriable,
            TranscriptionError::Api { retriable, .. } => *retriable,
            TranscriptionError::Local { server_down } => *server_down,
            TranscriptionError::Unknown(_) => false,
        }
    }
}

/// Overlap between consecutive chunks in samples (0.5s at 16kHz).
const CHUNK_OVERLAP_SAMPLES: usize = 8000;

/// Minimum max-chunk width (1s of audio) so forced splits stay meaningful.
fn max_chunk_samples(vad_config: &VADConfig, sample_rate: u32) -> usize {
    let sr = sample_rate.max(1) as f64;
    let raw = (vad_config.max_chunk_duration_sec * sr).round() as usize;
    raw.max(sample_rate as usize)
}

/// Enforce `max_chunk_duration_sec` on each VAD span.
/// Adjacent sub-segments overlap by `overlap` samples so the main loop’s
/// `chunk_start = start.saturating_sub(CHUNK_OVERLAP_SAMPLES)` still provides context at boundaries
/// without us double-applying overlap inside each sub-piece.
fn split_segments_by_max_duration(
    segments: Vec<(usize, usize)>,
    max_samples: usize,
    overlap: usize,
) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (s, e) in segments {
        if e <= s {
            continue;
        }
        let mut cur = s;
        while cur < e {
            let end_piece = (cur + max_samples).min(e);
            out.push((cur, end_piece));
            if end_piece >= e {
                break;
            }
            let next = end_piece.saturating_sub(overlap);
            // Guarantee forward progress if overlap is misconfigured vs max_samples.
            cur = next.max(cur + 1);
        }
    }
    out
}

/// True if `text` is effectively the vocabulary prompt echoed back (Whisper hallucination on silence).
fn is_prompt_echo(text: &str, vocabulary: &str) -> bool {
    let normalize = |s: &str| {
        let mut words: Vec<String> = s
            .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
            .map(|w| w.trim().to_lowercase())
            .filter(|w| !w.is_empty())
            .collect();
        words.sort_unstable();
        words
    };
    let t = normalize(text.trim());
    let v = normalize(vocabulary.trim());
    !t.is_empty() && t == v
}

/// Remove prompt leakage from cloud STT output:
/// - exact vocabulary echo (already handled by `is_prompt_echo`)
/// - repeated leading vocabulary prefix(s) before actual dictated text
fn sanitize_prompt_leakage(text: &str, vocabulary: Option<&str>) -> String {
    let mut cleaned = text.trim().to_string();
    let Some(vocab) = vocabulary.map(str::trim).filter(|v| !v.is_empty()) else {
        return cleaned;
    };

    if is_prompt_echo(&cleaned, vocab) {
        return String::new();
    }

    // Strip repeated exact vocabulary prefixes at the beginning, if present.
    loop {
        let lower_cleaned = cleaned.to_lowercase();
        let lower_vocab = vocab.to_lowercase();
        if !lower_cleaned.starts_with(&lower_vocab) {
            break;
        }
        cleaned = cleaned
            .get(vocab.len()..)
            .unwrap_or("")
            .trim_start_matches(|c: char| c.is_whitespace() || [',', ';', ':', '-'].contains(&c))
            .trim_start()
            .to_string();
        if cleaned.is_empty() {
            return cleaned;
        }
    }

    // If the text begins with 2+ comma-separated dictionary terms, strip that run.
    // This is conservative enough to avoid deleting normal prose starts.
    let vocab_terms: std::collections::HashSet<String> = vocab
        .split([',', ';', '\n'])
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if vocab_terms.len() >= 2 && cleaned.contains(',') {
        let parts: Vec<&str> = cleaned.split(',').collect();
        let mut leading_vocab_parts = 0usize;
        for p in &parts {
            let token = p.trim().to_lowercase();
            if token.is_empty() {
                break;
            }
            if vocab_terms.contains(&token) {
                leading_vocab_parts += 1;
            } else {
                break;
            }
        }
        if leading_vocab_parts >= 2 {
            if leading_vocab_parts >= parts.len() {
                return String::new();
            }
            cleaned = parts[leading_vocab_parts..]
                .join(",")
                .trim_start_matches(|c: char| {
                    c.is_whitespace() || [',', ';', ':', '-'].contains(&c)
                })
                .trim_start()
                .to_string();
        }
    }

    cleaned
}

#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f32,
    pub language: String,
}

pub struct STTManager;

impl Default for STTManager {
    fn default() -> Self {
        Self
    }
}

impl STTManager {
    pub fn new() -> Self {
        Self
    }
}

fn transcribe_windows(
    provider: &dyn STTProvider,
    audio: &[f32],
    sample_rate: u32,
    windows: &[(usize, usize)],
    language_hint: Option<&str>,
    vocabulary: Option<&str>,
) -> anyhow::Result<TranscriptionResult> {
    let mut combined_text = String::new();
    let mut prev_text: Option<String> = None;
    let mut confidence_sum = 0.0f32;
    let mut count = 0usize;
    let mut language = String::new();
    let mut combined_prompt_buf = String::new();

    for &(start, end) in windows {
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
        let cleaned_chunk = sanitize_prompt_leakage(&result.text, vocabulary);
        let t = cleaned_chunk.trim();
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
            // Chain sanitized text only, so prompt leakage does not snowball across chunks.
            prev_text = Some(t.to_string());
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
    let max_samples = max_chunk_samples(vad_config, sample_rate);
    let mut vad = VADProcessor::new(vad_config.clone())?;
    let raw_segments = vad.process(audio);
    let raw_len = raw_segments.len();

    // No VAD hits: one full-buffer call for short clips; time windows for long audio (same as splitting one span).
    if raw_segments.is_empty() {
        if audio.len() <= max_samples {
            let mut result =
                provider.transcribe_blocking(audio, sample_rate, vocabulary, language_hint)?;
            result.text = sanitize_prompt_leakage(&result.text, vocabulary);
            log::info!(
                "STT chunking: vad_segments=0 chunked_windows=1 empty_vad_timewindows=false (single_shot)"
            );
            return Ok(result);
        }
        let windows = split_segments_by_max_duration(
            vec![(0, audio.len())],
            max_samples,
            CHUNK_OVERLAP_SAMPLES,
        );
        log::info!(
            "STT chunking: vad_segments=0 chunked_windows={} empty_vad_timewindows=true",
            windows.len()
        );
        let mut result = transcribe_windows(
            provider,
            audio,
            sample_rate,
            &windows,
            language_hint,
            vocabulary,
        )?;
        result.text = sanitize_prompt_leakage(&result.text, vocabulary);
        return Ok(result);
    }

    let windows = split_segments_by_max_duration(raw_segments, max_samples, CHUNK_OVERLAP_SAMPLES);
    log::info!(
        "STT chunking: vad_segments={} chunked_windows={} empty_vad_timewindows=false",
        raw_len,
        windows.len()
    );

    transcribe_windows(
        provider,
        audio,
        sample_rate,
        &windows,
        language_hint,
        vocabulary,
    )
}

pub async fn validate_api_key(provider: &str, api_key: &str) -> anyhow::Result<bool> {
    match provider {
        "groq" => groq::validate_key(api_key).await,
        "openai" => openai::validate_key(api_key).await,
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::vad::VADConfig;

    #[test]
    fn split_long_segment_covers_range() {
        let windows = split_segments_by_max_duration(vec![(0, 100_000)], 32_000, 8_000);
        assert!(
            windows.len() >= 3,
            "expected multiple windows, got {}",
            windows.len()
        );
        assert_eq!(windows.first().unwrap().0, 0);
        assert_eq!(windows.last().unwrap().1, 100_000);
        for w in &windows {
            assert!(w.1 > w.0);
            assert!(w.1 - w.0 <= 32_000);
        }
    }

    #[test]
    fn split_short_segment_unchanged() {
        let windows = split_segments_by_max_duration(vec![(100, 500)], 10_000, 8_000);
        assert_eq!(windows, vec![(100, 500)]);
    }

    #[test]
    fn empty_vad_long_audio_same_as_full_span_split() {
        let max_samples = 16_000;
        let audio_len = 100_000;
        let w = split_segments_by_max_duration(
            vec![(0, audio_len)],
            max_samples,
            CHUNK_OVERLAP_SAMPLES,
        );
        assert!(w.len() > 1);
        assert_eq!(w[0].0, 0);
        assert_eq!(w.last().unwrap().1, audio_len);
    }

    #[test]
    fn max_chunk_samples_respects_floor_one_second() {
        let mut cfg = VADConfig::default();
        cfg.max_chunk_duration_sec = 0.1;
        assert_eq!(max_chunk_samples(&cfg, 16_000), 16_000);
    }

    #[test]
    fn sanitize_prompt_leakage_removes_exact_echo() {
        let vocab = "Kalam, Balacode, Rolla, Farahat";
        let out = sanitize_prompt_leakage(vocab, Some(vocab));
        assert!(out.is_empty());
    }

    #[test]
    fn sanitize_prompt_leakage_removes_leading_vocab_prefix() {
        let vocab = "Kalam, Balacode, Rolla, Farahat";
        let text = "Kalam, Balacode, Rolla, Farahat primarily these are the features that we are looking for";
        let out = sanitize_prompt_leakage(text, Some(vocab));
        assert_eq!(
            out,
            "primarily these are the features that we are looking for"
        );
    }

    #[test]
    fn sanitize_prompt_leakage_keeps_normal_text() {
        let vocab = "Kalam, Balacode, Rolla, Farahat";
        let text = "we are looking to have served the use cases described below";
        let out = sanitize_prompt_leakage(text, Some(vocab));
        assert_eq!(out, text);
    }
}
