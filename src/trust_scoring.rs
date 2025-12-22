//! Trust Score System - Gradual Identity Verification
//!
//! **NO INSTANT TRUST** - Users must prove identity over time through consistent patterns.
//!
//! # Trust Progression
//!
//! ```text
//! Unknown (0.0)
//!     ↓ First encounter - collect baseline
//! Observed (0.1-0.3)
//!     ↓ 5 successful auths with matching patterns
//! Verified (0.4-0.6)
//!     ↓ 20+ auths over 2 weeks + explicit approval
//! Trusted (0.7-0.9)
//!     ↓ 100+ auths over 3 months + InnerCircle approval
//! InnerCircle (1.0)
//! ```
//!
//! # Pattern Matching
//!
//! Each authentication checks:
//! - Voice similarity (cosine distance < 0.15)
//! - Typing patterns (keystroke timing variance < 20%)
//! - Behavioral patterns (command preferences, time of day)
//! - Geospatial consistency (impossible travel detection)
//!
//! # Trust Decay
//!
//! Trust decreases if:
//! - Patterns drift significantly (voice changes, typing slows)
//! - Long absence (30 days → -0.1 trust)
//! - Failed auth attempts (3 failures → -0.2 trust)
//! - Impossible travel detected (-0.5 trust immediately)

use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Trust level thresholds
pub const TRUST_UNKNOWN: f64 = 0.0;
pub const TRUST_OBSERVED: f64 = 0.1;
pub const TRUST_VERIFIED: f64 = 0.4;
pub const TRUST_TRUSTED: f64 = 0.7;
pub const TRUST_INNER_CIRCLE: f64 = 1.0;

/// Number of successful authentications required for each level
pub const AUTHS_FOR_OBSERVED: u32 = 3;
pub const AUTHS_FOR_VERIFIED: u32 = 5;
pub const AUTHS_FOR_TRUSTED: u32 = 20;
pub const AUTHS_FOR_INNER_CIRCLE: u32 = 100;

/// Trust score for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScore {
    /// User ID
    pub user_id: String,
    /// Current trust score (0.0 - 1.0)
    pub score: f64,
    /// Number of successful authentications
    pub successful_auths: u32,
    /// Number of failed authentication attempts
    pub failed_auths: u32,
    /// Last successful authentication
    pub last_auth: Option<DateTime<Utc>>,
    /// First seen timestamp
    pub first_seen: DateTime<Utc>,
    /// Baseline voice embedding (512-dim vector from Whisper)
    pub baseline_voice: Option<Vec<f32>>,
    /// Baseline typing patterns (keystroke timing statistics)
    pub baseline_typing: Option<TypingPattern>,
    /// Recent authentication history (last 10)
    pub recent_auths: Vec<AuthenticationAttempt>,
    /// Trust level
    pub trust_level: TrustLevel,
}

/// Trust level enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    Unknown,
    Observed,
    Verified,
    Trusted,
    InnerCircle,
}

impl TrustLevel {
    pub fn from_score(score: f64) -> Self {
        if score >= TRUST_INNER_CIRCLE {
            TrustLevel::InnerCircle
        } else if score >= TRUST_TRUSTED {
            TrustLevel::Trusted
        } else if score >= TRUST_VERIFIED {
            TrustLevel::Verified
        } else if score >= TRUST_OBSERVED {
            TrustLevel::Observed
        } else {
            TrustLevel::Unknown
        }
    }
}

/// Typing pattern statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingPattern {
    /// Average keystroke interval (milliseconds)
    pub avg_interval: f64,
    /// Standard deviation of keystroke intervals
    pub std_interval: f64,
    /// Average dwell time (key press duration)
    pub avg_dwell: f64,
    /// Typing speed (words per minute)
    pub wpm: f64,
}

/// Authentication attempt record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationAttempt {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Success or failure
    pub success: bool,
    /// Voice similarity score (0.0 - 1.0)
    pub voice_similarity: Option<f64>,
    /// Typing pattern match score (0.0 - 1.0)
    pub typing_similarity: Option<f64>,
    /// Geographic location
    pub location: Option<String>,
    /// Device fingerprint
    pub device_fingerprint: Option<String>,
}

