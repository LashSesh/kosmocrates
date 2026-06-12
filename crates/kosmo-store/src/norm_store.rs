//! Durable, append-only store for the norm organ: `norms.jsonl` +
//! `observations.jsonl` in a caller-supplied directory (never a home
//! directory — the caller owns placement, the store owns discipline).
//!
//! Discipline (the same host-write invariant as the crystal and cartography
//! stores):
//! - Opening is read-only and permitted in any policy mode; nothing is
//!   created on disk until the first successful append.
//! - Appending requires a non-`ReportOnly` mode **and** `allow_host_write`.
//! - Every record is verified on load (`verify_id`, and for norms the full
//!   anti-disease [`Norm::validate`]); corruption is a **hard error**, never
//!   a skip.
//!
//! Governance encoding: promotion (arming a trigger) re-appends the same
//! `norm_id` with the trigger set — the file is the audit trail, and the
//! loader resolves each `norm_id` to its **latest** occurrence. This works
//! precisely because the trigger is excluded from the norm's content hash.

use kosmo_core::{Digest, ImplementationMode, PolicyProfile};
use kosmo_hyphae::{FacetBundleObservation, Norm};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const NORMS_FILE: &str = "norms.jsonl";
const OBSERVATIONS_FILE: &str = "observations.jsonl";

/// Error type for [`NormStore`] operations.
#[derive(Debug)]
pub enum NormStoreError {
    Io {
        message: String,
    },
    PolicyDenied {
        reason: String,
    },
    /// A stored record's id does not match its content, or a stored norm
    /// fails validation — the store refuses to open over a corrupt file.
    IntegrityViolation {
        id: Digest,
        detail: String,
    },
    UnknownNorm {
        norm_id: Digest,
    },
    InvalidTrigger {
        reason: String,
    },
}

impl fmt::Display for NormStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { message } => write!(f, "norm store I/O: {message}"),
            Self::PolicyDenied { reason } => write!(f, "norm store policy denied: {reason}"),
            Self::IntegrityViolation { id, detail } => {
                write!(f, "norm store integrity violation for {id:?}: {detail}")
            }
            Self::UnknownNorm { norm_id } => write!(f, "unknown norm {norm_id:?}"),
            Self::InvalidTrigger { reason } => write!(f, "invalid trigger: {reason}"),
        }
    }
}

impl std::error::Error for NormStoreError {}

/// The norm organ's durable memory. **Starts empty**: there is no built-in
/// catalog anywhere in the system (the Wonderlamp rejection list, enforced
/// structurally).
pub struct NormStore {
    dir: PathBuf,
    /// Latest version per `norm_id`, in first-seen order.
    norms: Vec<Norm>,
    observations: Vec<FacetBundleObservation>,
}

impl NormStore {
    /// Open the store rooted at `dir` (the directory need not exist yet —
    /// it is created on the first append, not on open).
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, NormStoreError> {
        let dir = dir.into();
        let mut store = Self {
            dir,
            norms: Vec::new(),
            observations: Vec::new(),
        };

        let norms_path = store.dir.join(NORMS_FILE);
        if norms_path.exists() {
            for norm in read_jsonl::<Norm>(&norms_path)? {
                if !norm.verify_id() {
                    return Err(NormStoreError::IntegrityViolation {
                        id: norm.norm_id,
                        detail: "norm_id does not match content".into(),
                    });
                }
                if let Err(e) = norm.validate() {
                    return Err(NormStoreError::IntegrityViolation {
                        id: norm.norm_id,
                        detail: e.to_string(),
                    });
                }
                store.upsert_norm(norm);
            }
        }

        let obs_path = store.dir.join(OBSERVATIONS_FILE);
        if obs_path.exists() {
            for obs in read_jsonl::<FacetBundleObservation>(&obs_path)? {
                if !obs.verify_id() {
                    return Err(NormStoreError::IntegrityViolation {
                        id: obs.observation_id,
                        detail: "observation_id does not match content".into(),
                    });
                }
                if !store
                    .observations
                    .iter()
                    .any(|o| o.observation_id == obs.observation_id)
                {
                    store.observations.push(obs);
                }
            }
        }

        Ok(store)
    }

