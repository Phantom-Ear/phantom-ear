// Model download and management module

use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

use crate::asr::{WhisperModel, get_models_dir, get_model_path, is_model_downloaded};
#[cfg(feature = "parakeet")]
use crate::asr::parakeet_backend::{ParakeetModel, get_parakeet_models_dir, get_parakeet_model_path, is_parakeet_model_downloaded};

/// Progress event sent to frontend during download
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub model_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percentage: f32,
    pub status: DownloadStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadStatus {
    Starting,
    Downloading,
    Completed,
    Failed,
    Cancelled,
}

/// Download a Whisper model with progress events
pub async fn download_model(
    app: &AppHandle,
    model: WhisperModel,
) -> Result<PathBuf> {
    let model_path = get_model_path(model)?;
    let model_name = format!("{:?}", model);

    // Check if already downloaded
    if model_path.exists() {
        log::info!("Model {} already downloaded at {:?}", model_name, model_path);
        emit_progress(app, &model_name, 0, 0, 100.0, DownloadStatus::Completed);
        return Ok(model_path);
    }

    log::info!("Starting download of model {} from {}", model_name, model.download_url());
    emit_progress(app, &model_name, 0, model.size_mb() * 1024 * 1024, 0.0, DownloadStatus::Starting);

    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600)) // 1 hour timeout for large models
        .build()?;

    // Start download
    let response = client
        .get(model.download_url())
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        emit_progress(app, &model_name, 0, 0, 0.0, DownloadStatus::Failed);
        return Err(anyhow!("Download failed with status: {}", status));
    }

    let total_size = response.content_length().unwrap_or(model.size_mb() * 1024 * 1024);

    // Ensure models directory exists
    let models_dir = get_models_dir()?;
    std::fs::create_dir_all(&models_dir)?;

    // Create temp file for download
    let temp_path = model_path.with_extension("tmp");
    let mut file = tokio::fs::File::create(&temp_path).await?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    let mut last_progress_update = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        // Emit progress every 100ms to avoid flooding
        if last_progress_update.elapsed() >= std::time::Duration::from_millis(100) {
            let percentage = (downloaded as f32 / total_size as f32) * 100.0;
            emit_progress(app, &model_name, downloaded, total_size, percentage, DownloadStatus::Downloading);
            last_progress_update = std::time::Instant::now();
        }
    }

    file.flush().await?;
    drop(file);

    // Rename temp file to final path
    tokio::fs::rename(&temp_path, &model_path).await?;

    log::info!("Model {} downloaded successfully to {:?}", model_name, model_path);
    emit_progress(app, &model_name, total_size, total_size, 100.0, DownloadStatus::Completed);

    Ok(model_path)
}

/// Emit download progress event to frontend
fn emit_progress(
    app: &AppHandle,
    model_name: &str,
    downloaded: u64,
    total: u64,
    percentage: f32,
    status: DownloadStatus,
) {
    let progress = DownloadProgress {
        model_name: model_name.to_string(),
        downloaded_bytes: downloaded,
        total_bytes: total,
        percentage,
        status,
    };

    if let Err(e) = app.emit("model-download-progress", &progress) {
        log::error!("Failed to emit progress event: {}", e);
    }
}

