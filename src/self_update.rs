//! Native self-update: manual `proxysss update` and an opt-in daily auto-update
//! loop. Everything runs in-process with reqwest/rustls and pure-Rust archive
//! extraction (`tar`/`flate2`/`zip`); there is no dependency on curl, bash,
//! PowerShell or a host tar/bsdtar version, so OS/runner upgrades cannot break
//! the update path.
//!
//! Flow: resolve target version -> pick fastest reachable download source
//! (GitHub origin or accelerator mirrors) -> download -> extract the binary ->
//! verify it actually executes on this host (`--version`) -> atomically replace
//! the running executable -> optionally restart the background/service instance.

use std::io::Read;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

use crate::config::UpdateConfig;

/// Public GitHub release accelerators. Each prefixes the full origin URL, e.g.
/// `https://gh-proxy.com/https://github.com/owner/repo/releases/download/...`.
const BUILTIN_MIRRORS: &[&str] = &[
    "https://gh-proxy.com/",
    "https://ghproxy.net/",
    "https://gh.llkk.cc/",
];

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;

/// Release asset name for the current platform, matching the published matrix.
pub fn asset_name_for(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("linux", "x86_64") => Some("proxysss-linux-amd64.tar.gz"),
        ("linux", "aarch64") => Some("proxysss-linux-arm64.tar.gz"),
        ("macos", "aarch64") => Some("proxysss-darwin-arm64.tar.gz"),
        ("windows", "x86_64") => Some("proxysss-windows-amd64.zip"),
        _ => None,
    }
}

pub fn current_asset_name() -> Result<&'static str> {
    asset_name_for(std::env::consts::OS, std::env::consts::ARCH).ok_or_else(|| {
        anyhow!(
            "no prebuilt release asset for {}/{}; build from source",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })
}

/// `v1.3.16` / `1.3.16` -> `1.3.16`.
pub fn normalize_version_tag(tag: &str) -> String {
    tag.trim().trim_start_matches('v').trim().to_string()
}

/// Numeric dot-component comparison; a candidate is newer only if strictly
/// greater than the current version. Non-numeric suffixes compare as 0.
pub fn version_is_newer(candidate: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        normalize_version_tag(v)
            .split('.')
            .map(|part| {
                part.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u64>()
                    .unwrap_or(0)
            })
            .collect()
    };
    let a = parse(candidate);
    let b = parse(current);
    let len = a.len().max(b.len());
    for i in 0..len {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

fn origin_download_url(repo: &str, version: &str, asset: &str) -> String {
    format!("https://github.com/{repo}/releases/download/v{version}/{asset}")
}

fn join_mirror(base: &str, origin_url: &str) -> String {
    let base = base.trim_end_matches('/');
    format!("{base}/{origin_url}")
}

/// Ordered candidate download URLs for a policy. `auto` keeps origin first so a
/// reachable origin is preferred, then accelerators; the latency probe picks the
/// fastest reachable candidate regardless of order.
pub fn mirror_candidates(
    repo: &str,
    version: &str,
    asset: &str,
    policy: &str,
    extra: &[String],
) -> Vec<String> {
    let origin = origin_download_url(repo, version, asset);
    let policy = policy.trim();
    let mut mirrors: Vec<String> = extra
        .iter()
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .collect();
    for builtin in BUILTIN_MIRRORS {
        mirrors.push(builtin.to_string());
    }
    match policy {
        "origin" => vec![origin],
        "mirror" => mirrors.iter().map(|m| join_mirror(m, &origin)).collect(),
        "auto" | "" => {
            let mut out = vec![origin.clone()];
            out.extend(mirrors.iter().map(|m| join_mirror(m, &origin)));
            out
        }
        custom => {
            // A custom base URL is treated as an accelerator prefix; keep origin
            // as a fallback so a bad custom mirror cannot strand the update.
            let mut out = vec![join_mirror(custom, &origin)];
            out.extend(mirrors.iter().map(|m| join_mirror(m, &origin)));
            out.push(origin);
            out
        }
    }
}

fn http_client(timeout: Duration) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("proxysss-self-update/", env!("CARGO_PKG_VERSION")))
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(10))
        .build()
        .context("failed to build HTTP client")
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
}

