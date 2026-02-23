// Model download and management module

use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

#[cfg(feature = "parakeet")]
use crate::asr::parakeet_backend::{
    get_parakeet_model_path, get_parakeet_models_dir, is_parakeet_model_downloaded, ParakeetModel,
};
use crate::asr::{get_model_path, get_models_dir, is_model_downloaded, WhisperModel};

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
pub async fn download_model(app: &AppHandle, model: WhisperModel) -> Result<PathBuf> {
    let model_path = get_model_path(model)?;
    let model_name = format!("{:?}", model);

    // Check if already downloaded (with size validation)
    if model_path.exists() {
        let file_size = std::fs::metadata(&model_path).map(|m| m.len()).unwrap_or(0);
        let expected_min = model.size_mb() * 1024 * 1024 * 8 / 10; // at least 80% of expected size
        if file_size >= expected_min {
            log::info!(
                "Model {} already downloaded at {:?} ({} bytes)",
                model_name,
                model_path,
                file_size
            );
            emit_progress(app, &model_name, 0, 0, 100.0, DownloadStatus::Completed);
            return Ok(model_path);
        } else {
            log::warn!(
                "Model {} exists but is too small ({} bytes, expected >= {}), re-downloading",
                model_name,
                file_size,
                expected_min
            );
            let _ = std::fs::remove_file(&model_path);
        }
    }

    log::info!(
        "Starting download of model {} from {}",
        model_name,
        model.download_url()
    );
    emit_progress(
        app,
        &model_name,
        0,
        model.size_mb() * 1024 * 1024,
        0.0,
        DownloadStatus::Starting,
    );

    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600)) // 1 hour timeout for large models
        .build()?;

    // Start download
    let response = client.get(model.download_url()).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        emit_progress(app, &model_name, 0, 0, 0.0, DownloadStatus::Failed);
        return Err(anyhow!("Download failed with status: {}", status));
    }

    let total_size = response
        .content_length()
        .unwrap_or(model.size_mb() * 1024 * 1024);

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
            emit_progress(
                app,
                &model_name,
                downloaded,
                total_size,
                percentage,
                DownloadStatus::Downloading,
            );
            last_progress_update = std::time::Instant::now();
        }
    }

    file.flush().await?;
    drop(file);

    // Validate downloaded file size (detect proxy/firewall interception)
    let file_size = tokio::fs::metadata(&temp_path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    let expected_min = model.size_mb() * 1024 * 1024 * 8 / 10; // at least 80% of expected size
    if file_size < expected_min {
        let _ = tokio::fs::remove_file(&temp_path).await;
        log::warn!(
            "HuggingFace download too small ({} bytes), trying GitHub Releases fallback...",
            file_size
        );

        // Fallback to GitHub Releases (zipped model)
        return download_from_github_fallback(app, model).await;
    }

    // Rename temp file to final path
    tokio::fs::rename(&temp_path, &model_path).await?;

    log::info!(
        "Model {} downloaded successfully to {:?} ({} bytes)",
        model_name,
        model_path,
        file_size
    );
    emit_progress(
        app,
        &model_name,
        total_size,
        total_size,
        100.0,
        DownloadStatus::Completed,
    );

    Ok(model_path)
}

/// Fallback: download zipped model from GitHub Releases and extract
async fn download_from_github_fallback(app: &AppHandle, model: WhisperModel) -> Result<PathBuf> {
    let model_path = get_model_path(model)?;
    let model_name = format!("{:?}", model);
    let github_url = model.github_release_url();

    log::info!("Trying GitHub Releases fallback: {}", github_url);
    emit_progress(
        app,
        &model_name,
        0,
        model.size_mb() * 1024 * 1024,
        0.0,
        DownloadStatus::Starting,
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()?;

    let response = client.get(&github_url).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        emit_progress(app, &model_name, 0, 0, 0.0, DownloadStatus::Failed);
        return Err(anyhow!(
            "Download failed from both HuggingFace and GitHub (status: {}). This may be caused by a corporate firewall. Try the manual download option.",
            status
        ));
    }

    let total_size = response
        .content_length()
        .unwrap_or(model.size_mb() * 1024 * 1024);

    let models_dir = get_models_dir()?;
    std::fs::create_dir_all(&models_dir)?;

    let zip_path = model_path.with_extension("zip.tmp");
    let mut file = tokio::fs::File::create(&zip_path).await?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    let mut last_progress_update = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        if last_progress_update.elapsed() >= std::time::Duration::from_millis(100) {
            let percentage = (downloaded as f32 / total_size as f32) * 90.0; // 90% for download, 10% for extract
            emit_progress(
                app,
                &model_name,
                downloaded,
                total_size,
                percentage,
                DownloadStatus::Downloading,
            );
            last_progress_update = std::time::Instant::now();
        }
    }

    file.flush().await?;
    drop(file);

    // Validate zip size
    let zip_size = tokio::fs::metadata(&zip_path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    let expected_min = model.size_mb() * 1024 * 1024 * 5 / 10; // zipped can be ~50%+ of original
    if zip_size < expected_min {
        let _ = tokio::fs::remove_file(&zip_path).await;
        emit_progress(app, &model_name, 0, 0, 0.0, DownloadStatus::Failed);
        return Err(anyhow!(
            "GitHub download also too small ({:.1} MB). Corporate firewall likely blocking all downloads. Use the manual download option.",
            zip_size as f64 / (1024.0 * 1024.0)
        ));
    }

    // Extract .bin from zip
    log::info!("Extracting model from zip...");
    let zip_path_clone = zip_path.clone();
    let model_path_clone = model_path.clone();
    let extract_result = tokio::task::spawn_blocking(move || {
        extract_bin_from_zip(&zip_path_clone, &model_path_clone)
    })
    .await?;

    // Clean up zip
    let _ = tokio::fs::remove_file(&zip_path).await;

    extract_result?;

    log::info!(
        "Model {} extracted from GitHub zip to {:?}",
        model_name,
        model_path
    );
    emit_progress(
        app,
        &model_name,
        total_size,
        total_size,
        100.0,
        DownloadStatus::Completed,
    );

    Ok(model_path)
}

