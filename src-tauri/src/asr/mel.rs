// Mel spectrogram computation for Parakeet ASR
// Converts raw audio samples to log-mel spectrogram features
//
// Parameters match NeMo's default preprocessing:
// - Sample rate: 16000 Hz
// - Window size: 25ms (400 samples)
// - Hop size: 10ms (160 samples)
// - FFT size: 512
// - Mel bins: 80
// - Frequency range: 0 - 8000 Hz

use ndarray::Array2;
use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;

/// Configuration for mel spectrogram computation
pub struct MelConfig {
    pub sample_rate: u32,
    pub n_fft: usize,
    pub hop_length: usize,
    pub win_length: usize,
    pub n_mels: usize,
    pub f_min: f32,
    pub f_max: f32,
}

impl Default for MelConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            n_fft: 512,
            hop_length: 160,   // 10ms at 16kHz
            win_length: 400,   // 25ms at 16kHz
            n_mels: 80,
            f_min: 0.0,
            f_max: 8000.0,
        }
    }
}

/// Compute log-mel spectrogram from audio samples
pub struct MelSpectrogram {
    config: MelConfig,
    mel_filterbank: Array2<f32>,
    window: Vec<f32>,
}

impl MelSpectrogram {
    pub fn new(config: MelConfig) -> Self {
        let mel_filterbank = create_mel_filterbank(
            config.n_fft,
            config.n_mels,
            config.sample_rate,
            config.f_min,
            config.f_max,
        );
        let window = hann_window(config.win_length);
        Self {
            config,
            mel_filterbank,
            window,
        }
    }

    /// Compute log-mel spectrogram features
    /// Input: raw audio samples (16kHz mono f32)
    /// Output: Array2<f32> of shape [n_mels, time_steps]
    pub fn compute(&self, samples: &[f32]) -> Array2<f32> {
        let n_fft = self.config.n_fft;
        let hop = self.config.hop_length;
        let win_len = self.config.win_length;

        // Number of frames
        let n_frames = if samples.len() >= win_len {
            (samples.len() - win_len) / hop + 1
        } else {
            0
        };

        if n_frames == 0 {
            return Array2::zeros((self.config.n_mels, 0));
        }

        // FFT setup
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(n_fft);
        let n_freq = n_fft / 2 + 1;

        // Compute STFT magnitude squared
        let mut power_spec = Array2::zeros((n_freq, n_frames));

        for frame_idx in 0..n_frames {
            let start = frame_idx * hop;

            // Apply window and zero-pad to n_fft
            let mut buffer: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); n_fft];
            for i in 0..win_len.min(samples.len() - start) {
                buffer[i] = Complex::new(samples[start + i] * self.window[i], 0.0);
            }

            // FFT
            fft.process(&mut buffer);

            // Power spectrum (magnitude squared)
            for freq_idx in 0..n_freq {
                let mag_sq = buffer[freq_idx].norm_sqr();
                power_spec[[freq_idx, frame_idx]] = mag_sq;
            }
        }

        // Apply mel filterbank: [n_mels, n_freq] x [n_freq, n_frames] = [n_mels, n_frames]
        let mel_spec = self.mel_filterbank.dot(&power_spec);

        // Log mel spectrogram (with small epsilon to avoid log(0))
        mel_spec.mapv(|x| (x.max(1e-10)).ln())
    }
}

/// Create a Hanning window
fn hann_window(length: usize) -> Vec<f32> {
    (0..length)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / length as f32).cos()))
        .collect()
}

/// Convert frequency in Hz to mel scale
fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

/// Convert mel scale to frequency in Hz
fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0)
}

/// Create mel filterbank matrix of shape [n_mels, n_fft/2 + 1]
fn create_mel_filterbank(
    n_fft: usize,
    n_mels: usize,
    sample_rate: u32,
    f_min: f32,
    f_max: f32,
) -> Array2<f32> {
    let n_freq = n_fft / 2 + 1;
    let mel_min = hz_to_mel(f_min);
    let mel_max = hz_to_mel(f_max);

    // Create n_mels + 2 equally spaced points on mel scale
    let mel_points: Vec<f32> = (0..=n_mels + 1)
        .map(|i| mel_min + (mel_max - mel_min) * i as f32 / (n_mels + 1) as f32)
        .collect();

    // Convert back to Hz and then to FFT bin indices
    let hz_points: Vec<f32> = mel_points.iter().map(|&m| mel_to_hz(m)).collect();
    let bin_points: Vec<f32> = hz_points
        .iter()
        .map(|&hz| hz * n_fft as f32 / sample_rate as f32)
        .collect();

    let mut filterbank = Array2::zeros((n_mels, n_freq));

    for mel_idx in 0..n_mels {
        let left = bin_points[mel_idx];
        let center = bin_points[mel_idx + 1];
        let right = bin_points[mel_idx + 2];

        for freq_idx in 0..n_freq {
            let freq = freq_idx as f32;

            if freq >= left && freq <= center && center > left {
                filterbank[[mel_idx, freq_idx]] = (freq - left) / (center - left);
            } else if freq > center && freq <= right && right > center {
                filterbank[[mel_idx, freq_idx]] = (right - freq) / (right - center);
            }
        }
    }

    filterbank
}

impl Default for MelSpectrogram {
    fn default() -> Self {
        Self::new(MelConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mel_spectrogram_basic() {
        let mel = MelSpectrogram::default();
        // 1 second of silence at 16kHz
        let samples = vec![0.0f32; 16000];
        let result = mel.compute(&samples);
        assert_eq!(result.shape()[0], 80); // 80 mel bins
        assert!(result.shape()[1] > 0); // has time steps
    }

    #[test]
    fn test_mel_filterbank_shape() {
        let fb = create_mel_filterbank(512, 80, 16000, 0.0, 8000.0);
        assert_eq!(fb.shape(), &[80, 257]); // 80 mels x (512/2 + 1) freq bins
    }

    #[test]
    fn test_hz_mel_roundtrip() {
        let hz = 1000.0;
        let mel = hz_to_mel(hz);
        let hz2 = mel_to_hz(mel);
        assert!((hz - hz2).abs() < 0.01);
    }
}
