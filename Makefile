BINARY = mcli
# Where cargo puts its output: target/, or build.target-dir from .cargo/config.toml.
TARGET_DIR = $(shell cargo metadata --format-version 1 --no-deps | sed -E 's/.*"target_directory":"([^"]*)".*/\1/')

all: test build

# Builds ./mcli.
build:
	@echo "--> Building $(BINARY)"
	cargo build --release --locked
	cp "$(TARGET_DIR)/release/$(BINARY)" ./$(BINARY)
	@echo "--> ./$(BINARY)"

# Builds ./mcli and puts it on PATH (~/.cargo/bin).
install: build
	@echo "--> Installing $(BINARY)"
	cargo install --path . --locked

test:
	cargo test --release --locked

lint:
	cargo fmt --check
	cargo clippy --release --all-targets --locked -- -D warnings

# The previous Go version (1.x), kept until it is removed from the repository.
go-build:
	go build -o ./build/$(BINARY) ./cmd/suid

.PHONY: all build install test lint go-build
.DEFAULT_GOAL := build
