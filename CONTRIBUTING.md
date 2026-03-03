# Contributing to wip-git

## Dev Setup

```bash
git clone https://github.com/ricardo-hdrn/wip-git.git
cd wip-git
cargo build
```

## Code Style

- `cargo fmt` — run before every commit
- `cargo clippy` — must pass with no warnings
- Keep dependencies minimal; justify new ones in the MR

## Running Tests

```bash
cargo test
```

Integration tests live in `tests/integration.rs` and create temporary git
repos (bare remote + local clone) to exercise the full save/load/list/drop/gc
flow.

## MR Process

1. Fork the project and create a feature branch from `develop`
2. Make your changes
3. Ensure `cargo fmt --check && cargo clippy && cargo test` all pass
4. Open a merge request against `develop`
5. Keep commits atomic — one logical change per commit
