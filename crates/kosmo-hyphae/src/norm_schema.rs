//! Norm schema — a norm is a **learned archetype** that can emit only
//! [`WishFacet`]s.
//!
//! Assimilated from Wonderlamp's `isls-norms` type system with the CRUD
//! disease excised. The predecessor's `Norm` carried `NormLayers` with fixed
//! `database/model/query/service/api/frontend` slots and shipped a built-in
//! 31-norm catalog of CRUD molecules — a template forge in disguise. Here a
//! norm is a named, content-addressed bundle of [`NormFacetTemplate`]s: each
//! template names a [`WishFacetKind`] plus a key pattern over the single
//! placeholder `{name}`. Instantiation can only ever produce wish facets —
//! never file trees, never stacks, never entity scaffolds — so a norm's whole
//! power is *"which measurable targets does one word fan out into"*, exactly
//! the archetype seam `kosmo-intent` already owns.
//!
//! Governance invariants:
//! - **The catalog starts empty.** Norms exist only by observation
//!   ([`crate::norm_learning`]) or explicit operator injection
//!   ([`NormInjectionSpec`]).
//! - **`trigger` is `None` until an operator promotes the norm.** A learned
//!   norm cannot arm itself into the prose compiler. The trigger is therefore
//!   *governance metadata*, excluded from the content hash: promotion changes
//!   what a norm may do, not what it is.
//! - **`validate()` is the anti-disease gate**: path-like key patterns,
//!   foreign placeholders and non-templatable facet kinds are rejected
//!   (`name/arity` of [`WishFacetKind::Signature`] is the one sanctioned `/`).
//! - CROSS-006: `evidence_bundle_id ≠ ZERO` for every durable norm.

use kosmo_core::{Digest, WishFacet, WishFacetKind};
use serde::{Deserialize, Serialize};
use std::fmt;

/// The single placeholder a [`NormFacetTemplate`] key pattern may carry.
pub const NAME_PLACEHOLDER: &str = "{name}";

/// Composability level of a norm, from Wonderlamp's taxonomy (the taxonomy was
/// healthy; only its built-in inhabitants were the disease).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormLevel {
    /// A single facet pattern.
    Atom,
    /// A small bundle of facet patterns (2–4).
    Molecule,
    /// A broad bundle (5+).
    Organism,
    /// A composition of organisms (reserved for composed norms; learning
    /// never emits this level directly).
    Ecosystem,
}

impl NormLevel {
    /// Level implied by a template-bundle breadth (the learning default).
    pub fn from_breadth(template_count: usize) -> Self {
        match template_count {
            0 | 1 => Self::Atom,
            2..=4 => Self::Molecule,
            _ => Self::Organism,
        }
    }
}

/// Where a norm came from — provenance is part of its identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormOrigin {
    /// Distilled from realized facet-bundle observations; the ids bind the
    /// exact observations that justified the promotion to candidate.
    Learned { observation_ids: Vec<Digest> },
    /// Explicitly injected by an operator (`source` is a short human label,
    /// e.g. the spec file name — never a host path).
    OperatorInjected { source: String },
}

/// Why a norm (or one of its templates) failed validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormValidationError {
    IdMismatch,
    EvidenceUnbound,
    EmptyTemplate,
    /// This facet kind may not appear in a template (e.g. `Resolution` is
    /// pipeline-emitted; `Run`/`Service` probes are operator-authored).
    KindNotTemplatable {
        kind: WishFacetKind,
    },
    /// The key pattern smells like a file path or file name — the CRUD
    /// relapse this schema exists to prevent.
    PathLikePattern {
        pattern: String,
        why: String,
    },
    /// A `{`/`}` construct other than the literal `{name}` (Wonderlamp's
    /// parameterized templates are deliberately not supported).
    UnknownPlaceholder {
        pattern: String,
    },
    /// A structural kind's pattern carries no `{name}`: it would emit the
    /// same frozen artifact on every instantiation — a scaffold, not a norm.
    MissingPlaceholder {
        pattern: String,
    },
    /// A `Signature` pattern must be exactly `<stem>/<digits>`.
    BadSignaturePattern {
        pattern: String,
    },
    /// A norm may not require itself.
    SelfRequirement,
    /// Injection-only: the spec arrived with a trigger already set. Arming a
    /// trigger is a separate governance step (`--promote-norm`).
    TriggerPreset,
}

