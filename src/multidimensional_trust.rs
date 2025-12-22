//! Multi-Dimensional Trust System
//!
//! **CRITICAL INSIGHT:** Single scalar trust is dangerous and attackable.
//!
//! # The Problem with Scalar Trust
//!
//! ```text
//! Attacker with good deepfake voice:
//!   voice_similarity: 0.95 ✅
//!   typing_similarity: N/A (no data)
//!   location_consistency: 0.3 ❌
//!   → Overall trust: 0.6 (gets Verified!)
//!
//! Legitimate user with new mic:
//!   voice_similarity: 0.65 (different mic) ❌
//!   typing_similarity: 0.95 ✅
//!   location_consistency: 0.98 ✅
//!   → Overall trust drops unfairly
//! ```
//!
//! # Solution: Orthogonal Trust Dimensions
//!
//! Track separate trust scores for independent modalities:
//!
//! 1. **Identity Trust (Voice)** - Biometric voice patterns
//! 2. **Identity Trust (Typing)** - Behavioral keystroke patterns
//! 3. **Identity Trust (Face)** - Facial recognition (future)
//! 4. **Location Consistency** - Geospatial patterns
//! 5. **Relationship Trust** - Interaction history, diversity, authority
//! 6. **Device Trust** - Known device fingerprints
//!
//! # Policy-Based Access Control
//!
//! Define per-operation requirements:
//!
//! ```rust
//! // InnerCircle operation
//! requires: voice >= 0.9 AND location >= 0.8 AND relationship >= 0.7
//!
//! // Normal access
//! requires: ANY 2 of (voice >= 0.7, typing >= 0.7, device >= 0.8)
//!
//! // Read-only
//! requires: relationship >= 0.3
//! ```
//!
//! # Familiarity vs. Authority
//!
//! **Familiarity:** Increases with interactions (speed-runnable)
//! **Authority:** Gated by explicit designation (not speed-runnable)
//!
//! ```text
//! Daily user with 100 auths in 1 week:
//!   familiarity: 0.9 (high)
//!   authority: 0.0 (not designated)
//!   → Can access: normal operations
//!   → Cannot access: InnerCircle (needs explicit designation)
//!
//! Magnus explicitly designates Mark (HR):
//!   familiarity: 0.3 (only 5 auths)
//!   authority: 1.0 (explicitly designated)
//!   → Can access: HR operations immediately
//! ```

use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Multi-dimensional trust scores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustDimensions {
    /// User ID
    pub user_id: String,

    /// Identity trust: Voice biometrics (0.0 - 1.0)
    pub voice_trust: f64,
    /// Identity trust: Typing behavior (0.0 - 1.0)
    pub typing_trust: f64,
    /// Identity trust: Facial recognition (0.0 - 1.0) [future]
    pub face_trust: f64,

    /// Location consistency (0.0 - 1.0)
    pub location_trust: f64,

    /// Device trust: Known devices (0.0 - 1.0)
    pub device_trust: f64,

    /// Relationship trust: Interaction history (0.0 - 1.0)
    pub relationship_trust: f64,

    /// Authority level: Explicit designation (0.0 - 1.0)
    pub authority: f64,

    /// Metadata
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub total_interactions: u32,
    pub successful_auths: u32,
    pub failed_auths: u32,

    /// Explicit designations (e.g., "HR", "successor", "inner_circle")
    pub designations: Vec<String>,
}

