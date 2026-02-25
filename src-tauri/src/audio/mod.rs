// Audio capture module
// Handles audio input capture for macOS and Windows

pub mod system;
pub use system::SystemAudioCapture;

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, Stream, StreamConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const BUFFER_SECONDS: usize = 30;

pub struct AudioCapture {
    host: Host,
    device: Option<Device>,
    stream: Option<Stream>,
    samples: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
    sample_rate: u32,
    recent_samples: Arc<Mutex<Vec<f32>>>, // For RMS calculation
}

#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub name: String,
    pub is_default: bool,
}

// Make AudioCapture Send by storing stream separately
unsafe impl Send for AudioCapture {}

impl AudioCapture {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();

        Ok(Self {
            host,
            device: None,
            stream: None,
            samples: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(AtomicBool::new(false)),
            sample_rate: 16000,
            recent_samples: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// List available input devices
    pub fn list_devices(&self) -> Result<Vec<AudioDevice>> {
        let default_device = self.host.default_input_device();
        let default_name = default_device.as_ref().and_then(|d| d.name().ok());

        let devices: Vec<AudioDevice> = self
            .host
            .input_devices()?
            .filter_map(|device| {
                device.name().ok().map(|name| {
                    let is_default = default_name.as_ref() == Some(&name);
                    AudioDevice { name, is_default }
                })
            })
            .collect();

        Ok(devices)
    }

    /// Select input device by name, or use default if None
    pub fn select_device(&mut self, device_name: Option<&str>) -> Result<()> {
        self.device = match device_name {
            Some(name) => self
                .host
                .input_devices()?
                .find(|d| d.name().ok().as_deref() == Some(name)),
            None => self.host.default_input_device(),
        };

        if self.device.is_none() {
            return Err(anyhow!("No audio input device found"));
        }

        log::info!(
            "Selected audio device: {:?}",
            self.device.as_ref().and_then(|d| d.name().ok())
        );
        Ok(())
    }

    /// Start capturing audio
    pub fn start(&mut self) -> Result<()> {
        if self.is_recording.load(Ordering::SeqCst) {
            return Ok(());
        }

        if self.device.is_none() {
            self.select_device(None)?;
        }

        let device = self
            .device
            .as_ref()
            .ok_or_else(|| anyhow!("No device selected"))?;
        let supported_config = device.default_input_config()?;

        log::info!("Device config: {:?}", supported_config);

        self.sample_rate = supported_config.sample_rate().0;
        self.is_recording.store(true, Ordering::SeqCst);

        // Clear existing samples
        if let Ok(mut samples) = self.samples.lock() {
            samples.clear();
            // Pre-allocate for 30 seconds
            samples.reserve(self.sample_rate as usize * BUFFER_SECONDS);
        }

        // Clear recent samples for RMS calculation
        if let Ok(mut recent) = self.recent_samples.lock() {
            recent.clear();
        }

        let config = StreamConfig {
            channels: supported_config.channels(),
            sample_rate: supported_config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        let samples = self.samples.clone();
        let recent_samples = self.recent_samples.clone();
        let channels = config.channels as usize;
        let err_fn = |err| log::error!("Audio stream error: {}", err);

        let stream = match supported_config.sample_format() {
            SampleFormat::F32 => {
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buf) = samples.lock() {
                            // Convert to mono
                            if channels > 1 {
                                for chunk in data.chunks(channels) {
                                    let mono = chunk.iter().sum::<f32>() / channels as f32;
                                    buf.push(mono);
                                }
                            } else {
                                buf.extend_from_slice(data);
                            }
                        }
                        // Store recent samples for RMS calculation
                        if let Ok(mut recent) = recent_samples.lock() {
                            recent.clear();
                            if channels > 1 {
                                for chunk in data.chunks(channels) {
                                    let mono = chunk.iter().sum::<f32>() / channels as f32;
                                    recent.push(mono);
                                }
                            } else {
                                recent.extend_from_slice(data);
                            }
                            // Keep only last 1024 samples
                            if recent.len() > 1024 {
                                let excess = recent.len() - 1024;
                                recent.drain(0..excess);
                            }
                        }
                    },
                    err_fn,
                    None,
                )?
            }
            SampleFormat::I16 => {
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buf) = samples.lock() {
                            if channels > 1 {
                                for chunk in data.chunks(channels) {
                                    let mono = chunk
                                        .iter()
                                        .map(|&s| s as f32 / i16::MAX as f32)
                                        .sum::<f32>()
                                        / channels as f32;
                                    buf.push(mono);
                                }
                            } else {
                                for &s in data {
                                    buf.push(s as f32 / i16::MAX as f32);
                                }
                            }
                        }
                        // Store recent samples for RMS calculation
                        if let Ok(mut recent) = recent_samples.lock() {
                            recent.clear();
                            if channels > 1 {
                                for chunk in data.chunks(channels) {
                                    let mono = chunk
                                        .iter()
                                        .map(|&s| s as f32 / i16::MAX as f32)
                                        .sum::<f32>()
                                        / channels as f32;
                                    recent.push(mono);
                                }
                            } else {
                                for &s in data {
                                    recent.push(s as f32 / i16::MAX as f32);
                                }
                            }
                            if recent.len() > 1024 {
                                let excess = recent.len() - 1024;
                                recent.drain(0..excess);
                            }
                        }
                    },
                    err_fn,
                    None,
                )?
            }
            _ => return Err(anyhow!("Unsupported sample format")),
        };

        stream.play()?;
        self.stream = Some(stream);

        log::info!("Audio capture started at {} Hz", self.sample_rate);
        Ok(())
    }

    /// Stop capturing audio
    pub fn stop(&mut self) -> Result<()> {
        self.is_recording.store(false, Ordering::SeqCst);
        self.stream = None;
        log::info!("Audio capture stopped");
        Ok(())
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    /// Get the current sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get all audio samples and clear buffer
    pub fn get_samples(&self) -> Vec<f32> {
        if let Ok(mut samples) = self.samples.lock() {
            let result = samples.clone();
            samples.clear();
            result
        } else {
            vec![]
        }
    }

    /// Get available sample count
    pub fn available_samples(&self) -> usize {
        self.samples.lock().map(|s| s.len()).unwrap_or(0)
    }

    /// Get the current RMS audio level (0.0 to 1.0)
    pub fn get_rms_level(&self) -> f32 {
        if let Ok(samples) = self.recent_samples.lock() {
            if samples.is_empty() {
                return 0.0;
            }
            let sum: f32 = samples.iter().map(|s| s * s).sum();
            let rms = (sum / samples.len() as f32).sqrt();
            // Normalize to 0-1 range (assuming 16-bit equivalent)
            (rms * 3.0).min(1.0) // Scale up for better visualization
        } else {
            0.0
        }
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