impl fmt::Display for NormValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdMismatch => write!(f, "norm_id does not match content"),
            Self::EvidenceUnbound => write!(f, "evidence_bundle_id is ZERO (CROSS-006)"),
            Self::EmptyTemplate => write!(f, "norm has no facet templates"),
            Self::KindNotTemplatable { kind } => {
                write!(f, "facet kind {kind:?} may not appear in a norm template")
            }
            Self::PathLikePattern { pattern, why } => {
                write!(f, "key pattern '{pattern}' is path-like ({why})")
            }
            Self::UnknownPlaceholder { pattern } => {
                write!(
                    f,
                    "key pattern '{pattern}' uses a placeholder other than {{name}}"
                )
            }
            Self::MissingPlaceholder { pattern } => {
                write!(
                    f,
                    "structural key pattern '{pattern}' carries no {{name}} placeholder"
                )
            }
            Self::BadSignaturePattern { pattern } => {
                write!(
                    f,
                    "signature pattern '{pattern}' is not of the form <stem>/<digits>"
                )
            }
            Self::SelfRequirement => write!(f, "norm requires itself"),
            Self::TriggerPreset => {
                write!(
                    f,
                    "injected norm carries a trigger; arm it via --promote-norm instead"
                )
            }
        }
    }
}

impl std::error::Error for NormValidationError {}

/// File extensions whose appearance in a key pattern marks it as a file name —
/// a template trying to smuggle a file tree through the facet door.
const CODE_EXTENSIONS: &[&str] = &[
    "rs", "py", "pyi", "js", "mjs", "cjs", "jsx", "ts", "tsx", "go", "java", "rb", "c", "h", "cpp",
    "hpp", "cs", "php", "html", "css", "sh", "toml", "yaml", "yml", "json", "sql",
];

/// One facet pattern of a norm: a [`WishFacetKind`] plus a key pattern over
/// the `{name}` placeholder. `instantiate("user")` on
/// `(Contract, "create_{name}(String)->String")` yields the contract facet
/// `create_user(String)->String` — measurable, scaffoldable, never a file.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NormFacetTemplate {
    pub kind: WishFacetKind,
    pub key_pattern: String,
}

impl NormFacetTemplate {
    pub fn new(kind: WishFacetKind, key_pattern: impl Into<String>) -> Self {
        Self {
            kind,
            key_pattern: key_pattern.into().trim().to_string(),
        }
    }

    /// Substitute every `{name}` and return the concrete facet.
    pub fn instantiate(&self, name: &str) -> WishFacet {
        WishFacet::new(
            self.kind.clone(),
            self.key_pattern.replace(NAME_PLACEHOLDER, name),
        )
    }

