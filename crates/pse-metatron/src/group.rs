use crate::error::ScanResult;

/// Recursively generate all permutations of the given elements.
fn permutations_of(elements: &[usize]) -> Vec<Vec<usize>> {
    if elements.is_empty() {
        return vec![Vec::new()];
    }
    let mut result = Vec::new();
    for (idx, &item) in elements.iter().enumerate() {
        let mut rest = elements.to_vec();
        rest.remove(idx);
        for mut perm in permutations_of(&rest) {
            perm.insert(0, item);
            result.push(perm);
        }
    }
    result
}

/// Generate all 5040 permutations of S7.
/// Each permutation is a Vec of length 7 with values 1..=7.
pub fn generate_s7() -> Vec<Vec<usize>> {
    permutations_of(&[1, 2, 3, 4, 5, 6, 7])
}

/// Generate a 13x13 permutation matrix from an S7 element.
/// Nodes 1..7 are permuted according to sigma, nodes 8..13 are identity.
///
/// (P_sigma)_ij = 1 if i,j <= 7 and j = sigma(i)
/// (P_sigma)_ij = 1 if i = j > 7
/// (P_sigma)_ij = 0 otherwise
pub fn permutation_matrix_13(sigma: &[usize]) -> Vec<Vec<f64>> {
    let mut p = vec![vec![0.0; 13]; 13];

    // Active zone: nodes 1..7 (indices 0..6)
    for i in 0..7 {
        let j = sigma[i] - 1; // sigma values are 1-indexed
        p[i][j] = 1.0;
    }

    // Reference frame: nodes 8..13 (indices 7..12) are identity
    for i in 7..13 {
        p[i][i] = 1.0;
    }

    p
}

/// Conjugation: A' = P * A * P^T
/// Since P is orthogonal, P^(-1) = P^T.
pub fn conjugate(a: &[Vec<f64>], p: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = a.len();
    let mut result = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            let mut sum = 0.0;
            for k in 0..n {
                for l in 0..n {
                    sum += p[i][k] * a[k][l] * p[j][l]; // P * A * P^T
                }
            }
            result[i][j] = sum;
        }
    }
    result
}

/// Generate hexagon rotation by k steps.
/// Center (node 1) stays fixed. Hexagon nodes 2..7 are rotated.
pub fn hexagon_rotation(k: i32) -> Vec<usize> {
    let mut mapping = vec![1]; // Center stays
    let shift = ((k % 6) + 6) % 6;
    for i in 0..6 {
        mapping.push(2 + ((i as i32 + shift) % 6) as usize);
    }
    mapping
}

/// C6 subgroup: hexagon rotations (center fixed). |C6| = 6.
pub fn generate_c6() -> Vec<Vec<usize>> {
    (0..6).map(hexagon_rotation).collect()
}

/// Generate hexagon reflection across the axis through center and axis_node.
/// axis_node must be in 2..=7.
pub fn hexagon_reflection(axis_node: usize) -> ScanResult<Vec<usize>> {
    // Angles for hexagon nodes (in degrees)
    const ANGLES: [(usize, f64); 6] = [
        (2, 0.0),
        (3, 60.0),
        (4, 120.0),
        (5, 180.0),
        (6, 240.0),
        (7, 300.0),
    ];

    // Find the angle of the axis node
    let axis_angle = ANGLES
        .iter()
        .find(|(n, _)| *n == axis_node)
        .map(|(_, a)| *a)
        .unwrap_or(0.0);

    let mut mapping = vec![1usize]; // Center stays

    for &(node, angle) in &ANGLES {
        // Reflect: new_angle = 2 * axis_angle - angle
        let reflected_angle = (2.0 * axis_angle - angle + 360.0) % 360.0;

        // Find the node at the reflected angle
        let mut target = node; // default to self
        for &(n, a) in &ANGLES {
            if (a - reflected_angle).abs() < 1e-10
                || (a - reflected_angle + 360.0).abs() < 1e-10
                || (a - reflected_angle - 360.0).abs() < 1e-10
            {
                target = n;
                break;
            }
        }
        mapping.push(target);
    }

    Ok(mapping)
}

/// D6 subgroup: rotations + reflections. |D6| = 12.
pub fn generate_d6() -> ScanResult<Vec<Vec<usize>>> {
    let mut group = generate_c6();
    for axis in 2..=7 {
        group.push(hexagon_reflection(axis)?);
    }
    Ok(group)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s7_count() {
        assert_eq!(generate_s7().len(), 5040);
    }

    #[test]
    fn s7_all_unique() {
        let perms = generate_s7();
        let mut sorted = perms.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 5040);
    }

    #[test]
    fn s7_matrix_valid() {
        let perms = generate_s7();
        for sigma in &perms {
            let p = permutation_matrix_13(sigma);
            assert_eq!(p.len(), 13);
            // Each row has exactly one 1
            for row in &p {
                assert_eq!(row.len(), 13);
                let sum: f64 = row.iter().sum();
                assert!((sum - 1.0).abs() < 1e-10, "Row sum != 1");
            }
            // Each column has exactly one 1
            for j in 0..13 {
                let col_sum: f64 = (0..13).map(|i| p[i][j]).sum();
                assert!((col_sum - 1.0).abs() < 1e-10, "Col sum != 1");
            }
        }
    }

    #[test]
    fn identity_permutation_gives_identity_matrix() {
        let id = vec![1, 2, 3, 4, 5, 6, 7];
        let p = permutation_matrix_13(&id);
        for i in 0..13 {
            for j in 0..13 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert_eq!(p[i][j], expected);
            }
        }
    }

    #[test]
    fn conjugate_with_identity() {
        let mut a = vec![vec![0.0; 13]; 13];
        a[0][1] = 1.0;
        a[1][0] = 1.0;
        let id = permutation_matrix_13(&[1, 2, 3, 4, 5, 6, 7]);
        let result = conjugate(&a, &id);
        assert_eq!(result, a);
    }

    #[test]
    fn c6_count() {
        assert_eq!(generate_c6().len(), 6);
    }

    #[test]
    fn c6_subset_of_s7() {
        let c6 = generate_c6();
        let s7 = generate_s7();
        for perm in &c6 {
            assert!(s7.contains(perm), "C6 element {:?} not in S7", perm);
        }
    }

    #[test]
    fn d6_count() {
        let d6 = generate_d6().unwrap();
        assert_eq!(d6.len(), 12);
    }

    #[test]
    fn c6_identity_first() {
        let c6 = generate_c6();
        assert_eq!(c6[0], vec![1, 2, 3, 4, 5, 6, 7]);
    }
}
