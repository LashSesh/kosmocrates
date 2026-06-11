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
use kosmo_hyphae::crystal::StructuralCrystalRecord;
use std::fmt;
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

        for (expected_seq, entry) in (1_u64..).zip(entries.iter()) {
            if !entry.verify_id() {
                return Ok(CartographyIntegrityReport::new(
                    self.manifest.id,
                    CartographyIntegrityStatus::DigestMismatch {
                        commit_id: entry.id,
                    },
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

// ────────────────────────────────────────────────────────────────────────────
// CrystalRecordStore
// ────────────────────────────────────────────────────────────────────────────

/// Error type for [`CrystalRecordStore`] operations.
#[derive(Debug)]
pub enum CrystalStoreError {
    Io { message: String },
    PolicyDenied { reason: String },
    IntegrityViolation { record_id: Digest },
}

impl fmt::Display for CrystalStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { message } => write!(f, "crystal store I/O: {message}"),
            Self::PolicyDenied { reason } => write!(f, "crystal store policy denied: {reason}"),
            Self::IntegrityViolation { record_id } => {
                write!(
                    f,
                    "crystal store integrity violation: record_id={record_id:?}"
                )
            }
        }
    }
}

impl std::error::Error for CrystalStoreError {}

/// An append-only, durable store for [`StructuralCrystalRecord`]s backed by a
/// JSONL file.
///
/// Each line of the backing file is the JSON serialization of one record.
/// Records are deduplicated by `record_id` — re-appending an already-stored
/// record is a no-op. Opening is a read-only operation; disk writes require
/// `allow_host_write` (same host-write invariant as [`JsonlCartographyStore`]).
///
/// The primary use-case is persisting the CAD library across integration runs
/// so the `IntegrationRunOptions::prior_crystals` slice can be pre-populated
/// from the previous session.
pub struct CrystalRecordStore {
    path: PathBuf,
    records: Vec<StructuralCrystalRecord>,
}

impl CrystalRecordStore {
    /// Open an existing JSONL store or create an empty (unwritten) one.
    ///
    /// Replays every line of an existing file, verifies each record's
    /// `record_id`, and rejects the file on any integrity failure.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, CrystalStoreError> {
        let path = path.into();
        let mut records: Vec<StructuralCrystalRecord> = Vec::new();

        if path.exists() {
            let file = File::open(&path).map_err(|e| CrystalStoreError::Io {
                message: format!("open {}: {e}", path.display()),
            })?;
            let reader = BufReader::new(file);
            for (lineno, line) in reader.lines().enumerate() {
                let line = line.map_err(|e| CrystalStoreError::Io {
                    message: format!("read line {}: {e}", lineno + 1),
                })?;
                if line.trim().is_empty() {
                    continue;
                }
                let record: StructuralCrystalRecord =
                    serde_json::from_str(&line).map_err(|e| CrystalStoreError::Io {
                        message: format!("parse line {}: {e}", lineno + 1),
                    })?;
                if !record.verify_id() {
                    return Err(CrystalStoreError::IntegrityViolation {
                        record_id: record.record_id,
                    });
                }
                records.push(record);
            }
        }

        Ok(Self { path, records })
    }

    /// Path of the backing JSONL file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// All records currently held in the store.
    pub fn records(&self) -> &[StructuralCrystalRecord] {
        &self.records
    }

    /// Number of records currently persisted.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the store holds no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Append a record to the store.
    ///
    /// Policy requirements (same as [`JsonlCartographyStore`]):
    /// - `ReportOnly` is denied (no mutation).
    /// - `allow_host_write` must be true (a durable append is a host write).
    ///
    /// Re-appending a record with the same `record_id` is silently deduplicated
    /// and returns `Ok(())`.
    pub fn append(
        &mut self,
        record: StructuralCrystalRecord,
        policy: &PolicyProfile,
    ) -> Result<(), CrystalStoreError> {
        if policy.mode == ImplementationMode::ReportOnly {
            return Err(CrystalStoreError::PolicyDenied {
                reason: "ImplementationMode::ReportOnly forbids crystal store mutation".into(),
            });
        }
        if !policy.allow_host_write {
            return Err(CrystalStoreError::PolicyDenied {
                reason: "crystal record append requires allow_host_write \
                         (DryRun may not write host files)"
                    .into(),
            });
        }

        // Dedup by record_id — idempotent append.
        if self.records.iter().any(|r| r.record_id == record.record_id) {
            return Ok(());
        }

        let line = serde_json::to_string(&record).map_err(|e| CrystalStoreError::Io {
            message: format!("serialize record: {e}"),
        })?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| CrystalStoreError::Io {
                message: format!("open for append {}: {e}", self.path.display()),
            })?;
        file.write_all(line.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .and_then(|_| file.flush())
            .and_then(|_| file.sync_all())
            .map_err(|e| CrystalStoreError::Io {
                message: format!("write {}: {e}", self.path.display()),
            })?;

        self.records.push(record);
        Ok(())
    }

    /// Re-verify every record's `record_id` against its content.
    ///
    /// Returns `Ok(count)` on success, or `Err(CrystalStoreError::IntegrityViolation)`
    /// on the first corrupted record.
    pub fn verify_integrity(&self) -> Result<usize, CrystalStoreError> {
        for record in &self.records {
            if !record.verify_id() {
                return Err(CrystalStoreError::IntegrityViolation {
                    record_id: record.record_id,
                });
            }
        }
        Ok(self.records.len())
    }
}

