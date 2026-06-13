//! Venture — a content-addressed web of wishes with dependencies.
//!
//! A [`crate::Wish`] is one measurable target; a [`Venture`] is a whole
//! *system* of them: stages, each carrying a wish, ordered by explicit
//! dependencies. It is the executable counterpart of the substrate's
//! collapse plans — where a collapse plan describes how an observed body
//! could be transformed, a venture declares the body to be *erected*, wish
//! by wish, each stage under the same gates and observations as a single
//! descent.
//!
//! Discipline:
//! - **Fail-closed at construction**: out-of-range dependencies,
//!   self-dependencies and cycles are construction errors — an invalid
//!   venture never exists as a value.
//! - **Deterministic order**: [`Venture::execution_order`] is a fixed
//!   topological order (smallest ready index first), so two runs of the
//!   same venture walk the same staircase.
//! - **Honest failure**: in a [`VentureSession`], a failed stage marks all
//!   its transitive dependents [`StageStanding::Blocked`] — the session
//!   never attempts what cannot stand.
//! - Content-addressed (`venture_id`) and evidence-bound (CROSS-006), like
//!   every durable artifact.
//!
//! This module is pure state: no IO, no observation. The runner descends
//! each ready stage and feeds the outcome back via [`VentureSession::mark`].

use crate::digest::Digest;
use crate::wish::Wish;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};
use std::fmt;

/// One stage of a venture: a wish plus the stages that must be realized
/// before it may be attempted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VentureStage {
    pub label: String,
    pub wish: Wish,
    /// Indices (into the venture's stage list) of prerequisite stages.
    pub after: Vec<usize>,
}

impl VentureStage {
    pub fn new(
        label: impl Into<String>,
        wish: Wish,
        after: impl IntoIterator<Item = usize>,
    ) -> Self {
        let mut after: Vec<usize> = after.into_iter().collect();
        after.sort_unstable();
        after.dedup();
        Self {
            label: label.into(),
            wish,
            after,
        }
    }
}

/// Why a venture could not be constructed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VentureError {
    DependencyOutOfRange {
        stage: usize,
        after: usize,
    },
    SelfDependency {
        stage: usize,
    },
    /// The dependency graph contains a cycle through these stages.
    Cycle {
        stages: Vec<usize>,
    },
}

impl fmt::Display for VentureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DependencyOutOfRange { stage, after } => {
                write!(f, "stage {stage} depends on nonexistent stage {after}")
            }
            Self::SelfDependency { stage } => write!(f, "stage {stage} depends on itself"),
            Self::Cycle { stages } => write!(f, "dependency cycle through stages {stages:?}"),
        }
    }
}

impl std::error::Error for VentureError {}

/// Content for the deterministic venture id.
#[derive(Serialize)]
struct VentureContent<'a> {
    label: &'a str,
    stages: &'a Vec<VentureStage>,
    policy_id: &'a Digest,
    evidence_bundle_id: &'a Digest,
}

/// A content-addressed declaration of a whole system: wishes, ordered by
/// dependencies, fail-closed acyclic by construction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Venture {
    pub venture_id: Digest,
    pub label: String,
    pub stages: Vec<VentureStage>,
    pub policy_id: Digest,
    pub evidence_bundle_id: Digest,
}

impl Venture {
    pub fn new(
        label: impl Into<String>,
        stages: impl IntoIterator<Item = VentureStage>,
        policy_id: Digest,
        evidence_bundle_id: Digest,
    ) -> Result<Self, VentureError> {
        let stages: Vec<VentureStage> = stages.into_iter().collect();
        for (i, stage) in stages.iter().enumerate() {
            for &dep in &stage.after {
                if dep == i {
                    return Err(VentureError::SelfDependency { stage: i });
                }
                if dep >= stages.len() {
                    return Err(VentureError::DependencyOutOfRange {
                        stage: i,
                        after: dep,
                    });
                }
            }
        }
        let mut v = Self {
            venture_id: Digest::ZERO,
            label: label.into(),
            stages,
            policy_id,
            evidence_bundle_id,
        };
        // Acyclicity: the topological walk must consume every stage.
        let order = v.execution_order();
        if order.len() != v.stages.len() {
            let ordered: BTreeSet<usize> = order.into_iter().collect();
            let stuck: Vec<usize> = (0..v.stages.len())
                .filter(|i| !ordered.contains(i))
                .collect();
            return Err(VentureError::Cycle { stages: stuck });
        }
        v.venture_id = v.compute_id();
        Ok(v)
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&VentureContent {
            label: &self.label,
            stages: &self.stages,
            policy_id: &self.policy_id,
            evidence_bundle_id: &self.evidence_bundle_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.venture_id == self.compute_id()
    }

    /// CROSS-006: a durable venture must be evidence-bound.
    pub fn is_evidence_bound(&self) -> bool {
        self.evidence_bundle_id != Digest::ZERO
    }

    /// Deterministic topological order: among the ready stages, the smallest
    /// index goes first (Kahn's algorithm over a `BTreeSet`). On a cyclic
    /// graph the order is partial — `new` uses that to reject the cycle.
    pub fn execution_order(&self) -> Vec<usize> {
        let n = self.stages.len();
        let mut remaining_deps: Vec<BTreeSet<usize>> = self
            .stages
            .iter()
            .map(|s| s.after.iter().copied().collect())
            .collect();
        let mut ready: BTreeSet<usize> = (0..n).filter(|&i| remaining_deps[i].is_empty()).collect();
        let mut order = Vec::with_capacity(n);
        while let Some(&next) = ready.iter().next() {
            ready.remove(&next);
            order.push(next);
            for (i, deps) in remaining_deps.iter_mut().enumerate() {
                if deps.remove(&next) && deps.is_empty() {
                    ready.insert(i);
                }
            }
        }
        order
    }

    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }
}

/// The standing of one stage inside a session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageStanding {
    /// Not yet attempted.
    Pending,
    /// Its descent realized the wish.
    Realized,
    /// Its descent ended without realizing the wish.
    Failed,
    /// A (transitive) prerequisite failed — never attempted, by policy.
    Blocked,
}

impl StageStanding {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Realized => "realized",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
        }
    }
}

/// The durable execution state of a venture: which stages stand, which
/// fell, which are barred. Serializable, so an aborted run resumes exactly
/// where it stopped. Pure state — the runner observes and descends, the
/// session only records and orders.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VentureSession {
    venture: Venture,
    standings: Vec<StageStanding>,
}

impl VentureSession {
    pub fn new(venture: Venture) -> Self {
        let standings = vec![StageStanding::Pending; venture.stages.len()];
        Self { venture, standings }
    }

    pub fn venture(&self) -> &Venture {
        &self.venture
    }

    pub fn standings(&self) -> &[StageStanding] {
        &self.standings
    }

    /// The next stage to attempt: the smallest-index `Pending` stage whose
    /// prerequisites are all `Realized`. `None` when nothing is attemptable
    /// (complete, failed out, or blocked).
    pub fn next_ready(&self) -> Option<usize> {
        (0..self.venture.stages.len()).find(|&i| {
            self.standings[i] == StageStanding::Pending
                && self.venture.stages[i]
                    .after
                    .iter()
                    .all(|&d| self.standings[d] == StageStanding::Realized)
        })
    }

    /// Record a stage outcome. A `Failed` mark cascades: every transitive
    /// dependent that is still `Pending` becomes `Blocked` — the session
    /// never attempts what cannot stand.
    pub fn mark(&mut self, stage: usize, standing: StageStanding) {
        if stage >= self.standings.len() {
            return;
        }
        self.standings[stage] = standing;
        if standing != StageStanding::Failed && standing != StageStanding::Blocked {
            return;
        }
        // Cascade over the dependency graph (BFS from the fallen stage).
        let mut queue = VecDeque::from([stage]);
        while let Some(fallen) = queue.pop_front() {
            for (i, s) in self.venture.stages.iter().enumerate() {
                if s.after.contains(&fallen) && self.standings[i] == StageStanding::Pending {
                    self.standings[i] = StageStanding::Blocked;
                    queue.push_back(i);
                }
            }
        }
    }

    pub fn realized_count(&self) -> usize {
        self.standings
            .iter()
            .filter(|s| **s == StageStanding::Realized)
            .count()
    }

    /// Every stage realized — the venture stands.
    pub fn is_complete(&self) -> bool {
        !self.standings.is_empty() && self.standings.iter().all(|s| *s == StageStanding::Realized)
    }

