use thiserror::Error;

pub type ScanResult<T> = Result<T, ScanError>;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("graph has {n} nodes, maximum is 7")]
    TooManyNodes { n: usize },

    #[error("graph has 0 nodes")]
    EmptyGraph,

    #[error("adjacency matrix is not symmetric at ({i}, {j})")]
    NotSymmetric { i: usize, j: usize },

    #[error("self-loop detected at node {node}")]
    SelfLoop { node: usize },

    #[error("non-binary weight {weight} at ({i}, {j})")]
    NonBinaryWeight { i: usize, j: usize, weight: f64 },

    #[error("adjacency matrix dimensions {rows}x{cols} do not match")]
    DimensionMismatch { rows: usize, cols: usize },

    #[error("node index {index} out of bounds (n={n})")]
    InvalidIndex { index: usize, n: usize },

    #[error("orbit-stabilizer theorem violated: {orbit} * {stabilizer} != 5040")]
    OrbitStabilizerViolation { orbit: usize, stabilizer: usize },

    #[error(transparent)]
    Serde(#[from] serde_json::Error),

    #[error("adapter error: {0}")]
    AdapterError(String),
}