#[cfg(test)]
mod crystal_store_tests {
    use super::*;
    use kosmo_core::{
        AuthorityLabel, Digest, EvidenceBundle, EvidenceKind, EvidenceRef, ReplayStatus,
        TaintLabel, Q16,
    };
    use kosmo_hyphae::{
        assimilation::AssimilationDecision,
        crystal::StructuralCrystalCandidate,
        gates::GateCascade,
        structural_yield::{StructuralYield, StructuralYieldKind},
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn temp_path(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("kosmo-crystal-store-{tag}-{nanos}.jsonl"));
        p
    }

    fn make_record(seed: &[u8]) -> StructuralCrystalRecord {
        let policy = PolicyProfile::operator_approved();
        let ev = EvidenceBundle::seal(
            vec![EvidenceRef::new(d(seed), EvidenceKind::HostScan, "scan")],
            policy.id,
            ReplayStatus::Replayable,
        );
        let void_id = d(seed);
        let yield_ = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(void_id),
            None,
            TaintLabel::Clean,
            AuthorityLabel::Foundry,
            ev.bundle_id,
            policy.id,
        );
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&yield_, &ev);
        let decision = AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id);
        let candidate = StructuralCrystalCandidate::from_decision_with_signals(
            &decision,
            Some(void_id),
            Q16::ONE,
            Q16::HALF,
        );
        candidate
            .certify(ReplayStatus::Replayable)
            .expect("candidate must certify")
            .1
    }

    fn op_approved() -> PolicyProfile {
        PolicyProfile::operator_approved()
    }

    #[test]
    fn report_only_denies_append() {
        let path = temp_path("ro");
        let mut store = CrystalRecordStore::open(&path).unwrap();
        let record = make_record(b"r1");
        let res = store.append(record, &PolicyProfile::default());
        assert!(matches!(res, Err(CrystalStoreError::PolicyDenied { .. })));
        assert!(!path.exists());
    }

    #[test]
    fn dry_run_denies_append() {
        let path = temp_path("dryrun");
        let mut store = CrystalRecordStore::open(&path).unwrap();
        let record = make_record(b"r2");
        let res = store.append(record, &PolicyProfile::dry_run());
        match &res {
            Err(CrystalStoreError::PolicyDenied { reason }) => {
                assert!(reason.contains("allow_host_write"));
            }
            other => panic!("expected PolicyDenied, got {other:?}"),
        }
        assert!(!path.exists());
    }

    #[test]
    fn operator_approved_appends_and_reloads() {
        let path = temp_path("persist");
        let r1 = make_record(b"r1");
        let r2 = make_record(b"r2");
        let r1_id = r1.record_id;
        let r2_id = r2.record_id;
        {
            let mut store = CrystalRecordStore::open(&path).unwrap();
            store.append(r1, &op_approved()).unwrap();
            store.append(r2, &op_approved()).unwrap();
            assert_eq!(store.len(), 2);
        }
        let reopened = CrystalRecordStore::open(&path).unwrap();
        assert_eq!(reopened.len(), 2);
        assert!(reopened.records().iter().any(|r| r.record_id == r1_id));
        assert!(reopened.records().iter().any(|r| r.record_id == r2_id));
        assert_eq!(reopened.verify_integrity().unwrap(), 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn dedup_prevents_duplicate_records() {
        let path = temp_path("dedup");
        let record = make_record(b"dup");
        let id = record.record_id;
        let mut store = CrystalRecordStore::open(&path).unwrap();
        store.append(record.clone(), &op_approved()).unwrap();
        store.append(record, &op_approved()).unwrap(); // dedup — no-op
        assert_eq!(store.len(), 1);
        // Reload confirms only one line on disk.
        let reopened = CrystalRecordStore::open(&path).unwrap();
        assert_eq!(reopened.len(), 1);
        assert_eq!(reopened.records()[0].record_id, id);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn integrity_detects_tampering() {
        let path = temp_path("tamper");
        let record = make_record(b"tamper");
        {
            let mut store = CrystalRecordStore::open(&path).unwrap();
            store.append(record, &op_approved()).unwrap();
        }
        // Append a line with a corrupted record_id directly.
        {
            use std::io::Write as _;
            let mut forged = make_record(b"other");
            forged.record_id = d(b"wrong-id");
            let line = serde_json::to_string(&forged).unwrap();
            let mut f = OpenOptions::new().append(true).open(&path).unwrap();
            writeln!(f, "{line}").unwrap();
        }
        let res = CrystalRecordStore::open(&path);
        assert!(
            matches!(res, Err(CrystalStoreError::IntegrityViolation { .. })),
            "tampered record must be detected on open"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn empty_store_integrity_is_ok() {
        let path = temp_path("empty");
        let store = CrystalRecordStore::open(&path).unwrap();
        assert!(store.is_empty());
        assert_eq!(store.verify_integrity().unwrap(), 0);
    }

    #[test]
    fn records_carry_structural_signals() {
        let path = temp_path("signals");
        let policy = PolicyProfile::operator_approved();
        let ev = EvidenceBundle::seal(
            vec![EvidenceRef::new(d(b"v1"), EvidenceKind::HostScan, "scan")],
            policy.id,
            ReplayStatus::Replayable,
        );
        let void_id = d(b"void1");
        let yield_ = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(void_id),
            None,
            TaintLabel::Clean,
            AuthorityLabel::Foundry,
            ev.bundle_id,
            policy.id,
        );
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&yield_, &ev);
        let decision = AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id);
        let candidate = StructuralCrystalCandidate::from_decision_with_signals(
            &decision,
            Some(void_id),
            Q16::from_raw(49152), // 0.75
            Q16::HALF,
        );
        let (_, record) = candidate.certify(ReplayStatus::Replayable).unwrap();
        assert_eq!(record.rho_coherence, Q16::from_raw(49152));
        assert_eq!(record.omega_phase, Q16::HALF);
        assert_eq!(record.source_void_id, Some(void_id));

        let mut store = CrystalRecordStore::open(&path).unwrap();
        store.append(record.clone(), &op_approved()).unwrap();
        let reopened = CrystalRecordStore::open(&path).unwrap();
        let reloaded = &reopened.records()[0];
        assert_eq!(reloaded.record_id, record.record_id);
        assert_eq!(reloaded.rho_coherence, Q16::from_raw(49152));
        assert_eq!(reloaded.omega_phase, Q16::HALF);
        assert_eq!(reloaded.source_void_id, Some(void_id));
        let _ = std::fs::remove_file(&path);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// JsonlCartographyStore tests
// ────────────────────────────────────────────────────────────────────────────

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
        let res = store.append(
            commit(CorpusScope::LocalHostProject, 1),
            &PolicyProfile::default(),
        );
        assert!(matches!(
            res,
            Err(CartographyStoreError::PolicyDenied { .. })
        ));
        assert!(
            !path.exists(),
            "no file may be created when persist is denied"
        );
    }

    #[test]
    fn dry_run_cannot_persist_host_write_required() {
        // The emergent property: DryRun has allow_host_write == false, so a
        // durable append must be denied even though DryRun is not ReportOnly.
        let path = temp_path("dryrun");
        let mut store =
            JsonlCartographyStore::open(&path, CorpusScope::LocalHostProject, d(b"pol")).unwrap();
        let res = store.append(
            commit(CorpusScope::LocalHostProject, 1),
            &PolicyProfile::dry_run(),
        );
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
                JsonlCartographyStore::open(&path, CorpusScope::LocalHostProject, d(b"pol"))
                    .unwrap();
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
            Err(CartographyStoreError::SequenceViolation {
                expected: 1,
                got: 2
            })
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
                JsonlCartographyStore::open(&path, CorpusScope::LocalHostProject, d(b"pol"))
                    .unwrap();
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
            matches!(
                report.status,
                CartographyIntegrityStatus::DigestMismatch { .. }
            ),
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
