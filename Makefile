.PHONY: all test lint build build-linux install-hooks mutants changelog fuzz fuzz-manifest fuzz-path

all: test build

test: lint
	cargo nextest run

lint:
	cargo fmt --all -- --check
	cargo clippy --all-targets -- -D warnings

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

fuzz-manifest:
	cd fuzz && cargo +nightly fuzz run fuzz_manifest corpus/fuzz_manifest -- -max_total_time=$(FUZZ_TIMEOUT)

fuzz-path:
	cd fuzz && cargo +nightly fuzz run fuzz_path_resolve corpus/fuzz_path_resolve -- -max_total_time=$(FUZZ_TIMEOUT)

fuzz: fuzz-manifest fuzz-path

changelog:
	git-cliff -o CHANGELOG.md
