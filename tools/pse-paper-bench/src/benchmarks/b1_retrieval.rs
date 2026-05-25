use crate::{datasets, fixtures, metrics::ir_metrics, baselines, stats};
use std::collections::HashMap;
use serde::Serialize;

#[derive(Serialize, Clone, Default)]
pub struct SystemMetrics {
    pub hit1: f64, pub hit1_lo: f64, pub hit1_hi: f64,
    pub hit3: f64, pub hit3_lo: f64, pub hit3_hi: f64,
    pub hit5: f64, pub hit5_lo: f64, pub hit5_hi: f64,
    pub mrr: f64,  pub mrr_lo: f64,  pub mrr_hi: f64,
    pub ndcg10: f64, pub ndcg10_lo: f64, pub ndcg10_hi: f64,
}

#[derive(Serialize, Clone, Default)]
pub struct B1Results {
    pub pfauenthron: SystemMetrics,
    pub bm25: SystemMetrics,
    pub cosine_only: SystemMetrics,
    pub stability_only: SystemMetrics,
    pub delta_vs_bm25_hit3: f64,
    pub delta_vs_cosine_hit3: f64,
    pub n_queries: usize,
    pub n_docs: usize,
}

pub fn run() -> B1Results {
    let ds = datasets::retrieval_dataset();
    let dir = tempfile::tempdir().expect("tmpdir");
    let mut store = fixtures::open_store(dir.path());

    // Build BM25 index over document bodies
    let doc_texts: Vec<String> = ds.documents.iter().map(|d| format!("{} {}", d.title, d.body)).collect();
    let bm25 = baselines::bm25::Bm25Index::build(&doc_texts);

    // Commit all documents to ILStore
    let mut doc_id_map: HashMap<String, String> = HashMap::new(); // dataset doc_id → crystal_id_hex
    for (i, doc) in ds.documents.iter().enumerate() {
        let crystal = fixtures::build_crystal(
            &format!("b1-doc-{}", doc.id),
            doc.stability,
            doc.kuramoto,
        );
        let question = format!("{} {}", doc.title, doc.body);
        let _hash = fixtures::commit_crystal(&mut store, &crystal, &question, i + 1);
        // Recover the hex ID from the crystal_id bytes
        let hex_id = hex_encode(&crystal.crystal_id);
        doc_id_map.insert(doc.id.clone(), hex_id);
    }
    let all_ids: Vec<String> = doc_id_map.values().cloned().collect();

    // Run per-query evaluation
    let mut pse_h1 = vec![];
    let mut pse_h3 = vec![];
    let mut pse_h5 = vec![];
    let mut pse_mrr = vec![];
    let mut pse_ndcg = vec![];

    let mut bm_h3 = vec![];
    let mut cos_h3 = vec![];

    use pse_adapter_il::text_to_vector8;

    for query in &ds.queries {
        let graded_map: HashMap<String, u8> = query.relevant.iter().filter_map(|r| {
            doc_id_map.get(&r.doc_id).map(|cid| (cid.clone(), r.grade))
        }).collect();

        let qvec = text_to_vector8(&query.text);

        // Pfauenthron++
        let pse_ranked: Vec<String> = store.score_tripolar(&qvec).into_iter().map(|m| m.crystal_id_hex).collect();
        pse_h1.push(ir_metrics::hit_at_k(&pse_ranked, &graded_map, 1));
        pse_h3.push(ir_metrics::hit_at_k(&pse_ranked, &graded_map, 3));
        pse_h5.push(ir_metrics::hit_at_k(&pse_ranked, &graded_map, 5));
        pse_mrr.push(ir_metrics::mrr(&pse_ranked, &graded_map));
        pse_ndcg.push(ir_metrics::ndcg_at_k(&pse_ranked, &graded_map, 10));

        // BM25
        let bm_scores = bm25.score(&query.text);
        let mut bm_ranked: Vec<(usize, f64)> = bm_scores.iter().copied().enumerate().collect();
        bm_ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
        let bm_ranked_ids: Vec<String> = bm_ranked.iter().map(|(i, _)| {
            doc_id_map.get(&ds.documents[*i].id).cloned().unwrap_or_default()
        }).collect();
        bm_h3.push(ir_metrics::hit_at_k(&bm_ranked_ids, &graded_map, 3));

        // Cosine-only
        let cos_ranked = baselines::cosine_only::rank(&store, &qvec, all_ids.len());
        cos_h3.push(ir_metrics::hit_at_k(&cos_ranked, &graded_map, 3));
    }

    let ci = |v: &[f64], label: &str| stats::bootstrap_ci(v, 2000, stats::label_seed(label));

    let h1ci = ci(&pse_h1, "b1-hit1");
    let h3ci = ci(&pse_h3, "b1-hit3");
    let h5ci = ci(&pse_h5, "b1-hit5");
    let mci  = ci(&pse_mrr, "b1-mrr");
    let nci  = ci(&pse_ndcg, "b1-ndcg10");

    // Stability-only: rank all docs by stability descending
    let _stab_ranked = baselines::stability_only::rank(&store, &all_ids);

    B1Results {
        pfauenthron: SystemMetrics {
            hit1: h1ci.mean, hit1_lo: h1ci.lo95, hit1_hi: h1ci.hi95,
            hit3: h3ci.mean, hit3_lo: h3ci.lo95, hit3_hi: h3ci.hi95,
            hit5: h5ci.mean, hit5_lo: h5ci.lo95, hit5_hi: h5ci.hi95,
            mrr:  mci.mean,  mrr_lo:  mci.lo95,  mrr_hi:  mci.hi95,
            ndcg10: nci.mean, ndcg10_lo: nci.lo95, ndcg10_hi: nci.hi95,
        },
        bm25: SystemMetrics {
            hit3: stats::bootstrap_ci(&bm_h3, 2000, stats::label_seed("b1-bm25-hit3")).mean,
            ..Default::default()
        },
        cosine_only: SystemMetrics {
            hit3: stats::bootstrap_ci(&cos_h3, 2000, stats::label_seed("b1-cos-hit3")).mean,
            ..Default::default()
        },
        stability_only: SystemMetrics { ..Default::default() },
        delta_vs_bm25_hit3: stats::paired_diff(&bm_h3, &pse_h3),
        delta_vs_cosine_hit3: stats::paired_diff(&cos_h3, &pse_h3),
        n_queries: ds.queries.len(),
        n_docs: ds.documents.len(),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
