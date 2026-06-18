//! The behavioural lattice — the *executed* cousin of the structural [`alchemy`]
//! lattice.
//!
//! Where alchemy combines structural *profiles* (counts) and dedups by density
//! *similarity*, behaviour combines actual *functions* over a finite domain and
//! dedups by **observational equality after real execution**: the same
//! behaviour, however expressed, collapses to one point — ground truth, not the
//! structural proxy.
//!
//! An element carries real, runnable code — an [`Expr`] over a closed, total
//! value-algebra — and its *table*, the result of running that code on every
//! input of the domain `{0, …, DOMAIN-1}`. Combination is **composition**, and
//! it is genuinely executed: `combine(f, g)` runs `g` then `f`. (The algebra is
//! closed and total on purpose — discovering structure must never mean running
//! untrusted polyglot code, which is unbounded and unsafe.)
//!
//! The payoff is the **bridge** back to structure: every behaviour also carries
//! the [`StructuralCounts`] fingerprint of its expression, so the lattice can
//! measure where the structural proxy *agrees* with executed behaviour and where
//! it fails — **synonyms** (different code, identical behaviour, which the proxy
//! under-merges) and **false friends** (similar structure, divergent behaviour,
//! which the proxy over-merges).
//!
//! [`alchemy`]: crate::alchemy

use crate::alchemy::StructuralCounts;
use kosmo_core::Q16;
use std::collections::HashSet;

/// The finite value domain `D = {0, 1, …, DOMAIN-1}`. Behaviour is a *total*
/// function `D → D`; its table is the observation of running it on all of `D`.
pub const DOMAIN: usize = 12;

/// A piece of runnable code in the closed, total value-algebra over `D`.
///
/// Every variant is total (defined on all inputs) and deterministic, so a
/// behaviour can always be fully observed by evaluation — no partiality, no
/// effects, no escape from the domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    /// `x ↦ x`.
    Identity,
    /// `x ↦ (x + 1) mod DOMAIN`.
    Succ,
    /// `x ↦ (2x) mod DOMAIN`.
    Double,
    /// `x ↦ k mod DOMAIN` — a constant, ignoring its input.
    Const(u8),
    /// `f ∘ g`: run `g`, then `f`.
    Compose(Box<Expr>, Box<Expr>),
}

impl Expr {
    /// Run the code on one input — real, total evaluation.
    pub fn eval(&self, x: u8) -> u8 {
        let n = DOMAIN as u8;
        match self {
            Expr::Identity => x % n,
            Expr::Succ => (x + 1) % n,
            Expr::Double => (x.wrapping_mul(2)) % n,
            Expr::Const(k) => k % n,
            Expr::Compose(f, g) => f.eval(g.eval(x)),
        }
    }

    /// The structural fingerprint of the *expression* (not its behaviour): how
    /// many of each primitive it is built from. The four axes are the primitive
    /// kinds — step (`Succ`), scale (`Double`), constant (`Const`), and glue
    /// (`Compose`) — reusing [`StructuralCounts`] so the bridge can compare an
    /// expression's *shape* against the structural lattice's own measure.
    pub fn op_counts(&self) -> StructuralCounts {
        match self {
            Expr::Identity => StructuralCounts::default(),
            Expr::Succ => StructuralCounts { functions: 1, ..Default::default() },
            Expr::Double => StructuralCounts { types: 1, ..Default::default() },
            Expr::Const(_) => StructuralCounts { imports: 1, ..Default::default() },
            Expr::Compose(f, g) => {
                let a = f.op_counts();
                let b = g.op_counts();
                StructuralCounts {
                    functions: a.functions + b.functions,
                    types: a.types + b.types,
                    imports: a.imports + b.imports,
                    tests: a.tests + b.tests + 1,
                }
            }
        }
    }
}

/// A behaviour: runnable code paired with its fully-observed truth.
#[derive(Clone, Debug)]
pub struct Behaviour {
    /// A human-readable rendering of the expression.
    pub name: String,
    /// The runnable code.
    pub expr: Expr,
    /// `table[x] = expr.eval(x)` for every `x` in the domain — the executed
    /// observation that *is* the behaviour's identity.
    pub table: [u8; DOMAIN],
    /// The structural fingerprint of `expr` (its shape, for the bridge).
    pub fingerprint: StructuralCounts,
}

impl Behaviour {
    /// Build a behaviour from code, executing it across the whole domain.
    pub fn from_expr(name: impl Into<String>, expr: Expr) -> Self {
        let mut table = [0u8; DOMAIN];
        for (x, slot) in table.iter_mut().enumerate() {
            *slot = expr.eval(x as u8);
        }
        let fingerprint = expr.op_counts();
        Self {
            name: name.into(),
            expr,
            table,
            fingerprint,
        }
    }

