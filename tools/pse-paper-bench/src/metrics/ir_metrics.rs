use std::collections::HashMap;

/// Hit@k: fraction of queries where the top-k results contain at least one relevant doc (grade >= 1).
pub fn hit_at_k(ranked: &[String], graded: &HashMap<String, u8>, k: usize) -> f64 {
    let top: Vec<_> = ranked.iter().take(k).collect();
    let hit = top
        .iter()
        .any(|id| graded.get(*id).copied().unwrap_or(0) >= 1);
    if hit {
        1.0
    } else {
        0.0
    }
}

/// MRR: 1/rank of first relevant result (grade >= 1), or 0.0 if none in top-10.
pub fn mrr(ranked: &[String], graded: &HashMap<String, u8>) -> f64 {
    for (i, id) in ranked.iter().take(10).enumerate() {
        if graded.get(id).copied().unwrap_or(0) >= 1 {
            return 1.0 / (i + 1) as f64;
        }
    }
    0.0
}

/// NDCG@k using graded relevance (0/1/2).
pub fn ndcg_at_k(ranked: &[String], graded: &HashMap<String, u8>, k: usize) -> f64 {
    let dcg = compute_dcg(ranked, graded, k);
    // Ideal: sort graded labels descending
    let mut ideal_grades: Vec<u8> = graded.values().copied().collect();
    ideal_grades.sort_by(|a, b| b.cmp(a));
    let ideal_ids: Vec<String> = ideal_grades
        .iter()
        .enumerate()
        .map(|(i, _)| format!("ideal_{i}"))
        .collect();
    let ideal_graded: HashMap<String, u8> = ideal_ids
        .iter()
        .cloned()
        .zip(ideal_grades.into_iter())
        .collect();
    let idcg = compute_dcg(&ideal_ids, &ideal_graded, k);
    if idcg < 1e-12 {
        return 0.0;
    }
    dcg / idcg
}

fn compute_dcg(ranked: &[String], graded: &HashMap<String, u8>, k: usize) -> f64 {
    ranked
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, id)| {
            let rel = graded.get(id).copied().unwrap_or(0) as f64;
            let gain = (2.0f64.powf(rel) - 1.0) / (i as f64 + 2.0).log2();
            gain
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graded(pairs: &[(&str, u8)]) -> HashMap<String, u8> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn hit_at_1_exact_match() {
        let g = graded(&[("d1", 2), ("d2", 0)]);
        assert_eq!(hit_at_k(&["d1".into(), "d2".into()], &g, 1), 1.0);
        assert_eq!(hit_at_k(&["d2".into(), "d1".into()], &g, 1), 0.0);
    }

    #[test]
    fn mrr_first_position() {
        let g = graded(&[("d1", 2), ("d2", 0)]);
        assert!((mrr(&["d1".into()], &g) - 1.0).abs() < 1e-10);
        assert!((mrr(&["d2".into(), "d1".into()], &g) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn ndcg_perfect_ranking() {
        let g = graded(&[("d1", 2), ("d2", 1), ("d3", 0)]);
        let ranked = vec!["d1".to_string(), "d2".to_string(), "d3".to_string()];
        let score = ndcg_at_k(&ranked, &g, 3);
        assert!(
            score > 0.99,
            "perfect ranking should give NDCG ~ 1.0, got {score}"
        );
    }
}
