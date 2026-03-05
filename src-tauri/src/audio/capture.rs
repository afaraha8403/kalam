#![allow(dead_code)]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SupportedStreamConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::resample::Resampler;
use super::vad::{VADConfig, VADProcessor};

const TARGET_SAMPLE_RATE: u32 = 16000;

pub struct AudioCapture {
    device: cpal::Device,
    config: SupportedStreamConfig,
    is_recording: Arc<AtomicBool>,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    resampler: Option<Resampler>,
    _vad: VADProcessor,
    _stream: Option<cpal::Stream>,
}

unsafe impl Send for AudioCapture {}
unsafe impl Sync for AudioCapture {}

impl AudioCapture {
    pub fn new() -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device available"))?;

        let config = device.default_input_config()?;
        log::info!("Default input config: {:?}", config);

        let vad = VADProcessor::new(VADConfig::default())?;

        Ok(Self {
            device,
            config,
            is_recording: Arc::new(AtomicBool::new(false)),
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            resampler: None,
            _vad: vad,
            _stream: None,
        })
    }

    pub fn set_device(&mut self, device_id: &str) -> anyhow::Result<()> {
        let host = cpal::default_host();
        
        // Get the new device
        let new_device = if device_id == "default" || device_id.is_empty() {
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
            
            found_device.or_else(|| host.default_input_device())
                .ok_or_else(|| anyhow::anyhow!("Device '{}' not found", device_id))?
        };
        
        // Get the config for the new device
        let new_config = new_device.default_input_config()?;
        log::info!("Switched to device with config: {:?}", new_config);
        
        self.device = new_device;
        self.config = new_config;
        
        Ok(())
    }

    pub async fn start_recording(&mut self) -> anyhow::Result<()> {
        if self.is_recording.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Clear buffer
        {
            let mut buffer = self.audio_buffer.lock().await;
            buffer.clear();
        }

        let is_recording = self.is_recording.clone();
        let audio_buffer = self.audio_buffer.clone();
        let sample_format = self.config.sample_format();
        let channels = self.config.channels();

        is_recording.store(true, Ordering::SeqCst);

        // Build and start the stream
        let stream = match sample_format {
            SampleFormat::F32 => self.build_stream_f32(is_recording.clone(), audio_buffer.clone(), channels)?,
            SampleFormat::I16 => self.build_stream_i16(is_recording.clone(), audio_buffer.clone(), channels)?,
            SampleFormat::U16 => self.build_stream_u16(is_recording.clone(), audio_buffer.clone(), channels)?,
            _ => return Err(anyhow::anyhow!("Unsupported sample format")),
        };

        stream.play()?;
        self._stream = Some(stream);

        Ok(())
    }

    pub async fn stop_recording(&mut self) -> anyhow::Result<Vec<f32>> {
        self.is_recording.store(false, Ordering::SeqCst);

        // Wait a moment for final samples
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Drop the stream to stop recording
        self._stream = None;

        let buffer = self.audio_buffer.lock().await;
        Ok(buffer.clone())
    }

    pub async fn test_microphone(&mut self) -> anyhow::Result<f32> {
        // Quick 1-second test to get audio level
        self.start_recording().await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let audio = self.stop_recording().await?;

        // Calculate RMS level
        if audio.is_empty() {
            return Ok(0.0);
        }

        let sum: f32 = audio.iter().map(|s| s * s).sum();
        let rms = (sum / audio.len() as f32).sqrt();
        
        log::info!("Microphone test captured {} samples, RMS level: {}", audio.len(), rms);
        
        // Normalize to 0-1 range (typical RMS values are 0-0.3 for normal speech)
        let normalized = (rms * 3.0).min(1.0);
        
        Ok(normalized)
    }

    fn build_stream_f32(
        &self,
        is_recording: Arc<AtomicBool>,
        audio_buffer: Arc<Mutex<Vec<f32>>>,
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

                // Store in buffer (async runtime handle)
                let buffer = audio_buffer.clone();
                let mono = mono_data.clone();
                tokio::spawn(async move {
                    let mut buf = buffer.lock().await;
                    buf.extend_from_slice(&mono);
                });
            },
            err_fn,
            None,
        )?;

        Ok(stream)
    }

    fn build_stream_i16(
        &self,
        is_recording: Arc<AtomicBool>,
        audio_buffer: Arc<Mutex<Vec<f32>>>,
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

                let buffer = audio_buffer.clone();
                let mono = mono_data.clone();
                tokio::spawn(async move {
                    let mut buf = buffer.lock().await;
                    buf.extend_from_slice(&mono);
                });
            },
            err_fn,
            None,
        )?;

        Ok(stream)
    }

    fn build_stream_u16(
        &self,
        is_recording: Arc<AtomicBool>,
        audio_buffer: Arc<Mutex<Vec<f32>>>,
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
                    data.iter().map(|&s| (s as f32 - 32768.0) / 32768.0).collect()
                };

                let buffer = audio_buffer.clone();
                let mono = mono_data.clone();
                tokio::spawn(async move {
                    let mut buf = buffer.lock().await;
                    buf.extend_from_slice(&mono);
                });
            },
            err_fn,
            None,
        )?;

        Ok(stream)
    }
}
