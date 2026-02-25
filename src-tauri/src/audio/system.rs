// System audio capture via ScreenCaptureKit (macOS 12.3+)
// Captures all application audio output — works regardless of speaker volume or
// whether the user is wearing headphones/AirPods.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

// ─── SCK handler (macOS only) ─────────────────────────────────────────────────

#[cfg(target_os = "macos")]
use screencapturekit::prelude::*;

#[cfg(target_os = "macos")]
struct SystemAudioHandler {
    samples: Arc<Mutex<Vec<f32>>>,
}

#[cfg(target_os = "macos")]
impl SCStreamOutputTrait for SystemAudioHandler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, of_type: SCStreamOutputType) {
        if of_type != SCStreamOutputType::Audio {
            return;
        }
        if let Some(buffer_list) = sample.audio_buffer_list() {
            let mut guard = self.samples.lock().unwrap();
            for buf in &buffer_list {
                let bytes = buf.data();
                if bytes.is_empty() {
                    continue;
                }
                // SCK delivers native f32 non-interleaved PCM.
                // CoreAudio guarantees 4-byte alignment for mData, but verify defensively.
                if bytes.as_ptr().align_offset(std::mem::align_of::<f32>()) != 0 {
                    log::warn!("SCK audio buffer not f32-aligned, skipping");
                    continue;
                }
                let floats = unsafe {
                    std::slice::from_raw_parts(
                        bytes.as_ptr() as *const f32,
                        bytes.len() / std::mem::size_of::<f32>(),
                    )
                };
                guard.extend_from_slice(floats);
            }
        }
    }
}

// ─── Public struct ────────────────────────────────────────────────────────────

/// Captures system-wide audio output via ScreenCaptureKit.
///
/// On non-macOS platforms this is a no-op stub so the rest of the codebase
/// compiles without platform-specific guards everywhere.
pub struct SystemAudioCapture {
    samples: Arc<Mutex<Vec<f32>>>,
    is_running: Arc<AtomicBool>,
    #[cfg(target_os = "macos")]
    stream: Option<SCStream>,
}

// SCStream holds ObjC objects; we always access it from a single async task.
unsafe impl Send for SystemAudioCapture {}

impl SystemAudioCapture {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            is_running: Arc::new(AtomicBool::new(false)),
            #[cfg(target_os = "macos")]
            stream: None,
        }
    }

    /// Start capturing system audio. Returns an error string on failure.
    pub fn start(&mut self) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            // Enumerate available displays — required for SCContentFilter even in audio-only mode.
            let content = SCShareableContent::get()
                .map_err(|e| format!("SCShareableContent::get failed: {e:?}"))?;

            let display = content
                .displays()
                .into_iter()
                .next()
                .ok_or_else(|| "No display found for system audio capture".to_string())?;

            // Build a content filter for the primary display.
            let filter = SCContentFilter::create()
                .with_display(&display)
                .with_excluding_windows(&[])
                .build();

            // Configure for audio-only at 16 kHz mono to match the mic pipeline.
            // Width/height are set to minimum values since we only want audio.
            let config = SCStreamConfiguration::new()
                .with_width(2)
                .with_height(2)
                .with_captures_audio(true)
                .with_excludes_current_process_audio(true)
                .with_sample_rate(16000)
                .with_channel_count(1);

            let handler = SystemAudioHandler {
                samples: self.samples.clone(),
            };

            let mut stream = SCStream::new(&filter, &config);
            stream.add_output_handler(handler, SCStreamOutputType::Audio);
            stream
                .start_capture()
                .map_err(|e| format!("SCStream::start_capture failed: {e:?}"))?;

            self.stream = Some(stream);
            self.is_running.store(true, Ordering::SeqCst);
            log::info!("System audio capture started (ScreenCaptureKit, 16 kHz mono)");
            return Ok(());
        }

        #[cfg(not(target_os = "macos"))]
        Err("System audio capture requires macOS 12.3+".to_string())
    }

    /// Stop capturing and release SCK resources.
    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        #[cfg(target_os = "macos")]
        if let Some(stream) = self.stream.take() {
            let _ = stream.stop_capture();
            log::info!("System audio capture stopped");
        }
    }

    /// Drain and return all buffered PCM samples since the last call.
    pub fn get_samples(&self) -> Vec<f32> {
        self.samples
            .lock()
            .map(|mut buf| {
                let out = buf.clone();
                buf.clear();
                out
            })
            .unwrap_or_default()
    }

    /// Always 16 000 Hz — configured at start.
    pub fn sample_rate(&self) -> u32 {
        16000
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

impl Drop for SystemAudioCapture {
    fn drop(&mut self) {
        self.stop();
    }
}
