//! Swarm-consensus scoring — assimilated from Wonderlamp's Ophanim/Konus
//! pipeline, re-expressed in integer `Q16` on the epistemic substrate.
//!
//! Wonderlamp scored n LLM candidates in `f64` and, when consensus failed,
//! "gracefully" emitted the best anyway. Here the same mathematics is
//! bit-replayable integer arithmetic (CROSS-007 honored beyond necessity —
//! this is not a gate path, but determinism is free) and the *delivery* is
//! fail-closed: the consensus total is advisory, and the synthesizer layer
//! folds it into the existing `Q16` confidence so the agent's
//! `min_confidence` filter — operator policy — does the gating (CROSS-010:
//! energy ranks, the policy gates).
//!
//! Per candidate k (Ophanim):
//! - `ψ_k` — agreement: mean weighted Jaccard similarity to every peer
//!   (functions 40% / types 25% / imports 20% / size 15%; the Wonderlamp
//!   weights, kept verbatim as documented heuristics). Below
//!   [`ConsensusConfig::psi_outlier`] the candidate is ejected (`D_k = 0`).
//! - `ρ_k` — content richness: `min(1, (|functions| + |types|) / 5)` — the
//!   original Ophanim rule.
//! - `ω_k` — structural validity: the fraction of recognized source files
//!   whose extraction yields structure and whose delimiters balance.
//! - `D_k = ψ·ρ·ω` — the house tripolar form.
//!
//! Ensemble (Konus): `D_total = geomean(max(D_k, ε)) ⊗ Ω` where `Ω` is the
//! mean pairwise similarity. The `ε` floor (one raw unit) keeps a single
//! ejected outlier from hard-zeroing the diagnostic total while still
//! dragging it decisively below any sane threshold — the soft-unanimity
//! semantics of the original, integerized.
//!
//! Patches that touch no recognized source files (config, docs) score the
//! **neutral band** `ρ = ω = HALF`: structure-free work is not evidence of
//! disagreement, and refusing to ever emit it would be over-closed.

use std::collections::BTreeSet;

use kosmo_core::Q16;
use kosmo_hyphae::codematrix::pooled_symbol_sets;
use kosmo_hyphae::xlang::SourceLanguage;

use crate::{FileChange, FileChangeKind, Patch, SynthesisResult};

/// Language-agnostic features of one candidate patch, pooled across its
/// file changes via the xlang classifier pass (7 languages, never regex).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CandidateFeatures {
    /// Function keys in `name/arity` form.
    pub functions: BTreeSet<String>,
    /// Type names.
    pub types: BTreeSet<String>,
    /// Imported module names.
    pub imports: BTreeSet<String>,
    /// Total proposed lines across all file changes.
    pub loc: u32,
    /// File changes with a recognized source extension.
    pub recognized_files: u32,
    /// Recognized files that extract ≥1 observation with balanced delimiters.
    pub valid_files: u32,
}

/// Extract the consensus features of a patch. Deletes contribute no
/// structure; unrecognized extensions are skipped (not everything is code).
pub fn extract_features(patch: &Patch) -> CandidateFeatures {
    let sources: Vec<(&str, &str)> = patch
        .file_changes
        .iter()
        .filter(|fc| !matches!(fc.kind, FileChangeKind::Delete))
        .map(|fc| (fc.path.to_str().unwrap_or(""), fc.content.as_str()))
        .collect();
    let (pooled, recognized) = pooled_symbol_sets(sources.iter().copied());

    let valid = sources
        .iter()
        .filter(|(loc, content)| {
            SourceLanguage::from_path(loc).is_some_and(|lang| {
                let sets = kosmo_hyphae::xlang::symbol_sets(lang, content);
                sets.total >= 1 && delimiters_balance(content)
            })
        })
        .count() as u32;

    CandidateFeatures {
        functions: pooled.functions,
        types: pooled.types,
        imports: pooled.imports,
        loc: patch.total_lines(),
        recognized_files: recognized,
        valid_files: valid,
    }
}

fn delimiters_balance(content: &str) -> bool {
    let mut pairs = [(0i64, 0i64); 3];
    for ch in content.chars() {
        match ch {
            '(' => pairs[0].0 += 1,
            ')' => pairs[0].1 += 1,
            '[' => pairs[1].0 += 1,
            ']' => pairs[1].1 += 1,
            '{' => pairs[2].0 += 1,
            '}' => pairs[2].1 += 1,
            _ => {}
        }
    }
    pairs.iter().all(|(open, close)| open == close)
}

