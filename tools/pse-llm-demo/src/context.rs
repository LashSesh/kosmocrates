//! Crystal Context Renderer — bridges PSE topology → LLM-injectable text.
//!
//! Takes `CrystalRecord`s (crystal + source chunks) and produces a compact
//! prompt block that can be prepended to an LLM system prompt.  The renderer
//! also provides a keyword-based coverage scorer for the A/B comparison.

use crate::memory::CrystalRecord;

// Domain keywords for the coverage metric.  Extracted from the question
// bank — covers thermodynamics + information theory overlap vocabulary.
const COVERAGE_KEYWORDS: &[&str] = &[
    "entropy",
    "thermodynamic",
    "information",
    "maxwell",
    "boltzmann",
    "irreversible",
    "disorder",
    "uncertainty",
    "statistical",
    "macrostate",
    "microstate",
    "compression",
    "landauer",
    "arrow",
    "equilibrium",
    "logarithm",
    "spontaneous",
    "dissipation",
    "shannon",
    "phase",
];

/// Render the top-k most stable crystal records into an LLM-injectable string.
///
/// The output is designed to be injected into the system prompt.  It names
/// the patterns, quotes their source sentences, and instructs the model to
/// treat them as structural anchors — not as facts to repeat verbatim.
pub fn render_crystal_context(records: &[CrystalRecord], top_k: usize) -> String {
    if records.is_empty() {
        return String::new();
    }

    let mut sorted = records.to_vec();
    sorted.sort_by(|a, b| {
        b.crystal
            .stability_score
            .partial_cmp(&a.crystal.stability_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sorted.truncate(top_k);

    let mut out = format!(
        "[PSE Cognitive Substrate — {} stable pattern(s) from prior session(s)]\n\n",
        sorted.len()
    );

    for r in &sorted {
        let id: String = r
            .crystal
            .crystal_id
            .iter()
            .take(4)
            .map(|b| format!("{b:02x}"))
            .collect();
        out.push_str(&format!(
            "▸ Pattern #{id} (stability {:.3}, session {})\n",
            r.crystal.stability_score, r.session
        ));
        for chunk in r.source_chunks.iter().take(3) {
            let t = chunk.trim();
            if !t.is_empty() {
                out.push_str(&format!("  \"{t}\"\n"));
            }
        }
        out.push('\n');
    }

    out.push_str(
        "These patterns are topologically stable structures identified by PSE from prior \
         reasoning.  Use them as conceptual anchors — they reflect which ideas formed \
         durable structural relationships, not instructions to repeat specific phrases.\n",
    );

    out
}

/// Count how many domain keywords appear (case-insensitive) in the response.
/// Returns `(hits, total)`.
pub fn score_coverage(response: &str) -> (usize, usize) {
    let lower = response.to_lowercase();
    let hits = COVERAGE_KEYWORDS
        .iter()
        .filter(|k| lower.contains(*k))
        .count();
    (hits, COVERAGE_KEYWORDS.len())
}

/// Print a side-by-side A/B report to stdout.
pub fn print_ab_report(
    baseline_response: &str,
    augmented_response: &str,
    baseline_ms: u128,
    augmented_ms: u128,
) {
    let (b_hits, total) = score_coverage(baseline_response);
    let (a_hits, _) = score_coverage(augmented_response);

    let b_pct = b_hits as f64 / total as f64 * 100.0;
    let a_pct = a_hits as f64 / total as f64 * 100.0;
    let delta = a_hits as i64 - b_hits as i64;
    let delta_pct = a_pct - b_pct;

    println!("────── A/B: PSE-Augmented vs Baseline ─────────────────────────");
    println!();
    println!("  Metric: domain keyword coverage ({total} keywords)");
    println!();
    println!("  Baseline  [{baseline_ms:>5}ms]: {b_hits:>2}/{total} keywords  ({b_pct:.0}%)");
    println!("  Augmented [{augmented_ms:>5}ms]: {a_hits:>2}/{total} keywords  ({a_pct:.0}%)");
    println!();

    if delta > 0 {
        println!(
            "  ✓ PSE augmentation: +{delta} keywords  (+{delta_pct:.0}pp) — \
             DEMONSTRATED"
        );
    } else if delta == 0 {
        println!("  ~ PSE augmentation: no coverage difference on this session.");
        println!("    (Run more sessions — crystal density increases over time.)");
    } else {
        println!(
            "  · PSE augmentation: {delta} keywords  ({delta_pct:.0}pp) — \
             baseline was stronger this session."
        );
    }
    println!();

    // Preview both responses (first 200 chars each)
    let b_preview: String = baseline_response.chars().take(200).collect();
    let a_preview: String = augmented_response.chars().take(200).collect();
    println!("  Baseline  preview : \"{}…\"", b_preview.replace('\n', " "));
    println!("  Augmented preview : \"{}…\"", a_preview.replace('\n', " "));
    println!();
}
