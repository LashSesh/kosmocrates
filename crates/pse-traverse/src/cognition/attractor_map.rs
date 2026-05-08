//! Attractor ranking surface used by the singularity trigger and the
//! candidate bundle. The `AttractorMap` is content-addressed and
//! permutation-stable.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::primitives::{CognitionError, Fixed};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AttractorEntry {
    pub attractor_id: Hash256,
    pub state_hash: Hash256,
    pub depth: Fixed,
    pub stability: Fixed,
    pub score: Fixed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttractorMap {
    pub map_id: Hash256,
    pub entries: Vec<AttractorEntry>,
    pub best: Option<Hash256>,
}

impl AttractorMap {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        // Sort entries lex on (-score, attractor_id) so the best
        // attractor is the first element. We approximate by sorting
        // by score ascending then reversing so highest-score wins.
        self.entries.sort();
        self.entries.reverse();
        self.best = self.entries.first().map(|e| e.attractor_id.clone());
        // Re-sort to canonical ascending order before hashing.
        self.entries.sort();
        let mut probe = self.clone();
        probe.map_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.map_id = Hash256(id);
        Ok(self)
    }
}
