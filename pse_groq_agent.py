#!/usr/bin/env python3
"""
PSE Stack Validation Runner -- Live-Test mit Groq API.

Unterstuetzt alle fuenf Fixture-Schemas:
  v1_external_trace_fixture_scaffold  -- Layer 1: Agent Exoskeleton (Datei-Relevanz)
  v1_graph_relevance_fixture          -- Layer 2: Topologie / Graph-Navigation
  v1_pattern_retrieval_fixture        -- Layer 3: Memory / Evidence-Retrieval
  v1_scheduling_decision_fixture      -- Layer 4: Scheduling-Entscheidungen
  v1_macro_step_fixture               -- Layer 5: Core Engine macro_step() Orchestration
  v1_cognition_fixture                -- Cognition: PSE-TRAVERSE-COGNITION-01
  v1_horizon_fixture                  -- Horizon: PSE-TRAVERSE-HORIZON-03
  v1_dynamics_fixture                 -- Dynamics: PSE-TRAVERSE-DYNAMICS-01

Vergleicht in jedem Schema:
  1. Raw LLM:        Groq/Llama ohne PSE-Struktur
  2. PSE-Rahmen:     Groq/Llama mit schicht-spezifischen Kognitions-Constraints

Aufruf:
  python pse_groq_agent.py [fixture_path]
  set GROQ_API_KEY=gsk_...  && python pse_groq_agent.py [fixture_path]
"""

import json
import os
import sys
import time
import urllib.request
import urllib.error

# ─── Konfiguration ───────────────────────────────────────────────────────────

FIXTURE_PATH_DEFAULT = (
    "crates/pse-eval-matrix/fixtures/"
    "agent_exoskeleton/honest_trace_fixture_v1.json"
)
OUTPUT_PATH = "target/tmp/pse_groq_result.json"
GROQ_MODEL = "llama-3.3-70b-versatile"
GROQ_URL = "https://api.groq.com/openai/v1/chat/completions"

CEREBRAS_MODEL = "llama3.1-70b"
CEREBRAS_URL = "https://api.cerebras.ai/v1/chat/completions"

# ─── API-Key laden ────────────────────────────────────────────────────────────

def get_api_key():
    key = (os.environ.get("CEREBRAS_API_KEY", "")
           or os.environ.get("GROQ_API_KEY", "")).strip()
    if not key:
        print("API-Key eingeben (Groq: gsk_...  /  Cerebras: csk_...):")
        key = input("  > ").strip()
    if not (key.startswith("gsk_") or key.startswith("csk-")):
        print("FEHLER: Key muss mit 'gsk_' (Groq) oder 'csk-' (Cerebras) beginnen.")
        sys.exit(1)
    return key


def _provider(api_key):
    """Return (url, model) based on key prefix."""
    if api_key.startswith("csk-"):
        return CEREBRAS_URL, CEREBRAS_MODEL
    return GROQ_URL, GROQ_MODEL


# ─── API aufrufen ─────────────────────────────────────────────────────────────

class TpdLimitError(Exception):
    """Raised when the provider's tokens-per-day limit is exhausted."""
    pass


def call_groq(api_key, prompt, _retry=True, max_tokens=512):
    url, model = _provider(api_key)
    body = json.dumps({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.0,
        "max_tokens": max_tokens,
    }).encode("utf-8")

    user_agent = (
        "cerebras-cloud-sdk-python/1.13.0"
        if api_key.startswith("csk-")
        else "groq-python/0.11.0"
    )
    req = urllib.request.Request(
        url,
        data=body,
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}",
            "User-Agent": user_agent,
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read())
            return data["choices"][0]["message"]["content"].strip()
    except urllib.error.HTTPError as e:
        error_body = e.read().decode("utf-8", errors="replace")
        if e.code == 429:
            # Day-level TPD limit — cannot recover by waiting seconds
            if "TPD" in error_body or "tokens per day" in error_body.lower():
                raise TpdLimitError(error_body[:300])
            if _retry:
                print("       Rate-Limit (429) -- warte 10s...")
                time.sleep(10)
                return call_groq(api_key, prompt, _retry=False, max_tokens=max_tokens)
        return f"HTTP_ERROR:{e.code}:{error_body[:300]}"
    except Exception as e:
        return f"ERROR:{e}"


# ─── Layer 1: Agent Exoskeleton (Datei-Relevanz) ─────────────────────────────

def build_raw_prompt(case):
    items = "\n".join(
        f"  [{c['id']}] {c['text'][:120]}"
        for c in case["candidates"]
    )
    return f"""Du bist ein Software-Debugging-Assistent.

AUFGABE: {case['title']}

KANDIDATEN (moegliche Ursachen):
{items}

Welche 3 Kandidaten sind am relevantesten?
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"]}}"""


def build_pse_prompt(case):
    items = "\n".join(
        f"  [{c['id']}] quelle={c['source']}\n"
        f"           tags={c['tags']}\n"
        f"           text={c['text'][:120]}"
        for c in case["candidates"]
    )
    return f"""Du operierst im PSE Agent-Exoskelett Kognitionsrahmen.

== KOGNITIONS-CONSTRAINTS (strikt anwenden) ==
Phase: Diagnostisch -> Kausal -> Aufloesung

Prioritaetsregeln:
  HOCHPRIORITAT: Items mit Tags [causal, action_required, inspect_path]
  UNTERDRUECKEN: Items mit Tags [stale, red_herring, distractor]
  Quelldateien bevorzugen gegenueber Artefakten (target/, reports)
  Aktuelle Evidenz bevorzugen gegenueber veralteten Berichten

== AUFGABE ==
{case['title']}

== KANDIDATEN ==
{items}

== PFLICHTFORMAT ==
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"], "rejected": ["<id4>", ...]}}"""


# ─── Layer 2: Topologie / Graph-Navigation ───────────────────────────────────

def build_raw_prompt_topology(case):
    ctx = case.get("graph_context", {})
    metrics = ctx.get("system_metrics", {})
    metrics_text = ("  " + ", ".join(f"{k}={v}" for k, v in metrics.items())) if metrics else "  (keine)"
    items = "\n".join(
        f"  [{c['id']}] {c['source']}: {c['text'][:120]}"
        for c in case["candidates"]
    )
    return f"""Du bist ein Graph-Topologie-Analyst.

AUFGABE: {case['title']}

System-Metriken:
{metrics_text}

KANDIDATEN (Knoten / Subgraphen / Pfade):
{items}

Welche 3 Kandidaten sind fuer die aktuelle Fragestellung am relevantesten?
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"]}}"""


def build_pse_prompt_topology(case):
    ctx = case.get("graph_context", {})
    metrics = ctx.get("system_metrics", {})
    metrics_text = "\n".join(f"  {k}: {v}" for k, v in metrics.items()) if metrics else "  (keine)"
    items = "\n".join(
        f"  [{c['id']}] knoten={c['source']}\n"
        f"           tags={c['tags']}\n"
        f"           {c['text'][:120]}"
        for c in case["candidates"]
    )
    return f"""Du operierst im PSE Topologie-Evaluierungsrahmen.

== KOGNITIONS-CONSTRAINTS (Topologie-Layer) ==
Phase: Beobachtung -> Spektrale Analyse -> Kausal-Verbindung

Systemmetriken:
{metrics_text}

Prioritaetsregeln:
  HOCHPRIORITAT: Tags [hub, bridge, causal_connector, high_fiedler, phase_coupled, inspect_path, causal]
  UNTERDRUECKEN: Tags [leaf, isolated, peripheral, spectral_noise, stale, red_herring, distractor]
  Verbundene Teilgraphen vor isolierten Knoten
  Spektrale Zentralitaet (Fiedler-Naeherung) als Primaerkriterium
  Knoten nahe dem Kairos-Gate-Schwellenwert priorisieren

== AUFGABE ==
{case['title']}

== KANDIDATEN ==
{items}

== PFLICHTFORMAT ==
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"], "rejected": ["<id4>", ...]}}"""


# ─── Layer 3: Memory / Evidence-Retrieval ────────────────────────────────────

def build_raw_prompt_memory(case):
    query = case.get("query_description", case["title"])
    items = "\n".join(
        f"  [{c['id']}] {c['source']}: {c['text'][:120]}"
        for c in case["candidates"]
    )
    return f"""Du bist ein Muster-Retrieval-System.

ABFRAGE: {query}

GESPEICHERTE MUSTER (Kandidaten):
{items}

Welche 3 Kandidaten passen am besten zur Abfrage?
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"]}}"""


def build_pse_prompt_memory(case):
    query = case.get("query_description", case["title"])
    items = "\n".join(
        f"  [{c['id']}] muster={c['source']}\n"
        f"           tags={c['tags']}\n"
        f"           {c['text'][:120]}"
        for c in case["candidates"]
    )
    return f"""Du operierst im PSE Gedaechtnis-Evaluierungsrahmen.

== KOGNITIONS-CONSTRAINTS (Memory-Layer) ==
Phase: Abfrage -> Kristall-Signatur-Vergleich -> Resonanz-Filterung

Prioritaetsregeln:
  HOCHPRIORITAT: Tags [high_resonance, verified, canonical, proof_of_isomorphism, causal, inspect_path]
  UNTERDRUECKEN: Tags [stale_crystal, low_confidence, unverified, hash_mismatch, stale, red_herring, distractor]
  Verifizierte aktuelle Muster vor veralteten Kristallen
  Resonanz-Proximitaet + Cosinus-Aehnlichkeit als kombiniertes Kriterium

== ABFRAGE ==
{query}

== KANDIDATEN ==
{items}

== PFLICHTFORMAT ==
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"], "rejected": ["<id4>", ...]}}"""


