# Contributing to BeatSplit

Thank you for your interest in contributing! This project especially welcomes
contributions from musicians, producers, and developers in the markets it serves.

## Getting Started

1. Fork the repo and create a feature branch:
   ```bash
   git checkout -b feat/my-feature
   ```
2. Make your changes with tests.
3. Run the quality checks:
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   cargo test
   ```
4. Open a pull request describing the change and its motivation.

## Guidelines

- **Contract changes:** open an issue to discuss the design first — this code moves money.
- **Tests required:** every new behavior needs a test.
- **No new features** beyond what is planned in the README without prior discussion.
- **Good first issues** are labeled `good first issue`.

## Code Style

- Rust: follow `rustfmt.toml` and pass `cargo clippy -- -D warnings`.
- TypeScript: follow the ESLint config in `app/`.
- Keep commits focused and atomic.

See `SECURITY.md` for how to report vulnerabilities.
