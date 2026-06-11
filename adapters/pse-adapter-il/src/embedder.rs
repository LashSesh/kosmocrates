//! Text-embedding seam — the versioned socket between free text and the
//! store's retrieval vectors.
//!
//! Every vector the file-based [`ILStore`] persists or compares is produced
//! by exactly one [`TextEmbedder`], and the store's index records *which*
//! one (`embedder_id`). Opening a populated store with a different embedder
//! is a hard error: cosine similarity between vectors from two different
//! embedding functions is meaningless, and a silent mismatch would degrade
//! recall into noise instead of failing.
//!
//! The default — [`HashEmbedder8`], id `hash8-v1` — wraps the original
//! [`text_to_vector8`] 4-gram hash projection bit-for-bit, so every ledger
//! written before this seam existed remains readable and cosine-compatible
//! (a missing `embedder_id` on a populated index normalises to `hash8-v1`).
//!
//! A real embedding model (768+ trained dimensions) becomes a drop-in:
//! implement the trait, open the store with
//! [`ILStore::open_with_embedder`], and the integrity discipline —
//! tagging, dimension consistency, loud mismatches — carries over
//! unchanged.
//!
//! [`ILStore`]: crate::ILStore
//! [`ILStore::open_with_embedder`]: crate::ILStore::open_with_embedder
//! [`text_to_vector8`]: crate::adapter::text_to_vector8

/// A named, fixed-dimension text-embedding function.
///
/// Contract:
/// - **Deterministic.** Same text ⇒ same vector, across processes.
/// - **Stable identity.** `id()` names the function *and* its version;
///   any change to the mapping requires a new id.
/// - **Fixed dimension.** Every vector has exactly `dim()` components.
pub trait TextEmbedder: Send + Sync {
    /// Stable identifier, e.g. `"hash8-v1"`.
    fn id(&self) -> &str;
    /// Number of vector components this embedder produces.
    fn dim(&self) -> usize;
    /// Embed `text` into a `dim()`-component vector.
    fn embed(&self, text: &str) -> Vec<f64>;
}

/// The original 8-dimensional 4-gram hash projection
/// ([`text_to_vector8`](crate::adapter::text_to_vector8)), packaged as the
/// default [`TextEmbedder`].
///
/// Structural, not semantic: it captures lexical overlap (shared 4-grams),
/// which is exactly what the hash projection always did — the seam makes
/// that explicit and replaceable instead of implicit and load-bearing.
#[derive(Debug, Clone, Copy, Default)]
pub struct HashEmbedder8;

/// The id every pre-seam ledger is normalised to.
pub const HASH8_EMBEDDER_ID: &str = "hash8-v1";

impl TextEmbedder for HashEmbedder8 {
    fn id(&self) -> &str {
        HASH8_EMBEDDER_ID
    }

    fn dim(&self) -> usize {
        8
    }

    fn embed(&self, text: &str) -> Vec<f64> {
        crate::adapter::text_to_vector8(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash8_is_bit_for_bit_the_legacy_projection() {
        let e = HashEmbedder8;
        assert_eq!(e.id(), "hash8-v1");
        assert_eq!(e.dim(), 8);
        let text = "missing test coverage for python module routing";
        assert_eq!(e.embed(text), crate::adapter::text_to_vector8(text));
        assert_eq!(e.embed(text).len(), e.dim());
    }
}