    /// Is this behaviour *informative* — does its output depend on its input?
    /// A constant transmits nothing; it is the behavioural void, the executed
    /// cousin of [`StructuralCounts::defines_substance`]. Unlike structural
    /// substance, this is **not** closed under combination: composing two
    /// informative functions can collapse to a constant.
    pub fn is_informative(&self) -> bool {
        self.table.iter().any(|&v| v != self.table[0])
    }

    /// Observational equality: the same input→output map, however expressed.
    /// This is the ground truth the structural similarity only approximates.
    pub fn behaves_like(&self, other: &Behaviour) -> bool {
        self.table == other.table
    }
}

/// The four generators of the value-algebra — the behavioural primitives.
pub fn generators() -> Vec<Behaviour> {
    vec![
        Behaviour::from_expr("id", Expr::Identity),
        Behaviour::from_expr("succ", Expr::Succ),
        Behaviour::from_expr("double", Expr::Double),
        Behaviour::from_expr("zero", Expr::Const(0)),
    ]
}

/// Compose two behaviours — `a ∘ b`, run `b` then `a`. Genuinely executed: the
/// offspring's table is the composition of the parents' tables.
pub fn compose(a: &Behaviour, b: &Behaviour) -> Behaviour {
    let expr = Expr::Compose(Box::new(a.expr.clone()), Box::new(b.expr.clone()));
    Behaviour::from_expr(format!("{}∘{}", a.name, b.name), expr)
}

/// What happened when a candidate met the lattice's gates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Admission {
    /// A new behaviour — admitted.
    Discovered,
    /// Its table was already held — a synonym (different code, same behaviour).
    Synonym,
    /// Turned away by the informativeness gate (a constant).
    Constant,
}

/// The behavioural lattice: a set of behaviours closed (to a fixpoint) under
/// composition, deduped by executed observational equality. Saturation discovers
/// the **transformation monoid** generated by the seeds — exact, finite algebra.
#[derive(Clone, Debug)]
pub struct Monoid {
    elements: Vec<Behaviour>,
    seen: HashSet<[u8; DOMAIN]>,
    require_informative: bool,
    explored: u64,
    synonyms: u64,
    rejected_constant: u64,
}

impl Monoid {
    /// An empty lattice. With `require_informative`, the validity gate is armed:
    /// a constant behaviour (the behavioural void) is never admitted.
    pub fn new(require_informative: bool) -> Self {
        Self {
            elements: Vec::new(),
            seen: HashSet::new(),
            require_informative,
            explored: 0,
            synonyms: 0,
            rejected_constant: 0,
        }
    }

