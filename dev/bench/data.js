window.BENCHMARK_DATA = {
  "lastUpdate": 1790822296222,
  "repoUrl": "https://github.com/brujack/etch-cli",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "name": "Bruce Jackson",
            "username": "brujack",
            "email": "bjackson@pobox.com"
          },
          "committer": {
            "name": "Bruce Jackson",
            "username": "brujack",
            "email": "bjackson@pobox.com"
          },
          "id": "5deb2660ed9cf6387c80868608ee132247bd6b85",
          "message": "chore(renovate): let pins and digests flow, hold only majors\n\nOperator ruling. The automerge-ok guard held pin and digest PRs as well as\nmajors, because pinDigest is its own updateType matching neither existing\nrule -- so pins were caught by omission rather than by decision, and that\nstalls ADR-0006 digest pinning.\n\nPlaced AFTER the major rule, not between: renovate_preset_sync tests that\npackageRules BEGIN with the canonical pair, so a trailing rule reads as a\ndeliberate append and an interposed one as drift.\n\nFour update types remain held by omission rather than decision --\nlockFileMaintenance, rollback, bump and replacement. Verified against\nRenovate's published schema, which enumerates ten. replacement is the one\nworth deciding deliberately: it swaps one dependency for a different one.\n\nCo-Authored-By: Claude Opus 5 <noreply@anthropic.com>\nClaude-Session: https://claude.ai/code/session_013gZBa9GmXhZccqZeemsGmo",
          "timestamp": "2026-08-24T22:04:06Z",
          "url": "https://github.com/brujack/etch-cli/commit/5deb2660ed9cf6387c80868608ee132247bd6b85"
        },
        "date": 1788229739834,
        "tool": "cargo",
        "benches": [
          {
            "name": "manifest_yaml",
            "value": 9880,
            "range": "± 242",
            "unit": "ns/iter"
          },
          {
            "name": "manifest_toml",
            "value": 706,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "file_link_resolve/single_dotfile",
            "value": 1188,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "file_link_resolve/nested_path",
            "value": 1823,
            "range": "± 40",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Bruce Jackson",
            "username": "brujack",
            "email": "bjackson@pobox.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "f1be54deb6b1ca330da64829bc8823c59e957adc",
          "message": "fix(deps): bump rustls to 0.23.45 for RUSTSEC-2026-0285 (#130)\n\n* fix(deps): bump rustls to 0.23.45 for RUSTSEC-2026-0285\n\nrustls 0.23.40 accepts TLS 1.3 handshake messages across encryption\nlevel boundaries. etch is a TLS client on this path (octocrab,\nreqwest, gix transport, update-informer).\n\n`cargo update -p rustls` stops at 0.23.43: 0.23.44+ needs a newer\naws-lc-rs, which plain -p keeps locked. `--precise 0.23.45` also\nmoves rustls-webpki 0.103.15, aws-lc-rs 1.18.1, aws-lc-sys 0.45.0.\n\nScheduled cargo-audit has failed since 2026-09-21; the blocking\ncargo-deny PR job failed on main for the same advisory.\n\nCo-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>\nClaude-Session: https://claude.ai/code/session_01JpgnFhAKUzzmpqSKqmDiZz\n\n* docs(backlog): note fuzz/Cargo.lock still pins rustls 0.23.40\n\nCo-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>\nClaude-Session: https://claude.ai/code/session_01JpgnFhAKUzzmpqSKqmDiZz\n\n* test: assert CI Python pin directly, not via CLAUDE.md\n\n5110f6b moved the platform-invariance reasoning to ai-config\nknowledge, so the regex over CLAUDE.md matched nothing and main's\nmake test has failed since. The invariant that matters is that CI\npins below 3.14, keeping the compression.zstd test skipped while\nthe Python coverage floor stands; assert that against ci.yml.\n\nMutation-checked: pin set to 3.14 turns the test red.\n\nCo-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>\nClaude-Session: https://claude.ai/code/session_01JpgnFhAKUzzmpqSKqmDiZz\n\n* fix(test-quality): accept a patch-level Python pin\n\nA \"3.13.1\" pin failed with \"expected one python-version pin, found []\",\nnaming the wrong cause. Match an optional patch component; compare on\nmajor.minor only.\n\nCo-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>\nClaude-Session: https://claude.ai/code/session_01JpgnFhAKUzzmpqSKqmDiZz\n\n---------\n\nCo-authored-by: Claude Opus 5.5 (1M context) <noreply@anthropic.com>",
          "timestamp": "2026-09-28T16:31:39Z",
          "url": "https://github.com/brujack/etch-cli/commit/f1be54deb6b1ca330da64829bc8823c59e957adc"
        },
        "date": 1790822295056,
        "tool": "cargo",
        "benches": [
          {
            "name": "manifest_yaml",
            "value": 11802,
            "range": "± 342",
            "unit": "ns/iter"
          },
          {
            "name": "manifest_toml",
            "value": 845,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "file_link_resolve/single_dotfile",
            "value": 1535,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "file_link_resolve/nested_path",
            "value": 2286,
            "range": "± 53",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}