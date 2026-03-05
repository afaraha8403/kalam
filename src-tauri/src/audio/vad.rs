#![allow(dead_code)]

use serde::{Deserialize, Serialize};

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

pub struct VADProcessor {
    config: VADConfig,
    // In production, integrate with Silero VAD or similar
}

impl VADProcessor {
    pub fn new(config: VADConfig) -> anyhow::Result<Self> {
        Ok(Self { config })
    }

    pub fn process(&mut self, audio: &[f32]) -> Vec<(usize, usize)> {
        // Simple energy-based VAD for MVP
        // In production, use Silero VAD
        let _window_size = (self.config.silence_timeout_sec * 16000.0) as usize;
        let mut segments = Vec::new();
        let mut start = None;

        for (i, &sample) in audio.iter().enumerate() {
            let energy = sample.abs();
            let is_speech = energy > self.config.speech_threshold;

            if is_speech && start.is_none() {
                start = Some(i);
            } else if !is_speech && start.is_some() {
                let s = start.unwrap();
                if i - s > (self.config.min_speech_duration_sec * 16000.0) as usize {
                    segments.push((s, i));
                }
                start = None;
            }
        }

        if let Some(s) = start {
            segments.push((s, audio.len()));
        }

        segments
    }
}
