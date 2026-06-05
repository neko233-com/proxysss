use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use rcgen::generate_simple_self_signed;

use crate::config::{GatewayConfig, DEFAULT_CONFIG_FILE_NAME, DEFAULT_SCRIPT_FILE_NAME};

const SERVICE_NAME: &str = "proxysss";
const LAUNCH_AGENT_LABEL: &str = "com.neko233.proxysss";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn resolve_run_config_path(config: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = config {
        return Ok(path);
    }

    let local = PathBuf::from(DEFAULT_CONFIG_FILE_NAME);
    if local.exists() {
        return Ok(local);
    }

    let default = default_config_dir()?.join(DEFAULT_CONFIG_FILE_NAME);
    if default.exists() {
        return Ok(default);
    }

    Ok(local)
}

pub fn init_layout(dir: Option<PathBuf>, overwrite: bool) -> Result<()> {
    let base_dir = resolve_base_dir(dir)?;
    let cert_dir = base_dir.join("certs");
    let plugin_dir = base_dir.join("plugins");
    let config_path = base_dir.join(DEFAULT_CONFIG_FILE_NAME);
    let script_path = base_dir.join(DEFAULT_SCRIPT_FILE_NAME);
    let plugin_path = plugin_dir.join("player-affinity.ts");
    let traffic_stats_plugin_path = plugin_dir.join("traffic-stats.ts");
    let structured_log_plugin_path = plugin_dir.join("structured-log.ts");

    fs::create_dir_all(&cert_dir)
        .with_context(|| format!("failed to create {}", cert_dir.display()))?;
    fs::create_dir_all(&plugin_dir)
        .with_context(|| format!("failed to create {}", plugin_dir.display()))?;

    let script_command = detect_script_command().unwrap_or_else(|| "deno".to_string());
    let config_yaml = GatewayConfig::render_default_yaml(&script_command).replace(
        "\n  cwd: .\n",
        &format!(
            "\n  cwd: {}\n",
            base_dir.display().to_string().replace('\\', "/")
        ),
    );
    write_if_needed(&config_path, &config_yaml, overwrite)?;
    write_if_needed(&script_path, DEFAULT_GATEWAY_SCRIPT, overwrite)?;
    write_if_needed(&plugin_path, DEFAULT_PLUGIN_PLAYER_AFFINITY, overwrite)?;
    write_if_needed(
        &traffic_stats_plugin_path,
        DEFAULT_PLUGIN_TRAFFIC_STATS,
        overwrite,
    )?;
    write_if_needed(
        &structured_log_plugin_path,
        DEFAULT_PLUGIN_STRUCTURED_LOG,
        overwrite,
    )?;

    ensure_cert_pair(
        &cert_dir.join("proxysss-cert.pem"),
        &cert_dir.join("proxysss-key.pem"),
        "gateway.local",
        overwrite,
    )?;

    println!("initialized config layout at {}", base_dir.display());
    Ok(())
}

pub fn bootstrap_certs_in_dir(dir: Option<PathBuf>, overwrite: bool) -> Result<()> {
    let base_dir = resolve_base_dir(dir)?;
    let cert_dir = base_dir.join("certs");
    fs::create_dir_all(&cert_dir)
        .with_context(|| format!("failed to create {}", cert_dir.display()))?;
    ensure_cert_pair(
        &cert_dir.join("proxysss-cert.pem"),
        &cert_dir.join("proxysss-key.pem"),
        "gateway.local",
        overwrite,
    )
}

pub fn ensure_cert_pair(
    cert_path: &Path,
    key_path: &Path,
    server_name: &str,
    overwrite: bool,
) -> Result<()> {
    if !overwrite && cert_path.exists() && key_path.exists() {
        return Ok(());
    }

    if let Some(parent) = cert_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let cert = generate_simple_self_signed(vec![
        server_name.to_string(),
        "localhost".to_string(),
        "127.0.0.1".to_string(),
    ])
    .context("failed generating self-signed certificate")?;

    fs::write(cert_path, cert.cert.pem())
        .with_context(|| format!("failed writing {}", cert_path.display()))?;
    fs::write(key_path, cert.key_pair.serialize_pem())
        .with_context(|| format!("failed writing {}", key_path.display()))?;

    println!(
        "generated certificate pair at {} and {}",
        cert_path.display(),
        key_path.display()
    );
    Ok(())
}

