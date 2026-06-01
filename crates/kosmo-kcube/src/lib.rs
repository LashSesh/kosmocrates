//! Real `.kcube` archive executor for the Kosmocrates substrate.
//!
//! ## What this crate does
//!
//! It turns a set of [`KcubeArtifact`]s into a real `.kcube` file on disk
//! under a [`KcubeExportPolicy`], producing a content-addressed
//! [`KcubeWriteReport`] with optional roundtrip verification.  It is the
//! runtime counterpart of the data model in `kosmo-core::kcube`.
//!
//! ## Why it is a separate crate
//!
//! `kosmo-core` is the portable, process-free substrate (no host I/O).
//! Writing files is a host capability, so it lives here — exactly as
//! `kosmo-foundry` isolates process spawning.
//!
//! ## Archive format
//!
//! A `.kcube` file consists of two sections:
//!
//! ```text
//! [8 B]  magic       b"KCUBEPM\n"
//! [4 B]  version     0u32 LE
//! [8 B]  art_len     u64 LE — byte count of the artifact section
//! [art_len B]  artifact section (sorted entries; see below)
//! [8 B]  marker      b"KCUBMFST"
//! [4 B]  mfst_len    u32 LE
//! [mfst_len B]  JSON-serialized KcubePackage
//! ```
//!
//! **Artifact section layout** (covered by `KcubePackage.package_digest`):
//!
//! ```text
//! [4 B]  n_entries   u32 LE
//! for each entry, sorted lexicographically by path:
//!   [4 B]  path_len  u32 LE
//!   [path_len B]  UTF-8 path bytes
//!   [8 B]  data_len  u64 LE
//!   [data_len B]  artifact bytes
//! ```
//!
//! `package_digest = SHA-256(artifact_section_bytes)`.  The manifest is
//! appended after, so computing the artifact digest does not require parsing
//! the manifest, and the manifest binds `package_digest` for verification.
//!
//! ## Safety contract
//!
//! - **`allow_write = false` is inert.**  The executor returns
//!   `DeniedByPolicy` without touching the filesystem.
//! - **Kind allowlist is enforced.**  Every artifact kind is checked against
//!   `KcubeExportPolicy.allowed_artifact_kinds` before any write begins.
//! - **Overwrite guard.**  When `allow_overwrite = false` (the default) the
//!   executor will not silently replace an existing file.
//! - **Roundtrip verification.**  When `require_roundtrip_verification = true`
//!   (the default) the written file is read back and its artifact-section
//!   SHA-256 is compared to `package_digest`.  A mismatch yields
//!   `WrittenRoundtripFailed`.
//! - **No floats anywhere.**  `written_bytes` and `elapsed_ms` are `u64`.
//! - **Content-addressed reports.**  `KcubeWriteReport.id` and
//!   `KcubePackage.id` are SHA-256 digests of their content fields
//!   (INVARIANT-007).
//! - **Evidence is mandatory.**  `evidence_bundle_id ≠ ZERO` is the caller's
//!   responsibility (CROSS-006); the executor passes it through as-is.

use kosmo_core::{
    Digest, KcubeArtifactKind, KcubeExportPolicy, KcubeManifestEntry, KcubePackage,
    KcubeRoundtripVerification, KcubeWriteOutcome, KcubeWriteReport,
};
use std::path::PathBuf;
use std::time::Instant;

// ─── Archive format constants ─────────────────────────────────────────────────

const MAGIC: &[u8; 8] = b"KCUBEPM\n";
const VERSION: u32 = 0;
const MANIFEST_MARKER: &[u8; 8] = b"KCUBMFST";

/// Byte offset of the `art_len` field in the file header.
const ARTIFACT_LEN_OFFSET: usize = 12;
/// First byte of the artifact section (magic[8] + version[4] + art_len[8]).
const ARTIFACT_START: usize = 20;

// ─── Public types ─────────────────────────────────────────────────────────────

/// One artifact to be packed into a `.kcube` archive.
#[derive(Clone, Debug)]
pub struct KcubeArtifact {
    pub kind: KcubeArtifactKind,
    /// Relative path within the archive (e.g. `"crystals/c1.bin"`).
    pub path: String,
    pub bytes: Vec<u8>,
}

