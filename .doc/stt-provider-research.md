# Speech-to-Text (STT) Provider Research & Integration Guide

**Date:** March 4, 2026  
**Purpose:** Comprehensive guide for Kalam Voice STT providers - Cloud APIs & Local Models  
**Target:** Support multiple providers with unified interface

---

## Executive Summary

This document provides a complete overview of Speech-to-Text options for Kalam Voice, including:
- **Cloud APIs** (Groq, OpenAI, Google, Azure, AssemblyAI, Deepgram, AWS)
- **Local/Self-hosted Models** (Whisper variants, Qwen, Wav2Vec 2.0, NeMo)
- **Hybrid Approaches** (Cloud + Local fallback)
- **Integration Architecture** (Unified provider interface)

**Recommendation:** Implement a **provider-agnostic architecture** with Groq as default cloud provider and faster-whisper as default local option.

---

## 1. Cloud API Providers

### 1.1 Groq (Recommended Default)

**Overview:** Ultra-fast Whisper inference via LPU (Language Processing Unit)

**Models Available:**
| Model | Size | Price | Speed | Best For |
|-------|------|-------|-------|----------|
| whisper-large-v3-turbo | 809M | $0.04/hour | ~300ms | Real-time dictation |
| whisper-large-v3 | 1.5B | $0.111/hour | ~500ms | Maximum accuracy |
| distil-whisper-large-v3 | 756M | $0.04/hour | ~200ms | Speed + efficiency |

**Pros:**
- ✅ **Fastest cloud option** (300-500ms latency)
- ✅ OpenAI-compatible API
- ✅ Very competitive pricing
- ✅ Excellent for real-time applications
- ✅ Supports streaming

**Cons:**
- ❌ Requires API key (no free tier)
- ❌ Cloud-only (privacy concerns)
- ❌ Rate limits on free accounts

**Integration:**
```rust
// Rust implementation using reqwest
use reqwest::Client;

pub struct GroqProvider {
    api_key: String,
    client: Client,
    model: String,
}

impl GroqProvider {
    pub async fn transcribe(&self, audio: &[u8]) -> Result<String> {
        let response = self.client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(
                reqwest::multipart::Form::new()
                    .part("file", reqwest::multipart::Part::bytes(audio.to_vec())
                        .file_name("audio.wav")
                        .mime_type("audio/wav"))
                    .text("model", self.model.clone())
                    .text("language", "auto")
                    .text("response_format", "json")
            )
            .send()
            .await?;
            
        let result: TranscriptionResponse = response.json().await?;
        Ok(result.text)
    }
}
```

**Pricing Estimate:**
- 1 hour of audio/day = ~$1.20/month
- 5 hours/day = ~$6/month
- 10 hours/day = ~$12/month

---

### 1.2 OpenAI Whisper API

**Overview:** Official Whisper API from OpenAI

**Models:**
- whisper-1 (large-v2 equivalent)

**Pricing:**
- $0.006/minute ($0.36/hour)

**Pros:**
- ✅ Official implementation
- ✅ High accuracy
- ✅ 99 languages supported
- ✅ Well-documented

**Cons:**
- ❌ Slower than Groq (1-2s latency)
- ❌ More expensive
- ❌ No streaming support

**Integration:**
Similar to Groq (OpenAI-compatible endpoint)

---

### 1.3 Google Cloud Speech-to-Text

**Overview:** Google's enterprise STT service

**Models:**
- **Chirp 3** (Latest, best quality)
- **Chirp 2** (Previous generation)
- **Latest Long** (Long-form audio)
- **Latest Short** (Short utterances)
- **Command and Search** (Voice commands)

**Pricing:**
- $0.024/minute for Chirp 3 ($1.44/hour)
- $0.016/minute for standard models ($0.96/hour)
- Free tier: 60 minutes/month

**Pros:**
- ✅ Excellent accuracy
- ✅ 125+ languages
- ✅ Speaker diarization
- ✅ Word-level timestamps
- ✅ Enterprise features (SLA, support)
- ✅ Custom model training

**Cons:**
- ❌ Higher latency (2-5s)
- ❌ More expensive
- ❌ Complex SDK
- ❌ Requires Google Cloud project