pub fn install_service(config: Option<PathBuf>) -> Result<()> {
    let executable = env::current_exe().context("failed to resolve current executable")?;
    let config_path = resolve_run_config_path(config)?;
    let working_dir = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    match env::consts::OS {
        "windows" => install_windows_service(&executable, &config_path),
        "linux" => install_linux_service(&executable, &config_path, &working_dir),
        "macos" => install_macos_service(&executable, &config_path, &working_dir),
        os => Err(anyhow!("unsupported service install os {os}")),
    }
}

pub fn uninstall_service() -> Result<()> {
    match env::consts::OS {
        "windows" => uninstall_windows_service(),
        "linux" => {
            let unit_path = linux_service_path()?;
            if unit_path.exists() {
                fs::remove_file(&unit_path)
                    .with_context(|| format!("failed to remove {}", unit_path.display()))?;
            }
            run_command(
                Command::new("systemctl").args(["--user", "disable", "--now", "proxysss.service"]),
                "disable systemd user service",
            )?;
            run_command(
                Command::new("systemctl").args(["--user", "daemon-reload"]),
                "reload systemd user daemon",
            )
        }
        "macos" => {
            let plist_path = macos_launch_agent_path()?;
            let _ = run_command(
                Command::new("launchctl").args(["unload", plist_path.to_string_lossy().as_ref()]),
                "unload launch agent",
            );
            if plist_path.exists() {
                fs::remove_file(&plist_path)
                    .with_context(|| format!("failed to remove {}", plist_path.display()))?;
            }
            Ok(())
        }
        os => Err(anyhow!("unsupported service uninstall os {os}")),
    }
}

pub fn start_service() -> Result<()> {
    match env::consts::OS {
        "windows" => start_windows_service(),
        "linux" => run_command(
            Command::new("systemctl").args(["--user", "start", "proxysss.service"]),
            "start systemd user service",
        ),
        "macos" => run_command(
            Command::new("launchctl").args([
                "load",
                "-w",
                macos_launch_agent_path()?.to_string_lossy().as_ref(),
            ]),
            "load launch agent",
        ),
        os => Err(anyhow!("unsupported service start os {os}")),
    }
}

pub fn stop_service() -> Result<()> {
    match env::consts::OS {
        "windows" => run_command(
            Command::new("powershell").args([
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'proxysss.exe' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }",
            ]),
            "stop proxysss.exe processes",
        ),
        "linux" => run_command(Command::new("systemctl").args(["--user", "stop", "proxysss.service"]), "stop systemd user service"),
        "macos" => run_command(Command::new("launchctl").args(["unload", macos_launch_agent_path()?.to_string_lossy().as_ref()]), "unload launch agent"),
        os => Err(anyhow!("unsupported service stop os {os}")),
    }
}

pub fn service_status() -> Result<()> {
    match env::consts::OS {
        "windows" => windows_service_status(),
        "linux" => run_command(
            Command::new("systemctl").args(["--user", "status", "proxysss.service", "--no-pager"]),
            "show systemd user service status",
        ),
        "macos" => run_command(
            Command::new("launchctl").args(["list", LAUNCH_AGENT_LABEL]),
            "show launch agent status",
        ),
        os => Err(anyhow!("unsupported service status os {os}")),
    }
}

fn install_windows_service(executable: &Path, config_path: &Path) -> Result<()> {
    let launcher_path = write_windows_hidden_launcher(executable, config_path)?;
    let task_command = format!("wscript.exe //B //Nologo \"{}\"", launcher_path.display());

    if install_windows_run_key(&task_command).is_ok() {
        let _ = run_command(
            Command::new("schtasks").args(["/Delete", "/TN", SERVICE_NAME, "/F"]),
            "delete stale windows scheduled task",
        );
        return start_windows_command(executable, config_path);
    }

    let scheduled_task = run_command(
        Command::new("schtasks").args([
            "/Create",
            "/F",
            "/SC",
            "ONLOGON",
            "/TN",
            SERVICE_NAME,
            "/TR",
            &task_command,
        ]),
        "create windows scheduled task",
    );

    if scheduled_task.is_ok() {
        return start_windows_service();
    }

    Err(anyhow!(
        "failed to install windows auto-start entry using HKCU Run or Scheduled Tasks"
    ))
}

