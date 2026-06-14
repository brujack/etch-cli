.PHONY: all test lint build build-linux install-hooks mutants bench changelog fuzz fuzz-manifest fuzz-path semver validate-plan

all: test build

test: lint
	cargo nextest run

lint:
	cargo fmt --all -- --check
	cargo clippy --all-targets -- -D warnings
	cargo machete

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
