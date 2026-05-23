# MEF Knowledge Engine Extension - Quick Start Guide

## Overview

The MEF Knowledge Engine extension adds knowledge derivation, vector memory indexing, and deterministic routing capabilities to the MEF-Core system. This guide provides quick-start instructions and usage examples.

## Features

🔹 **Knowledge Processing**
- Canonical JSON serialization (deterministic, stable)
- Content-addressed knowledge IDs via SHA256
- HD-style seed derivation (BIP-39 compliant)
- 8D vector construction from 5D spiral + 3D spectral features

🔹 **Vector Memory**
- Pluggable backend system
- In-memory backend (included)
- Support for FAISS/HNSW (future)
- L2 distance search

🔹 **S7 Routing**
- 5040 route permutation space
- Deterministic route selection
- Mesh metric computation
- In-process and service modes

🔹 **Gate Evaluation**
- FIRE/HOLD decision logic
- Path invariance, alignment, Lyapunov metrics
- PoR validation

## Installation

Add the extension modules to your `Cargo.toml`:

```toml
[dependencies]
mef-schemas = { path = "../mef-schemas" }
mef-knowledge = { path = "../mef-knowledge" }
mef-memory = { path = "../mef-memory" }
mef-router = { path = "../mef-router" }
```

## Quick Start

### 1. Canonical JSON Serialization

Create deterministic JSON representations:

```rust
use mef_knowledge::canonical_json;
use serde_json::json;

let data = json!({
    "zebra": 1.123456789,
    "apple": 2.0,
    "monkey": 3.333333333
});

let canonical = canonical_json(&data)?;
// Keys sorted: apple, monkey, zebra
// Floats rounded to 6 decimals
// Same input always produces same output
```

### 2. Content-Addressed Knowledge IDs

Generate content-addressed IDs:

```rust
use mef_knowledge::compute_mef_id;

let mef_id = compute_mef_id("tic_001", "route_001", "MEF/domain/stage/0001")?;
// Returns: "mef_a1b2c3d4e5f6..."
// SHA256-based, deterministic
```

### 3. Seed Derivation

Derive child seeds using HD-style derivation:

```rust
use mef_knowledge::derive_seed;

let root_seed = b"your_secure_root_seed_here";
let path = "MEF/domain/stage/0001";

let derived = derive_seed(root_seed, path)?;
// HMAC-SHA256(root_seed, path)
// Root seed never persisted!
```

### 4. 8D Vector Construction

Build normalized 8D vectors:

```rust
use mef_knowledge::{Vector8Builder, Vector8Config};

let builder = Vector8Builder::default();
let x5 = vec![0.1, 0.2, 0.3, 0.4, 0.5];  // 5D spiral
let sigma = (0.3, 0.3, 0.4);              // (ψ, ρ, ω)

let z_hat = builder.build(&x5, sigma)?;
assert_eq!(z_hat.len(), 8);
// z_hat is normalized: ||z_hat||₂ = 1
```

### 5. Memory Storage and Search

Store and search vectors:

```rust
use mef_memory::{MemoryStore, MemoryItem, SpectralSignature};

let mut store = MemoryStore::in_memory();

// Create a normalized 8D vector
let val = 1.0 / (8.0_f64).sqrt();
let vector = vec![val; 8];
let spectral = SpectralSignature {
    psi: 0.3,
    rho: 0.3,
    omega: 0.4,
};

let item = MemoryItem::new(
    "mem_001".to_string(),
    vector.clone(),
    spectral,
    None,
)?;

// Store
store.store(item)?;

// Search for similar vectors (k=5)
let results = store.search(&vector, 5)?;
for result in results {
    println!("ID: {}, Distance: {}", result.item.id, result.distance);
}
```

### 6. Route Selection

Select routes deterministically:

```rust
use mef_router::select_route;
use std::collections::HashMap;

let mut metrics = HashMap::new();
metrics.insert("betti".to_string(), 2.0);
metrics.insert("lambda_gap".to_string(), 0.5);
metrics.insert("persistence".to_string(), 0.3);

let route = select_route("seed123", &metrics)?;
println!("Route ID: {}", route.route_id);
println!("Permutation: {:?}", route.permutation);
println!("Mesh Score: {}", route.mesh_score);

// Same seed + metrics → same route (deterministic)
```

### 7. Gate Evaluation

Evaluate gate conditions:

```rust
use mef_schemas::MerkabaGateEvent;

let event = MerkabaGateEvent::new(
    "event_001".to_string(),
    "mef_001".to_string(),
    0.01,  // path_invariance (ΔPI)
    0.8,   // alignment (Φ)
    -0.1,  // lyapunov_delta (ΔV)
    true,  // por_valid
    0.05,  // epsilon threshold
    0.7,   // phi threshold
);

if event.decision == GateDecision::FIRE {
    println!("Gate FIRED - knowledge propagates");
} else {
    println!("Gate HELD - knowledge blocked");
}
```

## Complete Example

Here's a complete example combining all components:

