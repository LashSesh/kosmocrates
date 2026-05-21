"""
Type stubs for pse_core — PSE Python bindings.

Install:  maturin develop --release  (in bindings/python/ with an active venv)
"""

from __future__ import annotations
from typing import Optional, List, Tuple

# ── PseCrystal ────────────────────────────────────────────────────────────────

class PseCrystal:
    """A crystallised stable-topology observation.

    Crystals are content-addressed and deterministic: the same observation
    sequence always produces the same ``id``.
    """

    @property
    def id(self) -> str:
        """64-character hex string — content-addressed, deterministic."""
        ...

    @property
    def stability(self) -> float:
        """Stability score in [0, 1]. Higher = more structurally stable."""
        ...

    @property
    def region_size(self) -> int:
        """Number of vertices in the crystal's topological region."""
        ...

    def to_json(self) -> str:
        """Serialise this crystal to a JSON string."""
        ...

    def __repr__(self) -> str: ...


# ── PseCrystalRecord ─────────────────────────────────────────────────────────

class PseCrystalRecord:
    """A crystal together with the source sentences that produced it.

    Returned by ``PseState.process_text_tracked()``.
    Pass ``PseState.save_records()`` output to ``render_crystal_context()``
    to generate LLM-injectable context.
    """

    @property
    def crystal(self) -> PseCrystal:
        """The underlying crystal."""
        ...

    @property
    def source_chunks(self) -> List[str]:
        """The sentence chunks from the sliding window that fired the gate."""
        ...

    @property
    def session(self) -> int:
        """1-indexed session number in which this crystal was formed."""
        ...

    @property
    def question(self) -> str:
        """The prompt/question active when this crystal was formed."""
        ...

    def to_json(self) -> str:
        """Serialise this record to a JSON string."""
        ...

    def __repr__(self) -> str: ...


# ── PseState ─────────────────────────────────────────────────────────────────

class PseState:
    """PSE cognitive substrate state.

    Cold start::

        state = PseState()

    Warm start (cross-session memory + records)::

        state = PseState(
            memory_json=open("crystals.json").read(),
            records_json=open("records.json").read(),
        )

    Session 1–2: use ``process_text`` (no provenance needed for replay).
    Session 3+:  use ``process_text_tracked`` to build context for A/B.
    """

    def __init__(
        self,
        memory_json: Optional[str] = None,
        records_json: Optional[str] = None,
        source_name: Optional[str] = None,
    ) -> None:
        """Create a new PSE state.

        Args:
            memory_json:  JSON from a previous ``save_memory()`` call.
            records_json: JSON from a previous ``save_records()`` call.
            source_name:  Label for this observation source (default: "pse-python").
        """
        ...

    # ── Low-level step ────────────────────────────────────────────────────────

    def step(self, data: bytes) -> Optional[PseCrystal]:
        """Process a single raw-bytes observation. Returns crystal if gate fires."""
        ...

    def step_text(self, text: str) -> Optional[PseCrystal]:
        """Process a single text observation. Equivalent to ``step(text.encode())``."""
        ...

    # ── High-level ingestion ──────────────────────────────────────────────────

    def process_text(self, text: str) -> List[PseCrystal]:
        """Ingest a full text. Returns all crystals formed.

        Use for sessions 1–2 (replay does not need provenance).
        """
        ...

    def process_text_tracked(
        self,
        text: str,
        session: int = 1,
        question: str = "",
    ) -> List[PseCrystalRecord]:
        """Ingest a full text and record source provenance per crystal.

        Use for sessions 3+ to build context for ``render_crystal_context()``.

        Args:
            text:     The LLM response or text to ingest.
            session:  1-indexed session number (shown in context output).
            question: The prompt/question that produced this text.
        """
        ...

    # ── Accessors ─────────────────────────────────────────────────────────────

    def pattern_hits(self) -> int:
        """Pattern-memory hits since state creation (cross-session recognition)."""
        ...

    def commit_index(self) -> int:
        """Monotone tick counter. Equals total macro_step calls made."""
        ...

    def crystal_count(self) -> int:
        """Total crystals (loaded + newly formed)."""
        ...

    def record_count(self) -> int:
        """Total crystal records with provenance."""
        ...

    def crystals(self) -> List[PseCrystal]:
        """Return all accumulated crystals."""
        ...

    def records(self) -> List[PseCrystalRecord]:
        """Return all accumulated crystal records (with provenance)."""
        ...

    # ── Persistence ───────────────────────────────────────────────────────────

    def save_memory(self) -> str:
        """Serialise crystals to JSON. Pass as ``memory_json`` next session."""
        ...

    def save_records(self) -> str:
        """Serialise crystal records to JSON. Pass as ``records_json`` next session."""
        ...

    def __repr__(self) -> str: ...


# ── Module-level functions ────────────────────────────────────────────────────

def render_crystal_context(records_json: str, top_k: int = 5) -> str:
    """Render crystal records into an LLM-injectable context block.

    Args:
        records_json: JSON from ``PseState.save_records()``.
        top_k:        Number of most-stable crystals to include (default: 5).

    Returns a string ready to prepend to a system prompt.

    Example::

        context = pse_core.render_crystal_context(state.save_records(), top_k=5)
        response = llm_call(question, system_extra=context)
    """
    ...


def score_coverage(text: str, keywords: List[str]) -> Tuple[int, int]:
    """Count how many keywords (case-insensitive) appear in text.

    Returns ``(hits, total)`` — divide for a coverage percentage.

    Example::

        hits, total = pse_core.score_coverage(response, DOMAIN_KEYWORDS)
        print(f"{hits}/{total} = {hits/total:.0%}")
    """
    ...
