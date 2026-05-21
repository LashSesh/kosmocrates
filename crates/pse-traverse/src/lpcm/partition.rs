//! Window partitioning and run descriptor validation (§11, §9.1 steps 1-3).

use std::collections::BTreeMap;

use super::evidence::LpcmInputWindow;
use super::fragment::{FragmentRef, FragmentedPhaseSpaceWindow};
use super::primitives::{lpcm_content_address, EvidenceRef, Hash256, LpcmError, LpcmResult};
use super::run_descriptor::{LpcmRunDescriptor, PartitionPolicy};

/// Validate the run descriptor. Returns error if required fields are missing or wrong.
pub fn validate_descriptor(rd: &LpcmRunDescriptor) -> LpcmResult<()> {
    if rd.lpcm_version != LpcmRunDescriptor::lpcm_version_tag() {
        return Err(LpcmError::InvalidDescriptor {
            reason: format!(
                "lpcm_version must be {:?}, got {:?}",
                LpcmRunDescriptor::lpcm_version_tag(),
                rd.lpcm_version,
            ),
        });
    }
    if rd.run_id.is_empty() {
        return Err(LpcmError::InvalidDescriptor {
            reason: "run_id must not be empty".to_string(),
        });
    }
    Ok(())
}

/// Partition the input window into a FragmentedPhaseSpaceWindow.
///
/// Deterministic: given the same input and descriptor, always produces the same fragments.
pub fn partition_window(
    rd: &LpcmRunDescriptor,
    input: &LpcmInputWindow,
) -> LpcmResult<FragmentedPhaseSpaceWindow> {
    match &rd.policies.partition_policy {
        PartitionPolicy::FixedGrid { cells_per_axis } => {
            partition_fixed_grid(rd, input, *cells_per_axis)
        }
        PartitionPolicy::GraphKHop { k } => partition_graph_khop(rd, input, *k),
        PartitionPolicy::CarrierWindow { width, stride } => {
            partition_carrier_window(rd, input, *width, *stride)
        }
        PartitionPolicy::TopologyAdaptive {
            max_vertices,
            max_overlap: _,
        } => partition_topology_adaptive(rd, input, *max_vertices),
    }
}

fn partition_fixed_grid(
    rd: &LpcmRunDescriptor,
    input: &LpcmInputWindow,
    cells_per_axis: u32,
) -> LpcmResult<FragmentedPhaseSpaceWindow> {
    let n = cells_per_axis.max(1) as usize;
    let evidence_refs: Vec<_> = input.evidence_refs.iter().collect();
    let total = evidence_refs.len();

    // Distribute evidence refs uniformly across n cells.
    let chunk_size = if total == 0 { 1 } else { (total + n - 1) / n };
    let mut fragments: Vec<FragmentRef> = Vec::new();

    for (ordinal, chunk) in evidence_refs.chunks(chunk_size).enumerate() {
        let frag_evidence: Vec<EvidenceRef> = chunk.iter().map(|e| (*e).clone()).collect();
        let boundary_hash = lpcm_content_address(&(ordinal as u64, &rd.source_window_hash))?;
        let fragment_id = lpcm_content_address(&(&boundary_hash, &frag_evidence))?;

        fragments.push(FragmentRef {
            fragment_id,
            ordinal: ordinal as u64,
            evidence_refs: frag_evidence,
            boundary_hash,
            carrier_hash: input.carrier_hash.clone(),
        });
    }

    // If no evidence refs, create one empty fragment.
    if fragments.is_empty() {
        let boundary_hash = lpcm_content_address(&(0u64, &rd.source_window_hash))?;
        let fragment_id = lpcm_content_address(&(&boundary_hash, &Vec::<EvidenceRef>::new()))?;
        fragments.push(FragmentRef {
            fragment_id,
            ordinal: 0,
            evidence_refs: vec![],
            boundary_hash,
            carrier_hash: input.carrier_hash.clone(),
        });
    }

    // Build adjacency: consecutive fragments are adjacent.
    let mut adjacency: BTreeMap<Hash256, Vec<Hash256>> = BTreeMap::new();
    for i in 0..fragments.len() {
        let mut adj = Vec::new();
        if i > 0 {
            adj.push(fragments[i - 1].fragment_id.clone());
        }
        if i + 1 < fragments.len() {
            adj.push(fragments[i + 1].fragment_id.clone());
        }
        adjacency.insert(fragments[i].fragment_id.clone(), adj);
    }

    let partition_report_ref = lpcm_content_address(&(&fragments, "fixed-grid"))?;
    let window_id = lpcm_content_address(&(&fragments, &adjacency))?;

    Ok(FragmentedPhaseSpaceWindow {
        window_id,
        source_window_hash: input.window_hash.clone(),
        fragments,
        adjacency,
        partition_report_ref,
    })
}

