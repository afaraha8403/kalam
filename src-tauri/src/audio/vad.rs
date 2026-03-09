#![allow(dead_code)]

use serde::{Deserialize, Serialize};

const SAMPLE_RATE: u32 = 16000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VADConfig {
    pub speech_threshold: f32,
    pub silence_timeout_sec: f64,
    pub min_speech_duration_sec: f64,
    pub max_chunk_duration_sec: f64,
    pub speech_padding_sec: f64,
}

impl Default for VADConfig {
    fn default() -> Self {
        Self {
            speech_threshold: 0.5,
            silence_timeout_sec: 1.5,
            min_speech_duration_sec: 0.25,
            max_chunk_duration_sec: 30.0,
            speech_padding_sec: 0.3,
        }
    }
}

impl VADConfig {
    pub fn fast() -> Self {
        Self {
            silence_timeout_sec: 0.8,
            ..Default::default()
        }
    }

    pub fn accurate() -> Self {
        Self {
            silence_timeout_sec: 2.5,
            ..Default::default()
        }
    }
}

/// Resolve path to Silero VAD ONNX model for future use.
/// When an ONNX runner (e.g. tract or ort with compatible version) is integrated,
/// load the model from this path and use it in VADProcessor for accurate segmentation.
pub fn silero_model_path() -> Option<std::path::PathBuf> {
    if let Ok(path) = std::env::var("KALAM_SILERO_VAD_PATH") {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    if let Some(dir) = directories::ProjectDirs::from("com", "Kalam", "Kalam") {
        let path = dir.data_local_dir().join("silero_vad.onnx");
        if path.exists() {
            return Some(path);
        }
    }
    None
}

pub struct VADProcessor {
    config: VADConfig,
}

impl VADProcessor {
    /// Build VAD from config. Uses VADConfig thresholds (Fast/Balanced/Accurate) for segmentation.
    /// When Silero ONNX model is available at silero_model_path() and a runner is integrated,
    /// it can be used here for more accurate speech detection.
    pub fn new(config: VADConfig) -> anyhow::Result<Self> {
        Ok(Self { config })
    }

    /// Process 16 kHz mono f32 audio and return speech segments as (start_sample, end_sample).
    /// Applies speech_threshold, silence_timeout_sec, min_speech_duration_sec from config.
    pub fn process(&mut self, audio: &[f32]) -> Vec<(usize, usize)> {
        self.process_energy(audio)
    }

    fn process_energy(&self, audio: &[f32]) -> Vec<(usize, usize)> {
        let min_speech_samples =
            (self.config.min_speech_duration_sec * SAMPLE_RATE as f64) as usize;
        let silence_timeout_samples =
            (self.config.silence_timeout_sec * SAMPLE_RATE as f64) as usize;
        let speech_padding_samples = (self.config.speech_padding_sec * SAMPLE_RATE as f64) as usize;

        let mut segments = Vec::new();
        let mut start = None;
        let mut silence_run = 0usize;

        for (i, &sample) in audio.iter().enumerate() {
            let energy = sample.abs();
            let is_speech = energy > self.config.speech_threshold;

            if is_speech {
                silence_run = 0;
                if start.is_none() {
                    start = Some(i);
                }
            } else if let Some(s) = start {
                silence_run += 1;
                if silence_run >= silence_timeout_samples {
                    let duration = i.saturating_sub(s);
                    if duration >= min_speech_samples {
                        let end = (i + speech_padding_samples).min(audio.len());
                        segments.push((s, end));
                    }
                    start = None;
                    silence_run = 0;
                }
            }
        }

        if let Some(s) = start {
            let duration = audio.len().saturating_sub(s);
            if duration >= min_speech_samples {
                segments.push((s, audio.len()));
            }
        }

        segments
    }
}
