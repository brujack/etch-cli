# gix 0.80 → 0.87: Design

**Date:** 2026-08-22
**Status:** Approved, pending implementation plan

## Problem

GitHub reports **28 open Dependabot alerts on `main`, 23 high**, dominated by `gix`. Three
distinct gix advisories all name **0.83.0** as first-patched; the repo pins `0.80.0` in both
`lib/Cargo.toml` and `app/Cargo.toml`.

`cargo update` cannot reach the fix. Under 0.x semver, `0.80 → 0.83` is a breaking boundary,
so the manifests must move.

**The repo's own advisory gate reports clean on this tree.** `cargo deny check advisories`
returns `advisories ok` while all 23 alerts stand. That is not a bug in the gate: `cargo-deny`
reads the **RustSec** database and Dependabot reads the **GitHub Advisory Database**, and
these are different corpora. Verified — `GHSA-fr8x-3vfx-f45h` and `GHSA-pg4w-g64p-qwhj` carry
no RUSTSEC identifier. The gate is sound about RustSec and silent about everything else. That
finding is **out of scope here** and separately backlogged; it is recorded because it explains
why a blocking gate did not catch this.

### Threat model, stated honestly

The gix advisories are mostly `.gitmodules`/submodule issues — arbitrary command execution,
path traversal out of `.git/modules`, credential disclosure, symlink worktree escape during
checkout, and DoS from crafted pack data.

etch-cli clones repositories. An earlier framing of this work called that "cloning untrusted
input", which **overstates it**: this is proof-of-concept code, the operator authors the
manifests, and the repositories cloned are ones they chose (`ohmyzsh`, their own plugins).
Reaching the crafted-`.gitmodules` path requires deliberately cloning a hostile repository.

So the justification for this change is **cost, not urgency**: the fix is cheap and mechanical,
it clears the alert board, and it stops the dependency drifting further from upstream. It is
not an incident response.

No submodule API is called anywhere in this codebase (`git grep -i submodule -- '*.rs'`
returns nothing). Two advisories do name paths this code definitely executes — checkout and
pack handling — so the exposure is non-zero, just not acute.

## Decision

**Bump `gix` to 0.87.0 — the current release — not 0.83.0, the minimum patched version.**

The cost of this change is the API break, and that cost is identical at either target. Taking
0.83.0 would pay it in full and still leave the repo four minor versions behind, guaranteeing
a second identical exercise within months.

Scope is gix only. The other alerts on the board — `quinn-proto`, `tar`, `serde_with`, and the
already-`deny.toml`-ignored `hickory-proto` — are deliberately excluded to keep the diff
reviewable against one dependency's behaviour change.

### The API change

`gix::url::parse` became `pub fn parse(input: impl AsBStr)`. The existing call form passes a
bare `.into()`, which no longer resolves to a unique type:

```rust
// before — E0283: cannot satisfy `_: AsBStr`
gix::url::parse(self.repo_url.as_str().into())

// after — &[u8] satisfies AsBStr directly
gix::url::parse(self.repo_url.as_str().as_bytes())
```

Measured by performing the bump and compiling on Darwin arm64, rustc 1.94.0, `cargo check --workspace --all-targets`: **14 errors, all this one pattern.** Four
production call sites — `lib/src/actions/git/clone.rs`, `lib/src/actions/git/pull.rs`,
`lib/src/actions/zsh/oh_my_zsh.rs`, `lib/src/manifests/providers/git.rs` — and roughly ten
more in existing test bodies. The remedy was verified on one site and the error cleared.

### The test, and why it is part of this change rather than follow-up

**No test reaches the gix branch of `execute()`.** This was stated in an earlier draft as
"nothing calls `execute()`", which is false and was caught by the population check in this
spec's own self-review — the original figure came from truncated `grep` output. The accurate
statement is narrower and more useful.

`execute()` has two branches. When `update_existing && directory.exists()` it shells out to
`git -C <dir> pull` as a subprocess and returns early, never touching gix. Otherwise it runs
`gix::prepare_clone` → `fetch_then_checkout` → `main_worktree`.

The clone atom has **8 tests**. Six exercise `plan()`. The two that call `execute()` both set
`update_existing: true` against an existing directory, so both take the **shell-out** branch
against a mocked `git` binary on `PATH`. The gix branch is executed by nothing. There are also
no `#[ignore]` network tests anywhere in the repo.

**This dictates a constraint on the new test, and it is the kind that fails silently.** A test
constructed with `update_existing: true` against a directory that already exists would return
at the early branch, pass, and verify nothing whatsoever about gix — indistinguishable from a
real pass. The new test must therefore target a **non-existent destination directory**, and it
should assert something only the gix path can produce (the cloned file's content), not merely
that `execute()` returned `Ok`.

The consequence for this bump is specific: `cargo check` clean and the full suite green would
say nothing about whether cloning still works after a four-version jump. The `url::parse` sites
are covered because they sit in `plan()`, so the *compile* fix is verifiable; the gix *runtime*
behaviour is not.

`behavior.md` requires naming the verification before building. For the gix path there is
currently no verification to name. Adding one test is what makes the bump verifiable at all.

**One unit test**, in the clone atom's existing `#[cfg(test)]` module:

1. `git init` a fixture repository in a `tempfile::tempdir()` and commit a single file with
   known content.
2. Construct the `Clone` atom against that local path with a **destination that does not yet
   exist**, so `execute()` takes the gix branch rather than the shell-out early return.
3. Assert the committed file exists at the destination **with its expected content**, and that
   the clone contains `.git`.

Step 2's constraint is load-bearing, not incidental — see the two-branch note above. Step 3
asserts content rather than an `Ok` return for the same reason: a test that only checks the
result type cannot distinguish a working clone from a branch that did nothing.

No network. gix parses `InputScheme::Local`, so a filesystem path exercises the real chain.

A **unit** test rather than one in `app/tests/`, deliberately: per `rust.md`, tarpaulin does
not instrument through `assert_cmd` subprocesses or `Deref<Target = dyn Trait>` vtable
dispatch, so an integration test would exercise the code and score zero coverage for it.

## Verification

| Check                          | What it proves                                       | When                |
| ------------------------------ | ---------------------------------------------------- | ------------------- |
| `make test`                    | Compiles at 0.87, and the clone path still functions | pre-push            |
| `cargo deny check advisories`  | No RustSec regression from the new tree              | pre-push            |
| Dependabot alert count 28 → ~5 | The advisories actually cleared                      | **post-merge only** |

The third is the only check that proves the stated problem is solved, and it cannot gate the
PR — the alert set is recomputed against the default branch after merge. Recorded as a
post-merge step rather than presented as a gate.

## Consequences and risks

**The bump crosses four minor versions, and the new test covers one path.** Behaviour beyond
`url::parse` may have shifted silently. The test covers `clone`; `pull`, `plugin` and
`oh_my_zsh` share the `url::parse` fix but not the execution coverage. This is a known,
accepted gap rather than an oversight — widening it is a larger piece of work than the bump
it would be attached to.

**Coverage will rise above the 81% floor.** The figure will be reported, not ratcheted:
ADR-0004 raises the floor on sustained measurement, not on one PR's result.

**Not addressed here, both separately backlogged:** the cargo-deny/GHSA corpus gap, and the
non-gix advisories.

## Related

- ADR-0004 — Rust coverage floor
- `docs/superpowers/README.md` backlog — the gix row this closes, and the corpus-gap row it does not
- `rust.md` — tarpaulin instrumentation limits informing the unit-vs-integration choice
