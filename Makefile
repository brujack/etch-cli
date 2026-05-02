.PHONY: all test lint build install-hooks

all: test build

test: lint
	cargo test

lint:
	cargo fmt --all -- --check
	cargo clippy --all-targets -- -D warnings

build:
	cargo build --release

install-hooks:
	cp scripts/pre-commit .git/hooks/pre-commit
	chmod +x .git/hooks/pre-commit
	cp scripts/pre-push .git/hooks/pre-push
	chmod +x .git/hooks/pre-push
