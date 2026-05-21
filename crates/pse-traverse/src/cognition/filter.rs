//! Sauberer Einstiegspunkt für PSEs strukturellen Reasoning-Filter (§16).
//!
//! Der Filter wertet einen `CognitionInput` gegen strukturelle Qualitätsschwellen
//! aus und gibt eine binäre Entscheidung zurück: Schritt commiten oder halten.
//!
//! # Schnellstart
//!
//! ```rust
//! use pse_traverse::cognition::{filter::CognitionFilter, pipeline::CognitionInput};
//!
//! let filter = CognitionFilter::standard();
//! let input  = CognitionInput::minimal(pse_traverse::dynamic_state::Hash256::zero());
//! let result = filter.evaluate(&input).unwrap();
//! println!("committed: {}", result.committed);
//! ```

use std::collections::BTreeMap;

use crate::dynamic_state::Hash256;

use super::pipeline::{run_cognition, CognitionInput};
use super::primitives::{CognitionError, Fixed};
use super::report::CognitionOutcome;
use super::run_descriptor::{CognitionPolicies, CognitionRunDescriptor, CognitionThresholds};

/// Filter kalibriert auf eine gegebene PSE-Schichttiefe.
///
/// - `layer_count = 0`  → permissiv (kein strukturelles Gate).
/// - `layer_count >= 6` → volle strukturelle Prüfung.
pub struct CognitionFilter {
    layer_count: u32,
}

/// Ergebnis der Auswertung einer einzelnen Reasoning-Phase.
pub struct FilterResult {
    /// `true` wenn alle strukturellen Gates bestehen und der Agent commiten darf.
    pub committed: bool,
    /// Panorama-Abdeckung dieser Phase.
    pub panorama_coverage: Fixed,
    /// Welches Gate einen Hold ausgelöst hat (falls committed=false).
    pub failed_gate: Option<String>,
}

impl CognitionFilter {
    /// Permissiver Filter — alle Gates offen. Eignet sich für Baseline-Vergleiche (B0).
    pub fn permissive() -> Self {
        Self { layer_count: 0 }
    }

    /// Voller struktureller Filter (Schichttiefe 6). Standard für produktive Nutzung.
    pub fn standard() -> Self {
        Self { layer_count: 6 }
    }

    /// Benutzerdefinierte Schichttiefe (0–13).
    pub fn with_layers(layer_count: u32) -> Self {
        Self { layer_count }
    }

    /// Wertet eine einzelne Reasoning-Phase aus.
    pub fn evaluate(&self, input: &CognitionInput) -> Result<FilterResult, CognitionError> {
        let rd = build_filter_rd(self.layer_count, input.logical_step);
        let result = run_cognition(input, &rd)?;
        let committed = matches!(result.outcome, CognitionOutcome::CandidateBundle { .. });
        let panorama_coverage = result.panorama.coverage_score.clone();
        let failed_gate = match &result.outcome {
            CognitionOutcome::Hold { hold } => Some(format!("{:?}", hold.failed_gate)),
            _ => None,
        };
        Ok(FilterResult {
            committed,
            panorama_coverage,
            failed_gate,
        })
    }
}

/// Baut einen `CognitionRunDescriptor` für den Filter.
///
/// Schwellwerte skalieren linear mit `layer_count / 6`:
/// - B0 (0 Layer): permissiv (alle Schwellen = 0)
/// - B6+ (≥6 Layer): volle Prüfung (`min_constraint_mass=0.6`, etc.)
fn build_filter_rd(layer_count: u32, logical_step: u64) -> CognitionRunDescriptor {
    let thresholds = if layer_count == 0 {
        CognitionThresholds::permissive()
    } else {
        let scale = (layer_count as f64 / 6.0_f64).min(1.0);
        CognitionThresholds {
            min_resonance: Fixed::quantize(0.0, 9).unwrap(),
            max_entropy: Fixed::quantize(2.0, 9).unwrap(),
            min_constraint_mass: Fixed::Rational {
                num: (60.0 * scale) as i128,
                den: 100,
            },
            min_entropy_reduction: Fixed::quantize(0.0, 9).unwrap(),
            min_feasible_uniqueness: Fixed::quantize(0.0, 9).unwrap(),
            min_panorama_coverage: Fixed::Rational {
                num: (10.0 * scale) as i128,
                den: 100,
            },
            min_self_model_coherence: Fixed::Rational {
                num: (40.0 * scale) as i128,
                den: 100,
            },
            max_drift: Fixed::quantize(2.0, 9).unwrap(),
            min_attractor_score: Fixed::quantize(0.0, 9).unwrap(),
            min_trigger_score: Fixed::quantize(0.0, 9).unwrap(),
        }
    };
    CognitionRunDescriptor {
        run_id: format!("pse.filter.l{layer_count}.step{logical_step}"),
        problem_spec_hash: Hash256::zero(),
        traversal_report_hash: None,
        projection_v2_hash: None,
        seed: layer_count as u64,
        operator_versions: BTreeMap::new(),
        thresholds,
        policies: CognitionPolicies::default_policies(),
        canonicalization_version: "cognition-v0.1".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic_state::Hash256;

    #[test]
    fn strong_input_commits_on_standard_filter() {
        let mut input = CognitionInput::minimal(Hash256::zero());
        input.support_strength = Fixed::quantize(1.0, 9).unwrap();
        input.logical_step = 1;
        let result = CognitionFilter::standard().evaluate(&input).unwrap();
        assert!(
            result.committed,
            "support=1.0 muss auf standard-Filter commiten"
        );
    }

    #[test]
    fn weak_input_holds_on_standard_filter() {
        let mut input = CognitionInput::minimal(Hash256::zero());
        input.support_strength = Fixed::quantize(0.2, 9).unwrap();
        // constraint_count=1 damit constraint_mass = 1×0.2 = 0.2 < 0.6 → Hold
        input.constraint_count = 1;
        input.logical_step = 1;
        let result = CognitionFilter::standard().evaluate(&input).unwrap();
        assert!(
            !result.committed,
            "support=0.2 muss auf standard-Filter halten"
        );
        assert!(result.failed_gate.is_some());
    }

    #[test]
    fn permissive_filter_always_commits() {
        let mut input = CognitionInput::minimal(Hash256::zero());
        input.support_strength = Fixed::quantize(0.0, 9).unwrap();
        let result = CognitionFilter::permissive().evaluate(&input).unwrap();
        assert!(result.committed, "permissiver Filter darf nie halten");
    }

    #[test]
    fn panorama_coverage_reflects_support_strength() {
        let mut strong = CognitionInput::minimal(Hash256::zero());
        strong.support_strength = Fixed::quantize(1.0, 9).unwrap();
        strong.constraint_count = 1;
        strong.logical_step = 1;

        let mut weak = CognitionInput::minimal(Hash256::zero());
        weak.support_strength = Fixed::quantize(0.33, 9).unwrap();
        weak.constraint_count = 1;
        weak.logical_step = 1;

        let f = CognitionFilter::permissive();
        let strong_r = f.evaluate(&strong).unwrap();
        let weak_r = f.evaluate(&weak).unwrap();

        // Starke Abdeckung muss höhere Panorama-Coverage erzeugen.
        use crate::cognition::primitives::fixed_ge;
        assert!(
            fixed_ge(&strong_r.panorama_coverage, &weak_r.panorama_coverage),
            "starke coverage muss höhere Panorama ergeben"
        );
    }
}