**Integration:**
```rust
// Using google-cloud-speech crate
use google_cloud_speech::client::Client;

pub struct GoogleProvider {
    client: Client,
}

impl GoogleProvider {
    pub async fn transcribe(&self, audio: &[u8]) -> Result<String> {
        // Requires google-cloud-sdk setup
        // More complex authentication (service accounts)
    }
}
```

---

### 1.4 Azure Speech Services

**Overview:** Microsoft's enterprise speech platform

**Models:**
- **Universal** (General purpose)
- **Conversation** (Call centers, meetings)
- **Dictation** (Single speaker)
- **Whisper** (OpenAI Whisper on Azure)

**Pricing:**
- $1.00/hour for standard
- $0.80/hour with commitment
- Free tier: 5 hours/month

**Pros:**
- ✅ Enterprise-grade
- ✅ Custom model training
- ✅ Speaker recognition
- ✅ Real-time transcription
- ✅ Container deployment option
- ✅ 100+ languages

**Cons:**
- ❌ Complex pricing
- ❌ Requires Azure subscription
- ❌ Higher latency than Groq

---

### 1.5 AssemblyAI

**Overview:** Developer-friendly speech AI platform

**Models:**
- **Universal-3 Pro** (Best accuracy, streaming)
- **Universal-2** (Balanced)
- **Universal-1** (Speed-focused)
- **Whisper** (OpenAI compatible)

**Pricing:**
- $0.37/hour for Universal-3 Pro
- $0.12/hour for Universal-1
- Free tier: 100 hours (one-time)

**Pros:**
- ✅ Excellent developer experience
- ✅ Real-time streaming
- ✅ Speaker diarization
- ✅ PII redaction
- ✅ Sentiment analysis
- ✅ Summarization
- ✅ Easy SDK

**Cons:**
- ❌ More expensive than Groq
- ❌ Cloud-only
- ❌ Lower tier accuracy

**Integration:**
```rust
use assemblyai::Client;

pub struct AssemblyAIProvider {
    client: Client,
}

impl AssemblyAIProvider {
    pub async fn transcribe(&self, audio_path: &str) -> Result<String> {
        let transcript = self.client
            .transcribe()
            .audio_url(audio_path)
            .speaker_labels(true)
            .send()
            .await?;
            
        Ok(transcript.text)
    }
}
```

---

### 1.6 Deepgram

**Overview:** Fast, accurate speech recognition platform

**Models:**
- **Nova-2** (Latest, best accuracy)
- **Nova-1** (Previous generation)
- **Whisper** (OpenAI compatible)

**Pricing:**
- $0.0043/minute ($0.258/hour) for Nova-2
- $0.0025/minute ($0.15/hour) for pre-recorded
- Free tier: $200 credit

**Pros:**
- ✅ Very fast (faster than real-time)
- ✅ Streaming support
- ✅ Speaker diarization
- ✅ Keyword boosting
- ✅ Custom vocabulary
- ✅ Good pricing

**Cons:**
- ❌ Cloud-only
- ❌ Fewer languages than Google/Azure
- ❌ Less enterprise features

---

### 1.7 AWS Transcribe

**Overview:** Amazon's cloud speech recognition

**Models:**
- **Standard** (General purpose)
- **Medical** (HIPAA-compliant)
- **Call Analytics** (Contact centers)

**Pricing:**
- $0.024/minute ($1.44/hour) for standard
- Free tier: 60 minutes/month

**Pros:**
- ✅ Integration with AWS ecosystem
- ✅ Custom vocabulary
- ✅ Speaker diarization
- ✅ Content redaction
- ✅ 100+ languages

**Cons:**
- ❌ Higher latency
- ❌ Complex SDK
- ❌ Requires AWS setup

---

## 2. Local/Self-Hosted Models

### 2.1 Whisper (Local)

#### 2.1.1 OpenAI Whisper (Python)

**Overview:** Original PyTorch implementation

**Models Available:**
| Model | Size | RAM | Speed | WER |
|-------|------|-----|-------|-----|
| tiny | 39M | ~1GB | ~10x | 18.6% |
| base | 74M | ~1GB | ~7x | 14.6% |
| small | 244M | ~2GB | ~4x | 10.3% |
| medium | 769M | ~5GB | ~2x | 8.0% |
| large-v3 | 1.5B | ~10GB | 1x | **7.2%** |
| turbo | 809M | ~6GB | ~8x | **7.2%** |

