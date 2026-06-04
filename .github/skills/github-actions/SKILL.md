---
name: github-actions
description: 'Inspect, validate, and fix GitHub Actions workflows in this repository. Use when working on .github/workflows, actionlint, runner labels, checkout/setup action versions, CI failures, workflow packaging, release automation, or local Actions verification with act and gh.'
argument-hint: 'Describe the workflow problem, check, or validation you want to run'
user-invocable: true
---

# GitHub Actions

## When To Use
- Fix failing workflows under `.github/workflows/`.
- Add or update CI checks for Rust builds, packaging, and releases.
- Validate runner labels, action versions, and workflow syntax.
- Reproduce GitHub Actions issues locally with `actionlint`, `act`, `gh`, or the same `cargo` commands used in CI.

## Repository Workflow Map
- `ci.yml`: workflow lint, rustfmt, clippy, tests, and release builds.
- `deploy.yml`: package bundles for Windows, Linux, and macOS.
- `release.yml`: tag builds and release asset publishing.

## Standard Procedure
1. Read the target workflow in `.github/workflows/` and identify the failing job or deprecated action.
2. Run local workflow validation before editing:
   - `actionlint`
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace --all-targets`
3. Prefer minimal workflow changes:
   - keep `actions/checkout` current
   - use supported runner labels
   - pin third-party tool download versions when practical
4. After edits, rerun `actionlint` and the narrow Rust checks affected by the workflow.
5. If a workflow packages binaries, verify filenames, targets, and artifact names match the Rust build output.

## Local Tools
- `gh`: inspect workflow runs, logs, and artifacts.
- `gh dash`: browse repository activity interactively.
- `act`: execute supported workflows locally when a Docker runtime is available.
- `actionlint`: static validation for workflow syntax and semantics.

## Project Notes
- The repo currently relies on Rust checks for CI quality gates.
- Use `macos-15-intel` for x86_64 macOS builds and `macos-latest` for arm64 unless a workflow needs a narrower label.
- Keep workflow validation in CI so GitHub Actions changes are checked on every push and pull request.

## References
- [Validation commands](./references/validation.md)