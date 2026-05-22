#!/usr/bin/env python3
"""
gen_word_phases.py — Generate a PSE word-phase file from GloVe or FastText vectors.

Each word is projected to a 2D unit vector via PCA.  The resulting (x, y)
coordinates are used by pse-llm-demo to compute semantically meaningful phase
hints for each text chunk: atan2(sum_y, sum_x) over the words in the chunk.

Usage:
    python3 gen_word_phases.py <embedding_file> <output_file> [--limit N]

Arguments:
    embedding_file  GloVe text file (e.g. glove.6B.50d.txt) or FastText .vec file.
                    GloVe: "word f1 f2 ... fD" per line.
                    FastText .vec: first line is "N D", rest is GloVe format.
    output_file     TSV output: word<TAB>x<TAB>y   (one entry per line)
    --limit N       Limit to first N words (default: no limit).
                    Recommended: 50000 for a compact file, 400000 for full GloVe.

Output format:
    Lines starting with '#' are comments (skipped by the Rust loader).
    Data lines: word<TAB>x<TAB>y   where x,y are float32 on the unit circle.

Then configure the demo:
    export PSE_LLM_EMBEDDING_FILE=/path/to/output_file
    cargo run --release -p pse-llm-demo

Recommended sources:
    GloVe 6B 50d  https://nlp.stanford.edu/projects/glove/
                  (glove.6B.zip, 50d variant is ~170 MB, loads in seconds)
    FastText       https://fasttext.cc/docs/en/english-vectors.html
                  (wiki-news-300d-1M.vec.zip — 1M words, 300d)

Dependencies:
    numpy >= 1.20   (pip install numpy)
"""

import argparse
import sys
import time

try:
    import numpy as np
except ImportError:
    print("ERROR: numpy is required.  Install with: pip install numpy", file=sys.stderr)
    sys.exit(1)


# ── Loader ─────────────────────────────────────────────────────────────────────

def load_embeddings(path: str, limit: int | None) -> tuple[list[str], "np.ndarray"]:
    """Load GloVe / FastText text vectors.  Returns (words, matrix float32)."""
    words: list[str] = []
    vecs: list[list[float]] = []

    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        first = fh.readline()
        parts = first.strip().split()

        # Detect FastText header: exactly two integers on the first line.
        is_header = len(parts) == 2
        try:
            int(parts[0]); int(parts[1])
        except (ValueError, IndexError):
            is_header = False

        if not is_header:
            # GloVe — first line is a data line.
            if len(parts) >= 3:
                words.append(parts[0])
                vecs.append([float(v) for v in parts[1:]])

        for line in fh:
            if limit is not None and len(words) >= limit:
                break
            parts = line.rstrip().split(" ")
            if len(parts) < 3:
                continue
            try:
                words.append(parts[0])
                vecs.append([float(v) for v in parts[1:]])
            except ValueError:
                continue

    if not vecs:
        raise ValueError(f"No vectors loaded from {path!r}")

    return words, np.array(vecs, dtype=np.float32)


# ── PCA via covariance eigendecomposition ─────────────────────────────────────

def project_2d(X: "np.ndarray") -> "np.ndarray":
    """Project N×D matrix to N×2 using the top-2 principal components.

    Uses eigendecomposition of the D×D covariance matrix — O(D³) not O(N²),
    so it scales to millions of words regardless of vocabulary size.
    """
    X = X - X.mean(axis=0)                       # centre
    C = (X.T @ X) / X.shape[0]                   # D×D covariance
    eigenvalues, eigenvectors = np.linalg.eigh(C) # ascending order
    # Take the two largest eigenvectors (last two columns of eigh output).
    top2 = eigenvectors[:, -2:][:, ::-1]          # D×2, PC1 first
    return (X @ top2).astype(np.float32)          # N×2


def normalise_unit_circle(X2: "np.ndarray") -> "np.ndarray":
    """Normalise each row to unit length so every word contributes equally
    to the circular mean — prevents high-magnitude vectors from dominating."""
    norms = np.linalg.norm(X2, axis=1, keepdims=True).clip(min=1e-9)
    return (X2 / norms).astype(np.float32)


# ── Main ───────────────────────────────────────────────────────────────────────

def main() -> None:
    p = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p.add_argument("input", help="GloVe or FastText text embedding file")
    p.add_argument("output", help="Output TSV file (word<TAB>x<TAB>y)")
    p.add_argument(
        "--limit",
        type=int,
        default=None,
        metavar="N",
        help="Max vocabulary size to load (default: all)",
    )
    args = p.parse_args()

    t0 = time.time()
    print(f"Loading embeddings from {args.input!r} ...", file=sys.stderr)
    words, X = load_embeddings(args.input, args.limit)
    print(
        f"  Loaded {len(words):,} words, dim={X.shape[1]}  ({time.time()-t0:.1f}s)",
        file=sys.stderr,
    )

    t1 = time.time()
    print("Computing 2D PCA projection ...", file=sys.stderr)
    X2 = project_2d(X)
    X2 = normalise_unit_circle(X2)
    print(f"  Done  ({time.time()-t1:.2f}s)", file=sys.stderr)

    # Sanity check: phases should span the full circle.
    angles = np.arctan2(X2[:, 1], X2[:, 0])
    print(
        f"  Phase range: [{angles.min():.3f}, {angles.max():.3f}] rad  "
        f"(std={angles.std():.3f})",
        file=sys.stderr,
    )

    t2 = time.time()
    print(f"Writing to {args.output!r} ...", file=sys.stderr)
    with open(args.output, "w", encoding="utf-8") as fh:
        fh.write(f"# PSE word-phase file — generated from {args.input}\n")
        fh.write(f"# {len(words)} words, PCA 2D projection, unit-normalised\n")
        fh.write("# Format: word<TAB>x<TAB>y\n")
        for word, (x, y) in zip(words, X2):
            fh.write(f"{word}\t{x:.6f}\t{y:.6f}\n")
    print(
        f"  Wrote {len(words):,} entries to {args.output!r}  ({time.time()-t2:.1f}s)",
        file=sys.stderr,
    )
    print(
        f"\nDone in {time.time()-t0:.1f}s total.\n"
        f"Set: export PSE_LLM_EMBEDDING_FILE={args.output}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
