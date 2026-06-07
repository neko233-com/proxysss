# Backend Services Lab (`D:\Server\proxysss_dir`)

This folder is **only for services you reverse-proxy through proxysss** (echo backends, static assets, WebDAV content). Do **not** put `proxysss.exe` or `proxysss.yaml` here.

## Install proxysss (gateway)

Use the installer CLI; binary and config stay in default paths:

- Binary: `%LOCALAPPDATA%\proxysss\proxysss.exe`
- Config: `%APPDATA%\proxysss\proxysss.yaml`

```powershell
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action update -Version latest
```

Merge the route snippet from `examples/lab-proxysss/` into the main `%APPDATA%\proxysss\proxysss.yaml` file.

## Deploy backends to `D:\Server\proxysss_dir`

```powershell
Copy-Item -Recurse -Force .\examples\lab\* D:\Server\proxysss_dir\
cd D:\Server\proxysss_dir
.\start-services.ps1
```

## Backends started

| Service | Listen | Used by proxysss |
| --- | --- | --- |
| HTTP echo | `127.0.0.1:8081` | `services.reverse_proxy` → `/echo` |
| TCP echo | `127.0.0.1:7001` | `tcp.listeners` → script upstream |
| UDP echo | `127.0.0.1:8101` | `udp.listeners` → script upstream |
| FTP passthrough target | `127.0.0.1:2121` | `services.ftp.upstream` |
| Static files | `./public` | `services.static_sites` root |
| WebDAV files | `./webdav` | `services.webdav` root |

`start-services.ps1` uses `proxysss` from PATH **only** to run `demo *-echo` helpers.

## Stop

```powershell
.\stop-services.ps1
```
