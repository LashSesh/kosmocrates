//! Phase layout for the 13-node Metatron scaffold.
//!
//! Maps the cuboctahedron+center geometry to a `(phase, radius)` table
//! suitable for constructing a PSE phase-ladder. The mapping is:
//!
//!  - The center (node 1) sits at `(phase = 0, radius = 0)` — the
//!    null-center carrier with no angular position.
//!  - The 12 outer cuboctahedron vertices are projected along the
//!    `(1, 1, 1)` body diagonal onto the plane normal to it. Their
//!    azimuthal angles in that projection plane become their carrier
//!    phases; their radius is `1.0` (the cuboctahedron's edge-to-center
//!    distance, normalised).
//!
//! The (1, 1, 1) projection is the canonical "Metatron's cube" 2-D view
//! discussed in `crate::geometry`: under this projection the 12 outer
//! vertices form two concentric hexagons offset by 30°. Each azimuth
//! is therefore distinct, giving 12 unique carrier phases plus the
//! null-centre — the maximally-symmetric 13-carrier phase-ladder
//! configuration in 3-D.

use crate::geometry::canonical_nodes;

/// `(phase ∈ [0, 2π), radius ∈ {0, 1})` for each of the 13 nodes in
/// canonical order. Index 0 is the centre (radius 0); indices 1..=12
/// are the cuboctahedron outer vertices (radius 1).
pub fn metatron_phase_layout() -> Vec<(f64, f64)> {
    let nodes = canonical_nodes();
    let mut out = Vec::with_capacity(nodes.len());

    // Orthonormal basis for the plane normal to (1, 1, 1).
    //  e1 = (1, -1,  0) / sqrt(2)
    //  e2 = (1,  1, -2) / sqrt(6)
    let inv_sqrt2 = 1.0 / 2.0_f64.sqrt();
    let inv_sqrt6 = 1.0 / 6.0_f64.sqrt();
    let e1 = [inv_sqrt2, -inv_sqrt2, 0.0];
    let e2 = [inv_sqrt6, inv_sqrt6, -2.0 * inv_sqrt6];

    for node in &nodes {
        if node.node_type == "center" {
            out.push((0.0, 0.0));
        } else {
            let v = node.coords;
            let p1 = v[0] * e1[0] + v[1] * e1[1] + v[2] * e1[2];
            let p2 = v[0] * e2[0] + v[1] * e2[1] + v[2] * e2[2];
            let phi = p2.atan2(p1).rem_euclid(std::f64::consts::TAU);
            out.push((phi, 1.0));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_has_13_entries() {
        let layout = metatron_phase_layout();
        assert_eq!(layout.len(), 13);
    }

    #[test]
    fn first_entry_is_center_with_radius_zero() {
        let layout = metatron_phase_layout();
        let (phi, r) = layout[0];
        assert_eq!(phi, 0.0);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn outer_entries_have_radius_one() {
        let layout = metatron_phase_layout();
        for (i, (_, r)) in layout.iter().enumerate().skip(1) {
            assert!(
                (r - 1.0).abs() < 1e-12,
                "outer entry {} has radius {}, expected 1.0",
                i,
                r
            );
        }
    }

    #[test]
    fn outer_phases_lie_in_zero_two_pi() {
        let layout = metatron_phase_layout();
        for (i, (phi, _)) in layout.iter().enumerate().skip(1) {
            assert!(
                *phi >= 0.0 && *phi < std::f64::consts::TAU,
                "phase {} out of [0, 2π): {}",
                i,
                phi
            );
        }
    }

    #[test]
    fn outer_phases_are_pairwise_distinct() {
        // The 12 outer vertices project to 12 distinct azimuths; this
        // is the very property that makes the cuboctahedron the
        // maximally symmetric 12-direction sampling of the unit sphere.
        let layout = metatron_phase_layout();
        let phases: Vec<f64> = layout.iter().skip(1).map(|(p, _)| *p).collect();
        for i in 0..phases.len() {
            for j in (i + 1)..phases.len() {
                assert!(
                    (phases[i] - phases[j]).abs() > 1e-9,
                    "phases {} and {} collide at {}",
                    i + 1,
                    j + 1,
                    phases[i]
                );
            }
        }
    }

    #[test]
    fn phase_layout_is_deterministic() {
        // Repeated calls produce identical output (no hidden RNG).
        let a = metatron_phase_layout();
        let b = metatron_phase_layout();
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.0, y.0);
            assert_eq!(x.1, y.1);
        }
    }

    #[test]
    fn cuboctahedron_phases_are_balanced() {
        // The cuboctahedron is the "vector equilibrium" — its centre of
        // mass under the unit-disc layout sits at the origin. We check
        // a weaker but useful property: the sum of the 12 outer phasors
        // (cos φᵢ, sin φᵢ) is the zero vector to within floating-point
        // tolerance. This is the property that makes the configuration
        // bias-free in any direction, the empirical cash value of
        // "perfectly balanced 12-direction sampling".
        let layout = metatron_phase_layout();
        let mut sum_x = 0.0_f64;
        let mut sum_y = 0.0_f64;
        for (phi, _) in layout.iter().skip(1) {
            sum_x += phi.cos();
            sum_y += phi.sin();
        }
        assert!(sum_x.abs() < 1e-9, "sum cos = {} (expected ~0)", sum_x);
        assert!(sum_y.abs() < 1e-9, "sum sin = {} (expected ~0)", sum_y);
    }
}
