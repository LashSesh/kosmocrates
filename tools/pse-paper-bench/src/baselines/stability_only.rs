// Stability-only ranking: rank by ρ (stability_score) descending.
// Uses crystal_meta() to retrieve (qtic_class, stability_score) per crystal.

pub fn rank(store: &pse_adapter_il::ILStore, crystal_ids: &[String]) -> Vec<String> {
    let mut scored: Vec<(String, f64)> = crystal_ids.iter().filter_map(|id| {
        store.crystal_meta(id).map(|(_, stab)| (id.clone(), stab))
    }).collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.into_iter().map(|(id, _)| id).collect()
}