impl TrustDimensions {
    /// Create new trust dimensions for unknown user
    pub fn new(user_id: String) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            voice_trust: 0.0,
            typing_trust: 0.0,
            face_trust: 0.0,
            location_trust: 0.0,
            device_trust: 0.0,
            relationship_trust: 0.0,
            authority: 0.0,
            first_seen: now,
            last_seen: now,
            total_interactions: 0,
            successful_auths: 0,
            failed_auths: 0,
            designations: vec![],
        }
    }

    /// Check if user meets trust policy
    pub fn meets_policy(&self, policy: &TrustPolicy) -> bool {
        match policy.mode {
            PolicyMode::All => {
                // All conditions must be met
                policy
                    .conditions
                    .iter()
                    .all(|cond| self.meets_condition(cond))
            }
            PolicyMode::Any => {
                // At least one condition must be met
                policy
                    .conditions
                    .iter()
                    .any(|cond| self.meets_condition(cond))
            }
            PolicyMode::AtLeastN(n) => {
                // At least N conditions must be met
                let met_count = policy
                    .conditions
                    .iter()
                    .filter(|cond| self.meets_condition(cond))
                    .count();
                met_count >= n
            }
        }
    }

    /// Check if user meets a single condition
    fn meets_condition(&self, condition: &TrustCondition) -> bool {
        let actual = match condition.dimension {
            TrustDimension::Voice => self.voice_trust,
            TrustDimension::Typing => self.typing_trust,
            TrustDimension::Face => self.face_trust,
            TrustDimension::Location => self.location_trust,
            TrustDimension::Device => self.device_trust,
            TrustDimension::Relationship => self.relationship_trust,
            TrustDimension::Authority => self.authority,
        };

        actual >= condition.threshold
    }

    /// Check if user has explicit designation
    pub fn has_designation(&self, designation: &str) -> bool {
        self.designations.iter().any(|d| d == designation)
    }
}

/// Trust dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustDimension {
    Voice,
    Typing,
    Face,
    Location,
    Device,
    Relationship,
    Authority,
}

/// Trust policy for an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustPolicy {
    /// Policy name
    pub name: String,
    /// Policy mode (All, Any, AtLeastN)
    pub mode: PolicyMode,
    /// Trust conditions
    pub conditions: Vec<TrustCondition>,
    /// Required designations (if any)
    pub required_designations: Vec<String>,
}

/// Policy mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyMode {
    /// All conditions must be met (AND)
    All,
    /// At least one condition must be met (OR)
    Any,
    /// At least N conditions must be met
    AtLeastN(usize),
}

/// Trust condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustCondition {
    /// Trust dimension
    pub dimension: TrustDimension,
    /// Minimum threshold
    pub threshold: f64,
}

/// Multi-dimensional trust manager
pub struct MultiDimensionalTrustManager {
    db: Arc<Mutex<Connection>>,
}

