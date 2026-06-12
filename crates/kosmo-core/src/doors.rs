//! Doors — the self-describing docking surface.
//!
//! A *door* is one operator-facing entry point of the system: a CLI mode of
//! a binary, an HTTP route of a server. The catalog of doors is the machine
//! answer to the operator's standing question — *"what, exactly, could I
//! operate right now?"* — spoken by the system itself: content-addressed,
//! deterministic, and (at every surface that declares one) pinned by test
//! against the parser or router that implements it, so the description can
//! never drift from the mechanism.
//!
//! The catalog carries the governance truth alongside the mechanics: every
//! door declares whether it only observes, writes a workspace, appends to a
//! durable store, or is an operator decree — the docking surface and its
//! power, in one artifact. Surfaces describe **themselves** (a binary
//! catalogs its own doors, a server its own routes); catalogs from several
//! surfaces merge deterministically into one.

use serde::{Deserialize, Serialize};

use crate::digest::Digest;

/// Where a door opens.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DoorSurface {
    /// A CLI mode/flag of a named binary.
    Cli { binary: String },
    /// An HTTP route of a named server binary.
    Http { binary: String, method: String },
}

/// One input a door accepts (a companion flag or a request field).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorInput {
    /// The input's name as the operator types it (`"--fence"`, `"path"`).
    pub name: String,
    /// Value shape hint (`"<dir>"`, `"<n>"`); `None` for a bare switch.
    pub value: Option<String>,
    /// Whether the door cannot open without it.
    pub required: bool,
}

impl DoorInput {
    pub fn switch(name: impl Into<String>) -> Self {
        DoorInput {
            name: name.into(),
            value: None,
            required: false,
        }
    }
    pub fn valued(name: impl Into<String>, value: impl Into<String>) -> Self {
        DoorInput {
            name: name.into(),
            value: Some(value.into()),
            required: false,
        }
    }
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

/// The door's write power — governance as data, fail-closed vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoorGovernance {
    /// Observes and reports only; never mutates anything durable.
    ReadOnly,
    /// May mutate the target workspace (always behind an explicit flag).
    WritesWorkspace,
    /// Appends to a durable store (norms, ledger, session files).
    AppendsStore,
    /// An operator decree (e.g. arming a norm trigger) — the invocation
    /// itself is the approval.
    GovernanceAct,
}

impl DoorGovernance {
    pub fn label(&self) -> &'static str {
        match self {
            DoorGovernance::ReadOnly => "read-only",
            DoorGovernance::WritesWorkspace => "writes-workspace",
            DoorGovernance::AppendsStore => "appends-store",
            DoorGovernance::GovernanceAct => "governance-act",
        }
    }
}

/// What a door needs from the world to open fully.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoorNeed {
    /// A real LLM provider (key + network).
    Provider,
    /// A cargo toolchain on the host.
    Cargo,
    /// Outbound network access.
    Network,
    /// A target workspace path.
    Workspace,
    /// A caller-pathed durable store directory/file.
    Store,
    /// A spec/draft file authored by the operator.
    File,
}

/// One operator-facing entry point, content-addressed over its own
/// description (`door_id` = digest of surface, names, inputs, governance).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Door {
    pub door_id: Digest,
    pub surface: DoorSurface,
    /// The name the operator speaks (`"--steward"`, `"/api/landscape"`).
    pub name: String,
    /// Alternate spellings that open the same door (`"--testbench"`).
    pub aliases: Vec<String>,
    /// One operator-facing sentence.
    pub summary: String,
    pub inputs: Vec<DoorInput>,
    pub governance: DoorGovernance,
    pub needs: Vec<DoorNeed>,
}

#[derive(Serialize)]
struct DoorBody<'a> {
    surface: &'a DoorSurface,
    name: &'a str,
    aliases: &'a [String],
    summary: &'a str,
    inputs: &'a [DoorInput],
    governance: &'a DoorGovernance,
    needs: &'a [DoorNeed],
}

