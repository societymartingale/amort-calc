# Mirrors .github/workflows/ci.yml

# List available recipes
default:
    @just --list

# Format all code
fmt:
    cargo fmt --all

# Check formatting (CI)
fmt-check:
    cargo fmt --all -- --check

# Clippy for wasm32 and host, warnings as errors (CI)
clippy:
    cargo clippy --all-targets --target wasm32-unknown-unknown -- -D warnings
    cargo clippy --all-targets -- -D warnings

# Run tests (CI)
test:
    cargo test --locked

# fmt-check + clippy
lint: fmt-check clippy

# Build the release site into dist/ (CI)
build:
    trunk build --release --locked

# Everything CI runs: lint, test, build
ci: lint test build

# Alias for ci
check: ci

# Dev server with live reload
serve:
    trunk serve

# Remove build artifacts
clean:
    cargo clean
    rm -rf dist
