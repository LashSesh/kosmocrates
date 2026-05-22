//! Crystal Context Renderer — bridges PSE topology → LLM-injectable text.
//!
//! Takes `CrystalRecord`s (crystal + source chunks) and produces a compact
//! prompt block that can be prepended to an LLM system prompt.  The renderer
//! also provides a keyword-based coverage scorer for the A/B comparison.

use crate::memory::CrystalRecord;

// Default domain: cognitive architectures (ACT-R, SOAR, Global Workspace Theory).
//
// This domain is chosen because small LLMs (7-8B params) have shallow knowledge
// of specific mechanisms — chunking, subsymbolic activation, spreading activation,
// conflict resolution strategies — so PSE context from prior sessions measurably
// lifts coverage.  Override via PSE_LLM_KEYWORDS (comma-separated).
const DEFAULT_KEYWORDS: &[&str] = &[
    "actr",
    "soar",
    "chunking",
    "declarative",
    "procedural",
    "activation",
    "spreading",
    "retrieval",
    "production",
    "conflict resolution",
    "working memory",
    "global workspace",
    "subsymbolic",
    "base-level",
    "imaginal",
    "metacognition",
    "ida",
    "lida",
    "unified theory",
    "cognitive architecture",
];

/// Return the active keyword list.
/// Reads `PSE_LLM_KEYWORDS` (comma-separated) if set; otherwise uses defaults.
pub fn domain_keywords() -> Vec<String> {
    if let Ok(kw) = std::env::var("PSE_LLM_KEYWORDS") {
        kw.split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        DEFAULT_KEYWORDS.iter().map(|s| s.to_string()).collect()
    }
}

/// Render the top-k most stable crystal records into an LLM-injectable string.
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
pub fn score_coverage(response: &str, keywords: &[String]) -> (usize, usize) {
    let lower = response.to_lowercase();
    let hits = keywords.iter().filter(|k| lower.contains(k.as_str())).count();
    (hits, keywords.len())
}

/// Print a side-by-side A/B report to stdout.
pub fn print_ab_report(
    baseline_response: &str,
    augmented_response: &str,
    baseline_ms: u128,
    augmented_ms: u128,
    keywords: &[String],
) {
    let (b_hits, total) = score_coverage(baseline_response, keywords);
    let (a_hits, _) = score_coverage(augmented_response, keywords);

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

    if b_pct > 65.0 {
        println!("  ⚠ Baseline coverage already {b_pct:.0}% — the LLM knows this domain well.");
        println!("    PSE gain is limited when the model's prior is strong.");
        println!("    Use PSE_LLM_QUESTIONS_FILE + PSE_LLM_KEYWORDS for a specialised domain.");
        println!();
    }

    if delta > 0 {
        println!(
            "  ✓ PSE augmentation: +{delta} keywords  (+{delta_pct:.0}pp) — DEMONSTRATED"
        );
    } else if delta == 0 {
        println!("  ~ PSE augmentation: no coverage difference on this session.");
        println!("    Crystal density grows over sessions — run more to accumulate.");
    } else {
        println!(
            "  · PSE augmentation: {delta} keywords  ({delta_pct:.0}pp) — \
             baseline was stronger this session."
        );
    }
    println!();

    let b_preview: String = baseline_response.chars().take(200).collect();
    let a_preview: String = augmented_response.chars().take(200).collect();
    println!("  Baseline  preview : \"{}…\"", b_preview.replace('\n', " "));
    println!("  Augmented preview : \"{}…\"", a_preview.replace('\n', " "));
    println!();
}