fn uninstall_windows_service() -> Result<()> {
    let mut removed_any = false;

    if run_command(
        Command::new("schtasks").args(["/Delete", "/TN", SERVICE_NAME, "/F"]),
        "delete windows scheduled task",
    )
    .is_ok()
    {
        removed_any = true;
    }

    if delete_windows_run_key().is_ok() {
        removed_any = true;
    }

    if removed_any {
        return Ok(());
    }

    Err(anyhow!("no windows auto-start entry found for proxysss"))
}

fn start_windows_service() -> Result<()> {
    let current_exe = env::current_exe().context("failed to resolve current executable")?;
    let config_path = resolve_run_config_path(None)?;
    start_windows_command(&current_exe, &config_path)
}

fn windows_service_status() -> Result<()> {
    if run_command(
        Command::new("schtasks").args(["/Query", "/TN", SERVICE_NAME]),
        "query windows scheduled task",
    )
    .is_ok()
    {
        return Ok(());
    }

    let output = Command::new("reg")
        .args([
            "query",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "/v",
            SERVICE_NAME,
        ])
        .output()
        .context("failed to query windows run registry entry")?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            println!("startup mode: HKCU Run");
            println!("{stdout}");
        }
        return Ok(());
    }

    Err(anyhow!(
        "query windows auto-start failed: no scheduled task or HKCU Run entry found"
    ))
}

fn install_windows_run_key(task_command: &str) -> Result<()> {
    run_command(
        Command::new("reg").args([
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "/v",
            SERVICE_NAME,
            "/t",
            "REG_SZ",
            "/d",
            task_command,
            "/f",
        ]),
        "create windows HKCU run entry",
    )
}

fn delete_windows_run_key() -> Result<()> {
    run_command(
        Command::new("reg").args([
            "delete",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "/v",
            SERVICE_NAME,
            "/f",
        ]),
        "delete windows HKCU run entry",
    )
}

fn start_windows_command(executable: &Path, config_path: &Path) -> Result<()> {
    let mut command = Command::new(executable);
    command.args(["run", "--config", &config_path.display().to_string()]);
    configure_hidden_windows_process(&mut command);
    command
        .spawn()
        .with_context(|| format!("failed to start {}", executable.display()))?;
    Ok(())
}

#[cfg(windows)]
fn write_windows_hidden_launcher(executable: &Path, config_path: &Path) -> Result<PathBuf> {
    let dir = config_path
        .parent()
        .ok_or_else(|| anyhow!("config path has no parent: {}", config_path.display()))?;
    let launcher_path = dir.join("proxysss-start.vbs");
    let working_dir = dir.display().to_string();
    let run_command = format!(
        "\"{}\" run --config \"{}\"",
        executable.display(),
        config_path.display()
    );
    let script = format!(
        "Set shell = CreateObject(\"WScript.Shell\")\r\nshell.CurrentDirectory = \"{}\"\r\nshell.Run \"{}\", 0, False\r\n",
        vbs_escape(&working_dir),
        vbs_escape(&run_command)
    );
    fs::write(&launcher_path, script)
        .with_context(|| format!("failed writing {}", launcher_path.display()))?;
    Ok(launcher_path)
}

#[cfg(windows)]
fn vbs_escape(value: &str) -> String {
    value.replace('"', "\"\"")
}

#[cfg(not(windows))]
fn write_windows_hidden_launcher(_executable: &Path, _config_path: &Path) -> Result<PathBuf> {
    Err(anyhow!("windows launcher is only available on windows"))
}

