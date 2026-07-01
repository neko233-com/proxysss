---
name: github-actions
description: 'Inspect, validate, and fix GitHub Actions workflows in this repository. Use when working on .github/workflows, actionlint, runner labels, checkout/setup action versions, CI failures, workflow packaging, release automation, or local Actions verification with act and gh.'
argument-hint: 'Describe the workflow problem, check, or validation you want to run'
user-invocable: true
---

# GitHub Actions

## When To Use
- Fix failing workflows under `.github/workflows/`.
- Add or update workflow packaging, release automation, and any explicitly requested local/CI checks.
- Validate runner labels, action versions, and workflow syntax.
- Reproduce GitHub Actions issues locally with `actionlint`, `act`, `gh`, or the same `cargo` commands used in CI.

## Repository Workflow Map
- `ci.yml`: packaging-only build matrix for the six release bundles; no default tests, smoke, or performance benchmark jobs.
- `deploy.yml`: package bundles for Windows, Linux, and macOS.
- `release.yml`: tag builds and release asset publishing.

## Standard Procedure
1. Read the target workflow in `.github/workflows/` and identify the failing job or deprecated action.
2. Run local workflow validation before editing:
   - `actionlint`
   - narrow cargo build/check commands only when the workflow change affects Rust compilation behavior
3. Prefer minimal workflow changes:
   - keep `actions/checkout` current
   - use `actions/upload-artifact@v6` and `actions/download-artifact@v6` (Node.js 24 LTS); never add v4 artifact actions
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
- The repo currently keeps default CI packaging-only. Rust quality checks and benchmark/smoke jobs are manual unless a user explicitly asks to add them back.
- Use `macos-15-intel` for x86_64 macOS builds and `macos-latest` for arm64 unless a workflow needs a narrower label.
- Keep package artifact names and release asset names aligned across CI, deploy, and release workflows.

## References
- [Validation commands](./references/validation.md)
