//! Persistent on-disk stores for the `kosmo-*` substrate.
//!
//! ## What this crate adds
//!
//! `kosmo-core` defines the [`CorpusCartographyStore`] trait and an
//! `InMemoryCartographyStore` that holds commits in RAM. This crate adds
//! [`JsonlCartographyStore`]: an append-only, durable implementation that
//! persists each commit as one JSON line (JSONL) and reconstructs the manifest
//! by replaying the file on open.
//!
//! ## Why it is a separate crate
//!
//! `kosmo-core` is intentionally free of filesystem and process I/O so it stays
//! portable (the workspace compiles parts of the substrate to wasm). Disk
//! persistence is a host capability, so it lives here — the same isolation
//! principle as `kosmo-foundry` (process execution) and `kosmo-pse-bridge`
//! (the PSE crossing).
//!
//! ## Emergent safety property
//!
//! Writing a commit to disk **is a host write**. The in-memory store only has
//! to block `ReportOnly` (it never touches the host); a durable store must
//! additionally require `PolicyProfile.allow_host_write`. Because `DryRun`
//! keeps `allow_host_write == false` (it may execute in a sandbox but must not
//! mutate host files), **`DryRun` cannot persist** — only `OperatorApproved`
//! (or a custom profile that explicitly sets `allow_host_write`) may append to
//! disk. This is the very same host-write policy bit the Foundry sandbox
//! honours, now governing persistence: one invariant, enforced everywhere.
//!
//! Reading and integrity-verifying a store never mutates the host and is
//! therefore permitted in any mode.

use kosmo_core::{
    CartographyIntegrityReport, CartographyIntegrityStatus, CartographyStorageManifest,
    CartographyStoreCommit, CartographyStoreError, CorpusCartographyStore, CorpusScope, Digest,
    ImplementationMode, PolicyProfile,
};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// An append-only, durable [`CorpusCartographyStore`] backed by a JSONL file.
///
/// Each line of the backing file is the JSON serialization of one
/// [`CartographyStoreCommit`]. The in-memory `manifest` mirrors the file and is
/// the source of truth for sequence/scope checks; `verify_integrity` re-reads
/// the file from disk to confirm the durable copy matches.
pub struct JsonlCartographyStore {
    path: PathBuf,
    manifest: CartographyStorageManifest,
}

impl JsonlCartographyStore {
    /// Open an existing JSONL store or create an empty (unwritten) one.
    ///
    /// Replays every line of an existing file into the manifest, enforcing
    /// gapless sequence ordering and scope consistency as it goes. Opening is a
    /// read-only operation and is permitted in any policy mode — no file is
    /// created on disk until the first successful `append`.
    pub fn open(
        path: impl Into<PathBuf>,
        scope: CorpusScope,
        policy_id: Digest,
    ) -> Result<Self, CartographyStoreError> {
        let path = path.into();
        let mut manifest = CartographyStorageManifest::empty(scope.clone(), policy_id);

        if path.exists() {
            let file = File::open(&path).map_err(|e| CartographyStoreError::Io {
                message: format!("open {}: {e}", path.display()),
            })?;
            let reader = BufReader::new(file);
            let mut expected_seq: u64 = 1;
            for (lineno, line) in reader.lines().enumerate() {
                let line = line.map_err(|e| CartographyStoreError::Io {
                    message: format!("read line {}: {e}", lineno + 1),
                })?;
                if line.trim().is_empty() {
                    continue;
                }
                let commit: CartographyStoreCommit =
                    serde_json::from_str(&line).map_err(|e| CartographyStoreError::Io {
                        message: format!("parse line {}: {e}", lineno + 1),
                    })?;
                if commit.scope != scope {
                    return Err(CartographyStoreError::ScopeMismatch);
                }
                if commit.sequence != expected_seq {
                    return Err(CartographyStoreError::SequenceViolation {
                        expected: expected_seq,
                        got: commit.sequence,
                    });
                }
                manifest = manifest.with_commit(commit);
                expected_seq += 1;
            }
        }

        Ok(Self { path, manifest })
    }

    /// Path of the backing JSONL file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Number of commits currently persisted.
    pub fn len(&self) -> usize {
        self.manifest.entries.len()
    }

    /// Whether the store holds no commits.
    pub fn is_empty(&self) -> bool {
        self.manifest.entries.is_empty()
    }

