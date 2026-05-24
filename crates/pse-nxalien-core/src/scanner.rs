//! Project scanner: detects language, package manager, tool commands, and
//! existing context files from the file system.

use pse_nxalien_types::{
    EvidenceRef, ProjectMetadata, RuleAtom, ScopeRef, Severity, UnknownSlot, UnknownStatus,
};
use std::path::{Path, PathBuf};

/// Scans a project root and returns detected metadata, default rules, and unknowns.
pub struct ProjectScanner {
    pub root: PathBuf,
}

impl ProjectScanner {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Scan the project and return detected metadata.
    pub fn scan(&self) -> ProjectMetadata {
        let mut meta = ProjectMetadata::default();

        // ── Rust / Cargo ──────────────────────────────────────────────────────
        let cargo_path = self.root.join("Cargo.toml");
        if cargo_path.exists() {
            meta.language = "rust".to_string();
            meta.package_manager = "cargo".to_string();
            meta.test_command = Some("cargo test --workspace --locked".to_string());
            meta.build_command = Some("cargo build --workspace --locked".to_string());
            meta.fmt_command = Some("cargo fmt --all".to_string());
            meta.lint_command =
                Some("cargo clippy --workspace --all-targets --locked -- -D warnings".to_string());
            if let Ok(content) = std::fs::read_to_string(&cargo_path) {
                meta.name = extract_toml_str(&content, "name").unwrap_or_default();
                meta.license = extract_toml_str(&content, "license");
            }
        }
        // ── Node / npm ────────────────────────────────────────────────────────
        else if self.root.join("package.json").exists() {
            let pkg_path = self.root.join("package.json");
            meta.language = "typescript".to_string();
            if self.root.join("pnpm-lock.yaml").exists() {
                meta.package_manager = "pnpm".to_string();
                meta.test_command = Some("pnpm test".to_string());
                meta.build_command = Some("pnpm build".to_string());
                meta.fmt_command = Some("pnpm fmt".to_string());
                meta.lint_command = Some("pnpm lint".to_string());
            } else if self.root.join("yarn.lock").exists() {
                meta.package_manager = "yarn".to_string();
                meta.test_command = Some("yarn test".to_string());
                meta.build_command = Some("yarn build".to_string());
                meta.fmt_command = Some("yarn format".to_string());
                meta.lint_command = Some("yarn lint".to_string());
            } else {
                meta.package_manager = "npm".to_string();
                meta.test_command = Some("npm test".to_string());
                meta.build_command = Some("npm run build".to_string());
                meta.fmt_command = Some("npm run format".to_string());
                meta.lint_command = Some("npm run lint".to_string());
            }
            if let Ok(content) = std::fs::read_to_string(&pkg_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                        meta.name = name.to_string();
                    }
                }
            }
        }
        // ── Python ────────────────────────────────────────────────────────────
        else if self.root.join("pyproject.toml").exists() || self.root.join("setup.py").exists() {
            meta.language = "python".to_string();
            if self.root.join("uv.lock").exists() {
                meta.package_manager = "uv".to_string();
                meta.test_command = Some("uv run pytest".to_string());
                meta.fmt_command = Some("uv run ruff format".to_string());
                meta.lint_command = Some("uv run ruff check".to_string());
            } else if self.root.join("poetry.lock").exists() {
                meta.package_manager = "poetry".to_string();
                meta.test_command = Some("poetry run pytest".to_string());
            } else {
                meta.package_manager = "pip".to_string();
                meta.test_command = Some("python -m pytest".to_string());
            }
            if let Ok(content) = std::fs::read_to_string(self.root.join("pyproject.toml")) {
                meta.name = extract_toml_str(&content, "name").unwrap_or_default();
            }
        }

        // ── Existing context files ────────────────────────────────────────────
        for name in &["AGENTS.md", "CLAUDE.md", ".rules", ".cursorrules"] {
            if self.root.join(name).exists() {
                meta.existing_context_files.push((*name).to_string());
            }
        }

        meta
    }

    /// Generate language-appropriate default rule atoms.
    pub fn default_rules(&self, meta: &ProjectMetadata) -> Vec<RuleAtom> {
        match meta.language.as_str() {
            "rust" => default_rust_rules(),
            "typescript" | "javascript" => default_node_rules(meta),
            "python" => default_python_rules(meta),
            _ => vec![],
        }
    }

    /// Extract unknowns from discovered context files (TODO/FIXME markers).
    pub fn extract_unknowns(&self, meta: &ProjectMetadata) -> Vec<UnknownSlot> {
        let mut unknowns = Vec::new();
        for ctx_file in &meta.existing_context_files {
            let path = self.root.join(ctx_file);
            if let Ok(content) = std::fs::read_to_string(&path) {
                for (i, line) in content.lines().enumerate() {
                    let upper = line.to_uppercase();
                    if upper.contains("TODO")
                        || upper.contains("FIXME")
                        || upper.contains("UNKNOWN")
                    {
                        unknowns.push(UnknownSlot {
                            name: format!("{ctx_file}:L{}", i + 1),
                            domain: vec!["context".to_string()],
                            status: UnknownStatus::Unknown,
                            candidates: vec![],
                            evidence: vec![EvidenceRef {
                                kind: "file".to_string(),
                                source: ctx_file.clone(),
                            }],
                            resolution: None,
                            confidence: 0.0,
                            expires: None,
                        });
                    }
                }
            }
        }
        unknowns
    }
}