/// Tunables of the consensus assessment. Defaults are the Wonderlamp
/// constants, documented as heuristics.
#[derive(Clone, Debug)]
pub struct ConsensusConfig {
    /// Monolith threshold Θ: ensembles below it are divergent.
    pub theta: Q16,
    /// Ophanim outlier cutoff: candidates with ψ below it score `D = 0`.
    pub psi_outlier: Q16,
    /// Maximum Coagula repair rounds the synthesizer layer may attempt.
    pub max_repair_rounds: u32,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            theta: Q16::ratio(20, 100).expect("const"),
            psi_outlier: Q16::ratio(15, 100).expect("const"),
            max_repair_rounds: 2,
        }
    }
}

/// One candidate's tripolar consensus score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateScore {
    pub psi: Q16,
    pub rho: Q16,
    pub omega: Q16,
    pub d: Q16,
}

/// The ensemble verdict — advisory numbers; delivery policy lives with the
/// caller (the synthesizer folds `d_total` into the result confidence).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsensusReport {
    pub scores: Vec<CandidateScore>,
    /// `geomean(max(D_k, ε)) ⊗ Ω` — the ensemble's agreement-weighted total.
    pub d_total: Q16,
    /// Index of the highest-`D` candidate (stable tie-break: lowest index).
    pub best_index: usize,
    /// Function keys present in ≥2 candidates but missing from the best —
    /// the Coagula repair menu.
    pub repair_targets: Vec<String>,
}

impl ConsensusReport {
    /// `true` when the ensemble agrees at or above the configured Θ.
    pub fn convergent(&self, cfg: &ConsensusConfig) -> bool {
        self.d_total.at_least(cfg.theta)
    }
}

/// Weighted Jaccard similarity between two candidates' features.
///
/// Both-empty axes count as vacuous agreement (`ONE`): the absence of types
/// in both candidates is consensus, not divergence.
pub fn similarity(a: &CandidateFeatures, b: &CandidateFeatures) -> Q16 {
    let j_fn = jaccard(&a.functions, &b.functions);
    let j_ty = jaccard(&a.types, &b.types);
    let j_im = jaccard(&a.imports, &b.imports);
    let s_sz = if a.loc == 0 && b.loc == 0 {
        Q16::ONE
    } else {
        Q16::ratio(a.loc.min(b.loc) as u64, a.loc.max(b.loc) as u64).unwrap_or(Q16::ZERO)
    };

    // Integer percent weights summed before the single division — exact at
    // the boundaries (all-ONE inputs yield exactly ONE, no flooring drift).
    const WEIGHTS: [(i64, usize); 4] = [(40, 0), (25, 1), (20, 2), (15, 3)];
    let axes = [j_fn, j_ty, j_im, s_sz];
    let sum: i64 = WEIGHTS.iter().map(|(w, i)| w * axes[*i].raw()).sum();
    Q16::from_raw(sum / 100)
}

fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> Q16 {
    if a.is_empty() && b.is_empty() {
        return Q16::ONE;
    }
    let intersection = a.intersection(b).count() as u64;
    let union = a.union(b).count() as u64;
    Q16::ratio(intersection, union).unwrap_or(Q16::ZERO)
}

/// Assess an ensemble of synthesis results (Ophanim + Konus).
///
/// Deterministic: identical inputs yield an identical report, bit for bit.
pub fn assess_candidates(results: &[SynthesisResult], cfg: &ConsensusConfig) -> ConsensusReport {
    let features: Vec<CandidateFeatures> =
        results.iter().map(|r| extract_features(&r.patch)).collect();
    assess_features(&features, cfg)
}

