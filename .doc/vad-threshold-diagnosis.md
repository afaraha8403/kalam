# VAD Threshold Diagnosis: Dictation Truncation on Quiet Microphones

## Symptom

User dictated for ~58 seconds but only 19 characters were transcribed.
The app captured 933,600 samples (58.35s at 16 kHz) but VAD kept only 29,172
samples (~1.8s) — discarding 97% of the recording as "silence."

## Root Cause

The energy-based VAD in `src-tauri/src/audio/vad.rs` compared raw sample
amplitude (`sample.abs()`) against a fixed `speech_threshold` of **0.5**.

```
let energy = sample.abs();
let is_speech = energy > self.config.speech_threshold;  // 0.5
```

A threshold of 0.5 means the sample must reach 50% of full scale to count as
speech. Normal microphone input produces samples in the **0.01–0.10** range for
typical speech — 5–50× below the threshold. Only the loudest transients
(plosives, shouts) ever crossed 0.5, so the VAD found a single ~1.8s segment
out of a full minute of speech.

### Why the threshold was 0.5

The code was scaffolded for Silero VAD (neural network), where the threshold is
a **probability score** (0.0–1.0, with 0.5 meaning "50% confident this is
speech"). The `silero_model_path()` function and comments in `vad.rs` confirm
this intent. However, Silero was never integrated (no ONNX runtime in
`Cargo.toml`), so the fallback `process_energy()` method runs instead — and it
uses the same 0.5 value as a raw amplitude threshold, which is the wrong scale.

### Compounding factor: audio filter defaulted to Off

The audio filter chain (`src-tauri/src/audio/filter.rs`) includes peak
normalization that boosts quiet audio to a target level (−6 dBFS for Light
preset). This filter runs **before** VAD in the transcription pipeline
(`lib.rs:3054`). If enabled, it would have normalized the quiet mic input to
usable levels, making the 0.5 threshold reachable.

But `AudioFilterConfig::default()` returned `off_values()` — filter disabled.
New installs got no normalization, so the VAD saw raw, quiet samples.

## Evidence from Logs

```
01:45:02 Audio recording started
01:46:01 Audio recording stopped, 933600 samples at 16000Hz   ← 58s of audio
01:46:01 STT chunking: vad_segments=1 chunked_windows=1       ← VAD found 1 segment
01:46:01 Creating WAV: 29172 samples at 16000Hz               ← only 1.8s kept
01:46:01 Transcription completed, length: 19                   ← 19 chars returned
```

Earlier test recordings confirmed the mic RMS was ~0.01 (line 542–547):

```
Test stopped: 49280 samples, RMS: 0.010598031, sample_rate: 16000
```

RMS of 0.01 means peak amplitude was roughly 0.03–0.05 — well below the 0.5
threshold.

## Fix (Phase 1)

### 1. Lower VAD `speech_threshold` to raw-amplitude-appropriate values

| Preset   | Old threshold | New threshold | Rationale                                    |
|----------|---------------|---------------|----------------------------------------------|
| Balanced | 0.5           | 0.02          | Catches typical speech, ignores mic noise    |
| Fast     | 0.5           | 0.03          | Slightly less sensitive for quicker cutoff   |
| Accurate | 0.5           | 0.01          | More sensitive, catches quieter speech       |

### 2. Default audio filter to Light (enabled)

`AudioFilterConfig::default()` now returns `light_values()` instead of
`off_values()`. New installs get normalization before VAD, making the threshold
behave consistently across all mic gain levels.

Existing users who explicitly set the filter to Off keep their setting — the
config is loaded from `config.json` and the Rust default only applies when the
field is absent.

### Files changed

| File | Change |
|------|--------|
| `src-tauri/src/audio/vad.rs` | `speech_threshold` 0.5 → 0.02; Fast 0.03, Accurate 0.01; added comment clarifying this is raw amplitude |
| `src-tauri/src/audio/filter.rs` | `Default` impl returns `light_values()`; added `default_is_light_and_active` test |
| `src/pages/Settings.svelte` | `defaultAudioFilter()` returns Light/enabled |
| `cypress/support/dev-bridge-handlers.ts` | Test mock default matches new Light preset |
| `CHANGELOG.md` | Documented the fix |

## Phase 2: Adaptive VAD Threshold (future)

Even with the filter on and a lower static threshold, different environments
(noisy room, very quiet mic, high-gain studio mic) can still cause false
positives or missed speech. The recommended next step is **adaptive threshold
estimation**:

1. After recording stops and the filter chain runs, compute the noise floor
   from the audio buffer (RMS of the lowest-energy 10–20% of frames).
2. Set the effective speech threshold to `noise_floor_rms * sensitivity_multiplier`.
3. The VAD preset dropdown maps to different multipliers (e.g., Fast=6×,
   Balanced=4×, Accurate=3×) instead of absolute thresholds.

This eliminates the need for users to understand amplitude values and
automatically adapts to any mic/environment combination.

### Why adaptive over relative energy (whisper.cpp style)

whisper.cpp compares recent energy (last 1s) to slightly-less-recent energy
(last 2s). This works for streaming but is a narrower solution. Kalam has the
full recording buffer available after stop, so a global noise-floor estimate is
more robust and simpler to implement.

### Phase 3: Silero VAD (neural network)

The codebase already scaffolds Silero integration (`silero_model_path()`,
comments in `VADProcessor::new`). Rust crates exist (`silero-vad-rs`,
`silero-vad-rust` on crates.io) but add ONNX runtime as a dependency
(~10–20 MB binary size increase). Silero achieves better F1 (0.738 vs 0.680
for energy-based) on real-world audio but at 1× realtime vs 721× for
energy-based approaches.

## References

- [Simplismart VAD Parameter Tuning](https://docs.simplismart.ai/troubleshooting-faq/vad-parameter-tuning) — onset/offset threshold ranges by audio type
- [whisper.cpp VAD discussion (issue #2335)](https://github.com/ggerganov/whisper.cpp/issues/2335) — energy-based VAD uses relative energy, not absolute amplitude
- [Faster Whisper negative threshold fix (PR #1191)](https://github.com/SYSTRAN/faster-whisper/pull/1191) — similar threshold misconfiguration bug
- [Vocal.com: VAD with Adaptive Thresholding](https://vocal.com/voice-quality-enhancement/voice-activity-detection-with-adaptive-thresholding/) — adaptive noise-floor estimation algorithm
- [fast-vad vs Silero VAD benchmarks](https://themenonlab.blog/blog/fast-vad-721x-realtime-voice-activity-detection) — energy-based has higher recall (0.785 vs 0.712)
- [silero-vad-rust crate](https://crates.io/crates/silero-vad-rust) — Rust Silero integration via ONNX runtime
