# proxysss routes for `D:\Server\proxysss_dir` backends

proxysss itself is installed via `install.ps1` into default paths. This folder provides an **explicit include** that points the gateway at services under `D:\Server\proxysss_dir`.

## Setup

1. Install/update proxysss (default paths):

```powershell
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action update -Version latest
```

2. Copy include file:

```powershell
$configRoot = Join-Path $env:APPDATA "proxysss"
New-Item -ItemType Directory -Force -Path (Join-Path $configRoot "conf.d") | Out-Null
Copy-Item .\examples\lab-proxysss\lab-backends.include.yaml (Join-Path $configRoot "conf.d\lab-backends.yaml")
```

3. Enable explicit include in `%APPDATA%\proxysss\proxysss.yaml`:

```yaml
include:
  enabled: true
  required: false
  files:
    - ./conf.d/lab-backends.yaml
```

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

Keep TLS settings in `%APPDATA%\proxysss\proxysss.yaml`. For local HTTPS use `tls.mode: self_signed`. For public domains use `tls.mode: acme_external` — see `acme.example.yaml`.
