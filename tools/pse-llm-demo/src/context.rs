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

/// True if `keyword` appears in `text` at a word boundary on both sides.
/// A boundary is: start/end of string, or a non-alphabetic character.
/// This prevents "actr" matching "reactor", "production" matching "reproduction", etc.
fn keyword_at_boundary(text: &str, keyword: &str) -> bool {
    let mut start = 0;
    while start < text.len() {
        match text[start..].find(keyword) {
            None => return false,
            Some(rel) => {
                let pos = start + rel;
                let pre_ok = pos == 0
                    || !text[..pos]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_alphabetic());
                let end = pos + keyword.len();
                let post_ok = end >= text.len()
                    || !text[end..]
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_alphabetic());
                if pre_ok && post_ok {
                    return true;
                }
                start = pos + 1;
            }
        }
    }
    false
}

/// Count how many domain keywords appear at word boundaries (case-insensitive).
/// Returns `(hits, total)`.
pub fn score_coverage(response: &str, keywords: &[String]) -> (usize, usize) {
    let lower = response.to_lowercase();
    let hits = keywords
        .iter()
        .filter(|k| keyword_at_boundary(&lower, k))
        .count();
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
    println!(
        "  (timing reflects wall-clock network latency — not a meaningful performance metric)"
    );
    println!();

    if b_pct > 65.0 {
        println!("  ⚠ Baseline coverage already {b_pct:.0}% — the LLM knows this domain well.");
        println!("    PSE gain is limited when the model's prior is strong.");
        println!("    Use PSE_LLM_QUESTIONS_FILE + PSE_LLM_KEYWORDS for a specialised domain.");
        println!();
    }

    if delta > 0 {
        println!("  ✓ PSE augmentation: +{delta} keyword(s)  (+{delta_pct:.0}pp) on this run.");
        println!("    Coverage gap grows with crystal density — more sessions → stronger signal.");
    } else if delta == 0 {
        println!("  ~ PSE augmentation: no coverage difference on this session.");
        println!("    Crystal density grows over sessions — run more to accumulate.");
    } else {
        println!(
            "  · PSE augmentation: {delta} keyword(s)  ({delta_pct:.0}pp) — \
             baseline was stronger this run."
        );
        println!("    A single session is not conclusive; crystal density grows over time.");
    }
    println!();

    let b_preview: String = baseline_response.chars().take(200).collect();
    let a_preview: String = augmented_response.chars().take(200).collect();
    println!(
        "  Baseline  preview : \"{}…\"",
        b_preview.replace('\n', " ")
    );
    println!(
        "  Augmented preview : \"{}…\"",
        a_preview.replace('\n', " ")
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_match_exact_word() {
        assert!(keyword_at_boundary("uses actr in simulation", "actr"));
        assert!(keyword_at_boundary("actr models declarative", "actr"));
        assert!(keyword_at_boundary("study of actr", "actr"));
    }

    #[test]
    fn boundary_no_match_substring() {
        // "actr" inside "reactor" — must not match
        assert!(!keyword_at_boundary("the reactor is hot", "actr"));
        // "production" inside "reproduction"
        assert!(!keyword_at_boundary("involves reproduction", "production"));
        // "ida" inside "candida"
        assert!(!keyword_at_boundary("candida albicans", "ida"));
        // "activation" inside "deactivation"
        assert!(!keyword_at_boundary("deactivation pathway", "activation"));
    }

    #[test]
    fn boundary_match_phrase() {
        assert!(keyword_at_boundary(
            "uses global workspace theory",
            "global workspace"
        ));
        assert!(keyword_at_boundary(
            "conflict resolution in soar",
            "conflict resolution"
        ));
    }

    #[test]
    fn boundary_match_hyphenated() {
        assert!(keyword_at_boundary("base-level activation", "base-level"));
        // "base-level" must not match "base-levelup" (hypothetical)
        assert!(!keyword_at_boundary("base-levelup", "base-level"));
    }

    #[test]
    fn score_coverage_counts_correctly() {
        let keywords: Vec<String> = vec!["actr".into(), "soar".into(), "chunking".into()];
        // reactor should not count for actr; soar should match
        let text = "soar uses chunking from impasses unlike reactor systems";
        let (hits, total) = score_coverage(text, &keywords);
        assert_eq!(total, 3);
        assert_eq!(hits, 2); // soar + chunking, NOT actr (reactor doesn't count)
    }
}