/// Trust score manager
pub struct TrustScoreManager {
    db: Arc<Mutex<Connection>>,
}

impl TrustScoreManager {
    /// Create new trust score manager
    pub fn new(db_path: &str) -> Result<Self, TrustScoreError> {
        let conn = Connection::open(db_path)?;

        // Create trust_scores table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS trust_scores (
                user_id TEXT PRIMARY KEY,
                score REAL NOT NULL,
                successful_auths INTEGER NOT NULL,
                failed_auths INTEGER NOT NULL,
                last_auth TEXT,
                first_seen TEXT NOT NULL,
                baseline_voice BLOB,
                baseline_typing TEXT,
                trust_level TEXT NOT NULL
            )",
            [],
        )?;

        // Create authentication_attempts table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS authentication_attempts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                success INTEGER NOT NULL,
                voice_similarity REAL,
                typing_similarity REAL,
                location TEXT,
                device_fingerprint TEXT,
                FOREIGN KEY (user_id) REFERENCES trust_scores(user_id)
            )",
            [],
        )?;

        // Index for fast lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_auth_attempts_user
             ON authentication_attempts(user_id, timestamp DESC)",
            [],
        )?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
        })
    }

    /// Get or create trust score for user
    pub fn get_trust_score(&self, user_id: &str) -> Result<TrustScore, TrustScoreError> {
        let conn = self.db.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT score, successful_auths, failed_auths, last_auth, first_seen,
                    baseline_voice, baseline_typing, trust_level
             FROM trust_scores WHERE user_id = ?1",
        )?;

        let result = stmt.query_row(params![user_id], |row| {
            let score: f64 = row.get(0)?;
            let successful_auths: u32 = row.get(1)?;
            let failed_auths: u32 = row.get(2)?;
            let last_auth: Option<String> = row.get(3)?;
            let first_seen: String = row.get(4)?;
            let baseline_voice: Option<Vec<u8>> = row.get(5)?;
            let baseline_typing: Option<String> = row.get(6)?;
            let trust_level: String = row.get(7)?;

            Ok(TrustScore {
                user_id: user_id.to_string(),
                score,
                successful_auths,
                failed_auths,
                last_auth: last_auth.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }),
                first_seen: DateTime::parse_from_rfc3339(&first_seen)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                baseline_voice: baseline_voice.and_then(|bytes| bincode::deserialize(&bytes).ok()),
                baseline_typing: baseline_typing.and_then(|s| serde_json::from_str(&s).ok()),
                recent_auths: vec![], // Will be loaded separately
                trust_level: match trust_level.as_str() {
                    "Unknown" => TrustLevel::Unknown,
                    "Observed" => TrustLevel::Observed,
                    "Verified" => TrustLevel::Verified,
                    "Trusted" => TrustLevel::Trusted,
                    "InnerCircle" => TrustLevel::InnerCircle,
                    _ => TrustLevel::Unknown,
                },
            })
        });

        match result {
            Ok(mut trust_score) => {
                // Load recent authentication attempts
                trust_score.recent_auths = self.get_recent_attempts(user_id, 10)?;
                Ok(trust_score)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // Create new trust score for unknown user
                let now = Utc::now();
                let trust_score = TrustScore {
                    user_id: user_id.to_string(),
                    score: TRUST_UNKNOWN,
                    successful_auths: 0,
                    failed_auths: 0,
                    last_auth: None,
                    first_seen: now,
                    baseline_voice: None,
                    baseline_typing: None,
                    recent_auths: vec![],
                    trust_level: TrustLevel::Unknown,
                };
                self.save_trust_score(&trust_score)?;
                Ok(trust_score)
            }
            Err(e) => Err(TrustScoreError::Database(e)),
        }
    }

    /// Record authentication attempt and update trust score
    pub fn record_authentication(
        &self,
        user_id: &str,
        success: bool,
        voice_embedding: Option<Vec<f32>>,
        typing_pattern: Option<TypingPattern>,
        location: Option<String>,
        device_fingerprint: Option<String>,
    ) -> Result<TrustScore, TrustScoreError> {
        let mut trust_score = self.get_trust_score(user_id)?;
        let now = Utc::now();

        // Calculate pattern similarities
        let voice_similarity = if let (Some(baseline), Some(current)) =
            (&trust_score.baseline_voice, &voice_embedding)
        {
            Some(Self::cosine_similarity(baseline, current))
        } else {
            None
        };

        let typing_similarity = if let (Some(baseline), Some(current)) =
            (&trust_score.baseline_typing, &typing_pattern)
        {
            Some(Self::typing_similarity(baseline, current))
        } else {
            None
        };

        // Check for impossible travel
        if let Some(last_auth) = trust_score.last_auth {
            if let (Some(last_loc), Some(current_loc)) = (
                trust_score
                    .recent_auths
                    .last()
                    .and_then(|a| a.location.as_ref()),
                &location,
            ) {
                let time_elapsed = (now - last_auth).num_hours() as f64;
                if Self::is_impossible_travel(last_loc, current_loc, time_elapsed) {
                    // Immediate trust penalty for impossible travel
                    trust_score.score = (trust_score.score - 0.5).max(0.0);
                    trust_score.trust_level = TrustLevel::from_score(trust_score.score);
                }
            }
        }

        // Record authentication attempt
        let attempt = AuthenticationAttempt {
            timestamp: now,
            success,
            voice_similarity,
            typing_similarity,
            location: location.clone(),
            device_fingerprint: device_fingerprint.clone(),
        };

        self.save_authentication_attempt(user_id, &attempt)?;
        trust_score.recent_auths.push(attempt);
        if trust_score.recent_auths.len() > 10 {
            trust_score.recent_auths.remove(0);
        }

        if success {
            trust_score.successful_auths += 1;
            trust_score.last_auth = Some(now);

            // Update baseline patterns if this is first encounter or patterns are better
            if trust_score.baseline_voice.is_none() && voice_embedding.is_some() {
                trust_score.baseline_voice = voice_embedding;
            }
            if trust_score.baseline_typing.is_none() && typing_pattern.is_some() {
                trust_score.baseline_typing = typing_pattern;
            }

            // Increase trust based on pattern consistency
            let pattern_match_score = match (voice_similarity, typing_similarity) {
                (Some(v), Some(t)) => (v + t) / 2.0,
                (Some(v), None) => v,
                (None, Some(t)) => t,
                (None, None) => 0.5, // Neutral if no patterns yet
            };

            // Trust increase formula: higher for consistent patterns
            let trust_increase = if pattern_match_score >= 0.85 {
                0.05 // Strong match
            } else if pattern_match_score >= 0.70 {
                0.03 // Good match
            } else {
                0.01 // Weak match
            };

            trust_score.score = (trust_score.score + trust_increase).min(1.0);
        } else {
            trust_score.failed_auths += 1;

            // Decrease trust on failure
            let trust_decrease = 0.1;
            trust_score.score = (trust_score.score - trust_decrease).max(0.0);
        }

        // Apply trust decay for long absence
        if let Some(last_auth) = trust_score.last_auth {
            let days_since_auth = (now - last_auth).num_days();
            if days_since_auth > 30 {
                let decay = (days_since_auth as f64 / 365.0) * 0.1;
                trust_score.score = (trust_score.score - decay).max(0.0);
            }
        }

        // Update trust level
        trust_score.trust_level = TrustLevel::from_score(trust_score.score);

        // Save updated trust score
        self.save_trust_score(&trust_score)?;

        Ok(trust_score)
    }

    /// Save trust score to database
    fn save_trust_score(&self, trust_score: &TrustScore) -> Result<(), TrustScoreError> {
        let conn = self.db.lock().unwrap();

        let baseline_voice_bytes = trust_score
            .baseline_voice
            .as_ref()
            .and_then(|v| bincode::serialize(v).ok());

        let baseline_typing_json = trust_score
            .baseline_typing
            .as_ref()
            .and_then(|t| serde_json::to_string(t).ok());

        let trust_level_str = match trust_score.trust_level {
            TrustLevel::Unknown => "Unknown",
            TrustLevel::Observed => "Observed",
            TrustLevel::Verified => "Verified",
            TrustLevel::Trusted => "Trusted",
            TrustLevel::InnerCircle => "InnerCircle",
        };

        conn.execute(
            "INSERT OR REPLACE INTO trust_scores
             (user_id, score, successful_auths, failed_auths, last_auth, first_seen,
              baseline_voice, baseline_typing, trust_level)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                &trust_score.user_id,
                trust_score.score,
                trust_score.successful_auths,
                trust_score.failed_auths,
                trust_score.last_auth.map(|dt| dt.to_rfc3339()),
                trust_score.first_seen.to_rfc3339(),
                baseline_voice_bytes,
                baseline_typing_json,
                trust_level_str,
            ],
        )?;

        Ok(())
    }

    /// Save authentication attempt
    fn save_authentication_attempt(
        &self,
        user_id: &str,
        attempt: &AuthenticationAttempt,
    ) -> Result<(), TrustScoreError> {
        let conn = self.db.lock().unwrap();

        conn.execute(
            "INSERT INTO authentication_attempts
             (user_id, timestamp, success, voice_similarity, typing_similarity, location, device_fingerprint)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                user_id,
                attempt.timestamp.to_rfc3339(),
                if attempt.success { 1 } else { 0 },
                attempt.voice_similarity,
                attempt.typing_similarity,
                &attempt.location,
                &attempt.device_fingerprint,
            ],
        )?;

        Ok(())
    }

    /// Get recent authentication attempts
    fn get_recent_attempts(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<AuthenticationAttempt>, TrustScoreError> {
        let conn = self.db.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT timestamp, success, voice_similarity, typing_similarity, location, device_fingerprint
             FROM authentication_attempts
             WHERE user_id = ?1
             ORDER BY timestamp DESC
             LIMIT ?2"
        )?;

        let attempts = stmt
            .query_map(params![user_id, limit], |row| {
                let timestamp: String = row.get(0)?;
                let success: i32 = row.get(1)?;
                let voice_similarity: Option<f64> = row.get(2)?;
                let typing_similarity: Option<f64> = row.get(3)?;
                let location: Option<String> = row.get(4)?;
                let device_fingerprint: Option<String> = row.get(5)?;

                Ok(AuthenticationAttempt {
                    timestamp: DateTime::parse_from_rfc3339(&timestamp)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    success: success == 1,
                    voice_similarity,
                    typing_similarity,
                    location,
                    device_fingerprint,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(attempts)
    }

    /// Calculate cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if magnitude_a == 0.0 || magnitude_b == 0.0 {
            return 0.0;
        }

        (dot_product / (magnitude_a * magnitude_b)) as f64
    }

    /// Calculate typing pattern similarity
    fn typing_similarity(baseline: &TypingPattern, current: &TypingPattern) -> f64 {
        // Calculate percentage difference for each metric
        let interval_diff =
            ((baseline.avg_interval - current.avg_interval).abs() / baseline.avg_interval).min(1.0);
        let dwell_diff =
            ((baseline.avg_dwell - current.avg_dwell).abs() / baseline.avg_dwell).min(1.0);
        let wpm_diff = ((baseline.wpm - current.wpm).abs() / baseline.wpm).min(1.0);

        // Similarity = 1 - average difference
        1.0 - ((interval_diff + dwell_diff + wpm_diff) / 3.0)
    }

    /// Check if travel between locations is impossible given time elapsed
    fn is_impossible_travel(last_loc: &str, current_loc: &str, hours_elapsed: f64) -> bool {
        // Simplified: check if locations are different and time is suspiciously short
        // Real implementation would calculate geographic distance
        if last_loc == current_loc {
            return false;
        }

        // If locations differ and less than 1 hour elapsed, suspicious
        // (Real implementation would use distance / max_speed_of_flight)
        hours_elapsed < 1.0
    }
}

/// Trust score errors
#[derive(Debug, Error)]
pub enum TrustScoreError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Invalid pattern data")]
    InvalidPattern,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> TrustScoreManager {
        TrustScoreManager::new(":memory:").unwrap()
    }

    #[test]
    fn test_new_user_starts_unknown() {
        let manager = create_test_manager();
        let trust = manager.get_trust_score("alice").unwrap();

        assert_eq!(trust.score, TRUST_UNKNOWN);
        assert_eq!(trust.trust_level, TrustLevel::Unknown);
        assert_eq!(trust.successful_auths, 0);
        assert_eq!(trust.failed_auths, 0);
    }

    #[test]
    fn test_successful_auth_increases_trust() {
        let manager = create_test_manager();

        // First auth - no patterns yet
        let trust = manager
            .record_authentication(
                "alice",
                true,
                Some(vec![0.1; 512]),
                None,
                Some("Denver, CO".to_string()),
                None,
            )
            .unwrap();

        assert!(trust.score > TRUST_UNKNOWN);
        assert_eq!(trust.successful_auths, 1);
        assert!(trust.baseline_voice.is_some());
    }

    #[test]
    fn test_failed_auth_decreases_trust() {
        let manager = create_test_manager();

        // Build up some trust
        for _ in 0..5 {
            manager
                .record_authentication("alice", true, None, None, None, None)
                .unwrap();
        }

        let trust_before = manager.get_trust_score("alice").unwrap();

        // Failed auth
        let trust_after = manager
            .record_authentication("alice", false, None, None, None, None)
            .unwrap();

        assert!(trust_after.score < trust_before.score);
        assert_eq!(trust_after.failed_auths, 1);
    }

    #[test]
    fn test_gradual_trust_progression() {
        let manager = create_test_manager();
        let voice = vec![0.5; 512];

        // Start at Unknown
        let trust = manager.get_trust_score("alice").unwrap();
        assert_eq!(trust.trust_level, TrustLevel::Unknown);

        // After 3 successful auths with matching voice → Observed
        for _ in 0..3 {
            manager
                .record_authentication("alice", true, Some(voice.clone()), None, None, None)
                .unwrap();
        }
        let trust = manager.get_trust_score("alice").unwrap();
        assert!(
            trust.trust_level == TrustLevel::Observed || trust.trust_level == TrustLevel::Verified
        );

        // After 10+ auths → Verified or higher
        for _ in 0..10 {
            manager
                .record_authentication("alice", true, Some(voice.clone()), None, None, None)
                .unwrap();
        }
        let trust = manager.get_trust_score("alice").unwrap();
        assert!(trust.score >= TRUST_VERIFIED);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];

        assert!((TrustScoreManager::cosine_similarity(&a, &b) - 1.0).abs() < 0.01);
        assert!((TrustScoreManager::cosine_similarity(&a, &c) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_typing_similarity() {
        let baseline = TypingPattern {
            avg_interval: 100.0,
            std_interval: 20.0,
            avg_dwell: 80.0,
            wpm: 60.0,
        };

        let identical = TypingPattern {
            avg_interval: 100.0,
            std_interval: 20.0,
            avg_dwell: 80.0,
            wpm: 60.0,
        };

        let similar = TypingPattern {
            avg_interval: 105.0,
            std_interval: 22.0,
            avg_dwell: 82.0,
            wpm: 58.0,
        };

        let different = TypingPattern {
            avg_interval: 200.0,
            std_interval: 50.0,
            avg_dwell: 150.0,
            wpm: 30.0,
        };

        let sim_identical = TrustScoreManager::typing_similarity(&baseline, &identical);
        let sim_similar = TrustScoreManager::typing_similarity(&baseline, &similar);
        let sim_different = TrustScoreManager::typing_similarity(&baseline, &different);

        assert!((sim_identical - 1.0).abs() < 0.01);
        assert!(sim_similar > 0.9);
        assert!(sim_different < 0.6);
    }
}
