# Frame Sentinel - Multi-Dimensional Trust Scoring and Relationship Management

**CRITICAL:** Single scalar trust is dangerous and attackable. This crate provides orthogonal trust dimensions and policy-based access control.

Extracted from the Frame microservices architecture.

## Problem: Scalar Trust is Exploitable

```text
Attacker with good deepfake voice:
  voice_similarity: 0.95 ✅
  typing_similarity: N/A (no data)
  location_consistency: 0.3 ❌
  → Overall trust: 0.6 (gets Verified!) 🚨

Legitimate user with new mic:
  voice_similarity: 0.65 (different mic) ❌
  typing_similarity: 0.95 ✅
  location_consistency: 0.98 ✅
  → Overall trust drops unfairly 🚨
```

## Solution: Orthogonal Trust Dimensions

Track **6 independent trust dimensions**:

1. **Identity Trust (Voice)** - Biometric voice patterns
2. **Identity Trust (Typing)** - Behavioral keystroke dynamics
3. **Identity Trust (Face)** - Facial recognition (future)
4. **Location Consistency** - Geospatial patterns
5. **Relationship Trust** - Interaction history, social graph
6. **Device Trust** - Known device fingerprints

## Quick Start

```toml
[dependencies]
sam-trust = "0.1.0"
```

## Dependency Architecture

**frame-sentinel depends on:**

```
frame-sentinel
└── frame-catalog (database, embeddings)
```

**Used by:** frame-identity (trust integration)

**Position in Frame ecosystem:**

```
frame-catalog
    └→ frame-sentinel
        └→ frame-identity
```

```rust
use sam_trust::{TrustScoreManager, TrustLevel};

// Initialize trust tracking
let manager = TrustScoreManager::new("trust.db")?;

// Record successful authentication
manager.record_successful_auth("user123")?;

// Check trust level
let trust = manager.get_trust_score("user123")?;
println!("Trust: {:?} ({:.2})", trust.level, trust.score);
```

## Progressive Trust Levels

Users must prove identity over time:

- **Unknown** (0.0): First encounter, collect baseline
- **Observed** (0.1-0.3): 3+ matching authentications
- **Verified** (0.4-0.6): 20+ auths over 2 weeks
- **Trusted** (0.7-0.9): 100+ auths over 3 months
- **InnerCircle** (1.0): Explicit approval required

## Policy-Based Access Control

Define per-operation requirements:

```rust
use sam_trust::{TrustPolicy, TrustDimension, TrustCondition, PolicyMode};

// InnerCircle operation (all conditions required)
let policy = TrustPolicy {
    mode: PolicyMode::All,
    conditions: vec![
        TrustCondition::new(TrustDimension::Voice, 0.9),
        TrustCondition::new(TrustDimension::Location, 0.8),
        TrustCondition::new(TrustDimension::Relationship, 0.7),
    ],
};

// Normal access (any 2 of 3)
let policy = TrustPolicy {
    mode: PolicyMode::AtLeast(2),
    conditions: vec![
        TrustCondition::new(TrustDimension::Voice, 0.7),
        TrustCondition::new(TrustDimension::Typing, 0.7),
        TrustCondition::new(TrustDimension::Device, 0.8),
    ],
};
```

## Relationship Graph

Track social connections with transitive inference:

```rust
use sam_trust::{RelationshipGraph, RelationType};

let mut graph = RelationshipGraph::new();

// Direct relationships
graph.add_relationship("sam", "magnus", RelationType::Creator);
graph.add_relationship("magnus", "john", RelationType::Sibling);

// Infer transitive relationships
let relationships = graph.get_all_relationships("sam", "john");
// Returns: [Uncle] (creator → sibling → uncle)
```

## Modules

- **multidimensional_trust** (419 LOC) - 6D trust system, policy engine
- **trust_scoring** (483 LOC) - Progressive trust levels, pattern matching
- **relationships** (590 LOC) - Social graph, transitive inference

## Features

- No instant trust - gradual identity proof required
- Trust decay on absence, failed auth, impossible travel
- Familiarity vs Authority distinction (prevents speed-running)
- Pattern matching: voice, typing, behavior, geospatial
- Relationship inference: grandparent, uncle, cousin, etc.

## Compatibility

- **Rust Edition**: 2021
- **MSRV**: 1.70+
- **Platforms**: All

## Dependencies

- `rusqlite` (0.31) - Trust score persistence
- `frame-catalog` - Database trait for relationship storage
- `bincode` (1.3) - Baseline voice pattern serialization

## License

MIT - See [LICENSE](LICENSE) for details.

## Author

Magnus Trent <magnus@blackfall.dev>

## Links

- **GitHub:** https://github.com/Blackfall-Labs/frame-sentinel
