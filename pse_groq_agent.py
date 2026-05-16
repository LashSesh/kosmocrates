#!/usr/bin/env python3
"""
PSE Agent Exoskeleton -- Live-Test mit Groq API (kostenlos, kein Rate-Limit-Problem).

Vergleicht:
  1. Raw LLM:         Groq/Llama ohne PSE-Struktur
  2. PSE-Exoskelett:  Groq/Llama mit PSE-Kognitionsrahmen

Fuehre aus (vom pse-Ordner):
  python pse_groq_agent.py
  oder mit Key als Umgebungsvariable:
  set GROQ_API_KEY=gsk_...  && python pse_groq_agent.py
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
        "max_tokens": 256,
    }).encode("utf-8")

    req = urllib.request.Request(
        GROQ_URL,
        data=body,
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read())
            return data["choices"][0]["message"]["content"].strip()
    except urllib.error.HTTPError as e:
        error_body = e.read().decode("utf-8", errors="replace")
        if e.code == 429 and _retry:
            # Groq liefert Retry-After Header; wir warten 10s pauschal
            print("       Rate-Limit (429) -- warte 10s...")
            time.sleep(10)
            return call_groq(api_key, prompt, _retry=False)
        return f"HTTP_ERROR:{e.code}:{error_body[:300]}"
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
    raw_text = call_groq(api_key, build_raw_prompt(case))
    raw_top3, _, raw_err = parse_response(raw_text)
    raw_hits = score(raw_top3, ground_truth)

    if raw_err:
        print(f"       FEHLER: {raw_err}")
    else:
        print(f"       Top-3:  {raw_top3}")
        print(f"       Treffer: {raw_hits}/{len(ground_truth)}")

    # --- PSE ---
    print("\n  [2/2] PSE-Exoskelett aktiv...")
    pse_text = call_groq(api_key, build_pse_prompt(case))
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
        verdict = "RAW GEWINNT  -- PSE-Struktur hat nicht geholfen (Diagnose!)"
        symbol = "-"

    print(f"  [{symbol}] {verdict}")

    return {
        "trace_id": trace_id,
        "ground_truth": ground_truth,
        "raw_llm": {"top3": raw_top3, "hits": raw_hits, "error": raw_err},
        "pse_exoskeleton": {
            "top3": pse_top3,
            "rejected": pse_rejected,
            "hits": pse_hits,
            "error": pse_err,
        },
        "verdict": verdict,
    }


def main():
    if not os.path.exists(FIXTURE_PATH):
        print(f"\nFEHLER: Fixture nicht gefunden unter:\n  {FIXTURE_PATH}")
        print("\nBitte dieses Skript vom PSE-Hauptordner aus ausfuehren:")
        print("  cd C:\\...\\pse")
        print("  python pse_groq_agent.py")
        sys.exit(1)

    with open(FIXTURE_PATH, encoding="utf-8") as f:
        fixture = json.load(f)

    cases = fixture["cases"]
    print(f"\nPSE AGENT EXOSKELETON -- LIVE GROQ TEST")
    print(f"Fixture:  {fixture['fixture_name']}")
    print(f"Layer:    {fixture['intended_layer']}")
    print(f"Cases:    {len(cases)}")
    print(f"Modell:   {GROQ_MODEL}")

    api_key = get_api_key()

    results = []
    for i, case in enumerate(cases, 1):
        result = run_case(case, api_key, i, len(cases))
        results.append(result)

    # Gesamtauswertung
    valid = [r for r in results if not r["raw_llm"]["error"]]
    pse_wins = sum(1 for r in valid if r["pse_exoskeleton"]["hits"] > r["raw_llm"]["hits"])
    ties     = sum(1 for r in valid if r["pse_exoskeleton"]["hits"] == r["raw_llm"]["hits"])
    raw_wins = sum(1 for r in valid if r["pse_exoskeleton"]["hits"] < r["raw_llm"]["hits"])

    print(f"\n{'=' * 62}")
    print(f"  GESAMTERGEBNIS ({len(valid)} Cases ausgewertet)")
    print(f"  PSE gewinnt:   {pse_wins}")
    print(f"  Unentschieden: {ties}")
    print(f"  Raw gewinnt:   {raw_wins}")
    if valid:
        pse_total = sum(r["pse_exoskeleton"]["hits"] for r in valid)
        raw_total = sum(r["raw_llm"]["hits"] for r in valid)
        gt_total  = sum(len(r["ground_truth"]) for r in valid)
        print(f"  PSE-Trefferquote:  {pse_total}/{gt_total}")
        print(f"  Raw-Trefferquote:  {raw_total}/{gt_total}")
    print(f"{'=' * 62}")

    os.makedirs("target/tmp", exist_ok=True)
    output = {
        "fixture": fixture["fixture_name"],
        "model": GROQ_MODEL,
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
