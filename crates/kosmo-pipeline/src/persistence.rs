//! Policy-gated persistence helpers for pipeline outputs.
//!
//! `run_dry_pipeline` never writes host files. These helpers allow an operator
//! with `allow_host_write = true` to durably commit pipeline outputs after the
//! fact. All functions fail-closed: if the policy forbids the write the
//! `CartographyStoreError::PolicyDenied` variant is returned without touching
//! disk.

use kosmo_core::{
    CartographyEntryKind, CartographyStoreCommit, CartographyStoreError, CorpusCartographyStore,
    CorpusScope, Digest, PolicyProfile,
};
use kosmo_hyphae::CorpusCartographyUpdate;
use kosmo_store::JsonlCartographyStore;
use std::path::Path;

/// Durably persist a `CorpusCartographyUpdate` produced by a pipeline run.
///
/// Opens (or creates) the JSONL store at `path`, builds a
/// [`CartographyStoreCommit`] whose `payload_digest` is the update's
/// content address (`update_id`), and appends it.
///
/// # Policy gate
///
/// Persistence is a host write. The call fails with
/// [`CartographyStoreError::PolicyDenied`] if:
/// - `policy.mode == ReportOnly`, or
/// - `policy.allow_host_write == false` (DryRun mode).
///
/// # CROSS-006
///
/// `evidence_bundle_id` in the emitted commit is set to `update.update_id`.
/// The update IS the causal artifact (its ID content-addresses the entire
/// update), so this satisfies the non-ZERO evidence invariant.
pub fn persist_cartography_update(
    update: &CorpusCartographyUpdate,
    path: &Path,
    scope: CorpusScope,
    policy: &PolicyProfile,
) -> Result<Digest, CartographyStoreError> {
    let mut store = JsonlCartographyStore::open(path, scope.clone(), policy.id)?;
    let manifest = store.read_manifest()?;
    let next_seq = manifest.head_sequence + 1;
    let commit = CartographyStoreCommit::new(
        scope,
        next_seq,
        update.update_id,
        CartographyEntryKind::CartographyUpdate,
        update.update_id, // CROSS-006: non-ZERO evidence ref pointing at the update itself
        policy.id,
    )
    .with_label("after_cartography_id", update.after_cartography_id.to_hex())
    .with_label(
        "added_entity_count",
        update.added_entity_ids.len().to_string(),
    );
    store.append(commit, policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{Digest, PolicyProfile};
    use kosmo_hyphae::{passive_run, CorpusCartography};
    use kosmo_workbench::WorkspaceIndex;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(tag: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("kosmo-pipeline-persist-{tag}-{nanos}.jsonl"));
        p
    }

    fn make_update(policy: &PolicyProfile) -> kosmo_hyphae::CorpusCartographyUpdate {
        let corpus = CorpusCartography::empty(policy.id);
        let index = WorkspaceIndex::from_entries("test".into(), vec![], policy.id);
        let run = passive_run(&index, policy);
        let (_, update) = corpus.update_from_run(&run);
        update
    }

    #[test]
    fn persist_denied_by_report_only_policy() {
        let policy = PolicyProfile::default_report_only();
        let path = temp_path("ro");
        let update = make_update(&policy);
        let res =
            persist_cartography_update(&update, &path, CorpusScope::LocalHostProject, &policy);
        assert!(
            matches!(res, Err(CartographyStoreError::PolicyDenied { .. })),
            "ReportOnly policy must deny persistence"
        );
        assert!(!path.exists(), "no file created when denied");
    }

    #[test]
    fn persist_succeeds_with_operator_approved_policy() {
        let policy = PolicyProfile::operator_approved();
        let path = temp_path("oa");
        let update = make_update(&policy);
        let commit_id =
            persist_cartography_update(&update, &path, CorpusScope::LocalHostProject, &policy)
                .expect("OperatorApproved must allow persistence");
        assert_ne!(commit_id, Digest::ZERO);
        assert!(path.exists(), "JSONL file must be created on success");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn persist_commit_carries_correct_payload_digest() {
        let policy = PolicyProfile::operator_approved();
        let path = temp_path("pd");
        let update = make_update(&policy);
        persist_cartography_update(&update, &path, CorpusScope::LocalHostProject, &policy).unwrap();
        let store =
            JsonlCartographyStore::open(&path, CorpusScope::LocalHostProject, policy.id).unwrap();
        let manifest = store.read_manifest().unwrap();
        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.entries[0].payload_digest, update.update_id);
        assert_eq!(
            manifest.entries[0].payload_kind,
            CartographyEntryKind::CartographyUpdate,
        );
        let _ = std::fs::remove_file(&path);
    }
}
