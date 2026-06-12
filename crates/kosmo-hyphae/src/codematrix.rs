//! 5D code fingerprint — assimilated from Wonderlamp's `codematrix.rs`,
//! re-expressed on the epistemic substrate.
//!
//! Wonderlamp measured five qualities of a generated file in `f64` with
//! path-bound layer inference; here the same five axes are integer `Q16`
//! over the language-agnostic [`xlang`](crate::xlang) classifier pass —
//! content-addressed, deterministic, and **strictly advisory**: the
//! fingerprint ranks and informs (consensus richness, optional ranking
//! dimensions), it never appears in a `GateResult` path (CROSS-010).
//!
//! Axes:
//! - `r_richness` — structural observations per non-empty-line band
//!   (saturates at one observation per [`RICHNESS_BAND`] lines).
//! - `f_functions` / `t_types` — the share of structure that is functions /
//!   types (simplex-style, like the 4D
//!   [`CrossLanguageFingerprint`](crate::xlang::CrossLanguageFingerprint)).
//! - `s_structure` — delimiter discipline: the fraction of `()`/`[]`/`{}`
//!   families that balance over the whole text.
//! - `e_errors` — error-propagation observations per function, capped at
//!   `ONE` (a file without functions honestly scores `ZERO`).
//!
//! Resonance between two fingerprints is the [`Q16::geomean`] of per-axis
//! similarities — the geometric mean's soft unanimity makes one maximally
//! disagreeing axis silence the whole comparison, mirroring the consensus
//! doctrine used everywhere else in the system.

use kosmo_core::{Digest, Q16};
use serde::{Deserialize, Serialize};

use crate::xlang::{symbol_sets, SourceLanguage, SymbolSets};

/// One structural observation per this many non-empty lines saturates the
/// richness axis. A documented heuristic constant — advisory math only.
pub const RICHNESS_BAND: u32 = 4;

/// Serialize-only content for [`CodeMatrixFingerprint`] addressing.
#[derive(Serialize)]
struct CodeMatrixContent {
    language: String,
    r_richness: i64,
    f_functions: i64,
    t_types: i64,
    s_structure: i64,
    e_errors: i64,
    source_evidence_id: Digest,
}

/// A content-addressed, `Q16`-only 5D fingerprint of one source text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CodeMatrixFingerprint {
    pub fingerprint_id: Digest,
    pub language: SourceLanguage,
    pub r_richness: Q16,
    pub f_functions: Q16,
    pub t_types: Q16,
    pub s_structure: Q16,
    pub e_errors: Q16,
    pub source_evidence_id: Digest,
}

impl CodeMatrixFingerprint {
    /// Fingerprint `content` in a known `language`.
    pub fn from_source(
        language: SourceLanguage,
        source_evidence_id: Digest,
        content: &str,
    ) -> Self {
        let sets = symbol_sets(language, content);
        let lines = content.lines().filter(|l| !l.trim().is_empty()).count() as u64;

        let r_richness = if lines == 0 {
            Q16::ZERO
        } else {
            let banded = (sets.total as u64).saturating_mul(RICHNESS_BAND as u64);
            Q16::ratio(banded.min(lines), lines).unwrap_or(Q16::ZERO)
        };
        let total = sets.total.max(1) as u64;
        let f_functions = Q16::ratio(sets.functions.len() as u64, total).unwrap_or(Q16::ZERO);
        let t_types = Q16::ratio(sets.types.len() as u64, total).unwrap_or(Q16::ZERO);
        let s_structure = delimiter_discipline(content);
        let fns = sets.functions.len() as u64;
        let e_errors = if fns == 0 {
            Q16::ZERO
        } else {
            Q16::ratio((sets.error_propagations as u64).min(fns), fns).unwrap_or(Q16::ZERO)
        };

        Self::assemble(
            language,
            source_evidence_id,
            r_richness,
            f_functions,
            t_types,
            s_structure,
            e_errors,
        )
    }

    /// Detect the language from `location` and fingerprint, or `None` for an
    /// unrecognised extension (fail-closed, like `extract_auto`).
    pub fn from_auto(source_evidence_id: Digest, location: &str, content: &str) -> Option<Self> {
        SourceLanguage::from_path(location)
            .map(|lang| Self::from_source(lang, source_evidence_id, content))
    }

    fn assemble(
        language: SourceLanguage,
        source_evidence_id: Digest,
        r_richness: Q16,
        f_functions: Q16,
        t_types: Q16,
        s_structure: Q16,
        e_errors: Q16,
    ) -> Self {
        let fingerprint_id = Digest::of(&CodeMatrixContent {
            language: format!("{language:?}"),
            r_richness: r_richness.raw(),
            f_functions: f_functions.raw(),
            t_types: t_types.raw(),
            s_structure: s_structure.raw(),
            e_errors: e_errors.raw(),
            source_evidence_id,
        });
        Self {
            fingerprint_id,
            language,
            r_richness,
            f_functions,
            t_types,
            s_structure,
            e_errors,
            source_evidence_id,
        }
    }

    /// Recompute the content address and compare (tamper check).
    pub fn verify_id(&self) -> bool {
        let recomputed = Self::assemble(
            self.language,
            self.source_evidence_id,
            self.r_richness,
            self.f_functions,
            self.t_types,
            self.s_structure,
            self.e_errors,
        );
        recomputed.fingerprint_id == self.fingerprint_id
    }

    /// The five axes in canonical order.
    pub fn axes(&self) -> [Q16; 5] {
        [
            self.r_richness,
            self.f_functions,
            self.t_types,
            self.s_structure,
            self.e_errors,
        ]
    }

