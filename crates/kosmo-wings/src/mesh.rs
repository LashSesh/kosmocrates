//! Wing algebra and the wing mesh (spec ch. 12.1–12.2).

use kosmo_core::Digest;
use serde::{Deserialize, Serialize};

/// The composition algebra over wings (ch. 12.1): parallel (different horizons),
/// sequential (one works on another's condensate), resonant (joint invariant),
/// conflict (NOT swallowed — written to residue/boundary), defer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Composition {
    Parallel,
    Sequential,
    Resonant,
    Conflict,
    Defer,
}

/// A wing mesh `W = (V_W, E_W)`: wings plus typed composition edges. Conflicting
/// arms are not summed; the conflict is exkalibrated (ch. 10.3 / 12.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WingMesh {
    pub wings: Vec<Digest>,
    pub edges: Vec<(usize, usize, Composition)>,
}

impl WingMesh {
    pub fn new(wings: Vec<Digest>, edges: Vec<(usize, usize, Composition)>) -> Self {
        Self { wings, edges }
    }
    pub fn id(&self) -> Digest {
        Digest::of(self)
    }
    /// Conflict edges that MUST be exkalibrated to the boundary, never summed.
    pub fn conflicts(&self) -> Vec<(usize, usize)> {
        self.edges
            .iter()
            .filter(|(_, _, c)| matches!(c, Composition::Conflict))
            .map(|(a, b, _)| (*a, *b))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mesh_is_content_addressed_and_surfaces_conflicts() {
        let wings = vec![Digest::of_bytes(b"w0"), Digest::of_bytes(b"w1"), Digest::of_bytes(b"w2")];
        let mesh = WingMesh::new(
            wings.clone(),
            vec![
                (0, 1, Composition::Sequential),
                (1, 2, Composition::Conflict),
            ],
        );
        // Conflicts are surfaced, not swallowed (ch. 12.1).
        assert_eq!(mesh.conflicts(), vec![(1, 2)]);
        // Same content → same id (I1/I4).
        assert_eq!(mesh.id(), WingMesh::new(wings, mesh.edges.clone()).id());
    }
}