/// Download a Parakeet ONNX model (and its vocab file) with progress events
#[cfg(feature = "parakeet")]
pub async fn download_parakeet_model(
    app: &AppHandle,
    model: ParakeetModel,
) -> Result<PathBuf> {
    let model_path = get_parakeet_model_path(model)?;
    let model_name = format!("parakeet-{:?}", model).to_lowercase();

    // Check if already downloaded
    if model_path.exists() {
        log::info!("Parakeet model {} already downloaded at {:?}", model_name, model_path);
        emit_progress(app, &model_name, 0, 0, 100.0, DownloadStatus::Completed);
        return Ok(model_path);
    }

    log::info!("Starting download of Parakeet model {} from {}", model_name, model.download_url());
    emit_progress(app, &model_name, 0, model.size_mb() * 1024 * 1024, 0.0, DownloadStatus::Starting);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()?;

    // Download the ONNX model
    let response = client
        .get(model.download_url())
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        emit_progress(app, &model_name, 0, 0, 0.0, DownloadStatus::Failed);
        return Err(anyhow!("Download failed with status: {}", status));
    }

    let total_size = response.content_length().unwrap_or(model.size_mb() * 1024 * 1024);

    // Ensure models directory exists
    let models_dir = get_parakeet_models_dir()?;
    std::fs::create_dir_all(&models_dir)?;

    // Download to temp file
    let temp_path = model_path.with_extension("tmp");
    let mut file = tokio::fs::File::create(&temp_path).await?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    let mut last_progress_update = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        if last_progress_update.elapsed() >= std::time::Duration::from_millis(100) {
            let percentage = (downloaded as f32 / total_size as f32) * 100.0;
            emit_progress(app, &model_name, downloaded, total_size, percentage, DownloadStatus::Downloading);
            last_progress_update = std::time::Instant::now();
        }
    }

    file.flush().await?;
    drop(file);
    tokio::fs::rename(&temp_path, &model_path).await?;

    log::info!("Parakeet model {} downloaded successfully", model_name);

    // Also try to download the vocab file (non-fatal if it fails)
    let vocab_path = models_dir.join(model.vocab_filename());
    if !vocab_path.exists() {
        log::info!("Downloading vocab file for {}", model_name);
        match client.get(model.vocab_url()).send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.bytes().await {
                    Ok(bytes) => {
                        if let Err(e) = tokio::fs::write(&vocab_path, &bytes).await {
                            log::warn!("Failed to write vocab file: {}", e);
                        } else {
                            log::info!("Vocab file downloaded to {:?}", vocab_path);
                        }
                    }
                    Err(e) => log::warn!("Failed to download vocab file: {}", e),
                }
            }
            Ok(resp) => log::warn!("Vocab download returned status: {}", resp.status()),
            Err(e) => log::warn!("Failed to download vocab file: {}", e),
        }
    }

    emit_progress(app, &model_name, total_size, total_size, 100.0, DownloadStatus::Completed);
    Ok(model_path)
}

/// Get status of all models
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub size_mb: u64,
    pub downloaded: bool,
    pub recommended: bool,
    #[serde(default)]
    pub backend: String,
}

