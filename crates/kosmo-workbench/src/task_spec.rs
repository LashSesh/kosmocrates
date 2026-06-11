use kosmo_core::{Digest, EvidenceKind, EvidenceRef, FoundryCheckKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What a Workbench task is meant to accomplish.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskKind {
    WorkspaceScan { root: String },
    FoundryCheck { kind: FoundryCheckKind },
    HyphaePassiveRun,
    DryRunReport,
    Custom { name: String },
}

/// Serialize-only content struct for content-addressing.
#[derive(Serialize)]
struct TaskContent<'a> {
    kind: &'a TaskKind,
    context_pack_id: &'a Digest,
    policy_id: &'a Digest,
    run_id: &'a Digest,
    labels: &'a BTreeMap<String, String>,
}

/// A single unit of work declared by the Workbench.
///
/// `task_id` is the content-addressed digest of the task's identity.
/// Tasks are immutable once created — to change a task, create a new one.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskSpec {
    pub task_id: Digest,
    pub kind: TaskKind,
    pub context_pack_id: Digest,
    pub policy_id: Digest,
    pub run_id: Digest,
    pub labels: BTreeMap<String, String>,
}

impl TaskSpec {
    pub fn new(kind: TaskKind, context_pack_id: Digest, policy_id: Digest, run_id: Digest) -> Self {
        let mut spec = Self {
            task_id: Digest::ZERO,
            kind,
            context_pack_id,
            policy_id,
            run_id,
            labels: BTreeMap::new(),
        };
        spec.task_id = spec.compute_id();
        spec
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self.task_id = self.compute_id();
        self
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&TaskContent {
            kind: &self.kind,
            context_pack_id: &self.context_pack_id,
            policy_id: &self.policy_id,
            run_id: &self.run_id,
            labels: &self.labels,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.task_id == self.compute_id()
    }

    pub fn to_evidence_ref(&self) -> EvidenceRef {
        EvidenceRef::new(
            self.task_id,
            EvidenceKind::RunRecord,
            format!("task:{:?}", self.kind),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::Digest;

    #[test]
    fn task_spec_id_deterministic() {
        let pid = Digest::of_bytes(b"pol");
        let rid = Digest::of_bytes(b"run");
        let ctx = Digest::of_bytes(b"ctx");
        let t1 = TaskSpec::new(TaskKind::DryRunReport, ctx, pid, rid);
        let t2 = TaskSpec::new(TaskKind::DryRunReport, ctx, pid, rid);
        assert_eq!(t1.task_id, t2.task_id);
    }

    #[test]
    fn task_spec_verify_id() {
        let t = TaskSpec::new(
            TaskKind::WorkspaceScan {
                root: "/repo".into(),
            },
            Digest::ZERO,
            Digest::ZERO,
            Digest::ZERO,
        )
        .with_label("phase", "2");
        assert!(t.verify_id());
    }

    #[test]
    fn task_spec_label_changes_id() {
        let base = TaskSpec::new(
            TaskKind::DryRunReport,
            Digest::ZERO,
            Digest::ZERO,
            Digest::ZERO,
        );
        let labeled = base.clone().with_label("x", "y");
        assert_ne!(base.task_id, labeled.task_id);
    }
}
