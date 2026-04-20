use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::AppError;

/// Persisted score record for leaderboard.
#[derive(Debug, Clone)]
pub struct ScoreRecord {
    pub id: i64,
    pub track_name: String,
    pub difficulty: String,
    pub timing_preset: String,
    pub bpm: f64,
    pub scope: String,
    pub score_pct: f64,
    pub perfect: u32,
    pub great: u32,
    pub good: u32,
    pub ok: u32,
    pub miss: u32,
    pub wrong_piece: u32,
    pub max_combo: u32,
    pub played_at: i64,
}

/// User preferences stored per profile.
#[derive(Debug, Clone, Default)]
pub struct Preferences {
    pub last_track: Option<String>,
    pub last_kit: Option<String>,
    pub last_theme: Option<String>,
    pub last_bpm: Option<f64>,
    pub last_difficulty: Option<String>,
    pub last_timing: Option<String>,
}

/// Database connection wrapper for SQLite.
pub struct Database {
    pub conn: Connection,
}

impl Database {
    /// Open or create the database at the given path and initialize the schema.
    pub fn open(path: &Path) -> Result<Self, AppError> {
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Open an in-memory SQLite database (used for testing).
    pub fn open_in_memory() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory()?;
        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Create all tables and indexes if they don't exist.
    fn init_schema(&self) -> Result<(), AppError> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS profile (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                github_id TEXT,
                github_username TEXT
            );

            CREATE TABLE IF NOT EXISTS scores (
                id INTEGER PRIMARY KEY,
                profile_id INTEGER NOT NULL REFERENCES profile(id),
                track_name TEXT NOT NULL,
                track_hash TEXT NOT NULL,
                difficulty TEXT NOT NULL,
                timing_preset TEXT NOT NULL,
                bpm REAL NOT NULL,
                scope TEXT NOT NULL,
                score_pct REAL NOT NULL,
                perfect_count INTEGER NOT NULL,
                great_count INTEGER NOT NULL,
                good_count INTEGER NOT NULL,
                ok_count INTEGER NOT NULL,
                miss_count INTEGER NOT NULL,
                wrong_piece_count INTEGER NOT NULL,
                max_combo INTEGER NOT NULL,
                played_at INTEGER NOT NULL,
                UNIQUE(profile_id, track_name, track_hash, difficulty, timing_preset, bpm, scope, played_at)
            );

            CREATE INDEX IF NOT EXISTS idx_scores_track ON scores(track_name, scope, score_pct DESC);
            CREATE INDEX IF NOT EXISTS idx_scores_profile ON scores(profile_id, played_at DESC);

            CREATE TABLE IF NOT EXISTS preferences (
                profile_id INTEGER PRIMARY KEY REFERENCES profile(id),
                last_track TEXT,
                last_kit TEXT,
                last_theme TEXT,
                last_bpm REAL,
                last_difficulty TEXT,
                last_timing TEXT
            );
            ",
        )?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Score CRUD
    // -------------------------------------------------------------------------

