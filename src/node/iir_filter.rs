use super::AudioNode;
use crate::{
    alloc::AudioBuffer,
    buffer::{ChannelConfig, ChannelConfigOptions},
    context::{AsBaseAudioContext, AudioContextRegistration},
    process::{AudioParamValues, AudioProcessor},
    SampleRate,
};
use num_complex::Complex;
use std::f64::consts::PI;

const MAX_IIR_COEFFS_LEN: usize = 20;

/// The IirFilterOptions is used to specify the filter coefficients
pub struct IirFilterOptions {
    /// audio node options
    pub channel_config: ChannelConfigOptions,
    /// feedforward coefficients
    pub feedforward: Vec<f64>,
    /// feedback coefficients
    pub feedback: Vec<f64>,
}

/// An AudioNode implementing a general IIR filter
pub struct IirFilterNode {
    sample_rate: f64,
    registration: AudioContextRegistration,
    channel_config: ChannelConfig,
    feedforward: Vec<f64>,
    feedback: Vec<f64>,
}

impl AudioNode for IirFilterNode {
    fn registration(&self) -> &AudioContextRegistration {
        &self.registration
    }

    fn channel_config_raw(&self) -> &ChannelConfig {
        &self.channel_config
    }

    fn number_of_inputs(&self) -> u32 {
        1
    }
    fn number_of_outputs(&self) -> u32 {
        1
    }
}

impl IirFilterNode {
    /// Creates an IirFilterNode
    ///
    /// # Arguments
    ///
    /// * `context` - Audio context in which the node will live
    /// * `options` - node options
    pub fn new<C: AsBaseAudioContext>(context: &C, options: IirFilterOptions) -> Self {
        context.base().register(move |registration| {
            let IirFilterOptions {
                feedforward,
                feedback,
                channel_config,
            } = options;

            assert!(feedforward.len() <= MAX_IIR_COEFFS_LEN, "NotSupportedError");
            assert!(!feedforward.is_empty(), "NotSupportedError");
            assert!(!feedforward.iter().all(|&ff| ff == 0.), "InvalidStateError");
            assert!(feedback.len() <= MAX_IIR_COEFFS_LEN, "NotSupportedError");
            assert!(!feedback.is_empty(), "NotSupportedError");
            assert!(!feedback.iter().all(|&ff| ff == 0.), "InvalidStateError");

            let sample_rate = context.base().sample_rate().0 as f64;

            let config = RendererConfig {
                feedforward: feedforward.clone(),
                feedback: feedback.clone(),
            };

            let render = IirFilterRenderer::new(config);

            let node = IirFilterNode {
                sample_rate,
                registration,
                channel_config: channel_config.into(),
                feedforward,
                feedback,
            };

            (node, Box::new(render))
        })
    }

    /// Returns the frequency response for the specified frequencies
    ///
    /// # Arguments
    ///
    /// * `frequency_hz` - frequencies for which frequency response of the filter should be calculated
    /// * `mag_response` - magnitude of the frequency response of the filter
    /// * `phase_response` - phase of the frequency response of the filter
    pub fn get_frequency_response(
        &self,
        frequency_hz: &[f32],
        mag_response: &mut [f32],
        phase_response: &mut [f32],
    ) {
        for (i, &f) in frequency_hz.iter().enumerate() {
            let mut num: Complex<f64> = Complex::new(0., 0.);
            let mut denom: Complex<f64> = Complex::new(0., 0.);

            for (idx, &ff) in self.feedforward.iter().enumerate() {
                num +=
                    Complex::from_polar(ff, idx as f64 * -2.0 * PI * f as f64 / self.sample_rate);
            }

            for (idx, &fb) in self.feedback.iter().enumerate() {
                denom +=
                    Complex::from_polar(fb, idx as f64 * -2.0 * PI * f as f64 / self.sample_rate);
            }

            let h_f = num / denom;

            mag_response[i] = h_f.norm() as f32;
            phase_response[i] = h_f.arg() as f32;
        }
    }
}

struct RendererConfig {
    feedforward: Vec<f64>,
    feedback: Vec<f64>,
}

/// Renderer associated with the IirFilterNode
struct IirFilterRenderer {
    norm_coeffs: Vec<(f64, f64)>,
    states: Vec<f64>,
}

impl AudioProcessor for IirFilterRenderer {
    fn process(
        &mut self,
        inputs: &[crate::alloc::AudioBuffer],
        outputs: &mut [crate::alloc::AudioBuffer],
        _params: AudioParamValues,
        _timestamp: f64,
        _sample_rate: SampleRate,
    ) {
        // single input/output node
        let input = &inputs[0];
        let output = &mut outputs[0];

        self.filter(input, output);
    }

    fn tail_time(&self) -> bool {
        false
    }
}

impl IirFilterRenderer {
    fn new(config: RendererConfig) -> Self {
        let RendererConfig {
            feedforward,
            feedback,
        } = config;

        let coeffs = Self::build_coeffs(feedforward, feedback);
        let norm_coeffs = Self::normalize_coeffs(coeffs);
        let states = Self::build_filter_states(&norm_coeffs);

        Self {
            norm_coeffs,
            states,
        }
    }

    #[inline]
    fn build_coeffs(mut feedforward: Vec<f64>, mut feedback: Vec<f64>) -> Vec<(f64, f64)> {
        match (feedforward.len(), feedback.len()) {
            (ffs_len, fbs_len) if ffs_len > fbs_len => {
                feedforward = feedforward
                    .into_iter()
                    .chain(std::iter::repeat(0.))
                    .take(fbs_len)
                    .collect();
            }
            (ffs_len, fbs_len) if ffs_len < fbs_len => {
                feedback = feedback
                    .into_iter()
                    .chain(std::iter::repeat(0.))
                    .take(ffs_len)
                    .collect();
            }
            _ => (),
        };

        let coeffs: Vec<(f64, f64)> = feedforward.into_iter().zip(feedback).collect();

        coeffs
    }

    #[inline]
    fn normalize_coeffs(coeffs: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
        let a_0 = coeffs[0].1;

        coeffs.iter().map(|(ff, fb)| (ff / a_0, fb / a_0)).collect()
    }

    #[inline]
    fn build_filter_states(coeffs: &[(f64, f64)]) -> Vec<f64> {
        let coeffs_len = coeffs.len();
        vec![0.; coeffs_len - 1]
    }

    /// Generate an output by filtering the input
    ///
    /// # Arguments
    ///
    /// * `input` - Audiobuffer input
    /// * `output` - Audiobuffer output
    #[inline]
    fn filter(&mut self, input: &AudioBuffer, output: &mut AudioBuffer) {
        for (i_data, o_data) in input.channels().iter().zip(output.channels_mut()) {
            for (&i, o) in i_data.iter().zip(o_data.iter_mut()) {
                *o = self.tick(i);
            }
        }
    }

    /// Generate an output sample by filtering an input sample
    ///
    /// # Arguments
    ///
    /// * `input` - Audiobuffer input
    #[inline]
    fn tick(&mut self, input: f32) -> f32 {
        let input = input as f64;
        let output = self.norm_coeffs[0].0 * input + self.states[0];

        for (idx, (ff, fb)) in self.norm_coeffs.iter().skip(1).enumerate() {
            self.states[idx] = ff * input - fb * output + self.states.get(idx + 1).unwrap_or(&0.);
        }
        output as f32
    }
}