    /// Seed with `gens`, admitting each novel (and, if armed, informative) one.
    pub fn seeded(gens: impl IntoIterator<Item = Behaviour>, require_informative: bool) -> Self {
        let mut m = Self::new(require_informative);
        for g in gens {
            m.admit(g);
        }
        m
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn elements(&self) -> &[Behaviour] {
        &self.elements
    }

    /// Total compositions executed across all rounds.
    pub fn explored(&self) -> u64 {
        self.explored
    }

    /// How many compositions landed on a behaviour already held — **synonyms**:
    /// different code, identical executed behaviour. The exact count the
    /// structural proxy would have to recover to match ground truth.
    pub fn synonyms(&self) -> u64 {
        self.synonyms
    }

    /// How many candidates the informativeness gate turned away (constants).
    pub fn rejected_constant(&self) -> u64 {
        self.rejected_constant
    }

    fn admit(&mut self, cand: Behaviour) -> Admission {
        if self.require_informative && !cand.is_informative() {
            self.rejected_constant += 1;
            return Admission::Constant;
        }
        if self.seen.contains(&cand.table) {
            self.synonyms += 1;
            return Admission::Synonym;
        }
        self.seen.insert(cand.table);
        self.elements.push(cand);
        Admission::Discovered
    }

    /// One round: compose every ordered pair (composition is not commutative)
    /// of the behaviours held at the round's start, admitting novel offspring.
    /// Returns the number of discoveries.
    pub fn expand_once(&mut self) -> usize {
        let snapshot = self.elements.clone();
        let mut discoveries = 0;
        for a in &snapshot {
            for b in &snapshot {
                self.explored += 1;
                if self.admit(compose(a, b)) == Admission::Discovered {
                    discoveries += 1;
                }
            }
        }
        discoveries
    }

    /// Drive composition to a fixpoint (the generated monoid), bounded by
    /// `max_rounds` and `cap`. Returns total discoveries.
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

    /// The bridge to structure: among the held behaviours — all of which are
    /// behaviourally *distinct* by construction — how many pairs are
    /// structurally *similar* at or above `tau`? Those are **false friends**:
    /// the structural proxy would wrongly merge them. Returns
    /// `(false_friends, total_pairs)`.
    pub fn false_friends(&self, tau: Q16) -> (usize, usize) {
        let mut friends = 0;
        let mut pairs = 0;
        for i in 0..self.elements.len() {
            for j in (i + 1)..self.elements.len() {
                pairs += 1;
                let sim = self.elements[i]
                    .fingerprint
                    .similarity(self.elements[j].fingerprint);
                if sim.raw() >= tau.raw() {
                    friends += 1;
                }
            }
        }
        (friends, pairs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_is_real_and_total() {
        assert_eq!(Expr::Succ.eval(11), 0, "succ wraps the domain");
        assert_eq!(Expr::Double.eval(7), 2, "2*7 = 14 = 2 mod 12");
        assert_eq!(Expr::Const(5).eval(9), 5, "const ignores input");
        let comp = Expr::Compose(Box::new(Expr::Succ), Box::new(Expr::Double));
        assert_eq!(comp.eval(7), 3, "succ(double(7)) = succ(2) = 3");
    }

    #[test]
    fn composition_is_not_commutative() {
        // succ∘double ≠ double∘succ in general.
        let sd = compose(
            &Behaviour::from_expr("succ", Expr::Succ),
            &Behaviour::from_expr("double", Expr::Double),
        );
        let ds = compose(
            &Behaviour::from_expr("double", Expr::Double),
            &Behaviour::from_expr("succ", Expr::Succ),
        );
        assert_ne!(sd.table, ds.table, "order matters under composition");
    }

    #[test]
    fn same_behaviour_however_expressed_is_one_point() {
        // double ≡ succ∘succ only at... no: build a genuine synonym.
        // x+1+1 (succ∘succ) equals "add 2"; double is 2x — different. Use a real
        // synonym: id∘succ behaves exactly like succ, though its code differs.
        let succ = Behaviour::from_expr("succ", Expr::Succ);
        let id_succ = compose(&Behaviour::from_expr("id", Expr::Identity), &succ);
        assert_ne!(id_succ.expr, succ.expr, "different code");
        assert!(id_succ.behaves_like(&succ), "identical executed behaviour");
    }

    #[test]
    fn the_lattice_dedups_synonyms_by_execution() {
        let mut m = Monoid::seeded(generators(), false);
        let seeded = m.len();
        m.saturate(32, 4096);
        // Composition explored far more than it discovered: the surplus is
        // synonyms — different code collapsing to a held behaviour.
        assert!(m.len() >= seeded);
        assert!(
            m.synonyms() > 0,
            "executed dedup collapses distinct code to one behaviour"
        );
        // Every held behaviour is observationally unique.
        let mut tables: Vec<_> = m.elements().iter().map(|b| b.table).collect();
        tables.sort();
        tables.dedup();
        assert_eq!(tables.len(), m.len(), "no two held behaviours coincide");
    }

    #[test]
    fn informativeness_gate_rejects_constants_even_as_offspring() {
        // double∘double∘… eventually reaches a constant on Z_12? zero is the
        // obvious constant seed; with the gate armed it never enters, and any
        // composition through it (which would be constant) is rejected too.
        let mut armed = Monoid::seeded(generators(), true);
        assert!(
            armed.rejected_constant() >= 1,
            "the constant generator `zero` is turned away at seeding"
        );
        let before = armed.len();
        armed.saturate(32, 4096);
        for b in armed.elements() {
            assert!(b.is_informative(), "no constant survives in an armed lattice");
        }
        assert!(armed.len() >= before);
    }

    #[test]
    fn the_bridge_finds_false_friends() {
        // Structurally-similar expressions that behave differently must exist:
        // Const(0) and Const(3) share a fingerprint (one `Const` op) but behave
        // differently. Confirm the structural measure cannot tell them apart.
        let a = Behaviour::from_expr("zero", Expr::Const(0));
        let b = Behaviour::from_expr("three", Expr::Const(3));
        assert_eq!(a.fingerprint, b.fingerprint, "identical structure");
        assert_ne!(a.table, b.table, "different behaviour");
        assert_eq!(
            a.fingerprint.similarity(b.fingerprint),
            Q16::ONE,
            "the structural proxy would wrongly merge them"
        );
    }
}