    /// The anti-disease gate, per template. See [`NormValidationError`].
    pub fn validate(&self) -> Result<(), NormValidationError> {
        let p = &self.key_pattern;
        let templatable = matches!(
            self.kind,
            WishFacetKind::Crate
                | WishFacetKind::Module
                | WishFacetKind::Symbol
                | WishFacetKind::Capability
                | WishFacetKind::Signature
                | WishFacetKind::Contract
                | WishFacetKind::Test
                | WishFacetKind::Doc
                | WishFacetKind::Behavior
                | WishFacetKind::Composition
        );
        if !templatable {
            return Err(NormValidationError::KindNotTemplatable {
                kind: self.kind.clone(),
            });
        }
        if p.is_empty() {
            return Err(NormValidationError::MissingPlaceholder { pattern: p.clone() });
        }
        // Only the literal "{name}" may use braces.
        if p.replace(NAME_PLACEHOLDER, "").contains(['{', '}']) {
            return Err(NormValidationError::UnknownPlaceholder { pattern: p.clone() });
        }
        // A structural pattern without the placeholder is a frozen scaffold.
        // Capability tags may be fixed (e.g. "audit-log").
        if self.kind != WishFacetKind::Capability && !p.contains(NAME_PLACEHOLDER) {
            return Err(NormValidationError::MissingPlaceholder { pattern: p.clone() });
        }
        if p.contains('\\') {
            return Err(NormValidationError::PathLikePattern {
                pattern: p.clone(),
                why: "backslash".into(),
            });
        }
        // '/' is a path separator everywhere except the one sanctioned form:
        // a Signature key "name/arity".
        if self.kind == WishFacetKind::Signature {
            match p.split_once('/') {
                Some((stem, arity))
                    if !stem.is_empty()
                        && !arity.is_empty()
                        && !arity.contains('/')
                        && arity.chars().all(|c| c.is_ascii_digit()) => {}
                _ => {
                    return Err(NormValidationError::BadSignaturePattern { pattern: p.clone() });
                }
            }
        } else if p.contains('/') {
            return Err(NormValidationError::PathLikePattern {
                pattern: p.clone(),
                why: "path separator".into(),
            });
        }
        // ".rs", ".py", … anywhere at a token boundary names a file.
        for ext in CODE_EXTENSIONS {
            let needle = format!(".{ext}");
            for (i, _) in p.match_indices(&needle) {
                let after = i + needle.len();
                let boundary = p[after..]
                    .chars()
                    .next()
                    .map(|c| !c.is_alphanumeric())
                    .unwrap_or(true);
                if boundary {
                    return Err(NormValidationError::PathLikePattern {
                        pattern: p.clone(),
                        why: format!("file extension .{ext}"),
                    });
                }
            }
        }
        Ok(())
    }
}

/// Serialize-only content for the norm id. **`trigger` is deliberately
/// absent**: arming a trigger is governance over an existing artifact, not a
/// new artifact — promotion re-appends the same `norm_id` with the trigger
/// set, and stores resolve to the latest occurrence.
#[derive(Serialize)]
struct NormContent<'a> {
    name: &'a str,
    description: &'a str,
    level: &'a NormLevel,
    semantic_class: &'a str,
    template: &'a Vec<NormFacetTemplate>,
    requires: &'a Vec<Digest>,
    variants: &'a Vec<String>,
    linked_candidate_id: &'a Digest,
    evidence_bundle_id: &'a Digest,
    policy_id: &'a Digest,
    origin: &'a NormOrigin,
}

/// A content-addressed, governable, learned archetype.
///
/// `linked_candidate_id` ties the norm to its [`crate::norm::NormGeneCandidate`],
/// so the existing promotion-feedback fitness loop
/// ([`crate::norm::NormFitnessTrace`]) applies to norms unchanged.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Norm {
    pub norm_id: Digest,
    pub name: String,
    pub description: String,
    pub level: NormLevel,
    /// Free-form classification derived from the facet kinds (descriptive,
    /// never behavioural — no code path branches on it).
    pub semantic_class: String,
    /// Prose-compiler activation word. **Always `None` at creation**; set only
    /// through [`Norm::with_trigger`] on the operator's explicit promotion.
    /// Excluded from `norm_id` (governance metadata, not identity).
    pub trigger: Option<String>,
    pub template: Vec<NormFacetTemplate>,
    /// Norms this one composes over (by `norm_id`).
    pub requires: Vec<Digest>,
    /// Free-form variant labels (e.g. alternative realizations).
    pub variants: Vec<String>,
    pub linked_candidate_id: Digest,
    pub evidence_bundle_id: Digest,
    pub policy_id: Digest,
    pub origin: NormOrigin,
}

