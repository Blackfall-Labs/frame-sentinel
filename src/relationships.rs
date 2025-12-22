//! Relationship graph for tracking social connections
//!
//! This module provides relationship tracking with transitive inference.
//! For example, if Magnus is SAM's creator and John is Magnus's brother,
//! then SAM can infer that John is SAM's uncle.

use sam_vector::database::{Database, DatabaseError};

pub type Result<T> = std::result::Result<T, DatabaseError>;
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

/// Relationship type between users
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationType {
    // Family relationships
    Parent,
    Child,
    Sibling,
    Spouse,

    // Extended family (inferred)
    Grandparent,
    Grandchild,
    Uncle,
    Aunt,
    Nephew,
    Niece,
    Cousin,

    // Social relationships
    Friend,
    Colleague,
    Mentor,
    Student,

    // System relationships
    Creator,       // Person who built SAM
    Administrator, // Person who manages SAM
    User,          // Regular user

    // Custom
    Custom(String),
}

impl RelationType {
    /// Convert relationship type to string for storage
    pub fn to_string(&self) -> String {
        match self {
            RelationType::Parent => "parent".to_string(),
            RelationType::Child => "child".to_string(),
            RelationType::Sibling => "sibling".to_string(),
            RelationType::Spouse => "spouse".to_string(),
            RelationType::Grandparent => "grandparent".to_string(),
            RelationType::Grandchild => "grandchild".to_string(),
            RelationType::Uncle => "uncle".to_string(),
            RelationType::Aunt => "aunt".to_string(),
            RelationType::Nephew => "nephew".to_string(),
            RelationType::Niece => "niece".to_string(),
            RelationType::Cousin => "cousin".to_string(),
            RelationType::Friend => "friend".to_string(),
            RelationType::Colleague => "colleague".to_string(),
            RelationType::Mentor => "mentor".to_string(),
            RelationType::Student => "student".to_string(),
            RelationType::Creator => "creator".to_string(),
            RelationType::Administrator => "administrator".to_string(),
            RelationType::User => "user".to_string(),
            RelationType::Custom(s) => format!("custom:{}", s),
        }
    }

    /// Parse relationship type from string
    pub fn from_string(s: &str) -> Self {
        match s {
            "parent" => RelationType::Parent,
            "child" => RelationType::Child,
            "sibling" => RelationType::Sibling,
            "spouse" => RelationType::Spouse,
            "grandparent" => RelationType::Grandparent,
            "grandchild" => RelationType::Grandchild,
            "uncle" => RelationType::Uncle,
            "aunt" => RelationType::Aunt,
            "nephew" => RelationType::Nephew,
            "niece" => RelationType::Niece,
            "cousin" => RelationType::Cousin,
            "friend" => RelationType::Friend,
            "colleague" => RelationType::Colleague,
            "mentor" => RelationType::Mentor,
            "student" => RelationType::Student,
            "creator" => RelationType::Creator,
            "administrator" => RelationType::Administrator,
            "user" => RelationType::User,
            s if s.starts_with("custom:") => RelationType::Custom(s[7..].to_string()),
            _ => RelationType::Custom(s.to_string()),
        }
    }

    /// Get the inverse relationship
    pub fn inverse(&self) -> Option<RelationType> {
        match self {
            RelationType::Parent => Some(RelationType::Child),
            RelationType::Child => Some(RelationType::Parent),
            RelationType::Grandparent => Some(RelationType::Grandchild),
            RelationType::Grandchild => Some(RelationType::Grandparent),
            RelationType::Uncle => Some(RelationType::Nephew), // Simplified
            RelationType::Aunt => Some(RelationType::Niece),   // Simplified
            RelationType::Nephew => Some(RelationType::Uncle), // Simplified
            RelationType::Niece => Some(RelationType::Aunt),   // Simplified
            RelationType::Mentor => Some(RelationType::Student),
            RelationType::Student => Some(RelationType::Mentor),
            RelationType::Sibling
            | RelationType::Spouse
            | RelationType::Friend
            | RelationType::Colleague
            | RelationType::Cousin => Some(self.clone()),
            _ => None,
        }
    }
}

/// A relationship between two users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from_user_id: String,
    pub to_user_id: String,
    pub relationship_type: RelationType,
    pub confidence: f32, // 0.0 (uncertain) to 1.0 (certain)
    pub source: RelationshipSource,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<String>,
}

/// Source of relationship information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipSource {
    Explicit,      // User explicitly stated
    Inferred,      // Inferred through transitive rules
    Configuration, // From config file
}

