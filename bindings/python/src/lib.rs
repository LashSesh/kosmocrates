//! Python bindings for PSE — Post-Symbolic Engine.
//!
//! Build with maturin:
//!   maturin develop --release   # editable install into current venv
//!   maturin build  --release    # produce a wheel

use std::f64::consts::TAU;

use ::pse_core::{load_memory_from_crystals, macro_step, GlobalState};
use ::pse_graph::{ObservationAdapter, ObserveError};
use ::pse_types::{
    content_address_raw, Config, MeasurementContext, Observation, ProvenanceEnvelope,
    SemanticCrystal,
};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

struct TextPhaseAdapter {
    source_id: String,
}

impl TextPhaseAdapter {
    fn new(source: impl Into<String>) -> Self {
        Self { source_id: source.into() }
    }
}

impl ObservationAdapter for TextPhaseAdapter {
    fn source_id(&self) -> &str { &self.source_id }

    fn canonicalize(&self, raw: &[u8], context: &MeasurementContext) -> Result<Observation, ObserveError> {
        let phase = if raw.is_empty() { 0.0 } else {
            let avg: f64 = raw.iter().map(|&b| b as f64).sum::<f64>() / raw.len() as f64;
            (avg / 255.0) * TAU
        };
        let payload = raw.to_vec();
        let digest = content_address_raw(&payload);
        Ok(Observation {
            timestamp: 0.0,
            source_id: self.source_id.clone(),
            provenance: ProvenanceEnvelope { origin: self.source_id.clone(), chain: Vec::new(), sig: None },
            payload,
            context: context.clone(),
            digest,
            schema_version: "1.0.0".to_string(),
            phase_hint: Some(phase),
        })
    }
}

// ── Config ────────────────────────────────────────────────────────────────────