# ─── Layer 4: Scheduling-Entscheidungen ──────────────────────────────────────

def build_raw_prompt_scheduling(case):
    ctx = case.get("scheduling_context", {})
    m = ctx.get("system_metrics", {})
    metrics_text = f"Drift d={m.get('drift','?')}, Friction F={m.get('friction','?')}, Shock S={m.get('shock','?')}" if m else "(keine Metriken)"
    task_type = case.get("task_type", "prioritize")
    question = ("Welche 3 Aufgaben sollten zurueckgestellt (deferred) werden?"
                if task_type == "defer"
                else "Welche 3 Aufgaben haben hoechste Prioritaet und sollten als naechstes ausgefuehrt werden?")
    items = "\n".join(
        f"  [{c['id']}] {c['source']}: {c['text'][:120]}"
        for c in case["candidates"]
    )
    return f"""Du bist ein Aufgaben-Scheduler.

KONTEXT: {case['title']}
System-Metriken: {metrics_text}

VERFUEGBARE AUFGABEN:
{items}

{question}
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"]}}"""


def build_pse_prompt_scheduling(case):
    ctx = case.get("scheduling_context", {})
    m = ctx.get("system_metrics", {})
    d = m.get("drift", 0)
    s = m.get("shock", 0)
    metrics_text = f"Drift d={m.get('drift','?')}, Friction F={m.get('friction','?')}, Shock S={m.get('shock','?')}" if m else "(keine)"
    task_type = case.get("task_type", "prioritize")
    if task_type == "defer":
        ziel = "ZIEL: Identifiziere Aufgaben die zurueckgestellt werden sollen (hoechste Prioritaet = soll warten)"
        boost = "HOCHPRIORITAT fuer Zurueckstellung: Tags [can_wait, deferred, background, low_priority, non_critical]"
        suppress = "NICHT zurueckstellen: Tags [critical_path, pressure_relief, constraint_fix, kairos_aligned]"
    else:
        ziel = "ZIEL: Identifiziere Aufgaben die als naechstes ausgefuehrt werden sollen"
        boost = "HOCHPRIORITAT: Tags [pressure_relief, constraint_fix, critical_path, kairos_aligned, causal]"
        suppress = "UNTERDRUECKEN: Tags [deferred, low_priority, background, can_wait, non_critical]"
    items = "\n".join(
        f"  [{c['id']}] aufgabe={c['source']}\n"
        f"           tags={c['tags']}\n"
        f"           {c['text'][:120]}"
        for c in case["candidates"]
    )
    return f"""Du operierst im PSE Scheduling-Evaluierungsrahmen.

== KOGNITIONS-CONSTRAINTS (Scheduling-Layer) ==
Phase: Druckmessung -> Strategie-Auswahl -> Aufgaben-Priorisierung

Systemdruck: {metrics_text}
Bei Drift d > 0.7: Constraint-Fix-Tasks bevorzugen (aktuell d={d})
Bei Shock S > 0.8: Load-Reduction und Defer-Tasks priorisieren (aktuell S={s})

{ziel}

Prioritaetsregeln:
  {boost}
  {suppress}

== KONTEXT ==
{case['title']}

== AUFGABEN ==
{items}

== PFLICHTFORMAT ==
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"], "rejected": ["<id4>", ...]}}"""


# ─── Signature Layer: PSE-TRAVERSE-SIGNATURE-01 ──────────────────────────────