    /// Append one JSON line to the backing file, fsync, and return.
    fn write_line(&self, commit: &CartographyStoreCommit) -> Result<(), CartographyStoreError> {
        let line = serde_json::to_string(commit).map_err(|e| CartographyStoreError::Io {
            message: format!("serialize commit: {e}"),
        })?;
        if line.contains('\n') {
            // A JSONL line must never contain a raw newline.
            return Err(CartographyStoreError::Io {
                message: "serialized commit contained a newline".into(),
            });
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| CartographyStoreError::Io {
                message: format!("open for append {}: {e}", self.path.display()),
            })?;
        file.write_all(line.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .and_then(|_| file.flush())
            .and_then(|_| file.sync_all())
            .map_err(|e| CartographyStoreError::Io {
                message: format!("write {}: {e}", self.path.display()),
            })?;
        Ok(())
    }
}

impl CorpusCartographyStore for JsonlCartographyStore {
    fn append(
        &mut self,
        commit: CartographyStoreCommit,
        policy: &PolicyProfile,
    ) -> Result<Digest, CartographyStoreError> {
        // 1. ReportOnly forbids all mutation.
        if policy.mode == ImplementationMode::ReportOnly {
            return Err(CartographyStoreError::PolicyDenied {
                reason: "ImplementationMode::ReportOnly forbids cartography store mutation".into(),
            });
        }

        // 2. A durable append is a HOST WRITE. Unlike the in-memory store, it
        //    requires allow_host_write — so DryRun (allow_host_write == false)
        //    cannot persist. Fail closed.
        if !policy.allow_host_write {
            return Err(CartographyStoreError::PolicyDenied {
                reason: "persistent cartography append requires allow_host_write \
                         (DryRun may not write host files)"
                    .into(),
            });
        }

        // 3. Scope must match the store.
        if commit.scope != self.manifest.scope {
            return Err(CartographyStoreError::ScopeMismatch);
        }

        // 4. Sequence must be gapless and monotonic.
        let expected_seq = self.manifest.head_sequence + 1;
        if commit.sequence != expected_seq {
            return Err(CartographyStoreError::SequenceViolation {
                expected: expected_seq,
                got: commit.sequence,
            });
        }

        // 5. Persist to disk first, then update the in-memory mirror. If the
        //    write fails the manifest is left untouched (fail-closed).
        self.write_line(&commit)?;
        let commit_id = commit.id;
        self.manifest = self.manifest.clone().with_commit(commit);
        Ok(commit_id)
    }

    fn read_manifest(&self) -> Result<CartographyStorageManifest, CartographyStoreError> {
        Ok(self.manifest.clone())
    }

    fn verify_integrity(
        &self,
        evidence_bundle_id: Digest,
    ) -> Result<CartographyIntegrityReport, CartographyStoreError> {
        // Re-read the durable copy from disk and verify it against the same
        // rules the in-memory store uses, so the report reflects what is
        // actually persisted, not just the RAM mirror.
        let on_disk = Self::open(
            self.path.clone(),
            self.manifest.scope.clone(),
            self.manifest.policy_id,
        )?;
        let entries = &on_disk.manifest.entries;

        if entries.is_empty() {
            return Ok(CartographyIntegrityReport::new(
                self.manifest.id,
                CartographyIntegrityStatus::Empty,
                0,
                evidence_bundle_id,
            ));
        }

        let mut expected_seq: u64 = 1;
        for entry in entries {
            if !entry.verify_id() {
                return Ok(CartographyIntegrityReport::new(
                    self.manifest.id,
                    CartographyIntegrityStatus::DigestMismatch { commit_id: entry.id },
                    expected_seq - 1,
                    evidence_bundle_id,
                ));
            }
            if entry.sequence != expected_seq {
                return Ok(CartographyIntegrityReport::new(
                    self.manifest.id,
                    CartographyIntegrityStatus::SequenceGap {
                        expected: expected_seq,
                        found: entry.sequence,
                    },
                    expected_seq - 1,
                    evidence_bundle_id,
                ));
            }
            expected_seq += 1;
        }

        Ok(CartographyIntegrityReport::new(
            self.manifest.id,
            CartographyIntegrityStatus::Intact,
            entries.len() as u64,
            evidence_bundle_id,
        ))
    }

    fn scope(&self) -> &CorpusScope {
        &self.manifest.scope
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::CartographyEntryKind;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    /// A unique temp path per test invocation (no external tempfile dep).
    fn temp_path(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("kosmo-store-{tag}-{nanos}.jsonl"));
        p
    }

    fn commit(scope: CorpusScope, seq: u64) -> CartographyStoreCommit {
        CartographyStoreCommit::new(
            scope,
            seq,
            d(format!("payload-{seq}").as_bytes()),
            CartographyEntryKind::EvidenceSummary,
            d(b"bundle"),
            d(b"pol"),
        )
    }

    fn op_approved() -> PolicyProfile {
        PolicyProfile::operator_approved()
    }