**Pros:**
- ✅ Reference implementation
- ✅ Easy to install (`pip install openai-whisper`)
- ✅ Supports all features (translate, timestamps)

**Cons:**
- ❌ Requires Python runtime
- ❌ Slower than optimized versions
- ❌ Large memory footprint

**Integration:**
```rust
// Via Python sidecar
use std::process::Command;

pub struct WhisperPythonProvider {
    model: String,
}

impl WhisperPythonProvider {
    pub async fn transcribe(&self, audio_path: &str) -> Result<String> {
        let output = Command::new("python")
            .arg("-m")
            .arg("whisper")
            .arg(audio_path)
            .arg("--model")
            .arg(&self.model)
            .arg("--output_format")
            .arg("json")
            .output()?;
            
        // Parse JSON output
    }
}
```

#### 2.1.2 Whisper.cpp (Recommended for Local)

**Overview:** C++ port of Whisper - optimized for local inference

**Performance:**
| Model | Disk | RAM | Speed (Mac M1) |
|-------|------|-----|----------------|
| tiny | 75MB | ~273MB | Real-time |
| base | 142MB | ~388MB | Real-time |
| small | 466MB | ~852MB | Real-time |
| medium | 1.5GB | ~2.1GB | 0.5x real-time |
| large | 2.9GB | ~3.9GB | 0.3x real-time |

**Quantization Options:**
- Q5_0: 5-bit quantization (2.5x smaller, minimal quality loss)
- Q8_0: 8-bit quantization (2x smaller, ~1% WER increase)

**Pros:**
- ✅ **No dependencies** (single binary)
- ✅ **Cross-platform** (Windows, macOS, Linux, iOS, Android)
- ✅ **Hardware acceleration** (Metal, CUDA, OpenVINO, Core ML)
- ✅ **Low memory** vs Python version
- ✅ **C API** for easy bindings
- ✅ **Streaming support**

**Cons:**
- ❌ Requires model download (~75MB-2.9GB)
- ❌ Setup complexity (compile from source or use pre-built)

**Rust Integration:**
```rust
// Using whisper-rs crate
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams};

pub struct WhisperCppProvider {
    ctx: WhisperContext,
}

impl WhisperCppProvider {
    pub fn new(model_path: &str) -> Result<Self> {
        let ctx = WhisperContext::new_with_params(
            model_path,
            WhisperContextParameters::default()
        )?;
        Ok(Self { ctx })
    }
    
    pub fn transcribe(&self, audio: &[f32]) -> Result<String> {
        let mut params = FullParams::new(Default::default());
        params.set_language(Some("auto"));
        params.set_translate(false);
        
        let mut state = self.ctx.create_state()?;
        state.full(params, audio)?;
        
        let num_segments = state.full_n_segments()?;
        let mut result = String::new();
        
        for i in 0..num_segments {
            result.push_str(&state.full_get_segment_text(i)?);
        }
        
        Ok(result)
    }
}
```

**Implementation Steps:**
1. Download whisper.cpp pre-built binary or compile
2. Download GGML models from HuggingFace
3. Use `whisper-rs` crate for Rust bindings
4. Bundle model with app or download on-demand

---

#### 2.1.3 Faster-Whisper (Optimized Python)

**Overview:** Whisper optimized with CTranslate2 - 4x faster, less memory

**Performance vs OpenAI Whisper:**
| Implementation | GPU Time (13min) | VRAM | CPU Time | RAM |
|----------------|------------------|------|----------|-----|
| openai/whisper | 2m23s | 4708MB | 6m58s | 2335MB |
| **faster-whisper (int8)** | **59s** | **2926MB** | **1m42s** | **1477MB** |

**Pros:**
- ✅ 4x faster than original
- ✅ 2x less memory
- ✅ INT8 quantization
- ✅ Batching support
- ✅ Easy pip install

**Cons:**
- ❌ Still requires Python
- ❌ GPU requires CUDA setup

**Best For:** Users who want better performance but still need Python ecosystem

---

#### 2.1.4 Distil-Whisper (Distilled Models)

**Overview:** Smaller, faster Whisper models via knowledge distillation

**Models:**
| Model | Params | Speed vs Large | WER | Size |
|-------|--------|----------------|-----|------|
| distil-large-v3 | 756M | 6.3x | 9.7% | ~1.5GB |
| distil-medium.en | 394M | 6.8x | 11.1% | ~800MB |
| distil-small.en | 166M | 5.6x | 12.1% | ~350MB |

