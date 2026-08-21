.PHONY: all test lint build build-linux install-hooks mutants bench changelog fuzz fuzz-manifest fuzz-path semver validate-plan docs-debt

# Derived from the tracked set (git ls-files), not a hand-maintained list --
# an omitted file would leave a hand-list's coverage unchanged rather than
# lowering it (tdd.md "Coverage Denominators"). The env -u prefix strips a
# GIT_DIR that git exports into a worktree pre-push hook's environment
# (ci.md/shell.md); without it this parse-time assignment can silently
# resolve against the wrong repository.
SHELLCHECK := $(shell command -v shellcheck 2>/dev/null)
SHELL_FILES := $(shell env -u GIT_DIR -u GIT_WORK_TREE -u GIT_COMMON_DIR -u GIT_INDEX_FILE \
                 git ls-files '*.sh' '*.bash' 'scripts/pre-commit' 'scripts/pre-push' 'scripts/commit-msg')

all: test build

test: lint
	cargo nextest run
	pytest tests/ -v

lint:
	cargo fmt --all -- --check
	cargo clippy --all-targets -- -D warnings
	cargo machete
	ruff check scripts/ tests/ .claude/scripts/
	ruff format --check scripts/ tests/ .claude/scripts/
	@if [ -z "$(SHELL_FILES)" ]; then \
	  printf 'lint: derived shell file list is EMPTY — refusing to report a pass having linted nothing.\n' >&2; \
	  exit 1; \
	fi
	@if [ -n "$(SHELLCHECK)" ]; then \
	  if [ -n "$(SHELL_FILES)" ]; then shellcheck $(SHELL_FILES) && printf "shellcheck OK\n" || exit 1; fi; \
	else \
	  printf "shellcheck not found, skipping (install: brew install shellcheck)\n"; \
	fi

# `grep -c` exits 1 when it counts zero matches, so without the `|| true` this
# target would fail at exactly the moment the debt is cleared — the success
# state. The count still prints; only the exit status is suppressed.
docs-debt:
	@cargo clippy --workspace --all-targets --quiet -- -W missing_docs 2>&1 \
	  | grep -c 'missing documentation' || true

build:
	cargo build --release
	cp target/release/etch ~/Downloads/etch

build-linux:
	cargo zigbuild --release --target x86_64-unknown-linux-gnu
	cp target/x86_64-unknown-linux-gnu/release/etch ~/Downloads/etch-linux

install-hooks:
	cp scripts/pre-commit .git/hooks/pre-commit
	chmod +x .git/hooks/pre-commit
	cp scripts/pre-push .git/hooks/pre-push
	chmod +x .git/hooks/pre-push
	cp scripts/commit-msg .git/hooks/commit-msg
	chmod +x .git/hooks/commit-msg

mutants:
	cd lib && cargo mutants --timeout 120 --no-shuffle

FUZZ_TIMEOUT ?= 60
# Resolve nightly cargo via rustup to support both rustup-managed and Homebrew-installed rustup
CARGO_NIGHTLY := $(shell PATH="$$PATH:/opt/homebrew/bin" rustup which --toolchain nightly cargo 2>/dev/null)
NIGHTLY_BIN := $(shell dirname $(CARGO_NIGHTLY) 2>/dev/null)

fuzz-manifest:
	cd fuzz && PATH="$(NIGHTLY_BIN):$(HOME)/.cargo/bin:$(PATH)" $(CARGO_NIGHTLY) fuzz run fuzz_manifest corpus/fuzz_manifest -- -max_total_time=$(FUZZ_TIMEOUT)

fuzz-path:
	cd fuzz && PATH="$(NIGHTLY_BIN):$(HOME)/.cargo/bin:$(PATH)" $(CARGO_NIGHTLY) fuzz run fuzz_path_resolve corpus/fuzz_path_resolve -- -max_total_time=$(FUZZ_TIMEOUT)

fuzz: fuzz-manifest fuzz-path

bench:
	cargo bench -p etch-lib

semver:
	cargo semver-checks check-release -p etch-lib --baseline-rev origin/main

changelog:
	git-cliff -o CHANGELOG.md

# 10-80-10 cycle (ai-config ADR-0009/0010) — validate a plan file
validate-plan:
ifndef PLAN
	@printf "error: PLAN is required, e.g. make validate-plan PLAN=docs/superpowers/plans/foo.md\n" >&2
	@exit 2
endif
	@python3 ~/.claude/scripts/validate-plan.py "$(PLAN)"