impl KcubeArtifact {
    pub fn new(kind: KcubeArtifactKind, path: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self { kind, path: path.into(), bytes }
    }
}

/// Error variants returned when reading or parsing a `.kcube` file fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KcubeReadError {
    Io(String),
    TooShort { len: usize },
    InvalidMagic,
    InvalidVersion { actual: u32 },
    ArtifactSectionTruncated,
    InvalidManifestMarker,
    ManifestTruncated,
    ManifestParseError(String),
}

impl std::fmt::Display for KcubeReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::TooShort { len } => write!(f, "file too short ({len} bytes)"),
            Self::InvalidMagic => write!(f, "invalid magic bytes"),
            Self::InvalidVersion { actual } => write!(f, "unknown format version {actual}"),
            Self::ArtifactSectionTruncated => write!(f, "artifact section truncated"),
            Self::InvalidManifestMarker => write!(f, "manifest marker not found at expected offset"),
            Self::ManifestTruncated => write!(f, "manifest section truncated"),
            Self::ManifestParseError(e) => write!(f, "manifest JSON parse error: {e}"),
        }
    }
}

// ─── KcubeExecutor ────────────────────────────────────────────────────────────

/// Drives real `.kcube` archive write and read operations against a target
/// directory.
pub struct KcubeExecutor {
    target_dir: PathBuf,
}

impl KcubeExecutor {
    /// Create an executor that reads and writes `.kcube` files in `target_dir`.
    pub fn new(target_dir: impl Into<PathBuf>) -> Self {
        Self { target_dir: target_dir.into() }
    }

    /// Write a `.kcube` archive from the provided artifacts under `policy`.
    ///
    /// The output file is named `<scope-slug>-seq<sequence>.kcube`.  Returns a
    /// content-addressed [`KcubeWriteReport`] for every outcome including
    /// policy denials (no panic on failure).
    pub fn write(
        &self,
        scope: &str,
        artifacts: Vec<KcubeArtifact>,
        policy: &KcubeExportPolicy,
        evidence_bundle_id: Digest,
        sequence: u64,
    ) -> KcubeWriteReport {
        let started = Instant::now();
        let elapsed_ms = || started.elapsed().as_millis() as u64;

        // Gate 1: write permission
        if !policy.allow_write {
            return KcubeWriteReport::new(
                Digest::ZERO,
                policy.id,
                KcubeWriteOutcome::DeniedByPolicy { reason: "allow_write is false".into() },
                None,
                0,
                None,
                vec!["allow_write is false".into()],
                evidence_bundle_id,
                elapsed_ms(),
            );
        }

        // Gate 2: artifact kind allowlist
        for art in &artifacts {
            if !policy.allowed_artifact_kinds.is_empty()
                && !policy.is_artifact_kind_allowed(&art.kind)
            {
                let reason = format!(
                    "artifact kind {:?} not in policy allowed list",
                    art.kind
                );
                return KcubeWriteReport::new(
                    Digest::ZERO,
                    policy.id,
                    KcubeWriteOutcome::DeniedByPolicy { reason: reason.clone() },
                    None,
                    0,
                    None,
                    vec![reason],
                    evidence_bundle_id,
                    elapsed_ms(),
                );
            }
        }

        // Sort artifacts by path for a deterministic archive
        let mut sorted = artifacts;
        sorted.sort_by(|a, b| a.path.cmp(&b.path));

        // Encode the artifact section and bind package_digest to it
        let artifact_section = encode_artifact_section(&sorted);
        let package_digest = Digest::of_bytes(&artifact_section);

        // Build manifest entries (one per artifact)
        let entries: Vec<KcubeManifestEntry> = sorted
            .iter()
            .map(|art| {
                KcubeManifestEntry::new(
                    Digest::of_bytes(&art.bytes),
                    art.kind.clone(),
                    art.path.clone(),
                    art.bytes.len() as u64,
                )
            })
            .collect();

        // KcubePackage uses the external policy reference (policy.policy_id),
        // not the policy struct digest (policy.id).
        let package = KcubePackage::new(
            package_digest,
            entries,
            scope,
            policy.policy_id,
            evidence_bundle_id,
            sequence,
        );

        let file_bytes = build_file_bytes(&artifact_section, &package);
        let file_name = kcube_file_name(scope, sequence);
        let out_path = self.target_dir.join(&file_name);

        // Gate 3: overwrite guard
        if out_path.exists() && !policy.allow_overwrite {
            let reason =
                format!("target {file_name} exists and allow_overwrite is false");
            return KcubeWriteReport::new(
                package.id,
                policy.id,
                KcubeWriteOutcome::DeniedByPolicy { reason: reason.clone() },
                None,
                0,
                None,
                vec![reason],
                evidence_bundle_id,
                elapsed_ms(),
            );
        }

        // Ensure target directory exists
        if let Err(e) = std::fs::create_dir_all(&self.target_dir) {
            return KcubeWriteReport::new(
                package.id,
                policy.id,
                KcubeWriteOutcome::Failed { reason: format!("create_dir_all: {e}") },
                None,
                0,
                None,
                vec![e.to_string()],
                evidence_bundle_id,
                elapsed_ms(),
            );
        }

        // Write the archive
        if let Err(e) = std::fs::write(&out_path, &file_bytes) {
            return KcubeWriteReport::new(
                package.id,
                policy.id,
                KcubeWriteOutcome::Failed { reason: format!("write: {e}") },
                None,
                0,
                None,
                vec![e.to_string()],
                evidence_bundle_id,
                elapsed_ms(),
            );
        }

        let written_bytes = file_bytes.len() as u64;
        let written_path_digest =
            Digest::of_bytes(out_path.to_string_lossy().as_bytes());

        // Roundtrip verification: re-read and re-compute artifact SHA-256
        let roundtrip = if policy.require_roundtrip_verification {
            let observed = match std::fs::read(&out_path) {
                Ok(buf) => observed_package_digest(&buf).unwrap_or(Digest::ZERO),
                Err(_) => Digest::ZERO,
            };
            Some(KcubeRoundtripVerification::new(
                package_digest,
                observed,
                evidence_bundle_id,
            ))
        } else {
            None
        };

        let outcome = match &roundtrip {
            Some(rt) if !rt.verification_passed => KcubeWriteOutcome::WrittenRoundtripFailed,
            _ => KcubeWriteOutcome::Written,
        };

        KcubeWriteReport::new(
            package.id,
            policy.id,
            outcome,
            Some(written_path_digest),
            written_bytes,
            roundtrip,
            vec![],
            evidence_bundle_id,
            elapsed_ms(),
        )
    }

