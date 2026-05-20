//! Demonstriert PSEs strukturellen Reasoning-Filter.
//!
//! Das Beispiel zeigt den Kernmechanismus ohne LLM, ohne externe Abhängigkeiten:
//! Schwaches Reasoning → HOLD. Starkes Reasoning → COMMIT.
//!
//! Ausführen:
//!   cargo run -p pse-traverse --example reasoning_filter

use pse_traverse::{
    cognition::{
        filter::CognitionFilter,
        pipeline::{CognitionInput, CognitiveComponents},
        Fixed,
    },
    dynamic_state::Hash256,
};

fn input_with_support(support: f64, phase: u64) -> CognitionInput {
    let zero = Fixed::quantize(0.0, 9).unwrap();
    CognitionInput {
        null_center_id: Hash256::zero(),
        cognitive_components: CognitiveComponents {
            psi: Fixed::quantize(0.7, 9).unwrap(),
            rho: Fixed::quantize(0.7, 9).unwrap(),
            omega: Fixed::quantize(0.6, 9).unwrap(),
            chi: zero.clone(),
            tau: Fixed::quantize(0.2, 9).unwrap(),
        },
        source_traversal_report_hash: None,
        source_projection_report_hash: None,
        spiral_memory_candidates: vec![
            pse_traverse::cognition::CognitiveState5D::from_components(
                Fixed::quantize(0.7, 9).unwrap(),
                Fixed::quantize(0.7, 9).unwrap(),
                Fixed::quantize(0.6, 9).unwrap(),
                zero,
                Fixed::quantize(0.2, 9).unwrap(),
            )
            .unwrap(),
        ],
        constraint_count: 1,
        support_strength: Fixed::quantize(support, 9).unwrap(),
        logical_step: phase,
        carrier_ids: vec!["agent.0".into()],
    }
}

fn fixed_to_f64(f: &Fixed) -> f64 {
    match f {
        Fixed::Rational { num, den } => *num as f64 / *den as f64,
        Fixed::FixedI64 { raw, scale } => *raw as f64 / 10f64.powi(*scale as i32),
    }
}

fn main() {
    let filter = CognitionFilter::standard();

    println!("PSE Structural Reasoning Filter — Demo");
    println!("Layer depth: 6 (full structural scrutiny)");
    println!("Threshold: support_strength >= 0.60 → COMMIT\n");

    let cases: &[(&str, f64, u64)] = &[
        ("Starkes Reasoning  (support=1.00, phase=plan)",     1.00, 1),
        ("Mittleres Reasoning (support=0.67, phase=plan)",    0.67, 1),
        ("Schwaches Reasoning (support=0.33, phase=plan)",    0.33, 1),
        ("Kein Reasoning      (support=0.00, phase=plan)",    0.00, 1),
        ("Starkes Reasoning  (support=1.00, phase=evaluate)", 1.00, 3),
        ("Schwaches Reasoning (support=0.33, phase=evaluate)",0.33, 3),
    ];

    for (label, support, phase) in cases {
        let input = input_with_support(*support, *phase);
        let result = filter.evaluate(&input).expect("filter must not error");

        let decision = if result.committed { "COMMIT ✓" } else { "HOLD   ✗" };
        let panorama = fixed_to_f64(&result.panorama_coverage);
        let gate_info = result
            .failed_gate
            .as_deref()
            .unwrap_or("—");

        println!(
            "{decision}  panorama={:.3}  failed_gate={gate_info:<15}  {label}",
            panorama
        );
    }

    println!("\nKernaussage:");
    println!("  Strukturelle Abdeckung ≥ 60 % → Gate öffnet (COMMIT).");
    println!("  Strukturelle Abdeckung < 60 % → Gate schließt (HOLD).");
    println!("  Panorama-Coverage wächst proportional zur Support-Stärke.");
}
