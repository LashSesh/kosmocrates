#!/usr/bin/env python3
"""
PSE Full-Stack-Validation Runner

Runs all layer fixtures in canonical stack order and produces
an aggregated readiness report.

Usage:
  GROQ_API_KEY=gsk_... python pse_fullstack_runner.py
  python pse_fullstack_runner.py                          # prompts for key
  python pse_fullstack_runner.py --layers cognition horizon dynamics
  python pse_fullstack_runner.py --start-layer 8          # resume from layer N (1-based)
"""

import json
import os
import sys
import time

from pse_groq_agent import (
    TpdLimitError,
    call_groq,
    get_api_key,
    _provider,
    _inter_call_sleep,
    parse_response,
    run_case,
    run_case_productive,
    score,
    GROQ_MODEL, CEREBRAS_MODEL,
    PRODUCTIVE_SCHEMAS,
    PROMPT_BUILDERS,
    SCHEMA_LABELS,
)

def _active_model(api_key):
    _, model = _provider(api_key)
    return model

# ─── Canonical stack order ────────────────────────────────────────────────────
#
# Each entry: (layer_label, fixture_path)
# Ordered bottom-up: L1 foundation → Phase Matrix (last).

STACK = [
    (
        "L1 — Agent Exoskeleton",
        "crates/pse-eval-matrix/fixtures/agent_exoskeleton/honest_trace_fixture_v1.json",
    ),
    (
        "L2 — Topology / Graph",
        "crates/pse-eval-matrix/fixtures/topology_graph/graph_relevance_v1.json",
    ),
    (
        "L3 — Memory / Evidence",
        "crates/pse-eval-matrix/fixtures/memory_evidence/pattern_retrieval_v1.json",
    ),
    (
        "L4 — Scheduling",
        "crates/pse-eval-matrix/fixtures/scheduling/scheduling_decision_v1.json",
    ),
    (
        "L5 — Core Engine macro_step()",
        "crates/pse-eval-matrix/fixtures/core_orchestration/macro_step_v1.json",
    ),
    (
        "Cognition — PSE-TRAVERSE-COGNITION-01",
        "crates/pse-eval-matrix/fixtures/cognition/cognition_layer_v1.json",
    ),
    (
        "Horizon — PSE-TRAVERSE-HORIZON-03",
        "crates/pse-eval-matrix/fixtures/horizon/horizon_layer_v1.json",
    ),
    (
        "Dynamics — PSE-TRAVERSE-DYNAMICS-01",
        "crates/pse-eval-matrix/fixtures/dynamics/dynamics_layer_v1.json",
    ),
    (
        "Signature — PSE-TRAVERSE-SIGNATURE-01",
        "crates/pse-eval-matrix/fixtures/signature/signature_layer_v1.json",
    ),
    (
        "Topology — PSE-TRAVERSE-TPT-MTL-04",
        "crates/pse-eval-matrix/fixtures/topology_tpt_mtl/tpt_mtl_layer_v1.json",
    ),
    (
        "NCTCS — PSE-NCTCS-CONFORMANCE-01",
        "crates/pse-eval-matrix/fixtures/nctcs/nctcs_layer_v1.json",
    ),
    (
        "Metatron — PSE-METATRON-MONOLITH-01",
        "crates/pse-eval-matrix/fixtures/metatron/metatron_layer_v1.json",
    ),
    (
        "Phase Matrix — PHASEMATRIX-HIVEMIND-03",
        "crates/pse-eval-matrix/fixtures/phase_matrix/phase_matrix_layer_v1.json",
    ),
    (
        "Cross-Layer — Integration Phase 2",
        "crates/pse-eval-matrix/fixtures/cross_layer/cross_layer_v1.json",
    ),
    (
        "Productive — Free-Form Generation",
        "crates/pse-eval-matrix/fixtures/productive/productive_v1.json",
    ),
]

FULLSTACK_OUTPUT = "target/tmp/pse_fullstack_report.json"

# ─── Layer status classification ─────────────────────────────────────────────

def layer_status(pse_total, raw_total, gt_total, tie_count):
    """Classify layer readiness from aggregated case results."""
    if pse_total < gt_total:
        return "PARTIAL"          # PSE not at 100% — needs work
    if tie_count == 0:
        return "PSE_ONLY"         # PSE perfect, no transparent cases
    return "TRANSPARENT"          # PSE perfect, but Raw also correct on some


