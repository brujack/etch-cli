window.BENCHMARK_DATA = {
  "lastUpdate": 1788229740694,
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
      }
    ]
  }
}