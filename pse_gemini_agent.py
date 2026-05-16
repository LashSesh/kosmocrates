#!/usr/bin/env python3
"""
PSE Agent Exoskeleton -- Erster Live-Test mit Gemini API.

Vergleicht:
  1. Raw LLM:         Gemini ohne PSE-Struktur
  2. PSE-Exoskelett:  Gemini mit PSE-Kognitionsrahmen

Fuehre aus (vom pse-Ordner):
  python pse_gemini_agent.py
"""

import json
import os
import sys
import time
import urllib.request
import urllib.error

# ─── Konfiguration ───────────────────────────────────────────────────────────

FIXTURE_PATH = (
    "crates/pse-eval-matrix/fixtures/"
    "agent_exoskeleton/example_trace_fixture_v1.json"
)
OUTPUT_PATH = "target/tmp/pse_gemini_result.json"
GEMINI_MODEL = "gemini-2.0-flash-lite"
GEMINI_URL = (
    "https://generativelanguage.googleapis.com/v1beta/models/"
    f"{GEMINI_MODEL}:generateContent"
)

# ─── API-Key laden ────────────────────────────────────────────────────────────

def get_api_key():
    key = os.environ.get("GEMINI_API_KEY", "").strip()
    if not key:
        print("Gemini API-Key eingeben (beginnt mit AIza...):")
        key = input("  > ").strip()
    if not key.startswith("AIza"):
        print("FEHLER: API-Key sieht nicht korrekt aus.")
        sys.exit(1)
    return key


# ─── Gemini aufrufen ──────────────────────────────────────────────────────────

