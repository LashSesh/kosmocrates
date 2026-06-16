# The Wish Instrument — `kosmo-run --wish`

A truth-instrument for the AI age: **an AI built this — is it genuinely there, or
does it just look done?**

Most coding tools are *hands*: they generate code fast. This is the *eye*. It
measures whether what was built genuinely realizes what you asked — structurally,
honestly, and re-checkably. It never guesses: every verdict is deterministic and
offline (no LLM, no key, no wall-clock in any id).

> New here? Run `kosmo-run --vocab` to see how to phrase a wish, and `--help`
> for every flag.

## Thirty seconds

```text
$ kosmo-run --wish "a module parser and a function parse_line" ./my-crate
Kosmocrates wish
  "a module parser and a function parse_line"
  status REALIZED ✓   met 2/2
  all wished facets are present.
```

Exit `0` when realized, `1` when not — so scripts and CI can gate on it.

## How to phrase a wish

A wish is plain prose; join facets with `and`. Each stratum is a deeper claim —
and a deeper claim is harder to fake. (`kosmo-run --vocab` prints the full set.)

| Stratum   | Question            | Form                              | Example                        |
|-----------|---------------------|-----------------------------------|--------------------------------|
| Existence | is it there?        | `a module X`, `a function f`      | `a function add`               |
| Shape     | is it formed?       | `a doc for f`, `a signature f/2`  | `a doc for add`                |
| Wiring    | right types?        | `a contract f(T..)->R`            | `a contract add(i32,i32)->i32` |
| Verified  | does it behave?     | `a behaviour f(a,b)=>r`, `a test` | `a behaviour add(2,3)=>5`      |
| Live      | does it run/serve?  | `a run f,args=>out~exp`, `a service M:/p=>code` | `a run add,2,3=>out~5` |

`a contract add(i32,i32)->i32` is *met* only against `fn add(a: i32, b: i32) -> i32` —
"exists but wrong types" is a real, distinct failure.

## Genuine, or a hologram?

A wish can read "realized" yet be hollow. The default render judges it honestly:

```text
✓ genuine — wish solid and topology dense: a cut diamond
⚠ over-fit suspect — a live claim over a sparse topology; confirm the probe is substantive, not a stub
```

A passing test is execution-earned, so the instrument never *accuses* — it flags
a *suspect* and names the real risk. Use `--flat` for just the verdict; the cube
view (with the per-stratum render and this honesty line) is the default.

Mistyped a name? It guides you: `→ did you mean Module engine? (you wished enigne)`.

## Did the change break — or fake — anything?

Snapshot, let the AI edit, re-measure:

```text
$ kosmo-run --wish W --wish-session base.json ./crate    # snapshot the baseline
# … a change touches the workspace …
$ kosmo-run --wish W --since base.json ./crate
Kosmocrates wish — delta since baseline
  + gained 1: Module beta
  ✗ regressed 1 — the change broke a met facet: Symbol f
```

A **regression** (something that worked broke) exits `2` — distinct from `1`
(still incomplete) — so CI can gate specifically on "this change broke something".

## A whole project's definition-of-done

Put many wishes in a file (one per line, `#` for comments):

```text
$ kosmo-run --wishlist project.wishes ./crate
Kosmocrates wishlist — project.wishes
  realized 7/9
  ✓ a module parser (1/1)
  ✗ a behaviour parse("a=1")=>ok (0/1) — needs evidence (a passing test/run)
  coverage: existence 6/6 · verified 1/3
```

- **`--since <reading>`** (a prior `--json` snapshot) gates a PR: exit `2` if it
  regressed any project wish — or scaffolded a hologram (a *counterfeit fix*).
- **`--scaffold`** previews the closure plan; **`--apply`** builds it and prints a
  *build account* of what it closed (and any hologram it introduced).
- **`coverage:`** tells you whether your DoD is deep enough —
  `⚠ no behaviour or run wish` means you check structure, not that it works.

## The CI contract

```sh
kosmo-run --wishlist project.wishes --since base.json ./crate
case $? in
  0) echo "all realized" ;;
  1) echo "incomplete" ;;
  2) echo "REGRESSION — a change broke the project"; exit 1 ;;
esac
```

Only the **realization status** gates. The honesty signals (genuine/suspect,
coverage, did-you-mean) **rank, never gate** — they inform; you decide.

## Polyglot

The wish prose is language-agnostic; only the observation backend differs. Point
it at a **Python** or **JavaScript** package and the same wishes work — modules
are files (`__init__.py` / `index.js` standing for their package), functions and
classes are `def`/`class` or `function`/`class`, and Python docstrings satisfy
`a doc for f`:

```text
$ kosmo-run --wish "a module greeter and a function greet and a doc for greet" ./pkg
  status REALIZED ✓   met 3/3
```

Existence and Shape are observed lexically across **seven languages** — Rust,
Python, JavaScript, Go, Java, C, and C++ — through one shared xlang extractor
(`a function Add` against a `.go` file, `a type Calculator` against a `.java`
file, all work). Python docstrings also satisfy `a doc for f`, and **Python,
JavaScript, and Go reach the Live stratum**: a `kosmo:run:` probe is *executed*
under the same sandbox as Rust (`python3` / `node` / `go run`) —

```text
# sq.py / sq.js / sq.go:  kosmo:run: 5=>out~25   (the script prints 25)
$ kosmo-run --wish "a run 5=>out~25" ./pkg   →   REALIZED ✓   (by execution)
```

The remaining strata are honest follow-ons (Verified-by-passing-test for
non-Rust; Live for C/C++/Java, which need a compile step first).

And for the *whole* polyglot tree as one shape: `kosmo-run --codematrix` pools
every source's 5-axis code-fingerprint — any language — into one **holistic
cube** (a language-blind centroid) plus a **homogeneity** scalar (how unified the
topology is): all languages compressed into a single hypercube. It also reports
the **cross-language structural agreement** — the language-*independent*
fingerprint (CROSS-007) lands the *same structure* on the *same point* across
languages (a Rust `fn` and a Python `def` of the same shape register `≡` at 1.00,
functions-vs-classes at 0.50): the same behaviour anywhere → one point.

## Principles

- **Deterministic & offline** — same inputs, same bytes; no LLM, no key.
- **It does not lie** — including about itself: a passing test is never called a
  hologram, and the tool maps its own reach honestly (`--vocab`, `--doors`).
- **Advisory by default** — it measures and ranks; it gates only on the one
  sanctioned signal (realization), and only where you ask for it.
