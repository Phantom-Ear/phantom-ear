// Parakeet ASR Backend
// NVIDIA Parakeet CTC via ONNX Runtime
//
// Supports CTC models (0.6B, 1.1B). RNNT models are listed but not yet supported.
// English-only. Uses mel spectrogram preprocessing + ONNX inference + CTC greedy decoding.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ndarray::Array2;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::session::SessionOutputs;
use ort::value::Tensor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::backend::{AsrBackend, TranscriptionResult, TranscriptionSegment};
use super::mel::MelSpectrogram;

/// Parakeet model variants
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParakeetModel {
    /// Parakeet CTC 0.6B - fast inference
    Ctc06b,
    /// Parakeet CTC 1.1B - better accuracy
    Ctc11b,
    /// Parakeet RNNT 0.6B - streaming capable (not yet supported)
    Rnnt06b,
    /// Parakeet RNNT 1.1B - best accuracy (not yet supported)
    Rnnt11b,
}

impl ParakeetModel {
    pub fn filename(&self) -> &'static str {
        match self {
            ParakeetModel::Ctc06b => "parakeet-ctc-0.6b.onnx",
            ParakeetModel::Ctc11b => "parakeet-ctc-1.1b.onnx",
            ParakeetModel::Rnnt06b => "parakeet-rnnt-0.6b.onnx",
            ParakeetModel::Rnnt11b => "parakeet-rnnt-1.1b.onnx",
        }
    }

    pub fn vocab_filename(&self) -> &'static str {
        match self {
            ParakeetModel::Ctc06b => "parakeet-ctc-0.6b-vocab.json",
            ParakeetModel::Ctc11b => "parakeet-ctc-1.1b-vocab.json",
            ParakeetModel::Rnnt06b => "parakeet-rnnt-0.6b-vocab.json",
            ParakeetModel::Rnnt11b => "parakeet-rnnt-1.1b-vocab.json",
        }
    }

    pub fn size_mb(&self) -> u64 {
        match self {
            ParakeetModel::Ctc06b => 600,
            ParakeetModel::Ctc11b => 1100,
            ParakeetModel::Rnnt06b => 650,
            ParakeetModel::Rnnt11b => 1200,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ParakeetModel::Ctc06b => "Fast inference, good accuracy",
            ParakeetModel::Ctc11b => "Better accuracy, larger model",
            ParakeetModel::Rnnt06b => "Streaming capable, fast (not yet supported)",
            ParakeetModel::Rnnt11b => "Best accuracy, streaming (not yet supported)",
        }
    }

    pub fn is_ctc(&self) -> bool {
        matches!(self, ParakeetModel::Ctc06b | ParakeetModel::Ctc11b)
    }

    pub fn download_url(&self) -> &'static str {
        match self {
            ParakeetModel::Ctc06b => {
                "https://huggingface.co/nvidia/parakeet-ctc-0.6b/resolve/main/model.onnx"
            }
            ParakeetModel::Ctc11b => {
                "https://huggingface.co/nvidia/parakeet-ctc-1.1b/resolve/main/model.onnx"
            }
            ParakeetModel::Rnnt06b => {
                "https://huggingface.co/nvidia/parakeet-rnnt-0.6b/resolve/main/model.onnx"
            }
            ParakeetModel::Rnnt11b => {
                "https://huggingface.co/nvidia/parakeet-rnnt-1.1b/resolve/main/model.onnx"
            }
        }
    }

    pub fn vocab_url(&self) -> &'static str {
        match self {
            ParakeetModel::Ctc06b => {
                "https://huggingface.co/nvidia/parakeet-ctc-0.6b/resolve/main/vocab.json"
            }
            ParakeetModel::Ctc11b => {
                "https://huggingface.co/nvidia/parakeet-ctc-1.1b/resolve/main/vocab.json"
            }
            ParakeetModel::Rnnt06b => {
                "https://huggingface.co/nvidia/parakeet-rnnt-0.6b/resolve/main/vocab.json"
            }
            ParakeetModel::Rnnt11b => {
                "https://huggingface.co/nvidia/parakeet-rnnt-1.1b/resolve/main/vocab.json"
            }
        }
    }
}

impl std::str::FromStr for ParakeetModel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "ctc-0.6b" | "ctc06b" | "parakeet-ctc-0.6b" => Ok(ParakeetModel::Ctc06b),
            "ctc-1.1b" | "ctc11b" | "parakeet-ctc-1.1b" => Ok(ParakeetModel::Ctc11b),
            "rnnt-0.6b" | "rnnt06b" | "parakeet-rnnt-0.6b" => Ok(ParakeetModel::Rnnt06b),
            "rnnt-1.1b" | "rnnt11b" | "parakeet-rnnt-1.1b" => Ok(ParakeetModel::Rnnt11b),
            _ => Err(anyhow!("Unknown Parakeet model: {}", s)),
        }
    }
}

pub struct ParakeetBackend {
    model_path: Option<PathBuf>,
    language: String,
    is_loaded: bool,
    session: Option<Arc<Mutex<Session>>>,
    vocab: Option<Vec<String>>,
    mel: MelSpectrogram,
}

