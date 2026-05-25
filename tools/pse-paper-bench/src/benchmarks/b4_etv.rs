use crate::{datasets, fixtures, baselines, metrics::chain_metrics, stats};
use pse_reasoning::{guide, ThunderboltConfig};
use serde::Serialize;

#[derive(Serialize, Clone, Default)]
pub struct B4Results {
    pub pse_mean_d: f64, pub pse_peak_d: f64, pub pse_avg_length: f64, pub pse_diversity: f64,
    pub greedy_mean_d: f64, pub greedy_peak_d: f64, pub greedy_avg_length: f64,
    pub random_mean_d: f64, pub random_avg_length: f64,
    pub n_queries: usize,
}

pub fn run() -> B4Results {
    let ds = datasets::reasoning_dataset();
    let dir = tempfile::tempdir().expect("tmpdir");
    let mut store = fixtures::open_store(dir.path());

    let mut all_ids = vec![];
    for (i, doc) in ds.corpus.iter().enumerate() {
        let crystal = fixtures::build_crystal(&format!("b4-{}", doc.id), doc.stability, doc.kuramoto);
        let _ = fixtures::commit_crystal(&mut store, &crystal, &doc.text, i + 1);
        all_ids.push(hex_encode(&crystal.crystal_id));
    }

    let config = ThunderboltConfig { max_steps: 6, min_d_threshold: 0.001, top_k_per_step: 32 };

    let mut pse_mean_ds = vec![];
    let mut pse_peak_ds = vec![];
    let mut pse_lengths = vec![];
    let mut pse_diversities = vec![];
    let mut greedy_mean_ds = vec![];
    let mut greedy_peak_ds = vec![];
    let mut greedy_lengths = vec![];
    let mut rnd_mean_ds = vec![];
    let mut rnd_lengths = vec![];

    use pse_adapter_il::text_to_vector8;

    for (qi, query) in ds.queries.iter().enumerate() {
        // PSE ETV
        let chain = guide(&query.text, &store, &config);
        let steps: Vec<(String, f64, u8)> = chain.steps.iter()
            .map(|s| (s.crystal_id_hex.clone(), s.d_score, s.qtic_class)).collect();
        let cm = chain_metrics::compute_chain_metrics(&steps);
        pse_mean_ds.push(cm.mean_d);
        pse_peak_ds.push(cm.peak_d);
        pse_lengths.push(cm.chain_length as f64);
        pse_diversities.push(cm.crystal_diversity as f64);

        // Greedy cosine chain
        let qvec = text_to_vector8(&query.text);
        let mut visited = std::collections::HashSet::new();
        let mut greedy_steps = vec![];
        let mut cur_vec = qvec.clone();
        for _ in 0..config.max_steps {
            let ranked: Vec<_> = store.search(&cur_vec, 32).into_iter()
                .filter(|m| !visited.contains(&m.crystal_id_hex))
                .collect();
            if ranked.is_empty() { break; }
            let best = &ranked[0];
            let (qtic, _) = store.crystal_meta(&best.crystal_id_hex).unwrap_or((0, 0.5));
            greedy_steps.push((best.crystal_id_hex.clone(), best.score, qtic));
            visited.insert(best.crystal_id_hex.clone());
            if let Some(v) = store.crystal_vector8(&best.crystal_id_hex) { cur_vec = v; }
            else { break; }
        }
        let gcm = chain_metrics::compute_chain_metrics(&greedy_steps);
        greedy_mean_ds.push(gcm.mean_d);
        greedy_peak_ds.push(gcm.peak_d);
        greedy_lengths.push(gcm.chain_length as f64);

        // Random walk
        let seed = stats::label_seed(&format!("b4-rnd-{qi}"));
        let rnd = baselines::random_walk::random_chain(&store, &all_ids, config.max_steps, seed);
        let rcm = chain_metrics::compute_chain_metrics(&rnd);
        rnd_mean_ds.push(rcm.mean_d);
        rnd_lengths.push(rcm.chain_length as f64);
    }

    let mean_of = |v: &[f64]| if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 };

    B4Results {
        pse_mean_d: mean_of(&pse_mean_ds), pse_peak_d: mean_of(&pse_peak_ds),
        pse_avg_length: mean_of(&pse_lengths), pse_diversity: mean_of(&pse_diversities),
        greedy_mean_d: mean_of(&greedy_mean_ds), greedy_peak_d: mean_of(&greedy_peak_ds),
        greedy_avg_length: mean_of(&greedy_lengths),
        random_mean_d: mean_of(&rnd_mean_ds), random_avg_length: mean_of(&rnd_lengths),
        n_queries: ds.queries.len(),
    }
}

fn hex_encode(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }
