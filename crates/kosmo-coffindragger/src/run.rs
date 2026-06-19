//! CDK — the Con-Dragger cycle (spec ch. 3.2, 10, Listing 20.2):
//! `CDK = Cen ∘ Probe ∘ Seed ∘ Purge ∘ Pull`. The peristaltic runtime body
//! advances `Front → Anchor → PullBody → Compress`. This module realizes the
//! Purge primitive (exkalibration) of the cycle; the full orchestration over the
//! wing mesh (`kosmo-wings`) and the stack closure ([`crate::stack`]) is wired in
//! the diamondization ticket.

use kosmo_core::Digest;
use std::collections::HashSet;

/// The clean core and the exkalibrated boundary residue produced by a purge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PurgeResult {
    pub core: Vec<Digest>,
    pub residue: Vec<Digest>,
}

/// CDK **Purge** (§3.2): exkalibrate foreign mass — separate the foreign /
/// incompatible / noise units to the boundary as residue, keeping only the clean
/// core. Every ejection is accounted (I3 — boundary over absorption; nothing is
/// silently swallowed). Deterministic: input order is preserved.
pub fn purge(units: &[Digest], foreign: &HashSet<Digest>) -> PurgeResult {
    let core = units.iter().copied().filter(|u| !foreign.contains(u)).collect();
    let residue = units.iter().copied().filter(|u| foreign.contains(u)).collect();
    PurgeResult { core, residue }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purge_exkalibrates_foreign_mass_to_visible_residue() {
        let units: Vec<Digest> = [b"a".as_ref(), b"x", b"b", b"y"]
            .iter()
            .map(|u| Digest::of_bytes(u))
            .collect();
        let foreign: HashSet<Digest> =
            [Digest::of_bytes(b"x"), Digest::of_bytes(b"y")].into_iter().collect();
        let r = purge(&units, &foreign);
        assert_eq!(r.core, vec![Digest::of_bytes(b"a"), Digest::of_bytes(b"b")]);
        assert_eq!(r.residue, vec![Digest::of_bytes(b"x"), Digest::of_bytes(b"y")]);
        // I3 / Contract 8.4: nothing vanishes — core ∪ residue == input.
        assert_eq!(r.core.len() + r.residue.len(), units.len());
    }
}