/// Resolve the latest stable release version. Tries the GitHub API first, then
/// the `releases/latest` redirect (which accelerators can also proxy).
pub async fn fetch_latest_version(repo: &str, policy: &str, extra: &[String]) -> Result<String> {
    let client = http_client(PROBE_TIMEOUT)?;
    let api = format!("https://api.github.com/repos/{repo}/releases/latest");
    if let Ok(resp) = client
        .get(&api)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(release) = resp.json::<GithubRelease>().await {
                let v = normalize_version_tag(&release.tag_name);
                if !v.is_empty() {
                    return Ok(v);
                }
            }
        }
    }

    // Fallback: read the redirect Location from releases/latest.
    let no_redirect = reqwest::Client::builder()
        .user_agent(concat!("proxysss-self-update/", env!("CARGO_PKG_VERSION")))
        .timeout(PROBE_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("failed to build HTTP client")?;
    let latest = format!("https://github.com/{repo}/releases/latest");
    let mut candidates = vec![latest.clone()];
    if policy != "origin" {
        for m in extra
            .iter()
            .map(|s| s.as_str())
            .chain(BUILTIN_MIRRORS.iter().copied())
        {
            candidates.push(join_mirror(m, &latest));
        }
    }
    for url in candidates {
        if let Ok(resp) = no_redirect.get(&url).send().await {
            if let Some(loc) = resp.headers().get(reqwest::header::LOCATION) {
                if let Ok(loc) = loc.to_str() {
                    if let Some(tag) = loc.rsplit("/tag/").next() {
                        let v = normalize_version_tag(tag);
                        if !v.is_empty() {
                            return Ok(v);
                        }
                    }
                }
            }
        }
    }
    Err(anyhow!(
        "could not resolve the latest release version for {repo}"
    ))
}

/// Probe candidates and return the fastest reachable download URL. A probe is a
/// bounded ranged GET; only response headers are timed, the body is discarded.
pub async fn select_download_url(candidates: &[String]) -> Result<String> {
    if candidates.is_empty() {
        return Err(anyhow!("no download candidates"));
    }
    let client = http_client(PROBE_TIMEOUT)?;
    let mut best: Option<(Duration, String)> = None;
    for url in candidates {
        let start = Instant::now();
        let probe = client
            .get(url)
            .header(reqwest::header::RANGE, "bytes=0-0")
            .send()
            .await;
        match probe {
            Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 206 => {
                let elapsed = start.elapsed();
                tracing::debug!(url = %url, ms = elapsed.as_millis(), "update mirror reachable");
                if best.as_ref().map(|(d, _)| elapsed < *d).unwrap_or(true) {
                    best = Some((elapsed, url.clone()));
                }
            }
            Ok(resp) => {
                tracing::debug!(url = %url, status = %resp.status(), "update mirror rejected probe");
            }
            Err(err) => {
                tracing::debug!(url = %url, error = %err, "update mirror unreachable");
            }
        }
    }
    best.map(|(_, url)| url)
        .ok_or_else(|| anyhow!("no reachable download source (origin and mirrors all failed)"))
}

pub async fn download_asset(url: &str) -> Result<Vec<u8>> {
    let client = http_client(DOWNLOAD_TIMEOUT)?;
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("download failed: {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("download {url} returned {}", resp.status()));
    }
    if let Some(len) = resp.content_length() {
        if len > MAX_ARCHIVE_BYTES {
            return Err(anyhow!("release archive too large: {len} bytes"));
        }
    }
    let bytes = resp
        .bytes()
        .await
        .context("failed to read release archive")?;
    if bytes.len() as u64 > MAX_ARCHIVE_BYTES {
        return Err(anyhow!("release archive too large: {} bytes", bytes.len()));
    }
    Ok(bytes.to_vec())
}