impl RelationshipSource {
    pub fn to_string(&self) -> String {
        match self {
            RelationshipSource::Explicit => "explicit".to_string(),
            RelationshipSource::Inferred => "inferred".to_string(),
            RelationshipSource::Configuration => "configuration".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "explicit" => RelationshipSource::Explicit,
            "inferred" => RelationshipSource::Inferred,
            "configuration" => RelationshipSource::Configuration,
            _ => RelationshipSource::Explicit,
        }
    }
}

/// Relationship graph with transitive inference
pub struct RelationshipGraph {
    db: Arc<Mutex<rusqlite::Connection>>,
}

impl RelationshipGraph {
    /// Create a new relationship graph
    pub fn new(database: &Database) -> Self {
        RelationshipGraph {
            db: database.conn(),
        }
    }

    /// Initialize database schema for relationships
    pub fn initialize_schema(&self) -> Result<()> {
        let conn = self.db.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS relationships (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                from_user_id TEXT NOT NULL,
                to_user_id TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                confidence REAL NOT NULL,
                source TEXT NOT NULL,
                created_at TEXT NOT NULL,
                metadata TEXT,
                UNIQUE(from_user_id, to_user_id, relationship_type)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_relationships_from
             ON relationships(from_user_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_relationships_to
             ON relationships(to_user_id)",
            [],
        )?;

        Ok(())
    }

    /// Add a relationship between two users
    pub fn add_relationship(
        &self,
        from_user_id: String,
        to_user_id: String,
        relationship_type: RelationType,
        confidence: f32,
        source: RelationshipSource,
        metadata: Option<String>,
    ) -> Result<()> {
        let conn = self.db.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO relationships
             (from_user_id, to_user_id, relationship_type, confidence, source, created_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                from_user_id,
                to_user_id,
                relationship_type.to_string(),
                confidence,
                source.to_string(),
                Utc::now().to_rfc3339(),
                metadata,
            ],
        )?;

        Ok(())
    }

    /// Get all direct relationships for a user
    pub fn get_relationships(&self, user_id: &str) -> Result<Vec<Relationship>> {
        let conn = self.db.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT from_user_id, to_user_id, relationship_type, confidence, source, created_at, metadata
             FROM relationships
             WHERE from_user_id = ?1"
        )?;

