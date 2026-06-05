---
name: gh-cli
description: Monitor and operate GitHub Actions for the proxysss repository with the GitHub CLI. Use when checking CI/release workflow status, reading failed job logs, rerunning workflows, inspecting release assets, or when the user asks to watch GitHub Actions with gh.
---

# GitHub CLI Actions Monitor

## Prerequisites

1. Confirm `gh` is installed: `gh --version`
2. Confirm auth: `gh auth status`
3. If not authenticated, stop and ask the user to run `gh auth login` (repo + workflow scopes).

Default repository: `neko233-com/proxysss`

## Quick Status

```bash
gh repo view neko233-com/proxysss --json name,defaultBranchRef
gh workflow list -R neko233-com/proxysss
gh run list -R neko233-com/proxysss --limit 10
gh release list -R neko233-com/proxysss --limit 5
```

## Workflow Map

| Workflow | File | Purpose |
| --- | --- | --- |
| `ci` | `.github/workflows/ci.yml` | actionlint, rustfmt, clippy, tests, multi-OS build |
| `deploy` | `.github/workflows/deploy.yml` | package bundles |
| `release` | `.github/workflows/release.yml` | tag builds + GitHub Release assets |

## Investigate Failures

```bash
# Latest failed runs
gh run list -R neko233-com/proxysss --status failure --limit 5

# Inspect one run
gh run view <run-id> -R neko233-com/proxysss

# Failed step logs only
gh run view <run-id> -R neko233-com/proxysss --log-failed

# Watch an in-progress run
gh run watch <run-id> -R neko233-com/proxysss --exit-status
```

Filter by workflow:

```bash
gh run list -R neko233-com/proxysss --workflow=ci.yml --limit 5
gh run list -R neko233-com/proxysss --workflow=release.yml --limit 5
```

## Release Monitoring

After pushing a `v*` tag:

```bash
gh run list -R neko233-com/proxysss --workflow=release.yml --limit 3
gh release view vX.Y.Z -R neko233-com/proxysss
```

Verify all six assets exist:

- `proxysss-windows-amd64.exe`
- `proxysss-windows-arm64.exe`
- `proxysss-linux-amd64`
- `proxysss-linux-arm64`
- `proxysss-darwin-amd64`
- `proxysss-darwin-arm64`

If `publish` fails on `Extract current changelog`, add a `## vX.Y.Z` section to `CHANGELOG.md` and re-tag or rerun publish.

## Rerun And Repair

```bash
gh run rerun <run-id> -R neko233-com/proxysss --failed
gh workflow run release.yml -R neko233-com/proxysss --ref main
```

## Agent Handoff Format

When reporting Actions status to the user, include:

- Workflow name and run URL
- Conclusion (`success`, `failure`, `in_progress`)
- Failed job names (from `gh run view`)
- One-line root cause from `--log-failed`
- Next fix or rerun command

## Related Skills

- `.github/skills/github-actions/SKILL.md` — edit and validate workflow YAML locally
- `skills/proxysss-install/SKILL.md` — install released binaries after a green release run
