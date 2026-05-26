// Random-walk chain baseline for B4: at each step picks a random unvisited crystal.

use std::collections::HashSet;

pub fn random_chain(
    store: &pse_adapter_il::ILStore,
    all_ids: &[String],
    max_steps: usize,
    seed: u64,
) -> Vec<(String, f64, u8)> {
    // steps: (crystal_id_hex, d_score=0.0 (random has no D), qtic_class)
    const A: u64 = 6364136223846793005;
    const C: u64 = 1442695040888963407;
    let mut state = seed;
    let mut next_rand = || {
        state = state.wrapping_mul(A).wrapping_add(C);
        state
    };

    let mut visited: HashSet<String> = HashSet::new();
    let mut steps = Vec::new();
    for _ in 0..max_steps {
        let candidates: Vec<&String> = all_ids.iter().filter(|id| !visited.contains(*id)).collect();
        if candidates.is_empty() {
            break;
        }
        let idx = (next_rand() as usize) % candidates.len();
        let id = candidates[idx].clone();
        let (qtic, _stab) = store.crystal_meta(&id).unwrap_or((0, 0.5));
        steps.push((id.clone(), 0.0, qtic));
        visited.insert(id);
    }
    steps
}