/// Extract the gateway binary from a release archive (`.tar.gz` or `.zip`).
pub fn extract_binary(archive: &[u8], asset: &str) -> Result<Vec<u8>> {
    let want = if asset.ends_with(".zip") {
        "proxysss.exe"
    } else {
        "proxysss"
    };
    if asset.ends_with(".zip") {
        let cursor = std::io::Cursor::new(archive);
        let mut zip = zip::ZipArchive::new(cursor).context("invalid release ZIP")?;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i).context("read ZIP entry")?;
            let name = file.name().trim_start_matches("./").to_string();
            if name == want || name.ends_with(&format!("/{want}")) {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf).context("read ZIP binary")?;
                return Ok(buf);
            }
        }
        Err(anyhow!("{want} not found in release ZIP"))
    } else {
        let gz = flate2::read::GzDecoder::new(archive);
        let mut tar = tar::Archive::new(gz);
        for entry in tar.entries().context("invalid release tar.gz")? {
            let mut entry = entry.context("read tar entry")?;
            let path = entry.path().context("tar entry path")?.to_path_buf();
            let name = path.to_string_lossy().trim_start_matches("./").to_string();
            if name == want || name.ends_with(&format!("/{want}")) {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf).context("read tar binary")?;
                return Ok(buf);
            }
        }
        Err(anyhow!("{want} not found in release tar.gz"))
    }
}