    #[test]
    fn report_only_denies_persist() {
        let path = temp_path("ro");
        let mut store =
            JsonlCartographyStore::open(&path, CorpusScope::LocalHostProject, d(b"pol")).unwrap();
        let res = store.append(commit(CorpusScope::LocalHostProject, 1), &PolicyProfile::default());
        assert!(matches!(res, Err(CartographyStoreError::PolicyDenied { .. })));
        assert!(!path.exists(), "no file may be created when persist is denied");
    }

    #[test]
    fn dry_run_cannot_persist_host_write_required() {
        // The emergent property: DryRun has allow_host_write == false, so a
        // durable append must be denied even though DryRun is not ReportOnly.
        let path = temp_path("dryrun");
        let mut store =
            JsonlCartographyStore::open(&path, CorpusScope::LocalHostProject, d(b"pol")).unwrap();
        let res = store.append(commit(CorpusScope::LocalHostProject, 1), &PolicyProfile::dry_run());
        match res {
            Err(CartographyStoreError::PolicyDenied { reason }) => {
                assert!(reason.contains("allow_host_write"));
            }
            other => panic!("expected PolicyDenied for DryRun, got {other:?}"),
        }
        assert!(!path.exists());
    }

    #[test]
    fn operator_approved_persists_and_reloads() {
        let path = temp_path("persist");
        {
            let mut store =
                JsonlCartographyStore::open(&path, CorpusScope::LocalHostProject, d(b"pol")).unwrap();
            store
                .append(commit(CorpusScope::LocalHostProject, 1), &op_approved())
                .unwrap();
            store
                .append(commit(CorpusScope::LocalHostProject, 2), &op_approved())
                .unwrap();
            assert_eq!(store.len(), 2);
        }
        // Reopen: the manifest must be reconstructed from disk identically.
        let reopened =
            JsonlCartographyStore::open(&path, CorpusScope::LocalHostProject, d(b"pol")).unwrap();
        assert_eq!(reopened.len(), 2);
        let report = reopened.verify_integrity(d(b"bundle")).unwrap();
        assert!(report.status.is_intact());
        assert_eq!(report.checked_count, 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn sequence_violation_is_rejected_and_not_persisted() {
        let path = temp_path("seq");
        let mut store =
            JsonlCartographyStore::open(&path, CorpusScope::LocalHostProject, d(b"pol")).unwrap();
        // First commit must be sequence 1; offering 2 fails closed.
        let res = store.append(commit(CorpusScope::LocalHostProject, 2), &op_approved());
        assert!(matches!(
            res,
            Err(CartographyStoreError::SequenceViolation { expected: 1, got: 2 })
        ));
        assert!(!path.exists());
    }

    #[test]
    fn scope_mismatch_is_rejected() {
        let path = temp_path("scope");
        let mut store =
            JsonlCartographyStore::open(&path, CorpusScope::LocalHostProject, d(b"pol")).unwrap();
        let res = store.append(commit(CorpusScope::WorkspaceFamily, 1), &op_approved());
        assert!(matches!(res, Err(CartographyStoreError::ScopeMismatch)));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn integrity_detects_tampering() {
        let path = temp_path("tamper");
        {
            let mut store =
                JsonlCartographyStore::open(&path, CorpusScope::LocalHostProject, d(b"pol")).unwrap();
            store
                .append(commit(CorpusScope::LocalHostProject, 1), &op_approved())
                .unwrap();
        }
        // Corrupt the persisted line by appending a forged commit whose stored
        // id will not match its recomputed digest after reload. We simulate
        // tampering by writing a line with a mismatched id field.
        let mut forged = commit(CorpusScope::LocalHostProject, 2);
        forged.id = d(b"forged-wrong-id"); // id no longer matches content
        {
            use std::io::Write as _;
            let mut f = OpenOptions::new().append(true).open(&path).unwrap();
            let line = serde_json::to_string(&forged).unwrap();
            writeln!(f, "{line}").unwrap();
        }
        let reopened =
            JsonlCartographyStore::open(&path, CorpusScope::LocalHostProject, d(b"pol")).unwrap();
        let report = reopened.verify_integrity(d(b"bundle")).unwrap();
        assert!(
            matches!(report.status, CartographyIntegrityStatus::DigestMismatch { .. }),
            "tampered commit must be detected, got {:?}",
            report.status
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn empty_store_integrity_is_empty() {
        let path = temp_path("empty");
        let store =
            JsonlCartographyStore::open(&path, CorpusScope::WorkspaceFamily, d(b"pol")).unwrap();
        let report = store.verify_integrity(d(b"bundle")).unwrap();
        assert!(matches!(report.status, CartographyIntegrityStatus::Empty));
        assert!(report.verify_id());
    }
}
