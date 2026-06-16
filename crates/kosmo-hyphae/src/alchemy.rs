//! The combine operator — the alchemy / "Doodle God" move over structural
//! elements.
//!
//! An [`Element`] is a language-blind bundle of structural counts (functions,
//! types, imports, tests) — the same basis as
//! [`crate::xlang::CrossLanguageFingerprint`]. [`combine`] superposes two
//! elements into a third by *pooling* their counts: the alchemical `(a, b) → c`.
//! An [`Inventory`] admits an offspring only when it is **novel** — no held
//! element within a similarity threshold (the "is this a new element, or one I
//! already have?" test, reusing the CROSS-007 structural similarity). Driving
//! combination to a fixpoint ([`Inventory::saturate`]) grows a reachable catalog
//! from a few primitives, the unreached space its frontier.
//!
//! This is the **novelty** gate only — the discovery loop's engine. Whether an
//! offspring is also *valid* (the assimilation/crystal certification cascade) or
//! *behaviourally* distinct (executed equivalence) are further gates, layered on
//! top, not duplicated here.

use kosmo_core::Q16;

/// The four structural kinds an [`Element`] counts — the basis of CROSS-007.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StructuralCounts {
    pub functions: u64,
    pub types: u64,
    pub imports: u64,
    pub tests: u64,
}

impl StructuralCounts {
    /// Total structural observations.
    pub fn total(self) -> u64 {
        self.functions + self.types + self.imports + self.tests
    }

    /// Pool two bundles — the substance of [`combine`]. Commutative and
    /// associative: superposition never loses substance.
    pub fn pooled(self, other: Self) -> Self {
        Self {
            functions: self.functions + other.functions,
            types: self.types + other.types,
            imports: self.imports + other.imports,
            tests: self.tests + other.tests,
        }
    }

    /// The four density axes `count / total` (CROSS-007); all `ZERO` for an
    /// empty bundle.
    pub fn densities(self) -> [Q16; 4] {
        let total = self.total();
        let density = |count: u64| {
            if total == 0 {
                Q16::ZERO
            } else {
                Q16::ratio(count, total).unwrap_or(Q16::ZERO)
            }
        };
        [
            density(self.functions),
            density(self.types),
            density(self.imports),
            density(self.tests),
        ]
    }

    /// Language-independent structural similarity in `[0, 1]` — `1 − mean|Δdensity|`
    /// over the four axes, the same fixed-point formula as
    /// [`crate::xlang::CrossLanguageFingerprint::similarity`] (CROSS-007). Two
    /// bundles with the same *proportions* are identical (`ONE`), however large.
    pub fn similarity(self, other: Self) -> Q16 {
        let a = self.densities();
        let b = other.densities();
        let mut sum_diff_raw: i64 = 0;
        for (x, y) in a.iter().zip(b.iter()) {
            sum_diff_raw = sum_diff_raw.saturating_add((*x).saturating_sub(*y).abs().raw());
        }
        let mean_diff = Q16::from_raw(sum_diff_raw / 4);
        Q16::ONE.saturating_sub(mean_diff).abs()
    }
}

/// A named structural element in the alchemy inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub counts: StructuralCounts,
}

impl Element {
    pub fn new(name: impl Into<String>, counts: StructuralCounts) -> Self {
        Self {
            name: name.into(),
            counts,
        }
    }

    /// Structural similarity to another element (CROSS-007).
    pub fn similarity(&self, other: &Element) -> Q16 {
        self.counts.similarity(other.counts)
    }
}

/// The four primitive elements — the basis vectors of the structural simplex, the
/// "earth/air/fire/water" of the lab.
pub fn primitives() -> Vec<Element> {
    vec![
        Element::new(
            "function",
            StructuralCounts {
                functions: 1,
                ..Default::default()
            },
        ),
        Element::new(
            "type",
            StructuralCounts {
                types: 1,
                ..Default::default()
            },
        ),
        Element::new(
            "import",
            StructuralCounts {
                imports: 1,
                ..Default::default()
            },
        ),
        Element::new(
            "test",
            StructuralCounts {
                tests: 1,
                ..Default::default()
            },
        ),
    ]
}

/// The combine move: superpose two elements into a third by pooling their counts
/// — the alchemical `(a, b) → c`. Commutative; the offspring is named `a+b`.
pub fn combine(a: &Element, b: &Element) -> Element {
    Element::new(format!("{}+{}", a.name, b.name), a.counts.pooled(b.counts))
}

/// The discovery inventory: a set of elements that admits a candidate only when
/// it is *novel* (no held element within `threshold` similarity). Combination
/// over its pairs is the discovery loop; the catalog is what it has reached.
#[derive(Debug, Clone)]
pub struct Inventory {
    elements: Vec<Element>,
    threshold: Q16,
}

impl Inventory {
    /// An empty inventory whose novelty gate rejects a candidate at or above
    /// `threshold` similarity to a held element.
    pub fn new(threshold: Q16) -> Self {
        Self {
            elements: Vec::new(),
            threshold,
        }
    }