impl MultiDimensionalTrustManager {
    /// Create new trust manager
    pub fn new(db_path: &str) -> Result<Self, TrustError> {
        let conn = Connection::open(db_path)?;

        // Create trust_dimensions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS trust_dimensions (
                user_id TEXT PRIMARY KEY,
                voice_trust REAL NOT NULL,
                typing_trust REAL NOT NULL,
                face_trust REAL NOT NULL,
                location_trust REAL NOT NULL,
                device_trust REAL NOT NULL,
                relationship_trust REAL NOT NULL,
                authority REAL NOT NULL,
                first_seen TEXT NOT NULL,
                last_seen TEXT NOT NULL,
                total_interactions INTEGER NOT NULL,
                successful_auths INTEGER NOT NULL,
                failed_auths INTEGER NOT NULL,
                designations TEXT NOT NULL
            )",
            [],
        )?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
        })
    }

    /// Get trust dimensions for user
    pub fn get_trust(&self, user_id: &str) -> Result<TrustDimensions, TrustError> {
        let conn = self.db.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT voice_trust, typing_trust, face_trust, location_trust, device_trust,
                    relationship_trust, authority, first_seen, last_seen, total_interactions,
                    successful_auths, failed_auths, designations
             FROM trust_dimensions WHERE user_id = ?1",
        )?;

        let result = stmt.query_row(params![user_id], |row| {
            let designations_json: String = row.get(12)?;
            let designations: Vec<String> =
                serde_json::from_str(&designations_json).unwrap_or_default();

            Ok(TrustDimensions {
                user_id: user_id.to_string(),
                voice_trust: row.get(0)?,
                typing_trust: row.get(1)?,
                face_trust: row.get(2)?,
                location_trust: row.get(3)?,
                device_trust: row.get(4)?,
                relationship_trust: row.get(5)?,
                authority: row.get(6)?,
                first_seen: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                last_seen: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                total_interactions: row.get(9)?,
                successful_auths: row.get(10)?,
                failed_auths: row.get(11)?,
                designations,
            })
        });

        match result {
            Ok(trust) => Ok(trust),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // Create new trust dimensions
                let trust = TrustDimensions::new(user_id.to_string());
                self.save_trust(&trust)?;
                Ok(trust)
            }
            Err(e) => Err(TrustError::Database(e)),
        }
    }

    /// Update trust dimension
    pub fn update_dimension(
        &self,
        user_id: &str,
        dimension: TrustDimension,
        new_value: f64,
    ) -> Result<(), TrustError> {
        let mut trust = self.get_trust(user_id)?;

        // Update specific dimension
        match dimension {
            TrustDimension::Voice => trust.voice_trust = new_value.clamp(0.0, 1.0),
            TrustDimension::Typing => trust.typing_trust = new_value.clamp(0.0, 1.0),
            TrustDimension::Face => trust.face_trust = new_value.clamp(0.0, 1.0),
            TrustDimension::Location => trust.location_trust = new_value.clamp(0.0, 1.0),
            TrustDimension::Device => trust.device_trust = new_value.clamp(0.0, 1.0),
            TrustDimension::Relationship => trust.relationship_trust = new_value.clamp(0.0, 1.0),
            TrustDimension::Authority => trust.authority = new_value.clamp(0.0, 1.0),
        }

        trust.last_seen = Utc::now();
        self.save_trust(&trust)?;
        Ok(())
    }

    /// Record interaction (increases relationship trust)
    pub fn record_interaction(
        &self,
        user_id: &str,
        success: bool,
    ) -> Result<TrustDimensions, TrustError> {
        let mut trust = self.get_trust(user_id)?;

        trust.total_interactions += 1;
        trust.last_seen = Utc::now();

        if success {
            trust.successful_auths += 1;

            // Increase relationship trust based on interaction diversity
            let days_since_first = (Utc::now() - trust.first_seen).num_days();
            let interactions_per_day =
                trust.total_interactions as f64 / (days_since_first.max(1) as f64);

            // Penalize spam (too many interactions per day)
            let spam_penalty = if interactions_per_day > 10.0 {
                0.5 // High frequency = suspicious
            } else {
                1.0
            };

            // Relationship trust increases slowly, requires time diversity
            let relationship_increase =
                (0.01_f64 * spam_penalty).min(1.0 - trust.relationship_trust);
            trust.relationship_trust = (trust.relationship_trust + relationship_increase).min(1.0);
        } else {
            trust.failed_auths += 1;

            // Decrease relationship trust on failure
            trust.relationship_trust = (trust.relationship_trust - 0.05).max(0.0);
        }

        self.save_trust(&trust)?;
        Ok(trust)
    }

    /// Add explicit designation (requires authority)
    pub fn add_designation(&self, user_id: &str, designation: String) -> Result<(), TrustError> {
        let mut trust = self.get_trust(user_id)?;

        if !trust.designations.contains(&designation) {
            trust.designations.push(designation.clone());

            // Designations grant authority
            match designation.as_str() {
                "inner_circle" => trust.authority = 1.0,
                "hr" | "successor" => trust.authority = 0.8,
                "trusted_contact" => trust.authority = 0.5,
                _ => {} // Custom designations don't auto-grant authority
            }

            self.save_trust(&trust)?;
        }

        Ok(())
    }

    /// Remove designation
    pub fn remove_designation(&self, user_id: &str, designation: &str) -> Result<(), TrustError> {
        let mut trust = self.get_trust(user_id)?;

        trust.designations.retain(|d| d != designation);

        // Recalculate authority based on remaining designations
        trust.authority = if trust.has_designation("inner_circle") {
            1.0
        } else if trust.has_designation("hr") || trust.has_designation("successor") {
            0.8
        } else if trust.has_designation("trusted_contact") {
            0.5
        } else {
            0.0
        };

        self.save_trust(&trust)?;
        Ok(())
    }

    /// Save trust dimensions
    fn save_trust(&self, trust: &TrustDimensions) -> Result<(), TrustError> {
        let conn = self.db.lock().unwrap();

        let designations_json =
            serde_json::to_string(&trust.designations).unwrap_or_else(|_| "[]".to_string());

        conn.execute(
            "INSERT OR REPLACE INTO trust_dimensions
             (user_id, voice_trust, typing_trust, face_trust, location_trust, device_trust,
              relationship_trust, authority, first_seen, last_seen, total_interactions,
              successful_auths, failed_auths, designations)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                &trust.user_id,
                trust.voice_trust,
                trust.typing_trust,
                trust.face_trust,
                trust.location_trust,
                trust.device_trust,
                trust.relationship_trust,
                trust.authority,
                trust.first_seen.to_rfc3339(),
                trust.last_seen.to_rfc3339(),
                trust.total_interactions,
                trust.successful_auths,
                trust.failed_auths,
                designations_json,
            ],
        )?;

        Ok(())
    }
}

