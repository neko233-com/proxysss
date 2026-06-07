# proxysss routes for `D:\Server\proxysss_dir` backends

proxysss itself is installed via `install.ps1` into default paths. Keep the runtime configuration in a single `%APPDATA%\proxysss\proxysss.yaml` file and copy the route snippets from this folder into that file.

## Setup

1. Install/update proxysss (default paths):

```powershell
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action update -Version latest
```

2. Copy the route snippet into your main config:

```powershell
$configRoot = Join-Path $env:APPDATA "proxysss"
Copy-Item .\examples\lab-proxysss\lab-backends.include.yaml (Join-Path $configRoot "lab-backends.yaml")
```

3. Merge the YAML from `lab-backends.yaml` into `%APPDATA%\proxysss\proxysss.yaml` under the same top-level sections:

This keeps the gateway configuration in one file, which is the recommended and supported model.

4. Start backends (not proxysss):

```powershell
cd D:\Server\proxysss_dir
.\start-services.ps1
```

5. Start gateway:

```powershell
proxysss run
# or: proxysss start
```

6. Verify:

```powershell
.\examples\lab-proxysss\verify-proxy.ps1
```

## HTTPS / ACME

Keep TLS settings in `%APPDATA%\proxysss\proxysss.yaml`. For local HTTPS use `tls.mode: self_signed`. For public domains merge the snippet from `acme.example.yaml` into the same main YAML file.