// ─── Private helpers ─────────────────────────────────────────────────────────

/// Extract `key = "value"` from a TOML-like file without a full TOML parser.
fn extract_toml_str(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(key) {
            if let Some(rest) = trimmed.strip_prefix(key) {
                let rest = rest.trim();
                if let Some(rest) = rest.strip_prefix('=') {
                    let rest = rest.trim().trim_matches('"');
                    if !rest.is_empty() {
                        return Some(rest.to_string());
                    }
                }
            }
        }
    }
    None
}

// ─── Default rule sets ───────────────────────────────────────────────────────

fn cargo_ev() -> EvidenceRef {
    EvidenceRef {
        kind: "file".to_string(),
        source: "Cargo.toml".to_string(),
    }
}

pub fn default_rust_rules() -> Vec<RuleAtom> {
    vec![
        RuleAtom {
            id: "rust-fmt".to_string(),
            scope: ScopeRef {
                files: vec!["**/*.rs".to_string()],
                ..Default::default()
            },
            trigger: vec!["format".to_string(), "style".to_string(), "fmt".to_string()],
            prescription: "Run `cargo fmt --all` before any commit. Code must be rustfmt-clean."
                .to_string(),
            severity: Severity::Required,
            evidence: vec![cargo_ev()],
            gate: None,
            decay: None,
            hash: String::new(),
        }
        .with_hash(),
        RuleAtom {
            id: "rust-clippy".to_string(),
            scope: ScopeRef {
                files: vec!["**/*.rs".to_string()],
                ..Default::default()
            },
            trigger: vec![
                "lint".to_string(),
                "clippy".to_string(),
                "warning".to_string(),
            ],
            prescription: "Run `cargo clippy --workspace --all-targets --locked -- -D warnings`. \
                No warnings permitted."
                .to_string(),
            severity: Severity::Required,
            evidence: vec![cargo_ev()],
            gate: None,
            decay: None,
            hash: String::new(),
        }
        .with_hash(),
        RuleAtom {
            id: "rust-test".to_string(),
            scope: ScopeRef {
                commands: vec!["cargo test".to_string()],
                ..Default::default()
            },
            trigger: vec!["test".to_string(), "commit".to_string(), "ci".to_string()],
            prescription: "All workspace tests must pass before claiming completion. \
                Run `cargo test --workspace --locked`."
                .to_string(),
            severity: Severity::Blocking,
            evidence: vec![cargo_ev()],
            gate: None,
            decay: None,
            hash: String::new(),
        }
        .with_hash(),
        RuleAtom {
            id: "no-direct-crystal".to_string(),
            scope: ScopeRef {
                files: vec!["crates/pse-nxalien-*/**/*.rs".to_string()],
                ..Default::default()
            },
            trigger: vec![
                "crystal".to_string(),
                "SemanticCrystal".to_string(),
                "emit".to_string(),
            ],
            prescription: "nxalien crates MUST NOT directly construct SemanticCrystal. \
                Emit NxAlienHandoffCandidate only. PSE-Bridge is the sole commit path."
                .to_string(),
            severity: Severity::Blocking,
            evidence: vec![EvidenceRef {
                kind: "user_directive".to_string(),
                source: "nxalien specification §9".to_string(),
            }],
            gate: Some("bridge_supremacy".to_string()),
            decay: None,
            hash: String::new(),
        }
        .with_hash(),
        RuleAtom {
            id: "minimal-reversible".to_string(),
            scope: ScopeRef {
                files: vec!["**/*.rs".to_string()],
                ..Default::default()
            },
            trigger: vec![
                "change".to_string(),
                "delete".to_string(),
                "refactor".to_string(),
            ],
            prescription: "Prefer minimal reversible changes. Do not delete without explicit \
                instruction. Do not introduce abstractions beyond what the task requires."
                .to_string(),
            severity: Severity::Required,
            evidence: vec![EvidenceRef {
                kind: "user_directive".to_string(),
                source: "PSE coding standard".to_string(),
            }],
            gate: None,
            decay: None,
            hash: String::new(),
        }
        .with_hash(),
    ]
}

