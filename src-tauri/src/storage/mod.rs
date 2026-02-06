// Storage module — SQLite persistence for meetings, transcripts, and settings

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MeetingRow {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub ended_at: Option<String>,
    pub pinned: bool,
    pub duration_ms: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MeetingListItem {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub pinned: bool,
    pub segment_count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SegmentRow {
    pub id: String,
    pub meeting_id: String,
    pub time_label: String,
    pub text: String,
    pub timestamp_ms: i64,
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
            "SELECT id, title, created_at, ended_at, pinned, duration_ms FROM meetings WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(MeetingRow {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                ended_at: row.get(3)?,
                pinned: row.get::<_, i32>(4)? != 0,
                duration_ms: row.get(5)?,
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
            "SELECT m.id, m.title, m.created_at, m.pinned, m.duration_ms,
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
                    segment_count: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items)
    }

    pub fn update_meeting_title(&self, id: &str, title: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE meetings SET title = ?1 WHERE id = ?2",
            params![title, id],
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
            "INSERT INTO transcript_segments (id, meeting_id, time_label, text, timestamp_ms) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![seg.id, seg.meeting_id, seg.time_label, seg.text, seg.timestamp_ms],
        )?;
        Ok(())
    }

    pub fn get_segments(&self, meeting_id: &str) -> Result<Vec<SegmentRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, meeting_id, time_label, text, timestamp_ms FROM transcript_segments WHERE meeting_id = ?1 ORDER BY timestamp_ms ASC",
        )?;
        let segments = stmt
            .query_map(params![meeting_id], |row| {
                Ok(SegmentRow {
                    id: row.get(0)?,
                    meeting_id: row.get(1)?,
                    time_label: row.get(2)?,
                    text: row.get(3)?,
                    timestamp_ms: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(segments)
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