/// Predefined trust policies
impl TrustPolicy {
    /// Inner Circle operations (HIGHEST security)
    pub fn inner_circle() -> Self {
        Self {
            name: "InnerCircle".to_string(),
            mode: PolicyMode::All,
            conditions: vec![
                TrustCondition {
                    dimension: TrustDimension::Voice,
                    threshold: 0.9,
                },
                TrustCondition {
                    dimension: TrustDimension::Location,
                    threshold: 0.8,
                },
                TrustCondition {
                    dimension: TrustDimension::Relationship,
                    threshold: 0.7,
                },
                TrustCondition {
                    dimension: TrustDimension::Authority,
                    threshold: 1.0,
                },
            ],
            required_designations: vec!["inner_circle".to_string()],
        }
    }

    /// Normal access (MEDIUM security)
    pub fn normal_access() -> Self {
        Self {
            name: "NormalAccess".to_string(),
            mode: PolicyMode::AtLeastN(2),
            conditions: vec![
                TrustCondition {
                    dimension: TrustDimension::Voice,
                    threshold: 0.7,
                },
                TrustCondition {
                    dimension: TrustDimension::Typing,
                    threshold: 0.7,
                },
                TrustCondition {
                    dimension: TrustDimension::Device,
                    threshold: 0.8,
                },
            ],
            required_designations: vec![],
        }
    }

    /// Read-only access (LOW security)
    pub fn read_only() -> Self {
        Self {
            name: "ReadOnly".to_string(),
            mode: PolicyMode::Any,
            conditions: vec![TrustCondition {
                dimension: TrustDimension::Relationship,
                threshold: 0.3,
            }],
            required_designations: vec![],
        }
    }
}