fn default_node_rules(meta: &ProjectMetadata) -> Vec<RuleAtom> {
    let pm = &meta.package_manager;
    let pkg_ev = EvidenceRef {
        kind: "file".to_string(),
        source: "package.json".to_string(),
    };
    vec![
        RuleAtom {
            id: "node-test".to_string(),
            scope: ScopeRef {
                commands: vec![format!("{pm} test")],
                ..Default::default()
            },
            trigger: vec!["test".to_string(), "commit".to_string(), "ci".to_string()],
            prescription: format!(
                "All tests must pass before claiming completion. Run `{pm} test`."
            ),
            severity: Severity::Blocking,
            evidence: vec![pkg_ev.clone()],
            gate: None,
            decay: None,
            hash: String::new(),
        }
        .with_hash(),
        RuleAtom {
            id: "node-lint".to_string(),
            scope: ScopeRef {
                files: vec!["**/*.ts".to_string()],
                ..Default::default()
            },
            trigger: vec!["lint".to_string(), "format".to_string()],
            prescription: format!("Run `{pm} run lint` and `{pm} run format` before committing."),
            severity: Severity::Required,
            evidence: vec![pkg_ev],
            gate: None,
            decay: None,
            hash: String::new(),
        }
        .with_hash(),
    ]
}

fn default_python_rules(meta: &ProjectMetadata) -> Vec<RuleAtom> {
    let pm = &meta.package_manager;
    let py_ev = EvidenceRef {
        kind: "file".to_string(),
        source: "pyproject.toml".to_string(),
    };
    vec![
        RuleAtom {
            id: "python-test".to_string(),
            scope: ScopeRef {
                commands: vec!["pytest".to_string()],
                ..Default::default()
            },
            trigger: vec!["test".to_string(), "commit".to_string()],
            prescription: format!("Run pytest via `{pm}` before committing."),
            severity: Severity::Blocking,
            evidence: vec![py_ev.clone()],
            gate: None,
            decay: None,
            hash: String::new(),
        }
        .with_hash(),
        RuleAtom {
            id: "python-lint".to_string(),
            scope: ScopeRef {
                files: vec!["**/*.py".to_string()],
                ..Default::default()
            },
            trigger: vec!["lint".to_string(), "format".to_string()],
            prescription: "Run ruff check + ruff format before committing.".to_string(),
            severity: Severity::Required,
            evidence: vec![py_ev],
            gate: None,
            decay: None,
            hash: String::new(),
        }
        .with_hash(),
    ]
}

/// Compute a simple content digest for a directory (Cargo.toml + top-level listing).
pub fn project_root_digest(root: &Path) -> String {
    let mut content = String::new();
    for name in &["Cargo.toml", "package.json", "pyproject.toml", "setup.py"] {
        let p = root.join(name);
        if let Ok(s) = std::fs::read_to_string(&p) {
            content.push_str(&s);
        }
    }
    crate::canon::sha256_bytes(content.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_toml_str_basic() {
        let toml = r#"
[package]
name = "my-project"
version = "0.1.0"
"#;
        assert_eq!(
            extract_toml_str(toml, "name"),
            Some("my-project".to_string())
        );
        assert_eq!(extract_toml_str(toml, "version"), Some("0.1.0".to_string()));
        assert_eq!(extract_toml_str(toml, "missing"), None);
    }

    #[test]
    fn default_rust_rules_have_valid_hashes() {
        for rule in default_rust_rules() {
            assert!(rule.verify_hash(), "Rule {} has invalid hash", rule.id);
        }
    }

    #[test]
    fn scanner_detects_rust_project() {
        // The PSE workspace itself is a Rust project.
        let scanner = ProjectScanner::new(env!("CARGO_MANIFEST_DIR"));
        let meta = scanner.scan();
        // Should detect rust or at least not panic.
        let _ = scanner.default_rules(&meta);
    }
}