impl ParakeetBackend {
    pub fn new() -> Self {
        Self {
            model_path: None,
            language: "en".to_string(),
            is_loaded: false,
            session: None,
            vocab: None,
            mel: MelSpectrogram::default(),
        }
    }
}

impl Default for ParakeetBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AsrBackend for ParakeetBackend {
    fn name(&self) -> &str {
        "Parakeet"
    }

    fn is_loaded(&self) -> bool {
        self.is_loaded
    }

    fn load_model(&mut self, path: &Path) -> Result<()> {
        log::info!("Loading Parakeet ONNX model from: {:?}", path);

        if !path.exists() {
            return Err(anyhow!("Model file not found: {:?}", path));
        }

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if extension != "onnx" {
            return Err(anyhow!(
                "Parakeet requires an ONNX model file (.onnx), got .{}",
                extension
            ));
        }

        // Load vocabulary from JSON file alongside the model
        let vocab_path = path.with_extension("vocab.json");
        // Also try sibling file with matching name pattern
        let vocab_path2 = path.parent().map(|p| {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("model");
            p.join(format!("{}-vocab.json", stem))
        });

        let vocab = if vocab_path.exists() {
            load_vocab(&vocab_path)?
        } else if let Some(ref vp2) = vocab_path2 {
            if vp2.exists() {
                load_vocab(vp2)?
            } else {
                log::warn!("No vocab file found, using default English character vocab");
                default_char_vocab()
            }
        } else {
            log::warn!("No vocab file found, using default English character vocab");
            default_char_vocab()
        };

        log::info!("Vocabulary loaded: {} tokens", vocab.len());

        // Create ONNX Runtime session
        let session = Session::builder()
            .map_err(|e| anyhow!("Failed to create ONNX session builder: {}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow!("Failed to set optimization level: {}", e))?
            .with_intra_threads(num_cpus::get().min(4))
            .map_err(|e| anyhow!("Failed to set thread count: {}", e))?
            .commit_from_file(path)
            .map_err(|e| anyhow!("Failed to load ONNX model: {}", e))?;

        // Log model input/output info
        log::info!("ONNX model loaded. Inputs:");
        for input in session.inputs() {
            log::info!("  - {}", input.name());
        }
        log::info!("Outputs:");
        for output in session.outputs() {
            log::info!("  - {}", output.name());
        }

        self.session = Some(Arc::new(Mutex::new(session)));
        self.vocab = Some(vocab);
        self.model_path = Some(path.to_path_buf());
        self.is_loaded = true;

        log::info!("Parakeet model loaded successfully");
        Ok(())
    }

    fn set_language(&mut self, lang: &str) {
        if lang != "en" && lang != "auto" {
            log::warn!(
                "Parakeet only supports English. Ignoring language setting: {}",
                lang
            );
        }
        self.language = "en".to_string();
    }

    fn language(&self) -> &str {
        &self.language
    }

    async fn transcribe(&self, samples: &[f32]) -> Result<TranscriptionResult> {
        if !self.is_loaded {
            return Err(anyhow!("No model loaded"));
        }

        let session_arc = self
            .session
            .as_ref()
            .ok_or_else(|| anyhow!("ONNX session not initialized"))?;
        let vocab = self
            .vocab
            .as_ref()
            .ok_or_else(|| anyhow!("Vocabulary not loaded"))?;

        if samples.is_empty() {
            return Ok(TranscriptionResult {
                segments: vec![],
                full_text: String::new(),
            });
        }

        // Step 1: Compute mel spectrogram [n_mels, time_steps]
        let mel_features = self.mel.compute(samples);
        let (n_mels, n_frames) = (mel_features.shape()[0], mel_features.shape()[1]);

        if n_frames == 0 {
            return Ok(TranscriptionResult {
                segments: vec![],
                full_text: String::new(),
            });
        }

        // Lock session for inference
        let mut session = session_arc
            .lock()
            .map_err(|e| anyhow!("Failed to lock ONNX session: {}", e))?;

        // Try mel features first, fall back to raw audio
        let result = self
            .run_inference_mel(&mut session, &mel_features, n_mels, n_frames, vocab)
            .or_else(|e| {
                log::warn!("Mel-based inference failed ({}), trying raw audio input", e);
                self.run_inference_raw(&mut session, samples, vocab)
            })?;

        Ok(result)
    }
}

impl ParakeetBackend {
    /// Run inference with mel spectrogram input
    fn run_inference_mel(
        &self,
        session: &mut Session,
        mel_features: &Array2<f32>,
        n_mels: usize,
        n_frames: usize,
        vocab: &[String],
    ) -> Result<TranscriptionResult> {
        let mel_flat: Vec<f32> = mel_features.iter().copied().collect();

        let signal_value = Tensor::from_array(([1usize, n_mels, n_frames], mel_flat))
            .map_err(|e| anyhow!("Failed to create signal tensor: {}", e))?;
        let length_value = Tensor::from_array(([1usize], vec![n_frames as i64]))
            .map_err(|e| anyhow!("Failed to create length tensor: {}", e))?;

        let inputs = ort::inputs![
            "audio_signal" => signal_value,
            "length" => length_value
        ];

        let outputs = session
            .run(inputs)
            .map_err(|e| anyhow!("ONNX inference failed: {}", e))?;

        self.decode_ctc_output(&outputs, vocab)
    }

    /// Run inference with raw audio input (if model includes preprocessor)
    fn run_inference_raw(
        &self,
        session: &mut Session,
        samples: &[f32],
        vocab: &[String],
    ) -> Result<TranscriptionResult> {
        let signal_value = Tensor::from_array(([1usize, samples.len()], samples.to_vec()))
            .map_err(|e| anyhow!("Failed to create signal tensor: {}", e))?;
        let length_value = Tensor::from_array(([1usize], vec![samples.len() as i64]))
            .map_err(|e| anyhow!("Failed to create length tensor: {}", e))?;

        let inputs = ort::inputs![
            "audio_signal" => signal_value,
            "length" => length_value
        ];

        let outputs = session
            .run(inputs)
            .map_err(|e| anyhow!("ONNX inference failed: {}", e))?;

        self.decode_ctc_output(&outputs, vocab)
    }

    /// Decode CTC output logits to text
    fn decode_ctc_output(
        &self,
        outputs: &SessionOutputs<'_>,
        vocab: &[String],
    ) -> Result<TranscriptionResult> {
        // Get the first output by index
        let first_output = &outputs[0];

        let (shape_ref, logits_data) = first_output
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow!("Failed to extract logits: {}", e))?;

        let shape: Vec<usize> = shape_ref.iter().map(|&d| d as usize).collect();

        // Shape should be [batch, time_steps, vocab_size]
        if shape.len() != 3 {
            return Err(anyhow!("Unexpected output shape: {:?}, expected 3D", shape));
        }

        let time_steps = shape[1];
        let vocab_size = shape[2];
        let blank_id = 0usize; // CTC blank token is typically 0

        // CTC greedy decoding over raw flat tensor data
        // Data layout: [batch=0, time, vocab] → index = t * vocab_size + v
        let mut token_ids: Vec<usize> = Vec::new();
        let mut prev_token = blank_id;

        for t in 0..time_steps {
            let offset = t * vocab_size;
            let mut max_val = f32::NEG_INFINITY;
            let mut max_idx = 0usize;
            for v in 0..vocab_size {
                let val = logits_data[offset + v];
                if val > max_val {
                    max_val = val;
                    max_idx = v;
                }
            }

            // CTC collapse: skip blanks and repeated tokens
            if max_idx != blank_id && max_idx != prev_token {
                token_ids.push(max_idx);
            }
            prev_token = max_idx;
        }

        // Convert token IDs to text
        let text: String = token_ids
            .iter()
            .filter_map(|&id| vocab.get(id))
            .map(|token| {
                // Handle SentencePiece-style tokens (▁ = space)
                token.replace('\u{2581}', " ")
            })
            .collect::<String>()
            .trim()
            .to_string();

        if text.is_empty() {
            return Ok(TranscriptionResult {
                segments: vec![],
                full_text: String::new(),
            });
        }

        // Create a single segment for the full utterance
        let duration_ms = (samples_to_ms(time_steps, 160, 16000)) as i64;
        Ok(TranscriptionResult {
            segments: vec![TranscriptionSegment {
                text: text.clone(),
                start_ms: 0,
                end_ms: duration_ms,
            }],
            full_text: text,
        })
    }
}

/// Estimate duration in ms from mel frames
fn samples_to_ms(n_frames: usize, hop_length: usize, sample_rate: usize) -> u64 {
    (n_frames * hop_length * 1000 / sample_rate) as u64
}

/// Load vocabulary from a JSON file (array of token strings)
fn load_vocab(path: &Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow!("Failed to read vocab file {:?}: {}", path, e))?;
    let vocab: Vec<String> =
        serde_json::from_str(&content).map_err(|e| anyhow!("Failed to parse vocab JSON: {}", e))?;
    Ok(vocab)
}

/// Default English character-level vocabulary (fallback)
/// Token 0 = blank (CTC), then characters
fn default_char_vocab() -> Vec<String> {
    let mut vocab = vec!["<blank>".to_string()];
    for c in 'a'..='z' {
        vocab.push(c.to_string());
    }
    vocab.push(" ".to_string()); // space
    vocab.push("'".to_string()); // apostrophe
    vocab
}

/// Get the models directory for Parakeet
pub fn get_parakeet_models_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "phantomear", "PhantomEar")
        .ok_or_else(|| anyhow!("Could not determine project directories"))?;

    let models_dir = dirs.data_dir().join("models").join("parakeet");
    std::fs::create_dir_all(&models_dir)?;

    Ok(models_dir)
}

/// Check if a Parakeet model is downloaded
pub fn is_parakeet_model_downloaded(model: ParakeetModel) -> Result<bool> {
    let model_path = get_parakeet_models_dir()?.join(model.filename());
    Ok(model_path.exists())
}

/// Get the full path to a Parakeet model file
pub fn get_parakeet_model_path(model: ParakeetModel) -> Result<PathBuf> {
    Ok(get_parakeet_models_dir()?.join(model.filename()))
}
