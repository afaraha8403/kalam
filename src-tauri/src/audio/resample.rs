#![allow(dead_code)]

// Simplified resampler - just passes through for now
// TODO: Implement proper resampling with rubato
pub struct Resampler;

impl Resampler {
    pub fn new(
        _input_sample_rate: usize,
        _output_sample_rate: usize,
        _channels: usize,
    ) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn process(&mut self, input: &[f32]) -> anyhow::Result<Vec<f32>> {
        // For now, just return the input unchanged
        // This works if input is already 16kHz, which most mics aren't
        // A proper implementation would use rubato to resample
        Ok(input.to_vec())
    }
}