def call_gemini(api_key, prompt, _retry=True):
    url = f"{GEMINI_URL}?key={api_key}"
    body = json.dumps({
        "contents": [{"parts": [{"text": prompt}]}],
        "generationConfig": {
            "temperature": 0.0,
            "maxOutputTokens": 256,
        },
    }).encode("utf-8")

    req = urllib.request.Request(
        url,
        data=body,
        headers={"Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read())
            return data["candidates"][0]["content"]["parts"][0]["text"].strip()
    except urllib.error.HTTPError as e:
        error_body = e.read().decode("utf-8", errors="replace")
        if e.code == 429 and _retry:
            wait = 65
            print(f"       Rate-Limit (429) -- warte {wait}s und versuche nochmal...")
            time.sleep(wait)
            return call_gemini(api_key, prompt, _retry=False)
        return f"HTTP_ERROR:{e.code}:{error_body[:200]}"
    except Exception as e:
        return f"ERROR:{e}"


# ─── Prompts bauen ────────────────────────────────────────────────────────────

def build_raw_prompt(case):
    items = "\n".join(
        f"  [{c['id']}] {c['text'][:100]}"
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
        f"           text={c['text'][:100]}"
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


# ─── Ergebnis auswerten ───────────────────────────────────────────────────────

def parse_response(text):
    """Extrahiert JSON aus der Modellantwort."""
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
    except Exception as e:
        pass
    return [], [], f"Parse-Fehler: {text[:100]}"


def score(top3, ground_truth_ids):
    return sum(1 for i in top3 if i in ground_truth_ids)


# ─── Haupt-Loop ───────────────────────────────────────────────────────────────

def run_case(case, api_key, case_num, total_cases):
    trace_id = case["trace_id"]
    ground_truth = case["ground_truth"]["causal_files"]

    sep = "=" * 62
    print(f"\n{sep}")
    print(f"  CASE {case_num}/{total_cases}: {trace_id}")
    print(f"  {case['title']}")
    print(f"  Gesuchte kausale Items: {ground_truth}")
    print(sep)

    # --- Raw ---
    print("\n  [1/2] Raw LLM (kein PSE)...")
    raw_text = call_gemini(api_key, build_raw_prompt(case))
    raw_top3, _, raw_err = parse_response(raw_text)
    raw_hits = score(raw_top3, ground_truth)

    if raw_err:
        print(f"       FEHLER: {raw_err}")
    else:
        print(f"       Top-3:  {raw_top3}")
        print(f"       Treffer: {raw_hits}/{len(ground_truth)}")

    # 7s gap keeps us well under the free-tier 10 RPM limit
    time.sleep(7)

    # --- PSE ---
    print("\n  [2/2] PSE-Exoskelett aktiv...")
    pse_text = call_gemini(api_key, build_pse_prompt(case))
    pse_top3, pse_rejected, pse_err = parse_response(pse_text)
    pse_hits = score(pse_top3, ground_truth)

    if pse_err:
        print(f"       FEHLER: {pse_err}")
    else:
        print(f"       Top-3:     {pse_top3}")
        print(f"       Abgelehnt: {pse_rejected}")
        print(f"       Treffer:   {pse_hits}/{len(ground_truth)}")

    # --- Vergleich ---
    print()
    if raw_err or pse_err:
        verdict = "FEHLER — kein Vergleich moeglich"
        symbol = "?"
    elif pse_hits > raw_hits:
        verdict = "PSE GEWINNT  -- Exoskelett verbessert das Ergebnis"
        symbol = "+"
    elif pse_hits == raw_hits and pse_hits == len(ground_truth):
        verdict = "BEIDE KORREKT -- PSE haelt Qualitaet"
        symbol = "="
    elif pse_hits == raw_hits:
        verdict = "GLEICH -- kein Unterschied (Analyse noetig)"
        symbol = "~"
    else:
        verdict = "RAW GEWINNT  -- PSE-Struktur hat geholfen (Diagnose!)"
        symbol = "-"

    print(f"  [{symbol}] {verdict}")

    return {
        "trace_id": trace_id,
        "ground_truth": ground_truth,
        "raw_llm":  {"top3": raw_top3, "hits": raw_hits, "error": raw_err},
        "pse_exoskeleton": {
            "top3": pse_top3,
            "rejected": pse_rejected,
            "hits": pse_hits,
            "error": pse_err,
        },
        "verdict": verdict,
    }


def main():
    # Pruefen ob wir im richtigen Ordner sind
    if not os.path.exists(FIXTURE_PATH):
        print(f"\nFEHLER: Fixture nicht gefunden unter:\n  {FIXTURE_PATH}")
        print("\nBitte dieses Skript vom PSE-Hauptordner aus ausfuehren:")
        print("  cd C:\\...\\pse")
        print("  python pse_gemini_agent.py")
        sys.exit(1)

    # Fixture laden
    with open(FIXTURE_PATH, encoding="utf-8") as f:
        fixture = json.load(f)

    cases = fixture["cases"]
    print(f"\nPSE AGENT EXOSKELETON -- LIVE GEMINI TEST")
    print(f"Fixture:  {fixture['fixture_name']}")
    print(f"Layer:    {fixture['intended_layer']}")
    print(f"Cases:    {len(cases)}")
    print(f"Modell:   {GEMINI_MODEL}")

    api_key = get_api_key()

    # Alle Cases durchlaufen
    results = []
    for i, case in enumerate(cases, 1):
        if i > 1:
            time.sleep(7)
        result = run_case(case, api_key, i, len(cases))
        results.append(result)

    # Gesamtauswertung
    valid = [r for r in results if not r["raw_llm"]["error"]]
    pse_wins  = sum(1 for r in valid if r["pse_exoskeleton"]["hits"] > r["raw_llm"]["hits"])
    ties      = sum(1 for r in valid if r["pse_exoskeleton"]["hits"] == r["raw_llm"]["hits"])
    raw_wins  = sum(1 for r in valid if r["pse_exoskeleton"]["hits"] < r["raw_llm"]["hits"])

    print(f"\n{'=' * 62}")
    print(f"  GESAMTERGEBNIS ({len(valid)} Cases ausgewertet)")
    print(f"  PSE gewinnt:  {pse_wins}")
    print(f"  Unentschieden: {ties}")
    print(f"  Raw gewinnt:  {raw_wins}")
    if valid:
        pse_total  = sum(r["pse_exoskeleton"]["hits"] for r in valid)
        raw_total  = sum(r["raw_llm"]["hits"] for r in valid)
        gt_total   = sum(len(r["ground_truth"]) for r in valid)
        print(f"  PSE-Trefferquote:  {pse_total}/{gt_total}")
        print(f"  Raw-Trefferquote:  {raw_total}/{gt_total}")
    print(f"{'=' * 62}")

    # Ergebnisse speichern
    os.makedirs("target/tmp", exist_ok=True)
    output = {
        "fixture": fixture["fixture_name"],
        "model": GEMINI_MODEL,
        "diagnostic_only": True,
        "productive_agent_validated": False,
        "results": results,
        "summary": {
            "total_cases": len(cases),
            "valid_cases": len(valid),
            "pse_wins": pse_wins,
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
