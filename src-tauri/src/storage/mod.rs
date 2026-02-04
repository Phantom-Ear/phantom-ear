// Storage module
// SQLite database with sqlite-vec for vector search

use anyhow::Result;
use tauri::{AppHandle, Manager};

/// Initialize the SQLite database
pub async fn init_database(app: &AppHandle) -> Result<()> {
    // Get app data directory
    let app_dir = app.path().app_data_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get app data dir: {}", e))?;

    std::fs::create_dir_all(&app_dir)?;

    let db_path = app_dir.join("sidecar.db");

    // TODO: Initialize SQLite with sqlite-vec extension
    // Create tables:
    // - recordings (id, start_time, end_time, title)
    // - transcript_chunks (id, recording_id, text, start_ms, end_ms, embedding)
    // - settings (key, value)

    log::info!("Database initialized at: {:?}", db_path);
    Ok(())
}

pub struct Database {
    // Database connection
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        Ok(Self {})
    }

    /// Store a transcript chunk with its embedding
    pub fn store_chunk(&self, text: &str, start_ms: u64, end_ms: u64, embedding: &[f32]) -> Result<i64> {
        // TODO: Insert into transcript_chunks table
        Ok(0)
    }

    /// Search for similar chunks using vector similarity
    pub fn search_similar(&self, embedding: &[f32], limit: usize) -> Result<Vec<(i64, String, f32)>> {
        // TODO: Use sqlite-vec for vector similarity search
        Ok(vec![])
    }

    /// Get all chunks for a recording
    pub fn get_transcript(&self, recording_id: i64) -> Result<Vec<(String, u64, u64)>> {
        Ok(vec![])
    }
}