    /// Read and parse a `.kcube` file from the target directory by file name.
    pub fn read(&self, file_name: &str) -> Result<KcubePackage, KcubeReadError> {
        let bytes = std::fs::read(self.target_dir.join(file_name))
            .map_err(|e| KcubeReadError::Io(e.to_string()))?;
        parse_kcube_file(&bytes)
    }
}

// ─── Archive codec ────────────────────────────────────────────────────────────

/// Encodes the artifact section — the byte range covered by `package_digest`.
fn encode_artifact_section(sorted: &[KcubeArtifact]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&(sorted.len() as u32).to_le_bytes());
    for art in sorted {
        let pb = art.path.as_bytes();
        buf.extend_from_slice(&(pb.len() as u32).to_le_bytes());
        buf.extend_from_slice(pb);
        buf.extend_from_slice(&(art.bytes.len() as u64).to_le_bytes());
        buf.extend_from_slice(&art.bytes);
    }
    buf
}

/// Assembles the complete `.kcube` file bytes.
fn build_file_bytes(artifact_section: &[u8], package: &KcubePackage) -> Vec<u8> {
    let manifest_json =
        serde_json::to_vec(package).expect("KcubePackage is always JSON-serializable");
    let capacity = ARTIFACT_START + artifact_section.len() + 12 + manifest_json.len();
    let mut buf = Vec::with_capacity(capacity);
    buf.extend_from_slice(MAGIC);
    buf.extend_from_slice(&VERSION.to_le_bytes());
    buf.extend_from_slice(&(artifact_section.len() as u64).to_le_bytes());
    buf.extend_from_slice(artifact_section);
    buf.extend_from_slice(MANIFEST_MARKER);
    buf.extend_from_slice(&(manifest_json.len() as u32).to_le_bytes());
    buf.extend_from_slice(&manifest_json);
    buf
}