**Pros:**
- ✅ 6x faster than large-v3
- ✅ 50% smaller
- ✅ Within 1% WER of original
- ✅ English only (for now)

**Cons:**
- ❌ English only
- ❌ Slightly lower accuracy

**Recommendation:** Use distil-large-v3 for English dictation - excellent speed/accuracy tradeoff

---

### 2.2 Qwen-Audio (Alibaba)

**Overview:** Large audio language model from Alibaba Cloud

**Specs:**
- **Size:** 8B parameters
- **Capabilities:** ASR, audio understanding, music analysis
- **Languages:** Multilingual (strong on Chinese, English)
- **License:** Apache 2.0 (commercial use allowed)

**Pros:**
- ✅ Open source
- ✅ Commercial license
- ✅ Multi-task (not just ASR)
- ✅ Good for Chinese

**Cons:**
- ❌ Very large (8B params = ~16GB RAM)
- ❌ Slower than Whisper
- ❌ Newer, less mature

**Not Recommended For:** Real-time dictation (too slow and resource-heavy)

---

### 2.3 Wav2Vec 2.0 (Meta)

**Overview:** Facebook's self-supervised speech model

**Models:**
- wav2vec2-base-960h (95M params)
- wav2vec2-large-960h (317M params)
- wav2vec2-large-960h-lv60 (1B params)

**WER on LibriSpeech:**
- Clean: 1.8-2.8%
- Other: 3.3-6.3%

**Pros:**
- ✅ State-of-the-art accuracy
- ✅ Fast inference
- ✅ Good for English

**Cons:**
- ❌ English only
- ❌ No built-in punctuation
- ❌ Requires separate language model for best results
- ❌ More complex setup

**Best For:** High-accuracy English transcription, not real-time dictation

---

### 2.4 NVIDIA NeMo

**Overview:** NVIDIA's speech AI toolkit

**Models:**
- **Canary** (Multilingual ASR + Translation)
- **Parakeet** (English ASR)
- **Parakeet-TDT** (Fast English)
- **Conformer-CTC** (Streaming)
- **RNNT** (End-to-end)

**Pros:**
- ✅ Enterprise-grade
- ✅ Optimized for NVIDIA GPUs
- ✅ Fast inference (RTFx > 2000)
- ✅ Custom model training
- ✅ Speaker diarization
- ✅ Punctuation restoration

**Cons:**
- ❌ Requires NVIDIA GPU for best performance
- ❌ Complex setup
- ❌ Large models
- ❌ Python only

**Best For:** Enterprise deployments with NVIDIA hardware

---

### 2.5 SenseVoice (Alibaba)

**Overview:** Multilingual voice understanding model (mentioned in PRD)

**Specs:**
- **Size:** Small (~200MB)
- **Speed:** 70ms for 10s audio
- **Languages:** 50+ languages
- **Features:** ASR, emotion recognition, VAD, audio events

**Integration:**
```rust
// Via ONNX Runtime
use ort::session::Session;

pub struct SenseVoiceProvider {
    session: Session,
}

impl SenseVoiceProvider {
    pub fn new(model_path: &str) -> Result<Self> {
        let session = Session::builder()?
            .commit_from_file(model_path)?;
        Ok(Self { session })
    }
}
```

**Pros:**
- ✅ Very fast
- ✅ Small size
- ✅ Multiple capabilities
- ✅ Good for Asian languages

**Cons:**
- ❌ Requires ONNX runtime
- ❌ Less documentation
- ❌ Chinese company (privacy concerns)

---

## 3. Provider Comparison Matrix

### 3.1 Cloud Providers

| Provider | Latency | Price/Hour | Accuracy | Languages | Streaming | Free Tier |
|----------|---------|------------|----------|-----------|-----------|-----------|
| **Groq** | 300ms | $0.04 | ⭐⭐⭐⭐⭐ | 99 | ✅ | - |
| OpenAI | 1-2s | $0.36 | ⭐⭐⭐⭐⭐ | 99 | ❌ | - |
| Google | 2-5s | $1.44 | ⭐⭐⭐⭐⭐ | 125 | ✅ | 60min/mo |
| Azure | 1-3s | $1.00 | ⭐⭐⭐⭐⭐ | 100 | ✅ | 5hr/mo |
| AssemblyAI | 500ms | $0.37 | ⭐⭐⭐⭐⭐ | 50+ | ✅ | 100hr |
| Deepgram | 300ms | $0.26 | ⭐⭐⭐⭐ | 30+ | ✅ | $200 |
| AWS | 2-4s | $1.44 | ⭐⭐⭐⭐ | 100 | ✅ | 60min/mo |

