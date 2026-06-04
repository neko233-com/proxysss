# Validation Commands

Run these from the repository root.

## Workflow Validation
- `actionlint`
- `gh run list --limit 10`
- `gh run view <run-id> --log`

## Rust Checks Used By CI
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --all-targets`

## Local Workflow Execution
- `act pull_request`
- `act push`

`act` requires a working Docker runtime. If Docker is unavailable, use `actionlint` plus the same `cargo` commands from CI.