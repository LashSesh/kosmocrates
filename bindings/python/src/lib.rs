//! Python bindings for PSE — Post-Symbolic Engine.
//!
//! Build with maturin:
//!   maturin develop          # editable install into current venv
//!   maturin build --release  # produce a wheel

use ::pse_core::{load_memory_from_crystals, macro_step, GlobalState};
use ::pse_graph::PassthroughAdapter;
use ::pse_types::{Config, SemanticCrystal};
use pyo3::prelude::*;

// ── Config ────────────────────────────────────────────────────────────────────

fn default_config() -> Config {
    let mut cfg = Config::default();
    cfg.calibration.enabled = true;
    cfg.calibration.target_pass_rate = 0.20;
    cfg.calibration.window = 80;
    cfg.calibration.warmup_ticks = 10;
    cfg.carrier.adaptive = true;
    cfg
}

// ── Text chunking (mirrors tools/pse-llm-demo/src/observe.rs) ────────────────

fn chunk_text(text: &str) -> Vec<Vec<u8>> {
    let mut chunks: Vec<Vec<u8>> = Vec::new();

    for paragraph in text.split('\n') {
        let para = paragraph.trim();
        if para.is_empty() {
            continue;
        }
        let mut start = 0;
        let chars: Vec<char> = para.chars().collect();
        for (i, &ch) in chars.iter().enumerate() {
            if matches!(ch, '.' | '!' | '?') {
                let sentence: String = chars[start..=i].iter().collect();
                let sentence = sentence.trim().to_string();
                if sentence.len() >= 8 {
                    chunks.push(sentence.into_bytes());
                }
                start = i + 1;
            }
        }
        let tail: String = chars[start..].iter().collect();
        let tail = tail.trim().to_string();
        if tail.len() >= 8 {
            chunks.push(tail.into_bytes());
        }
    }

    if chunks.len() < 4 {
        chunks = text
            .split_whitespace()
            .collect::<Vec<_>>()
            .chunks(8)
            .map(|w| w.join(" ").into_bytes())
            .collect();
    }

    chunks
}

// ── Windowed ingestion ────────────────────────────────────────────────────────

fn ingest_chunks(
    state: &mut GlobalState,
    config: &Config,
    adapter: &PassthroughAdapter,
    chunks: &[Vec<u8>],
) -> Vec<SemanticCrystal> {
    const WINDOW: usize = 4;
    let mut crystals = Vec::new();
    let n = chunks.len();
    for i in 0..n.saturating_sub(WINDOW - 1) {
        let batch = chunks[i..(i + WINDOW).min(n)].to_vec();
        if let Ok(Some(c)) = macro_step(state, &batch, config, adapter) {
            crystals.push(c);
        }
    }
    crystals
}

// ── PseCrystal ───────────────────────────────────────────────────────────────

/// A crystallised stable-topology observation.
///
/// Crystals are content-addressed and deterministic: the same observation
/// sequence always produces the same ``id``.
#[pyclass(unsendable)]
pub struct PseCrystal {
    inner: SemanticCrystal,
}

#[pymethods]
impl PseCrystal {
    /// 64-character hex string — content-addressed, deterministic.
    #[getter]
    fn id(&self) -> String {
        self.inner
            .crystal_id
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// Stability score in [0, 1]. Higher = more structurally stable.
    #[getter]
    fn stability(&self) -> f64 {
        self.inner.stability_score
    }

    /// Number of vertices in the crystal's topological region.
    #[getter]
    fn region_size(&self) -> usize {
        self.inner.region.len()
    }

    /// Serialise this crystal to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let id = self.id();
        format!(
            "PseCrystal(id='{}…', stability={:.3}, region_size={})",
            &id[..8],
            self.inner.stability_score,
            self.inner.region.len()
        )
    }
}

// ── PseState ─────────────────────────────────────────────────────────────────

/// PSE cognitive substrate state.
///
/// Cold start::
///
///     state = PseState()
///
/// Warm start (cross-session memory)::
///
///     memory = open("memory.json").read()
///     state  = PseState(memory_json=memory)
///
/// Process an LLM response and persist::
///
///     crystals = state.process_text(llm_response)
///     open("memory.json", "w").write(state.save_memory())
#[pyclass(unsendable)]
pub struct PseState {
    state:    GlobalState,
    config:   Config,
    adapter:  PassthroughAdapter,
    crystals: Vec<SemanticCrystal>,
}