### 3.2 Local Models

| Model | Size | RAM | Speed | Accuracy | Setup |
|-------|------|-----|-------|----------|-------|
| **Whisper.cpp (tiny)** | 75MB | 273MB | Real-time | ⭐⭐⭐ | Easy |
| **Whisper.cpp (base)** | 142MB | 388MB | Real-time | ⭐⭐⭐⭐ | Easy |
| **Whisper.cpp (small)** | 466MB | 852MB | Real-time | ⭐⭐⭐⭐ | Easy |
| Whisper (large-v3) | 2.9GB | 3.9GB | 0.3x RT | ⭐⭐⭐⭐⭐ | Medium |
| Distil-Whisper | 756MB | 1.5GB | 6x faster | ⭐⭐⭐⭐ | Medium |
| Faster-Whisper | 2.9GB | 1.5GB | 4x faster | ⭐⭐⭐⭐⭐ | Medium |
| SenseVoice | 200MB | 500MB | Real-time | ⭐⭐⭐⭐ | Hard |
| Wav2Vec 2.0 | 317MB | 1GB | Real-time | ⭐⭐⭐⭐⭐ | Hard |
| Qwen-Audio | 16GB | 16GB | Slow | ⭐⭐⭐⭐⭐ | Hard |

---

## 4. Recommended Architecture

### 4.1 Unified Provider Interface

```rust
// src/stt/mod.rs

#[async_trait]
pub trait STTProvider: Send + Sync {
    async fn transcribe(&self, audio: AudioChunk) -> Result<TranscriptionResult>;
    async fn transcribe_stream(&self, stream: AudioStream) -> Result<TranscriptionStream>;
    fn supports_streaming(&self) -> bool;
    fn requires_internet(&self) -> bool;
    fn estimated_latency(&self) -> Duration;
}

pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f32,
    pub language: String,
    pub timestamp_start: f64,
    pub timestamp_end: f64,
    pub words: Vec<WordInfo>,
}

pub struct WordInfo {
    pub word: String,
    pub start: f64,
    pub end: f64,
    pub confidence: f32,
}

// Provider implementations
pub mod groq;
pub mod openai;
pub mod whisper_cpp;
pub mod faster_whisper;
pub mod google;
pub mod azure;
pub mod assemblyai;
```

### 4.2 Provider Selection Logic

```rust
// src/stt/provider_selector.rs

pub struct ProviderSelector {
    providers: Vec<Box<dyn STTProvider>>,
    config: STTConfig,
}

impl ProviderSelector {
    pub async fn transcribe(&self, audio: AudioChunk) -> Result<TranscriptionResult> {
        // Priority:
        // 1. User's preferred provider if available
        // 2. Local provider if offline
        // 3. Fastest available cloud provider
        // 4. Fallback chain
        
        if !self.config.prefer_cloud && self.has_local_provider() {
            return self.local_provider().transcribe(audio).await;
        }
        
        // Try cloud providers in order of preference
        for provider in &self.cloud_providers() {
            match provider.transcribe(audio.clone()).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    log::warn!("Provider {} failed: {}", provider.name(), e);
                    continue;
                }
            }
        }
        
        // Fallback to local
        self.local_provider().transcribe(audio).await
    }
}
```

### 4.3 Recommended Default Configuration

```yaml
# Default settings for Kalam Voice
stt:
  # Primary cloud provider
  default_cloud: "groq"
  
  # Primary local provider  
  default_local: "whisper_cpp_small"
  
  # Mode: cloud | local | hybrid | auto
  mode: "hybrid"
  
  # Cloud provider fallback chain
  cloud_fallback:
    - groq
    - openai
    - assemblyai
    - google
  
  # Local model settings
  local_models:
    whisper_tiny:
      path: "models/ggml-tiny.bin"
      size: "75MB"
      ram: "273MB"
    whisper_base:
      path: "models/ggml-base.bin"
      size: "142MB"
      ram: "388MB"
    whisper_small:
      path: "models/ggml-small.bin"
      size: "466MB"
      ram: "852MB"
  
  # Hybrid mode settings
  hybrid:
    # Switch to local when latency > threshold
    latency_threshold_ms: 1000
    # Switch to local when offline
    auto_offline: true
    # Switch to local for sensitive apps
    privacy_mode_apps:
      - "1Password"
      - "Bitwarden"
      - "*bank*"
```

