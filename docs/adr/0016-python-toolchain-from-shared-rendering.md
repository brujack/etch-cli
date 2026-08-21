# ADR-0016: Python Toolchain from the Shared Pinned Rendering

**Date:** 2026-08-21
**Status:** Accepted

## Context

etch-cli is 190 `.rs` against 6 `.py`, but the Python is not decorative: `scripts/test_metrics.py` and `.claude/scripts/triage_log.py` run in CI and in `make test`, and both are linted and gated.

That Python toolchain was sourced by a single hand-written line in `ci.yml`:

```yaml
run: pip install "ruff==0.16.1"
```

The same hand-pin existed in **four** repos. When the shared dev venv moved, four separate lines had to be found and updated, and nothing reported a miss — the drift surface was the _tool versions_, not the package lists. The pin was also unverified: a version string with no integrity check.

dotfiles now declares the shared venv's package set once (`pyproject.toml` + `uv.lock`) and renders `requirements-ci.txt` from it — fully pinned, hash-carrying, installable by stock pip with no `uv` on the runner.

Three alternatives were considered and rejected:

- **Keep the hand-pin.** Cheapest, and leaves the drift surface exactly as it was, with a fourth unverified copy.
- **Fetch the rendering at CI time.** dotfiles is private, so this needs a token in five repos to buy freshness nobody asked for.
- **Cross-repo writes from dotfiles.** Same token problem, plus a repo mutating four others.

A fourth question — whether the rendering should be narrowed to a CI-only subset — was raised and remains open (see Consequences).

## Decision

**Source the Python toolchain from a committed copy of dotfiles' `requirements-ci.txt`**, installed as `pip install -r requirements-ci.txt`, replacing the hand-pinned `ruff==`.

Four properties are load-bearing:

1. **The copy is byte-identical to its source.** This is what makes `diff requirements-ci.txt ~/git-repos/personal/dotfiles/requirements-ci.txt` _itself_ the staleness check. A local header — even a helpful one saying where the file came from — destroys that property, so `CLAUDE.md` carries an explicit "do not add a local header" rule.
2. **Sync is manual and periodic, and the staleness is stated rather than engineered away.** The goal is that CI and the dev venv install the same _versions_; a copy a month behind still gives reproducible CI, it is simply not synchronised.
3. **Hash enforcement is real, not incidental.** Any hash in the file puts pip in `--require-hashes` mode for the whole file. Verified by mutation rather than by reading: intact `rc=0`, all of a package's hashes corrupted `rc=1`. Note that corrupting only _one_ of a package's hashes returns `rc=0`, because pip matches against any listed hash — the obvious single-hash test concludes the opposite of the truth.
4. **A test asserts every reference names one file.** The repo names the artifact in four independent places — the CI install line, `ci.yml`'s comment, `CLAUDE.md`'s staleness command, and `README.md`'s install line. Nothing forces those strings to agree, and once install-target and diff-target drift the staleness check goes on faithfully measuring a file nobody installs, reporting clean forever. `tests/test_requirements_sync.py` collapses them to one asserted name.

**Gate Python coverage at a measured 87%,** with the same reasoning as ADR-0004 but a different justification for using a local measurement.

ADR-0061 requires a coverage floor to come from CI's own measurement, never a local one, because `dotfiles` measures 92% on macOS against 91% in CI. **That rule is not excepted here.** What was measured is that its _cause_ is absent: both covered files contain zero `sys.platform` / `platform.system()` / `darwin` / `win32` / `uname` branches, and the entire Python suite has exactly one conditional skip — `skipUnless(_HAS_ZSTD)`, gated on `compression.zstd` being 3.14+. CI pins Python 3.13, so that test skips identically on the runner and on any 3.13 interpreter. The denominator is platform-invariant by construction rather than by luck.

That justification depends on a value in another file, so it is asserted rather than assumed: a test compares `ci.yml`'s `python-version` pin against the version the argument names, and carries an instruction to delete itself if the reasoning it protects is ever removed.

## Consequences

**Easier.** A ruff, pytest, or pytest-cov version moves in one place — the lock — and reaches every consumer by copy. Every package is version-pinned and hash-verified, which the hand-pin never was. Local and CI now install the same versions, so a lint failure on one is reproducible on the other.

**Harder, and deliberately so.** etch-cli installs **80 packages to obtain three** — `ruff`, `pytest` and `pytest-cov` are its entire consumption. Measured as noise rather than assumed: the `Test` job totals 1419s, of which the old single-wheel ruff install was **4s** and the three `cargo install` steps are **431s**. The proportionality question is nonetheless real and is **open**, tracked in dotfiles, not resolved by this ADR. The set is not static — 65 at first adoption, 86 after cosmic-ray moved into the test-lint group, 80 after pylint was dropped.

**Required going forward.**

- Any future filename change to the rendering must move all four references together, or the suite goes red. That is the intent.
- A hashed requirements file **cannot be mixed with extras**: `pip install -r <hashed> extra-pkg` fails `--require-hashes`. A second dependency needs its own `pip install` line.
- The coverage justification has an **expiry**: if a platform-conditional branch ever enters `scripts/` or `.claude/scripts/`, it lapses and the figure must come from CI output.
- One corollary is worth stating because it is uncomfortable and easy to lose: the zstd error-path test has **never executed in CI**, under either runner, by design. That is deliberate and documented in its own docstring — but "our CI covers this error path" is not a sentence anyone should write about it.

**Licence exposure is now gated rather than assumed.** Adoption was blocked by a `dependency-review` HOLD: `pylint` is `GPL-2.0-or-later`, carried only in PEP 639's `License-Expression` field — its legacy `License` field is `None` and its classifiers are empty, so a conventional check reports it clean. It was invoked by nothing in any repo containing Python, and ruff is the fleet standard, so it was removed at the root in dotfiles#233 rather than waived here with an allow-list entry.

## Related

- [ADR-0004](0004-ci-coverage-floor.md) — Rust coverage floor exception; same shape, different language
- ai-config ADR-0058 — shared ruff configuration
- ai-config ADR-0061 — a coverage floor comes from CI's measurement, never a local one
- dotfiles#226, #228 — declare the venv package set once, render from the lock
- dotfiles#231 — cosmic-ray belongs in test-lint, not runtime
- dotfiles#232 — render `requirements-runtime-ci.txt` alongside the test-lint one
- dotfiles#233 — drop pylint, GPL-2.0-or-later and invoked by nothing
- etch-cli#126 — this change