/// Feature-level assessment — the pure core (used directly by tests and by
/// repair rounds that re-score without re-synthesizing).
pub fn assess_features(features: &[CandidateFeatures], cfg: &ConsensusConfig) -> ConsensusReport {
    let n = features.len();
    if n == 0 {
        return ConsensusReport {
            scores: vec![],
            d_total: Q16::ZERO,
            best_index: 0,
            repair_targets: vec![],
        };
    }

    // Pairwise similarities (symmetric, computed once).
    let mut pairwise: Vec<Vec<Q16>> = vec![vec![Q16::ONE; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let s = similarity(&features[i], &features[j]);
            pairwise[i][j] = s;
            pairwise[j][i] = s;
        }
    }

    let scores: Vec<CandidateScore> = features
        .iter()
        .enumerate()
        .map(|(k, f)| {
            // ψ: mean agreement with peers; a lone candidate is vacuously
            // in agreement with itself (ρ/ω then govern).
            let psi = if n == 1 {
                Q16::ONE
            } else {
                let sum: i64 = (0..n)
                    .filter(|j| *j != k)
                    .map(|j| pairwise[k][j].raw())
                    .sum();
                Q16::from_raw(sum / (n as i64 - 1))
            };
            // ρ: the original Ophanim richness rule, min(1, (|F|+|T|)/5);
            // structure-free patches sit in the neutral band.
            let structural = (f.functions.len() + f.types.len()) as u64;
            let rho = if f.recognized_files == 0 {
                Q16::HALF
            } else {
                Q16::ratio(structural.min(5), 5).unwrap_or(Q16::ZERO)
            };
            // ω: structural validity of the recognized files.
            let omega = if f.recognized_files == 0 {
                Q16::HALF
            } else {
                Q16::ratio(f.valid_files as u64, f.recognized_files as u64).unwrap_or(Q16::ZERO)
            };
            let d = if psi < cfg.psi_outlier {
                Q16::ZERO // outlier ejected
            } else {
                psi.checked_mul(rho)
                    .and_then(|pr| pr.checked_mul(omega))
                    .unwrap_or(Q16::ZERO)
            };
            CandidateScore { psi, rho, omega, d }
        })
        .collect();

    // Ω: global coherence = mean pairwise similarity (ONE for n < 2).
    let omega_global = if n < 2 {
        Q16::ONE
    } else {
        let mut sum: i64 = 0;
        let mut count: i64 = 0;
        for (i, row) in pairwise.iter().enumerate() {
            for s in &row[(i + 1)..] {
                sum += s.raw();
                count += 1;
            }
        }
        Q16::from_raw(sum / count)
    };

    // D_total with the ε floor: an ejected outlier drags the total toward
    // zero without hard-zeroing the diagnostic value.
    let floored: Vec<Q16> = scores
        .iter()
        .map(|s| if s.d.is_zero() { Q16::from_raw(1) } else { s.d })
        .collect();
    let d_total = Q16::geomean(&floored)
        .checked_mul(omega_global)
        .unwrap_or(Q16::ZERO);

    let best_index = scores
        .iter()
        .enumerate()
        .max_by(|(ia, a), (ib, b)| a.d.cmp(&b.d).then(ib.cmp(ia)))
        .map(|(i, _)| i)
        .unwrap_or(0);

    // Repair menu: function keys in ≥2 candidates, absent from the best.
    let best_fns = &features[best_index].functions;
    let mut counts: Vec<(&String, usize)> = vec![];
    for f in features {
        for name in &f.functions {
            match counts.iter_mut().find(|(n, _)| *n == name) {
                Some((_, c)) => *c += 1,
                None => counts.push((name, 1)),
            }
        }
    }
    let repair_targets: Vec<String> = counts
        .into_iter()
        .filter(|(name, c)| *c >= 2 && !best_fns.contains(*name))
        .map(|(name, _)| name.clone())
        .collect();

    ConsensusReport {
        scores,
        d_total,
        best_index,
        repair_targets,
    }
}

