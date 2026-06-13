# Changelog


## Bug Fixes

- dist-upgrade, autoremove, scope pip/gems to user envs

- scope pip list to user packages; extract testable helpers (#60)

- make add_to_group idempotent via id -nG membership check (#64)

- strip directory prefix from sha256sum in release workflow

- fix etch plugin update — inverted guard, missing fetch calls, wrong remote (#88)

- remove --yes from snap install invocation (#90)

- detect installed casks in installed_version() (#97)

- install cargo-machete in release workflow



## CI

- raise coverage gate 70% → 80% after PR #83 measured 80.17% on Linux



## Documentation

- document etch update command and fix cosign verify command

- fix etch update config example types and add missing fields

- document new-action checklist and restore lost backlog items

- mark pip.install Done, remove from backlog

- mark user.group idempotency Done, remove from backlog

- mark package.autoremove Done, remove from backlog

- mark npm.install Done, remove from backlog

- mark pyenv.install Done, remove from backlog

- add pyenv.install configure_opts for macOS Homebrew interference

- mark pyenv configure_opts Done, remove from backlog

- mark pyenv.virtualenv Done, remove from backlog

- mark ruby.install version_manager Done

- add ruby.install compile_flags for macOS OpenSSL

- add pyenv.install and pyenv.virtualenv examples and catalog entries

- document 8 previously undocumented actions in README and CLAUDE.md

- add npm.install and package.autoremove example manifests

- improve comments across 19 example manifests

- fix 4 inaccuracies in CLAUDE.md action catalog

- fix group/user action names, add plugin row

- note serde rename as canonical action name source

- add compile_flags plan, spec, mark Done

- add etch status coverage spec

- add etch status coverage implementation plan

- mark etch-status-coverage Done

- sync CLAUDE.md and plan index after learnings audit

- add ADR-0006 through ADR-0010 for major decisions

- mark package-upgrade Done in plan index

- document package.upgrade behavior and debug path

- add package.upgrade usage examples with where: guards

- flesh out package/install.yaml and upgrade.yaml

- consolidate package examples into package-management.yaml

- consolidate brew examples into brew-management.yaml

- remove stale etch-update-command pending row

- mark version-pinning Done, update coverage to ~77%

- ADR-0011 — version pinning error-on-mismatch semantics; update CLAUDE.md catalog

- mark macos.rosetta Done, remove from backlog

- document replace_all pattern for actions/mod.rs match arm edits

- add os.arch context field

- mark os.arch Done, remove from backlog

- document etch plugin CLI subcommands and plugin.rs tests

- mark verbose-apply Done, remove from backlog

- fix stale -v note and document verbose apply flag

- add subcommand help discoverability item

- close 6 coverage gap items after #80 merge

- update Linux CI coverage to ~78% after PR #80

- mark subcommand help-all Done after #81 merge

- update cli_commands.rs count 7→11 for help-all tests

- mark mas.install list: support Done after #82 merge

- update Linux CI coverage 77.64% → 77.67% after PR #82

- add error summary includes actual error item

- update Linux CI coverage 77.67% → 80.17% after PR #83; remove solved backlog items

- add 8 coverage gap items targeting atoms and actions layers

- update coverage 80.17% → 80.28% after PR #84; clear resolved backlog items

- update coverage 80.28% → 81.29% after PR #85; raise CI gate to 81%

- mark error-summary Done; remove from backlog

- move action catalog to knowledge/, trim testing section

- trim DoD to repo-specific addenda; fix coverage gate 80%→81%

- add package-streaming spec, plan, and mark Done

- update streaming behavior and coverage floor

- add 8 items from dotfiles/setup_env.sh capability gap analysis

- add claude.plugin spec

- add claude.install/upgrade to README, examples, and plan index

- add claude.install and claude.upgrade to action catalog

- add 2026-06-05 retrospective (PRs #70-#89)

- record snap install --yes bug

- claude.marketplace, claude.marketplace.remove, package.remove

- claude.marketplace, claude.marketplace.remove, package.remove

- mark claude-marketplace-package-remove Done in plan index

- update action count to 44, add cask field to package.remove catalog

- update action checklist to mention all 3 dispatch tests

- add claude.plugin.update design spec

- add claude.plugin.update implementation plan

- mark claude-plugin-update Done in plan index

- add claude.plugin.update to catalogs, bump action count to 45

- note dispatch test also fails during todo!() TDD phase

- add etch doctor implementation plan

- add etch doctor design spec

- mark etch-doctor Done in plan index

- add etch doctor to README and ADR-0012

- add snapshot/lib.rs/security notes from etch-doctor session

- add etch update --only/--skip design spec

- add etch update --only/--skip implementation plan

- mark etch-update-only-skip Done in plan index

- update README and ADR-0008 for --only/--skip flags

- add ruby.chruby design spec

- add ruby.chruby implementation plan

- mark ruby-chruby Done in plan index

- fix stale ruby.install chruby description in action catalog

- add zsh.oh-my-zsh design spec

- mark zsh.oh-my-zsh Done in plan index

- bump action count 45→46 after zsh.oh-my-zsh

- add macos.softwareupdate implementation plan

- add macos.softwareupdate design spec

- mark macos.softwareupdate Done, prune stale backlog

- bump action count 46→47 after macos.softwareupdate

- add terraform.tfenv design spec

- add terraform.tfenv implementation plan

- mark terraform.tfenv Done, prune backlog entry

- add binary.github version drift detection spec

- add binary.github drift detection implementation plan

- add pyenv.recreate-virtualenv action item

- mark binary.github drift detection Done

- ADR-0013 — binary.github version drift via sidecar files

- update binary example and coverage figure after PR #101

- add pyenv.virtualenv recreate: field design spec

- add pyenv.virtualenv recreate: implementation plan

- mark pyenv.virtualenv recreate: Done

- update pyenv.virtualenv recreate: in README, catalog, and coverage

- add user.default_shell and git.clone update_existing gaps

- add git.clone update_existing field design

- add git.clone update_existing implementation plan

- mark git.clone update_existing Done

- bump Linux coverage 81.33% → 81.38% after git.clone update_existing

- add user.default_shell design spec

- add user.default_shell implementation plan

- mark user.default_shell Done

- bump Linux coverage 81.38% → 81.48% after user.default_shell

- fix action count 49→50 and macos.default name

- bump Linux coverage 81.48% → 81.01% after state-manifest

- ADR-0014 — state manifest and etch history subcommand

- mark file-rollback plan Done, add status banner

- add rollback.rs to test file inventory in CLAUDE.md

- rename ansible-cop-review → ansible-good-practices in etch spec (#109)

- mark plan Done, add to index



## Features

- generate and publish SHA256 checksum

- add ruby.install action via ruby-install (#61)

- add gem.install action (#62)

- add pip.install action (#63)

- add package.autoremove action for apt orphan cleanup (#65)

- add npm.install action for global npm packages (#66)

- add pyenv.install action (#67)

- add configure_opts field to pyenv.install (#68)

- add pyenv.virtualenv action (#70)

- add version_manager field for rbenv post-install steps (#71)

- add compile_flags field (#72)

- add package.upgrade action for apt and snap (#74)

- add version: field to package.install for version pinning (#75)

- add macos.rosetta action (#76)

- add os.arch field to OS context provider (#77)

- re-register plugin subcommand in CLI (#78)

- add --verbose flag; suppress nothing-to-be-done by default (#79)

- add help-all subcommand to show all subcommand flags (#81)

- add list: field to mas.install for multiple apps (#82)

- embed error in summary line on action failure (#86)

- stream package manager output in real time (#87)

- add claude.install and claude.upgrade actions (#89)

- add claude.marketplace, claude.marketplace.remove, and package.remove actions (#91)

- add claude.plugin.update action (#92)

- add etch doctor subcommand (#93)

- replace per-category flags with --only/--skip (#95)

- add ruby.chruby action and extend version_manager: chruby (#96)

- add zsh.oh-my-zsh action (#98)

- add macos.softwareupdate action (#99)

- add terraform.tfenv action (#100)

- add version drift detection to binary.github (#101)

- add recreate: field to pyenv.virtualenv (#102)

- add update_existing field for clone-or-pull (#103)

- add user.default_shell action (#104)

- add state manifest and etch history command (#105)

- adopt 10-80-10 execution cycle (ai-config ADR-0009/0010) (#106)

- etch rollback subcommand with pre-apply file stash (#107)

- propagate DEBCONF_NONINTERACTIVE_SEEN and NEEDRESTART_MODE (#108)



## Testing

- add 7 integration tests for etch status (#73)

- add coverage for 7 backlog gap items (#80)

- cover actions/mod.rs false-condition and apply.rs error paths (#83)

- fix Linux tarpaulin gaps in atoms, actions, and values (#84)

- cover all 40 dispatch arms in inner_ref/notify/Deref (#85)



## Bug Fixes

- exit nonzero on manifest parse errors and step failures (#38)

- pin cosign-installer to v4.1.2

- migrate cosign to --bundle format (v4 dropped --output-signature)

- clear GIT_DIR in execute() to prevent hook repo corruption (#55)

- fix depends prefix and config path

- expand ~ in source paths before resolving

- set status.code from actual exit code on failure

- resolve three known bugs and improve apply output detail (#57)

- scope release notes to latest tag via git-cliff-action (#58)



## CI

- bump actions/checkout v5→v6 across all workflows

- enable regression alerts at 130% threshold

- add mutation score gate at 60% (#53)

- add workflow_dispatch to release-sign for manual re-runs

- make semver check continue-on-error



## Documentation

- error-path integration tests design

- error-path integration tests implementation plan

- mark criterion-benchmarks Done

- add dev-cycle improvements to backlog

- cargo-semver-checks design for etch-lib

- cargo-semver-checks implementation plan

- mark cargo-semver-checks Done

- add semver-checks baseline-rev note and update CI table

- remove benchmark alerts — done

- insta snapshot testing design for etch-cli

- mark insta-snapshot-testing Done

- remove cargo-deny license policy — not needed for personal tool

- add make semver and insta snapshot testing to README

- mark glob-file-link Done, update action catalog

- mark binary-url Done, update action catalog

- git.config action design

- add implementation plan

- mark Done, add DONE banner, update CLAUDE.md action catalog

- add capability gaps from dotfiles comparison

- add spec for array-add and delete operations

- add implementation plan

- mark Done in plan index

- add spec for git.pull action

- add implementation plan

- mark Done in plan index

- add manifest-examples backlog item, require examples in DoD

- add comprehensive manifest examples for all actions

- remove manifest-examples from backlog (implemented)

- mark Done, add plan and spec files

- add macos.service, update action catalog, fix stale link

- add design spec

- mark Done, add plan file

- note semver-checks advisory for Actions enum variants

- add design spec

- add implementation plan and update index

- template docs + handler-notify Done status

- add template variable namespace gotcha to file.copy entry

- add design spec

- add implementation plan

- mark Done, add DONE banner

- update integration test count to 11 after file.flags

- mark test-coverage plan Done

- update coverage to ~84%, document cli_commands.rs

- add mutation score threshold spec

- add mutation-score-threshold implementation plan

- mark mutation-score-threshold Done

- note Linux-only release binary; macOS requires build from source

- add 4 Nix-parity features with analysis

- add package.upgrade and etch update command

- write specs for 6 Nix-parity and update features

- mark drift-detection Done

- update Linux CI coverage figure to 75% after PR #56

- add file.link log message bug to backlog

- move bug to new Bugs section above Backlog

- add package install progress output to backlog

- add DEBIAN_FRONTEND bug to Bugs section

- add verbose apply output to backlog

- expand DEBIAN_FRONTEND bug with needrestart trigger

- add package.autoremove for apt cleanup on Ubuntu

- add Debugging section to README

- add journald DEBUG log suppression bug

- fix -v flag placement in Debugging section

- add etch update subcommand spec

- mark etch-update-command plan Done



## Features

- add git-cliff changelog generation to release workflow (#37)

- add cargo-fuzz targets for manifest parsing and path resolution (#39)

- add Criterion benchmarks for etch-lib (#40)

- add cargo-semver-checks for etch-lib API compatibility (#41)

- glob/wildcard pattern support (#43)

- add binary.url action for arbitrary URL installs (#44)

- add git.config action for declarative gitconfig management (#45)

- add array-add and delete operations (#46)

- add git.pull action (#47)

- add macos.service action for declarative launchctl management (#48)

- add systemd.service action (#49)

- add Ansible-style handler/notify pattern (#50)

- add file.flags action for BSD file flags (macOS) (#51)

- add personal-workstation machine setup template (#54)

- add drift detection via etch status command (#56)

- add etch update subcommand (#59)



## Testing

- add insta snapshot tests for CLI output (#42)

- add missing fish/contexts/plugin CLI tests (#52)

- add #[serial] to config_unset git tests



## Bug Fixes

- replace deprecated set-output workflow usage

- gate unix-only file action code to clear windows warnings

- gate unix-only test imports in file download action

- lint-test-release job

- replace serde_yml with serde_yaml_ng

- patch security vulnerabilities and resolve clippy warnings

- inline format args to satisfy clippy::uninlined_format_args

- inline format args in app crate for clippy compliance

- smoke tests — move fixtures to files/ and drop manifest_dir template (#5)

- log full error chain on manifest Tera render failure (#6)



## CI

- align minimum Rust version to 1.88.0 in PR workflow

- replace gitleaks-action with direct binary install

- align with math repo — Swatinem cache, tarpaulin coverage, direct snyk/gitleaks installs

- use Swatinem/rust-cache@v2 in build job (missed in prior pass)

- add cargo fmt check and --all-targets clippy to make lint (#2)

- temporarily disable release build job

- add docs-lint and docs-build jobs; clean remaining stale docs (#18)

- add weekly scheduled cargo-audit workflow

- add PR title lint workflow

- ignore known-safe advisories in scheduled cargo audit

- add coverage badge (#31)

- add CodeQL SAST workflow (#32)

- add monthly mutation testing workflow for etch-lib (#36)



## Documentation

- add maintainer search notice to README

- add CLAUDE.md

- add platform pruning spec

- add platform pruning implementation plan

- update coverage to 39%, mark Phase 3 done

- add test coverage improvement spec

- add test coverage implementation plan

- update coverage figures — 65% CI gate, ~75% local macOS

- fix smoke test command syntax — -d is a global flag

- remove fixed bug from superpowers backlog

- mark dead-code-removal done, update coverage figure to ~79.4%, clean backlog

- add dotfiles gap analysis to backlog

- add dry-run mode spec

- add dry-run implementation plan

- mark dry-run Done; add gh --repo flag warning to CLAUDE.md

- add dotfiles symlinks migration spec (Phase 2)

- add etch.yaml to dotfiles repo and core.yaml symlinks

- fix os.family → os.name in dotfiles symlinks spec

- add dotfiles symlinks implementation plan

- add tilde expansion gap to backlog

- mark dotfiles-symlinks plan In Progress

- mark dotfiles-symlinks plan Done (dotfiles PR #83 merged)

- add privileged file actions gap to backlog

- add file.chmod action spec

- add file.chmod implementation plan

- add file action privileged support spec

- add file-action-privileged implementation plan

- mark file-action-privileged Done; update action catalog

- remove stale backlog entries (file.chmod and privileged done)

- add tilde expansion spec; move from backlog

- add tilde expansion implementation plan

- mark tilde-expansion Done (PR #23 merged)

- add brew.upgrade and mas.upgrade to backlog

- add brew.cleanup to backlog

- add brew.bundle spec; remove from backlog

- add brew.bundle implementation plan

- mark brew-bundle Done; add to action catalog

- add package.install cask spec; remove from backlog

- add package.install cask implementation plan

- mark package-install-cask Done; update action catalog

- add mas.install spec; remove from backlog

- add mas.install implementation plan

- mark mas-install Done; add to action catalog

- document Homebrew workflow + tap management; update action catalog

- add docs/adr/ with 4 architectural decision records

- add brew.upgrade/cleanup and mas.upgrade spec; remove from backlog

- add brew.upgrade/cleanup/mas.upgrade implementation plan

- mark brew-upgrade-cleanup-mas-upgrade Done; update action catalog

- add machine profiles spec (pure convention, zero code)

- add machine profiles implementation plan

- add Machine Profiles section to CLAUDE.md

- add machine-profiles examples

- mark machine-profiles Done; remove from backlog

- add command.run skip_if_exists spec; remove from backlog

- add command.run skip_if_exists implementation plan

- mark command-run-skip-if-exists Done; update action catalog

- add knowledge directory for reference material

- add architecture docs and web research categories

- document 70% coverage as explicit tdd.md exception

- add Definition of Done

- 2026-05-17 retrospective — fork rename + coverage push + action sprint

- add Committing Work section with caveman-commit reference

- document cargo-audit vs cargo-deny ignore config split

- mark CodeQL plan done

- ADR-0005 — CodeQL SAST is advisory

- add test-metrics plan

- mark test-metrics plan done

- add release-pipeline and sbom-cosign plans

- mark sbom-cosign plans done

- add integration tests spec

- add integration tests plan

- document files/ subdir requirement in integration test spec

- mark integration-tests plan done

- document integration tests in Testing section

- add mutation testing spec

- add mutation testing plan

- mark mutation-testing plan done

- fix Status Key "In Progress" row labeled Done



## Features

- improve --dry-run output with banner, step counts, and verbose atoms (#19)

- add file.chmod action (#21)

- add privileged/sudo support to file.chown, file.link, file.copy (#22)

- expand ~ in manifest_paths via shellexpand (#23)

- add brew.bundle action (#24)

- add cask: true field to package.install for Homebrew cask installations (#25)

- add mas.install action for Mac App Store installations (#26)

- add brew.upgrade, brew.cleanup, and mas.upgrade actions (#27)

- add skip_if_exists to command.run (#28)

- auto-rustfmt on .rs writes

- add PR template, conventional commits hook, cargo-audit in CI

- adopt cargo-nextest as test runner (#29)

- switch from Dependabot to Renovate

- flaky-test tracking via nextest CI profile and test-metrics artifact (#33)

- SBOM generation and cosign signing for releases (#34)



## Performance

- skip tests for non-Rust file changes



## Refactoring

- remove dead code across values, contexts, lua, and git provider (#15)

- remove Context::ListContext variant and clean up allow(dead_code) (#16)

- remove Windows #[cfg(not(unix))] stubs and update docs (#17)



## Testing

- raise coverage from 39% to 75% (#4)

- add GitManifestProvider unit tests — 5% → 48% coverage (#7)

- cover LuaFunction/LuaRuntime json_schema bodies (+0.30% coverage) (#8)

- cover decrypt display/get_path/missing-path and remove readonly-parent (#9)

- cover FileCopy template render error path (#10)

- 100% coverage for group/add and user/mod (#11)

- 100% coverage for group/mod, user/add_group, and None providers (#12)

- package/mod 100% coverage and file/mod non-NotFound load error (#13)

- directory/copy trailing-slash and plugin cache hit (+0.07%) (#14)

- add coverage-improving tests; raise CI gate to 75% (#20)

- add proptest roundtrip property tests for json_to_lua (#30)

- add integration tests for file.link, file.copy, command.run, directory.create (#35)



## Bug Fixes

- properly ignore files directory (#551)

- added code to fetch the latest binary when version value passed is 'latest' (#572)



## Features

- inject variables via cli (#553)



## Bug Fixes

- remove deprecated option (#497)

- ensure queries can run when provider needs bootstrapped (#498)

- no diffmenu is now the default and the flag was removed (#501)

- Adds EndeavourOS to the yay list (#530)

- purge "files" at the ignore walker phase instead of the final walk (#531)



## CI

- remove tarpaulin (#529)



## Features

- add paru as new providor (#496)

- chown file (#506)

- chown files on download (#507)

- add unarchive file action (#517)

- git clone via gix (#523)

- gix manifest sourcing (#533)



## Features

- Manifest status (Plan to add status of manifests) (#423)

- Privilege providers (#479)



## Bug Fixes

- correct error propagation from exec errors

- ensure the manifest name is the dependency prefix for local dependencies when at top level

- error when there's unrecognized fields in manifest

- vartiants examples

- missing `/` for `share/keyrings` (#434)



## Features

- "gen_completions" command (#422)



## Refactoring

- replace deprecated serde_yaml with serde_yml (#417)

- replace structopt/paw with clap (#419)



## Bug Fixes

- attempt to fix aarch64 linux binaries (#357)

- aarch64 binary artifact (#360)

- octocrab::instance needs to be wrapped in a runtime call



## Bug Fixes

- warnings



## Bug Fixes

- bump lib version



## Bug Fixes

- broken link



## Features

- side-effects



## Bug Fixes

- use new syntax for build badge

- manifest_paths example and local dependencies

- remove cargo warnings



## Bug Fixes

- ensure all actions in a manifest run

- check to location for file.copy

- ensure failed where clause doesn't stop exeuction



## Features

- allow label selector on manifest apply (#264)



## Bug Fixes

- ensure we canonicalize manifest directory in-case it's a symlink

- ensure directory doesn't exist for execute test

- make linter happy

- tidy up warnings

- manifest path wasn't being adjusted correctly without config

- remove debug statements

- integrate package repository

- cleanup output with extra context and less noise

- rebase on main

- move code settings to examples

- ensure primary name for all actions is represented in JSON Schema

- use canonical name for package providers

- remove debug statement

- adjust to canonical source path

- rebase and adjust scoring match for binary downloads

- handle bitness unknown

- include lockfile to ease packaging for distributions



## Features

- action and atoms for git.clone

- add nix shell

- prototype macOS default support

- migrate version to subcommand

- migrate apply to subcommand

- replace Koto with Rhai

- seperate package repository and install

- generate JSON Schema documents

- add hostname as os context value

- initial build of github releases support



## Performance

- limit the scope of canonical source path



## Refactoring

- add more descriptive error message to file resolution (#241)



## Bug Fixes

- assertion for simple test-case

- handle lack of repository for aptitude

- normalize paths without canonicalize, as the latter errors

- expect that metadata for file won't exist before previous atoms execute

- removed vendored openssl

- use vendored openssl only on Windows

- attempt to remove openssl, as it's not a direct dependency



## Features

- allow apt-key to be used with apt repositories

- allow file.link to "walk" and link all files in a directory

- implement already installed query for provider yay



## Bug Fixes

- correct matrix generation variable

- version bump for 0.7.x release

- avoid warnings for now

- ensure artifacts have unique names

- can't use name as it searches for a  tag with that

- i'm guessing , but maybe its not the name and its actually the release notes

- hopefully release path

- dir isn't required for command.exec/run

- handle .exec extenion on windows build

- handle .exec extenion on windows build

- output was cached and caused problems



## Features

- deterministic manifest naming based on path and filename



## Bug Fixes

- remove use of alias due to bug in serde

- revert to upstream os_info

- name/list tests for use without alias

- remove and ignore lockfile



## Features

- elevate variants and conditions to all actions

- predicates using eval lib

- switch predicates to koto

- switch predicates to koto on primary predicate



## Features

- file.download action

- add user id to user context



## Bug Fixes

- ensure variants are overlayed correctly



## Features

- add OS contexts

- add atom to create directory and remove Exec dependency

- directory.create action

- support #ref:path with Git sourced manifests



## Testing

- cleanup tests



## Bug Fixes

- ensure ci/cd intermediates are ignored by git for cargo publish



## Bug Fixes

- can't run tests with cross until new version

- appease clippy



## Bug Fixes

- there's no templating for directory actions, so remove tera (#76)

- users can only be built on unix (#84)

- remove unused imports for windows (#85)

- renames missing from windows code

- only render files which have templating enabled

- step needs to be mut for execute

- extract initializer and finalizer code to step

- ensure BSD package provider uses finalizer

- ensure unix tests run on unix

- correct mutability for windows atoms execute

- omit tests on windows

- make considerations for container build environment

- this should ensure tests pass on Windows



## Features

- initial BSD pkg support (#72)

- end to end tests and initial ChangeSets (#78)

- initial atom implementation for file create and chown (#83)

- adding command atom with initializers and finalizers (#87)

- add file symlink atom (#88)

- migrate all actions to atoms (#89)

- http download atom (#90)

- add ability to print version

- promote initializers and finalizers to all atoms

- run initializers on action atoms



## Refactoring

- replace custom ResultExt with anyhow (#71)



## Bug Fixes

- cleanup linting and windows compilation errors (#60)

- remove early return



## Features

- "--dry-run" flag (#57)



## Bug Fixes

- codecov supports tokenless uploads



## Features

- Add yay provider (#51)

- allow specifying manifest location through config file (#55)



## Bug Fixes

- chmod should be integer (#52)



## Features

- command.run action and sudo fixes for Aptitude (#50)



## Bug Fixes

- correctly parse chmod into octal



## Bug Fixes

- use correct variable for changelog generator



## Bug Fixes

- use example for directory.copy test

- covert octal chmod value to u32



## Bug Fixes

- use vendors openssl (#45)



## Bug Fixes

- don't trigger rebuilds on 'edit' of PR desc



## Features

- support HTTP url's as Git repositories (#44)



## Bug Fixes

- move unix use statement to conditional function



## Bug Fixes

- make chmod work for windows and unix



## Bug Fixes

- swap out changelog generator for a conventional commits generator



## Features

- support Winget for Windows (#40)

- support chmod / permissions for file.copy (#41)



## Bug Fixes

- check if the symlink exists to the correct file before linking (#33)



## Features

- Added Powershell download script (#31)



## Bug Fixes

- rename function for Windows file.link support (#29)



## Bug Fixes

- typo

- check for brewfix before querying Cellar/Caskroom (#28)



## Documentation

- add one line installation option



## Features

- add script to install Comtrya

- file.link action added (#27)



## Features

- add aarch64-apple-darwin target



## Bug Fixes

- specify changelog generator version



## Bug Fixes

- only run changelog action once

- use correct path variable for dir scan (#9)



## Documentation

- clean up old docs



## Documentation

- add changelog to release process (#7)



## Bug Fixes

- use correct bin name / path



## Bug Fixes

- homebrew works best when we remove ambiguity and include repository in package name



## Features

- support extra_args on package.install



## Bug Fixes

- exit when a manifest fails



## Bug Fixes

- use Tera one_off to avoid false template parsing



## Features

- adding directory.copy action



## Bug Fixes

- add span for YAML parsing



## Bug Fixes

- install required rust components

- dirs_next should be dirs-next

- remove incorrect clippy flag

- GitLab CI doesn't support { var syntax

- correctly provide target variable

- check for homebrew packages already installed and correctly error for those that can't be



## Bug Fixes

- omit files directory



## Features

- add trace flag



## Bug Fixes

- ensure we check for apt-add-repository



## Features

- use proper output logging



## Features

- add aptitude support



## Features

- add repository support to package providers: homebrew



## Features

- drop walkdir for ignore: this allows filtering with gitignore files



## Bug Fixes

- correctly add args to brew install

- handle relative paths for manifest-dir

- remove broken dep



## Documentation

- update docs to latest syntax



## Features

- initial implementation of CommandAction



## Bug Fixes

- run all manifests when none provided as arg

- remove broken links

- walk sub graph correctly



## Documentation

- add Manifests to vocab in README

- remove last module reference



## Features

- initial commit

- add user context provider

- allow variants in Yaml and modularise the providers

- refactor Package, PackageConfig, and Provider management

- apt provider

- allow running one, or many, manifests with --manifests

- simple GitLab CI to ensure project builds

- allow disabling template rendering for individual files


