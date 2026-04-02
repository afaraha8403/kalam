//! Integration tests for the STT pipeline using real WAV fixtures.
//!
//! - **Pipeline smoke** (`fixture_runs_through_transcribe_chunked`): no network; uses a stub provider.
//! - **Cloud providers**: run only when `GROQ_API_KEY` / `OPENAI_API_KEY` are set (e.g. CI secrets or local export).
//!   Assertions are fuzzy because Whisper output varies slightly.
//!
//! Fixture: `tests/fixtures/test_english.wav` (mono speech; may be resampled to 16 kHz in-test).

use std::path::PathBuf;
use std::time::Duration;

use hound::{SampleFormat, WavReader};
use kalam_voice::audio::vad::VADConfig;
use kalam_voice::stt::groq::GroqProvider;
use kalam_voice::stt::openai::OpenAIProvider;
use kalam_voice::stt::provider::STTProvider;
use kalam_voice::stt::transcribe_chunked;
use kalam_voice::stt::TranscriptionResult;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("test_english.wav")
}

/// Linear resample to the rate the rest of the pipeline expects (16 kHz, same as live capture).
fn resample_linear(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = ((samples.len() as f64) / ratio).ceil() as usize;
    let mut out = Vec::with_capacity(out_len.max(1));
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let i0 = src_pos.floor() as usize;
        let i1 = (i0 + 1).min(samples.len() - 1);
        let frac = src_pos - i0 as f64;
        let s = samples[i0] as f64 * (1.0 - frac) + samples[i1] as f64 * frac;
        out.push(s as f32);
    }
    out
}

fn read_wav_mono_f32(path: &std::path::Path) -> (Vec<f32>, u32) {
    let mut reader = WavReader::open(path).expect("open WAV fixture");
    let spec = reader.spec();
    assert_eq!(
        spec.channels, 1,
        "fixture must be mono for this test harness"
    );
    let samples: Vec<f32> = match spec.sample_format {
        SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            if bits == 16 {
                reader
                    .samples::<i16>()
                    .map(|s| s.expect("sample") as f32 / 32768.0)
                    .collect()
            } else {
                panic!("unsupported int bits_per_sample: {}", bits);
            }
        }
        SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.expect("sample"))
            .collect(),
    };
    (samples, spec.sample_rate)
}

fn load_fixture_audio_16k() -> Vec<f32> {
    let path = fixture_path();
    assert!(
        path.exists(),
        "missing fixture {:?}; add tests/fixtures/test_english.wav",
        path
    );
    let (samples, rate) = read_wav_mono_f32(&path);
    resample_linear(&samples, rate, 16_000)
}

/// Matches SAPI / Whisper-style output for the canned fox phrase (wording may differ).
fn matches_fox_fixture_transcript(text: &str) -> bool {
    let t = text.to_lowercase();
    let hits = ["quick", "brown", "fox", "lazy"]
        .iter()
        .filter(|w| t.contains(*w))
        .count();
    hits >= 3
}

struct StubProvider;

impl STTProvider for StubProvider {
    fn transcribe_blocking(
        &self,
        _audio: &[f32],
        _sample_rate: u32,
        _prompt: Option<&str>,
        _language_hint: Option<&str>,
    ) -> anyhow::Result<TranscriptionResult> {
        Ok(TranscriptionResult {
            text: "fixture_pipeline_ok".to_string(),
            confidence: 1.0,
            language: "en".to_string(),
        })
    }

    fn requires_internet(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "stub"
    }
}

#[test]
fn fixture_runs_through_transcribe_chunked() {
    let audio = load_fixture_audio_16k();
    assert!(
        audio.len() > 8_000,
        "fixture audio unexpectedly short ({} samples)",
        audio.len()
    );
    let vad = VADConfig::default();
    let provider = StubProvider;
    let out = transcribe_chunked(
        &provider,
        &audio,
        16_000,
        &vad,
        Some("en"),
        None as Option<&str>,
    )
    .expect("transcribe_chunked");
    assert!(
        out.text.contains("fixture_pipeline_ok"),
        "got {:?}",
        out.text
    );
}

#[test]
fn groq_transcribes_fox_fixture_when_api_key_set() {
    let Ok(key) = std::env::var("GROQ_API_KEY") else {
        eprintln!("skip groq_transcribes_fox_fixture_when_api_key_set: GROQ_API_KEY not set");
        return;
    };
    if key.trim().is_empty() {
        eprintln!("skip groq_transcribes_fox_fixture_when_api_key_set: GROQ_API_KEY empty");
        return;
    }

    let audio = load_fixture_audio_16k();
    let provider = GroqProvider::new(key.trim().to_string(), Duration::from_secs(90), None)
        .expect("groq client");
    let vad = VADConfig::default();
    let out = transcribe_chunked(
        &provider,
        &audio,
        16_000,
        &vad,
        Some("en"),
        None as Option<&str>,
    )
    .expect("groq transcribe");
    assert!(
        matches_fox_fixture_transcript(&out.text),
        "unexpected transcript (fuzzy match failed): {:?}",
        out.text
    );
}

#[test]
fn openai_transcribes_fox_fixture_when_api_key_set() {
    let Ok(key) = std::env::var("OPENAI_API_KEY") else {
        eprintln!("skip openai_transcribes_fox_fixture_when_api_key_set: OPENAI_API_KEY not set");
        return;
    };
    if key.trim().is_empty() {
        eprintln!("skip openai_transcribes_fox_fixture_when_api_key_set: OPENAI_API_KEY empty");
        return;
    }

    let audio = load_fixture_audio_16k();
    let provider = OpenAIProvider::new(key.trim().to_string(), Duration::from_secs(90), None)
        .expect("openai client");
    let vad = VADConfig::default();
    let out = transcribe_chunked(
        &provider,
        &audio,
        16_000,
        &vad,
        Some("en"),
        None as Option<&str>,
    )
    .expect("openai transcribe");
    assert!(
        matches_fox_fixture_transcript(&out.text),
        "unexpected transcript (fuzzy match failed): {:?}",
        out.text
    );
}
