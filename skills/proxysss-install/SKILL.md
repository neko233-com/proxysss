---
name: proxysss-install
description: Install, update, initialize, verify, and hand over proxysss to an AI agent. Use when Codex is asked to one-click install proxysss, replace nginx with proxysss, bootstrap proxysss config/service files, verify default ports 80 and 7777, or prepare proxysss so another agent can inspect and operate it.
---

# Proxysss Install

## Workflow

Use the repository scripts when working inside a proxysss checkout. Use the remote install command when no checkout exists.

1. Detect OS and shell.
2. Install or update proxysss.
3. Initialize config unless the user explicitly wants to preserve an existing config.
4. Run `proxysss check-config`.
5. Show the agent handoff summary with default URLs and useful inspect commands.

## Install Commands

Windows PowerShell:

```powershell
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action install -Version latest
```

Linux/macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh | bash
```

From a local checkout:

```bash
cargo build --release
./target/release/proxysss init
./target/release/proxysss check-config --config ./proxysss.yaml
```

On Windows local checkout:

```powershell
cargo build --release
.\target\release\proxysss.exe init
.\target\release\proxysss.exe check-config --config .\proxysss.yaml
```

## Verification

After install, prefer these commands:

```bash
proxysss config explain
proxysss config capabilities
proxysss config watched-scripts
proxysss config routes
proxysss config reload-plan
proxysss config nginx-parity --format yaml
proxysss service status
```

Expected defaults:

- Public welcome page: `http://127.0.0.1/`
- Admin console: `http://127.0.0.1:7777/`
- Admin credentials in fresh dev config: `root` / `root`
- Default demo plugins: `plugins/structured-log.ts`, `plugins/traffic-stats.ts`, and `plugins/player-affinity.ts`

If port 80 requires elevation, report that clearly and either run the service installer or ask for elevated execution.

## Agent Handoff

End with:

- The binary path if known.
- The config path.
- The welcome URL.
- The admin URL.
- Any warnings from `check-config`.
- Watched scripts from `proxysss config watched-scripts`.
- Route topology from `proxysss config routes`.
- Hot-reload boundaries from `proxysss config reload-plan`.
- Nginx parity matrix from `proxysss config nginx-parity --format yaml`.
- The next command the operator should run to start or inspect proxysss.
