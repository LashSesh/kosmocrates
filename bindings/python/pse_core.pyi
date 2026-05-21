"""
Type stubs for pse_core — PSE Python bindings.

Install:  maturin develop  (in bindings/python/ with an active venv)
"""

from __future__ import annotations
from typing import Optional, List

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


class PseState:
    """PSE cognitive substrate state.

    Cold start::

        state = PseState()

    Warm start (cross-session memory)::

        memory = open("memory.json").read()
        state  = PseState(memory_json=memory)

    Process an LLM response and persist::

        crystals = state.process_text(llm_response)
        open("memory.json", "w").write(state.save_memory())
    """

    def __init__(
        self,
        memory_json: Optional[str] = None,
        source_name: Optional[str] = None,
    ) -> None:
        """Create a new PSE state.

        Args:
            memory_json: JSON string from a previous ``save_memory()`` call.
                         Omit for a cold-start session.
            source_name: Optional label for this observation source.
        """
        ...

    def step(self, data: bytes) -> Optional[PseCrystal]:
        """Process a single raw-bytes observation.

        Returns a ``PseCrystal`` if the Kairos gate fires, otherwise ``None``.
        """
        ...

    def step_text(self, text: str) -> Optional[PseCrystal]:
        """Process a single text observation.

        Equivalent to ``step(text.encode())``.
        """
        ...

    def process_text(self, text: str) -> List[PseCrystal]:
        """Ingest a full text (e.g. an LLM response). Returns all crystals formed.

        The text is split into sentence-level chunks and processed with a
        4-observation sliding window. This is the recommended entry point for
        LLM response ingestion.
        """
        ...

    def pattern_hits(self) -> int:
        """Pattern-memory hits since state creation.

        A hit means PSE recognised a topology from a prior session in the
        current observation stream — the cross-session memory proof.
        """
        ...

    def commit_index(self) -> int:
        """Monotone tick counter. Equals total number of macro_step calls made."""
        ...

    def crystal_count(self) -> int:
        """Total crystals in this state (loaded from memory + newly formed)."""
        ...

    def crystals(self) -> List[PseCrystal]:
        """Return all accumulated crystals as a list."""
        ...

    def save_memory(self) -> str:
        """Serialise accumulated crystals to JSON for cross-session persistence.

        Pass the returned string as ``memory_json`` to a future ``PseState()`` call.
        """
        ...

    def __repr__(self) -> str: ...