---

## 5. Implementation Roadmap

### Phase 1: MVP (Months 1-3)
**Cloud Providers:**
1. ✅ Groq (default) - Fast, cheap, easy
2. ✅ OpenAI Whisper (fallback) - Compatible, reliable

**Local:**
3. ✅ Whisper.cpp (tiny/base) - Easy integration, small size

### Phase 2: Enhanced (Months 4-6)
4. Faster-Whisper (better local performance)
5. Distil-Whisper (English optimization)
6. AssemblyAI (advanced features)

### Phase 3: Enterprise (Months 7-9)
7. Google Cloud Speech
8. Azure Speech
9. AWS Transcribe
10. Custom model support

### Phase 4: Advanced (Months 10-12)
11. Speaker diarization
12. Real-time streaming optimization
13. Custom vocabulary/personalization
14. Multi-model ensemble

---

## 6. Specific Recommendations

### 6.1 For Different User Scenarios

| Use Case | Recommended Provider | Why |
|----------|---------------------|-----|
| **General dictation** | Groq (cloud) + Whisper.cpp small (local) | Best speed/cost/quality balance |
| **Privacy-focused** | Whisper.cpp base | 100% offline, good quality |
| **Low-end hardware** | Whisper.cpp tiny | Minimal resources |
| **Professional/Enterprise** | Google/Azure + Whisper.cpp medium | Compliance + quality |
| **English only** | Distil-Whisper | 6x faster, almost same quality |
| **Multi-language** | Groq or Whisper.cpp large | 99 languages supported |
| **Offline capability** | Whisper.cpp small | Works without internet |

### 6.2 Bundle Size Considerations

**Option A: Cloud-Only (Minimal Bundle)**
- Bundle size: ~15MB (Tauri app only)
- Requires internet
- Models: Groq, OpenAI

**Option B: Cloud + Basic Local (Recommended)**
- Bundle size: ~15MB + optional 142MB download
- Fallback when offline
- Models: Groq + Whisper base

**Option C: Full Offline**
- Bundle size: ~15MB + 466MB (included)
- Complete offline capability
- Models: Whisper small

**Option D: Enterprise**
- Bundle size: ~15MB + 2.9GB (included)
- Maximum quality offline
- Models: Whisper large

---

## 7. Integration Code Examples

### 7.1 Complete Provider Factory

```rust
// src/stt/factory.rs

pub struct STTProviderFactory;

impl STTProviderFactory {
    pub fn create(config: &ProviderConfig) -> Box<dyn STTProvider> {
        match config.provider_type {
            ProviderType::Groq => Box::new(GroqProvider::new(&config.api_key)),
            ProviderType::OpenAI => Box::new(OpenAIProvider::new(&config.api_key)),
            ProviderType::WhisperCpp => {
                Box::new(WhisperCppProvider::new(&config.model_path))
            }
            ProviderType::AssemblyAI => Box::new(AssemblyAIProvider::new(&config.api_key)),
            // ... other providers
        }
    }
}
```

### 7.2 Settings UI

```yaml
# User-facing settings
stt_provider:
  type: select
  options:
    - value: "groq"
      label: "Groq (Fast Cloud)"
      requires_api_key: true
    - value: "openai"
      label: "OpenAI Whisper"
      requires_api_key: true
    - value: "whisper_tiny"
      label: "Whisper Tiny (Local - Fastest)"
      requires_download: true
      download_size: "75MB"
    - value: "whisper_base"
      label: "Whisper Base (Local - Balanced)"
      requires_download: true
      download_size: "142MB"
    - value: "whisper_small"
      label: "Whisper Small (Local - Best Quality)"
      requires_download: true
      download_size: "466MB"
```

---

## 8. Security & Privacy Considerations

### 8.1 Data Handling by Provider

