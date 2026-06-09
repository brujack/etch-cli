# Architectural Decision Records

Repo-specific decisions for etch-cli. Cross-cutting decisions that apply across personal repos live in [`dotfiles/docs/adr/`](https://github.com/brujack/dotfiles/tree/master/docs/adr).

| ADR                                                      | Title                                                     | Date       | Status   |
| -------------------------------------------------------- | --------------------------------------------------------- | ---------- | -------- |
| [0001](0001-fork-comtrya-as-etch-cli.md)                 | Fork comtrya as etch-cli                                  | 2026-05-02 | Accepted |
| [0002](0002-platform-pruning.md)                         | Prune to macOS and Ubuntu 24.04/26.04 only                | 2026-05-02 | Accepted |
| [0003](0003-file-action-config-shared-struct.md)         | FileActionConfig shared struct for file actions           | 2026-05-15 | Accepted |
| [0004](0004-ci-coverage-floor.md)                        | CI coverage floor exception to global 90% standard        | 2026-05-16 | Accepted |
| [0005](0005-codeql-sast-advisory.md)                     | CodeQL SAST is advisory                                   | 2026-05-19 | Accepted |
| [0006](0006-native-action-expansion-strategy.md)         | Native action expansion strategy                          | 2026-05-19 | Accepted |
| [0007](0007-etch-status-drift-detection-subcommand.md)   | etch status drift detection subcommand                    | 2026-05-19 | Accepted |
| [0008](0008-etch-update-subcommand.md)                   | etch update subcommand                                    | 2026-05-24 | Accepted |
| [0009](0009-release-strategy-git-cliff-cosign-sha256.md) | Release strategy — git-cliff changelog, cosign v4, SHA256 | 2026-05-23 | Accepted |
| [0010](0010-mutation-testing-cargo-mutants.md)           | Mutation testing with cargo-mutants                       | 2026-05-24 | Accepted |
| [0011](0011-version-pinning-error-on-mismatch.md)        | package.install version pinning uses error-on-mismatch    | 2026-06-02 | Accepted |
| [0012](0012-etch-doctor-subcommand.md)                   | etch doctor subcommand                                    | 2026-06-06 | Accepted |
| [0013](0013-binary-github-version-drift-sidecar.md)      | binary.github version drift detection via sidecar files   | 2026-06-09 | Accepted |
