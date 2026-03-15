#![allow(dead_code)]

use reqwest::blocking::multipart::{Form, Part};

use super::provider::STTProvider;
use super::{TranscriptionError, TranscriptionResult};

pub struct GroqProvider {
    api_key: String,
    client: reqwest::blocking::Client,
    model: String,
}

impl GroqProvider {
    pub fn new(api_key: String) -> anyhow::Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            api_key,
            client,
            model: "whisper-large-v3-turbo".to_string(),
        })
    }
}

impl STTProvider for GroqProvider {
    fn transcribe_blocking(
        &self,
        audio: &[f32],
        sample_rate: u32,
        prompt: Option<&str>,
        language_hint: Option<&str>,
    ) -> anyhow::Result<TranscriptionResult> {
        let samples_i16: Vec<i16> = audio
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect();

        log::info!(
            "Creating WAV: {} samples at {}Hz",
            samples_i16.len(),
            sample_rate
        );
        let wav_data = create_wav(&samples_i16, sample_rate, 1)?;

        let mut form = Form::new()
            .part(
                "file",
                Part::bytes(wav_data)
                    .file_name("audio.wav")
                    .mime_str("audio/wav")?,
            )
            .text("model", self.model.clone())
            .text("response_format", "verbose_json");
        if let Some(lang) = language_hint {
            form = form.text("language", lang.to_string());
        }
        if let Some(p) = prompt {
            log::info!(
                "Groq transcription prompt ({} chars): {:?}",
                p.len(),
                p.chars().take(100).collect::<String>()
            );
            form = form.text("prompt", p.to_string());
        }

        let response = self.client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .map_err(|e| {
                // Classify reqwest errors as network (timeout, connection) -> retriable.
                let retriable = e.is_timeout().unwrap_or(false)
                    || e.is_connect().unwrap_or(false)
                    || e.is_connection_refused().unwrap_or(false);
                anyhow::anyhow!(TranscriptionError::Network { retriable })
            })?;

        let status_code = response.status().as_u16();
        if !response.status().is_success() {
            let _ = response.text(); // consume body
            let retriable = status_code == 429 || (500..600).contains(&status_code);
            return Err(anyhow::anyhow!(TranscriptionError::Api {
                status: status_code,
                retriable,
            }));
        }

        let result: GroqResponse = response.json().map_err(|e| {
            anyhow::anyhow!(TranscriptionError::Unknown(e.to_string()))
        })?;

        Ok(TranscriptionResult {
            text: result.text,
            confidence: result
                .segments
                .first()
                .map(|s| s.avg_logprob as f32)
                .unwrap_or(0.9),
            language: result.language.unwrap_or_else(|| "unknown".to_string()),
        })
    }

    fn requires_internet(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "Groq (whisper-large-v3-turbo)"
    }
}

pub async fn validate_key(api_key: &str) -> anyhow::Result<bool> {
    log::info!("Validating Groq API key...");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client
        .get("https://api.groq.com/openai/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let status = response.status();
    log::info!("Groq API response status: {}", status);

    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        log::error!(
            "Groq API key validation failed: {} - {}",
            status,
            error_text
        );
        return Err(anyhow::anyhow!("API returned {}: {}", status, error_text));
    }

    log::info!("Groq API key validation successful");
    Ok(true)
}

#[derive(Debug, serde::Deserialize)]
struct GroqResponse {
    text: String,
    language: Option<String>,
    segments: Vec<GroqSegment>,
}

#[derive(Debug, serde::Deserialize)]
struct GroqSegment {
    #[serde(rename = "avg_logprob")]
    avg_logprob: f64,
}

fn create_wav(samples: &[i16], sample_rate: u32, channels: u16) -> anyhow::Result<Vec<u8>> {
    use std::io::Write;

    let data_len = samples.len() * 2;
    let file_len = 36 + data_len;

    let mut wav = Vec::with_capacity(44 + data_len);

    // RIFF header
    wav.write_all(b"RIFF")?;
    wav.write_all(&(file_len as u32).to_le_bytes())?;
    wav.write_all(b"WAVE")?;

    // fmt chunk
    wav.write_all(b"fmt ")?;
    wav.write_all(&16u32.to_le_bytes())?; // chunk size
    wav.write_all(&1u16.to_le_bytes())?; // format (PCM)
    wav.write_all(&channels.to_le_bytes())?;
    wav.write_all(&sample_rate.to_le_bytes())?;
    wav.write_all(&(sample_rate * channels as u32 * 2).to_le_bytes())?; // byte rate
    wav.write_all(&(channels * 2).to_le_bytes())?; // block align
    wav.write_all(&16u16.to_le_bytes())?; // bits per sample

    // data chunk
    wav.write_all(b"data")?;
    wav.write_all(&(data_len as u32).to_le_bytes())?;

    // Write samples
    for &sample in samples {
        wav.write_all(&sample.to_le_bytes())?;
    }

    Ok(wav)
}
