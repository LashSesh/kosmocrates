#!/usr/bin/env python3
"""
PSE Stack Validation Runner -- Live-Test mit Groq API.

Unterstuetzt alle fuenf Fixture-Schemas:
  v1_external_trace_fixture_scaffold  -- Layer 1: Agent Exoskeleton (Datei-Relevanz)
  v1_graph_relevance_fixture          -- Layer 2: Topologie / Graph-Navigation
  v1_pattern_retrieval_fixture        -- Layer 3: Memory / Evidence-Retrieval
  v1_scheduling_decision_fixture      -- Layer 4: Scheduling-Entscheidungen
  v1_macro_step_fixture               -- Layer 5: Core Engine macro_step() Orchestration

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

# ─── API-Key laden ────────────────────────────────────────────────────────────

def get_api_key():
    key = os.environ.get("GROQ_API_KEY", "").strip()
    if not key:
        print("Groq API-Key eingeben (beginnt mit gsk_...):")
        key = input("  > ").strip()
    if not key.startswith("gsk_"):
        print("FEHLER: Groq API-Key muss mit 'gsk_' beginnen.")
        sys.exit(1)
    return key


# ─── Groq aufrufen ───────────────────────────────────────────────────────────

def call_groq(api_key, prompt, _retry=True):
    body = json.dumps({
        "model": GROQ_MODEL,
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.0,
        "max_tokens": 512,
    }).encode("utf-8")

    req = urllib.request.Request(
        GROQ_URL,
        data=body,
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}",
            "User-Agent": "groq-python/0.11.0",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read())
            return data["choices"][0]["message"]["content"].strip()
    except urllib.error.HTTPError as e:
        error_body = e.read().decode("utf-8", errors="replace")
        if e.code == 429 and _retry:
            print("       Rate-Limit (429) -- warte 10s...")
            time.sleep(10)
            return call_groq(api_key, prompt, _retry=False)
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


# ─── Schema-Dispatch ─────────────────────────────────────────────────────────

PROMPT_BUILDERS = {
    "v1_external_trace_fixture_scaffold": (build_raw_prompt, build_pse_prompt),
    "v1_graph_relevance_fixture":         (build_raw_prompt_topology, build_pse_prompt_topology),
    "v1_pattern_retrieval_fixture":       (build_raw_prompt_memory, build_pse_prompt_memory),
    "v1_scheduling_decision_fixture":     (build_raw_prompt_scheduling, build_pse_prompt_scheduling),
    "v1_macro_step_fixture":              (build_raw_prompt_macro_step, build_pse_prompt_macro_step),
}

SCHEMA_LABELS = {
    "v1_external_trace_fixture_scaffold": "Layer 1 — Agent Exoskeleton",
    "v1_graph_relevance_fixture":         "Layer 2 — Topologie / Graph",
    "v1_pattern_retrieval_fixture":       "Layer 3 — Memory / Evidence",
    "v1_scheduling_decision_fixture":     "Layer 4 — Scheduling",
    "v1_macro_step_fixture":              "Layer 5 — Core Engine macro_step()",
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