        let relationships = stmt
            .query_map(params![user_id], |row| {
                Ok(Relationship {
                    from_user_id: row.get(0)?,
                    to_user_id: row.get(1)?,
                    relationship_type: RelationType::from_string(&row.get::<_, String>(2)?),
                    confidence: row.get(3)?,
                    source: RelationshipSource::from_string(&row.get::<_, String>(4)?),
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Utc),
                    metadata: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(relationships)
    }

    /// Get relationship between two specific users
    pub fn get_relationship_between(
        &self,
        from_user_id: &str,
        to_user_id: &str,
    ) -> Result<Option<Relationship>> {
        let conn = self.db.lock().unwrap();

        let result = conn.query_row(
            "SELECT from_user_id, to_user_id, relationship_type, confidence, source, created_at, metadata
             FROM relationships
             WHERE from_user_id = ?1 AND to_user_id = ?2
             ORDER BY confidence DESC
             LIMIT 1",
            params![from_user_id, to_user_id],
            |row| {
                Ok(Relationship {
                    from_user_id: row.get(0)?,
                    to_user_id: row.get(1)?,
                    relationship_type: RelationType::from_string(&row.get::<_, String>(2)?),
                    confidence: row.get(3)?,
                    source: RelationshipSource::from_string(&row.get::<_, String>(4)?),
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Utc),
                    metadata: row.get(6)?,
                })
            },
        ).optional()?;

        Ok(result)
    }

    /// Infer new relationships using transitive rules
    ///
    /// Examples of inference:
    /// - If A is parent of B, and B is parent of C, then A is grandparent of C
    /// - If A is creator and B is sibling of A, then B is uncle/aunt to SAM
    pub fn infer_relationships(&self, sam_user_id: &str) -> Result<Vec<Relationship>> {
        let mut inferred = Vec::new();

        // Get all existing relationships
        let all_rels = self.get_all_relationships()?;

        // Build adjacency map
        let mut graph: HashMap<String, Vec<Relationship>> = HashMap::new();
        for rel in &all_rels {
            graph
                .entry(rel.from_user_id.clone())
                .or_insert_with(Vec::new)
                .push(rel.clone());
        }

        // Inference Rule 1: Creator's sibling → Uncle/Aunt
        // If X is creator of SAM, and Y is sibling of X, then Y is uncle/aunt of SAM
        for rel in &all_rels {
            if rel.relationship_type == RelationType::Creator {
                let creator_id = &rel.from_user_id;

                // Find siblings of creator
                if let Some(creator_rels) = graph.get(creator_id) {
                    for creator_rel in creator_rels {
                        if creator_rel.relationship_type == RelationType::Sibling {
                            let sibling_id = &creator_rel.to_user_id;

                            // Check if this relationship already exists
                            if self
                                .get_relationship_between(sibling_id, sam_user_id)?
                                .is_none()
                            {
                                inferred.push(Relationship {
                                    from_user_id: sibling_id.clone(),
                                    to_user_id: sam_user_id.to_string(),
                                    relationship_type: RelationType::Uncle, // Simplified
                                    confidence: rel.confidence * creator_rel.confidence * 0.9,
                                    source: RelationshipSource::Inferred,
                                    created_at: Utc::now(),
                                    metadata: Some(format!(
                                        "Inferred: {} is sibling of creator {}",
                                        sibling_id, creator_id
                                    )),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Inference Rule 2: Parent of parent → Grandparent
        // If A is parent of B, and B is parent of C, then A is grandparent of C
        for rel in &all_rels {
            if rel.relationship_type == RelationType::Parent {
                let parent_id = &rel.from_user_id; // A
                let middle_id = &rel.to_user_id; // B

                // Find where B is parent of someone else
                for other_rel in &all_rels {
                    if other_rel.relationship_type == RelationType::Parent
                        && &other_rel.from_user_id == middle_id
                    {
                        let grandchild_id = &other_rel.to_user_id; // C

                        if self
                            .get_relationship_between(parent_id, grandchild_id)?
                            .is_none()
                        {
                            inferred.push(Relationship {
                                from_user_id: parent_id.clone(),
                                to_user_id: grandchild_id.clone(),
                                relationship_type: RelationType::Grandparent,
                                confidence: rel.confidence * other_rel.confidence * 0.95,
                                source: RelationshipSource::Inferred,
                                created_at: Utc::now(),
                                metadata: Some(format!(
                                    "Inferred: {} is parent of {}, {} is parent of {}",
                                    parent_id, middle_id, middle_id, grandchild_id
                                )),
                            });
                        }
                    }
                }
            }
        }

        // Inference Rule 3: Parent's sibling → Uncle/Aunt
        for rel in &all_rels {
            if rel.relationship_type == RelationType::Parent {
                let parent_id = &rel.from_user_id;
                let child_id = &rel.to_user_id;

                // Find siblings of parent
                if let Some(parent_rels) = graph.get(parent_id) {
                    for parent_rel in parent_rels {
                        if parent_rel.relationship_type == RelationType::Sibling {
                            let sibling_id = &parent_rel.to_user_id;

                            if self
                                .get_relationship_between(sibling_id, child_id)?
                                .is_none()
                            {
                                inferred.push(Relationship {
                                    from_user_id: sibling_id.clone(),
                                    to_user_id: child_id.clone(),
                                    relationship_type: RelationType::Uncle, // Simplified
                                    confidence: rel.confidence * parent_rel.confidence * 0.9,
                                    source: RelationshipSource::Inferred,
                                    created_at: Utc::now(),
                                    metadata: Some(format!(
                                        "Inferred: {} is sibling of parent {}",
                                        sibling_id, parent_id
                                    )),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(inferred)
    }

    /// Store inferred relationships in database
    pub fn store_inferred_relationships(&self, relationships: Vec<Relationship>) -> Result<usize> {
        let mut count = 0;

        for rel in relationships {
            self.add_relationship(
                rel.from_user_id,
                rel.to_user_id,
                rel.relationship_type,
                rel.confidence,
                rel.source,
                rel.metadata,
            )?;
            count += 1;
        }

        Ok(count)
    }

    /// Get all relationships (for inference)
    fn get_all_relationships(&self) -> Result<Vec<Relationship>> {
        let conn = self.db.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT from_user_id, to_user_id, relationship_type, confidence, source, created_at, metadata
             FROM relationships"
        )?;

        let relationships = stmt
            .query_map([], |row| {
                Ok(Relationship {
                    from_user_id: row.get(0)?,
                    to_user_id: row.get(1)?,
                    relationship_type: RelationType::from_string(&row.get::<_, String>(2)?),
                    confidence: row.get(3)?,
                    source: RelationshipSource::from_string(&row.get::<_, String>(4)?),
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Utc),
                    metadata: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(relationships)
    }

    /// Get human-readable description of relationship
    pub fn describe_relationship(
        &self,
        from_user_id: &str,
        to_user_id: &str,
    ) -> Result<Option<String>> {
        if let Some(rel) = self.get_relationship_between(from_user_id, to_user_id)? {
            let desc = match rel.relationship_type {
                RelationType::Creator => "creator".to_string(),
                RelationType::Parent => "parent".to_string(),
                RelationType::Child => "child".to_string(),
                RelationType::Sibling => "sibling".to_string(),
                RelationType::Uncle => "uncle".to_string(),
                RelationType::Aunt => "aunt".to_string(),
                RelationType::Friend => "friend".to_string(),
                RelationType::Colleague => "colleague".to_string(),
                _ => rel.relationship_type.to_string(),
            };

            let confidence_desc = if rel.confidence >= 0.9 {
                ""
            } else if rel.confidence >= 0.7 {
                " (likely)"
            } else {
                " (possibly)"
            };

            Ok(Some(format!("{}{}", desc, confidence_desc)))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_type_conversion() {
        let rel = RelationType::Uncle;
        let s = rel.to_string();
        assert_eq!(s, "uncle");

        let parsed = RelationType::from_string(&s);
        assert_eq!(parsed, RelationType::Uncle);
    }

    #[test]
    fn test_relationship_inverse() {
        assert_eq!(RelationType::Parent.inverse(), Some(RelationType::Child));
        assert_eq!(RelationType::Child.inverse(), Some(RelationType::Parent));
        assert_eq!(RelationType::Sibling.inverse(), Some(RelationType::Sibling));
    }

    #[test]
    fn test_add_and_get_relationship() {
        let db = Database::new(":memory:").unwrap();
        let graph = RelationshipGraph::new(&db);
        graph.initialize_schema().unwrap();

        graph
            .add_relationship(
                "user1".to_string(),
                "user2".to_string(),
                RelationType::Friend,
                1.0,
                RelationshipSource::Explicit,
                None,
            )
            .unwrap();

        let rels = graph.get_relationships("user1").unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].to_user_id, "user2");
        assert_eq!(rels[0].relationship_type, RelationType::Friend);
    }

    #[test]
    fn test_infer_uncle_from_creator_sibling() {
        let db = Database::new(":memory:").unwrap();
        let graph = RelationshipGraph::new(&db);
        graph.initialize_schema().unwrap();

        // Magnus is creator
        graph
            .add_relationship(
                "magnus".to_string(),
                "sam".to_string(),
                RelationType::Creator,
                1.0,
                RelationshipSource::Explicit,
                None,
            )
            .unwrap();

        // John is Magnus's sibling
        graph
            .add_relationship(
                "magnus".to_string(),
                "john".to_string(),
                RelationType::Sibling,
                1.0,
                RelationshipSource::Explicit,
                None,
            )
            .unwrap();

        // Infer relationships
        let inferred = graph.infer_relationships("sam").unwrap();

        // Should infer that John is SAM's uncle
        assert_eq!(inferred.len(), 1);
        assert_eq!(inferred[0].from_user_id, "john");
        assert_eq!(inferred[0].to_user_id, "sam");
        assert_eq!(inferred[0].relationship_type, RelationType::Uncle);
        assert!(inferred[0].confidence > 0.8);

        // Store and verify
        graph.store_inferred_relationships(inferred).unwrap();

        let desc = graph.describe_relationship("john", "sam").unwrap();
        assert!(desc.is_some());
        assert!(desc.unwrap().contains("uncle"));
    }

    #[test]
    fn test_infer_grandparent() {
        let db = Database::new(":memory:").unwrap();
        let graph = RelationshipGraph::new(&db);
        graph.initialize_schema().unwrap();

        // A is parent of B
        graph
            .add_relationship(
                "a".to_string(),
                "b".to_string(),
                RelationType::Parent,
                1.0,
                RelationshipSource::Explicit,
                None,
            )
            .unwrap();

        // B is parent of C
        graph
            .add_relationship(
                "b".to_string(),
                "c".to_string(),
                RelationType::Parent,
                1.0,
                RelationshipSource::Explicit,
                None,
            )
            .unwrap();

        // Infer relationships
        let inferred = graph.infer_relationships("sam").unwrap();

        // Should infer that A is grandparent of C
        let grandparent_rel = inferred
            .iter()
            .find(|r| r.relationship_type == RelationType::Grandparent);

        assert!(grandparent_rel.is_some());
        let rel = grandparent_rel.unwrap();
        assert_eq!(rel.from_user_id, "a");
        assert_eq!(rel.to_user_id, "c");
        assert!(rel.confidence > 0.9);
    }
}