def build_raw_prompt_signature(case):
    ctx = case.get("signature_context", {})
    gate = ctx.get("gate_result", {})
    pareto = ctx.get("pareto_objectives", "")
    gate_info = ("Gate: " + ", ".join(f"{k}={v}" for k, v in gate.items())) if gate else ""
    pareto_info = f"Pareto-Ziele: {pareto}" if pareto else ""
    context_block = "\n".join(x for x in [gate_info, pareto_info] if x)
    items = "\n".join(
        f"  [{c['id']}] {c['source']}: {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du bist ein Signatur-Analyse-Experte (PSE-TRAVERSE-SIGNATURE-01).

AUFGABE: {case['title']}

{context_block}

KANDIDATEN:
{items}

Welche 3 Kandidaten sind korrekt fuer diese Situation?
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"]}}"""


def build_pse_prompt_signature(case):
    ctx = case.get("signature_context", {})
    gate = ctx.get("gate_result", {})
    pareto = ctx.get("pareto_objectives", "")
    dom_rule = ctx.get("dominance_rule", "")
    desc = ctx.get("description", "")
    gate_info = "\n".join(f"  {k}: {v}" for k, v in gate.items()) if gate else ""
    items = "\n".join(
        f"  [{c['id']}] item={c['source']}\n"
        f"           tags={c['tags']}\n"
        f"           {c['text'][:140]}"
        for c in case["candidates"]
    )
    pareto_block = f"Pareto-Ziele: {pareto}\nDominanzkritierium: {dom_rule}" if pareto else ""
    return f"""Du operierst im PSE Signature-Evaluierungsrahmen (PSE-TRAVERSE-SIGNATURE-01).

== KOGNITIONS-CONSTRAINTS (Signature Layer) ==
SignatureGate-Checks:
  gap_score >= min_gap_score
  degeneracy_score <= max_degeneracy_score
  fragmentation_score <= max_fragmentation_score
  RegimeHint in allowed_regimes (wenn konfiguriert)

Pareto-Kriterium (NonDominatedFrontier):
  A dominiert B genau dann wenn A >= B in ALLEN Zielen UND A > B in mindestens einem
  Ziele: gap_score (hoeher besser), fragmentation_score (niedriger besser), degeneracy_score (niedriger besser)
  Einzelziel-Selektion (nur gap ODER nur degeneracy) ist ungueltig

{pareto_block}
{gate_info}
{desc}

Prioritaetsregeln:
  HOCHPRIORITAT: Tags [on_frontier, pareto_dominant, causal, valid_remedy,
                       diagnostic, inspect_path, gap_fix, fragmentation_fix]
  UNTERDRUECKEN: Tags [dominated, eliminated, masks_problem, spec_violation, bypass,
                       wrong_phase, wrong_metric, wrong_regime, incomplete_criteria,
                       single_objective, init_only, data_loss, distractor]

== AUFGABE ==
{case['title']}

== KANDIDATEN ==
{items}

== PFLICHTFORMAT ==
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"], "rejected": ["<id4>", ...]}}"""


# ─── Dynamics Layer: PSE-TRAVERSE-DYNAMICS-01 ────────────────────────────────

def build_raw_prompt_dynamics(case):
    ctx = case.get("dynamics_context", {})
    gate = ctx.get("gate_result", {})
    gate_info = "Gate-Ergebnis: " + ", ".join(f"{k}={v}" for k, v in gate.items()) if gate else "(kein Gate)"
    items = "\n".join(
        f"  [{c['id']}] {c['source']}: {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du bist ein Dynamics-Layer-Analyst (PSE-TRAVERSE-DYNAMICS-01).

AUFGABE: {case['title']}

{gate_info}

KANDIDATEN:
{items}

Welche 3 Kandidaten sind korrekt fuer diese Situation?
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"]}}"""


def build_pse_prompt_dynamics(case):
    ctx = case.get("dynamics_context", {})
    gate = ctx.get("gate_result", {})
    gate_info = "\n".join(f"  {k}: {v}" for k, v in gate.items()) if gate else "(kein Gate)"
    desc = ctx.get("description", "")
    items = "\n".join(
        f"  [{c['id']}] op={c['source']}\n"
        f"           tags={c['tags']}\n"
        f"           {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du operierst im PSE Dynamics-Evaluierungsrahmen (PSE-TRAVERSE-DYNAMICS-01).

== KOGNITIONS-CONSTRAINTS (Dynamics Layer) ==
GATE-01: proof=None -> immer Hold (unconditional, keine Config-Ausnahme moeglich)
DynamicGateConfig-Pruefungen (nur wenn proof != None):
  path_delta <= max_path_delta
  alignment >= min_alignment
  energy_delta < 0 (wenn require_energy_decrease=true)
  abs(density_delta) <= max_density_delta (wenn konfiguriert)

Gate-Ergebnis:
{gate_info}

{desc}

MorphodynamicCompressor-Operationen:
  Merge -> reduziert Knotenanzahl und mittlere Verschiebung (path_delta sinkt)
  Prune -> entfernt schwache Kanten (reduziert Bewegungsimpulse)
  Split -> erhoeht Knotenanzahl und Verteilung (path_delta steigt!)
  Hebbian -> passt Kantengewichte an (kein direkter Effekt auf path_delta)

Prioritaetsregeln:
  HOCHPRIORITAT: Tags [gate_01_failclosed, spec_outcome, valid_next_op, path_delta_repair,
                       diagnostic, inspect_path, recovery_path, merge_reduces_delta]
  UNTERDRUECKEN: Tags [wrong_outcome, impossible_here, config_irrelevant, invalid_bypass,
                       wrong_parameter, increases_delta, wrong_operation, masks_problem,
                       init_only, spec_violation, wrong_context, wrong_policy, distractor]

== AUFGABE ==
{case['title']}

== KANDIDATEN ==
{items}

== PFLICHTFORMAT ==
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"], "rejected": ["<id4>", ...]}}"""


# ─── Horizon Layer: PSE-TRAVERSE-HORIZON-03 ──────────────────────────────────

def build_raw_prompt_horizon(case):
    ctx = case.get("horizon_context", {})
    gate = ctx.get("crossing_gate", {})
    gate_info = "Gate-Zustand: " + ", ".join(
        f"{k}={v}" for k, v in gate.items()
    ) if gate else "(kein Gate)"
    items = "\n".join(
        f"  [{c['id']}] {c['source']}: {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du bist ein Horizon-Pipeline-Analyst (PSE-TRAVERSE-HORIZON-03).

AUFGABE: {case['title']}

{gate_info}

KANDIDATEN (moegliche Policies, Outcomes, Aktionen):
{items}

Welche 3 Kandidaten sind fuer diese Situation am korrektesten?
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"]}}"""


def build_pse_prompt_horizon(case):
    ctx = case.get("horizon_context", {})
    gate = ctx.get("crossing_gate", {})
    gate_info = "\n".join(
        f"  {k}: {v}" for k, v in gate.items()
    ) if gate else "(kein Gate)"
    desc = ctx.get("description", "")
    items = "\n".join(
        f"  [{c['id']}] policy={c['source']}\n"
        f"           tags={c['tags']}\n"
        f"           {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du operierst im PSE Horizon-Evaluierungsrahmen (PSE-TRAVERSE-HORIZON-03).

== KOGNITIONS-CONSTRAINTS (Horizon Layer) ==
HorizonCrossingGate: G_cross = G_visible AND G_cone AND G_causal AND G_dual AND tension_ok AND attenuation_ok
CombinedGate: G_v0.3 = G_projection_v2 AND G_cross AND ReplayReady

Crossing-Gate-Zustand:
{gate_info}

{desc}

Deterministische Policy-Tabelle (strikt anwenden):
  NUR !g_visible (alle anderen true) -> WaitForHorizon -> HorizonV3Outcome::WaitForHorizon
  !g_cone                            -> RefineProjectionCone -> HorizonV3Outcome::RefineCone
  !g_causal                          -> MigrateCarrier -> HorizonV3Outcome::NeedsCarrierMigration
  !tension_ok ODER !attenuation_ok   -> Recondense -> HorizonV3Outcome::Recondense
  !g_dual                            -> Recondense -> HorizonV3Outcome::Recondense

Prioritaetsregeln:
  HOCHPRIORITAT: Tags [spec_policy, spec_outcome, causal, diagnostic, inspect_path] mit passendem gate-Label
  UNTERDRUECKEN: Tags [wrong_gate, completed, init_only, last_resort, distractor]

== AUFGABE ==
{case['title']}

== KANDIDATEN ==
{items}

== PFLICHTFORMAT ==
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"], "rejected": ["<id4>", ...]}}"""


# ─── Layer Cognition: PSE-TRAVERSE-COGNITION-01 ───────────────────────────────

def build_raw_prompt_cognition(case):
    ctx = case.get("cognition_context", {})
    gate = ctx.get("handoff_gate", {})
    task_type = case.get("task_type", "select_recovery_actions")

    if task_type == "select_admitted_wormholes":
        question = "Welche 3 Wormhole-Vorschlaege erfuellen alle Zulassungskriterien?"
    else:
        question = "Welche 3 Recovery-Aktionen sind fuer diesen Gate-Fehler korrekt?"

    gate_info = ""
    if gate:
        gate_info = "Gate-Zustand: " + ", ".join(
            f"{k}={v}" for k, v in gate.items() if k != "passed"
        ) + f", passed={gate.get('passed', '?')}"
    desc = ctx.get("description", "")

    items = "\n".join(
        f"  [{c['id']}] {c['source']}: {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du bist ein Kognitions-System-Analyst.

AUFGABE: {case['title']}

Kontext:
{gate_info}
{desc}

KANDIDATEN:
{items}

{question}
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"]}}"""


def build_pse_prompt_cognition(case):
    ctx = case.get("cognition_context", {})
    gate = ctx.get("handoff_gate", {})
    task_type = case.get("task_type", "select_recovery_actions")
    desc = ctx.get("description", "")

    failed_gate = ctx.get("failed_gate", "")
    admission_gate = ctx.get("admission_gate", "")

    if task_type == "select_admitted_wormholes":
        ziel = "ZIEL: Identifiziere Wormhole-Vorschlaege die ALLE vier Kriterien erfuellen"
        boost = "HOCHPRIORITAT: Tags [admitted, valid_reason_code, budget_within_limit, audit_traced, causal]"
        suppress = "UNTERDRUECKEN: Tags [rejected, invalid_reason_code, budget_exceeded, zero_audit_trace, ttl_exceeded, distractor]"
        gate_context = f"Zulassungsgate: {admission_gate}"
    else:
        ziel = f"ZIEL: Identifiziere Recovery-Aktionen fuer den fehlgeschlagenen Gate: {failed_gate}"
        boost = "HOCHPRIORITAT: Tags [spec_policy, valid_next_op, causal] mit passendem Gate-Label"
        suppress = "UNTERDRUECKEN: Tags [wrong_gate, wrong_module, wrong_layer, wrong_context, last_resort, distractor]"
        gate_status = " | ".join(
            f"{k}={v}" for k, v in gate.items() if isinstance(v, bool)
        ) if gate else ""
        gate_context = f"Gate-Zustand: {gate_status}"

    items = "\n".join(
        f"  [{c['id']}] aktion={c['source']}\n"
        f"           tags={c['tags']}\n"
        f"           {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du operierst im PSE Kognitions-Evaluierungsrahmen (PSE-TRAVERSE-COGNITION-01).

== KOGNITIONS-CONSTRAINTS (Cognition Layer) ==
Pipeline: CanonicalState -> State5D -> SpiralMemory -> ConstraintLattice ->
          HypercubePuzzle -> PhasePanorama -> Scheduler -> Wormhole ->
          SelfModel -> DualTrigger -> Fixpoint -> Singularity -> HandoffGate

{gate_context}
{desc}

{ziel}

Prioritaetsregeln:
  {boost}
  {suppress}

Gate-Policy-Mapping (nur fuer Recovery-Faelle):
  G_perc Fehler     -> RefineConstraints (Perkolation)
  G_panorama Fehler -> ExpandPanorama
  G_self Fehler     -> CalibrateOperators
  G_trigger Fehler  -> WaitForPhaseWindow

Wormhole-Zulassung (4-faches AND-Kriterium):
  Budget(w) <= B_max AND TTL(w) <= TTL_max AND Reason(w) in R AND ReplayTrace != 0

== AUFGABE ==
{case['title']}

== KANDIDATEN ==
{items}

== PFLICHTFORMAT ==
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"], "rejected": ["<id4>", ...]}}"""


# ─── Layer 5: Core Engine macro_step() Orchestration ─────────────────────────

def build_raw_prompt_macro_step(case):
    ctx = case.get("engine_context", {})
    state_info = f"Engine-Zustand: {ctx.get('engine_state', '?')}"
    extra = []
    if "por_status" in ctx:
        extra.append(f"PoR-Status: {ctx['por_status']}")
    if "consensus_result" in ctx:
        extra.append(f"Konsensus-Ergebnis: {ctx['consensus_result']}")
    if "gate_passed" in ctx:
        extra.append(f"Gate bestanden: {ctx['gate_passed']}")
    context_block = "\n".join([state_info] + extra)
    items = "\n".join(
        f"  [{c['id']}] {c['source']}: {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du bist ein Engine-Orchestrierungs-Analyst.

AUFGABE: {case['title']}

Engine-Kontext:
{context_block}

KANDIDATEN (moegliche naechste Operatoren):
{items}

Welche 3 Operatoren sind als naechste Schritte am sinnvollsten?
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"]}}"""


def build_pse_prompt_macro_step(case):
    ctx = case.get("engine_context", {})
    state_info = f"Engine-Zustand: {ctx.get('engine_state', '?')}"
    extra = []
    if "por_status" in ctx:
        extra.append(f"PoR-Status: {ctx['por_status']}")
    if "consensus_result" in ctx:
        extra.append(f"Konsensus-Ergebnis: {ctx['consensus_result']}")
    if "gate_passed" in ctx:
        extra.append(f"Gate bestanden: {ctx['gate_passed']}")
    if "description" in ctx:
        extra.append(f"Beschreibung: {ctx['description']}")
    context_block = "\n".join([state_info] + extra)
    items = "\n".join(
        f"  [{c['id']}] operator={c['source']}\n"
        f"           tags={c['tags']}\n"
        f"           {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du operierst im PSE Core-Engine-Orchestrierungsrahmen.

== KOGNITIONS-CONSTRAINTS (macro_step Layer) ==
Phase: Beobachtung -> Resonanz -> Kairos-Gate -> Kristallisierung -> Archivierung

{context_block}

Operatoren-Sequenz-Regeln:
  HOCHPRIORITAT: Tags [valid_next_op, recovery_path, diagnostic, inspect_path, causal]
  UNTERDRUECKEN: Tags [wrong_phase, init_only, completed, redundant, wrong_context, premature, wrong_order, red_herring, distractor]
  Reihenfolge der macro_step()-Pipeline strikt einhalten
  Recovery-Operatoren vor Weiterverarbeitung bei Fehler-Flags
  Init-only-Operatoren (Phase-Ladder-Aufbau) nur beim Start

== AUFGABE ==
{case['title']}

== KANDIDATEN ==
{items}

== PFLICHTFORMAT ==
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"], "rejected": ["<id4>", ...]}}"""



# ─── Topology Layer: PSE-TRAVERSE-TPT-MTL-04 ──────────────────────────────────────────────

def build_raw_prompt_tpt_mtl(case):
    ctx = case.get("tpt_mtl_context", {})
    gate = ctx.get("gate_report", {})
    guard_rule = ctx.get("guard_rule", "")
    gate_info = ("Gate-Report: " + ", ".join(f"{k}={v}" for k, v in gate.items())) if gate else ""
    guard_info = f"Guard-Regel: {guard_rule}" if guard_rule else ""
    context_block = "\n".join(x for x in [gate_info, guard_info] if x)
    items = "\n".join(
        f"  [{c['id']}] {c['source']}: {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du bist ein Topologie-Pipeline-Analyst (PSE-TRAVERSE-TPT-MTL-04).

AUFGABE: {case['title']}

{context_block}

KANDIDATEN:
{items}

Welche 3 Kandidaten sind korrekt fuer diese Situation?
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"]}}"""


def build_pse_prompt_tpt_mtl(case):
    ctx = case.get("tpt_mtl_context", {})
    gate = ctx.get("gate_report", {})
    guard_rule = ctx.get("guard_rule", "")
    desc = ctx.get("description", "")
    gate_info = "\n".join(f"  {k}: {v}" for k, v in gate.items()) if gate else ""
    guard_info = f"Guard-Kriterium: {guard_rule}" if guard_rule else ""
    items = "\n".join(
        f"  [{c['id']}] op={c['source']}\n"
        f"           tags={c['tags']}\n"
        f"           {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du operierst im PSE Topologie-Evaluierungsrahmen (PSE-TRAVERSE-TPT-MTL-04).

== KOGNITIONS-CONSTRAINTS (Topology Layer) ==
outcome_kind() Prioritaet (strikt):
  (1) !boundary OR !replay  -> Abort        (hoechste Prioritaet, ueberschreibt alles)
  (2) !truth                -> Quarantine
  (3) !adapter OR !axis OR !micro_lift OR !carrier -> Recalibrate
  (4) all_passed AND matrix AND emission    -> Emit
  (5) sonst                 -> Hold

TopologyGuard Invariante I-08:
  Jede Mesh-Mutation MUSS TopologyGuardProof erzeugen.
  Kriterien: delta_beta_i in AllowedShift_i (alle Dim.) AND W_p(PD_old,PD_new) <= theta_PD
  Kein Proof -> stille Mutation -> I-08-Verletzung (abgelehnt)
  Proof mit betti_shift_exceeded -> abgelehnt
  Proof mit pd_distance_exceeded -> abgelehnt

Gate-Report:
{gate_info}
{guard_info}

{desc}

Prioritaetsregeln:
  HOCHPRIORITAT: Tags [abort_outcome, spec_outcome, boundary_gate, replay_gate, causal,
                       valid_proof, passes_guard, valid_next_op, diagnostic, inspect_path]
  UNTERDRUECKEN: Tags [wrong_priority, impossible_here, masks_problem, spec_violation,
                       betti_shift_exceeded, pd_distance_exceeded, silent_mutation,
                       I08_violation, no_proof, config_override, distractor, I05_violation]

== AUFGABE ==
{case['title']}

== KANDIDATEN ==
{items}

== PFLICHTFORMAT ==
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"], "rejected": ["<id4>", ...]}}"""


# ─── NCTCS Closure Layer: PSE-NCTCS-CONFORMANCE-01 ───────────────────────────

def build_raw_prompt_nctcs(case):
    ctx = case.get("nctcs_context", {})
    checks = ctx.get("conformance_checks", {})
    mat_rule = ctx.get("materialization_rule", "")
    checks_info = ("Konformitaets-Checks: " + ", ".join(f"{k}={v}" for k, v in checks.items())) if checks else ""
    rule_info = f"Materialisierungsregel: {mat_rule[:120]}" if mat_rule else ""
    context_block = "\n".join(x for x in [checks_info, rule_info] if x)
    items = "\n".join(
        f"  [{c['id']}] {c['source']}: {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du bist ein NCTCS-Konformitaets-Analyst (PSE-NCTCS-CONFORMANCE-01).

AUFGABE: {case['title']}

{context_block}

KANDIDATEN:
{items}

Welche 3 Kandidaten sind korrekt fuer diese Situation?
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"]}}"""


def build_pse_prompt_nctcs(case):
    ctx = case.get("nctcs_context", {})
    checks = ctx.get("conformance_checks", {})
    mat_rule = ctx.get("materialization_rule", "")
    desc = ctx.get("description", "")
    checks_info = "\n".join(f"  {k}: {v}" for k, v in checks.items()) if checks else ""
    items = "\n".join(
        f"  [{c['id']}] class={c['source']}\n"
        f"           tags={c['tags']}\n"
        f"           {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du operierst im PSE NCTCS-Evaluierungsrahmen (PSE-NCTCS-CONFORMANCE-01).

== KOGNITIONS-CONSTRAINTS (NCTCS Closure Layer) ==
Konformitaets-Klassen (kumulativ — jede erfordert alle vorherigen):
  C0_FormalTyped:              NullCenter exogen, Projektion distinct
  C1_PhaseGatedVisibility:     C0 AND visibility.passed AND candidate_requires_visibility_passed
  C2_GateBoundMaterialization: C1 AND materialization.passed AND no_direct_fabric_to_tensor_mutation
  C3_AuditableTensor:          C2 AND trace_replay.passed (100% Replay-Identitaet)
  C4_MacroControl:             C3 AND MacroControlState abgeleitet aus Tensor-History

reached_class = hoechste Klasse fuer die c<n>_passed=true gilt (kumulativ!)
  -> Wenn c2_passed=false: reached_class = C1 (NICHT C2, nicht C3, nicht C4)

NctcsGateOutcome Taxonomie:
  Pass         -> is_materializing()=TRUE (einziger Outcome der Tensor-Revision erlaubt)
  Hold         -> is_materializing()=false (Tensor bleibt unveraendert)
  Refine       -> is_materializing()=false (kein Tensor-Commit im Refinement-Zyklus)
  Reject       -> is_materializing()=false
  Quarantine   -> is_materializing()=false
  NoUpdate     -> is_materializing()=false
  HandoffReady -> is_materializing()=false

C2-Invariante: Ephemeral Fabric DARF NICHT direkt den persistenten Tensor mutieren.
  Alle Tensor-Updates MUESSEN durch die Gate-Kaskade gehen.
  no_direct_fabric_to_tensor_mutation=false -> C2 Pflichtverletzung

MacroControlState (C4) Derivation:
  DARF abgeleitet werden aus: null_center_ref, field_tensor (via Tensor-History),
    trace_head, policies, certificates, eval_summary_hash
  DARF NICHT aus: Resonanz-Feld, Ephemeral Fabric, Agent-State, Phase-State direkt

Konformitaets-Checks:
{checks_info}

Materialisierungsregel: {mat_rule[:200]}

{desc}

Prioritaetsregeln:
  HOCHPRIORITAT: Tags [reached_class, causal, spec_outcome, spec_obligation, gate_materializes,
                       c2_constraint, recovery_action, diagnostic, inspect_path, valid_next_op]
  UNTERDRUECKEN: Tags [wrong_class, wrong_outcome, wrong_status, spec_violation, masks_problem,
                       ephemeral_source, c2_violation, c4_violation, gate_no_update, distractor]

== AUFGABE ==
{case['title']}

== KANDIDATEN ==
{items}

== PFLICHTFORMAT ==
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"], "rejected": ["<id4>", ...]}}"""


# ─── Metatron Closure Layer: PSE-METATRON-MONOLITH-01 ────────────────────────

def build_raw_prompt_metatron(case):
    ctx = case.get("metatron_context", {})
    gate = ctx.get("gate_report", {})
    g_iso_rule = ctx.get("g_iso_rule", "")
    gate_info = ("Gate-Report: " + ", ".join(f"{k}={v}" for k, v in gate.items())) if gate else ""
    rule_info = f"G_iso-Regel: {g_iso_rule[:160]}" if g_iso_rule else ""
    context_block = "\n".join(x for x in [gate_info, rule_info] if x)
    items = "\n".join(
        f"  [{c['id']}] {c['source']}: {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du bist ein Metatron-Closure-Analyst (PSE-METATRON-MONOLITH-01).

AUFGABE: {case['title']}

{context_block}

KANDIDATEN:
{items}

Welche 3 Kandidaten sind korrekt fuer diese Situation?
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"]}}"""


def build_pse_prompt_metatron(case):
    ctx = case.get("metatron_context", {})
    gate = ctx.get("gate_report", {})
    g_iso_rule = ctx.get("g_iso_rule", "")
    desc = ctx.get("description", "")
    gate_info = "\n".join(f"  {k}: {v}" for k, v in gate.items()) if gate else ""
    items = "\n".join(
        f"  [{c['id']}] candidate={c['source']}\n"
        f"           tags={c['tags']}\n"
        f"           {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du operierst im PSE Metatron-Evaluierungsrahmen (PSE-METATRON-MONOLITH-01).

== KOGNITIONS-CONSTRAINTS (Metatron Holistic Eigenmode Closure Layer) ==
Pipeline: ValidationRun -> NCTCSReport -> MacroControlState
  -> LocalMonolithProjection* -> IsomorphicProjectionReport*
  -> SpectralGapStitchReport -> MetatronGate -> HolisticEigenmodeState

MetatronGate (G_meta):
  G_meta = G_nctcs AND G_trace AND G_replay AND G_iso AND G_gap AND G_eval AND G_drift
  Fail-closed: G_meta=0 -> KEINE HolisticEigenmodeState, nur Diagnostic-Report

MetatronClosureOutcome Varianten:
  Closed(HolisticEigenmodeState) -> nur wenn G_meta=1
  Diagnostic(MetatronDiagnosticReport) -> wenn G_meta=0 (Gate-Fehler)
  Rejected(MetatronRejectionReport) -> nur bei Pre-Flight-Fehler (vor Gate-Auswertung)
    Beispiel: trace_head=zero -> Rejected; G_gap=false -> Diagnostic (nicht Rejected!)

G_iso Kriterium (strikt):
  iso_all_passed = !iso_reports.is_empty() AND all(r.passed for r in iso_reports)
  r.passed = operator_path_compatible AND gate_order_preserved
             AND trace_dependency_preserved AND replay_dependency_preserved
  isomorphism_score ist ein berechneter Wert — er ueberschreibt NICHT das passed-Feld!
  Leere iso_reports Liste -> G_iso=false (unconditional)

G_gap Kriterium:
  SpectralGapStitchReport.improved_or_preserved = (delta_gap >= 0)
  delta_gap = post_gap - prior_gap
  Wenn post_gap < prior_gap -> delta_gap < 0 -> improved_or_preserved=false -> G_gap=false

MetatronOperator ist KEIN Controller:
  Deterministisch, content-addressed, replayable projection
  Ableitung aus: persistent field tensor, MacroControlState, Trace, Conformance
  NICHT aus: Resonanz-Feld, Ephemeral Fabric, Agent-State

Gate-Report:
{gate_info}
G_iso-Regel: {g_iso_rule[:200]}

{desc}

Prioritaetsregeln:
  HOCHPRIORITAT: Tags [diagnostic_outcome, g_gap_fail, g_iso_pass, g_iso_fail, causal,
                       operator_compatible, gate_order_preserved, trace_dep_preserved,
                       replay_dep_preserved, spec_outcome, recovery_action, diagnostic, inspect_path]
  UNTERDRUECKEN: Tags [closed_outcome, rejected_outcome, wrong_outcome, impossible_here,
                       preflight_only, spec_violation, masks_problem, wrong_source, wrong_gate,
                       empty_iso_list, score_not_gating, wrong_criterion, wrong_artifact, distractor]

== AUFGABE ==
{case['title']}

== KANDIDATEN ==
{items}

== PFLICHTFORMAT ==
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"], "rejected": ["<id4>", ...]}}"""


# ─── Phase Matrix: PHASEMATRIX-HIVEMIND-03 ────────────────────────────────────

def build_raw_prompt_phase_matrix(case):
    ctx = case.get("phase_matrix_context", {})
    gate = ctx.get("gate_report", {})
    inv1 = ctx.get("invariant1_rule", "")
    gate_info = ("Gate-Report: " + ", ".join(f"{k}={v}" for k, v in gate.items())) if gate else ""
    inv_info = f"Invariant-1-Regel: {inv1[:160]}" if inv1 else ""
    context_block = "\n".join(x for x in [gate_info, inv_info] if x)
    items = "\n".join(
        f"  [{c['id']}] {c['source']}: {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du bist ein Phase-Matrix-Analyst (PHASEMATRIX-HIVEMIND-03).

AUFGABE: {case['title']}

{context_block}

KANDIDATEN:
{items}

Welche 3 Kandidaten sind korrekt fuer diese Situation?
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"]}}"""


def build_pse_prompt_phase_matrix(case):
    ctx = case.get("phase_matrix_context", {})
    gate = ctx.get("gate_report", {})
    inv1 = ctx.get("invariant1_rule", "")
    desc = ctx.get("description", "")
    gate_info = "\n".join(f"  {k}: {v}" for k, v in gate.items()) if gate else ""
    items = "\n".join(
        f"  [{c['id']}] op={c['source']}\n"
        f"           tags={c['tags']}\n"
        f"           {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du operierst im PSE Phase-Matrix-Evaluierungsrahmen (PHASEMATRIX-HIVEMIND-03).

== KOGNITIONS-CONSTRAINTS (Phase Matrix — HIVEMIND-03 / 03.1) ==
StitcherGate-Formel:
  G_stitch = G_conv AND G_mci AND G_delta AND G_budget AND G_trace AND G_boundary AND G_evidence

StitchFailurePolicy Prioritaetsreihenfolge (strikt):
  (1) !g_boundary -> StitchFailurePolicy::BoundaryViolation   (hoechste Prioritaet)
  (2) !g_delta    -> StitchFailurePolicy::KeepTensorUnchanged
  (3) !g_trace    -> StitchFailurePolicy::RequireRecompute
  (4) sonst       -> StitchFailurePolicy::RejectCandidate

G_mci fail-closed: kein MirrorConsistencyReport fuer Kandidat -> G_mci=false (sofort!)

Invariant 1 (Fabric-H/Fabric-T Separation):
  Fabric-H Events DUERFEN NICHT direkt Fabric-T mutieren.
  Fabric-T aendert sich NUR via CouplingUpdate[], das eine passed StitcherGateReport referenziert.
  CouplingUpdate ohne StitcherGateReport-Referenz -> Invariant-1-Verletzung
  CouplingUpdate mit StitcherGateReport(passed=false) -> Invariant-1-Verletzung
  ResonanceFabricState schreibt direkt auf FieldTensorState -> Invariant-1-Verletzung

Dissolution-Grundsatz:
  Dissolution DARF Arbeitszustand komprimieren, MUSS ABER erhalten:
    Trace, Evidence, Gate-History
  Loeschen von Trace-Eintraegen bei Dissolution -> Spec-Verletzung!

Handoff-Regel:
  Handoff erzeugt NUR Kandidaten-Referenzen.
  Handoff ERSTELLT NIEMALS externe Commit- oder Finalisierungs-Artefakte.
  PSE-Bridge ist der EINZIGE Crystal-Commit-Pfad.

Gate-Report:
{gate_info}
Invariant-1: {inv1[:200]}

{desc}

Prioritaetsregeln:
  HOCHPRIORITAT: Tags [boundary_violation, keep_tensor_unchanged, require_recompute,
                       valid_coupling_update, invariant1_satisfied, gate_passed_ref,
                       causal, spec_outcome, recovery_action, diagnostic, inspect_path]
  UNTERDRUECKEN: Tags [wrong_policy, g_delta_policy, g_trace_policy, invariant1_violation,
                       no_gate_ref, failed_gate_ref, direct_fabric_mutation,
                       dissolution_grundsatz, trace_deletion, handoff_violation,
                       determinism_violation, masks_problem, spec_violation, distractor]

== AUFGABE ==
{case['title']}

== KANDIDATEN ==
{items}

== PFLICHTFORMAT ==
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"], "rejected": ["<id4>", ...]}}"""

# ─── Cross-Layer Integration (Phase 2) ───────────────────────────────────────

def build_raw_prompt_cross_layer(case):
    ctx = case.get("cross_layer_context", {})
    layers = ", ".join(ctx.get("layers_involved", []))
    desc = ctx.get("description", "")
    items = "\n".join(
        f"  [{c['id']}] {c['source']}: {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du bist ein PSE-Integrations-Analyst.

AUFGABE: {case['title']}

Layer-Kombination: {layers}
Kontext: {desc[:200]}

KANDIDATEN:
{items}

Welche 3 Kandidaten sind korrekt fuer diese Situation?
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"]}}"""


def build_pse_prompt_cross_layer(case):
    ctx = case.get("cross_layer_context", {})
    layers = ctx.get("layers_involved", [])
    desc = ctx.get("description", "")
    items = "\n".join(
        f"  [{c['id']}] src={c['source']}\n"
        f"           tags={c['tags']}\n"
        f"           {c['text'][:140]}"
        for c in case["candidates"]
    )
    return f"""Du operierst im PSE Cross-Layer-Evaluierungsrahmen (Phase 2 — Inter-Layer Integration).

== KOGNITIONS-CONSTRAINTS (Cross-Layer Integration) ==

ADAMANT Axiom 6.1.1 — Artifact Supremacy:
  Maschinen-lesbare Artefakte (z.B. MaterializationAudit) ueberschreiben Zusammenfassungs-Reports.
  NCTCS Hauptreport c2=true wird durch MaterializationAudit (no_direct_fabric_to_tensor_mutation=false)
  auf reached_class=C1 korrigiert. Die artefakt-korrigierte Klasse bestimmt G_nctcs.

NCTCS -> Metatron G_nctcs:
  G_nctcs erfordert: NCTCS artifact-korrigierte reached_class >= C2.
  G_nctcs=false -> G_meta=false -> MetatronClosureOutcome::Diagnostic (fail-closed).
  Hauptreport-Zusammenfassung ohne Artefakt-Abgleich ist ungueltig (Axiom 6.1.1).

Signature -> TPT-MTL AdapterGate:
  AdapterGate prueft: referenced SignatureGateReport.passed == true.
  Frontier-Mitgliedschaft eines Blueprints != SignatureGate-Bestehen.
  AdapterGate=false -> TptMtlOutcomeKind::Recalibrate (Prioritaet-Ast 3, nicht Abort!).
  Abort erfordert BoundaryGate oder ReplayGate-Fehler (Ast 1) — nicht AdapterGate.

Horizon G_dual -> Dynamics Live-Graph:
  Horizon Recondense_duality_coupling ist Voraussetzung fuer den aktuellen Live-Graph.
  Korrekte Cycle-N-Tags in Dynamics entbinden NICHT von der Horizon-Abhaengigkeit.
  Recondense_tension_damping loest duality_ok=false NICHT (falsches Sub-Ziel).
  Dynamics path_delta MUSSEN warten bis Horizon recondensation_status=done.

Phase Matrix -> Metatron G_trace:
  G_trace erfordert: ALLE CouplingUpdates im Batch haben Cycle-N StitcherGateReport-Referenzen.
  G_trace ist konjunktiv — eine DeterminismViolation schlaegt den Gate fehl (KEIN Mehrheitsvotum).
  G_trace != G_replay: G_replay prueft Replay-Identitaet, G_trace prueft Cycle-Determinismus.
  G_trace=false -> G_meta=false -> MetatronClosureOutcome::Diagnostic.

Metatron Fail-Closed Invariante:
  HolisticEigenmodeState DARF NICHT produziert werden wenn G_meta=false.
  Keine gefilterte Teilmenge, kein Status-Annotation-Workaround umgeht diese Regel.

Layer-Kombination: {", ".join(layers)}
{desc}

Prioritaetsregeln:
  HOCHPRIORITAT: Tags [causal, g_nctcs_false, artifact_corrected, g_trace_false,
                       adapter_gate_false, dynamics_deferred, diagnostic_outcome,
                       recovery_action, inspect_path, horizon_dependency,
                       cross_layer_nctcs_metatron, cross_layer_signature_tptmtl,
                       cross_layer_horizon_dynamics, cross_layer_phasematrix_metatron]
  UNTERDRUECKEN: Tags [distractor, artifact_supremacy_violation, wrong_g_nctcs,
                       closed_outcome, impossible_here, spec_violation, fail_closed_violation,
                       wrong_outcome, wrong_priority, stale_input, wrong_subvariant,
                       dependency_violation, stale_graph, g_trace_misread,
                       timestamp_manipulation, wrong_gate_reasoning, bypass]

== AUFGABE ==
{case['title']}

== KANDIDATEN ==
{items}

== PFLICHTFORMAT ==
Antworte NUR mit diesem JSON (keine Erklaerung):
{{"top3": ["<id1>", "<id2>", "<id3>"], "rejected": ["<id4>", ...]}}"""


# ─── Productive Validation Layer ─────────────────────────────────────────────
#
# Free-form generation tasks. No candidate list. Scored by required_elements
# substring matching. PSE constraints supply the formal vocabulary.

PSE_AUDIT_CONSTRAINTS = """\
AUDIT-CONSTRAINTS (strikt anwenden):

Phase Matrix — G_trace (Determinismus-Invariante):
  G_trace prueft Cycle-Determinismus: ALLE CouplingUpdates im Batch MUESSEN eine
  StitcherGateReport-Referenz aus dem AKTUELLEN Zyklus (cycle N) enthalten.
  CouplingUpdate mit cycle N-1 Referenz -> DeterminismViolation (stale).
  CouplingUpdate ohne gate_ref -> G_mci fail-closed -> sofortige Ablehnung.
  G_trace ist konjunktiv: EIN DeterminismViolation -> G_trace=false (kein Mehrheitsvotum).

NCTCS — Axiom 6.1.1 Artifact Supremacy:
  Maschinen-lesbare Artefakte (MaterializationAudit) ueberschreiben Zusammenfassungs-Reports.
  MaterializationAudit: no_direct_fabric_to_tensor_mutation=false -> C2 NICHT erreicht.
  Artifact-korrigierte reached_class: wenn c2 durch Artefakt widerlegt -> reached_class=C1.
  G_nctcs erfordert artifact-korrigierte reached_class >= C2.
  Artefakt-Korrektur hat Vorrang vor NctcsConformanceSummary (egal was Summary sagt).

NCTCS — NctcsGateOutcome is_materializing():
  Pass         -> is_materializing()=TRUE  (einziger Outcome der Tensor-Revision erlaubt)
  Hold         -> is_materializing()=false
  Refine       -> is_materializing()=false
  Reject       -> is_materializing()=false
  Quarantine   -> is_materializing()=false
  NoUpdate     -> is_materializing()=false
  HandoffReady -> is_materializing()=false
  NUR Pass erlaubt Tensor-Update. Jede andere Zuweisung ist eine Spec-Verletzung.

TPT-MTL — I-08 TopologyGuardProof Invariante:
  Jede Mesh-Mutation MUSS einen TopologyGuardProof erzeugen.
  Kein Proof -> stille Mutation -> I-08-Verletzung (sofort abgelehnt).
  Proof mit betti_shift_exceeded=true -> I-08-Verletzung.
  Proof mit pd_distance_exceeded=true -> I-08-Verletzung (W_p > theta_PD).
  W_p = Wasserstein-Persistenzdiagramm-Abstand (theta_PD ist der Schwellwert).

Cross-Layer — Metatron fail-closed Invariante:
  G_meta = G_nctcs AND G_trace AND G_replay AND G_iso AND G_gap AND G_eval AND G_drift
  G_nctcs=false -> G_meta=false -> MetatronClosureOutcome::Diagnostic (NICHT Closed).
  HolisticEigenmodeState DARF NICHT produziert werden wenn G_meta=false.
  Keine gefilterte Ausgabe umgeht diese Invariante.
"""

PSE_RECOVERY_CONSTRAINTS = """\
RECOVERY-CONSTRAINTS (inter-layer Abhaengigkeiten):

Horizon -> Dynamics (Dualitaets-Kopplung):
  Horizon Recondense_duality_coupling ist Voraussetzung fuer den Dynamics Live-Graphen.
  g_dual=false -> Horizon muss Recondense (spezifisch: Recondense fuer duality) ausfuehren.
  Dynamics path_delta Berechnungen MUESSEN warten bis recondensation_status=done.
  Korrekte Cycle-N-Tags in Dynamics entbinden NICHT von der Horizon-Abhaengigkeit.
  Dynamics MorphodynamicCompressor: Merge reduziert Knotenanzahl und path_delta.

NCTCS -> Metatron (G_nctcs gate cascade):
  C2 Pflichtanforderung: no_direct_fabric_to_tensor_mutation=true.
  C2 Fehler -> G_nctcs=false -> G_meta=false -> MetatronClosureOutcome::Diagnostic.
  HolisticEigenmodeState nur wenn G_meta=true (fail-closed).
  gate cascade: C2 -> G_nctcs -> G_meta -> MetatronClosureOutcome.

Signature -> TPT-MTL (AdapterGate):
  AdapterGate prueft: referenced SignatureGateReport.passed == true.
  Frontier-Mitgliedschaft eines Blueprints != SignatureGate bestanden.
  AdapterGate=false -> TptMtlOutcomeKind::Recalibrate (Prioritaet-Ast 3).
  Abort erfordert BoundaryGate oder ReplayGate-Fehler (Ast 1) — NICHT AdapterGate.
  Remediation: Signature muss SignatureGateReport(passed=true) produzieren.

Phase Matrix -> Metatron (G_trace Konjunktion):
  G_trace ist konjunktiv: ALLE CouplingUpdates muessen current cycle Referenzen haben.
  Auch zwei stale Updates von acht -> G_trace=false (kein Mehrheitsvotum).
  G_trace=false -> G_meta=false -> MetatronClosureOutcome::Diagnostic.
  Remediation: alle stale Updates mit current cycle StitcherGateReports ersetzen.

Cognition Pipeline (Gate-Fehler-Recovery):
  G_perc Fehler     -> RefineConstraints
  G_panorama Fehler -> ExpandPanorama
  G_self Fehler     -> CalibrateOperators
  G_trigger Fehler  -> WaitForPhaseWindow
  Wormhole-Zulassung: Budget(w) <= B_max AND TTL(w) <= TTL_max AND Reason(w) in R AND ReplayTrace != 0
  Wormhole admittieren via: AdmitWormhole(wormhole, reason_code)
  Wenn Panorama nicht erweiterbar: QuerySpiralMemory fuer AttractorShortcut (counterfactual Traversierung)
"""

PSE_GATE_TRACE_CONSTRAINTS = """\
GATE-TRACE-CONSTRAINTS (deterministische Entscheidungstabellen):

TPT-MTL outcome_kind() Prioritaetsreihenfolge (strikt, branch 1 zuerst):
  branch 1: !boundary OR !replay  -> TptMtlOutcomeKind::Abort        (hoechste Prioritaet)
  branch 2: !truth                -> TptMtlOutcomeKind::Quarantine
  branch 3: !adapter OR !axis OR !micro_lift OR !carrier -> TptMtlOutcomeKind::Recalibrate
  branch 4: all_passed AND matrix AND emission           -> TptMtlOutcomeKind::Emit
  branch 5: sonst                 -> TptMtlOutcomeKind::Hold
  adapter=false triggert branch 3 -> Recalibrate (NICHT Abort — Abort nur bei boundary/replay).

StitchFailurePolicy Prioritaetsreihenfolge (strikt, branch 1 zuerst):
  branch 1: !g_boundary -> StitchFailurePolicy::BoundaryViolation   (hoechste Prioritaet)
  branch 2: !g_delta    -> StitchFailurePolicy::KeepTensorUnchanged
  branch 3: !g_trace    -> StitchFailurePolicy::RequireRecompute
  branch 4: sonst       -> StitchFailurePolicy::RejectCandidate
  g_delta=false triggert branch 2 -> KeepTensorUnchanged (NUR wenn g_boundary=true).

Dynamics GATE-01 (fail-closed, unconditional):
  proof=None -> GATE-01 -> Hold (unconditional).
  KEIN config-Parameter kann dieses Verhalten ueberschreiben.
  max_path_delta, min_alignment, require_energy_decrease werden NICHT geprueft wenn proof=None.
  Erst wenn proof != None: DynamicGateConfig-Pruefungen aktiv.

HorizonCrossingGate Policy-Tabelle (strikt):
  !g_visible (alle anderen true) -> WaitForHorizon   -> HorizonV3Outcome::WaitForHorizon
  !g_cone                        -> RefineProjectionCone -> HorizonV3Outcome::RefineCone
  !g_causal                      -> MigrateCarrier   -> HorizonV3Outcome::NeedsCarrierMigration
  !tension_ok ODER !attenuation_ok -> Recondense     -> HorizonV3Outcome::Recondense
  !g_dual                        -> Recondense       -> HorizonV3Outcome::Recondense
  g_causal prueft causal admissibility: die declared Carrier-Trajektorie muss mit der
    observed Trajektorie uebereinstimmen. Diskrepanz -> g_causal=false.

NCTCS reached_class + Axiom 6.1.1:
  Kumulatives Schema: C0 < C1 < C2 < C3 < C4.
  reached_class aus checks: hoechstes C_n fuer das alle C_0..C_n bestanden.
  Axiom 6.1.1 Korrektur: MaterializationAudit(no_direct_fabric_to_tensor_mutation=false)
    -> C2-Invariante verletzt -> artifact-korrigierte reached_class = C1 (unabhaengig von checks).
  Jede Klasse hat eine obligation-Ebene (C1-obligation, C2-obligation, etc.).
  G_nctcs = artifact-korrigierte reached_class >= C2.
"""


def _fmt_context(ctx):
    """Format task_context dict as indented key: value lines."""
    lines = []
    for k, v in ctx.items():
        if isinstance(v, (dict, list)):
            lines.append(f"  {k}: {json.dumps(v, ensure_ascii=False)}")
        else:
            lines.append(f"  {k}: {v}")
    return "\n".join(lines) if lines else "  (leer)"


def _fmt_events(events):
    if not events:
        return "  (keine Ereignisse)"
    return "\n".join(
        f"  [{e.get('timestamp_hint','?')}] {e.get('source','?')}: {e.get('message','')}"
        for e in events
    )


def build_raw_prompt_productive(case):
    ctx_str = _fmt_context(case.get("task_context", {}))
    evt_str = _fmt_events(case.get("events", []))
    return f"""You are a PSE system analyst.

TASK TYPE: {case.get('task_type', 'audit')}
TASK: {case['title']}

{case.get('task_description', '')}

CONTEXT:
{ctx_str}

EVENTS:
{evt_str}

Provide a detailed analysis. Use complete sentences. Name all relevant items by their IDs.
"""


def build_pse_prompt_productive(case):
    task_type = case.get("task_type", "audit")
    if task_type == "audit":
        constraints = PSE_AUDIT_CONSTRAINTS
    elif task_type == "recovery_plan":
        constraints = PSE_RECOVERY_CONSTRAINTS
    else:
        constraints = PSE_GATE_TRACE_CONSTRAINTS

    ctx_str = _fmt_context(case.get("task_context", {}))
    evt_str = _fmt_events(case.get("events", []))
    return f"""Du operierst im PSE Produktiv-Validierungsrahmen.

AUFGABEN-TYP: {task_type}

== KOGNITIONS-CONSTRAINTS ==
{constraints}

== AUFGABE ==
{case['title']}

{case.get('task_description', '')}

== KONTEXT ==
{ctx_str}

== EREIGNISSE ==
{evt_str}

Respond in English. Use the exact English PSE technical terms from the constraints verbatim
(e.g. G_trace, DeterminismViolation, Recondense, recondensation_status, MigrateCarrier,
NeedsCarrierMigration, KeepTensorUnchanged, Axiom 6.1.1, MetatronClosureOutcome, etc.).
Write in complete sentences. Name all relevant items by their exact IDs.
"""


PRODUCTIVE_SCHEMAS = {"v1_productive_task_fixture"}


def run_case_productive(case, api_key, case_num, total_cases):
    trace_id = case["trace_id"]
    required = case["ground_truth"]["required_elements"]
    gt_label = case.get("ground_truth_label", "Gesuchte Elemente")

    sep = "=" * 62
    print(f"\n{sep}")
    print(f"  CASE {case_num}/{total_cases}: {trace_id}")
    print(f"  {case['title']}")
    print(f"  {gt_label}")
    print(f"  Erforderliche Elemente ({len(required)}): {required}")
    print(sep)

    # --- Raw ---
    print("\n  [1/2] Raw LLM (kein PSE)...")
    raw_text = call_groq(api_key, build_raw_prompt_productive(case), max_tokens=1024)
    raw_err = None
    if raw_text.startswith("HTTP_ERROR") or raw_text.startswith("ERROR"):
        raw_err = raw_text
        raw_hits = 0
    else:
        raw_lower = raw_text.lower()
        raw_hits = sum(1 for e in required if e.lower() in raw_lower)
        found = [e for e in required if e.lower() in raw_lower]
        missing = [e for e in required if e.lower() not in raw_lower]
        print(f"       Hits: {raw_hits}/{len(required)}")
        if found:
            print(f"       Gefunden: {found}")
        if missing:
            print(f"       Fehlend:  {missing}")

    if raw_err:
        print(f"       FEHLER: {raw_err}")

    # --- PSE ---
    print("\n  [2/2] PSE-Rahmen aktiv...")
    pse_text = call_groq(api_key, build_pse_prompt_productive(case), max_tokens=1024)
    pse_err = None
    if pse_text.startswith("HTTP_ERROR") or pse_text.startswith("ERROR"):
        pse_err = pse_text
        pse_hits = 0
    else:
        pse_lower = pse_text.lower()
        pse_hits = sum(1 for e in required if e.lower() in pse_lower)
        found = [e for e in required if e.lower() in pse_lower]
        missing = [e for e in required if e.lower() not in pse_lower]
        print(f"       Hits: {pse_hits}/{len(required)}")
        if found:
            print(f"       Gefunden: {found}")
        if missing:
            print(f"       Fehlend:  {missing}")

    if pse_err:
        print(f"       FEHLER: {pse_err}")

    # --- Vergleich ---
    print()
    if raw_err or pse_err:
        verdict = "FEHLER — kein Vergleich moeglich"
        symbol = "?"
    elif pse_hits > raw_hits:
        verdict = "PSE GEWINNT — mehr erforderliche Elemente gefunden"
        symbol = "+"
    elif pse_hits == raw_hits and pse_hits == len(required):
        verdict = "BEIDE KORREKT — alle Elemente gefunden"
        symbol = "="
    elif pse_hits == raw_hits:
        verdict = "GLEICH — kein messbarer Unterschied"
        symbol = "~"
    else:
        verdict = "RAW GEWINNT — PSE-Rahmen hat nicht geholfen (Diagnose!)"
        symbol = "-"

    print(f"  [{symbol}] {verdict}")

    raw_prec = round(raw_hits / len(required), 3) if required else 0.0
    pse_prec = round(pse_hits / len(required), 3) if required else 0.0

    return {
        "trace_id": trace_id,
        "ground_truth": required,
        "raw_llm": {
            "response_preview": raw_text[:300] if not raw_err else "",
            "hits": raw_hits,
            "precision": raw_prec,
            "error": raw_err,
        },
        "pse_exoskeleton": {
            "response_preview": pse_text[:300] if not pse_err else "",
            "hits": pse_hits,
            "precision": pse_prec,
            "error": pse_err,
        },
        "verdict": verdict,
    }


# ─── Schema-Dispatch ─────────────────────────────────────────────────────────

PROMPT_BUILDERS = {
    "v1_external_trace_fixture_scaffold": (build_raw_prompt, build_pse_prompt),
    "v1_graph_relevance_fixture":         (build_raw_prompt_topology, build_pse_prompt_topology),
    "v1_pattern_retrieval_fixture":       (build_raw_prompt_memory, build_pse_prompt_memory),
    "v1_scheduling_decision_fixture":     (build_raw_prompt_scheduling, build_pse_prompt_scheduling),
    "v1_macro_step_fixture":              (build_raw_prompt_macro_step, build_pse_prompt_macro_step),
    "v1_cognition_fixture":               (build_raw_prompt_cognition, build_pse_prompt_cognition),
    "v1_horizon_fixture":                 (build_raw_prompt_horizon, build_pse_prompt_horizon),
    "v1_dynamics_fixture":                (build_raw_prompt_dynamics, build_pse_prompt_dynamics),
    "v1_signature_fixture":               (build_raw_prompt_signature, build_pse_prompt_signature),
    "v1_tpt_mtl_fixture":                 (build_raw_prompt_tpt_mtl, build_pse_prompt_tpt_mtl),
    "v1_nctcs_fixture":                   (build_raw_prompt_nctcs, build_pse_prompt_nctcs),
    "v1_metatron_fixture":                (build_raw_prompt_metatron, build_pse_prompt_metatron),
    "v1_phase_matrix_fixture":             (build_raw_prompt_phase_matrix, build_pse_prompt_phase_matrix),
    "v1_cross_layer_fixture":              (build_raw_prompt_cross_layer, build_pse_prompt_cross_layer),
}

SCHEMA_LABELS = {
    "v1_external_trace_fixture_scaffold": "Layer 1 — Agent Exoskeleton",
    "v1_graph_relevance_fixture":         "Layer 2 — Topologie / Graph",
    "v1_pattern_retrieval_fixture":       "Layer 3 — Memory / Evidence",
    "v1_scheduling_decision_fixture":     "Layer 4 — Scheduling",
    "v1_macro_step_fixture":              "Layer 5 — Core Engine macro_step()",
    "v1_cognition_fixture":               "Cognition — PSE-TRAVERSE-COGNITION-01",
    "v1_horizon_fixture":                 "Horizon — PSE-TRAVERSE-HORIZON-03",
    "v1_dynamics_fixture":                "Dynamics — PSE-TRAVERSE-DYNAMICS-01",
    "v1_signature_fixture":               "Signature — PSE-TRAVERSE-SIGNATURE-01",
    "v1_tpt_mtl_fixture":                 "Topology — PSE-TRAVERSE-TPT-MTL-04",
    "v1_nctcs_fixture":                   "NCTCS — PSE-NCTCS-CONFORMANCE-01",
    "v1_metatron_fixture":                "Metatron — PSE-METATRON-MONOLITH-01",
    "v1_phase_matrix_fixture":             "Phase Matrix — PHASEMATRIX-HIVEMIND-03",
    "v1_cross_layer_fixture":              "Cross-Layer — Integration Phase 2",
    "v1_productive_task_fixture":          "Productive — Free-Form Generation",
}


# ─── Ergebnis auswerten ───────────────────────────────────────────────────────

def parse_response(text):
    if text.startswith("HTTP_ERROR") or text.startswith("ERROR"):
        return [], [], text
    try:
        start = text.find("{")
        end = text.rfind("}") + 1
        if start >= 0 and end > start:
            data = json.loads(text[start:end])
            top3 = [str(x) for x in data.get("top3", [])]
            rejected = [str(x) for x in data.get("rejected", [])]
            return top3, rejected, None
    except Exception:
        pass
    return [], [], f"Parse-Fehler: {text[:100]}"


def score(top3, ground_truth_ids):
    hits = sum(1 for i in top3 if i in ground_truth_ids)
    precision = hits / len(top3) if top3 else 0.0
    return hits, precision


# ─── Haupt-Loop ───────────────────────────────────────────────────────────────

def run_case(case, api_key, case_num, total_cases, raw_fn, pse_fn):
    trace_id = case["trace_id"]
    ground_truth = case["ground_truth"]["causal_files"]
    gt_label = case.get("ground_truth_label", "Gesuchte Items")

    sep = "=" * 62
    print(f"\n{sep}")
    print(f"  CASE {case_num}/{total_cases}: {trace_id}")
    print(f"  {case['title']}")
    print(f"  {gt_label}: {ground_truth}")
    print(sep)

    gt_count = len(ground_truth)

    # --- Raw ---
    print("\n  [1/2] Raw LLM (kein PSE)...")
    raw_text = call_groq(api_key, raw_fn(case))
    raw_top3, _, raw_err = parse_response(raw_text)
    raw_hits, raw_prec = score(raw_top3, ground_truth)

    if raw_err:
        print(f"       FEHLER: {raw_err}")
    else:
        print(f"       Top-3:     {raw_top3}")
        print(f"       Recall:    {raw_hits}/{gt_count}   Precision: {raw_hits}/{len(raw_top3) or 1} ({raw_prec:.0%})")

    # --- PSE ---
    print("\n  [2/2] PSE-Rahmen aktiv...")
    pse_text = call_groq(api_key, pse_fn(case))
    pse_top3, pse_rejected, pse_err = parse_response(pse_text)
    pse_hits, pse_prec = score(pse_top3, ground_truth)

    if pse_err:
        print(f"       FEHLER: {pse_err}")
    else:
        print(f"       Top-3:     {pse_top3}")
        print(f"       Abgelehnt: {pse_rejected}")
        print(f"       Recall:    {pse_hits}/{gt_count}   Precision: {pse_hits}/{len(pse_top3) or 1} ({pse_prec:.0%})")

    # --- Vergleich ---
    print()
    if raw_err or pse_err:
        verdict = "FEHLER — kein Vergleich moeglich"
        symbol = "?"
    elif pse_hits > raw_hits:
        verdict = "PSE GEWINNT  -- mehr kausale Treffer (Recall)"
        symbol = "+"
    elif pse_hits == raw_hits and pse_prec > raw_prec:
        verdict = "PSE PRAEZISER -- gleicher Recall, weniger Distractors in Top-3"
        symbol = "~+"
    elif pse_hits == raw_hits and pse_hits == gt_count:
        verdict = "BEIDE KORREKT -- PSE haelt Qualitaet"
        symbol = "="
    elif pse_hits == raw_hits:
        verdict = "GLEICH -- kein messbarer Unterschied"
        symbol = "~"
    else:
        verdict = "RAW GEWINNT  -- PSE-Rahmen hat nicht geholfen (Diagnose!)"
        symbol = "-"

    print(f"  [{symbol}] {verdict}")

    return {
        "trace_id": trace_id,
        "ground_truth": ground_truth,
        "raw_llm": {"top3": raw_top3, "hits": raw_hits, "precision": round(raw_prec, 3), "error": raw_err},
        "pse_exoskeleton": {
            "top3": pse_top3,
            "rejected": pse_rejected,
            "hits": pse_hits,
            "precision": round(pse_prec, 3),
            "error": pse_err,
        },
        "verdict": verdict,
    }


def main():
    fixture_path = sys.argv[1] if len(sys.argv) > 1 else FIXTURE_PATH_DEFAULT

    if not os.path.exists(fixture_path):
        print(f"\nFEHLER: Fixture nicht gefunden unter:\n  {fixture_path}")
        print("\nVerfuegbare Fixtures:")
        for root, _, files in os.walk("crates/pse-eval-matrix/fixtures"):
            for f in files:
                if f.endswith(".json"):
                    print(f"  {os.path.join(root, f)}")
        sys.exit(1)

    with open(fixture_path, encoding="utf-8") as f:
        fixture = json.load(f)

    schema = fixture.get("fixture_schema_version", "v1_external_trace_fixture_scaffold")
    if schema not in PROMPT_BUILDERS:
        print(f"WARNUNG: Unbekanntes Schema '{schema}' -- Fallback auf Layer-1-Prompts")
        schema = "v1_external_trace_fixture_scaffold"
    raw_fn, pse_fn = PROMPT_BUILDERS[schema]

    cases = fixture["cases"]
    print(f"\nPSE STACK VALIDATION -- LIVE GROQ TEST")
    print(f"Fixture:  {fixture['fixture_name']}")
    print(f"Schema:   {SCHEMA_LABELS.get(schema, schema)}")
    print(f"Cases:    {len(cases)}")
    print(f"Modell:   {GROQ_MODEL}")

    api_key = get_api_key()

    results = []
    for i, case in enumerate(cases, 1):
        result = run_case(case, api_key, i, len(cases), raw_fn, pse_fn)
        results.append(result)

    valid = [r for r in results if not r["raw_llm"]["error"]]
    pse_recall_wins = sum(1 for r in valid if r["pse_exoskeleton"]["hits"] > r["raw_llm"]["hits"])
    pse_prec_wins   = sum(1 for r in valid if r["pse_exoskeleton"]["hits"] == r["raw_llm"]["hits"]
                          and r["pse_exoskeleton"]["precision"] > r["raw_llm"]["precision"])
    ties            = sum(1 for r in valid if r["pse_exoskeleton"]["hits"] == r["raw_llm"]["hits"]
                          and r["pse_exoskeleton"]["precision"] == r["raw_llm"]["precision"])
    raw_wins        = sum(1 for r in valid if r["pse_exoskeleton"]["hits"] < r["raw_llm"]["hits"])

    print(f"\n{'=' * 62}")
    print(f"  GESAMTERGEBNIS ({len(valid)} Cases ausgewertet)")
    print(f"  PSE Recall-Sieg:    {pse_recall_wins}")
    print(f"  PSE Precision-Sieg: {pse_prec_wins}")
    print(f"  Unentschieden:      {ties}")
    print(f"  Raw gewinnt:        {raw_wins}")
    if valid:
        pse_total    = sum(r["pse_exoskeleton"]["hits"] for r in valid)
        raw_total    = sum(r["raw_llm"]["hits"] for r in valid)
        gt_total     = sum(len(r["ground_truth"]) for r in valid)
        pse_prec_avg = sum(r["pse_exoskeleton"]["precision"] for r in valid) / len(valid)
        raw_prec_avg = sum(r["raw_llm"]["precision"] for r in valid) / len(valid)
        print(f"  PSE Recall:         {pse_total}/{gt_total}")
        print(f"  Raw Recall:         {raw_total}/{gt_total}")
        print(f"  PSE Precision avg:  {pse_prec_avg:.0%}")
        print(f"  Raw Precision avg:  {raw_prec_avg:.0%}")
    print(f"{'=' * 62}")

    os.makedirs("target/tmp", exist_ok=True)
    output = {
        "fixture": fixture["fixture_name"],
        "schema": schema,
        "model": GROQ_MODEL,
        "diagnostic_only": True,
        "productive_agent_validated": False,
        "results": results,
        "summary": {
            "total_cases": len(cases),
            "valid_cases": len(valid),
            "pse_recall_wins": pse_recall_wins,
            "pse_precision_wins": pse_prec_wins,
            "ties": ties,
            "raw_wins": raw_wins,
        },
    }
    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)
    print(f"\n  Gespeichert: {OUTPUT_PATH}")
    print()


if __name__ == "__main__":
    main()