| Provider | Data Retention | HIPAA | GDPR | Notes |
|----------|---------------|-------|------|-------|
| Groq | No retention | ❌ | ✅ | Deletes after processing |
| OpenAI | May retain | ❌ | ✅ | Enterprise option available |
| Google | 30 days | ✅ | ✅ | HIPAA BAA available |
| Azure | Configurable | ✅ | ✅ | Enterprise compliance |
| AssemblyAI | Configurable | ✅ | ✅ | Zero retention option |
| Local | None | ✅ | ✅ | 100% private |

### 8.2 Recommendations

**For Healthcare/Legal:**
- Use local models (Whisper.cpp)
- Or: Google/Azure with HIPAA BAA
- Enable "privacy mode" for sensitive apps

**For General Use:**
- Groq is acceptable (no data retention)
- Provide clear privacy settings

**For Enterprise:**
- Self-hosted option
- Azure/Google with compliance agreements
- On-premise deployment

---

## 9. Cost Analysis

### 9.1 Monthly Cost Estimates (1 hour/day usage)

| Provider | Cost/Month | Notes |
|----------|-----------|-------|
| **Groq** | **$1.20** | Cheapest cloud option |
| OpenAI | $10.80 | Standard pricing |
| Google | $43.20 | Premium option |
| Azure | $30.00 | Mid-tier |
| AssemblyAI | $11.10 | Feature-rich |
| Deepgram | $7.74 | Competitive |
| Local | $0 | One-time hardware cost |

### 9.2 Break-Even Analysis

**Cloud vs Local:**
- Local GPU cost: ~$500 (one-time)
- Cloud savings: ~$10/month vs OpenAI
- Break-even: ~50 months (4 years)

**Recommendation:** Cloud is more cost-effective for most users unless:
- Very high usage (>8 hours/day)
- Privacy requirements mandate local
- Unreliable internet

---

## 10. Conclusion

### 10.1 Final Recommendations

**Primary Architecture:**
```
┌─────────────────────────────────────────┐
│           Kalam Voice App               │
├─────────────────────────────────────────┤
│  UI Layer (React)                       │
├─────────────────────────────────────────┤
│  STT Provider Manager (Rust)            │
│  ├─ Groq (default cloud)               │
│  ├─ Whisper.cpp (default local)        │
│  ├─ OpenAI (fallback)                  │
│  └─ User-configurable providers        │
├─────────────────────────────────────────┤
│  Audio Pipeline (cpal)                  │
│  ├─ Audio Capture                      │
│  ├─ VAD (Silero)                       │
│  └─ Chunking                           │
└─────────────────────────────────────────┘
```

**Phase 1 Implementation:**
1. **Groq** - Default cloud provider ($0.04/hour, 300ms latency)
2. **Whisper.cpp (base)** - Default local (142MB, real-time)
3. **OpenAI** - Fallback cloud provider
4. **Provider-agnostic interface** for future expansion

**Key Success Factors:**
- ✅ Start simple: Groq + Whisper.cpp
- ✅ Provider abstraction for flexibility
- ✅ Clear privacy controls
- ✅ Transparent pricing
- ✅ Graceful fallbacks

---

## 11. Resources & References

### 11.1 GitHub Repositories

- **Whisper.cpp:** https://github.com/ggerganov/whisper.cpp
- **Faster-Whisper:** https://github.com/SYSTRAN/faster-whisper
- **Distil-Whisper:** https://github.com/huggingface/distil-whisper
- **Whisper (OpenAI):** https://github.com/openai/whisper
- **NeMo:** https://github.com/NVIDIA/NeMo
- **SenseVoice:** https://github.com/FunAudioLLM/SenseVoice

### 11.2 Documentation

- **Groq:** https://console.groq.com/docs
- **OpenAI:** https://platform.openai.com/docs/guides/speech-to-text
- **Google Cloud:** https://cloud.google.com/speech-to-text/docs
- **Azure Speech:** https://learn.microsoft.com/en-us/azure/cognitive-services/speech-service/
- **AssemblyAI:** https://www.assemblyai.com/docs

### 11.3 Model Downloads

- **Whisper GGML:** https://huggingface.co/ggerganov
- **Distil-Whisper:** https://huggingface.co/distil-whisper
- **SenseVoice:** https://huggingface.co/FunAudioLLM/SenseVoiceSmall

---

**Document Version:** 1.0  
**Last Updated:** March 4, 2026  
**Next Review:** April 4, 2026  
**Author:** Technical Research Team
