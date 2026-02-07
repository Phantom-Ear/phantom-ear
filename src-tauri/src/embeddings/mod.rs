// Embeddings module — BGE-small-en-v1.5 ONNX for semantic search

use anyhow::{anyhow, Result};
use ort::session::Session;
use ort::value::Value;
use std::path::Path;
use std::sync::Mutex;

pub struct EmbeddingModel {
    session: Mutex<Session>,
    tokenizer: tokenizers::Tokenizer,
}

impl EmbeddingModel {
    /// Load BGE-small model from directory containing model.onnx + tokenizer.json
    pub fn load(model_dir: &Path) -> Result<Self> {
        let model_path = model_dir.join("model.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !model_path.exists() {
            return Err(anyhow!("model.onnx not found in {:?}", model_dir));
        }
        if !tokenizer_path.exists() {
            return Err(anyhow!("tokenizer.json not found in {:?}", model_dir));
        }

        log::info!("Loading embedding model from {:?}", model_dir);

        let session = Session::builder()?
            .with_intra_threads(2)?
            .commit_from_file(&model_path)?;

        let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;

        log::info!("Embedding model loaded successfully");
        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
        })
    }

    /// Embed a single text string → 384-dim L2-normalized vector
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let encoding = self.tokenizer.encode(text, true)
            .map_err(|e| anyhow!("Tokenization failed: {}", e))?;

        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&m| m as i64).collect();
        let token_type_ids: Vec<i64> = encoding.get_type_ids().iter().map(|&t| t as i64).collect();
        let seq_len = input_ids.len();

        let input_ids_tensor = Value::from_array(([1, seq_len], input_ids.clone()))?;
        let attention_mask_tensor = Value::from_array(([1, seq_len], attention_mask.clone()))?;
        let token_type_ids_tensor = Value::from_array(([1, seq_len], token_type_ids.clone()))?;

        let mut session = self.session.lock().map_err(|e| anyhow!("Session lock: {}", e))?;
        let outputs = session.run(ort::inputs![
            input_ids_tensor,
            attention_mask_tensor,
            token_type_ids_tensor,
        ])?;

        // Output shape: [1, seq_len, 384] — use mean pooling
        let (shape, data) = outputs[0].try_extract_tensor::<f32>()?;

        // shape derefs to &[i64]
        let dims: &[i64] = &shape;
        let hidden_dim = dims[dims.len() - 1] as usize;
        let seq_len_out = dims[1] as usize;

        // Mean pooling over token dimension, weighted by attention mask
        let mut pooled = vec![0.0f32; hidden_dim];
        let mut mask_sum = 0.0f32;

        for t in 0..seq_len_out {
            let mask_val = if t < attention_mask.len() { attention_mask[t] as f32 } else { 0.0 };
            mask_sum += mask_val;
            for d in 0..hidden_dim {
                let idx = t * hidden_dim + d; // flat index for [0, t, d] in row-major
                pooled[d] += data[idx] * mask_val;
            }
        }

        if mask_sum > 0.0 {
            for d in 0..hidden_dim {
                pooled[d] /= mask_sum;
            }
        }

        // L2 normalize
        let norm: f32 = pooled.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut pooled {
                *x /= norm;
            }
        }

        Ok(pooled)
    }

    /// Batch embed multiple texts
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // Simple sequential for now — can optimize with padding later
        texts.iter().map(|t| self.embed(t)).collect()
    }
}

/// Enrich a segment with meeting context before embedding
pub fn enrich_segment(meeting_title: &str, time_label: &str, text: &str) -> String {
    format!("[Meeting: \"{}\"] [{}] {}", meeting_title, time_label, text)
}
