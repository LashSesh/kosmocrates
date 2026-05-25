// Cosine-only retrieval: ψ only, ρ = ω = 1.
// Uses ILStore::search() which computes raw cosine similarity.

pub fn rank(store: &pse_adapter_il::ILStore, query_vec: &[f64], top_k: usize) -> Vec<String> {
    store.search(query_vec, top_k).into_iter().map(|m| m.crystal_id_hex).collect()
}