    /// Directory holding the JSONL files.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Latest version of every norm, in first-seen order.
    pub fn norms(&self) -> &[Norm] {
        &self.norms
    }

    pub fn get(&self, norm_id: &Digest) -> Option<&Norm> {
        self.norms.iter().find(|n| &n.norm_id == norm_id)
    }

    pub fn observations(&self) -> &[FacetBundleObservation] {
        &self.observations
    }

    /// In-memory upsert: replace the existing version of this `norm_id` or
    /// append at the end (first-seen order preserved).
    fn upsert_norm(&mut self, norm: Norm) {
        match self.norms.iter_mut().find(|n| n.norm_id == norm.norm_id) {
            Some(slot) => *slot = norm,
            None => self.norms.push(norm),
        }
    }

    fn check_policy(policy: &PolicyProfile) -> Result<(), NormStoreError> {
        if policy.mode == ImplementationMode::ReportOnly {
            return Err(NormStoreError::PolicyDenied {
                reason: "ImplementationMode::ReportOnly forbids norm store mutation".into(),
            });
        }
        if !policy.allow_host_write {
            return Err(NormStoreError::PolicyDenied {
                reason: "norm store append requires allow_host_write \
                         (DryRun may not write host files)"
                    .into(),
            });
        }
        Ok(())
    }

    fn append_line<T: serde::Serialize>(
        &self,
        file: &str,
        record: &T,
    ) -> Result<(), NormStoreError> {
        std::fs::create_dir_all(&self.dir).map_err(|e| NormStoreError::Io {
            message: format!("create {}: {e}", self.dir.display()),
        })?;
        let path = self.dir.join(file);
        let line = serde_json::to_string(record).map_err(|e| NormStoreError::Io {
            message: format!("serialize record: {e}"),
        })?;
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| NormStoreError::Io {
                message: format!("open for append {}: {e}", path.display()),
            })?;
        f.write_all(line.as_bytes())
            .and_then(|_| f.write_all(b"\n"))
            .and_then(|_| f.flush())
            .and_then(|_| f.sync_all())
            .map_err(|e| NormStoreError::Io {
                message: format!("write {}: {e}", path.display()),
            })
    }

    /// Append a norm (or a new *version* of a known `norm_id`, e.g. a
    /// promotion). The norm is re-validated at the door; appending the
    /// version already stored is an idempotent no-op.
    pub fn append_norm(
        &mut self,
        norm: &Norm,
        policy: &PolicyProfile,
    ) -> Result<(), NormStoreError> {
        Self::check_policy(policy)?;
        if let Err(e) = norm.validate() {
            return Err(NormStoreError::IntegrityViolation {
                id: norm.norm_id,
                detail: e.to_string(),
            });
        }
        if self.get(&norm.norm_id) == Some(norm) {
            return Ok(()); // identical version — idempotent
        }
        self.append_line(NORMS_FILE, norm)?;
        self.upsert_norm(norm.clone());
        Ok(())
    }

    /// Append an observation. Re-appending a known `observation_id` is an
    /// idempotent no-op.
    pub fn append_observation(
        &mut self,
        obs: &FacetBundleObservation,
        policy: &PolicyProfile,
    ) -> Result<(), NormStoreError> {
        Self::check_policy(policy)?;
        if !obs.verify_id() {
            return Err(NormStoreError::IntegrityViolation {
                id: obs.observation_id,
                detail: "observation_id does not match content".into(),
            });
        }
        if self
            .observations
            .iter()
            .any(|o| o.observation_id == obs.observation_id)
        {
            return Ok(());
        }
        self.append_line(OBSERVATIONS_FILE, obs)?;
        self.observations.push(obs.clone());
        Ok(())
    }

    /// The operator's governance step: arm `trigger` on a stored norm. The
    /// promoted version is appended to the audit trail (same `norm_id`,
    /// trigger now set) and returned. The trigger must be a single
    /// `[a-z0-9_-]` word; reserved-word collision against the wish grammar
    /// is the *caller's* check (the grammar lives in `kosmo-intent`, which
    /// this crate deliberately does not depend on).
    pub fn promote(
        &mut self,
        norm_id: Digest,
        trigger: &str,
        policy: &PolicyProfile,
    ) -> Result<Norm, NormStoreError> {
        Self::check_policy(policy)?;
        let word = trigger.trim().to_ascii_lowercase();
        if word.is_empty() {
            return Err(NormStoreError::InvalidTrigger {
                reason: "empty trigger word".into(),
            });
        }
        if !word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            return Err(NormStoreError::InvalidTrigger {
                reason: format!("'{word}' must be a single [a-z0-9_-] word"),
            });
        }
        let norm = self
            .get(&norm_id)
            .cloned()
            .ok_or(NormStoreError::UnknownNorm { norm_id })?;
        let promoted = norm.with_trigger(&word);
        self.append_norm(&promoted, policy)?;
        Ok(promoted)
    }
}

