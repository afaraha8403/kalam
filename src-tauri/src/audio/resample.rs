#![allow(dead_code)]

use rubato::{
    Resampler as RubatoResampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

const TARGET_SAMPLE_RATE: u32 = 16000;
const INPUT_BLOCK_LEN: usize = 1024;
const CHANNELS: usize = 1;

pub struct Resampler {
    inner: Option<SincFixedIn<f32>>,
    input_rate: usize,
    output_rate: usize,
}

impl Resampler {
    pub fn new(
        input_sample_rate: usize,
        output_sample_rate: usize,
        _channels: usize,
    ) -> anyhow::Result<Self> {
        if input_sample_rate == output_sample_rate {
            return Ok(Self {
                inner: None,
                input_rate: input_sample_rate,
                output_rate: output_sample_rate,
            });
        }

        let ratio = output_sample_rate as f64 / input_sample_rate as f64;
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };
        let inner = SincFixedIn::<f32>::new(
            ratio,
            2.0,
            params,
            INPUT_BLOCK_LEN,
            CHANNELS,
        )
        .map_err(|e| anyhow::anyhow!("Resampler init: {}", e))?;

        Ok(Self {
            inner: Some(inner),
            input_rate: input_sample_rate,
            output_rate: output_sample_rate,
        })
    }

    pub fn process(&mut self, input: &[f32]) -> anyhow::Result<Vec<f32>> {
        let Some(ref mut resampler) = self.inner else {
            return Ok(input.to_vec());
        };

        let mut output = Vec::with_capacity(
            (input.len() * self.output_rate / self.input_rate) + INPUT_BLOCK_LEN,
        );
        let mut pos = 0;

        while pos < input.len() {
            let remaining = input.len() - pos;
            let take = remaining.min(INPUT_BLOCK_LEN);
            let mut block: Vec<f32> = input[pos..pos + take].to_vec();
            pos += take;

            if block.len() < INPUT_BLOCK_LEN {
                block.resize(INPUT_BLOCK_LEN, 0.0);
            }

            let waves_in = vec![block];
            let out = resampler
                .process(&waves_in, None)
                .map_err(|e| anyhow::anyhow!("Resampler process: {}", e))?;

            if let Some(channel) = out.first() {
                let to_push = if take < INPUT_BLOCK_LEN {
                    let expected = (take as f64 * self.output_rate as f64 / self.input_rate as f64)
                        .ceil() as usize;
                    channel.len().min(expected)
                } else {
                    channel.len()
                };
                output.extend_from_slice(&channel[..to_push]);
            }
        }

        Ok(output)
    }
}

/// One-shot resample from input_sample_rate to 16kHz mono. Convenience for capture pipeline.
pub fn resample_to_16k_mono(input: &[f32], input_sample_rate: u32) -> anyhow::Result<Vec<f32>> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    if input_sample_rate == TARGET_SAMPLE_RATE {
        return Ok(input.to_vec());
    }
    let mut r = Resampler::new(
        input_sample_rate as usize,
        TARGET_SAMPLE_RATE as usize,
        1,
    )?;
    r.process(input)
}