    /// Seed with `prims`, admitting each as novel (deduping the seeds themselves).
    pub fn seeded(prims: impl IntoIterator<Item = Element>, threshold: Q16) -> Self {
        let mut inv = Self::new(threshold);
        for p in prims {
            inv.admit(p);
        }
        inv
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn elements(&self) -> &[Element] {
        &self.elements
    }

    /// Is `cand` novel — no held element at or above `threshold` similarity? The
    /// empty bundle (the void) is never an element.
    pub fn is_novel(&self, cand: &Element) -> bool {
        if cand.counts.total() == 0 {
            return false;
        }
        !self
            .elements
            .iter()
            .any(|e| e.similarity(cand).raw() >= self.threshold.raw())
    }

    /// Admit `cand` iff novel; `true` when it was a discovery (added).
    pub fn admit(&mut self, cand: Element) -> bool {
        if self.is_novel(&cand) {
            self.elements.push(cand);
            true
        } else {
            false
        }
    }

    /// One round of discovery: combine every unordered pair of the elements held
    /// at the round's start (including an element with itself) and admit each
    /// novel offspring. Returns the number of discoveries. Deterministic (pairs
    /// in index order); offspring are deduped against each other within the round.
    pub fn expand_once(&mut self) -> usize {
        let snapshot = self.elements.clone();
        let mut discoveries = 0;
        for i in 0..snapshot.len() {
            for j in i..snapshot.len() {
                if self.admit(combine(&snapshot[i], &snapshot[j])) {
                    discoveries += 1;
                }
            }
        }
        discoveries
    }

    /// Drive combination to a fixpoint: repeat [`expand_once`] until a round
    /// discovers nothing, or `max_rounds` pass, or the catalog reaches `cap`
    /// elements. Returns total discoveries. Bounded — the catalog cannot grow
    /// without end (the similarity threshold quantizes the structural simplex).
    pub fn saturate(&mut self, max_rounds: usize, cap: usize) -> usize {
        let mut total = 0;
        for _ in 0..max_rounds {
            if self.elements.len() >= cap {
                break;
            }
            let found = self.expand_once();
            total += found;
            if found == 0 {
                break;
            }
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thresh(num: u64, den: u64) -> Q16 {
        Q16::ratio(num, den).unwrap()
    }

    fn elem(name: &str, f: u64, t: u64, i: u64, te: u64) -> Element {
        Element::new(
            name,
            StructuralCounts {
                functions: f,
                types: t,
                imports: i,
                tests: te,
            },
        )
    }

    #[test]
    fn combine_pools_counts_and_is_commutative() {
        let f = elem("function", 1, 0, 0, 0);
        let t = elem("type", 0, 1, 0, 0);
        let ab = combine(&f, &t);
        let ba = combine(&t, &f);
        assert_eq!(ab.counts, StructuralCounts { functions: 1, types: 1, imports: 0, tests: 0 });
        assert_eq!(ab.counts.total(), 2, "superposition pools substance");
        assert_eq!(ab.counts, ba.counts, "combine is commutative on substance");
    }

    #[test]
    fn proportional_bundles_are_identical_larger_is_the_same_element() {
        // fire + fire = fire: same proportions, similarity ONE.
        let f = elem("function", 1, 0, 0, 0);
        let ff = combine(&f, &f); // (2,0,0,0)
        assert_eq!(f.similarity(&ff), Q16::ONE, "same proportions → identical");
    }

    #[test]
    fn rediscovery_is_rejected_but_a_true_mixture_is_admitted() {
        let f = elem("function", 1, 0, 0, 0);
        let t = elem("type", 0, 1, 0, 0);
        let mut inv = Inventory::seeded([f.clone(), t.clone()], thresh(9, 10));
        assert_eq!(inv.len(), 2, "two distinct primitives admitted");
        // function + function ≡ function → not novel (rediscovery).
        assert!(!inv.is_novel(&combine(&f, &f)), "fire + fire = fire is no discovery");
        assert!(!inv.admit(combine(&f, &f)), "rediscovery is not admitted");
        // function + type is a genuinely new mixture → admitted.
        assert!(inv.is_novel(&combine(&f, &t)), "the mixture is novel");
        assert!(inv.admit(combine(&f, &t)), "the mixture is admitted as a discovery");
        assert_eq!(inv.len(), 3, "the catalog grew by exactly the discovery");
    }

    #[test]
    fn saturation_grows_a_catalog_from_primitives_and_terminates() {
        let mut inv = Inventory::seeded(primitives(), thresh(9, 10));
        let before = inv.len();
        let discovered = inv.saturate(16, 256);
        assert_eq!(before, 4, "four primitives");
        assert!(discovered > 0, "combination discovers new elements");
        assert!(inv.len() > before, "the catalog grew beyond the primitives");
        // Fixpoint: a further saturation discovers nothing more.
        assert_eq!(inv.saturate(16, 256), 0, "the catalog has closed (a fixpoint)");
    }

    #[test]
    fn saturation_is_deterministic() {
        let mut a = Inventory::seeded(primitives(), thresh(9, 10));
        let mut b = Inventory::seeded(primitives(), thresh(9, 10));
        a.saturate(16, 256);
        b.saturate(16, 256);
        assert_eq!(a.len(), b.len(), "the discovery loop is deterministic");
    }

    #[test]
    fn a_finer_threshold_discovers_a_larger_catalog() {
        let mut coarse = Inventory::seeded(primitives(), thresh(8, 10));
        let mut fine = Inventory::seeded(primitives(), thresh(97, 100));
        coarse.saturate(24, 1024);
        fine.saturate(24, 1024);
        assert!(
            fine.len() >= coarse.len(),
            "a finer novelty threshold resolves more distinct elements: fine={} coarse={}",
            fine.len(),
            coarse.len()
        );
    }
}
