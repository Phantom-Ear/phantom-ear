// Storage module — SQLite persistence for meetings, transcripts, and settings

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SemanticSearchResult {
    pub meeting_id: String,
    pub meeting_title: String,
    pub segment_id: String,
    pub text: String,
    pub time_label: String,
    pub score: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MeetingRow {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub ended_at: Option<String>,
    pub pinned: bool,
    pub duration_ms: i64,
    pub summary: Option<String>,
    pub tags: Option<String>,
    // Metadata
    pub topics: Option<String>,
    pub action_items: Option<String>,
    pub decisions: Option<String>,
    pub participant_count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MeetingListItem {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub pinned: bool,
    pub segment_count: i64,
    pub duration_ms: i64,
    pub tags: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SegmentRow {
    pub id: String,
    pub meeting_id: String,
    pub time_label: String,
    pub text: String,
    pub timestamp_ms: i64,
    pub speaker_id: Option<String>,
    // AI enhanced fields
    #[serde(default)]
    pub enhanced_text: Option<String>,
    #[serde(default)]
    pub is_question: bool,
    #[serde(default)]
    pub question_answer: Option<String>,
}

impl Default for SegmentRow {
    fn default() -> Self {
        Self {
            id: String::new(),
            meeting_id: String::new(),
            time_label: String::new(),
            text: String::new(),
            timestamp_ms: 0,
            speaker_id: None,
            enhanced_text: None,
            is_question: false,
            question_answer: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Speaker {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub meeting_id: String,
    pub meeting_title: String,
    pub segment_id: String,
    pub text: String,
    pub time_label: String,
    pub snippet: String,
}

// ============================================================================
// Database
// ============================================================================

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS meetings (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                ended_at TEXT,
                pinned INTEGER NOT NULL DEFAULT 0,
                duration_ms INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS transcript_segments (
                id TEXT PRIMARY KEY,
                meeting_id TEXT NOT NULL,
                time_label TEXT NOT NULL,
                text TEXT NOT NULL,
                timestamp_ms INTEGER NOT NULL,
                FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            -- Embeddings storage for semantic search
            CREATE TABLE IF NOT EXISTS segment_embeddings (
                segment_id TEXT PRIMARY KEY,
                embedding BLOB NOT NULL,
                FOREIGN KEY (segment_id) REFERENCES transcript_segments(id) ON DELETE CASCADE
            );

            -- FTS5 virtual table for full-text search
            CREATE VIRTUAL TABLE IF NOT EXISTS transcript_fts USING fts5(
                text,
                content='transcript_segments',
                content_rowid='rowid'
            );

            -- Triggers to keep FTS in sync
            CREATE TRIGGER IF NOT EXISTS transcript_fts_insert AFTER INSERT ON transcript_segments BEGIN
                INSERT INTO transcript_fts(rowid, text) VALUES (new.rowid, new.text);
            END;

            CREATE TRIGGER IF NOT EXISTS transcript_fts_delete AFTER DELETE ON transcript_segments BEGIN
                INSERT INTO transcript_fts(transcript_fts, rowid, text) VALUES('delete', old.rowid, old.text);
            END;

            CREATE TRIGGER IF NOT EXISTS transcript_fts_update AFTER UPDATE ON transcript_segments BEGIN
                INSERT INTO transcript_fts(transcript_fts, rowid, text) VALUES('delete', old.rowid, old.text);
                INSERT INTO transcript_fts(rowid, text) VALUES (new.rowid, new.text);
            END;
            ",
        )?;

        // Migration: add summary column if it doesn't exist
        let has_summary: bool = {
            let mut stmt = conn.prepare(
                "SELECT COUNT(*) FROM pragma_table_info('meetings') WHERE name='summary'"
            )?;
            let count: i64 = stmt.query_row([], |row| row.get(0))?;
            count > 0
        };
        if !has_summary {
            conn.execute_batch("ALTER TABLE meetings ADD COLUMN summary TEXT;")?;
            log::info!("Added summary column to meetings table");
        }

        // Migration: add speaker_id column to transcript_segments if it doesn't exist
        let has_speaker_id: bool = {
            let mut stmt = conn.prepare(
                "SELECT COUNT(*) FROM pragma_table_info('transcript_segments') WHERE name='speaker_id'"
            )?;
            let count: i64 = stmt.query_row([], |row| row.get(0))?;
            count > 0
        };
        if !has_speaker_id {
            conn.execute_batch("ALTER TABLE transcript_segments ADD COLUMN speaker_id TEXT;")?;
            log::info!("Added speaker_id column to transcript_segments table");
        }

        // Migration: add tags column to meetings if it doesn't exist
        let has_tags: bool = {
            let mut stmt = conn.prepare(
                "SELECT COUNT(*) FROM pragma_table_info('meetings') WHERE name='tags'"
            )?;
            let count: i64 = stmt.query_row([], |row| row.get(0))?;
            count > 0
        };
        if !has_tags {
            conn.execute_batch("ALTER TABLE meetings ADD COLUMN tags TEXT;")?;
            log::info!("Added tags column to meetings table");
        }

        // Migration: add metadata columns to meetings if they don't exist
        let has_topics: bool = {
            let mut stmt = conn.prepare(
                "SELECT COUNT(*) FROM pragma_table_info('meetings') WHERE name='topics'"
            )?;
            let count: i64 = stmt.query_row([], |row| row.get(0))?;
            count > 0
        };
        if !has_topics {
            conn.execute_batch(
                "ALTER TABLE meetings ADD COLUMN topics TEXT;
                 ALTER TABLE meetings ADD COLUMN action_items TEXT;
                 ALTER TABLE meetings ADD COLUMN decisions TEXT;
                 ALTER TABLE meetings ADD COLUMN participant_count INTEGER DEFAULT 0;"
            )?;
            log::info!("Added metadata columns to meetings table");
        }

        // Migration: add enhanced_text column to transcript_segments if it doesn't exist
        let has_enhanced_text: bool = {
            let mut stmt = conn.prepare(
                "SELECT COUNT(*) FROM pragma_table_info('transcript_segments') WHERE name='enhanced_text'"
            )?;
            let count: i64 = stmt.query_row([], |row| row.get(0))?;
            count > 0
        };
        if !has_enhanced_text {
            conn.execute_batch(
                "ALTER TABLE transcript_segments ADD COLUMN enhanced_text TEXT;
                 ALTER TABLE transcript_segments ADD COLUMN is_question INTEGER DEFAULT 0;
                 ALTER TABLE transcript_segments ADD COLUMN question_answer TEXT;"
            )?;
            log::info!("Added enhanced_text and question columns to transcript_segments table");
        }

        // Create speakers table if it doesn't exist
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS speakers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT NOT NULL,
                created_at TEXT NOT NULL
            );"
        )?;

        Ok(())
    }

    // ========================================================================
    // Meetings
    // ========================================================================

    pub fn create_meeting(&self, id: &str, title: &str, created_at: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO meetings (id, title, created_at) VALUES (?1, ?2, ?3)",
            params![id, title, created_at],
        )?;
        Ok(())
    }

    pub fn get_meeting(&self, id: &str) -> Result<Option<MeetingRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, ended_at, pinned, duration_ms, summary, tags, topics, action_items, decisions, participant_count FROM meetings WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(MeetingRow {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                ended_at: row.get(3)?,
                pinned: row.get::<_, i32>(4)? != 0,
                duration_ms: row.get(5)?,
                summary: row.get(6)?,
                tags: row.get(7)?,
                topics: row.get(8)?,
                action_items: row.get(9)?,
                decisions: row.get(10)?,
                participant_count: row.get(11)?,
            })
        })?;
        match rows.next() {
            Some(Ok(m)) => Ok(Some(m)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    pub fn list_meetings(&self) -> Result<Vec<MeetingListItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT m.id, m.title, m.created_at, m.pinned, m.duration_ms, m.tags,
                    (SELECT COUNT(*) FROM transcript_segments WHERE meeting_id = m.id) as seg_count
             FROM meetings m
             ORDER BY m.pinned DESC, m.created_at DESC",
        )?;
        let items = stmt
            .query_map([], |row| {
                Ok(MeetingListItem {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    pinned: row.get::<_, i32>(3)? != 0,
                    duration_ms: row.get(4)?,
                    tags: row.get(5)?,
                    segment_count: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items)
    }

    /// Get meetings within a date range (for analytics)
    pub fn get_meetings_in_range(&self, from_date: &str, to_date: &str) -> Result<Vec<MeetingListItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT m.id, m.title, m.created_at, m.pinned, m.duration_ms, m.tags,
                    (SELECT COUNT(*) FROM transcript_segments WHERE meeting_id = m.id) as seg_count
             FROM meetings m
             WHERE m.created_at >= ?1 AND m.created_at <= ?2
             ORDER BY m.created_at DESC",
        )?;
        let items = stmt
            .query_map(params![from_date, to_date], |row| {
                Ok(MeetingListItem {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    pinned: row.get::<_, i32>(3)? != 0,
                    duration_ms: row.get(4)?,
                    tags: row.get(5)?,
                    segment_count: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items)
    }

    /// Get meeting count and total duration for analytics
    pub fn get_meeting_stats(&self, from_date: Option<&str>, to_date: Option<&str>) -> Result<(i64, i64)> {
        let conn = self.conn.lock().unwrap();
        let (count, duration): (i64, i64) = if let (Some(from), Some(to)) = (from_date, to_date) {
            let mut stmt = conn.prepare(
                "SELECT COUNT(*), COALESCE(SUM(duration_ms), 0) FROM meetings WHERE created_at >= ?1 AND created_at <= ?2"
            )?;
            stmt.query_row(params![from, to], |row| Ok((row.get(0)?, row.get(1)?)))?
        } else {
            let mut stmt = conn.prepare(
                "SELECT COUNT(*), COALESCE(SUM(duration_ms), 0) FROM meetings"
            )?;
            stmt.query_row([], |row| Ok((row.get(0)?, row.get(1)?)))?
        };
        Ok((count, duration))
    }

    pub fn update_meeting_title(&self, id: &str, title: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE meetings SET title = ?1 WHERE id = ?2",
            params![title, id],
        )?;
        Ok(())
    }

    pub fn update_meeting_tags(&self, id: &str, tags: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE meetings SET tags = ?1 WHERE id = ?2",
            params![tags, id],
        )?;
        Ok(())
    }

    pub fn update_meeting_ended(&self, id: &str, ended_at: &str, duration_ms: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE meetings SET ended_at = ?1, duration_ms = ?2 WHERE id = ?3",
            params![ended_at, duration_ms, id],
        )?;
        Ok(())
    }

    pub fn set_meeting_pinned(&self, id: &str, pinned: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE meetings SET pinned = ?1 WHERE id = ?2",
            params![pinned as i32, id],
        )?;
        Ok(())
    }

    pub fn save_meeting_summary(&self, id: &str, summary: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE meetings SET summary = ?1 WHERE id = ?2",
            params![summary, id],
        )?;
        Ok(())
    }

    pub fn get_meeting_summary(&self, id: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT summary FROM meetings WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| row.get::<_, Option<String>>(0))?;
        match rows.next() {
            Some(Ok(val)) => Ok(val),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// Get recent completed meetings with their summaries for Phomy global queries
    pub fn get_recent_meetings_with_summaries(&self, limit: usize) -> Result<Vec<(String, String, String, Option<String>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, summary FROM meetings
             WHERE ended_at IS NOT NULL
             ORDER BY created_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Get segments within a timestamp range for a meeting (for time-based queries)
    pub fn get_segments_in_range(&self, meeting_id: &str, from_ms: i64, to_ms: i64) -> Result<Vec<SegmentRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, meeting_id, time_label, text, timestamp_ms, speaker_id
             FROM transcript_segments
             WHERE meeting_id = ?1 AND timestamp_ms >= ?2 AND timestamp_ms <= ?3
             ORDER BY timestamp_ms ASC",
        )?;
        let segments = stmt
            .query_map(params![meeting_id, from_ms, to_ms], |row| {
                Ok(SegmentRow {
                    id: row.get(0)?,
                    meeting_id: row.get(1)?,
                    time_label: row.get(2)?,
                    text: row.get(3)?,
                    timestamp_ms: row.get(4)?,
                    speaker_id: row.get(5)?,
                    enhanced_text: None,
                    is_question: false,
                    question_answer: None,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(segments)
    }

    /// Get the last N segments of a meeting (for "what did they just say")
    pub fn get_last_segments(&self, meeting_id: &str, limit: usize) -> Result<Vec<SegmentRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, meeting_id, time_label, text, timestamp_ms, speaker_id
             FROM transcript_segments
             WHERE meeting_id = ?1
             ORDER BY timestamp_ms DESC
             LIMIT ?2",
        )?;
        let mut segments = stmt
            .query_map(params![meeting_id, limit as i64], |row| {
                Ok(SegmentRow {
                    id: row.get(0)?,
                    meeting_id: row.get(1)?,
                    time_label: row.get(2)?,
                    text: row.get(3)?,
                    timestamp_ms: row.get(4)?,
                    speaker_id: row.get(5)?,
                    enhanced_text: None,
                    is_question: false,
                    question_answer: None,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        segments.reverse(); // Return in chronological order
        Ok(segments)
    }

    pub fn delete_meeting(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        // Segments cascade-deleted via FK
        conn.execute("DELETE FROM meetings WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ========================================================================
    // Segments
    // ========================================================================

    pub fn insert_segment(&self, seg: &SegmentRow) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO transcript_segments (id, meeting_id, time_label, text, timestamp_ms, speaker_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![seg.id, seg.meeting_id, seg.time_label, seg.text, seg.timestamp_ms, seg.speaker_id],
        )?;
        Ok(())
    }

    pub fn get_segments(&self, meeting_id: &str) -> Result<Vec<SegmentRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, meeting_id, time_label, text, timestamp_ms, speaker_id, enhanced_text, is_question, question_answer FROM transcript_segments WHERE meeting_id = ?1 ORDER BY timestamp_ms ASC",
        )?;
        let segments = stmt
            .query_map(params![meeting_id], |row| {
                Ok(SegmentRow {
                    id: row.get(0)?,
                    meeting_id: row.get(1)?,
                    time_label: row.get(2)?,
                    text: row.get(3)?,
                    timestamp_ms: row.get(4)?,
                    speaker_id: row.get(5)?,
                    enhanced_text: row.get(6)?,
                    is_question: row.get::<_, i32>(7)? != 0,
                    question_answer: row.get(8)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(segments)
    }

    /// Update segment text
    pub fn update_segment_text(&self, segment_id: &str, text: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE transcript_segments SET text = ?1 WHERE id = ?2",
            params![text, segment_id],
        )?;
        Ok(())
    }

    /// Update segment enhanced text
    pub fn update_segment_enhanced_text(&self, segment_id: &str, enhanced_text: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE transcript_segments SET enhanced_text = ?1 WHERE id = ?2",
            params![enhanced_text, segment_id],
        )?;
        Ok(())
    }

    /// Update segment question and answer
    pub fn update_segment_question(&self, segment_id: &str, is_question: bool, answer: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE transcript_segments SET is_question = ?1, question_answer = ?2 WHERE id = ?3",
            params![is_question as i32, answer, segment_id],
        )?;
        Ok(())
    }

    /// Update meeting metadata
    pub fn update_meeting_metadata(&self, id: &str, topics: Option<&str>, action_items: Option<&str>, decisions: Option<&str>, participant_count: i32) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE meetings SET topics = ?1, action_items = ?2, decisions = ?3, participant_count = ?4 WHERE id = ?5",
            params![topics, action_items, decisions, participant_count, id],
        )?;
        Ok(())
    }

    /// Update segment speaker
    pub fn update_segment_speaker(&self, segment_id: &str, speaker_id: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE transcript_segments SET speaker_id = ?1 WHERE id = ?2",
            params![speaker_id, segment_id],
        )?;
        Ok(())
    }

    /// Delete a segment
    pub fn delete_segment(&self, segment_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        // Also delete embedding if exists
        conn.execute(
            "DELETE FROM segment_embeddings WHERE segment_id = ?1",
            params![segment_id],
        )?;
        conn.execute(
            "DELETE FROM transcript_segments WHERE id = ?1",
            params![segment_id],
        )?;
        Ok(())
    }

    // ========================================================================
    // Speakers
    // ========================================================================

    /// List all speakers
    pub fn list_speakers(&self) -> Result<Vec<Speaker>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, color, created_at FROM speakers ORDER BY name ASC"
        )?;
        let speakers = stmt
            .query_map([], |row| {
                Ok(Speaker {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(speakers)
    }

    /// Create a new speaker
    pub fn create_speaker(&self, id: &str, name: &str, color: &str, created_at: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO speakers (id, name, color, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, color, created_at],
        )?;
        Ok(())
    }

    /// Update a speaker
    pub fn update_speaker(&self, id: &str, name: &str, color: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE speakers SET name = ?1, color = ?2 WHERE id = ?3",
            params![name, color, id],
        )?;
        Ok(())
    }

    /// Delete a speaker (clears speaker_id from segments)
    pub fn delete_speaker(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        // Clear speaker_id from segments first
        conn.execute(
            "UPDATE transcript_segments SET speaker_id = NULL WHERE speaker_id = ?1",
            params![id],
        )?;
        conn.execute(
            "DELETE FROM speakers WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Get speaker by ID
    pub fn get_speaker(&self, id: &str) -> Result<Option<Speaker>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, color, created_at FROM speakers WHERE id = ?1"
        )?;
        let speaker = stmt.query_row(params![id], |row| {
            Ok(Speaker {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: row.get(3)?,
            })
        }).optional()?;
        Ok(speaker)
    }

    // ========================================================================
    // Search
    // ========================================================================

    pub fn search_transcripts(&self, query: &str, limit: i64) -> Result<Vec<SearchResult>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT ts.meeting_id, m.title, ts.id, ts.text, ts.time_label,
                    snippet(transcript_fts, 0, '<b>', '</b>', '...', 32) as snip
             FROM transcript_fts
             JOIN transcript_segments ts ON ts.rowid = transcript_fts.rowid
             JOIN meetings m ON m.id = ts.meeting_id
             WHERE transcript_fts MATCH ?1
             LIMIT ?2",
        )?;
        let results = stmt
            .query_map(params![query, limit], |row| {
                Ok(SearchResult {
                    meeting_id: row.get(0)?,
                    meeting_title: row.get(1)?,
                    segment_id: row.get(2)?,
                    text: row.get(3)?,
                    time_label: row.get(4)?,
                    snippet: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(results)
    }

    // ========================================================================
    // Embeddings
    // ========================================================================

    /// Store an embedding (f32 LE bytes) for a segment
    pub fn insert_embedding(&self, segment_id: &str, embedding: &[f32]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        conn.execute(
            "INSERT OR REPLACE INTO segment_embeddings (segment_id, embedding) VALUES (?1, ?2)",
            params![segment_id, bytes],
        )?;
        Ok(())
    }

    /// Cosine similarity search across embeddings, optionally scoped to one meeting
    pub fn search_semantic(
        &self,
        query_emb: &[f32],
        limit: usize,
        meeting_id: Option<&str>,
    ) -> Result<Vec<SemanticSearchResult>> {
        let conn = self.conn.lock().unwrap();

        let sql = if meeting_id.is_some() {
            "SELECT se.segment_id, se.embedding, ts.text, ts.time_label, ts.meeting_id, m.title
             FROM segment_embeddings se
             JOIN transcript_segments ts ON ts.id = se.segment_id
             JOIN meetings m ON m.id = ts.meeting_id
             WHERE ts.meeting_id = ?1"
        } else {
            "SELECT se.segment_id, se.embedding, ts.text, ts.time_label, ts.meeting_id, m.title
             FROM segment_embeddings se
             JOIN transcript_segments ts ON ts.id = se.segment_id
             JOIN meetings m ON m.id = ts.meeting_id"
        };

        let mut stmt = conn.prepare(sql)?;

        let rows: Vec<(String, Vec<u8>, String, String, String, String)> = if let Some(mid) = meeting_id {
            stmt.query_map(params![mid], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        };

        // Compute cosine similarity in Rust and sort
        let mut scored: Vec<SemanticSearchResult> = rows
            .into_iter()
            .filter_map(|(seg_id, emb_bytes, text, time_label, mid, title)| {
                let emb = bytes_to_f32(&emb_bytes);
                if emb.len() != query_emb.len() {
                    return None;
                }
                let score = cosine_similarity(query_emb, &emb);
                Some(SemanticSearchResult {
                    meeting_id: mid,
                    meeting_title: title,
                    segment_id: seg_id,
                    text,
                    time_label,
                    score,
                })
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        Ok(scored)
    }

    /// Get segment IDs that don't have embeddings yet for a given meeting
    pub fn get_unembedded_segment_ids(&self, meeting_id: &str) -> Result<Vec<(String, String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT ts.id, ts.time_label, ts.text
             FROM transcript_segments ts
             LEFT JOIN segment_embeddings se ON se.segment_id = ts.id
             WHERE ts.meeting_id = ?1 AND se.segment_id IS NULL
             ORDER BY ts.timestamp_ms ASC",
        )?;
        let rows = stmt
            .query_map(params![meeting_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Count embedded vs total segments
    pub fn count_embeddings(&self) -> Result<(u64, u64)> {
        let conn = self.conn.lock().unwrap();
        let embedded: u64 = conn.query_row(
            "SELECT COUNT(*) FROM segment_embeddings",
            [],
            |row| row.get(0),
        )?;
        let total: u64 = conn.query_row(
            "SELECT COUNT(*) FROM transcript_segments",
            [],
            |row| row.get(0),
        )?;
        Ok((embedded, total))
    }

    // ========================================================================
    // Settings
    // ========================================================================

    pub fn save_settings_json(&self, json: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('app_settings', ?1)",
            params![json],
        )?;
        Ok(())
    }

    pub fn load_settings_json(&self) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT value FROM settings WHERE key = 'app_settings'")?;
        let mut rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        match rows.next() {
            Some(Ok(val)) => Ok(Some(val)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

fn bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom > 0.0 { dot / denom } else { 0.0 }
}