fn default_config() -> Config {
    let mut cfg = Config::default();
    cfg.calibration.enabled = true;
    cfg.calibration.target_pass_rate = 0.30;
    cfg.calibration.window = 20;
    cfg.calibration.warmup_ticks = 2;
    cfg.carrier.adaptive = true;
    cfg.thresholds.d = 0.05;
    cfg.thresholds.q = 0.05;
    cfg.thresholds.r = 0.05;
    cfg.thresholds.g = 0.05;
    cfg.thresholds.j = 0.05;
    cfg.thresholds.p = 0.05;
    cfg.thresholds.n = 0.05;
    cfg.thresholds.k = 0.05;
    cfg.consensus.consensus_threshold = 0.3;
    cfg.consensus.mirror_consistency_eta = 0.3;
    cfg.consensus.por_kappa_bar = 0.3;
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
    adapter: &TextPhaseAdapter,
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

/// Like `ingest_chunks` but also returns the source sentences per crystal.
fn ingest_chunks_tracked(
    state: &mut GlobalState,
    config: &Config,
    adapter: &TextPhaseAdapter,
    chunks: &[Vec<u8>],
) -> Vec<(SemanticCrystal, Vec<String>)> {
    const WINDOW: usize = 4;
    let mut out = Vec::new();
    let n = chunks.len();
    for i in 0..n.saturating_sub(WINDOW - 1) {
        let batch = chunks[i..(i + WINDOW).min(n)].to_vec();
        if let Ok(Some(c)) = macro_step(state, &batch, config, adapter) {
            let sources: Vec<String> = batch
                .iter()
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .collect();
            out.push((c, sources));
        }
    }
    out
}

// ── CrystalRecordData (internal, serialisable) ────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
struct CrystalRecordData {
    crystal: SemanticCrystal,
    source_chunks: Vec<String>,
    session: usize,
    question: String,
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
        self.inner.crystal_id.iter().map(|b| format!("{b:02x}")).collect()
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

// ── PseCrystalRecord ─────────────────────────────────────────────────────────

/// A crystal together with the source sentences that produced it.
///
/// Returned by ``PseState.process_text_tracked()``.
/// Pass a list of these to ``render_crystal_context()`` to build LLM context.
#[pyclass(unsendable)]
pub struct PseCrystalRecord {
    inner: CrystalRecordData,
}

#[pymethods]
impl PseCrystalRecord {
    #[getter]
    fn crystal(&self) -> PseCrystal {
        PseCrystal { inner: self.inner.crystal.clone() }
    }

    /// The sentence chunks from the sliding window that fired the Kairos gate.
    #[getter]
    fn source_chunks(&self) -> Vec<String> {
        self.inner.source_chunks.clone()
    }

    /// 1-indexed session number in which this crystal was formed.
    #[getter]
    fn session(&self) -> usize {
        self.inner.session
    }

    /// The question / prompt that was active when this crystal formed.
    #[getter]
    fn question(&self) -> String {
        self.inner.question.clone()
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let id: String = self
            .inner
            .crystal
            .crystal_id
            .iter()
            .take(4)
            .map(|b| format!("{b:02x}"))
            .collect();
        format!(
            "PseCrystalRecord(id='{}…', stability={:.3}, session={}, chunks={})",
            id,
            self.inner.crystal.stability_score,
            self.inner.session,
            self.inner.source_chunks.len()
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
/// Warm start (cross-session memory + records)::
///
///     state = PseState(memory_json=open("crystals.json").read(),
///                      records_json=open("records.json").read())
///
/// Session 1–2: use ``process_text`` (no provenance needed for replay).
/// Session 3+:  use ``process_text_tracked`` to build context for A/B.
#[pyclass(unsendable)]
pub struct PseState {
    state:    GlobalState,
    config:   Config,
    adapter:  TextPhaseAdapter,
    crystals: Vec<SemanticCrystal>,
    records:  Vec<CrystalRecordData>,
}

#[pymethods]
impl PseState {
    /// Create a new PSE state.
    ///
    /// Args:
    ///     memory_json:  JSON from a previous ``save_memory()`` call (crystals).
    ///     records_json: JSON from a previous ``save_records()`` call (provenance).
    ///     source_name:  Label for this observation source (default: "pse-python").
    #[new]
    #[pyo3(signature = (memory_json=None, records_json=None, source_name=None))]
    fn new(
        memory_json: Option<&str>,
        records_json: Option<&str>,
        source_name: Option<&str>,
    ) -> PyResult<Self> {
        let config = default_config();
        let mut state = GlobalState::new(&config);
        let mut crystals = Vec::new();
        let mut records = Vec::new();

        if let Some(json) = memory_json {
            let loaded: Vec<SemanticCrystal> = serde_json::from_str(json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            load_memory_from_crystals(&mut state, &loaded);
            crystals = loaded;
        }

        if let Some(json) = records_json {
            records = serde_json::from_str(json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        }

        let name = source_name.unwrap_or("pse-python");
        let adapter = TextPhaseAdapter::new(name);

        Ok(PseState { state, config, adapter, crystals, records })
    }

    // ── Low-level step ────────────────────────────────────────────────────────

    /// Process a single raw-bytes observation. Returns a crystal if the gate fires.
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

    /// Process a single text observation. Equivalent to ``step(text.encode())``.
    fn step_text(&mut self, text: &str) -> PyResult<Option<PseCrystal>> {
        self.step(text.as_bytes())
    }

    // ── High-level ingestion ──────────────────────────────────────────────────

    /// Ingest a full text. Returns all crystals formed.
    ///
    /// Use for sessions 1–2 (replay, no provenance needed).
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

    /// Ingest a full text and record source provenance per crystal.
    ///
    /// Use for sessions 3+ to build context for ``render_crystal_context()``.
    ///
    /// Args:
    ///     text:     The LLM response or text to ingest.
    ///     session:  1-indexed session number (used for display in context).
    ///     question: The prompt/question that produced this text.
    #[pyo3(signature = (text, session=1, question=""))]
    fn process_text_tracked(
        &mut self,
        text: &str,
        session: usize,
        question: &str,
    ) -> PyResult<Vec<PseCrystalRecord>> {
        let chunks = chunk_text(text);
        let pairs = ingest_chunks_tracked(&mut self.state, &self.config, &self.adapter, &chunks);
        let out = pairs
            .into_iter()
            .map(|(crystal, source_chunks)| {
                self.crystals.push(crystal.clone());
                let rec = CrystalRecordData {
                    crystal,
                    source_chunks,
                    session,
                    question: question.to_string(),
                };
                self.records.push(rec.clone());
                PseCrystalRecord { inner: rec }
            })
            .collect();
        Ok(out)
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    fn pattern_hits(&self) -> u64 {
        self.state.pattern_hits
    }

    fn commit_index(&self) -> u64 {
        self.state.commit_index
    }

    fn crystal_count(&self) -> usize {
        self.crystals.len()
    }

    fn record_count(&self) -> usize {
        self.records.len()
    }

    fn crystals(&self) -> Vec<PseCrystal> {
        self.crystals.iter().map(|c| PseCrystal { inner: c.clone() }).collect()
    }

    fn records(&self) -> Vec<PseCrystalRecord> {
        self.records
            .iter()
            .map(|r| PseCrystalRecord { inner: r.clone() })
            .collect()
    }

    // ── Persistence ───────────────────────────────────────────────────────────

    /// Serialise crystals to JSON (pass as ``memory_json`` next session).
    fn save_memory(&self) -> PyResult<String> {
        serde_json::to_string(&self.crystals)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Serialise crystal records (provenance) to JSON (pass as ``records_json``).
    fn save_records(&self) -> PyResult<String> {
        serde_json::to_string(&self.records)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PseState(tick={}, crystals={}, records={}, hits={})",
            self.state.commit_index,
            self.crystals.len(),
            self.records.len(),
            self.state.pattern_hits,
        )
    }
}

// ── Module-level functions ────────────────────────────────────────────────────

/// Render crystal records into an LLM-injectable context block.
///
/// Args:
///     records_json: JSON from ``PseState.save_records()``.
///     top_k:        Number of most-stable crystals to include (default: 5).
///
/// Returns a string ready to prepend to a system prompt.
#[pyfunction]
#[pyo3(signature = (records_json, top_k=5))]
fn render_crystal_context(records_json: &str, top_k: usize) -> PyResult<String> {
    let mut records: Vec<CrystalRecordData> = serde_json::from_str(records_json)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

    if records.is_empty() {
        return Ok(String::new());
    }

    records.sort_by(|a, b| {
        b.crystal
            .stability_score
            .partial_cmp(&a.crystal.stability_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    records.truncate(top_k);

    let mut out = format!(
        "[PSE Cognitive Substrate — {} stable pattern(s) from prior session(s)]\n\n",
        records.len()
    );
    for r in &records {
        let id: String = r.crystal.crystal_id.iter().take(4).map(|b| format!("{b:02x}")).collect();
        out.push_str(&format!(
            "▸ Pattern #{id} (stability {:.3}, session {})\n",
            r.crystal.stability_score, r.session
        ));
        for chunk in r.source_chunks.iter().take(3) {
            let t = chunk.trim();
            if !t.is_empty() {
                out.push_str(&format!("  \"{t}\"\n"));
            }
        }
        out.push('\n');
    }
    out.push_str(
        "These patterns are topologically stable structures identified by PSE from prior \
         reasoning. Use them as conceptual anchors — not instructions to repeat specific phrases.\n",
    );
    Ok(out)
}

/// Count how many keywords (case-insensitive) appear in text.
///
/// Returns ``(hits, total)`` — divide for a coverage percentage.
///
/// Example::
///
///     hits, total = pse_core.score_coverage(response, THERMO_KEYWORDS)
///     print(f"{hits}/{total} = {hits/total:.0%}")
#[pyfunction]
fn score_coverage(text: &str, keywords: Vec<String>) -> (usize, usize) {
    let lower = text.to_lowercase();
    let hits = keywords.iter().filter(|k| lower.contains(k.as_str())).count();
    (hits, keywords.len())
}

// ── Module ───────────────────────────────────────────────────────────────────

#[pymodule]
fn pse_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PseState>()?;
    m.add_class::<PseCrystal>()?;
    m.add_class::<PseCrystalRecord>()?;
    m.add_function(wrap_pyfunction!(render_crystal_context, m)?)?;
    m.add_function(wrap_pyfunction!(score_coverage, m)?)?;
    Ok(())
}