fn read_jsonl<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Vec<T>, NormStoreError> {
    let file = File::open(path).map_err(|e| NormStoreError::Io {
        message: format!("open {}: {e}", path.display()),
    })?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    for (lineno, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| NormStoreError::Io {
            message: format!("read {} line {}: {e}", path.display(), lineno + 1),
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let record: T = serde_json::from_str(&line).map_err(|e| NormStoreError::Io {
            message: format!("parse {} line {}: {e}", path.display(), lineno + 1),
        })?;
        records.push(record);
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{WishFacet, WishFacetKind};
    use kosmo_hyphae::{NormFacetTemplate, NormLevel, NormOrigin};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("kosmo-norm-store-{tag}-{nanos}"))
    }

    fn norm(name: &str) -> Norm {
        Norm::new(
            name,
            "a test norm",
            NormLevel::Molecule,
            "module+symbol",
            [
                NormFacetTemplate::new(WishFacetKind::Module, "{name}"),
                NormFacetTemplate::new(WishFacetKind::Symbol, "{name}_load"),
            ],
            [],
            [],
            d(b"cand"),
            d(b"ev"),
            d(b"pol"),
            NormOrigin::OperatorInjected {
                source: "test".into(),
            },
        )
    }

    fn observation(ws: &[u8], subject: &str) -> FacetBundleObservation {
        FacetBundleObservation::new(
            d(ws),
            [
                WishFacet::module(subject),
                WishFacet::symbol(format!("{subject}_load")),
            ],
            ["rust".to_string()],
            true,
            d(subject.as_bytes()),
            d(b"pol"),
        )
    }

    fn op() -> PolicyProfile {
        PolicyProfile::operator_approved()
    }

    #[test]
    fn the_catalog_starts_empty() {
        // The structural rejection of Wonderlamp's 31-norm catalog: a fresh
        // store contains NOTHING. Norms enter only by observation or
        // operator injection.
        let dir = temp_dir("empty");
        let store = NormStore::open(&dir).unwrap();
        assert!(store.norms().is_empty());
        assert!(store.observations().is_empty());
        assert!(!dir.exists(), "open must not create anything on disk");
    }

    #[test]
    fn report_only_and_dry_run_cannot_append() {
        let dir = temp_dir("policy");
        let mut store = NormStore::open(&dir).unwrap();
        let n = norm("a");
        assert!(matches!(
            store.append_norm(&n, &PolicyProfile::default()),
            Err(NormStoreError::PolicyDenied { .. })
        ));
        match store.append_norm(&n, &PolicyProfile::dry_run()) {
            Err(NormStoreError::PolicyDenied { reason }) => {
                assert!(reason.contains("allow_host_write"))
            }
            other => panic!("expected PolicyDenied, got {other:?}"),
        }
        assert!(!dir.exists());
    }

    #[test]
    fn appends_persist_and_reload() {
        let dir = temp_dir("persist");
        let (a, b) = (norm("a"), norm("b"));
        let obs = observation(b"ws1", "alpha");
        {
            let mut store = NormStore::open(&dir).unwrap();
            store.append_norm(&a, &op()).unwrap();
            store.append_norm(&b, &op()).unwrap();
            store.append_norm(&a, &op()).unwrap(); // identical — no-op
            store.append_observation(&obs, &op()).unwrap();
            store.append_observation(&obs, &op()).unwrap(); // dedup
            assert_eq!(store.norms().len(), 2);
            assert_eq!(store.observations().len(), 1);
        }
        let reopened = NormStore::open(&dir).unwrap();
        assert_eq!(reopened.norms().len(), 2);
        assert_eq!(reopened.norms()[0].norm_id, a.norm_id, "first-seen order");
        assert_eq!(reopened.observations().len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn promotion_is_an_audit_trail_and_the_loader_takes_the_latest() {
        let dir = temp_dir("promote");
        let n = norm("loader");
        {
            let mut store = NormStore::open(&dir).unwrap();
            store.append_norm(&n, &op()).unwrap();
            let promoted = store.promote(n.norm_id, "Loader", &op()).unwrap();
            assert_eq!(promoted.trigger.as_deref(), Some("loader"));
            assert_eq!(promoted.norm_id, n.norm_id);
        }
        // Two lines on disk (creation + promotion) …
        let lines = std::fs::read_to_string(dir.join("norms.jsonl")).unwrap();
        assert_eq!(
            lines.lines().count(),
            2,
            "promotion appends, never rewrites"
        );
        // … but one norm in the catalog, armed.
        let reopened = NormStore::open(&dir).unwrap();
        assert_eq!(reopened.norms().len(), 1);
        assert_eq!(reopened.norms()[0].trigger.as_deref(), Some("loader"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn promote_rejects_unknown_norm_and_bad_words() {
        let dir = temp_dir("promote-bad");
        let mut store = NormStore::open(&dir).unwrap();
        assert!(matches!(
            store.promote(d(b"ghost"), "word", &op()),
            Err(NormStoreError::UnknownNorm { .. })
        ));
        let n = norm("x");
        store.append_norm(&n, &op()).unwrap();
        assert!(matches!(
            store.promote(n.norm_id, "  ", &op()),
            Err(NormStoreError::InvalidTrigger { .. })
        ));
        assert!(matches!(
            store.promote(n.norm_id, "two words", &op()),
            Err(NormStoreError::InvalidTrigger { .. })
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corruption_is_a_hard_error_on_open() {
        let dir = temp_dir("tamper");
        let n = norm("honest");
        {
            let mut store = NormStore::open(&dir).unwrap();
            store.append_norm(&n, &op()).unwrap();
        }
        // Forge a line whose id does not match its content.
        let mut forged = norm("forged");
        forged.norm_id = d(b"wrong");
        let line = serde_json::to_string(&forged).unwrap();
        {
            use std::io::Write as _;
            let mut f = OpenOptions::new()
                .append(true)
                .open(dir.join("norms.jsonl"))
                .unwrap();
            writeln!(f, "{line}").unwrap();
        }
        assert!(matches!(
            NormStore::open(&dir),
            Err(NormStoreError::IntegrityViolation { .. })
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_diseased_norm_cannot_enter_the_store() {
        // validate() runs at the append door too: a path-like template is
        // refused even by a well-formed, correctly-hashed norm.
        let dir = temp_dir("disease");
        let diseased = Norm::new(
            "smuggler",
            "",
            NormLevel::Atom,
            "module",
            [NormFacetTemplate::new(WishFacetKind::Module, "src/{name}")],
            [],
            [],
            d(b"c"),
            d(b"e"),
            d(b"p"),
            NormOrigin::OperatorInjected {
                source: "test".into(),
            },
        );
        let mut store = NormStore::open(&dir).unwrap();
        assert!(matches!(
            store.append_norm(&diseased, &op()),
            Err(NormStoreError::IntegrityViolation { .. })
        ));
        assert!(!dir.exists());
    }
}
