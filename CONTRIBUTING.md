# Contributing to mini-crdt

Thank you for your interest in contributing! This document outlines the process for contributing to mini-crdt.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/sagami121/mini-crdt.git`
3. Create a branch: `git checkout -b my-feature`
4. Make your changes
5. Run tests: `cargo test --all-features`
6. Submit a pull request

## Development Setup

```bash
# Install Rust (if not already installed)
# https://rustup.rs/

# Run all tests
cargo test --all-features

# Run with no_std + alloc
cargo test --no-default-features --features alloc

# Run property-based tests
cargo test --test proptests

# Run benchmarks
cargo bench

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --all-features
```

## Code Style

- Follow standard Rust conventions (rustfmt)
- Run `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Add tests for new functionality
- Update documentation for public API changes

## Testing Requirements

All contributions must:

1. Pass all existing tests
2. Include tests for new functionality
3. Pass property-based tests where applicable
4. Work with all feature combinations:
   - `cargo test` (default/std)
   - `cargo test --features serde`
   - `cargo test --no-default-features --features alloc`
   - `cargo test --no-default-features --features "alloc,serde"`

## Pull Request Process

1. Ensure your PR description clearly explains the changes
2. Link any related issues
3. Keep PRs focused - one feature/fix per PR
4. All CI checks must pass
5. Maintainers will review and merge

## Reporting Issues

When reporting bugs, please include:

- Rust version (`rustc --version`)
- Operating system
- Steps to reproduce
- Expected vs actual behavior
- Minimal code example if possible

## Feature Requests

For new CRDT types or features:

1. Open an issue first to discuss the design
2. Consider mathematical properties (associativity, commutativity, idempotency)
3. Include property-based tests
4. Document the invariants and use cases

## License

By contributing, you agree that your contributions will be licensed under the MIT License (same as the project).