STATUS_LABEL = {
    "PSE_ONLY":    "[OK] PSE-Sieg (alle Cases)",
    "TRANSPARENT": "[OK] PSE perfekt + transparent (Haertungskandidat)",
    "PARTIAL":     "[!!] PSE nicht bei 100% -- Diagnose erforderlich",
}

# ─── Single-fixture runner ────────────────────────────────────────────────────

def run_fixture(label, path, api_key, layer_num, total_layers):
    """Run one fixture file and return per-layer result dict."""
    sep = "=" * 62
    print(f"\n{sep}")
    print(f"  LAYER {layer_num}/{total_layers}: {label}")
    print(f"  Fixture: {path}")
    print(sep)

    if not os.path.exists(path):
        print(f"  [!!] FEHLER: Fixture nicht gefunden: {path}")
        return {
            "layer": label,
            "path": path,
            "error": "fixture_not_found",
            "cases": [],
            "summary": None,
        }

    with open(path, encoding="utf-8") as f:
        fixture = json.load(f)

    schema = fixture.get("fixture_schema_version", "v1_external_trace_fixture_scaffold")
    if schema not in PROMPT_BUILDERS and schema not in PRODUCTIVE_SCHEMAS:
        print(f"  [!!] FEHLER: Unbekanntes Schema '{schema}'")
        return {
            "layer": label,
            "path": path,
            "error": f"unknown_schema:{schema}",
            "cases": [],
            "summary": None,
        }

    cases = fixture["cases"]

    case_results = []
    for i, case in enumerate(cases, 1):
        if schema in PRODUCTIVE_SCHEMAS:
            result = run_case_productive(case, api_key, i, len(cases))
        else:
            raw_fn, pse_fn = PROMPT_BUILDERS[schema]
            result = run_case(case, api_key, i, len(cases), raw_fn, pse_fn)
        case_results.append(result)
        if i < len(cases):
            time.sleep(_inter_call_sleep(api_key))

    # Aggregate layer totals
    valid = [r for r in case_results if not r["raw_llm"]["error"]]
    pse_total  = sum(r["pse_exoskeleton"]["hits"] for r in valid)
    raw_total  = sum(r["raw_llm"]["hits"] for r in valid)
    gt_total   = sum(len(r["ground_truth"]) for r in valid)
    pse_wins   = sum(1 for r in valid if r["pse_exoskeleton"]["hits"] > r["raw_llm"]["hits"])
    tie_count  = sum(
        1 for r in valid
        if r["pse_exoskeleton"]["hits"] == r["raw_llm"]["hits"]
        and r["pse_exoskeleton"]["hits"] == len(r["ground_truth"])
    )
    raw_wins   = sum(1 for r in valid if r["pse_exoskeleton"]["hits"] < r["raw_llm"]["hits"])

    transparent_cases = [
        r["trace_id"] for r in valid
        if r["pse_exoskeleton"]["hits"] == r["raw_llm"]["hits"]
        and r["pse_exoskeleton"]["hits"] == len(r["ground_truth"])
    ]

    status = layer_status(pse_total, raw_total, gt_total, tie_count)
    pse_pct = f"{pse_total}/{gt_total} ({pse_total/gt_total:.0%})" if gt_total else "–"
    raw_pct = f"{raw_total}/{gt_total} ({raw_total/gt_total:.0%})" if gt_total else "–"

    print(f"\n  ── Layer-Zusammenfassung: {label}")
    print(f"     PSE Recall: {pse_pct}  |  Raw Recall: {raw_pct}")
    print(f"     PSE-Sieger: {pse_wins}  |  Transparent: {tie_count}  |  Raw-Sieger: {raw_wins}")
    print(f"     Status: {STATUS_LABEL[status]}")
    if transparent_cases:
        print(f"     Haertungskandidaten: {transparent_cases}")

    return {
        "layer": label,
        "path": path,
        "schema": schema,
        "diagnostic_only": fixture.get("diagnostic_only", True),
        "productive_agent_validated": fixture.get("productive_agent_validated", False),
        "cases": case_results,
        "summary": {
            "total_cases": len(cases),
            "valid_cases": len(valid),
            "pse_recall": pse_total,
            "raw_recall": raw_total,
            "gt_slots": gt_total,
            "pse_wins": pse_wins,
            "ties": tie_count,
            "raw_wins": raw_wins,
            "transparent_cases": transparent_cases,
            "status": status,
        },
    }


# ─── Grand total printer ──────────────────────────────────────────────────────

