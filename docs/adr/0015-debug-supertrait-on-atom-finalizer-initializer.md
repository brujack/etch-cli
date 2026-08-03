# ADR-0015: Debug Supertrait on Atom, Finalizer, and Initializer

**Date:** 2026-08-02
**Status:** Accepted

## Context

Enabling `missing_debug_implementations` (Rust API Guidelines `C-DEBUG`) at `warn`
under `-D warnings` required `Debug` on all 43 flagged public types in `etch-lib`.
Forty derived cleanly. Three could not: `Step`, `finalizers::FlowControl`, and
`initializers::FlowControl` each hold a `Box<dyn Atom>`, `Box<dyn Finalizer>`, or
`Box<dyn Initializer>`, and none of those traits bounded `Debug`. A trait object is
only `Debug` if its trait requires it.

Alternatives considered:

- **Leave the three types un-derived and ship `missing_debug_implementations` at
  `allow`.** Cheapest, but it abandons the lint in the one crate where `C-DEBUG`
  actually has a consumer, and the `allow` would have no expiry condition — nothing
  would ever remove it.
- **Hand-write `Debug` for the three types, printing a placeholder for the boxed
  field.** Avoids touching the traits, but produces a `Debug` that omits the only
  interesting content — the atom being wrapped — and silently diverges from what a
  reader expects `{:?}` on a `Step` to show.
- **Add `Debug` as a supertrait on all three traits.** Makes every implementor
  `Debug` by construction, so the derives on `Step`/`FlowControl` work and stay
  working. Cost: it is a breaking change for any external implementor, and it
  permanently constrains every future `Atom`.

## Decision

Add `std::fmt::Debug` as a supertrait to `Atom`, `Finalizer`, and `Initializer`.
`Atom` already required `std::fmt::Display`, so a supertrait bound on the extension
point is a shape this codebase already uses.

The breaking-change objection was measured rather than reasoned about, because it is
the only argument against:

- `etch-lib` is **not published to crates.io** — the registry API returns `404` for
  it. (A bare request returns `403`; the User-Agent header is required to get the
  real answer. `CLAUDE.md` independently records the same fact.)
- **Zero implementors exist outside `lib/`** — neither `app/` nor `jsonschemagen/`
  implements any of the three traits.

The `license`/`repository`/`version = "0.14.0"` metadata on `etch-lib` is
aspirational, not evidence of a consumer. With nothing able to break, the objection
does not apply here — though it would the moment the crate is published.

## Consequences

**Every future `Atom`, `Finalizer`, and `Initializer` must be `Debug`.** This is the
intended effect: it is `C-DEBUG` enforced by the type system rather than by a lint
that a future `#[allow]` could switch off.

**Two types must keep a hand-written `Debug` and must never be "tidied" into a
derive.** `Decrypt` holds a plaintext passphrase and `Exec` holds an environment map;
both redact those fields manually. Because `Atom` now requires `Debug`, a derive on
either would put a secret into every `{:?}` of that value, including transitively
through `Step` and `Box<dyn Atom>`. `Display for Decrypt` already prints only the
path — the derive that this ADR's change made mandatory is precisely what reopened
that. Regression tests: `debug_output_redacts_the_passphrase` and
`debug_output_redacts_environment_values`, both mutation-verified.

**`cargo semver-checks` reports this as breaking, correctly.** The `Semver Check` CI
job fails on the PR that introduced it. That job is `continue-on-error: true` and is
excluded from `auto-merge`'s `needs:`, so it does not block — see the backlog row on
making it blocking. This is now the second known-breaking change on record alongside
`enum_variant_added`, and any work to make that job blocking has to account for both.

**Publishing `etch-lib` later re-opens the question.** The decision rests on the
measured absence of consumers, not on the change being harmless. If the crate is ever
published, this bound becomes a real compatibility commitment.

## Related

- ADR-0010 — mutation testing with cargo-mutants
- `ai-config/docs/superpowers/specs/2026-08-02-rust-api-guidelines-gate-design.md`
- etch-cli#122