    /// Nothing is attemptable any more (complete, or failure/blockage ended
    /// the staircase).
    pub fn is_settled(&self) -> bool {
        self.next_ready().is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wish::{WishFacet, WishPredicate};

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn wish(name: &str) -> Wish {
        Wish::new(
            format!("a module {name}"),
            [WishPredicate::require(WishFacet::module(name))],
            d(b"pol"),
            d(name.as_bytes()),
        )
    }

    fn stage(name: &str, after: &[usize]) -> VentureStage {
        VentureStage::new(name, wish(name), after.iter().copied())
    }

    /// A diamond: 0 → {1, 2} → 3.
    fn diamond() -> Venture {
        Venture::new(
            "diamond",
            [
                stage("base", &[]),
                stage("left", &[0]),
                stage("right", &[0]),
                stage("top", &[1, 2]),
            ],
            d(b"pol"),
            d(b"ev"),
        )
        .unwrap()
    }

    #[test]
    fn venture_is_content_addressed_and_evidence_bound() {
        let a = diamond();
        let b = diamond();
        assert_eq!(a.venture_id, b.venture_id);
        assert_ne!(a.venture_id, Digest::ZERO);
        assert!(a.verify_id());
        assert!(a.is_evidence_bound());
    }

    #[test]
    fn execution_order_is_a_deterministic_topological_walk() {
        let v = diamond();
        assert_eq!(v.execution_order(), vec![0, 1, 2, 3]);
        // Declaration order does not dictate the walk — dependencies do,
        // with smallest-ready-index as the deterministic tiebreak.
        let v = Venture::new(
            "reversed",
            [
                stage("top", &[1, 2]),
                stage("left", &[3]),
                stage("right", &[3]),
                stage("base", &[]),
            ],
            d(b"pol"),
            d(b"ev"),
        )
        .unwrap();
        assert_eq!(v.execution_order(), vec![3, 1, 2, 0]);
    }

    #[test]
    fn invalid_ventures_never_exist_as_values() {
        let out_of_range = Venture::new("x", [stage("a", &[7])], d(b"p"), d(b"e"));
        assert_eq!(
            out_of_range.unwrap_err(),
            VentureError::DependencyOutOfRange { stage: 0, after: 7 }
        );

        let selfish = Venture::new("x", [stage("a", &[0])], d(b"p"), d(b"e"));
        assert_eq!(
            selfish.unwrap_err(),
            VentureError::SelfDependency { stage: 0 }
        );

        let cyclic = Venture::new(
            "x",
            [stage("a", &[1]), stage("b", &[0]), stage("c", &[])],
            d(b"p"),
            d(b"e"),
        );
        assert_eq!(
            cyclic.unwrap_err(),
            VentureError::Cycle { stages: vec![0, 1] }
        );
    }

    #[test]
    fn session_walks_the_staircase_in_dependency_order() {
        let mut s = VentureSession::new(diamond());
        assert_eq!(s.next_ready(), Some(0));
        s.mark(0, StageStanding::Realized);
        assert_eq!(s.next_ready(), Some(1));
        s.mark(1, StageStanding::Realized);
        assert_eq!(s.next_ready(), Some(2));
        // Stage 3 stays unready until BOTH prerequisites stand.
        s.mark(2, StageStanding::Realized);
        assert_eq!(s.next_ready(), Some(3));
        s.mark(3, StageStanding::Realized);
        assert_eq!(s.next_ready(), None);
        assert!(s.is_complete());
        assert!(s.is_settled());
        assert_eq!(s.realized_count(), 4);
    }

    #[test]
    fn failure_blocks_every_transitive_dependent() {
        let mut s = VentureSession::new(diamond());
        s.mark(0, StageStanding::Realized);
        s.mark(1, StageStanding::Failed);
        // 3 depends on 1 (failed) → Blocked; 2 is independent → attemptable.
        assert_eq!(s.standings()[3], StageStanding::Blocked);
        assert_eq!(s.next_ready(), Some(2));
        s.mark(2, StageStanding::Realized);
        assert_eq!(s.next_ready(), None, "the staircase ends honestly");
        assert!(s.is_settled());
        assert!(!s.is_complete());
        assert_eq!(s.realized_count(), 2);
    }

    #[test]
    fn root_failure_blocks_the_whole_cascade() {
        let mut s = VentureSession::new(diamond());
        s.mark(0, StageStanding::Failed);
        assert_eq!(s.standings()[1], StageStanding::Blocked);
        assert_eq!(s.standings()[2], StageStanding::Blocked);
        assert_eq!(s.standings()[3], StageStanding::Blocked);
        assert!(s.is_settled());
    }

    #[test]
    fn session_serde_round_trip_preserves_progress() {
        let mut s = VentureSession::new(diamond());
        s.mark(0, StageStanding::Realized);
        s.mark(1, StageStanding::Failed);
        let json = serde_json::to_string(&s).unwrap();
        let back: VentureSession = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
        assert!(back.venture().verify_id());
        assert_eq!(back.next_ready(), Some(2), "resume continues exactly here");
    }

    #[test]
    fn empty_venture_is_vacuously_settled() {
        let v = Venture::new("empty", [], d(b"p"), d(b"e")).unwrap();
        assert!(v.execution_order().is_empty());
        let s = VentureSession::new(v);
        assert_eq!(s.next_ready(), None);
        assert!(!s.is_complete(), "an empty venture erects nothing");
        assert!(s.is_settled());
    }
}