def print_grand_total(layer_results):
    sep  = "=" * 62
    sep2 = "─" * 62

    valid_layers = [lr for lr in layer_results if lr.get("summary")]
    all_cases    = sum(lr["summary"]["valid_cases"] for lr in valid_layers)
    all_slots    = sum(lr["summary"]["gt_slots"]    for lr in valid_layers)
    pse_grand    = sum(lr["summary"]["pse_recall"]  for lr in valid_layers)
    raw_grand    = sum(lr["summary"]["raw_recall"]  for lr in valid_layers)
    pse_only     = sum(1 for lr in valid_layers if lr["summary"]["status"] == "PSE_ONLY")
    transparent  = sum(1 for lr in valid_layers if lr["summary"]["status"] == "TRANSPARENT")
    partial      = sum(1 for lr in valid_layers if lr["summary"]["status"] == "PARTIAL")

    all_transparent_cases = []
    for lr in valid_layers:
        for tc in lr["summary"]["transparent_cases"]:
            all_transparent_cases.append(f"{lr['layer']} / {tc}")

    print(f"\n\n{sep}")
    print(f"  FULL-STACK VALIDATION REPORT")
    print(f"  {len(valid_layers)} Layer  |  {all_cases} Cases  |  {all_slots} Evaluation-Slots")
    model_label = layer_results[0].get("model", GROQ_MODEL) if layer_results else GROQ_MODEL
    print(f"  Modell: {model_label}")
    print(sep)
    print(f"  PSE Recall (gesamt): {pse_grand}/{all_slots}  "
          f"({pse_grand/all_slots:.1%})" if all_slots else "  PSE Recall: –")
    print(f"  Raw Recall (gesamt): {raw_grand}/{all_slots}  "
          f"({raw_grand/all_slots:.1%})" if all_slots else "  Raw Recall: –")
    print(f"  PSE-Vorteil: +{pse_grand - raw_grand} Slots")
    print()
    print(f"  Layer-Status:")
    print(f"    [OK] PSE_ONLY    (keine transparenten Cases):  {pse_only}")
    print(f"    [OK] TRANSPARENT (PSE perfekt, Raw auch gut):  {transparent}")
    print(f"    [!!] PARTIAL     (PSE nicht bei 100%):         {partial}")

    print(f"\n{sep2}")
    print(f"  {'Layer':<42} {'PSE':>8} {'Raw':>8}  Status")
    print(sep2)
    for lr in valid_layers:
        s = lr["summary"]
        if not s:
            print(f"  {lr['layer']:<42} {'FEHLER':>8}")
            continue
        pse_str = f"{s['pse_recall']}/{s['gt_slots']}"
        raw_str = f"{s['raw_recall']}/{s['gt_slots']}"
        status_short = {"PSE_ONLY": "PSE_ONLY", "TRANSPARENT": "TRANSP.", "PARTIAL": "PARTIAL"}
        print(f"  {lr['layer']:<42} {pse_str:>8} {raw_str:>8}  {status_short.get(s['status'], s['status'])}")

    if all_transparent_cases:
        print(f"\n{sep2}")
        print(f"  Transparente Cases (Haertungsrunde):")
        for tc in all_transparent_cases:
            print(f"    - {tc}")

    print(f"\n{sep2}")
    diagnosed_layers = pse_only + transparent
    print(f"  Readiness: {diagnosed_layers}/{len(valid_layers)} Layer PSE-perfekt")
    if partial == 0 and transparent == 0:
        print(f"  => Stack bereit fuer Cross-Layer-Erweiterung")
    elif partial == 0 and transparent > 0:
        print(f"  => {transparent} Layer benoetigen Haertungsrunde (transparente Cases)")
    else:
        print(f"  => {partial} Layer benoetigen PSE-Diagnose")
    print(sep)


# ─── Main ─────────────────────────────────────────────────────────────────────

