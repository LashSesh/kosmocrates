use std::collections::{HashMap, HashSet};

const K1: f64 = 1.2;
const B: f64 = 0.75;

pub struct Bm25Index {
    docs: Vec<Vec<String>>,
    idf: HashMap<String, f64>,
    avgdl: f64,
}

impl Bm25Index {
    pub fn build(raw_docs: &[String]) -> Self {
        let docs: Vec<Vec<String>> = raw_docs.iter().map(|d| tokenize(d)).collect();
        let n = docs.len();
        let avgdl = if n == 0 {
            1.0
        } else {
            docs.iter().map(|d| d.len() as f64).sum::<f64>() / n as f64
        };
        let mut df: HashMap<String, usize> = HashMap::new();
        for doc in &docs {
            let terms: HashSet<&String> = doc.iter().collect();
            for t in terms {
                *df.entry(t.clone()).or_default() += 1;
            }
        }
        let idf = df
            .into_iter()
            .map(|(term, df_t)| {
                let score = ((n as f64 - df_t as f64 + 0.5) / (df_t as f64 + 0.5) + 1.0).ln();
                (term, score)
            })
            .collect();
        Self { docs, idf, avgdl }
    }

    /// Returns scores for each document (same order as input to build).
    pub fn score(&self, query: &str) -> Vec<f64> {
        let qtokens = tokenize(query);
        self.docs
            .iter()
            .map(|doc| {
                let dl = doc.len() as f64;
                let mut tf_map: HashMap<&String, usize> = HashMap::new();
                for t in doc {
                    *tf_map.entry(t).or_default() += 1;
                }
                qtokens
                    .iter()
                    .map(|qt| {
                        let idf = self.idf.get(qt).copied().unwrap_or(0.0);
                        let tf = tf_map.get(qt).copied().unwrap_or(0) as f64;
                        let tf_norm = tf * (K1 + 1.0) / (tf + K1 * (1.0 - B + B * dl / self.avgdl));
                        idf * tf_norm
                    })
                    .sum::<f64>()
            })
            .collect()
    }
}

pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 2)
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_match_ranks_first() {
        let docs = vec![
            "quantum mechanics wave function".to_string(),
            "thermodynamics entropy heat".to_string(),
            "entropy thermodynamics free energy".to_string(),
        ];
        let idx = Bm25Index::build(&docs);
        let scores = idx.score("entropy thermodynamics");
        let best = scores
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i);
        // doc 1 or 2 should rank highest (both mention entropy + thermodynamics)
        assert!(best == Some(1) || best == Some(2));
    }
}
