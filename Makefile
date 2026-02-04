.PHONY: fmt clippy test coverage build release audit all

fmt:
	cargo fmt

clippy:
	cargo clippy

test:
	cargo test

build:
	cargo build

release:
	cargo build --release

coverage:
	cargo tarpaulin --all-features --tests

audit:
	cargo audit

all: fmt clippy test release