/// Recomputes `package_digest` from a raw `.kcube` file byte slice.
///
/// Returns `None` if the header is malformed or truncated.
fn observed_package_digest(file_bytes: &[u8]) -> Option<Digest> {
    if file_bytes.len() < ARTIFACT_START {
        return None;
    }
    if &file_bytes[0..8] != MAGIC {
        return None;
    }
    let version = u32::from_le_bytes(file_bytes[8..12].try_into().ok()?);
    if version != VERSION {
        return None;
    }
    let art_len = u64::from_le_bytes(
        file_bytes[ARTIFACT_LEN_OFFSET..ARTIFACT_START].try_into().ok()?,
    ) as usize;
    let art_end = ARTIFACT_START.checked_add(art_len)?;
    if art_end > file_bytes.len() {
        return None;
    }
    Some(Digest::of_bytes(&file_bytes[ARTIFACT_START..art_end]))
}

/// Parses a `.kcube` file byte slice and returns the embedded [`KcubePackage`].
///
/// This is the public entry point for import/verification workflows.
pub fn parse_kcube_file(file_bytes: &[u8]) -> Result<KcubePackage, KcubeReadError> {
    if file_bytes.len() < ARTIFACT_START {
        return Err(KcubeReadError::TooShort { len: file_bytes.len() });
    }
    if &file_bytes[0..8] != MAGIC {
        return Err(KcubeReadError::InvalidMagic);
    }
    let version = u32::from_le_bytes(file_bytes[8..12].try_into().unwrap());
    if version != VERSION {
        return Err(KcubeReadError::InvalidVersion { actual: version });
    }
    let art_len = u64::from_le_bytes(
        file_bytes[ARTIFACT_LEN_OFFSET..ARTIFACT_START].try_into().unwrap(),
    ) as usize;
    let art_end = ARTIFACT_START
        .checked_add(art_len)
        .filter(|&e| e <= file_bytes.len())
        .ok_or(KcubeReadError::ArtifactSectionTruncated)?;

    let mfst_marker_start = art_end;
    if mfst_marker_start + 8 > file_bytes.len()
        || &file_bytes[mfst_marker_start..mfst_marker_start + 8] != MANIFEST_MARKER
    {
        return Err(KcubeReadError::InvalidManifestMarker);
    }

    let mfst_len_start = mfst_marker_start + 8;
    if mfst_len_start + 4 > file_bytes.len() {
        return Err(KcubeReadError::ManifestTruncated);
    }
    let mfst_len = u32::from_le_bytes(
        file_bytes[mfst_len_start..mfst_len_start + 4].try_into().unwrap(),
    ) as usize;

    let mfst_data_start = mfst_len_start + 4;
    let mfst_data_end = mfst_data_start
        .checked_add(mfst_len)
        .filter(|&e| e <= file_bytes.len())
        .ok_or(KcubeReadError::ManifestTruncated)?;

    serde_json::from_slice(&file_bytes[mfst_data_start..mfst_data_end])
        .map_err(|e| KcubeReadError::ManifestParseError(e.to_string()))
}

// ─── File-name helpers ────────────────────────────────────────────────────────