#[cfg(windows)]
fn configure_hidden_windows_process(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn configure_hidden_windows_process(_command: &mut Command) {}

fn install_linux_service(executable: &Path, config_path: &Path, working_dir: &Path) -> Result<()> {
    let unit_path = linux_service_path()?;
    if let Some(parent) = unit_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let content = format!(
        "[Unit]\nDescription=proxysss programmable gateway\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=simple\nWorkingDirectory={}\nExecStart={} run --config {}\nRestart=always\nRestartSec=1\nEnvironment=RUST_LOG=info,proxysss=info\n\n[Install]\nWantedBy=default.target\n",
        systemd_quote(working_dir),
        systemd_quote(executable),
        systemd_quote(config_path),
    );

    fs::write(&unit_path, content)
        .with_context(|| format!("failed writing {}", unit_path.display()))?;
    run_command(
        Command::new("systemctl").args(["--user", "daemon-reload"]),
        "reload systemd user daemon",
    )?;
    run_command(
        Command::new("systemctl").args(["--user", "enable", "--now", "proxysss.service"]),
        "enable and start systemd user service",
    )
}

fn install_macos_service(executable: &Path, config_path: &Path, working_dir: &Path) -> Result<()> {
    let plist_path = macos_launch_agent_path()?;
    if let Some(parent) = plist_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let plist = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>Label</key>\n  <string>{label}</string>\n  <key>ProgramArguments</key>\n  <array>\n    <string>{exe}</string>\n    <string>run</string>\n    <string>--config</string>\n    <string>{config}</string>\n  </array>\n  <key>WorkingDirectory</key>\n  <string>{cwd}</string>\n  <key>RunAtLoad</key>\n  <true/>\n  <key>KeepAlive</key>\n  <true/>\n</dict>\n</plist>\n",
        label = xml_escape(LAUNCH_AGENT_LABEL),
        exe = xml_escape(&executable.display().to_string()),
        config = xml_escape(&config_path.display().to_string()),
        cwd = xml_escape(&working_dir.display().to_string()),
    );

    fs::write(&plist_path, plist)
        .with_context(|| format!("failed writing {}", plist_path.display()))?;
    let _ = run_command(
        Command::new("launchctl").args(["unload", plist_path.to_string_lossy().as_ref()]),
        "unload existing launch agent",
    );
    run_command(
        Command::new("launchctl").args(["load", "-w", plist_path.to_string_lossy().as_ref()]),
        "load launch agent",
    )
}

fn resolve_base_dir(dir: Option<PathBuf>) -> Result<PathBuf> {
    match dir {
        Some(path) => Ok(path),
        None => default_config_dir(),
    }
}

fn default_config_dir() -> Result<PathBuf> {
    let dir =
        dirs::config_dir().ok_or_else(|| anyhow!("failed to resolve user config directory"))?;
    Ok(dir.join(SERVICE_NAME))
}

fn write_if_needed(path: &Path, content: &str, overwrite: bool) -> Result<()> {
    if path.exists() && !overwrite {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut file =
        fs::File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    file.write_all(content.as_bytes())
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn detect_script_command() -> Option<String> {
    find_in_path(if cfg!(windows) { "deno.exe" } else { "deno" })
        .map(|path| path.to_string_lossy().to_string())
}

fn find_in_path(binary: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    env::split_paths(&path_var)
        .map(|dir| dir.join(binary))
        .find(|candidate| candidate.exists())
}

fn run_command(command: &mut Command, description: &str) -> Result<()> {
    let output = command
        .output()
        .with_context(|| format!("failed to {description}"))?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            println!("{stdout}");
        }
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(anyhow!("{description} failed: {stderr}"))
}

fn linux_service_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("failed to resolve home directory"))?;
    Ok(home.join(".config/systemd/user/proxysss.service"))
}

fn macos_launch_agent_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("failed to resolve home directory"))?;
    Ok(home.join("Library/LaunchAgents/com.neko233.proxysss.plist"))
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn systemd_quote(path: &Path) -> String {
    format!("\"{}\"", path.display().to_string().replace('"', "\\\""))
}

const DEFAULT_GATEWAY_SCRIPT: &str = include_str!("../templates/gateway.ts");
const DEFAULT_PLUGIN_PLAYER_AFFINITY: &str =
    include_str!("../templates/plugins/player-affinity.ts");
const DEFAULT_PLUGIN_TRAFFIC_STATS: &str = include_str!("../templates/plugins/traffic-stats.ts");
const DEFAULT_PLUGIN_STRUCTURED_LOG: &str = include_str!("../templates/plugins/structured-log.ts");
