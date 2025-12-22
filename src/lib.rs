//! # Frame Sentinel - Multi-Dimensional Trust Scoring and Relationship Management
//!
//! **CRITICAL:** Single scalar trust is dangerous and attackable. This crate provides
//! orthogonal trust dimensions and policy-based access control.
//!
//! ## Features
//!
//! ### 🛡️ Multi-Dimensional Trust
//!
//! Track independent trust dimensions to prevent single-point-of-failure attacks:
//!
//! - **Identity Trust (Voice)**: Biometric voice pattern matching
//! - **Identity Trust (Typing)**: Behavioral keystroke dynamics
//! - **Identity Trust (Face)**: Facial recognition (future)
//! - **Location Consistency**: Geospatial pattern verification
//! - **Relationship Trust**: Interaction history and social connections
//! - **Device Trust**: Known device fingerprint matching
//!
//! ### 📊 Progressive Trust Levels
//!
//! Users must prove identity over time:
//!
//! - **Unknown** (0.0): First encounter
//! - **Observed** (0.1-0.3): 3+ matching authentications
//! - **Verified** (0.4-0.6): 20+ auths over 2 weeks
//! - **Trusted** (0.7-0.9): 100+ auths over 3 months
//! - **InnerCircle** (1.0): Explicit approval required
//!
//! ### 🔗 Relationship Graph
//!
//! Track social connections with transitive inference:
//!
//! - Direct relationships (parent, sibling, friend, colleague)
//! - Inferred relationships (grandparent, uncle, cousin)
//! - Trust propagation through social graph
//!
//! ## Usage
//!
//! ```rust,no_run
//! use sam_trust::{TrustScoreStore, TrustLevel};
//! use sam_trust::{RelationshipGraph, RelationType};
//!
//! // Track trust progression
//! let trust_store = TrustScoreStore::new("trust.db").unwrap();
//! trust_store.record_successful_auth("user123").unwrap();
//!
//! let trust = trust_store.get_trust_score("user123").unwrap();
//! println!("Trust level: {:?} ({:.2})", trust.level, trust.score);
//!
//! // Manage relationships
//! let mut graph = RelationshipGraph::new();
//! graph.add_relationship("sam", "magnus", RelationType::Creator);
//! graph.add_relationship("magnus", "john", RelationType::Sibling);
//!
//! // Infer transitive relationships
//! let relationships = graph.get_all_relationships("sam", "john");
//! // Returns: [Uncle] (inferred from creator → sibling chain)
//! ```

pub mod multidimensional_trust;
pub mod trust_scoring;
pub mod relationships;

// Re-export main types
pub use multidimensional_trust::{
    MultiDimensionalTrustManager, TrustDimensions, TrustDimension,
    TrustPolicy, PolicyMode, TrustCondition, TrustError,
};
pub use trust_scoring::{
    TrustScoreManager, TrustScore, TrustLevel, TypingPattern, AuthenticationAttempt,
    TrustScoreError,
    TRUST_UNKNOWN, TRUST_OBSERVED, TRUST_VERIFIED, TRUST_TRUSTED, TRUST_INNER_CIRCLE,
};
pub use relationships::{RelationshipGraph, RelationType, Relationship, RelationshipSource};