impl Door {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        surface: DoorSurface,
        name: impl Into<String>,
        aliases: Vec<String>,
        summary: impl Into<String>,
        inputs: Vec<DoorInput>,
        governance: DoorGovernance,
        needs: Vec<DoorNeed>,
    ) -> Self {
        let name = name.into();
        let summary = summary.into();
        let door_id = Digest::of(&DoorBody {
            surface: &surface,
            name: &name,
            aliases: &aliases,
            summary: &summary,
            inputs: &inputs,
            governance: &governance,
            needs: &needs,
        });
        Door {
            door_id,
            surface,
            name,
            aliases,
            summary,
            inputs,
            governance,
            needs,
        }
    }
}

/// A surface's complete door inventory, content-addressed over the doors it
/// carries. Deterministic order: doors sorted by (surface, name).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DoorCatalog {
    pub catalog_id: Digest,
    pub doors: Vec<Door>,
}

impl DoorCatalog {
    pub fn new(mut doors: Vec<Door>) -> Self {
        doors.sort_by(|a, b| (&a.surface, &a.name).cmp(&(&b.surface, &b.name)));
        doors.dedup_by(|a, b| a.door_id == b.door_id);
        let ids: Vec<Digest> = doors.iter().map(|d| d.door_id).collect();
        DoorCatalog {
            catalog_id: Digest::of(&ids),
            doors,
        }
    }

    /// Merge several surfaces' catalogs into one (deterministic, deduped).
    pub fn merge(catalogs: impl IntoIterator<Item = DoorCatalog>) -> Self {
        DoorCatalog::new(catalogs.into_iter().flat_map(|c| c.doors).collect())
    }

    pub fn len(&self) -> usize {
        self.doors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.doors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli(binary: &str) -> DoorSurface {
        DoorSurface::Cli {
            binary: binary.into(),
        }
    }

    fn door(name: &str, summary: &str) -> Door {
        Door::new(
            cli("demo"),
            name,
            vec![],
            summary,
            vec![DoorInput::switch("--json")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Workspace],
        )
    }

    #[test]
    fn door_identity_is_content_addressed() {
        let a = door("--wish", "measure a wish");
        let b = door("--wish", "measure a wish");
        assert_eq!(a.door_id, b.door_id, "same description, same identity");
        let c = door("--wish", "a DIFFERENT summary");
        assert_ne!(a.door_id, c.door_id, "the summary is part of the truth");
        assert_ne!(a.door_id, Digest::ZERO);
    }

    #[test]
    fn catalog_is_deterministic_and_deduped() {
        let unordered = DoorCatalog::new(vec![door("--b", "b"), door("--a", "a")]);
        let ordered = DoorCatalog::new(vec![door("--a", "a"), door("--b", "b")]);
        assert_eq!(unordered, ordered, "declaration order is irrelevant");
        assert_eq!(unordered.doors[0].name, "--a");

        let doubled = DoorCatalog::new(vec![door("--a", "a"), door("--a", "a")]);
        assert_eq!(doubled.len(), 1, "identical doors collapse");
    }

    #[test]
    fn merge_unites_surfaces() {
        let one = DoorCatalog::new(vec![door("--a", "a")]);
        let two = DoorCatalog::new(vec![
            door("--a", "a"), // same door seen from elsewhere
            Door::new(
                DoorSurface::Http {
                    binary: "demo-server".into(),
                    method: "GET".into(),
                },
                "/api/doors",
                vec![],
                "the catalog itself",
                vec![],
                DoorGovernance::ReadOnly,
                vec![],
            ),
        ]);
        let merged = DoorCatalog::merge([one, two]);
        assert_eq!(merged.len(), 2);
        // A merged catalog's identity is the identity of its content.
        let again = DoorCatalog::merge([
            DoorCatalog::new(merged.doors.clone()),
            DoorCatalog::new(vec![]),
        ]);
        assert_eq!(again.catalog_id, merged.catalog_id);
    }

    #[test]
    fn serde_roundtrip_preserves_identity() {
        let catalog = DoorCatalog::new(vec![door("--a", "a")]);
        let json = serde_json::to_string(&catalog).unwrap();
        let back: DoorCatalog = serde_json::from_str(&json).unwrap();
        assert_eq!(back, catalog);
    }
}
