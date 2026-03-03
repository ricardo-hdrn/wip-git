# wip-push development recipes

# Build debug binary
build:
    cargo build

# Run all tests
test:
    cargo test

# Run clippy lints
clippy:
    cargo clippy -- -D warnings

# Check formatting
fmt:
    cargo fmt -- --check

# Fix formatting in-place
fmt-fix:
    cargo fmt

# Build optimized release binary
release:
    cargo build --release

# Install locally via cargo
install:
    cargo install --path .

# Run all checks (fmt + clippy + test)
check: fmt clippy test
