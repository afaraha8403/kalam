#![allow(dead_code)]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SupportedStreamConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::resample::{self, Resampler};
use super::vad::{VADConfig, VADProcessor};

const TARGET_SAMPLE_RATE: u32 = 16000;

pub struct AudioCapture {
    device: cpal::Device,
    config: SupportedStreamConfig,
    is_recording: Arc<AtomicBool>,
    audio_buffer: Arc<std::sync::Mutex<Vec<f32>>>,
    resampler: Option<Resampler>,
    _vad: VADProcessor,
    _stream: Option<cpal::Stream>,
}

unsafe impl Send for AudioCapture {}
unsafe impl Sync for AudioCapture {}

impl AudioCapture {
    /// Create capture with the given VAD config (from STTConfig::vad_config()).
    pub fn new(vad_config: VADConfig) -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device available"))?;

        let config = device.default_input_config()?;
        log::info!("Default input config: {:?}", config);

        let vad = VADProcessor::new(vad_config)?;

        Ok(Self {
            device,
            config,
            is_recording: Arc::new(AtomicBool::new(false)),
            audio_buffer: Arc::new(std::sync::Mutex::new(Vec::new())),
            resampler: None,
            _vad: vad,
            _stream: None,
        })
    }

    pub fn set_device(&mut self, device_id: &str) -> anyhow::Result<()> {
        let host = cpal::default_host();

        // Get the new device
        let new_device = if device_id == "default" || device_id.is_empty() || device_id == "null" {
            host.default_input_device()
                .ok_or_else(|| anyhow::anyhow!("No default input device available"))?
        } else {
            // Try to find the device by matching IDs
            let input_devices = host.input_devices()?;
            let mut found_device = None;

            for (index, device) in input_devices.enumerate() {
                let expected_id = format!("device_{}", index);
                if expected_id == device_id {
                    found_device = Some(device);
                    break;
                }
            }

            found_device
                .or_else(|| host.default_input_device())
                .ok_or_else(|| anyhow::anyhow!("Device '{}' not found", device_id))?
        };

        // Get the config for the new device
        let new_config = new_device.default_input_config()?;
        log::info!("Switched to device with config: {:?}", new_config);

        self.device = new_device;
        self.config = new_config;

        Ok(())
    }

    /// If the current device (by id) is no longer available, switch to system default.
    /// Returns true if fallback was performed (caller should update config and notify).
    pub fn try_fallback_if_disconnected(&mut self, current_device_id: Option<&str>) -> bool {
        let device_id = match current_device_id {
            None | Some("") | Some("default") | Some("null") => return false,
            Some(id) => id,
        };
        if !device_id.starts_with("device_") {
            return false;
        }
        let n: usize = match device_id["device_".len()..].parse() {
            Ok(x) => x,
            Err(_) => return false,
        };
        let host = cpal::default_host();
        let count = match host.input_devices() {
            Ok(devices) => devices.count(),
            Err(_) => return false,
        };
        if count <= n && self.set_device("default").is_ok() {
            log::info!(
                "Audio device {} disconnected, fell back to default",
                device_id
            );
            return true;
        }
        false
    }

    pub async fn start_recording(&mut self) -> anyhow::Result<()> {
        if self.is_recording.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Clear buffer (std::sync::Mutex so cpal callback can push without Tokio)
        {
            let mut buffer = self.audio_buffer.lock().unwrap();
            buffer.clear();
        }

        let is_recording = self.is_recording.clone();
        let audio_buffer = self.audio_buffer.clone();
        let sample_format = self.config.sample_format();
        let channels = self.config.channels();

        is_recording.store(true, Ordering::SeqCst);

        // Build and start the stream
        let stream = match sample_format {
            SampleFormat::F32 => {
                self.build_stream_f32(is_recording.clone(), audio_buffer.clone(), channels)?
            }
            SampleFormat::I16 => {
                self.build_stream_i16(is_recording.clone(), audio_buffer.clone(), channels)?
            }
            SampleFormat::U16 => {
                self.build_stream_u16(is_recording.clone(), audio_buffer.clone(), channels)?
            }
            _ => return Err(anyhow::anyhow!("Unsupported sample format")),
        };

        stream.play()?;
        self._stream = Some(stream);

        Ok(())
    }

    /// Returns (samples, sample_rate). Sample rate is always 16000 when resampling was applied.
    pub async fn stop_recording(&mut self) -> anyhow::Result<(Vec<f32>, u32)> {
        self.is_recording.store(false, Ordering::SeqCst);
        // Allow callback thread to flush any in-flight frames, but cap wait time tightly.
        let mut prev_len = 0usize;
        for _ in 0..4 {
            tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
            let len = self.audio_buffer.lock().unwrap().len();
            if len == prev_len {
                break;
            }
            prev_len = len;
        }
        self._stream = None;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let buffer = self.audio_buffer.lock().unwrap().clone();
        let device_rate = self.config.config().sample_rate.0;

        if device_rate == TARGET_SAMPLE_RATE {
            return Ok((buffer, device_rate));
        }
        let resampled = resample::resample_to_16k_mono(&buffer, device_rate)?;
        Ok((resampled, TARGET_SAMPLE_RATE))
    }

    pub fn get_sample_rate(&self) -> u32 {
        self.config.config().sample_rate.0
    }

    /// Current input level (0–1) from recent buffer. Use while recording for a live meter.
    /// Uses last ~100ms of audio so the value reflects current volume.
    pub fn get_current_recording_level(&self) -> f32 {
        if !self.is_recording.load(Ordering::SeqCst) {
            return 0.0;
        }
        let buffer = self.audio_buffer.lock().unwrap();
        let sample_rate = self.get_sample_rate() as usize;
        let window_samples = (sample_rate / 10).max(256); // ~100ms or at least 256 samples
        let start = buffer.len().saturating_sub(window_samples);
        let slice = &buffer[start..];
        if slice.is_empty() {
            return 0.0;
        }
        let sum: f32 = slice.iter().map(|s| s * s).sum();
        let rms = (sum / slice.len() as f32).sqrt();
        // Scale so normal speech (~0.02–0.1 RMS) gives ~20–80%; shouting can hit 100%
        (rms * 10.0).min(1.0)
    }

    /// Stop recording and return (level 0-1, samples, sample_rate) for test playback.
    pub async fn stop_and_get_test_result(&mut self) -> anyhow::Result<(f32, Vec<f32>, u32)> {
        let (audio, sample_rate) = self.stop_recording().await?;

        if audio.is_empty() {
            return Ok((0.0, vec![], sample_rate));
        }

        let sum: f32 = audio.iter().map(|s| s * s).sum();
        let rms = (sum / audio.len() as f32).sqrt();
        log::info!(
            "Test stopped: {} samples, RMS: {}, sample_rate: {}",
            audio.len(),
            rms,
            sample_rate
        );
        let normalized = (rms * 10.0).min(1.0);
        Ok((normalized, audio, sample_rate))
    }

    fn build_stream_f32(
        &self,
        is_recording: Arc<AtomicBool>,
        audio_buffer: Arc<std::sync::Mutex<Vec<f32>>>,
        channels: u16,
    ) -> anyhow::Result<cpal::Stream> {
        let err_fn = |err| eprintln!("Stream error: {}", err);

        let stream = self.device.build_input_stream(
            &self.config.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if !is_recording.load(Ordering::SeqCst) {
                    return;
                }

                // Convert to mono if needed
                let mono_data: Vec<f32> = if channels > 1 {
                    data.chunks(channels as usize)
                        .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                        .collect()
                } else {
                    data.to_vec()
                };

                // Store in buffer (std::sync::Mutex: callback runs on cpal thread, no Tokio)
                if let Ok(mut buf) = audio_buffer.lock() {
                    buf.extend_from_slice(&mono_data);
                }
            },
            err_fn,
            None,
        )?;

        Ok(stream)
    }

    fn build_stream_i16(
        &self,
        is_recording: Arc<AtomicBool>,
        audio_buffer: Arc<std::sync::Mutex<Vec<f32>>>,
        channels: u16,
    ) -> anyhow::Result<cpal::Stream> {
        let err_fn = |err| eprintln!("Stream error: {}", err);

        let stream = self.device.build_input_stream(
            &self.config.config(),
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                if !is_recording.load(Ordering::SeqCst) {
                    return;
                }

                let mono_data: Vec<f32> = if channels > 1 {
                    data.chunks(channels as usize)
                        .map(|chunk| {
                            let sum: i32 = chunk.iter().map(|&s| s as i32).sum();
                            (sum as f32 / channels as f32) / 32768.0
                        })
                        .collect()
                } else {
                    data.iter().map(|&s| s as f32 / 32768.0).collect()
                };

                if let Ok(mut buf) = audio_buffer.lock() {
                    buf.extend_from_slice(&mono_data);
                }
            },
            err_fn,
            None,
        )?;

        Ok(stream)
    }

    fn build_stream_u16(
        &self,
        is_recording: Arc<AtomicBool>,
        audio_buffer: Arc<std::sync::Mutex<Vec<f32>>>,
        channels: u16,
    ) -> anyhow::Result<cpal::Stream> {
        let err_fn = |err| eprintln!("Stream error: {}", err);

        let stream = self.device.build_input_stream(
            &self.config.config(),
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                if !is_recording.load(Ordering::SeqCst) {
                    return;
                }

                let mono_data: Vec<f32> = if channels > 1 {
                    data.chunks(channels as usize)
                        .map(|chunk| {
                            let sum: u32 = chunk.iter().map(|&s| s as u32).sum();
                            ((sum as f32 / channels as f32) - 32768.0) / 32768.0
                        })
                        .collect()
                } else {
                    data.iter()
                        .map(|&s| (s as f32 - 32768.0) / 32768.0)
                        .collect()
                };

                if let Ok(mut buf) = audio_buffer.lock() {
                    buf.extend_from_slice(&mono_data);
                }
            },
            err_fn,
            None,
        )?;

        Ok(stream)
    }
}
