.PHONY: all test lint build build-linux install-hooks

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