impl Norm {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        level: NormLevel,
        semantic_class: impl Into<String>,
        template: impl IntoIterator<Item = NormFacetTemplate>,
        requires: impl IntoIterator<Item = Digest>,
        variants: impl IntoIterator<Item = String>,
        linked_candidate_id: Digest,
        evidence_bundle_id: Digest,
        policy_id: Digest,
        origin: NormOrigin,
    ) -> Self {
        let mut template: Vec<NormFacetTemplate> = template.into_iter().collect();
        template.sort();
        template.dedup();
        let mut requires: Vec<Digest> = requires.into_iter().collect();
        requires.sort();
        requires.dedup();
        let mut n = Self {
            norm_id: Digest::ZERO,
            name: name.into(),
            description: description.into(),
            level,
            semantic_class: semantic_class.into(),
            trigger: None,
            template,
            requires,
            variants: variants.into_iter().collect(),
            linked_candidate_id,
            evidence_bundle_id,
            policy_id,
            origin,
        };
        n.norm_id = n.compute_id();
        n
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&NormContent {
            name: &self.name,
            description: &self.description,
            level: &self.level,
            semantic_class: &self.semantic_class,
            template: &self.template,
            requires: &self.requires,
            variants: &self.variants,
            linked_candidate_id: &self.linked_candidate_id,
            evidence_bundle_id: &self.evidence_bundle_id,
            policy_id: &self.policy_id,
            origin: &self.origin,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.norm_id == self.compute_id()
    }

    /// CROSS-006: a durable norm must be evidence-bound.
    pub fn is_evidence_bound(&self) -> bool {
        self.evidence_bundle_id != Digest::ZERO
    }

    /// The operator's promotion step: arm the prose-compiler trigger. The word
    /// is trimmed and lowercased; the `norm_id` is unchanged (the trigger is
    /// outside the content hash).
    pub fn with_trigger(mut self, word: impl AsRef<str>) -> Self {
        self.trigger = Some(word.as_ref().trim().to_ascii_lowercase());
        self
    }

    /// Validate identity, evidence binding, and every template (the
    /// anti-disease gate). Stores run this on every load and append.
    pub fn validate(&self) -> Result<(), NormValidationError> {
        if !self.verify_id() {
            return Err(NormValidationError::IdMismatch);
        }
        if !self.is_evidence_bound() {
            return Err(NormValidationError::EvidenceUnbound);
        }
        if self.template.is_empty() {
            return Err(NormValidationError::EmptyTemplate);
        }
        if self.requires.contains(&self.norm_id) {
            return Err(NormValidationError::SelfRequirement);
        }
        for t in &self.template {
            t.validate()?;
        }
        Ok(())
    }

    /// Expand the norm for a concrete `name`: every template instantiated, in
    /// the (sorted) template order. Fail-closed: a name that cleans to nothing
    /// expands to nothing. Duplicate facets (possible when patterns differ
    /// only around the placeholder) are dropped.
    pub fn instantiate(&self, name: &str) -> Vec<WishFacet> {
        let clean: String = name
            .trim()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if clean.is_empty() || !clean.chars().any(|c| c.is_alphanumeric()) {
            return vec![];
        }
        let mut facets: Vec<WishFacet> = Vec::with_capacity(self.template.len());
        for t in &self.template {
            let f = t.instantiate(&clean);
            if !facets.contains(&f) {
                facets.push(f);
            }
        }
        facets
    }
}

/// The operator-injection input: a norm spec **without** id, evidence, policy
/// or origin — those are derived at the injection seam (the spec file's bytes
/// are the evidence; the active policy binds it; the file name is the origin
/// label). Deliberately has no `trigger` field: an injected norm always
/// arrives unarmed.
#[derive(Clone, Debug, Deserialize)]
pub struct NormInjectionSpec {
    pub name: String,
    pub description: String,
    pub level: NormLevel,
    pub semantic_class: String,
    pub template: Vec<NormFacetTemplate>,
    #[serde(default)]
    pub requires: Vec<Digest>,
    #[serde(default)]
    pub variants: Vec<String>,
}

