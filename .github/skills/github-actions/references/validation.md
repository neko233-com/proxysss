# Validation Commands

Run these from the repository root.

## Workflow Validation
- `actionlint`
- `gh run list --limit 10`
- `gh run view <run-id> --log`

## Optional Rust Checks
Default CI is packaging-only. Use these locally when the change affects Rust behavior or when the user explicitly asks for tests:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --all-targets`

## Local Workflow Execution
- `act pull_request`
- `act push`

`act` requires a working Docker runtime. If Docker is unavailable, use `actionlint` plus the narrow cargo/build commands relevant to the edited workflow.
