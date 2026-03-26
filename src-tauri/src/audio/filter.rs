//! Optional light DSP chain for dictation audio (after resample, before STT).
//! Kept conservative so cloud and local Whisper/SenseVoice models see natural speech spectra.

use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// UI preset: persisted so the Settings dropdown matches after restart.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "PascalCase")]
pub enum AudioFilterPreset {
    /// No processing (same as disabled).
    #[default]
    Off,
    Light,
    Moderate,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioFilterConfig {
    pub enabled: bool,
    #[serde(default)]
    pub preset: AudioFilterPreset,
    pub highpass_cutoff_hz: f32,
    pub noise_gate_threshold_db: f32,
    pub compressor_ratio: f32,
    pub compressor_threshold_db: f32,
    pub normalize_target_db: f32,
}

impl Default for AudioFilterConfig {
    /// New installs get Light: normalization ensures the VAD threshold works
    /// consistently regardless of mic gain level.
    fn default() -> Self {
        Self::light_values()
    }
}

impl AudioFilterConfig {
    /// Default persisted state: filter off, Light-style numbers kept for when user enables Light/Custom.
    pub fn off_values() -> Self {
        Self {
            enabled: false,
            preset: AudioFilterPreset::Off,
            highpass_cutoff_hz: 80.0,
            noise_gate_threshold_db: -45.0,
            compressor_ratio: 3.0,
            compressor_threshold_db: -18.0,
            normalize_target_db: -6.0,
        }
    }

    /// Light preset parameters (`enabled` + `preset` set for an active Light profile).
    pub fn light_values() -> Self {
        Self {
            enabled: true,
            preset: AudioFilterPreset::Light,
            highpass_cutoff_hz: 80.0,
            noise_gate_threshold_db: -45.0,
            compressor_ratio: 3.0,
            compressor_threshold_db: -18.0,
            normalize_target_db: -6.0,
        }
    }