/// Trust errors
#[derive(Debug, Error)]
pub enum TrustError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("User not found: {0}")]
    UserNotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> MultiDimensionalTrustManager {
        MultiDimensionalTrustManager::new(":memory:").unwrap()
    }

    #[test]
    fn test_new_user_all_dimensions_zero() {
        let manager = create_test_manager();
        let trust = manager.get_trust("alice").unwrap();

        assert_eq!(trust.voice_trust, 0.0);
        assert_eq!(trust.typing_trust, 0.0);
        assert_eq!(trust.location_trust, 0.0);
        assert_eq!(trust.relationship_trust, 0.0);
        assert_eq!(trust.authority, 0.0);
    }

    #[test]
    fn test_update_specific_dimension() {
        let manager = create_test_manager();

        // Update voice trust only
        manager
            .update_dimension("alice", TrustDimension::Voice, 0.9)
            .unwrap();

        let trust = manager.get_trust("alice").unwrap();
        assert_eq!(trust.voice_trust, 0.9);
        assert_eq!(trust.typing_trust, 0.0); // Other dimensions unchanged
    }

    #[test]
    fn test_inner_circle_policy_requires_all_conditions() {
        let manager = create_test_manager();
        let policy = TrustPolicy::inner_circle();

        // Set high voice and location, but no authority
        manager
            .update_dimension("alice", TrustDimension::Voice, 0.95)
            .unwrap();
        manager
            .update_dimension("alice", TrustDimension::Location, 0.9)
            .unwrap();
        manager
            .update_dimension("alice", TrustDimension::Relationship, 0.8)
            .unwrap();

        let trust = manager.get_trust("alice").unwrap();
        assert!(!trust.meets_policy(&policy)); // Fails: no authority

        // Add InnerCircle designation
        manager
            .add_designation("alice", "inner_circle".to_string())
            .unwrap();

        let trust = manager.get_trust("alice").unwrap();
        assert!(trust.meets_policy(&policy)); // Now passes
    }

    #[test]
    fn test_normal_access_requires_any_two() {
        let manager = create_test_manager();
        let policy = TrustPolicy::normal_access();

        // Only voice high
        manager
            .update_dimension("alice", TrustDimension::Voice, 0.8)
            .unwrap();
        let trust = manager.get_trust("alice").unwrap();
        assert!(!trust.meets_policy(&policy)); // Only 1 condition met

        // Add typing
        manager
            .update_dimension("alice", TrustDimension::Typing, 0.8)
            .unwrap();
        let trust = manager.get_trust("alice").unwrap();
        assert!(trust.meets_policy(&policy)); // 2 conditions met
    }

    #[test]
    fn test_relationship_trust_penalizes_spam() {
        let manager = create_test_manager();

        // Spam 100 interactions immediately
        for _ in 0..100 {
            manager.record_interaction("alice", true).unwrap();
        }

        let trust = manager.get_trust("alice").unwrap();
        // Relationship trust should be low due to spam penalty
        assert!(trust.relationship_trust < 0.5);
    }

    #[test]
    fn test_designation_grants_authority() {
        let manager = create_test_manager();

        manager
            .add_designation("alice", "inner_circle".to_string())
            .unwrap();
        let trust = manager.get_trust("alice").unwrap();

        assert_eq!(trust.authority, 1.0);
        assert!(trust.has_designation("inner_circle"));
    }

    #[test]
    fn test_remove_designation_removes_authority() {
        let manager = create_test_manager();

        manager
            .add_designation("alice", "inner_circle".to_string())
            .unwrap();
        manager.remove_designation("alice", "inner_circle").unwrap();

        let trust = manager.get_trust("alice").unwrap();
        assert_eq!(trust.authority, 0.0);
        assert!(!trust.has_designation("inner_circle"));
    }

    #[test]
    fn test_deepfake_attack_blocked() {
        let manager = create_test_manager();

        // Attacker has good voice deepfake
        manager
            .update_dimension("attacker", TrustDimension::Voice, 0.95)
            .unwrap();

        // But no typing data, no location consistency
        manager
            .update_dimension("attacker", TrustDimension::Typing, 0.0)
            .unwrap();
        manager
            .update_dimension("attacker", TrustDimension::Location, 0.3)
            .unwrap();

        let trust = manager.get_trust("attacker").unwrap();
        let policy = TrustPolicy::normal_access(); // Requires 2 of 3

        // Should FAIL: only voice is high (1 of 3)
        assert!(!trust.meets_policy(&policy));
    }

    #[test]
    fn test_legitimate_user_new_mic() {
        let manager = create_test_manager();

        // User with new mic (voice drops)
        manager
            .update_dimension("magnus", TrustDimension::Voice, 0.65)
            .unwrap();

        // But typing and location are good
        manager
            .update_dimension("magnus", TrustDimension::Typing, 0.95)
            .unwrap();
        manager
            .update_dimension("magnus", TrustDimension::Location, 0.98)
            .unwrap();

        let trust = manager.get_trust("magnus").unwrap();
        let policy = TrustPolicy::normal_access(); // Requires 2 of 3

        // Should PASS: typing + location (2 of 3)
        assert!(trust.meets_policy(&policy));
    }
}
