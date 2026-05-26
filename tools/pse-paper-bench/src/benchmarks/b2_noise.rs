use crate::{baselines, datasets, fixtures, metrics::ir_metrics, stats};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, Clone, Default)]
pub struct NoiseLevel {
    pub rate: f64,
    pub pse_hit3: f64,
    pub bm25_hit3: f64,
    pub pse_degradation: f64,
    pub bm25_degradation: f64,
}

#[derive(Serialize, Clone, Default)]
pub struct B2Results {
    pub levels: Vec<NoiseLevel>,
    pub baseline_hit3: f64, // clean (rate=0)
}

pub fn run() -> B2Results {
    let ds = datasets::retrieval_dataset();
    let rates = [0.0f64, 0.1, 0.3, 0.5];
    let mut levels = Vec::new();
    let mut baseline_hit3 = 0.0;

    use pse_adapter_il::text_to_vector8;

    for rate in rates {
        let dir = tempfile::tempdir().expect("tmpdir");
        let mut store = fixtures::open_store(dir.path());
        let noise_seed = stats::label_seed(&format!("b2-noise-{}", (rate * 1000.0) as u32));

        let doc_texts: Vec<String> = ds
            .documents
            .iter()
            .map(|d| {
                let text = format!("{} {}", d.title, d.body);
                fixtures::noise_inject(&text, rate, noise_seed)
            })
            .collect();
        let bm25 = baselines::bm25::Bm25Index::build(&doc_texts);

        let mut doc_id_map: HashMap<String, String> = HashMap::new();
        for (i, doc) in ds.documents.iter().enumerate() {
            let crystal = fixtures::build_crystal(
                &format!("b2-{rate:.2}-{}", doc.id),
                doc.stability,
                doc.kuramoto,
            );
            let question = &doc_texts[i];
            let _ = fixtures::commit_crystal(&mut store, &crystal, question, i + 1);
            doc_id_map.insert(doc.id.clone(), hex_encode(&crystal.crystal_id));
        }

        let mut pse_h3 = vec![];
        let mut bm_h3 = vec![];
        for query in &ds.queries {
            let graded_map: HashMap<String, u8> = query
                .relevant
                .iter()
                .filter_map(|r| doc_id_map.get(&r.doc_id).map(|cid| (cid.clone(), r.grade)))
                .collect();
            let q_noisy = fixtures::noise_inject(&query.text, rate, noise_seed ^ 0xAB);
            let qvec = text_to_vector8(&q_noisy);
            let pse_ranked: Vec<String> = store
                .score_tripolar(&qvec)
                .into_iter()
                .map(|m| m.crystal_id_hex)
                .collect();
            pse_h3.push(ir_metrics::hit_at_k(&pse_ranked, &graded_map, 3));
            let bm_scores = bm25.score(&q_noisy);
            let mut bm_ranked: Vec<(usize, f64)> = bm_scores.iter().copied().enumerate().collect();
            bm_ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
            let bm_ids: Vec<String> = bm_ranked
                .iter()
                .map(|(i, _)| {
                    doc_id_map
                        .get(&ds.documents[*i].id)
                        .cloned()
                        .unwrap_or_default()
                })
                .collect();
            bm_h3.push(ir_metrics::hit_at_k(&bm_ids, &graded_map, 3));
        }
        let pse_mean = pse_h3.iter().sum::<f64>() / pse_h3.len() as f64;
        let bm_mean = bm_h3.iter().sum::<f64>() / bm_h3.len() as f64;
        if rate == 0.0 {
            baseline_hit3 = pse_mean;
        }
        levels.push(NoiseLevel {
            rate,
            pse_hit3: pse_mean,
            bm25_hit3: bm_mean,
            pse_degradation: if rate == 0.0 {
                0.0
            } else {
                baseline_hit3 - pse_mean
            },
            bm25_degradation: 0.0,
        });
    }
    // Back-fill bm25 baseline
    let bm25_base = levels.first().map(|l| l.bm25_hit3).unwrap_or(0.0);
    for l in &mut levels {
        l.bm25_degradation = bm25_base - l.bm25_hit3;
    }
    B2Results {
        levels,
        baseline_hit3,
    }
}

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