impl NormInjectionSpec {
    /// Build the durable, validated [`Norm`] (plus its linked
    /// [`crate::norm::NormGeneCandidate`] id, computed internally at fitness
    /// zero — injected norms earn fitness through the feedback loop like
    /// everything else).
    pub fn into_norm(
        self,
        evidence_bundle_id: Digest,
        policy_id: Digest,
        source: impl Into<String>,
    ) -> Result<Norm, NormValidationError> {
        let candidate = crate::norm::NormGeneCandidate::new(
            self.name.clone(),
            self.description.clone(),
            kosmo_core::Q16::ZERO,
            evidence_bundle_id,
            policy_id,
        );
        let norm = Norm::new(
            self.name,
            self.description,
            self.level,
            self.semantic_class,
            self.template,
            self.requires,
            self.variants,
            candidate.candidate_id,
            evidence_bundle_id,
            policy_id,
            NormOrigin::OperatorInjected {
                source: source.into(),
            },
        );
        norm.validate()?;
        Ok(norm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn molecule() -> Norm {
        Norm::new(
            "loader",
            "module plus loader fn",
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
            NormOrigin::Learned {
                observation_ids: vec![d(b"o1")],
            },
        )
    }

    #[test]
    fn norm_is_content_addressed_and_trigger_is_outside_the_hash() {
        let a = molecule();
        let b = molecule();
        assert_eq!(a.norm_id, b.norm_id);
        assert_ne!(a.norm_id, Digest::ZERO);
        assert!(a.verify_id());

        // CRUD-relapse guard #2: arming a trigger is governance, not identity.
        let armed = a.clone().with_trigger("Loader ");
        assert_eq!(armed.trigger.as_deref(), Some("loader"));
        assert_eq!(armed.norm_id, a.norm_id, "trigger must not change norm_id");
        assert!(armed.verify_id(), "armed norm still verifies");
        assert!(armed.validate().is_ok());
    }

    #[test]
    fn validate_enforces_identity_and_evidence() {
        let mut tampered = molecule();
        tampered.description = "edited".into();
        assert_eq!(tampered.validate(), Err(NormValidationError::IdMismatch));

        let unbound = Norm::new(
            "x",
            "",
            NormLevel::Atom,
            "module",
            [NormFacetTemplate::new(WishFacetKind::Module, "{name}")],
            [],
            [],
            d(b"c"),
            Digest::ZERO,
            d(b"p"),
            NormOrigin::OperatorInjected { source: "s".into() },
        );
        assert_eq!(
            unbound.validate(),
            Err(NormValidationError::EvidenceUnbound)
        );
    }

    #[test]
    fn validate_rejects_path_like_templates() {
        let cases = [
            (WishFacetKind::Module, "src/{name}", "path separator"),
            (WishFacetKind::Module, "backend\\{name}", "backslash"),
            (WishFacetKind::Symbol, "{name}.rs", "file extension"),
            (WishFacetKind::Test, "{name}.py", "file extension"),
        ];
        for (kind, pattern, why) in cases {
            let err = NormFacetTemplate::new(kind, pattern)
                .validate()
                .unwrap_err();
            match err {
                NormValidationError::PathLikePattern { why: w, .. } => {
                    assert!(w.contains(why), "{pattern}: {w}")
                }
                other => panic!("{pattern}: expected PathLike, got {other:?}"),
            }
        }
    }

    #[test]
    fn signature_arity_is_the_one_sanctioned_slash() {
        assert!(NormFacetTemplate::new(WishFacetKind::Signature, "{name}/2")
            .validate()
            .is_ok());
        assert!(matches!(
            NormFacetTemplate::new(WishFacetKind::Signature, "src/{name}").validate(),
            Err(NormValidationError::BadSignaturePattern { .. })
        ));
        assert!(matches!(
            NormFacetTemplate::new(WishFacetKind::Signature, "{name}/a/2").validate(),
            Err(NormValidationError::BadSignaturePattern { .. })
        ));
    }

    #[test]
    fn placeholder_discipline_is_enforced() {
        // Foreign placeholders (Wonderlamp's parameterized templates) rejected.
        assert!(matches!(
            NormFacetTemplate::new(WishFacetKind::Module, "{entity}").validate(),
            Err(NormValidationError::UnknownPlaceholder { .. })
        ));
        // A frozen structural name is a scaffold, not a norm.
        assert!(matches!(
            NormFacetTemplate::new(WishFacetKind::Module, "models").validate(),
            Err(NormValidationError::MissingPlaceholder { .. })
        ));
        // Capability tags may be fixed.
        assert!(
            NormFacetTemplate::new(WishFacetKind::Capability, "audit-log")
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn pipeline_and_probe_kinds_are_not_templatable() {
        for kind in [
            WishFacetKind::Resolution,
            WishFacetKind::Dependency,
            WishFacetKind::Run,
            WishFacetKind::Service,
        ] {
            assert!(matches!(
                NormFacetTemplate::new(kind, "{name}").validate(),
                Err(NormValidationError::KindNotTemplatable { .. })
            ));
        }
    }

    #[test]
    fn instantiate_expands_every_template_and_fails_closed_on_empty_name() {
        let n = molecule();
        let facets = n.instantiate("user");
        assert_eq!(
            facets,
            vec![WishFacet::module("user"), WishFacet::symbol("user_load")]
        );
        assert!(n.instantiate("  ").is_empty());
        assert!(n.instantiate("///").is_empty());
    }

    #[test]
    fn injection_spec_round_trips_and_validates() {
        let json = r#"{
            "name": "web-organ",
            "description": "a module with an organ capability",
            "level": "Molecule",
            "semantic_class": "structural",
            "template": [
                {"kind": "Module", "key_pattern": "{name}"},
                {"kind": "Capability", "key_pattern": "organ:{name}"}
            ]
        }"#;
        let spec: NormInjectionSpec = serde_json::from_str(json).unwrap();
        let norm = spec
            .into_norm(d(b"spec-bytes"), d(b"pol"), "spec.json")
            .unwrap();
        assert!(norm.validate().is_ok());
        assert_eq!(norm.trigger, None, "injected norms always arrive unarmed");
        assert_eq!(norm.instantiate("heart").len(), 2);
        assert!(matches!(norm.origin, NormOrigin::OperatorInjected { .. }));
        assert_ne!(norm.linked_candidate_id, Digest::ZERO);
    }

    #[test]
    fn injection_spec_rejects_diseased_templates() {
        let json = r#"{
            "name": "forge",
            "description": "",
            "level": "Atom",
            "semantic_class": "x",
            "template": [{"kind": "Module", "key_pattern": "src/models/{name}"}]
        }"#;
        let spec: NormInjectionSpec = serde_json::from_str(json).unwrap();
        assert!(matches!(
            spec.into_norm(d(b"b"), d(b"p"), "f"),
            Err(NormValidationError::PathLikePattern { .. })
        ));
    }

    #[test]
    fn serde_round_trip_preserves_identity_and_trigger() {
        let armed = molecule().with_trigger("loader");
        let json = serde_json::to_string(&armed).unwrap();
        let back: Norm = serde_json::from_str(&json).unwrap();
        assert_eq!(back, armed);
        assert!(back.verify_id());
        assert_eq!(back.trigger.as_deref(), Some("loader"));
    }

    /// Cumulative disease test (plan guard): the norm organ's sources must not
    /// smuggle in Wonderlamp's stacks, layer paths or entity scaffolds.
    #[test]
    fn norm_sources_carry_no_crud_disease() {
        let sources = [
            include_str!("norm_schema.rs"),
            include_str!("norm_learning.rs"),
            include_str!("norm_genome.rs"),
        ];
        let forbidden = [
            "backend/",
            "frontend/",
            "src/models",
            "src/routes",
            "actix",
            "sqlx",
            "axum_",
            "handlebars",
            "warehouse",
            "ecommerce",
            "pagination",
            "jwt",
        ];
        for src in sources {
            let body = src.split("#[cfg(test)]").next().unwrap();
            for needle in forbidden {
                assert!(
                    !body.to_ascii_lowercase().contains(needle),
                    "forbidden fragment '{needle}' found in norm sources"
                );
            }
        }
    }
}