fn partition_graph_khop(
    rd: &LpcmRunDescriptor,
    input: &LpcmInputWindow,
    k: u32,
) -> LpcmResult<FragmentedPhaseSpaceWindow> {
    // Fall back to fixed grid for now (k-hop requires graph structure).
    partition_fixed_grid(rd, input, k.max(1))
}

fn partition_carrier_window(
    rd: &LpcmRunDescriptor,
    input: &LpcmInputWindow,
    width: u32,
    stride: u32,
) -> LpcmResult<FragmentedPhaseSpaceWindow> {
    let w = width.max(1) as usize;
    let s = stride.max(1) as usize;
    let evidence_refs: Vec<_> = input.evidence_refs.clone();
    let total = evidence_refs.len();

    let mut fragments: Vec<FragmentRef> = Vec::new();
    let mut start = 0usize;
    let mut ordinal = 0u64;

    while start < total.max(1) {
        let end = (start + w).min(total);
        let frag_evidence = evidence_refs[start..end].to_vec();
        let boundary_hash = lpcm_content_address(&(ordinal, &rd.source_window_hash))?;
        let fragment_id = lpcm_content_address(&(&boundary_hash, &frag_evidence))?;
        fragments.push(FragmentRef {
            fragment_id,
            ordinal,
            evidence_refs: frag_evidence,
            boundary_hash,
            carrier_hash: input.carrier_hash.clone(),
        });
        ordinal += 1;
        start += s;
        if start >= total {
            break;
        }
    }

    if fragments.is_empty() {
        let boundary_hash = lpcm_content_address(&(0u64, &rd.source_window_hash))?;
        let fragment_id = lpcm_content_address(&(&boundary_hash, &Vec::<EvidenceRef>::new()))?;
        fragments.push(FragmentRef {
            fragment_id,
            ordinal: 0,
            evidence_refs: vec![],
            boundary_hash,
            carrier_hash: input.carrier_hash.clone(),
        });
    }

    let mut adjacency: BTreeMap<Hash256, Vec<Hash256>> = BTreeMap::new();
    for i in 0..fragments.len() {
        let mut adj = Vec::new();
        if i > 0 {
            adj.push(fragments[i - 1].fragment_id.clone());
        }
        if i + 1 < fragments.len() {
            adj.push(fragments[i + 1].fragment_id.clone());
        }
        adjacency.insert(fragments[i].fragment_id.clone(), adj);
    }

    let partition_report_ref = lpcm_content_address(&(&fragments, "carrier-window"))?;
    let window_id = lpcm_content_address(&(&fragments, &adjacency))?;
    Ok(FragmentedPhaseSpaceWindow {
        window_id,
        source_window_hash: input.window_hash.clone(),
        fragments,
        adjacency,
        partition_report_ref,
    })
}

fn partition_topology_adaptive(
    rd: &LpcmRunDescriptor,
    input: &LpcmInputWindow,
    max_vertices: u32,
) -> LpcmResult<FragmentedPhaseSpaceWindow> {
    partition_fixed_grid(rd, input, (max_vertices.max(4) / 4).max(1))
}