    pub fn moderate_values() -> Self {
        Self {
            enabled: true,
            preset: AudioFilterPreset::Moderate,
            highpass_cutoff_hz: 100.0,
            noise_gate_threshold_db: -40.0,
            compressor_ratio: 4.0,
            compressor_threshold_db: -15.0,
            normalize_target_db: -3.0,
        }
    }
}

/// Apply the full chain in-place when the user chose an active preset and `enabled` is true.
pub fn apply_filter_chain(samples: &mut [f32], config: &AudioFilterConfig, sample_rate: u32) {
    if !config.enabled
        || matches!(config.preset, AudioFilterPreset::Off)
        || samples.is_empty()
        || sample_rate == 0
    {
        return;
    }
    let sr = sample_rate as f32;
    apply_highpass(samples, config.highpass_cutoff_hz.clamp(20.0, 500.0), sr);
    apply_noise_gate(
        samples,
        config.noise_gate_threshold_db.clamp(-80.0, 0.0),
        sr,
    );
    apply_compressor(
        samples,
        config.compressor_ratio.clamp(1.0, 20.0),
        config.compressor_threshold_db.clamp(-60.0, 0.0),
        sr,
    );
    apply_normalize(samples, config.normalize_target_db.clamp(-24.0, 0.0));
}

/// Single-pole high-pass IIR: removes rumble and very low-frequency noise.
fn apply_highpass(samples: &mut [f32], cutoff_hz: f32, sample_rate: f32) {
    let fc = cutoff_hz.max(1.0);
    let rc = 1.0 / (2.0 * PI * fc);
    let dt = 1.0 / sample_rate;
    let alpha = rc / (rc + dt);
    let mut prev_x = samples.first().copied().unwrap_or(0.0);
    let mut prev_y = 0.0f32;
    for x in samples.iter_mut() {
        let xi = *x;
        let y = alpha * (prev_y + xi - prev_x);
        prev_x = xi;
        prev_y = y;
        *x = y;
    }
}

fn db_to_linear(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// Envelope follower + smoothed gain; attenuates when envelope sits below threshold (soft knee).
fn apply_noise_gate(samples: &mut [f32], threshold_db: f32, sample_rate: f32) {
    let threshold = db_to_linear(threshold_db).max(1e-6);
    // ~5 ms attack, ~50 ms release on the envelope follower.
    let attack = 1.0 - (-1.0f32 / (sample_rate * 0.005f32).max(1.0)).exp();
    let release = 1.0 - (-1.0f32 / (sample_rate * 0.050f32).max(1.0)).exp();
    // Smooth applied gain to avoid zipper noise (~5 ms).
    let gain_smooth = 1.0 - (-1.0f32 / (sample_rate * 0.005f32).max(1.0)).exp();

    let mut envelope = 0.0f32;
    let mut gain = 1.0f32;
    for x in samples.iter_mut() {
        let ax = x.abs();
        let coeff = if ax > envelope { attack } else { release };
        envelope += (ax - envelope) * coeff;

        // Soft knee: below threshold, partially attenuate (floor avoids total silence / pumping).
        let target = if envelope < threshold {
            let floor_gain = 0.08f32;
            floor_gain + (1.0 - floor_gain) * (envelope / threshold).powf(0.85)
        } else {
            1.0
        };
        gain += (target - gain) * gain_smooth;
        *x *= gain;
    }
}

/// Simple feed-forward compressor on absolute level with smoothed gain reduction.
fn apply_compressor(samples: &mut [f32], ratio: f32, threshold_db: f32, sample_rate: f32) {
    let threshold = db_to_linear(threshold_db).max(1e-6);
    let attack = 1.0 - (-1.0f32 / (sample_rate * 0.010f32).max(1.0)).exp();
    let release = 1.0 - (-1.0f32 / (sample_rate * 0.100f32).max(1.0)).exp();
    let mut envelope = 0.0f32;
    let mut gain = 1.0f32;

    for x in samples.iter_mut() {
        let ax = x.abs();
        let coeff = if ax > envelope { attack } else { release };
        envelope += (ax - envelope) * coeff;

        let target_gain = if envelope > threshold {
            let over = envelope - threshold;
            let compressed = threshold + over / ratio;
            (compressed / envelope.max(1e-8)).min(1.0)
        } else {
            1.0
        };
        let coeff_g = if target_gain < gain { attack } else { release };
        gain += (target_gain - gain) * coeff_g;
        *x *= gain;
    }
}

/// Peak normalization to target dBFS (linear amplitude relative to full scale 1.0).
fn apply_normalize(samples: &mut [f32], target_db: f32) {
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    if peak < 1e-9 {
        return;
    }
    let target = db_to_linear(target_db).clamp(0.01, 1.0);
    let scale = target / peak;
    for s in samples.iter_mut() {
        *s = (*s * scale).clamp(-1.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_noop_when_off() {
        let mut v = vec![0.5f32, -0.5, 0.25];
        let orig = v.clone();
        let cfg = AudioFilterConfig::off_values();
        apply_filter_chain(&mut v, &cfg, 16_000);
        assert_eq!(v, orig);
    }

    #[test]
    fn default_is_light_and_active() {
        let cfg = AudioFilterConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.preset, AudioFilterPreset::Light);
    }

    #[test]
    fn chain_noop_when_preset_off_even_if_enabled_flag_true() {
        let mut v = vec![0.5f32, -0.5, 0.25];
        let orig = v.clone();
        let mut cfg = AudioFilterConfig::light_values();
        cfg.enabled = true;
        cfg.preset = AudioFilterPreset::Off;
        apply_filter_chain(&mut v, &cfg, 16_000);
        assert_eq!(v, orig);
    }

    #[test]
    fn normalize_scales_peak() {
        let mut v = vec![0.1f32, -0.2, 0.05];
        apply_normalize(&mut v, -6.0);
        let peak = v.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let want = db_to_linear(-6.0);
        assert!((peak - want).abs() < 0.02);
    }

    #[test]
    fn highpass_reduces_dc() {
        let mut v = vec![1.0f32; 512];
        apply_highpass(&mut v, 80.0, 16_000.0);
        let mean: f32 = v.iter().sum::<f32>() / v.len() as f32;
        assert!(mean.abs() < 0.2, "mean={}", mean);
    }
}