    /// Structural resonance with another fingerprint: the geometric mean of
    /// per-axis similarities `1 − |Δ|`. Self-resonance is `ONE`; one
    /// maximally disagreeing axis silences the comparison (soft unanimity).
    /// Advisory only — never an input to a gate (CROSS-010).
    pub fn resonance(&self, other: &Self) -> Q16 {
        let sims: Vec<Q16> = self
            .axes()
            .iter()
            .zip(other.axes().iter())
            .map(|(a, b)| Q16::ONE.saturating_sub(a.saturating_sub(*b).abs()))
            .collect();
        Q16::geomean(&sims)
    }

    /// Mean richness of the fingerprint itself — the `ρ` (content richness)
    /// input the swarm-consensus scoring consumes.
    pub fn richness(&self) -> Q16 {
        self.r_richness
    }
}

/// The fraction of the three delimiter families that balance over `content`.
fn delimiter_discipline(content: &str) -> Q16 {
    let mut counts = [(0i64, 0i64); 3]; // (open, close) for (), [], {}
    for ch in content.chars() {
        match ch {
            '(' => counts[0].0 += 1,
            ')' => counts[0].1 += 1,
            '[' => counts[1].0 += 1,
            ']' => counts[1].1 += 1,
            '{' => counts[2].0 += 1,
            '}' => counts[2].1 += 1,
            _ => {}
        }
    }
    let balanced = counts.iter().filter(|(open, close)| open == close).count() as u64;
    Q16::ratio(balanced, 3).unwrap_or(Q16::ZERO)
}

/// Convenience: pooled [`SymbolSets`] over many `(location, content)` pairs —
/// the per-candidate feature basis the consensus layer builds on. Locations
/// with unrecognised extensions are skipped (not everything is code).
pub fn pooled_symbol_sets<'a>(
    files: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> (SymbolSets, u32) {
    let mut pooled = SymbolSets::default();
    let mut recognized = 0u32;
    for (location, content) in files {
        if let Some(lang) = SourceLanguage::from_path(location) {
            recognized += 1;
            let sets = symbol_sets(lang, content);
            pooled.functions.extend(sets.functions);
            pooled.types.extend(sets.types);
            pooled.imports.extend(sets.imports);
            pooled.tests.extend(sets.tests);
            pooled.error_propagations += sets.error_propagations;
            pooled.total += sets.total;
        }
    }
    (pooled, recognized)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    const RUST_SRC: &str = "\
use std::collections::BTreeMap;

pub fn route(path: &str) -> Result<u32, String> {
    let n = path.len() as u32;
    Ok(n)
}

pub struct Router {
    table: BTreeMap<String, u32>,
}
";

    #[test]
    fn fingerprint_is_content_addressed_and_deterministic() {
        let a = CodeMatrixFingerprint::from_source(SourceLanguage::Rust, d(b"ev"), RUST_SRC);
        let b = CodeMatrixFingerprint::from_source(SourceLanguage::Rust, d(b"ev"), RUST_SRC);
        assert_eq!(a, b, "same input, same fingerprint");
        assert!(a.verify_id());
        let mut tampered = a.clone();
        tampered.e_errors = tampered.e_errors.saturating_add(Q16::ratio(1, 8).unwrap());
        assert!(!tampered.verify_id(), "tampering breaks the address");
    }

    #[test]
    fn axes_live_on_the_unit_interval() {
        let fp = CodeMatrixFingerprint::from_source(SourceLanguage::Rust, d(b"ev"), RUST_SRC);
        for axis in fp.axes() {
            assert!(axis >= Q16::ZERO && axis <= Q16::ONE, "{axis:?}");
        }
        assert!(fp.f_functions.is_positive(), "the fn is seen");
        assert!(fp.t_types.is_positive(), "the struct is seen");
        assert_eq!(fp.s_structure, Q16::ONE, "balanced delimiters");
    }

    #[test]
    fn resonance_self_is_one_and_symmetric() {
        let a = CodeMatrixFingerprint::from_source(SourceLanguage::Rust, d(b"ev"), RUST_SRC);
        let py = "import json\n\ndef handle(payload):\n    return json.loads(payload)\n";
        let b = CodeMatrixFingerprint::from_source(SourceLanguage::Python, d(b"ev"), py);
        assert_eq!(a.resonance(&a), Q16::ONE);
        assert_eq!(a.resonance(&b), b.resonance(&a), "symmetry");
        assert!(a.resonance(&b) < Q16::ONE);
    }

    #[test]
    fn from_auto_is_fail_closed_on_unknown_extensions() {
        assert!(CodeMatrixFingerprint::from_auto(d(b"ev"), "Cargo.toml", "[package]").is_none());
        assert!(CodeMatrixFingerprint::from_auto(d(b"ev"), "src/lib.rs", RUST_SRC).is_some());
    }

    #[test]
    fn pooled_sets_skip_unrecognized_and_union_names() {
        let files = [
            ("src/lib.rs", RUST_SRC),
            ("lib/handlers.py", "def handle(x):\n    return x\n"),
            ("README.md", "# not code"),
        ];
        let (pooled, recognized) = pooled_symbol_sets(files.iter().map(|(l, c)| (*l, *c)));
        assert_eq!(recognized, 2);
        assert!(pooled.functions.contains("route/1"));
        assert!(pooled.functions.contains("handle/1"));
        assert!(pooled.types.contains("Router"));
    }
}
