//! Spectral signature derivation — PSE-TRAVERSE-SIGNATURE-01.
//!
//! Derives a [`Signature`] from a [`StructuralOperator`].  The default path
//! (`MatrixProfileApprox`) uses the operator's pre-computed [`MatrixProfile`]
//! diagonal and directed-edge weights directly.  For small graphs
//! (n ≤ `n_exact_max`) the `ExactSmallMatrix` algorithm builds an explicit
//! n×n Laplacian and runs an in-process Jacobi eigensolver.

use serde::{Deserialize, Serialize};

use crate::canonical::hex_address;
use crate::operator::{SignatureError, SignatureValue, StructuralOperator};

// ──────────────────────────────────────────────────────────────────────────────
// SignatureAlgorithm
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SignatureAlgorithm {
    /// Use the matrix profile diagonal + directed edge weights (default).
    MatrixProfileApprox,
    /// Build an explicit n×n Laplacian and run Jacobi (only for n ≤ n_exact_max).
    ExactSmallMatrix,
    /// Power-iteration approximation (placeholder; currently falls back to
    /// `MatrixProfileApprox`).
    PowerIterationProfile,
}

// ──────────────────────────────────────────────────────────────────────────────
// SignatureConfig
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SignatureConfig {
    pub algorithm: SignatureAlgorithm,
    /// Use Jacobi eigensolver for n ≤ this threshold (default 8).
    pub n_exact_max: usize,
    /// Decimal places kept after quantisation (default 6).
    pub quantization_scale: i32,
    /// Maximum Jacobi iterations before reporting non-convergence (default 200).
    pub max_jacobi_iter: usize,
}