/// Convenience used by tests and synthesizer layers: features straight from
/// `(path, content)` pairs without building a full `Patch`.
pub fn features_from_files(files: &[(&str, &str)]) -> CandidateFeatures {
    let patch = Patch::new(
        kosmo_core::Digest::ZERO,
        files
            .iter()
            .map(|(p, c)| FileChange::create(*p, *c))
            .collect::<Vec<_>>(),
        "consensus-test",
    );
    extract_features(&patch)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUST_A: &str = "pub fn route(p: &str) -> u32 { p.len() as u32 }\npub struct Router { t: u32 }\nuse std::collections::BTreeMap;\n";
    const RUST_A2: &str = "use std::collections::BTreeMap;\npub struct Router { table: u32 }\npub fn route(path: &str) -> u32 { path.len() as u32 }\n";
    const RUST_B: &str =
        "pub fn parse(x: u64) -> u64 { x * 2 }\npub enum Token { A, B }\nuse std::fmt;\n";

    fn cands(files: &[&[(&str, &str)]]) -> Vec<CandidateFeatures> {
        files.iter().map(|f| features_from_files(f)).collect()
    }

    #[test]
    fn extraction_pools_across_languages() {
        let f = features_from_files(&[
            ("src/lib.rs", RUST_A),
            (
                "lib/handlers.py",
                "import json\ndef handle(x):\n    return x\n",
            ),
            ("README.md", "# prose, not code"),
        ]);
        assert!(f.functions.contains("route/1"));
        assert!(f.functions.contains("handle/1"));
        assert!(f.types.contains("Router"));
        assert!(f.imports.iter().any(|m| m.contains("json")));
        assert_eq!(f.recognized_files, 2, "markdown is skipped");
    }

    #[test]
    fn convergent_ensemble_scores_high_and_deterministically() {
        let features = cands(&[
            &[("src/lib.rs", RUST_A)],
            &[("src/lib.rs", RUST_A2)],
            &[("src/lib.rs", RUST_A)],
        ]);
        let cfg = ConsensusConfig::default();
        let report = assess_features(&features, &cfg);
        assert!(
            report.convergent(&cfg),
            "same structure in different orderings must agree: {:?}",
            report.d_total
        );
        assert!(report.repair_targets.is_empty());
        // Bit-for-bit determinism.
        assert_eq!(report, assess_features(&features, &cfg));
    }

    #[test]
    fn divergent_ensemble_falls_below_theta() {
        let features = cands(&[
            &[("src/a.rs", RUST_A)],
            &[("src/b.rs", RUST_B)],
            &[("src/c.rs", "pub fn zeta() -> bool { true }\n")],
        ]);
        let cfg = ConsensusConfig::default();
        let report = assess_features(&features, &cfg);
        assert!(
            !report.convergent(&cfg),
            "three unrelated answers are not consensus: {:?}",
            report.d_total
        );
    }

    #[test]
    fn outlier_is_ejected_but_does_not_hard_zero_the_total() {
        let features = cands(&[
            &[("src/lib.rs", RUST_A)],
            &[("src/lib.rs", RUST_A2)],
            &[("src/lib.rs", RUST_A)],
            &[(
                "src/alien.py",
                "def completely_other_thing(a, b, c):\n    return a\n",
            )],
        ]);
        let cfg = ConsensusConfig::default();
        let report = assess_features(&features, &cfg);
        let alien = &report.scores[3];
        assert_eq!(
            alien.d,
            Q16::ZERO,
            "outlier ψ below cutoff ejects: {alien:?}"
        );
        assert!(
            report.d_total.is_positive(),
            "ε floor keeps diagnostics alive"
        );
        assert!(!report.convergent(&cfg), "…but the ensemble is divergent");
        assert_ne!(report.best_index, 3);
    }

    #[test]
    fn repair_targets_name_what_the_best_is_missing() {
        // Two peers carry `extra/0`; the richest candidate lacks it.
        let with_extra = "pub fn route(p: &str) -> u32 { 1 }\npub fn extra() {}\n";
        let features = cands(&[
            &[("src/lib.rs", RUST_A)], // best: route + Router type
            &[("src/lib.rs", with_extra)],
            &[("src/lib.rs", with_extra)],
        ]);
        let report = assess_features(&features, &ConsensusConfig::default());
        if report.best_index == 0 {
            assert!(
                report.repair_targets.contains(&"extra/0".to_string()),
                "{:?}",
                report.repair_targets
            );
        } else {
            assert!(report.repair_targets.is_empty());
        }
    }

    #[test]
    fn structure_free_patches_sit_in_the_neutral_band() {
        let features = cands(&[
            &[("Cargo.toml", "[package]\nname = \"x\"\n")],
            &[("Cargo.toml", "[package]\nname = \"x\"\n")],
        ]);
        let cfg = ConsensusConfig::default();
        let report = assess_features(&features, &cfg);
        let s = &report.scores[0];
        assert_eq!(s.rho, Q16::HALF);
        assert_eq!(s.omega, Q16::HALF);
        assert_eq!(s.psi, Q16::ONE, "identical config = vacuous agreement");
        assert!(
            report.convergent(&cfg),
            "agreeing structure-free work is emittable: {:?}",
            report.d_total
        );
    }

    #[test]
    fn empty_ensemble_is_fail_closed() {
        let report = assess_features(&[], &ConsensusConfig::default());
        assert_eq!(report.d_total, Q16::ZERO);
        assert!(!report.convergent(&ConsensusConfig::default()));
    }

    /// The disease test: no ported source may smuggle stack names, file-tree
    /// templates, or entity scaffolds back in.
    #[test]
    fn assimilated_sources_carry_no_crud_disease() {
        for (name, src) in [
            ("consensus.rs", include_str!("consensus.rs")),
            (
                "codematrix.rs",
                include_str!("../../kosmo-hyphae/src/codematrix.rs"),
            ),
        ] {
            // Judge the shipped code, not this test's own forbidden list.
            let body = src.split("#[cfg(test)]").next().unwrap_or(src);
            let lower = body.to_lowercase();
            for forbidden in [
                "actix",
                "sqlx",
                "axum",
                "jwt",
                "postgres",
                "bcrypt",
                "backend/",
                "models/",
                "handlers/",
                "migrations/",
                "vanilla",
                "create_entity",
                "list_entities",
            ] {
                assert!(
                    !lower.contains(forbidden),
                    "{name} contains forbidden '{forbidden}'"
                );
            }
        }
    }
}