    /// Insert a score record for a profile. Ignores duplicates silently (OR IGNORE).
    pub fn save_score(
        &self,
        profile_id: i64,
        score: &ScoreRecord,
        track_hash: &str,
    ) -> Result<(), AppError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO scores (
                profile_id, track_name, track_hash, difficulty, timing_preset,
                bpm, scope, score_pct, perfect_count, great_count, good_count,
                ok_count, miss_count, wrong_piece_count, max_combo, played_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                profile_id,
                score.track_name,
                track_hash,
                score.difficulty,
                score.timing_preset,
                score.bpm,
                score.scope,
                score.score_pct,
                score.perfect,
                score.great,
                score.good,
                score.ok,
                score.miss,
                score.wrong_piece,
                score.max_combo,
                score.played_at,
            ],
        )?;
        Ok(())
    }

    /// Fetch the top `limit` scores for a track filtered by scope/difficulty/timing,
    /// ordered by score_pct DESC then max_combo DESC.
    pub fn top_scores(
        &self,
        track_name: &str,
        scope: &str,
        difficulty: &str,
        timing: &str,
        limit: u32,
    ) -> Result<Vec<ScoreRecord>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, track_name, difficulty, timing_preset, bpm, scope,
                    score_pct, perfect_count, great_count, good_count, ok_count,
                    miss_count, wrong_piece_count, max_combo, played_at
             FROM scores
             WHERE track_name = ?1 AND scope = ?2 AND difficulty = ?3 AND timing_preset = ?4
             ORDER BY score_pct DESC, max_combo DESC
             LIMIT ?5",
        )?;

        let rows = stmt.query_map(params![track_name, scope, difficulty, timing, limit], |row| {
            Ok(ScoreRecord {
                id: row.get(0)?,
                track_name: row.get(1)?,
                difficulty: row.get(2)?,
                timing_preset: row.get(3)?,
                bpm: row.get(4)?,
                scope: row.get(5)?,
                score_pct: row.get(6)?,
                perfect: row.get::<_, i64>(7)? as u32,
                great: row.get::<_, i64>(8)? as u32,
                good: row.get::<_, i64>(9)? as u32,
                ok: row.get::<_, i64>(10)? as u32,
                miss: row.get::<_, i64>(11)? as u32,
                wrong_piece: row.get::<_, i64>(12)? as u32,
                max_combo: row.get::<_, i64>(13)? as u32,
                played_at: row.get(14)?,
            })
        })?;

        let mut scores = Vec::new();
        for row in rows {
            scores.push(row?);
        }
        Ok(scores)
    }

    /// Fetch the most recent `limit` scores for a profile on a specific track.
    pub fn recent_scores(
        &self,
        profile_id: i64,
        track_name: &str,
        limit: u32,
    ) -> Result<Vec<ScoreRecord>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, track_name, difficulty, timing_preset, bpm, scope,
                    score_pct, perfect_count, great_count, good_count, ok_count,
                    miss_count, wrong_piece_count, max_combo, played_at
             FROM scores
             WHERE profile_id = ?1 AND track_name = ?2
             ORDER BY played_at DESC
             LIMIT ?3",
        )?;

        let rows = stmt.query_map(params![profile_id, track_name, limit], |row| {
            Ok(ScoreRecord {
                id: row.get(0)?,
                track_name: row.get(1)?,
                difficulty: row.get(2)?,
                timing_preset: row.get(3)?,
                bpm: row.get(4)?,
                scope: row.get(5)?,
                score_pct: row.get(6)?,
                perfect: row.get::<_, i64>(7)? as u32,
                great: row.get::<_, i64>(8)? as u32,
                good: row.get::<_, i64>(9)? as u32,
                ok: row.get::<_, i64>(10)? as u32,
                miss: row.get::<_, i64>(11)? as u32,
                wrong_piece: row.get::<_, i64>(12)? as u32,
                max_combo: row.get::<_, i64>(13)? as u32,
                played_at: row.get(14)?,
            })
        })?;

        let mut scores = Vec::new();
        for row in rows {
            scores.push(row?);
        }
        Ok(scores)
    }

    // -------------------------------------------------------------------------
    // Preferences CRUD
    // -------------------------------------------------------------------------

    /// Upsert preferences for a profile.
    pub fn save_preferences(&self, profile_id: i64, prefs: &Preferences) -> Result<(), AppError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO preferences
                (profile_id, last_track, last_kit, last_theme, last_bpm, last_difficulty, last_timing)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                profile_id,
                prefs.last_track,
                prefs.last_kit,
                prefs.last_theme,
                prefs.last_bpm,
                prefs.last_difficulty,
                prefs.last_timing,
            ],
        )?;
        Ok(())
    }

    /// Load preferences for a profile. Returns None if no row exists yet.
    pub fn load_preferences(&self, profile_id: i64) -> Result<Option<Preferences>, AppError> {
        let result = self
            .conn
            .query_row(
                "SELECT last_track, last_kit, last_theme, last_bpm, last_difficulty, last_timing
                 FROM preferences
                 WHERE profile_id = ?1",
                params![profile_id],
                |row| {
                    Ok(Preferences {
                        last_track: row.get(0)?,
                        last_kit: row.get(1)?,
                        last_theme: row.get(2)?,
                        last_bpm: row.get(3)?,
                        last_difficulty: row.get(4)?,
                        last_timing: row.get(5)?,
                    })
                },
            )
            .optional()?;
        Ok(result)
    }
}