impl Default for SignatureConfig {
    fn default() -> Self {
        SignatureConfig {
            algorithm: SignatureAlgorithm::MatrixProfileApprox,
            n_exact_max: 8,
            quantization_scale: 6,
            max_jacobi_iter: 200,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// SignatureSummary
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SignatureSummary {
    /// Number of values in the signature.
    pub dimension: usize,
    /// Smallest value.
    pub min_value: SignatureValue,
    /// Largest value.
    pub max_value: SignatureValue,
    /// Floor of arithmetic mean, quantised.
    pub mean_value: SignatureValue,
    /// Mean of consecutive differences, quantised.
    pub mean_spacing: SignatureValue,
    /// Number of values whose mantissa is exactly 0.
    pub zero_multiplicity: usize,
    /// Consecutive absolute differences, sorted ascending.
    pub gap_profile: Vec<SignatureValue>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Signature
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Signature {
    /// 64-char lower-case hex SHA-256 content address (set last, after all
    /// other fields are finalised).
    pub signature_id: String,
    /// Operator this signature was derived from.
    pub operator_id: String,
    /// Sorted ascending, quantised signature values.
    pub values: Vec<SignatureValue>,
    /// Summary statistics.
    pub summary: SignatureSummary,
    /// Algorithm used.
    pub algorithm: SignatureAlgorithm,
    /// Semver string for the algorithm implementation.
    pub algorithm_version: String,
}

// ──────────────────────────────────────────────────────────────────────────────
// Jacobi eigensolver (in-process, no external deps, n ≤ 8)
// ──────────────────────────────────────────────────────────────────────────────

fn jacobi_eigenvalues(mut a: Vec<Vec<f64>>, max_iter: usize) -> Result<Vec<f64>, SignatureError> {
    let n = a.len();
    for _iter in 0..max_iter {
        // Find the off-diagonal element with the largest absolute value.
        let mut max_off = 0.0f64;
        let (mut p, mut q) = (0, 1);
        for i in 0..n {
            for j in (i + 1)..n {
                if a[i][j].abs() > max_off {
                    max_off = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if max_off < 1e-12 {
            break;
        }
        if _iter == max_iter - 1 {
            return Err(SignatureError::AlgorithmDidNotConverge(max_iter));
        }
        // Jacobi rotation.
        let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
        let t = if theta.abs() < 1e15 {
            theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt())
        } else {
            1.0 / (2.0 * theta)
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;

        let app = a[p][p];
        let aqq = a[q][q];
        a[p][p] = app - t * a[p][q];
        a[q][q] = aqq + t * a[p][q];
        a[p][q] = 0.0;
        a[q][p] = 0.0;

        for r in 0..n {
            if r != p && r != q {
                let arp = a[r][p];
                let arq = a[r][q];
                a[r][p] = c * arp - s * arq;
                a[p][r] = a[r][p];
                a[r][q] = s * arp + c * arq;
                a[q][r] = a[r][q];
            }
        }
    }

    let mut eigs: Vec<f64> = (0..n).map(|i| a[i][i]).collect();
    for &e in &eigs {
        if !e.is_finite() {
            return Err(SignatureError::NonFiniteValue);
        }
    }
    eigs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Ok(eigs)
}

// ──────────────────────────────────────────────────────────────────────────────
// derive_signature
// ──────────────────────────────────────────────────────────────────────────────

/// Derive a spectral [`Signature`] from a [`StructuralOperator`].
pub fn derive_signature(
    operator: &StructuralOperator,
    config: &SignatureConfig,
) -> Result<Signature, SignatureError> {
    let n = operator.matrix_profile.n;
    let scale = config.quantization_scale;

    // ── 1. Guard: empty operator ──────────────────────────────────────────────
    if n == 0 {
        return Err(SignatureError::EmptyOperator);
    }

    // ── Determine effective algorithm ─────────────────────────────────────────
    let use_exact = matches!(config.algorithm, SignatureAlgorithm::ExactSmallMatrix)
        && n <= config.n_exact_max;

    let values: Vec<SignatureValue> = if use_exact {
        // ── ExactSmallMatrix: build explicit Laplacian, run Jacobi ───────────
        let mut lap: Vec<Vec<f64>> = vec![vec![0.0f64; n]; n];

        // Symmetric edges → Laplacian entries.
        for edge in &operator.matrix_profile.symmetric_edges {
            let w = edge.weight.to_f64();
            lap[edge.u][edge.u] += w;
            lap[edge.v][edge.v] += w;
            lap[edge.u][edge.v] -= w;
            lap[edge.v][edge.u] -= w;
        }

        // Constraint weights → diagonal penalty.
        for nw in &operator.matrix_profile.constraint_weights {
            lap[nw.node][nw.node] += nw.weight.to_f64();
        }

        let eigs = jacobi_eigenvalues(lap, config.max_jacobi_iter)?;

        eigs.into_iter()
            .map(|e| SignatureValue::quantize(e, scale))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        // ── MatrixProfileApprox (default) ─────────────────────────────────────
        // ── 2. Start with diagonal values (already sorted + quantised) ────────
        let mut vals: Vec<SignatureValue> = operator.matrix_profile.diagonal.clone();

        // ── 3. Add absolute directed-edge weights ─────────────────────────────
        for de in &operator.matrix_profile.directed_edges {
            let abs_val = de.weight.to_f64().abs();
            let sv = SignatureValue::quantize(abs_val, scale)
                .unwrap_or_else(|_| SignatureValue::zero(scale));
            vals.push(sv);
        }

        // ── 4. Sort combined values ascending ─────────────────────────────────
        vals.sort_by(|a, b| a.mantissa.cmp(&b.mantissa).then(a.scale.cmp(&b.scale)));
        vals
    };

    // ── 5. Compute SignatureSummary ────────────────────────────────────────────
    let len = values.len();

    let min_value = values
        .first()
        .cloned()
        .unwrap_or_else(|| SignatureValue::zero(scale));
    let max_value = values
        .last()
        .cloned()
        .unwrap_or_else(|| SignatureValue::zero(scale));

    // Mean: sum of to_f64(), divided by count, quantised.
    let sum_f64: f64 = values.iter().map(|v| v.to_f64()).sum();
    let mean_f64 = if len > 0 { sum_f64 / len as f64 } else { 0.0 };
    let mean_value = SignatureValue::quantize(mean_f64, scale)
        .unwrap_or_else(|_| SignatureValue::zero(scale));

    // Consecutive differences.
    let diffs: Vec<f64> = values
        .windows(2)
        .map(|w| w[1].to_f64() - w[0].to_f64())
        .collect();

    let mean_spacing_f64 = if diffs.is_empty() {
        0.0
    } else {
        diffs.iter().sum::<f64>() / diffs.len() as f64
    };
    let mean_spacing = SignatureValue::quantize(mean_spacing_f64, scale)
        .unwrap_or_else(|_| SignatureValue::zero(scale));

    let zero_multiplicity = values.iter().filter(|v| v.mantissa == 0).count();

    // Gap profile: quantise each diff, sort ascending.
    let mut gap_profile: Vec<SignatureValue> = diffs
        .iter()
        .map(|&d| {
            SignatureValue::quantize(d, scale).unwrap_or_else(|_| SignatureValue::zero(scale))
        })
        .collect();
    gap_profile.sort();

    let summary = SignatureSummary {
        dimension: len,
        min_value,
        max_value,
        mean_value,
        mean_spacing,
        zero_multiplicity,
        gap_profile,
    };

    // ── 6. Assemble with placeholder signature_id, then content-address ────────
    let mut sig = Signature {
        signature_id: String::new(),
        operator_id: operator.operator_id.clone(),
        values,
        summary,
        algorithm: config.algorithm.clone(),
        algorithm_version: "v0.1".into(),
    };

    let hex = hex_address(&sig).unwrap_or_else(|e| e.to_string());
    sig.signature_id = hex;

    Ok(sig)
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::dof::DoFGraph;
    use crate::field_cube::{DefaultFieldCubeBuilder, FieldCubeBuilder};
    use crate::operator::{build_operator_from_dof_graph, OperatorBuildConfig};
    use crate::spec::{
        ConstraintKind, ConstraintSpec, DimensionKind, DimensionSource, DimensionSpec,
        OutputSpec, ProblemSpec, ReplayPolicy, RiskPolicy, ValueDomain,
    };

    fn minimal_spec() -> ProblemSpec {
        ProblemSpec {
            id: "p.sig-test".into(),
            title: "T".into(),
            objective: "o".into(),
            domain: "d".into(),
            inputs: vec![],
            constraints: vec![ConstraintSpec {
                id: "c.a".into(),
                kind: ConstraintKind::Hard,
                predicate: "pred".into(),
                weight: 1.0,
                dimensions: vec!["d.x".into()],
            }],
            dimensions: vec![DimensionSpec {
                id: "d.x".into(),
                label: "X".into(),
                kind: DimensionKind::Boolean,
                values: ValueDomain::Boolean,
                required: true,
                source: DimensionSource::User,
            }],
            desired_outputs: vec![OutputSpec { id: "o.x".into(), kind: "y".into() }],
            risk_policy: RiskPolicy::default(),
            replay: ReplayPolicy::default(),
            metadata: BTreeMap::new(),
        }
    }

    fn minimal_operator() -> crate::operator::StructuralOperator {
        let cube = DefaultFieldCubeBuilder.build(&minimal_spec()).unwrap();
        let graph = DoFGraph::from_field_cube(&cube);
        let config = OperatorBuildConfig::default();
        build_operator_from_dof_graph(&graph, &config).unwrap()
    }

    #[test]
    fn signature_is_content_addressed() {
        let op = minimal_operator();
        let cfg = SignatureConfig::default();
        let sig1 = derive_signature(&op, &cfg).unwrap();
        let sig2 = derive_signature(&op, &cfg).unwrap();
        assert_eq!(sig1.signature_id, sig2.signature_id);
        assert_eq!(sig1.signature_id.len(), 64);
        assert!(sig1
            .signature_id
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }

    #[test]
    fn signature_quantization_is_stable() {
        // quantize(1.23456, 4) should give mantissa=12346.
        let sv = SignatureValue::quantize(1.23456, 4).unwrap();
        assert_eq!(sv.mantissa, 12346, "expected mantissa 12346, got {}", sv.mantissa);
        assert_eq!(sv.scale, 4);
    }

    #[test]
    fn signature_rejects_nan_and_inf() {
        assert!(
            SignatureValue::quantize(f64::NAN, 6).is_err(),
            "NaN should be rejected"
        );
        assert!(
            SignatureValue::quantize(f64::INFINITY, 6).is_err(),
            "+Inf should be rejected"
        );
        assert!(
            SignatureValue::quantize(f64::NEG_INFINITY, 6).is_err(),
            "-Inf should be rejected"
        );
    }
}