pub fn get_all_models_status() -> Result<Vec<ModelInfo>> {
    let whisper_models = vec![
        (WhisperModel::Tiny, false),
        (WhisperModel::Base, false),
        (WhisperModel::Small, true),   // Recommended default
        (WhisperModel::Medium, false),
        (WhisperModel::Large, false),
    ];

    #[allow(unused_mut)]
    let mut models: Vec<ModelInfo> = whisper_models
        .into_iter()
        .map(|(model, recommended)| {
            Ok(ModelInfo {
                name: format!("{:?}", model).to_lowercase(),
                size_mb: model.size_mb(),
                downloaded: is_model_downloaded(model)?,
                recommended,
                backend: "whisper".to_string(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // Add Parakeet CTC models only when feature is enabled
    #[cfg(feature = "parakeet")]
    {
        let parakeet_models = vec![
            (ParakeetModel::Ctc06b, false),
            (ParakeetModel::Ctc11b, false),
        ];

        for (model, recommended) in parakeet_models {
            models.push(ModelInfo {
                name: format!("parakeet-{:?}", model).to_lowercase().replace("ctc", "ctc-"),
                size_mb: model.size_mb(),
                downloaded: is_parakeet_model_downloaded(model).unwrap_or(false),
                recommended,
                backend: "parakeet".to_string(),
            });
        }
    }

    Ok(models)
}

/// Delete a downloaded model
pub fn delete_model(model: WhisperModel) -> Result<()> {
    let model_path = get_model_path(model)?;
    if model_path.exists() {
        std::fs::remove_file(&model_path)?;
        log::info!("Deleted model: {:?}", model);
    }
    Ok(())
}

// ============================================================================
// Embedding Model Download (BGE-small-en-v1.5)
// ============================================================================

const BGE_MODEL_DIR: &str = "bge-small-en-v1.5";
const BGE_MODEL_URL: &str = "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx";
const BGE_TOKENIZER_URL: &str = "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/tokenizer.json";

/// Get the directory where the embedding model is stored
pub fn get_embedding_model_dir() -> Result<PathBuf> {
    let models_dir = crate::asr::get_models_dir()?;
    Ok(models_dir.join(BGE_MODEL_DIR))
}

/// Check if embedding model is downloaded (both model.onnx and tokenizer.json)
pub fn is_embedding_model_downloaded() -> bool {
    match get_embedding_model_dir() {
        Ok(dir) => dir.join("model.onnx").exists() && dir.join("tokenizer.json").exists(),
        Err(_) => false,
    }
}

/// Download the BGE-small embedding model with progress events
pub async fn download_embedding_model(app: &AppHandle) -> Result<PathBuf> {
    let model_dir = get_embedding_model_dir()?;
    std::fs::create_dir_all(&model_dir)?;

    let model_path = model_dir.join("model.onnx");
    let tokenizer_path = model_dir.join("tokenizer.json");

    // Check if already downloaded
    if model_path.exists() && tokenizer_path.exists() {
        log::info!("Embedding model already downloaded at {:?}", model_dir);
        emit_progress(app, "bge-small-en-v1.5", 0, 0, 100.0, DownloadStatus::Completed);
        return Ok(model_dir);
    }

    let model_name = "bge-small-en-v1.5";
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()?;

    // Download model.onnx (~33MB)
    if !model_path.exists() {
        log::info!("Downloading BGE-small model.onnx...");
        emit_progress(app, model_name, 0, 33 * 1024 * 1024, 0.0, DownloadStatus::Starting);

        let response = client.get(BGE_MODEL_URL).send().await?;
        if !response.status().is_success() {
            emit_progress(app, model_name, 0, 0, 0.0, DownloadStatus::Failed);
            return Err(anyhow!("Model download failed: {}", response.status()));
        }

        let total_size = response.content_length().unwrap_or(33 * 1024 * 1024);
        let temp_path = model_path.with_extension("tmp");
        let mut file = tokio::fs::File::create(&temp_path).await?;
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();
        let mut last_update = std::time::Instant::now();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
            downloaded += chunk.len() as u64;

            if last_update.elapsed() >= std::time::Duration::from_millis(100) {
                let pct = (downloaded as f32 / total_size as f32) * 90.0; // 90% for model
                emit_progress(app, model_name, downloaded, total_size, pct, DownloadStatus::Downloading);
                last_update = std::time::Instant::now();
            }
        }

        tokio::io::AsyncWriteExt::flush(&mut file).await?;
        drop(file);
        tokio::fs::rename(&temp_path, &model_path).await?;
        log::info!("model.onnx downloaded");
    }

    // Download tokenizer.json (~700KB)
    if !tokenizer_path.exists() {
        log::info!("Downloading BGE-small tokenizer.json...");
        emit_progress(app, model_name, 0, 0, 92.0, DownloadStatus::Downloading);

        let response = client.get(BGE_TOKENIZER_URL).send().await?;
        if response.status().is_success() {
            let bytes = response.bytes().await?;
            tokio::fs::write(&tokenizer_path, &bytes).await?;
            log::info!("tokenizer.json downloaded");
        } else {
            emit_progress(app, model_name, 0, 0, 0.0, DownloadStatus::Failed);
            return Err(anyhow!("Tokenizer download failed: {}", response.status()));
        }
    }

    log::info!("Embedding model fully downloaded at {:?}", model_dir);
    emit_progress(app, model_name, 0, 0, 100.0, DownloadStatus::Completed);
    Ok(model_dir)
}

/// Get total disk space used by models
pub fn get_models_disk_usage() -> Result<u64> {
    let models_dir = get_models_dir()?;
    let mut total = 0u64;

    if models_dir.exists() {
        for entry in std::fs::read_dir(&models_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                total += entry.metadata()?.len();
            }
        }
    }

    Ok(total)
}