```rust
use mef_schemas::{RouteSpec, MemoryItem, KnowledgeObject, MerkabaGateEvent, SpectralSignature};
use mef_knowledge::{canonical_json, compute_mef_id, derive_seed, Vector8Builder};
use mef_memory::MemoryStore;
use mef_router::select_route;
use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    // 1. Derive seed
    let root_seed = b"secure_root_seed";
    let seed_path = "MEF/domain/stage/0001";
    let derived = derive_seed(root_seed, seed_path)?;
    
    // 2. Build 8D vector
    let builder = Vector8Builder::default();
    let x5 = vec![0.1, 0.2, 0.3, 0.4, 0.5];
    let sigma = (0.3, 0.3, 0.4);
    let z_hat = builder.build(&x5, sigma)?;
    
    // 3. Select route
    let mut metrics = HashMap::new();
    metrics.insert("betti".to_string(), 2.0);
    metrics.insert("lambda_gap".to_string(), 0.5);
    metrics.insert("persistence".to_string(), 0.3);
    let route = select_route("seed123", &metrics)?;
    
    // 4. Compute knowledge ID
    let mef_id = compute_mef_id("tic_001", &route.route_id, seed_path)?;
    
    // 5. Create knowledge object
    let knowledge = KnowledgeObject::new(
        mef_id.clone(),
        "tic_001".to_string(),
        route.route_id.clone(),
        seed_path.to_string(),
        derived,
        None,
    );
    
    // 6. Store in memory
    let mut store = MemoryStore::in_memory();
    let spectral = SpectralSignature {
        psi: sigma.0,
        rho: sigma.1,
        omega: sigma.2,
    };
    let mem_item = MemoryItem::new(
        mef_id.clone(),
        z_hat,
        spectral,
        None,
    )?;
    store.store(mem_item)?;
    
    // 7. Evaluate gate
    let gate_event = MerkabaGateEvent::new(
        "event_001".to_string(),
        mef_id,
        0.01,  // path_invariance
        0.8,   // alignment
        -0.1,  // lyapunov_delta
        true,  // por_valid
        0.05,  // epsilon
        0.7,   // phi
    );
    
    println!("Knowledge ID: {}", knowledge.mef_id);
    println!("Route: {:?}", route.permutation);
    println!("Gate Decision: {:?}", gate_event.decision);
    
    Ok(())
}
```

## Configuration (Phase 2)

Configuration will be loaded from YAML:

```yaml
mef:
  extension:
    knowledge:
      enabled: true
      inference:
        threshold: 0.5
    memory:
      enabled: true
      backend: inmemory
    router:
      mode: inproc
```

## Testing

Run tests for all extension modules:

```bash
# Test all extension modules
cargo test -p mef-schemas -p mef-knowledge -p mef-memory -p mef-router

# Test specific module
cargo test -p mef-knowledge

# Test with output
cargo test -p mef-router -- --nocapture
```

## Benchmarking (Phase 2)

Performance benchmarks will be available:

```bash
cargo bench -p mef-benchmarks -- extension
```

## Feature Flags

Control which backends are compiled:

```toml
[features]
default = ["inmemory"]
inmemory = []
faiss = []
hnsw = []
```

Build with FAISS support (future):

```bash
cargo build --features faiss
```

## API Integration (Phase 2)

REST API endpoints will be available:

```bash
# Derive knowledge
POST /api/v1/knowledge/derive
{
  "tic_id": "tic_001",
  "route_id": "route_001",
  "seed_path": "MEF/domain/stage/0001"
}

# Search memory
POST /api/v1/memory/search
{
  "query": [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
  "k": 5
}

# Select route
POST /api/v1/router/select
{
  "seed": "seed123",
  "metrics": {
    "betti": 2.0,
    "lambda_gap": 0.5,
    "persistence": 0.3
  }
}
```

## Error Handling

All modules use Result types for error handling:

```rust
use mef_knowledge::KnowledgeError;

match derive_seed(root_seed, path) {
    Ok(derived) => println!("Success: {} bytes", derived.len()),
    Err(KnowledgeError::SeedDerivation(msg)) => eprintln!("Error: {}", msg),
    Err(e) => eprintln!("Unexpected error: {}", e),
}
```

## Security Best Practices

1. **Never log root seeds**: Only log derived seeds and paths
2. **Use secure storage**: Store root seeds in secure enclaves
3. **Validate inputs**: All schemas validate inputs on construction
4. **Content addressing**: Verify knowledge objects via their content-addressed IDs

## Performance Tips

1. **Cache S7 permutations**: Generate once, reuse for all selections
2. **Use batch operations**: Store multiple memory items in batches
3. **Tune vector backends**: Choose appropriate backend for dataset size
4. **Monitor memory usage**: In-memory backend scales linearly with dataset

## Troubleshooting

### Vector not normalized

```rust
// Error: Vector not normalized: ||z|| = 2.828427
// Solution: Ensure input vectors are normalized
let norm: f64 = vec.iter().map(|x| x * x).sum::<f64>().sqrt();
let normalized: Vec<f64> = vec.iter().map(|x| x / norm).collect();
```

### Invalid permutation

```rust
// Error: Invalid permutation index: 7
// Solution: Permutation indices must be in range [0..7)
let valid = vec![0, 1, 2, 3, 4, 5, 6];
```

### Missing metrics

```rust
// Error: Missing 'lambda_gap' metric
// Solution: Provide all required metrics
metrics.insert("betti".to_string(), 2.0);
metrics.insert("lambda_gap".to_string(), 0.5);  // Required
metrics.insert("persistence".to_string(), 0.3);
```

## Next Steps

1. **Phase 2 Integration**: Wire scaffold to core modules
2. **API Routes**: Add HTTP endpoints for extension functionality
3. **Vector Backends**: Implement FAISS and HNSW backends
4. **Configuration**: Implement YAML config loading
5. **Monitoring**: Add metrics and observability

## Resources

- [ARCHITECTURE_EXTENSION.md](ARCHITECTURE_EXTENSION.md) - Detailed architecture guide
- [EXTENSION_INTEGRATION.md](EXTENSION_INTEGRATION.md) - Integration instructions
- [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md) - Implementation summary
- [SPEC-006](Infinity-Ledger_Expansion_1-4.pdf) - Original specification

## Support

For issues or questions:
1. Check the troubleshooting section
2. Review the architecture documentation
3. Examine test cases for usage examples
4. Open an issue on GitHub

## License

MIT License - See LICENSE file for details
