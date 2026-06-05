use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, Context, Result};
use rcgen::generate_simple_self_signed;

use crate::config::{
    default_managed_script_command, GatewayConfig, DEFAULT_CONFIG_FILE_NAME,
    DEFAULT_SCRIPT_FILE_NAME,
};

const SERVICE_NAME: &str = "proxysss";
const LAUNCH_AGENT_LABEL: &str = "com.neko233.proxysss";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x00000008;

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

    let script_command = preferred_script_command();
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

pub fn start_background(config: Option<PathBuf>) -> Result<()> {
    let executable = env::current_exe().context("failed to resolve current executable")?;
    let config_path = resolve_run_config_path(config)?;
    let working_dir = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    stop_processes(Some(config_path.clone()))?;

    let mut command = Command::new(&executable);
    command
        .args(["run", "--config", &config_path.display().to_string()])
        .current_dir(&working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_detached_process(&mut command);

    let child = command
        .spawn()
        .with_context(|| format!("failed to start {}", executable.display()))?;
    write_pid_file(&config_path, child.id())?;
    println!(
        "proxysss started in background with pid {} using {}",
        child.id(),
        config_path.display()
    );
    Ok(())
}

pub fn stop_background(config: Option<PathBuf>) -> Result<()> {
    let config_path = resolve_run_config_path(config)?;
    stop_processes(Some(config_path))
}

pub fn restart_background(config: Option<PathBuf>) -> Result<()> {
    let config_path = resolve_run_config_path(config)?;
    stop_processes(Some(config_path.clone()))?;
    start_background(Some(config_path))
}

pub fn background_status(config: Option<PathBuf>) -> Result<()> {
    let config_path = resolve_run_config_path(config)?;
    let pid_from_file = read_pid_file(&config_path)?;
    let mut pids = other_proxysss_pids(std::process::id())?;
    if let Some(pid) = pid_from_file {
        if !pids.contains(&pid) {
            pids.insert(0, pid);
        }
    }

    if pids.is_empty() {
        println!("proxysss background status: stopped");
    } else {
        println!("proxysss background status: running");
        for pid in pids {
            println!(" - pid={pid}");
        }
    }
    println!("config: {}", config_path.display());
    Ok(())
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

fn stop_processes(config_path: Option<PathBuf>) -> Result<()> {
    let current_pid = std::process::id();
    let mut stopped = false;

    if let Some(config_path) = &config_path {
        if let Some(pid) = read_pid_file(config_path)? {
            if pid != current_pid && kill_pid(pid)? {
                stopped = true;
            }
        }
        remove_pid_file(config_path)?;
    }

    let other_pids = other_proxysss_pids(current_pid)?;
    for pid in other_pids {
        if kill_pid(pid)? {
            stopped = true;
        }
    }

    if stopped {
        println!("stopped existing proxysss background/foreground processes");
    } else {
        println!("no existing proxysss process found");
    }

    Ok(())
}

fn pid_file_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("proxysss.pid")
}

fn write_pid_file(config_path: &Path, pid: u32) -> Result<()> {
    fs::write(pid_file_path(config_path), pid.to_string())
        .with_context(|| format!("failed to write pid file for {}", config_path.display()))
}

fn read_pid_file(config_path: &Path) -> Result<Option<u32>> {
    let pid_path = pid_file_path(config_path);
    if !pid_path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&pid_path)
        .with_context(|| format!("failed to read {}", pid_path.display()))?;
    Ok(text.trim().parse::<u32>().ok())
}

fn remove_pid_file(config_path: &Path) -> Result<()> {
    let pid_path = pid_file_path(config_path);
    if pid_path.exists() {
        fs::remove_file(&pid_path)
            .with_context(|| format!("failed to remove {}", pid_path.display()))?;
    }
    Ok(())
}

fn other_proxysss_pids(current_pid: u32) -> Result<Vec<u32>> {
    match env::consts::OS {
        "windows" => windows_proxysss_pids(current_pid),
        "linux" | "macos" => unix_proxysss_pids(current_pid),
        os => Err(anyhow!("unsupported process inspection os {os}")),
    }
}

fn kill_pid(pid: u32) -> Result<bool> {
    match env::consts::OS {
        "windows" => {
            let output = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .output()
                .with_context(|| format!("failed to stop pid {pid}"))?;
            Ok(output.status.success())
        }
        "linux" | "macos" => {
            let output = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .output()
                .with_context(|| format!("failed to stop pid {pid}"))?;
            Ok(output.status.success())
        }
        os => Err(anyhow!("unsupported process terminate os {os}")),
    }
}

fn unix_proxysss_pids(current_pid: u32) -> Result<Vec<u32>> {
    let output = Command::new("pgrep")
        .args(["-x", SERVICE_NAME])
        .output()
        .context("failed to list proxysss processes via pgrep")?;
    if !output.status.success() {
        return Ok(Vec::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .filter(|pid| *pid != current_pid)
        .collect())
}

fn windows_proxysss_pids(current_pid: u32) -> Result<Vec<u32>> {
    let script = format!(
        "Get-CimInstance Win32_Process | Where-Object {{ $_.Name -eq 'proxysss.exe' -and $_.ProcessId -ne {} }} | Select-Object -ExpandProperty ProcessId",
        current_pid
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .context("failed to list proxysss processes via PowerShell")?;
    if !output.status.success() {
        return Ok(Vec::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .collect())
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

#[cfg(windows)]
fn configure_detached_process(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    command.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
}

#[cfg(unix)]
fn configure_detached_process(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(any(unix, windows)))]
fn configure_detached_process(_command: &mut Command) {}

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
    let path = match dir {
        Some(path) => path,
        None => default_config_dir()?,
    };
    let path = if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .context("failed to resolve current directory")?
            .join(path)
    };
    Ok(normalize_path_lexically(&path))
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

pub fn preferred_script_command() -> String {
    default_managed_script_command()
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

fn normalize_path_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

const DEFAULT_GATEWAY_SCRIPT: &str = include_str!("../templates/gateway.ts");
const DEFAULT_PLUGIN_PLAYER_AFFINITY: &str =
    include_str!("../templates/plugins/player-affinity.ts");
const DEFAULT_PLUGIN_TRAFFIC_STATS: &str = include_str!("../templates/plugins/traffic-stats.ts");
const DEFAULT_PLUGIN_STRUCTURED_LOG: &str = include_str!("../templates/plugins/structured-log.ts");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_base_dir_returns_absolute_path_for_relative_input() {
        let path = resolve_base_dir(Some(PathBuf::from("target/proxysss-init-test")))
            .expect("resolve base dir");
        assert!(path.is_absolute());
        assert!(path.ends_with(Path::new("target").join("proxysss-init-test")));
    }

    #[test]
    fn preferred_script_command_matches_managed_runtime_path() {
        assert_eq!(preferred_script_command(), default_managed_script_command());
    }

    #[test]
    fn pid_file_path_lives_next_to_config() {
        let config = PathBuf::from("/tmp/example/proxysss.yaml");
        assert_eq!(pid_file_path(&config), PathBuf::from("/tmp/example/proxysss.pid"));
    }
}