/// Write the candidate binary to a temp file and confirm it executes on this
/// host and reports the expected version. Guards against corrupt downloads and
/// wrong-architecture/libc artifacts before touching the live executable.
pub fn verify_binary(bytes: &[u8], expected_version: &str) -> Result<()> {
    let dir = std::env::temp_dir();
    let name = if cfg!(windows) {
        format!("proxysss-verify-{}.exe", std::process::id())
    } else {
        format!("proxysss-verify-{}", std::process::id())
    };
    let path = dir.join(name);
    std::fs::write(&path, bytes).with_context(|| format!("write {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .with_context(|| format!("chmod {}", path.display()))?;
    }
    let output = std::process::Command::new(&path)
        .arg("--version")
        .output()
        .with_context(|| format!("failed to execute candidate binary {}", path.display()))?;
    let _ = std::fs::remove_file(&path);
    if !output.status.success() {
        return Err(anyhow!(
            "candidate binary failed to run: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.contains(expected_version) {
        return Err(anyhow!(
            "candidate binary version mismatch: expected {expected_version}, got {}",
            stdout.trim()
        ));
    }
    Ok(())
}

/// Atomically replace the current executable with new bytes. On Unix a rename
/// over the running inode is safe; on Windows the running image is renamed aside
/// first, then the new image is written to the original path.
pub fn apply_binary(bytes: &[u8]) -> Result<PathBuf> {
    let exe = std::env::current_exe().context("failed to resolve current executable")?;
    let parent = exe
        .parent()
        .ok_or_else(|| anyhow!("executable has no parent directory"))?;
    let pid = std::process::id();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let staging = parent.join(format!(".proxysss.new-{pid}"));
        std::fs::write(&staging, bytes).with_context(|| format!("write {}", staging.display()))?;
        std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755))
            .with_context(|| format!("chmod {}", staging.display()))?;
        // fsync the staged file before the atomic rename.
        let file = std::fs::File::open(&staging).context("open staged binary")?;
        file.sync_all().context("fsync staged binary")?;
        drop(file);
        std::fs::rename(&staging, &exe)
            .with_context(|| format!("rename over {}", exe.display()))?;
        Ok(exe)
    }

    #[cfg(windows)]
    {
        let old = parent.join(format!("proxysss.exe.old-{pid}"));
        // A running Windows image can be renamed but not overwritten in place.
        if old.exists() {
            let _ = std::fs::remove_file(&old);
        }
        std::fs::rename(&exe, &old)
            .with_context(|| format!("rename running image to {}", old.display()))?;
        match std::fs::write(&exe, bytes) {
            Ok(()) => {
                let _ = std::fs::remove_file(&old);
                Ok(exe)
            }
            Err(err) => {
                // Roll back so the original executable is never lost.
                let _ = std::fs::rename(&old, &exe);
                Err(anyhow!("write new executable failed: {err}"))
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = (parent, pid, bytes);
        Err(anyhow!("self-update is unsupported on this platform"))
    }
}

/// Best-effort restart of a running background/service instance after apply.
pub fn restart_running_instance(config_path: Option<PathBuf>) -> Result<()> {
    match std::env::consts::OS {
        "linux" => {
            let active = std::process::Command::new("systemctl")
                .args(["--user", "is-active", "--quiet", "proxysss.service"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if active {
                let status = std::process::Command::new("systemctl")
                    .args(["--user", "restart", "proxysss.service"])
                    .status()
                    .context("restart systemd user service")?;
                if status.success() {
                    println!("restarted proxysss.service");
                    return Ok(());
                }
            }
            crate::install::restart_background(config_path)
        }
        "macos" => {
            let plist =
                dirs::home_dir().map(|h| h.join("Library/LaunchAgents/com.neko233.proxysss.plist"));
            if let Some(plist) = plist {
                if plist.exists() {
                    let _ = std::process::Command::new("launchctl")
                        .args(["unload", &plist.to_string_lossy()])
                        .status();
                    let status = std::process::Command::new("launchctl")
                        .args(["load", "-w", &plist.to_string_lossy()])
                        .status()
                        .context("reload launch agent")?;
                    if status.success() {
                        println!("reloaded proxysss launch agent");
                        return Ok(());
                    }
                }
            }
            crate::install::restart_background(config_path)
        }
        "windows" => crate::install::restart_background(config_path),
        os => Err(anyhow!("unsupported restart os {os}")),
    }
}

/// Options for a manual update invocation.
pub struct UpdateOptions {
    pub version: String,
    pub allow_downgrade: bool,
    pub check_only: bool,
    pub restart: bool,
    pub mirror: Option<String>,
    pub config_path: Option<PathBuf>,
}

/// Orchestrate a manual update. Returns the version that was applied (or the
/// latest version when `check_only`).
pub async fn run_update(config: &UpdateConfig, opts: UpdateOptions) -> Result<String> {
    let asset = current_asset_name()?;
    let current = env!("CARGO_PKG_VERSION");
    let policy = opts.mirror.as_deref().unwrap_or(&config.mirror);

    let target = if opts.version.eq_ignore_ascii_case("latest") {
        fetch_latest_version(&config.repo, policy, &config.mirrors).await?
    } else {
        normalize_version_tag(&opts.version)
    };
    if target.is_empty() {
        return Err(anyhow!("empty target version"));
    }

    if opts.check_only {
        println!("current: {current}");
        println!("latest:  {target}");
        if version_is_newer(&target, current) {
            println!("update available");
        } else {
            println!("up to date");
        }
        return Ok(target);
    }

    if !opts.allow_downgrade && !version_is_newer(&target, current) {
        println!("proxysss {current} is up to date (latest {target})");
        return Ok(target);
    }

    let candidates = mirror_candidates(&config.repo, &target, asset, policy, &config.mirrors);
    let url = select_download_url(&candidates).await?;
    println!("downloading {asset} v{target} from {url}");
    let archive = download_asset(&url).await?;
    let binary = extract_binary(&archive, asset)?;
    verify_binary(&binary, &target)?;
    let exe = apply_binary(&binary)?;
    println!("updated {} to v{target}", exe.display());

    if opts.restart {
        match restart_running_instance(opts.config_path.clone()) {
            Ok(()) => {}
            Err(err) => tracing::warn!(error = %err, "no running instance restarted"),
        }
    }
    Ok(target)
}

/// Parse `HH:MM` into (hour, minute).
pub fn parse_check_time(value: &str) -> Result<(u32, u32)> {
    let (h, m) = value
        .trim()
        .split_once(':')
        .ok_or_else(|| anyhow!("check_time must be HH:MM, got {value:?}"))?;
    let hour: u32 = h.trim().parse().context("invalid check_time hour")?;
    let minute: u32 = m.trim().parse().context("invalid check_time minute")?;
    if hour > 23 || minute > 59 {
        return Err(anyhow!("check_time out of range: {value:?}"));
    }
    Ok((hour, minute))
}

/// Seconds from `now` until the next local occurrence of `hour:minute`.
pub fn seconds_until_next_check(now: time::OffsetDateTime, hour: u32, minute: u32) -> u64 {
    let today = now
        .date()
        .with_hms(hour as u8, minute as u8, 0)
        .expect("valid check time")
        .assume_offset(now.offset());
    let target = if today > now {
        today
    } else {
        today + time::Duration::days(1)
    };
    let delta = target - now;
    delta.whole_seconds().max(1) as u64
}

fn now_local() -> time::OffsetDateTime {
    time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc())
}

/// Spawn the opt-in daily auto-update loop. Runs for the process lifetime; the
/// gateway process exit tears it down. `update.auto_update` is restart-required.
pub fn spawn_auto_update(config: UpdateConfig, config_path: Option<PathBuf>) {
    if !config.auto_update {
        return;
    }
    let (hour, minute) = match parse_check_time(&config.check_time) {
        Ok(v) => v,
        Err(err) => {
            tracing::error!(error = %err, "auto-update disabled: invalid check_time");
            return;
        }
    };
    tracing::info!(
        check_time = %config.check_time,
        mirror = %config.mirror,
        auto_apply = config.auto_apply,
        "auto-update enabled"
    );
    tokio::spawn(async move {
        loop {
            let wait = seconds_until_next_check(now_local(), hour, minute);
            tracing::debug!(wait_secs = wait, "auto-update sleeping until next check");
            tokio::time::sleep(Duration::from_secs(wait)).await;

            let opts = UpdateOptions {
                version: "latest".to_string(),
                allow_downgrade: false,
                check_only: !config.auto_apply,
                restart: config.restart_after_apply,
                mirror: None,
                config_path: config_path.clone(),
            };
            match run_update(&config, opts).await {
                Ok(version) => {
                    if config.auto_apply {
                        tracing::info!(version = %version, "auto-update check complete");
                    } else {
                        tracing::info!(version = %version, "auto-update available (auto_apply off)");
                    }
                }
                Err(err) => tracing::warn!(error = %err, "auto-update check failed"),
            }
            // Avoid a tight loop if the clock jumps backward across the boundary.
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_matrix_matches_published_names() {
        assert_eq!(
            asset_name_for("linux", "x86_64"),
            Some("proxysss-linux-amd64.tar.gz")
        );
        assert_eq!(
            asset_name_for("linux", "aarch64"),
            Some("proxysss-linux-arm64.tar.gz")
        );
        assert_eq!(
            asset_name_for("macos", "aarch64"),
            Some("proxysss-darwin-arm64.tar.gz")
        );
        assert_eq!(
            asset_name_for("windows", "x86_64"),
            Some("proxysss-windows-amd64.zip")
        );
        assert_eq!(asset_name_for("macos", "x86_64"), None);
        assert_eq!(asset_name_for("windows", "aarch64"), None);
    }

    #[test]
    fn version_comparison_is_strict_and_numeric() {
        assert!(version_is_newer("1.3.17", "1.3.16"));
        assert!(version_is_newer("v1.4.0", "1.3.16"));
        assert!(version_is_newer("1.3.16", "1.3.9"));
        assert!(!version_is_newer("1.3.16", "1.3.16"));
        assert!(!version_is_newer("1.3.15", "1.3.16"));
        assert!(!version_is_newer("1.3", "1.3.0"));
    }

    #[test]
    fn normalize_strips_v_and_whitespace() {
        assert_eq!(normalize_version_tag(" v1.3.16 "), "1.3.16");
        assert_eq!(normalize_version_tag("1.3.16"), "1.3.16");
    }

    #[test]
    fn mirror_policy_ordering() {
        let origin = "https://github.com/neko233-com/proxysss/releases/download/v1.3.17/proxysss-linux-amd64.tar.gz";
        let auto = mirror_candidates(
            "neko233-com/proxysss",
            "1.3.17",
            "proxysss-linux-amd64.tar.gz",
            "auto",
            &[],
        );
        assert_eq!(auto[0], origin);
        assert!(auto
            .iter()
            .any(|u| u.starts_with("https://gh-proxy.com/https://github.com/")));
        let only_origin = mirror_candidates(
            "neko233-com/proxysss",
            "1.3.17",
            "proxysss-linux-amd64.tar.gz",
            "origin",
            &[],
        );
        assert_eq!(only_origin, vec![origin.to_string()]);
        let only_mirror = mirror_candidates(
            "neko233-com/proxysss",
            "1.3.17",
            "proxysss-linux-amd64.tar.gz",
            "mirror",
            &[],
        );
        assert!(!only_mirror.contains(&origin.to_string()));
        assert!(only_mirror
            .iter()
            .all(|u| u.contains("https://github.com/")));
        let custom = mirror_candidates(
            "neko233-com/proxysss",
            "1.3.17",
            "proxysss-linux-amd64.tar.gz",
            "https://my.mirror/",
            &[],
        );
        assert!(custom[0].starts_with("https://my.mirror/https://github.com/"));
        assert_eq!(custom.last().unwrap(), origin);
    }

    #[test]
    fn extra_mirrors_are_prepended() {
        let c = mirror_candidates(
            "r/p",
            "1.0.0",
            "a.tar.gz",
            "auto",
            &["https://x.dev/".to_string()],
        );
        assert_eq!(
            c[1],
            "https://x.dev/https://github.com/r/p/releases/download/v1.0.0/a.tar.gz"
        );
    }

    #[test]
    fn check_time_parsing() {
        assert_eq!(parse_check_time("04:00").unwrap(), (4, 0));
        assert_eq!(parse_check_time("23:59").unwrap(), (23, 59));
        assert!(parse_check_time("24:00").is_err());
        assert!(parse_check_time("04:60").is_err());
        assert!(parse_check_time("4").is_err());
    }

    #[test]
    fn next_check_rolls_to_tomorrow() {
        use time::macros::datetime;
        let now = datetime!(2026-11-23 05:00:00 UTC);
        // 04:00 already passed today -> tomorrow 04:00 = 23h.
        assert_eq!(seconds_until_next_check(now, 4, 0), 23 * 3600);
        let now2 = datetime!(2026-11-23 03:00:00 UTC);
        // 04:00 still ahead today -> 1h.
        assert_eq!(seconds_until_next_check(now2, 4, 0), 3600);
        let now3 = datetime!(2026-11-23 04:00:00 UTC);
        // exactly at boundary -> tomorrow.
        assert_eq!(seconds_until_next_check(now3, 4, 0), 24 * 3600);
    }

    fn make_targz(binary: &[u8]) -> Vec<u8> {
        let mut tar_builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(binary.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        tar_builder
            .append_data(&mut header, "proxysss", binary)
            .unwrap();
        let tar_bytes = tar_builder.into_inner().unwrap();
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut gz, &tar_bytes).unwrap();
        gz.finish().unwrap()
    }

    fn make_zip(binary: &[u8]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts: zip::write::FileOptions<'_, ()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("proxysss.exe", opts).unwrap();
        std::io::Write::write_all(&mut zip, binary).unwrap();
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn extract_tar_gz_round_trip() {
        let payload = b"\x7fELF fake binary bytes";
        let archive = make_targz(payload);
        let out = extract_binary(&archive, "proxysss-linux-amd64.tar.gz").unwrap();
        assert_eq!(out, payload);
    }

    #[test]
    fn extract_zip_round_trip() {
        let payload = b"MZ fake windows binary";
        let archive = make_zip(payload);
        let out = extract_binary(&archive, "proxysss-windows-amd64.zip").unwrap();
        assert_eq!(out, payload);
    }

    #[test]
    fn extract_missing_binary_errors() {
        // build a tar whose only entry is not the gateway binary
        let mut b = tar::Builder::new(Vec::new());
        let mut h = tar::Header::new_gnu();
        h.set_size(1);
        h.set_mode(0o644);
        h.set_cksum();
        b.append_data(&mut h, "README.md", &b"x"[..]).unwrap();
        let raw = b.into_inner().unwrap();
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut gz, &raw).unwrap();
        let archive = gz.finish().unwrap();
        assert!(extract_binary(&archive, "proxysss-linux-amd64.tar.gz").is_err());
    }
}
