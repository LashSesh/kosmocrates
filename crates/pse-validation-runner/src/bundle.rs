//! Bundle creation and verification (§5.3 bundle / verify-bundle).
//!
//! A bundle is a manifest file (`bundle_manifest.json`) that records the
//! content-hashes of all files in the output directory. Verification
//! recomputes and compares those hashes.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::primitives::{content_address, Hash256, ValidationError};

/// A single file entry in the bundle manifest.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BundleFileEntry {
    /// Relative path from bundle root.
    pub relative_path: String,
    /// SHA-256 of file contents.
    pub content_hash: Hash256,
    pub size_bytes: u64,
}

/// The bundle manifest (content-addressed).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleManifest {
    pub manifest_id: Hash256,
    pub run_id: String,
    pub report_id: Hash256,
    /// Sorted by relative_path for determinism.
    pub files: Vec<BundleFileEntry>,
    pub total_files: u64,
}

impl BundleManifest {
    pub fn content_hash(&self) -> Result<Hash256, ValidationError> {
        content_address(self)
    }
}

/// Create a bundle manifest by hashing all files under `out_dir`.
pub fn create_bundle(
    out_dir: &Path,
    run_id: &str,
    report_id: &Hash256,
) -> Result<BundleManifest, ValidationError> {
    let mut files = collect_files(out_dir)?;
    // Sort for determinism.
    files.sort();

    let total_files = files.len() as u64;
    let partial = BundleManifest {
        manifest_id: Hash256::zero(),
        run_id: run_id.to_string(),
        report_id: report_id.clone(),
        files,
        total_files,
    };
    let manifest_id = content_address(&partial)?;
    Ok(BundleManifest {
        manifest_id,
        ..partial
    })
}

/// Verify a bundle: recompute all file hashes and compare to manifest.
pub fn verify_bundle(out_dir: &Path, manifest: &BundleManifest) -> Result<(), ValidationError> {
    let mut current_files = collect_files(out_dir)?;
    current_files.sort();

    let current_map: BTreeMap<&str, &BundleFileEntry> = current_files
        .iter()
        .map(|e| (e.relative_path.as_str(), e))
        .collect();
    let expected_map: BTreeMap<&str, &BundleFileEntry> = manifest
        .files
        .iter()
        .map(|e| (e.relative_path.as_str(), e))
        .collect();

    for (path, expected) in &expected_map {
        match current_map.get(path) {
            None => {
                return Err(ValidationError::BundleVerificationFailed {
                    reason: format!("file missing from bundle: {path}"),
                })
            }
            Some(current) => {
                if current.content_hash != expected.content_hash {
                    return Err(ValidationError::BundleVerificationFailed {
                        reason: format!(
                            "hash mismatch for {path}: expected {} got {}",
                            expected.content_hash.hex(),
                            current.content_hash.hex()
                        ),
                    });
                }
            }
        }
    }

    // Check for unexpected files (files present but not in manifest).
    for path in current_map.keys() {
        if !expected_map.contains_key(path) {
            return Err(ValidationError::BundleVerificationFailed {
                reason: format!("unexpected file not in manifest: {path}"),
            });
        }
    }

    Ok(())
}

fn collect_files(root: &Path) -> Result<Vec<BundleFileEntry>, ValidationError> {
    let mut entries = Vec::new();
    collect_recursive(root, root, &mut entries)?;
    Ok(entries)
}

fn collect_recursive(
    root: &Path,
    dir: &Path,
    entries: &mut Vec<BundleFileEntry>,
) -> Result<(), ValidationError> {
    let read_dir = std::fs::read_dir(dir).map_err(|e| ValidationError::Io {
        reason: format!("read_dir {}: {e}", dir.display()),
    })?;

    for entry in read_dir {
        let entry = entry.map_err(|e| ValidationError::Io {
            reason: e.to_string(),
        })?;
        let path = entry.path();
        let ft = entry.file_type().map_err(|e| ValidationError::Io {
            reason: e.to_string(),
        })?;

        if ft.is_dir() {
            collect_recursive(root, &path, entries)?;
        } else if ft.is_file() {
            let rel = path.strip_prefix(root).map_err(|e| ValidationError::Io {
                reason: format!("strip_prefix: {e}"),
            })?;
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            // Skip the manifest file itself.
            if rel_str == "bundle_manifest.json" {
                continue;
            }
            let bytes = std::fs::read(&path).map_err(|e| ValidationError::Io {
                reason: format!("read {}: {e}", path.display()),
            })?;
            let size_bytes = bytes.len() as u64;
            let content_hash = sha256_hash(&bytes);
            entries.push(BundleFileEntry {
                relative_path: rel_str,
                content_hash,
                size_bytes,
            });
        }
    }
    Ok(())
}

fn sha256_hash(data: &[u8]) -> Hash256 {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    Hash256::from_hex(&hex::encode(digest)).unwrap_or(Hash256::zero())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn bundle_verify_detects_tampering() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("final_report.json");
        std::fs::write(&file, b"{}").unwrap();

        let manifest = create_bundle(dir.path(), "run_001", &Hash256::zero()).unwrap();
        assert!(verify_bundle(dir.path(), &manifest).is_ok());

        // Tamper with file.
        let mut f = std::fs::OpenOptions::new().write(true).open(&file).unwrap();
        f.write_all(b"{\"tampered\": true}").unwrap();
        drop(f);

        assert!(verify_bundle(dir.path(), &manifest).is_err());
    }

    #[test]
    fn bundle_manifest_is_content_addressed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.json"), b"{}").unwrap();

        let m1 = create_bundle(dir.path(), "r1", &Hash256::zero()).unwrap();
        let m2 = create_bundle(dir.path(), "r1", &Hash256::zero()).unwrap();
        assert_eq!(m1.manifest_id, m2.manifest_id);
    }
}
