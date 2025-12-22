# Changelog

## [0.1.0] - 2025-12-21

### Added
- Initial release extracted from Frame project
- **Multi-Dimensional Trust System**: 6 orthogonal trust dimensions
  - Identity Trust (Voice, Typing, Face)
  - Location Consistency
  - Relationship Trust
  - Device Trust
- **Progressive Trust Levels**: Unknown → Observed → Verified → Trusted → InnerCircle
  - 0+ auths: Unknown (0.0)
  - 3+ auths: Observed (0.1-0.3)
  - 20+ auths over 2 weeks: Verified (0.4-0.6)
  - 100+ auths over 3 months: Trusted (0.7-0.9)
  - Explicit approval: InnerCircle (1.0)
- **Policy-Based Access Control**: Define per-operation trust requirements
  - All conditions mode (AND logic)
  - Any-of mode (OR logic)
  - At-least-N mode (threshold logic)
- **Relationship Graph**: Social connection tracking with transitive inference
  - Direct relationships (parent, sibling, friend, colleague, etc.)
  - Inferred relationships (grandparent, uncle, cousin)
  - Trust propagation through social graph
- **Trust Decay**: Automatic trust reduction
  - Pattern drift detection
  - Absence penalties (30 days → -0.1)
  - Failed auth penalties (3 failures → -0.2)
  - Impossible travel detection (-0.5 immediately)

### Security Features
- No instant trust - gradual proof required
- Prevents scalar trust averaging attacks
- Familiarity vs Authority distinction (prevents speed-running)
- Pattern matching for voice, typing, behavior, geospatial
- Multi-factor trust evaluation

### Modules
- multidimensional_trust.rs (419 LOC) - 6D trust system, policy engine
- trust_scoring.rs (483 LOC) - Progressive levels, pattern matching
- relationships.rs (590 LOC) - Social graph, transitive inference

### Dependencies
- rusqlite 0.31 (trust score persistence)
- frame-catalog (Database trait for relationships)
- bincode 1.3 (baseline voice pattern serialization)
- chrono 0.4 (timestamps)

### Notes
- Extracted from [Frame](https://github.com/Blackfall-Labs/sam)
- Designed to prevent deepfake and impersonation attacks
- Production-ready for identity verification systems
