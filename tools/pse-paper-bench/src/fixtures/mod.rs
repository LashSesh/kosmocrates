use pse_adapter_il::ILStore;
use pse_types::{SemanticCrystal, Hash256, TopologySignature};
use std::collections::BTreeMap;
use std::path::Path;

/// Build a SemanticCrystal with controlled stability and kuramoto.
/// id_seed: any string; the first 32 bytes become the crystal_id hash.
pub fn build_crystal(id_seed: &str, stability: f64, kuramoto: f64) -> SemanticCrystal {
    let mut hash: Hash256 = [0u8; 32];
    for (i, b) in id_seed.bytes().enumerate().take(32) {
        hash[i] = b;
    }
    SemanticCrystal {
        crystal_id: hash,
        region: vec![],
        constraint_program: Default::default(),
        stability_score: stability,
        topology_signature: TopologySignature {
            kuramoto_coherence: kuramoto,
            betti_0: 1,
            euler_char: 1,
            ..TopologySignature::default()
        },
        betti_numbers: vec![1],
        evidence_chain: Default::default(),
        commit_proof: Default::default(),
        operator_versions: BTreeMap::new(),
        created_at: 1,
        free_energy: (1.0 - stability).powi(2),
        carrier_instance_idx: 0,
        scale_tag: String::new(),
        universe_id: String::new(),
        sub_crystal_ids: vec![],
        parent_crystal_ids: vec![],
        genesis_metadata: None,
        metatron_signature: None,
    }
}

/// Open (or create) an ILStore in the given directory.
pub fn open_store(dir: &Path) -> ILStore {
    ILStore::open(dir, "pse-paper-bench").expect("open ILStore")
}

/// Commit a crystal with the given question text and return the hex ID.
pub fn commit_crystal(
    store: &mut ILStore,
    crystal: &SemanticCrystal,
    question: &str,
    session: usize,
) -> String {
    store
        .commit(crystal, &[question.to_string()], session, question)
        .expect("commit crystal")
}

/// Noise-inject a text at `rate` fraction of characters.
/// Replaces random 4-grams with random alphanumeric characters.
/// Uses a seeded LCG for determinism.
pub fn noise_inject(text: &str, rate: f64, seed: u64) -> String {
    const A: u64 = 6364136223846793005;
    const C: u64 = 1442695040888963407;
    let mut state = seed;
    let mut next = || { state = state.wrapping_mul(A).wrapping_add(C); state };

    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out: Vec<char> = chars.clone();
    let n_corrupt = ((n as f64) * rate) as usize;
    for _ in 0..n_corrupt {
        let pos = (next() as usize) % n.max(1);
        let replacement = (b'a' + (next() as u8 % 26)) as char;
        out[pos] = replacement;
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn build_crystal_sets_stability() {
        let c = build_crystal("test-001", 0.85, 0.75);
        assert!((c.stability_score - 0.85).abs() < 1e-10);
    }
    #[test]
    fn noise_inject_deterministic() {
        let s = "the quick brown fox";
        let a = noise_inject(s, 0.3, 42);
        let b = noise_inject(s, 0.3, 42);
        assert_eq!(a, b);
    }
}