#[pymethods]
impl PseState {
    /// Create a new PSE state.
    ///
    /// Args:
    ///     memory_json: JSON string from a previous ``save_memory()`` call.
    ///                  Omit for a cold-start session.
    ///     source_name: Optional label for this observation source (default: "pse-python").
    #[new]
    #[pyo3(signature = (memory_json=None, source_name=None))]
    fn new(memory_json: Option<&str>, source_name: Option<&str>) -> PyResult<Self> {
        let config = default_config();
        let mut state = GlobalState::new(&config);
        let mut crystals = Vec::new();

        if let Some(json) = memory_json {
            let loaded: Vec<SemanticCrystal> = serde_json::from_str(json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            load_memory_from_crystals(&mut state, &loaded);
            crystals = loaded;
        }

        let name = source_name.unwrap_or("pse-python");
        let adapter = PassthroughAdapter::new(name);

        Ok(PseState { state, config, adapter, crystals })
    }

    /// Process a single raw-bytes observation.
    ///
    /// Returns a ``PseCrystal`` if the Kairos gate fires, otherwise ``None``.
    fn step(&mut self, data: &[u8]) -> PyResult<Option<PseCrystal>> {
        match macro_step(&mut self.state, &[data.to_vec()], &self.config, &self.adapter) {
            Ok(Some(c)) => {
                self.crystals.push(c.clone());
                Ok(Some(PseCrystal { inner: c }))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())),
        }
    }

    /// Process a single text observation (UTF-8 string).
    ///
    /// Equivalent to ``step(text.encode())``.
    fn step_text(&mut self, text: &str) -> PyResult<Option<PseCrystal>> {
        self.step(text.as_bytes())
    }

    /// Ingest a full text (e.g. an LLM response). Returns all crystals formed.
    ///
    /// The text is split into sentence-level chunks and processed with a
    /// 4-observation sliding window — the same strategy used by ``pse-llm-demo``.
    /// This is the recommended method for LLM response ingestion.
    fn process_text(&mut self, text: &str) -> PyResult<Vec<PseCrystal>> {
        let chunks = chunk_text(text);
        let new = ingest_chunks(&mut self.state, &self.config, &self.adapter, &chunks);
        let out = new
            .into_iter()
            .map(|c| {
                self.crystals.push(c.clone());
                PseCrystal { inner: c }
            })
            .collect();
        Ok(out)
    }

    /// Number of pattern-memory hits since state creation.
    ///
    /// A hit means PSE recognised a topology from a prior session in the
    /// current observation stream — the cross-session memory proof.
    fn pattern_hits(&self) -> u64 {
        self.state.pattern_hits
    }

    /// Monotone tick counter. Equals total number of ``macro_step`` calls made.
    fn commit_index(&self) -> u64 {
        self.state.commit_index
    }

    /// Total crystals in this state (loaded from memory + newly formed).
    fn crystal_count(&self) -> usize {
        self.crystals.len()
    }

    /// Return all accumulated crystals as a list.
    fn crystals(&self) -> Vec<PseCrystal> {
        self.crystals
            .iter()
            .map(|c| PseCrystal { inner: c.clone() })
            .collect()
    }

    /// Serialise accumulated crystals to a JSON string for cross-session persistence.
    ///
    /// Pass the returned string as ``memory_json`` to a future ``PseState()`` call.
    fn save_memory(&self) -> PyResult<String> {
        serde_json::to_string(&self.crystals)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PseState(tick={}, crystals={}, hits={})",
            self.state.commit_index,
            self.crystals.len(),
            self.state.pattern_hits,
        )
    }
}

// ── Module ───────────────────────────────────────────────────────────────────

// Function must match the cdylib `name` in Cargo.toml so maturin finds
// PyInit_pse_core.  The `#[pyo3(name)]` attribute renames only the Python
// module; we keep the Rust symbol name matching the lib name.
#[pymodule]
fn pse_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PseState>()?;
    m.add_class::<PseCrystal>()?;
    Ok(())
}