def _save_report(layer_results, stack_size, model=None):
    """Persist the report to disk; safe to call on partial runs."""
    os.makedirs("target/tmp", exist_ok=True)
    valid_layers = [lr for lr in layer_results if lr.get("summary")]
    all_slots = sum(lr["summary"]["gt_slots"]   for lr in valid_layers)
    pse_grand = sum(lr["summary"]["pse_recall"] for lr in valid_layers)
    raw_grand = sum(lr["summary"]["raw_recall"] for lr in valid_layers)

    report = {
        "runner": "pse_fullstack_runner",
        "model": model or GROQ_MODEL,
        "timestamp_hint": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "stack_size": stack_size,
        "layers_completed": len(layer_results),
        "grand_summary": {
            "total_cases": sum(lr["summary"]["valid_cases"] for lr in valid_layers),
            "total_slots": all_slots,
            "pse_recall": pse_grand,
            "raw_recall": raw_grand,
            "pse_recall_pct": round(pse_grand / all_slots, 4) if all_slots else 0,
            "raw_recall_pct": round(raw_grand / all_slots, 4) if all_slots else 0,
            "pse_advantage_slots": pse_grand - raw_grand,
            "layers_pse_only":    sum(1 for lr in valid_layers if lr["summary"]["status"] == "PSE_ONLY"),
            "layers_transparent": sum(1 for lr in valid_layers if lr["summary"]["status"] == "TRANSPARENT"),
            "layers_partial":     sum(1 for lr in valid_layers if lr["summary"]["status"] == "PARTIAL"),
            "all_transparent_cases": [
                f"{lr['layer']}/{tc}"
                for lr in valid_layers
                for tc in lr["summary"]["transparent_cases"]
            ],
        },
        "layers": layer_results,
    }

    with open(FULLSTACK_OUTPUT, "w", encoding="utf-8") as f:
        json.dump(report, f, ensure_ascii=False, indent=2)

    print(f"\nGespeichert: {FULLSTACK_OUTPUT}")


def main():
    # Optional: filter layers via --layers arg
    filter_keys = set()
    if "--layers" in sys.argv:
        idx = sys.argv.index("--layers")
        # Collect args until next flag (or end)
        filter_keys = {
            k.lower() for k in sys.argv[idx + 1:]
            if not k.startswith("--")
        }

    # Optional: resume from layer N (1-based)
    start_layer = 1
    if "--start-layer" in sys.argv:
        idx = sys.argv.index("--start-layer")
        try:
            start_layer = int(sys.argv[idx + 1])
        except (IndexError, ValueError):
            print("FEHLER: --start-layer braucht eine Zahl, z.B. --start-layer 8")
            sys.exit(1)

    stack = STACK
    if filter_keys:
        stack = [
            (label, path) for label, path in STACK
            if any(k in label.lower() for k in filter_keys)
        ]
        if not stack:
            print(f"Keine Layer gefunden fuer Filter: {filter_keys}")
            sys.exit(1)

    if start_layer > 1:
        if start_layer > len(stack):
            print(f"FEHLER: --start-layer {start_layer} > Stack-Groesse {len(stack)}")
            sys.exit(1)
        print(f"\n[Resume] Ueberspringe Layer 1-{start_layer - 1}, starte bei Layer {start_layer}")
        stack = stack[start_layer - 1:]

    print(f"\nPSE FULL-STACK-VALIDATION RUNNER")
    print(f"Output: {FULLSTACK_OUTPUT}")

    api_key = get_api_key()
    active_model = _active_model(api_key)
    print(f"Layers: {len(stack)}  |  Modell: {active_model}")

    # Load previous partial report so --start-layer appends instead of overwriting
    layer_results = []
    if start_layer > 1 and os.path.exists(FULLSTACK_OUTPUT):
        try:
            with open(FULLSTACK_OUTPUT, encoding="utf-8") as f:
                prev = json.load(f)
            layer_results = prev.get("layers", [])
            print(f"[Resume] {len(layer_results)} vorherige Layer aus {FULLSTACK_OUTPUT} geladen.")
        except Exception as e:
            print(f"[Warnung] Konnte vorherigen Bericht nicht laden: {e}")

    tpd_hit = False
    for i, (label, path) in enumerate(stack, start_layer):
        try:
            result = run_fixture(label, path, api_key, i, len(STACK))
        except TpdLimitError as e:
            print(f"\n[!!] TPD-Limit (Tokens-per-Day) erschoepft bei Layer {i}: {label}")
            print(f"     Fehlermeldung: {e}")
            print(f"     Bitte morgen weitermachen mit:  --start-layer {i}")
            tpd_hit = True
            break
        result["model"] = active_model
        layer_results.append(result)
        if i < len(STACK):
            time.sleep(_inter_call_sleep(api_key))

    if layer_results:
        print_grand_total(layer_results)
        _save_report(layer_results, len(STACK), model=active_model)

    if tpd_hit:
        sys.exit(2)


if __name__ == "__main__":
    main()