/// Derives a safe `.kcube` file name from `scope` and `sequence`.
///
/// Non-alphanumeric characters in `scope` are replaced with `-`.
pub fn kcube_file_name(scope: &str, sequence: u64) -> String {
    let slug: String = scope
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    format!("{slug}-seq{sequence}.kcube")
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::KcubeArtifactKind;

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn bundle_id() -> Digest {
        d(b"evidence-bundle")
    }

    fn policy_id() -> Digest {
        d(b"policy-ref")
    }

    fn target_dir() -> Digest {
        d(b"target-dir")
    }

    fn write_once_policy() -> KcubeExportPolicy {
        KcubeExportPolicy::write_once(
            policy_id(),
            target_dir(),
            vec![
                KcubeArtifactKind::StructuralCrystal,
                KcubeArtifactKind::EvidenceBundle,
            ],
        )
    }

    fn report_only_policy() -> KcubeExportPolicy {
        KcubeExportPolicy::report_only(policy_id(), target_dir())
    }

    fn crystal_artifact(tag: &str) -> KcubeArtifact {
        KcubeArtifact::new(
            KcubeArtifactKind::StructuralCrystal,
            format!("crystals/{tag}.bin"),
            format!("payload-{tag}").into_bytes(),
        )
    }

    fn tmp_dir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join("kosmo-kcube-tests").join(tag);
        std::fs::remove_dir_all(&p).ok();
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    // ── file name ──────────────────────────────────────────────────────────

    #[test]
    fn file_name_safe_chars() {
        assert_eq!(kcube_file_name("kosmo-core", 1), "kosmo-core-seq1.kcube");
    }

    #[test]
    fn file_name_replaces_spaces() {
        assert_eq!(kcube_file_name("my scope", 3), "my-scope-seq3.kcube");
    }

    #[test]
    fn file_name_replaces_slashes() {
        assert_eq!(kcube_file_name("a/b", 0), "a-b-seq0.kcube");
    }

    // ── codec round-trip (in-memory) ──────────────────────────────────────

    #[test]
    fn codec_roundtrip_zero_artifacts() {
        let arts: Vec<KcubeArtifact> = vec![];
        let art_sec = encode_artifact_section(&arts);
        let pkg_digest = Digest::of_bytes(&art_sec);
        let pkg = KcubePackage::new(pkg_digest, vec![], "scope", policy_id(), bundle_id(), 1);
        let file = build_file_bytes(&art_sec, &pkg);
        let observed = observed_package_digest(&file).unwrap();
        assert_eq!(observed, pkg_digest);
    }

    #[test]
    fn codec_roundtrip_multiple_artifacts() {
        let arts = vec![crystal_artifact("a"), crystal_artifact("b")];
        let art_sec = encode_artifact_section(&arts);
        let pkg_digest = Digest::of_bytes(&art_sec);
        let pkg = KcubePackage::new(pkg_digest, vec![], "s", policy_id(), bundle_id(), 1);
        let file = build_file_bytes(&art_sec, &pkg);
        assert_eq!(observed_package_digest(&file).unwrap(), pkg_digest);
    }

    #[test]
    fn parse_kcube_file_returns_package() {
        let arts = vec![crystal_artifact("x")];
        let art_sec = encode_artifact_section(&arts);
        let pkg_digest = Digest::of_bytes(&art_sec);
        let entries = vec![KcubeManifestEntry::new(
            d(b"art"),
            KcubeArtifactKind::StructuralCrystal,
            "crystals/x.bin",
            1,
        )];
        let pkg = KcubePackage::new(pkg_digest, entries, "sc", policy_id(), bundle_id(), 7);
        let file = build_file_bytes(&art_sec, &pkg);
        let parsed = parse_kcube_file(&file).unwrap();
        assert_eq!(parsed.id, pkg.id);
        assert!(parsed.verify_id());
    }

    #[test]
    fn codec_sort_order_is_deterministic() {
        let mut arts1 = vec![crystal_artifact("z"), crystal_artifact("a")];
        let mut arts2 = vec![crystal_artifact("a"), crystal_artifact("z")];
        arts1.sort_by(|a, b| a.path.cmp(&b.path));
        arts2.sort_by(|a, b| a.path.cmp(&b.path));
        let sec1 = encode_artifact_section(&arts1);
        let sec2 = encode_artifact_section(&arts2);
        assert_eq!(Digest::of_bytes(&sec1), Digest::of_bytes(&sec2));
    }

    // ── parse error cases ─────────────────────────────────────────────────

    #[test]
    fn parse_empty_bytes_too_short() {
        let e = parse_kcube_file(&[]).unwrap_err();
        assert_eq!(e, KcubeReadError::TooShort { len: 0 });
    }

    #[test]
    fn parse_wrong_magic() {
        let mut file = vec![0u8; ARTIFACT_START + 20];
        file[0..8].copy_from_slice(b"NOTMAGIC");
        assert_eq!(parse_kcube_file(&file).unwrap_err(), KcubeReadError::InvalidMagic);
    }

    #[test]
    fn parse_wrong_version() {
        let mut file = vec![0u8; ARTIFACT_START + 20];
        file[0..8].copy_from_slice(MAGIC);
        file[8..12].copy_from_slice(&99u32.to_le_bytes());
        assert_eq!(
            parse_kcube_file(&file).unwrap_err(),
            KcubeReadError::InvalidVersion { actual: 99 }
        );
    }

    #[test]
    fn parse_artifact_section_truncated() {
        let mut file = vec![0u8; ARTIFACT_START + 4];
        file[0..8].copy_from_slice(MAGIC);
        file[8..12].copy_from_slice(&VERSION.to_le_bytes());
        // art_len claims 1000 bytes but file only has 4 bytes after header
        file[ARTIFACT_LEN_OFFSET..ARTIFACT_START].copy_from_slice(&1000u64.to_le_bytes());
        assert_eq!(
            parse_kcube_file(&file).unwrap_err(),
            KcubeReadError::ArtifactSectionTruncated
        );
    }

    // ── policy gates ──────────────────────────────────────────────────────

    #[test]
    fn write_denied_when_allow_write_false() {
        let dir = tmp_dir("allow-write-false");
        let exec = KcubeExecutor::new(&dir);
        let policy = report_only_policy();
        let report = exec.write("scope", vec![crystal_artifact("a")], &policy, bundle_id(), 1);
        assert!(
            matches!(&report.outcome, KcubeWriteOutcome::DeniedByPolicy { reason } if reason.contains("allow_write")),
            "expected DeniedByPolicy, got {:?}", report.outcome
        );
        assert_eq!(report.written_bytes, 0);
        assert!(report.written_path_digest.is_none());
        assert!(report.verify_id());
    }

    #[test]
    fn write_denied_for_disallowed_kind() {
        let dir = tmp_dir("kind-denied");
        let exec = KcubeExecutor::new(&dir);
        // Policy only allows StructuralCrystal; we send a PolicyProfile artifact
        let policy = KcubeExportPolicy::write_once(
            policy_id(),
            target_dir(),
            vec![KcubeArtifactKind::StructuralCrystal],
        );
        let bad_art = KcubeArtifact::new(
            KcubeArtifactKind::PolicyProfile,
            "policy.json",
            b"{}".to_vec(),
        );
        let report = exec.write("scope", vec![bad_art], &policy, bundle_id(), 1);
        assert!(
            matches!(&report.outcome, KcubeWriteOutcome::DeniedByPolicy { reason } if reason.contains("PolicyProfile")),
            "expected DeniedByPolicy, got {:?}", report.outcome
        );
        assert!(report.verify_id());
    }

    #[test]
    fn write_denied_on_overwrite_by_default() {
        let dir = tmp_dir("overwrite-denied");
        let exec = KcubeExecutor::new(&dir);
        let policy = write_once_policy();
        // First write succeeds
        let r1 = exec.write("scope", vec![crystal_artifact("c")], &policy, bundle_id(), 5);
        assert!(r1.outcome.is_written(), "first write should succeed");
        // Second write to same sequence should be blocked
        let r2 = exec.write("scope", vec![crystal_artifact("d")], &policy, bundle_id(), 5);
        assert!(
            matches!(&r2.outcome, KcubeWriteOutcome::DeniedByPolicy { reason } if reason.contains("allow_overwrite")),
            "expected DeniedByPolicy on overwrite, got {:?}", r2.outcome
        );
        assert!(r2.verify_id());
    }

    // ── successful write ──────────────────────────────────────────────────

    #[test]
    fn write_creates_file_and_report_is_written() {
        let dir = tmp_dir("write-success");
        let exec = KcubeExecutor::new(&dir);
        let report = exec.write(
            "test-scope",
            vec![crystal_artifact("a"), crystal_artifact("b")],
            &write_once_policy(),
            bundle_id(),
            1,
        );
        assert!(report.outcome.is_written(), "{:?}", report.outcome);
        assert!(report.written_bytes > 0);
        assert!(report.written_path_digest.is_some());
        assert!(report.verify_id());
        // File must exist on disk
        let expected_name = kcube_file_name("test-scope", 1);
        assert!(dir.join(&expected_name).exists());
    }

    #[test]
    fn write_evidence_bound_in_report() {
        let dir = tmp_dir("evidence-bound");
        let exec = KcubeExecutor::new(&dir);
        let report = exec.write("sc", vec![crystal_artifact("e")], &write_once_policy(), bundle_id(), 2);
        assert_eq!(report.evidence_bundle_id, bundle_id());
        assert_ne!(report.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn write_roundtrip_passes_by_default() {
        let dir = tmp_dir("roundtrip-passes");
        let exec = KcubeExecutor::new(&dir);
        let report = exec.write("rt-scope", vec![crystal_artifact("r")], &write_once_policy(), bundle_id(), 1);
        assert!(report.outcome.is_written(), "{:?}", report.outcome);
        assert!(report.roundtrip_passed(), "roundtrip verification must pass");
        let rt = report.roundtrip.as_ref().unwrap();
        assert!(rt.verify_id());
    }

    #[test]
    fn write_no_roundtrip_when_policy_skips() {
        let dir = tmp_dir("no-roundtrip");
        let exec = KcubeExecutor::new(&dir);
        let mut policy = write_once_policy();
        policy.require_roundtrip_verification = false;
        let report = exec.write("sc", vec![crystal_artifact("n")], &policy, bundle_id(), 1);
        assert!(report.outcome.is_written(), "{:?}", report.outcome);
        assert!(report.roundtrip.is_none());
    }

    #[test]
    fn write_package_id_is_content_addressed() {
        let dir = tmp_dir("pkg-id-ca");
        let exec = KcubeExecutor::new(&dir);
        let r = exec.write("sc", vec![crystal_artifact("p")], &write_once_policy(), bundle_id(), 1);
        assert!(r.outcome.is_written(), "{:?}", r.outcome);
        // package_id in the report must be the id of the KcubePackage
        assert_ne!(r.package_id, Digest::ZERO);
        assert!(r.verify_id());
    }

    // ── read back ─────────────────────────────────────────────────────────

    #[test]
    fn read_parses_written_archive() {
        let dir = tmp_dir("read-parses");
        let exec = KcubeExecutor::new(&dir);
        let r = exec.write(
            "read-scope",
            vec![crystal_artifact("r1"), crystal_artifact("r2")],
            &write_once_policy(),
            bundle_id(),
            42,
        );
        assert!(r.outcome.is_written(), "{:?}", r.outcome);
        let file_name = kcube_file_name("read-scope", 42);
        let pkg = exec.read(&file_name).expect("read must succeed");
        assert!(pkg.verify_id());
        assert_eq!(pkg.scope, "read-scope");
        assert_eq!(pkg.entry_count(), 2);
        assert_eq!(pkg.created_at_sequence, 42);
    }

    #[test]
    fn read_package_digest_matches_written_digest() {
        let dir = tmp_dir("read-digest-match");
        let exec = KcubeExecutor::new(&dir);
        let report = exec.write("sc", vec![crystal_artifact("d")], &write_once_policy(), bundle_id(), 7);
        assert!(report.outcome.is_written(), "{:?}", report.outcome);
        let file_name = kcube_file_name("sc", 7);
        let pkg = exec.read(&file_name).unwrap();
        // The package_digest stored in the manifest must match the artifact section SHA-256
        let file_bytes = std::fs::read(dir.join(&file_name)).unwrap();
        let observed = observed_package_digest(&file_bytes).unwrap();
        assert_eq!(pkg.package_digest, observed);
    }

    #[test]
    fn read_error_for_missing_file() {
        let dir = tmp_dir("read-missing");
        let exec = KcubeExecutor::new(&dir);
        let result = exec.read("nonexistent.kcube");
        assert!(matches!(result, Err(KcubeReadError::Io(_))));
    }

    // ── invariant cross-checks ─────────────────────────────────────────────

    #[test]
    fn cross006_evidence_non_zero_in_denied_report() {
        let dir = tmp_dir("cross006-denied");
        let exec = KcubeExecutor::new(&dir);
        let report = exec.write("s", vec![], &report_only_policy(), bundle_id(), 1);
        assert_ne!(report.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn cross006_evidence_non_zero_in_write_report() {
        let dir = tmp_dir("cross006-write");
        let exec = KcubeExecutor::new(&dir);
        let report = exec.write("s", vec![crystal_artifact("v")], &write_once_policy(), bundle_id(), 1);
        assert_ne!(report.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn no_floats_in_written_bytes_and_elapsed() {
        let dir = tmp_dir("no-floats");
        let exec = KcubeExecutor::new(&dir);
        let r = exec.write("s", vec![], &write_once_policy(), bundle_id(), 1);
        // written_bytes and elapsed_ms are u64 — compile-time guarantee
        let _: u64 = r.written_bytes;
        let _: u64 = r.elapsed_ms;
        assert!(r.verify_id());
    }
}
