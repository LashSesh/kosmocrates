//! `DatasetManifest` and split definitions (§9.1, §10.1).
//!
//! A dataset manifest is content-addressed: any change to the splits,
//! row count or seed changes the manifest id and therefore the spec
//! id of any evaluation that references it.

use serde::{Deserialize, Serialize};

use crate::primitives::{content_address, EvalError, Hash256};

/// One of the three logical splits mandated by §9.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DatasetSplitKind {
    /// `calibration` — used to fit thresholds and adapter profiles.
    Calibration,
    /// `validation` — used to choose configurations.
    Validation,
    /// `test` — final, frozen, no calibration allowed.
    Test,
}

/// One split of a dataset.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DatasetSplit {
    /// Which split this is.
    pub kind: DatasetSplitKind,
    /// Number of rows / events / tasks in the split.
    pub row_count: u64,
    /// Optional content hash of the rows.
    pub rows_hash: Hash256,
    /// Optional content hash of the gold labels (mandatory when the
    /// associated workload claims success).
    pub labels_hash: Option<Hash256>,
}

/// `DatasetManifest` (§4.2 / §10.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetManifest {
    /// Content-addressed dataset id.
    pub dataset_id: Hash256,
    /// Stable, human-readable name (e.g. `"d.stream_minimal"`).
    pub name: String,
    /// Domain tag (e.g. `"synthetic"`, `"vitals"`, `"binance"`).
    pub domain: String,
    /// RNG seed (no implicit randomness — all sampling MUST be
    /// reproducible).
    pub seed: u64,
    /// The three logical splits.
    pub splits: Vec<DatasetSplit>,
    /// Optional schema-of-rows hash.
    pub schema_hash: Option<Hash256>,
}

impl DatasetManifest {
    /// Build a synthetic dataset manifest with the canonical three
    /// splits (small, equal-sized).
    pub fn synthetic(name: impl Into<String>) -> Self {
        let name = name.into();
        let splits = vec![
            DatasetSplit {
                kind: DatasetSplitKind::Calibration,
                row_count: 64,
                rows_hash: Hash256::zero(),
                labels_hash: None,
            },
            DatasetSplit {
                kind: DatasetSplitKind::Validation,
                row_count: 32,
                rows_hash: Hash256::zero(),
                labels_hash: None,
            },
            DatasetSplit {
                kind: DatasetSplitKind::Test,
                row_count: 64,
                rows_hash: Hash256::zero(),
                labels_hash: None,
            },
        ];
        let m = DatasetManifest {
            dataset_id: Hash256::zero(),
            name: name.clone(),
            domain: "synthetic".into(),
            seed: 0,
            splits,
            schema_hash: None,
        };
        m.clone().with_id().unwrap_or(m)
    }

    /// Recompute the content-addressed `dataset_id` after any field
    /// change.
    pub fn with_id(mut self) -> Result<Self, EvalError> {
        self.splits.sort();
        let mut probe = self.clone();
        probe.dataset_id = Hash256::zero();
        self.dataset_id = content_address(&probe)?;
        Ok(self)
    }

    /// Returns the named split, if any.
    pub fn split(&self, kind: DatasetSplitKind) -> Option<&DatasetSplit> {
        self.splits.iter().find(|s| s.kind == kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_has_three_splits() {
        let m = DatasetManifest::synthetic("d.test");
        assert!(m.split(DatasetSplitKind::Calibration).is_some());
        assert!(m.split(DatasetSplitKind::Validation).is_some());
        assert!(m.split(DatasetSplitKind::Test).is_some());
    }

    #[test]
    fn id_changes_with_seed() {
        let mut a = DatasetManifest::synthetic("d.test");
        let id_a = a.dataset_id.clone();
        a.seed = 42;
        let a = a.with_id().unwrap();
        assert_ne!(id_a, a.dataset_id);
    }
}
