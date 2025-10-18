# Repository Guidelines

## Project Structure & Module Organization
`src/` hosts the Rust application: `main.rs` wires the CLI entrypoint, while focused modules such as `cli.rs`, `git.rs`, `worktree.rs`, and `monitor.rs` encapsulate domain logic. Shared fixtures and end-to-end scenarios live in `tests/`; add new CLI flows beside files like `close_command_test.rs` and reuse helpers from `tests/common/`. Reference material sits in `docs/`, coverage exports belong in `coverage_reports/`, and build artifacts stay in `target/` (never commit them).

## Build, Test, and Development Commands
`cargo build` produces a debug binary; use `cargo build --release` before running automation such as `./test_close_command.sh`. Exercise commands locally with `cargo run -- <subcommand>` (for example, `cargo run -- monitor`). `cargo test` runs the full suite, while targeted runs like `cargo test --test init_tests` or `cargo test --lib` keep feedback quick. Enforce quality with `cargo fmt --all` and `cargo clippy --all-targets --all-features -- -D warnings`; the same tasks are exposed via `mise run format` and `mise run lint`. Generate optional coverage with `cargo tarpaulin --out Html --output-dir coverage_reports`.

## Coding Style & Naming Conventions
Follow idiomatic Rust style: 4-space indentation, snake_case functions, UpperCamelCase types, and SCREAMING_SNAKE_CASE constants. Prefer cohesive modules that mirror existing files (e.g., add new worktree behaviour beside `worktree.rs`). Propagate errors with `anyhow::Result` and the `?` operator, and include actionable log/context messages to assist distributed agents.

## Testing Guidelines
Integration suites depend on fixtures under `tests/common/`; extend them instead of duplicating setup. Mark long-running or orchestration suites with `#[ignore]` (see `tests/comprehensive_test_runner.rs`) and document their command (`cargo test comprehensive_test_suite -- --ignored`). When fixing regressions, update tracking docs like `docs/TEST_FIXES_SUMMARY.md` and note coverage deltas if tarpaulin was used.

## Commit & Pull Request Guidelines
Adopt the conventional prefixes observed in history (`feat:`, `fix:`, `test:`, `chore:`) and keep each commit focused. Expand the body when context is non-obvious. Pull requests should summarize affected modules, link issues, and include terminal output or screenshots for UX changes. Confirm that `cargo fmt`, `cargo clippy`, and relevant `cargo test` targets (or scripts like `./test_close_command.sh`) have been executed, and list those commands in the PR description.

## Environment & Tooling Notes
Install supporting tools (`cargo watch`, `cargo-tarpaulin`, `sqlite3`) before running the heavier suites. `.mise.toml` loads `.env` and `.env.local`; keep secrets out of version control and document required variables in `docs/` when introducing new configuration knobs.