/// Returns the current Unix timestamp in seconds.
pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_score(track_name: &str, scope: &str, score_pct: f64) -> ScoreRecord {
        ScoreRecord {
            id: 0,
            track_name: track_name.to_string(),
            difficulty: "hard".to_string(),
            timing_preset: "standard".to_string(),
            bpm: 120.0,
            scope: scope.to_string(),
            score_pct,
            perfect: 10,
            great: 5,
            good: 2,
            ok: 1,
            miss: 0,
            wrong_piece: 0,
            max_combo: 17,
            played_at: unix_now(),
        }
    }

    #[test]
    fn test_schema_creation() {
        let db = Database::open_in_memory().expect("in-memory db");
        // Schema already created — just verify tables exist by querying them.
        db.conn
            .execute_batch(
                "SELECT 1 FROM profile LIMIT 1;
                 SELECT 1 FROM scores LIMIT 1;
                 SELECT 1 FROM preferences LIMIT 1;",
            )
            .expect("tables should exist");
    }

    #[test]
    fn test_save_and_retrieve_scores() {
        let db = Database::open_in_memory().expect("in-memory db");

        // Create a profile first (FK constraint).
        db.conn
            .execute(
                "INSERT INTO profile (id, name, created_at) VALUES (1, 'Drummer', 1000)",
                [],
            )
            .unwrap();

        let score = make_score("basic-rock", "full", 95.5);
        db.save_score(1, &score, "abc123").expect("save_score");

        let top = db
            .top_scores("basic-rock", "full", "hard", "standard", 10)
            .expect("top_scores");
        assert_eq!(top.len(), 1);
        assert!((top[0].score_pct - 95.5).abs() < 0.001);
        assert_eq!(top[0].max_combo, 17);
    }

    #[test]
    fn test_recent_scores() {
        let db = Database::open_in_memory().expect("in-memory db");

        db.conn
            .execute(
                "INSERT INTO profile (id, name, created_at) VALUES (1, 'Drummer', 1000)",
                [],
            )
            .unwrap();

        let mut s1 = make_score("basic-rock", "full", 80.0);
        s1.played_at = 1000;
        let mut s2 = make_score("basic-rock", "full", 90.0);
        s2.played_at = 2000;

        db.save_score(1, &s1, "hash1").unwrap();
        db.save_score(1, &s2, "hash2").unwrap();

        let recent = db.recent_scores(1, "basic-rock", 10).expect("recent_scores");
        assert_eq!(recent.len(), 2);
        // Most recent first.
        assert_eq!(recent[0].played_at, 2000);
        assert_eq!(recent[1].played_at, 1000);
    }

    #[test]
    fn test_preferences_roundtrip() {
        let db = Database::open_in_memory().expect("in-memory db");

        db.conn
            .execute(
                "INSERT INTO profile (id, name, created_at) VALUES (1, 'Drummer', 1000)",
                [],
            )
            .unwrap();

        // Initially None.
        let prefs = db.load_preferences(1).unwrap();
        assert!(prefs.is_none());

        let new_prefs = Preferences {
            last_track: Some("basic-rock".to_string()),
            last_kit: Some("acoustic".to_string()),
            last_theme: Some("gruvbox".to_string()),
            last_bpm: Some(140.0),
            last_difficulty: Some("hard".to_string()),
            last_timing: Some("standard".to_string()),
        };

        db.save_preferences(1, &new_prefs).unwrap();

        let loaded = db.load_preferences(1).unwrap().expect("prefs should exist");
        assert_eq!(loaded.last_track.as_deref(), Some("basic-rock"));
        assert_eq!(loaded.last_kit.as_deref(), Some("acoustic"));
        assert_eq!(loaded.last_theme.as_deref(), Some("gruvbox"));
        assert!((loaded.last_bpm.unwrap() - 140.0).abs() < 0.001);
        assert_eq!(loaded.last_difficulty.as_deref(), Some("hard"));
        assert_eq!(loaded.last_timing.as_deref(), Some("standard"));
    }

    #[test]
    fn test_duplicate_score_ignored() {
        let db = Database::open_in_memory().expect("in-memory db");

        db.conn
            .execute(
                "INSERT INTO profile (id, name, created_at) VALUES (1, 'Drummer', 1000)",
                [],
            )
            .unwrap();

        let score = make_score("basic-rock", "full", 95.5);
        db.save_score(1, &score, "abc123").expect("first insert");
        // Exact same record — UNIQUE constraint triggers OR IGNORE.
        db.save_score(1, &score, "abc123").expect("duplicate insert silently ignored");

        let top = db.top_scores("basic-rock", "full", "hard", "standard", 10).unwrap();
        assert_eq!(top.len(), 1, "duplicate should not be inserted");
    }
}
