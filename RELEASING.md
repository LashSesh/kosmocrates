# Releasing Kosmocrates

This document is the source of truth for cutting a Kosmocrates release.
It covers versioning, tagging, the automated GitHub Actions pipeline,
SBOM and binary artifacts, the Docker image, and the (currently gated)
crates.io publication path.

> Audience: maintainers. Downstream consumers do not need to read this
> file — they should see [README.md](README.md) and the GitHub Releases
> page.

---

## 1. Versioning policy

Kosmocrates follows [Semantic Versioning](https://semver.org/) with the
pre-1.0 caveat: while we remain on `0.x`, **minor bumps may contain
breaking changes**. Once we ship `1.0.0`, semver is enforced strictly
across every published crate.

* `0.x.y` — pre-stable. Public API may shift between minor releases.
  Each minor bump documents breaking changes in `CHANGELOG.md`.
* `1.0.0` and beyond — strict semver. `cargo semver-checks` is gated
  in CI (planned, see [§7](#7-roadmap-to-10)).
* Pre-release suffixes (`-rc.1`, `-beta.2`, `-alpha.3`) are supported by
  the release workflow and produce GitHub pre-releases.

The single workspace version in `Cargo.toml` (`[workspace.package].version`)
is the source of truth — every member crate inherits it via
`version.workspace = true`. We do not maintain per-crate versions yet.

### MSRV

The minimum supported Rust version is declared as
`rust-version = "1.82"` in the workspace manifest. Bumping MSRV is a
breaking change and requires a minor (pre-1.0) or major (post-1.0)
bump plus a `CHANGELOG.md` entry.

---

## 2. Release artifacts

Every tagged release produces:

| Artifact | Platforms | Source |
|---|---|---|
| Compressed archives with `pse`, `nxalien`, `pse-server`, `pse-demo`, `pse-llm-demo` binaries | linux-x86_64, linux-aarch64, macos-x86_64, macos-aarch64, windows-x86_64 | `.github/workflows/release.yml` |
| `*.sha256` checksums per archive | all | release workflow |
| `kosmocrates-sbom.tar.gz` — CycloneDX SBOMs for the workspace | n/a | release workflow |
| `ghcr.io/lashsesh/kosmocrates/pse-server:vX.Y.Z` + `:latest` | linux/amd64, linux/arm64 | `.github/workflows/docker.yml` |

The release pipeline does **not** publish to `crates.io` yet — see
[§5](#5-publishing-to-cratesio).

---

## 3. Pre-release checklist

Before tagging:

- [ ] `main` is green (CI: fmt, clippy, build×3, test, doc, audit).
- [ ] `cargo test --workspace --locked --all-targets` passes locally.
- [ ] `cargo doc --workspace --no-deps` builds without warnings.
- [ ] `CHANGELOG.md` has a section for the new version, dated, with
      breaking changes called out under a `### Breaking changes`
      subsection if applicable.
- [ ] `README.md` quickstart commands still work (build + first run).
- [ ] The workspace version in `Cargo.toml` matches the tag you are
      about to push (without the leading `v`).
- [ ] If MSRV changed, `rust-version` was updated and `.github/workflows/ci.yml`
      reflects the new floor.

---

## 4. Cutting a release

Releases are tag-driven. Pushing a tag matching `v[0-9]+.[0-9]+.[0-9]+`
(optionally with a `-rc.N`/`-beta.N`/`-alpha.N` suffix) triggers
`release.yml`, which builds artifacts and creates a **draft** GitHub
release. The draft must be reviewed and published manually.

```bash
# 1. Bump the workspace version (single source of truth).
#    Edit Cargo.toml: [workspace.package].version = "0.2.0"
$EDITOR Cargo.toml

# 2. Update CHANGELOG.md with the new section.
$EDITOR CHANGELOG.md

# 3. Commit.
git add Cargo.toml CHANGELOG.md
git commit -m "release: v0.2.0"

# 4. Tag and push.
git tag -a v0.2.0 -m "Kosmocrates v0.2.0"
git push origin main
git push origin v0.2.0
```

The release workflow will:

1. Create a draft release from the tag, with the matching `CHANGELOG.md`
   section as the body.
2. Build the five end-user binaries on five target triples.
3. Generate SHA-256 checksums and a CycloneDX SBOM.
4. Attach everything to the draft release.

In parallel, `docker.yml` builds and pushes the multi-arch
`pse-server` image to GHCR, tagged `vX.Y.Z`, `X.Y`, and `latest`.

When all jobs are green, edit the draft release on GitHub, verify
the asset list, and click **Publish release**.

### Dry-run

To validate the binary build matrix without cutting a release, trigger
`release.yml` manually from the Actions UI (`workflow_dispatch`). The
job builds artifacts and uploads them as workflow artifacts (7-day
retention) instead of attaching them to a GitHub release.

---

## 5. Publishing to crates.io

**Status: not yet enabled.** The `publish-crates` job in `release.yml`
is currently gated by `if: false`.

Two prerequisites must be met before we flip that gate:

1. **Every publishable crate carries complete metadata.** That means
   every member crate of the workspace either:
   * declares `description`, `license`, `repository`, and `readme`
     (most are already inherited via `workspace.package`), **or**
   * sets `publish = false` if it is a binary tool, example, or
     research artifact that should not appear on crates.io.

   Suggested split:

   | Publish | Do not publish |
   |---|---|
   | All `crates/*` libraries | All `tools/*` binaries (CLIs, benches, demos) |
   | All `adapters/*` libraries | Adapter examples / fixtures |

2. **Every path dependency between workspace members carries a `version`
   field.** crates.io rejects publishes where a path-only dependency
   would dangle for downstream consumers. Convert
   ```toml
   pse-core = { path = "../pse-core" }
   ```
   to
   ```toml
   pse-core = { path = "../pse-core", version = "0.1" }
   ```
   workspace-wide. A scripted sweep is the right move here.

Once both prerequisites are satisfied, the publish job needs:

* A `CRATES_IO_TOKEN` repository secret.
* A topologically sorted publish order. The workspace has a deep
  dependency graph; the first cuts should publish leaf crates
  (`pse-types`, `pse-nxalien-types`) before any consumer.
  `cargo-workspaces` or `cargo-release` can compute this order.

Recommended tooling when we get here:

```bash
cargo install --locked cargo-workspaces
cargo workspaces publish --from-git --yes
```

Until then, the artifacts in §2 are the supported distribution channel.

---

## 6. Hotfix releases

For a `0.x.y` hotfix on top of an already-shipped `0.x.y-1`:

1. Branch from the previous release tag: `git checkout -b hotfix/v0.x.y v0.x.(y-1)`.
2. Cherry-pick the fix(es) from `main`.
3. Bump the patch version in `Cargo.toml`, update `CHANGELOG.md`.
4. Tag `v0.x.y` on the hotfix branch and push the tag.
5. Merge the hotfix branch back into `main` (do not force-push tags).

---

## 7. Roadmap to 1.0

In rough order of dependency:

1. Complete the publish-readiness sweep described in [§5](#5-publishing-to-cratesio).
2. Add `cargo-semver-checks` to CI as a required check on pull requests.
3. Add `cargo-deny` (licenses, banned crates, advisories) as a required
   CI check, replacing the non-blocking `cargo audit` job.
4. Add `#![deny(missing_docs)]` to the core public-API crates
   (`pse-core`, `pse-types`, `pse`).
5. Cut `1.0.0` once all of the above are green for a full minor cycle
   without regression.