/// Extract a .bin file from a zip archive
pub fn extract_bin_from_zip(zip_path: &PathBuf, target_path: &PathBuf) -> Result<()> {
    let file = std::fs::File::open(zip_path).map_err(|e| anyhow!("Failed to open zip: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| anyhow!("Invalid zip file: {}", e))?;

    // Find the .bin file inside the zip
    let bin_index = (0..archive.len())
        .find(|&i| {
            archive
                .by_index(i)
                .map(|f| f.name().ends_with(".bin"))
                .unwrap_or(false)
        })
        .ok_or_else(|| anyhow!("No .bin model file found inside the zip"))?;

    let mut bin_file = archive.by_index(bin_index)?;
    let mut out_file = std::fs::File::create(target_path)
        .map_err(|e| anyhow!("Failed to create output file: {}", e))?;
    std::io::copy(&mut bin_file, &mut out_file)
        .map_err(|e| anyhow!("Failed to extract model: {}", e))?;

    log::info!(
        "Extracted '{}' from zip ({} bytes)",
        bin_file.name(),
        bin_file.size()
    );
    Ok(())
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
pub async fn download_parakeet_model(app: &AppHandle, model: ParakeetModel) -> Result<PathBuf> {
    let model_path = get_parakeet_model_path(model)?;
    let model_name = format!("parakeet-{:?}", model).to_lowercase();

    // Check if already downloaded
    if model_path.exists() {
        log::info!(
            "Parakeet model {} already downloaded at {:?}",
            model_name,
            model_path
        );
        emit_progress(app, &model_name, 0, 0, 100.0, DownloadStatus::Completed);
        return Ok(model_path);
    }

    log::info!(
        "Starting download of Parakeet model {} from {}",
        model_name,
        model.download_url()
    );
    emit_progress(
        app,
        &model_name,
        0,
        model.size_mb() * 1024 * 1024,
        0.0,
        DownloadStatus::Starting,
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()?;

    // Download the ONNX model
    let response = client.get(model.download_url()).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        emit_progress(app, &model_name, 0, 0, 0.0, DownloadStatus::Failed);
        return Err(anyhow!("Download failed with status: {}", status));
    }

    let total_size = response
        .content_length()
        .unwrap_or(model.size_mb() * 1024 * 1024);

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
            emit_progress(
                app,
                &model_name,
                downloaded,
                total_size,
                percentage,
                DownloadStatus::Downloading,
            );
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
            Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                Ok(bytes) => {
                    if let Err(e) = tokio::fs::write(&vocab_path, &bytes).await {
                        log::warn!("Failed to write vocab file: {}", e);
                    } else {
                        log::info!("Vocab file downloaded to {:?}", vocab_path);
                    }
                }
                Err(e) => log::warn!("Failed to download vocab file: {}", e),
            },
            Ok(resp) => log::warn!("Vocab download returned status: {}", resp.status()),
            Err(e) => log::warn!("Failed to download vocab file: {}", e),
        }
    }

    emit_progress(
        app,
        &model_name,
        total_size,
        total_size,
        100.0,
        DownloadStatus::Completed,
    );
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
        (WhisperModel::Small, true), // Recommended default
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
                name: format!("parakeet-{:?}", model)
                    .to_lowercase()
                    .replace("ctc", "ctc-"),
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
const BGE_MODEL_URL: &str =
    "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx";
const BGE_TOKENIZER_URL: &str =
    "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/tokenizer.json";

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
        emit_progress(
            app,
            "bge-small-en-v1.5",
            0,
            0,
            100.0,
            DownloadStatus::Completed,
        );
        return Ok(model_dir);
    }

    let model_name = "bge-small-en-v1.5";
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()?;

    // Download model.onnx (~33MB)
    if !model_path.exists() {
        log::info!("Downloading BGE-small model.onnx...");
        emit_progress(
            app,
            model_name,
            0,
            33 * 1024 * 1024,
            0.0,
            DownloadStatus::Starting,
        );

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
                emit_progress(
                    app,
                    model_name,
                    downloaded,
                    total_size,
                    pct,
                    DownloadStatus::Downloading,
                );
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

/// Get download URLs for manual embedding model download
pub fn get_embedding_model_download_urls() -> (String, String) {
    (BGE_MODEL_URL.to_string(), BGE_TOKENIZER_URL.to_string())
}

/// Import embedding model from manually downloaded files
/// Accepts either:
/// - A zip file containing model.onnx and tokenizer.json
/// - A directory containing both files
/// - Individual model.onnx or tokenizer.json files (call twice)
pub fn import_embedding_model(source_path: &PathBuf) -> Result<PathBuf> {
    let model_dir = get_embedding_model_dir()?;
    std::fs::create_dir_all(&model_dir)?;

    let source_str = source_path.to_string_lossy();

    if source_str.ends_with(".zip") {
        // Extract from zip archive
        log::info!("Importing embedding model from zip: {:?}", source_path);
        extract_embedding_from_zip(source_path, &model_dir)?;
    } else if source_path.is_dir() {
        // Copy from directory
        log::info!(
            "Importing embedding model from directory: {:?}",
            source_path
        );

        let src_model = source_path.join("model.onnx");
        let src_tokenizer = source_path.join("tokenizer.json");

        if src_model.exists() {
            std::fs::copy(&src_model, model_dir.join("model.onnx"))?;
            log::info!("Copied model.onnx");
        }

        if src_tokenizer.exists() {
            std::fs::copy(&src_tokenizer, model_dir.join("tokenizer.json"))?;
            log::info!("Copied tokenizer.json");
        }
    } else if source_str.ends_with(".onnx") || source_str.contains("model.onnx") {
        // Individual model.onnx file
        log::info!("Importing model.onnx from: {:?}", source_path);
        std::fs::copy(source_path, model_dir.join("model.onnx"))?;
    } else if source_str.contains("tokenizer.json") {
        // Individual tokenizer.json file
        log::info!("Importing tokenizer.json from: {:?}", source_path);
        std::fs::copy(source_path, model_dir.join("tokenizer.json"))?;
    } else {
        return Err(anyhow!(
            "Unrecognized file type. Expected .zip, .onnx, tokenizer.json, or a directory"
        ));
    }

    // Verify we have both files
    let model_path = model_dir.join("model.onnx");
    let tokenizer_path = model_dir.join("tokenizer.json");

    if !model_path.exists() {
        return Err(anyhow!(
            "model.onnx not found after import. Please provide both model.onnx and tokenizer.json"
        ));
    }
    if !tokenizer_path.exists() {
        return Err(anyhow!("tokenizer.json not found after import. Please provide both model.onnx and tokenizer.json"));
    }

    log::info!("Embedding model imported successfully to {:?}", model_dir);
    Ok(model_dir)
}

/// Extract embedding model files from a zip archive
fn extract_embedding_from_zip(zip_path: &Path, target_dir: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path).map_err(|e| anyhow!("Failed to open zip: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| anyhow!("Invalid zip file: {}", e))?;

    let mut found_model = false;
    let mut found_tokenizer = false;

    for i in 0..archive.len() {
        let mut zip_file = archive.by_index(i)?;
        let name = zip_file.name().to_string();

        // Look for model.onnx (might be in a subdirectory like onnx/model.onnx)
        if name.ends_with("model.onnx") || name.ends_with(".onnx") {
            let out_path = target_dir.join("model.onnx");
            let mut out_file = std::fs::File::create(&out_path)
                .map_err(|e| anyhow!("Failed to create model.onnx: {}", e))?;
            std::io::copy(&mut zip_file, &mut out_file)
                .map_err(|e| anyhow!("Failed to extract model.onnx: {}", e))?;
            log::info!("Extracted '{}' ({} bytes)", name, zip_file.size());
            found_model = true;
        }

        // Look for tokenizer.json
        if name.ends_with("tokenizer.json") {
            let out_path = target_dir.join("tokenizer.json");
            let mut out_file = std::fs::File::create(&out_path)
                .map_err(|e| anyhow!("Failed to create tokenizer.json: {}", e))?;
            std::io::copy(&mut zip_file, &mut out_file)
                .map_err(|e| anyhow!("Failed to extract tokenizer.json: {}", e))?;
            log::info!("Extracted '{}' ({} bytes)", name, zip_file.size());
            found_tokenizer = true;
        }
    }

    if !found_model {
        return Err(anyhow!("No model.onnx found in the zip archive"));
    }
    if !found_tokenizer {
        return Err(anyhow!("No tokenizer.json found in the zip archive"));
    }

    Ok(())
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
