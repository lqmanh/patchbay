use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::platform::LaunchdStatus;
use crate::platform::exec::{run, run_checked};

const DAEMON_LABEL: &str = "io.github.lqmanh.patchbay.daemon";
const INTERCEPT_LABEL: &str = "io.github.lqmanh.patchbay.intercept";

pub fn launch_agent_path(home: &Path) -> PathBuf {
    agent_path(home, DAEMON_LABEL)
}

pub fn intercept_agent_path(home: &Path) -> PathBuf {
    agent_path(home, INTERCEPT_LABEL)
}

pub fn build_plist(label: &str, args: &[String], log_path: &Path, keep_alive: bool) -> String {
    let args_xml = args
        .iter()
        .map(|arg| format!("    <string>{}</string>", escape_xml(arg)))
        .collect::<Vec<_>>()
        .join("\n");
    let keep_alive_xml = if keep_alive {
        "  <key>KeepAlive</key>\n  <true/>\n"
    } else {
        ""
    };
    let log = escape_xml(&log_path.display().to_string());

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n\
<dict>\n\
  <key>Label</key>\n\
  <string>{label}</string>\n\
  <key>ProgramArguments</key>\n\
  <array>\n\
{args_xml}\n\
  </array>\n\
  <key>RunAtLoad</key>\n\
  <true/>\n\
{keep_alive_xml}\
  <key>StandardOutPath</key>\n\
  <string>{log}</string>\n\
  <key>StandardErrorPath</key>\n\
  <string>{log}</string>\n\
</dict>\n\
</plist>\n"
    )
}

pub fn build_launch_agent_plist(command: &[String], log_path: &Path) -> String {
    build_plist(DAEMON_LABEL, command, log_path, true)
}

pub fn build_transient_daemon_plist(command: &[String], log_path: &Path) -> String {
    // No KeepAlive: the app's heal path owns restarting a dead daemon.
    build_plist(DAEMON_LABEL, command, log_path, false)
}

pub fn build_intercept_agent_plist(command: &[String], log_path: &Path) -> String {
    build_plist(INTERCEPT_LABEL, command, log_path, false)
}

pub fn launchd_status(home: &Path) -> LaunchdStatus {
    let missing = LaunchdStatus {
        installed: false,
        loaded: false,
        running: false,
        pid: None,
    };
    if !launch_agent_path(home).exists() {
        return missing;
    }
    let installed = LaunchdStatus {
        installed: true,
        ..missing
    };

    let Ok(stdout) = run_checked("launchctl", &["list", DAEMON_LABEL]) else {
        return installed;
    };
    let pid = parse_pid(&stdout);
    LaunchdStatus {
        installed: true,
        loaded: true,
        running: pid.is_some(),
        pid,
    }
}

pub fn install_launch_agent(
    home: &Path,
    command: &[String],
    log_path: &Path,
) -> anyhow::Result<()> {
    install_agent(
        home,
        DAEMON_LABEL,
        &build_launch_agent_plist(command, log_path),
    )
}

pub fn install_intercept_agent(
    home: &Path,
    command: &[String],
    log_path: &Path,
) -> anyhow::Result<()> {
    install_agent(
        home,
        INTERCEPT_LABEL,
        &build_intercept_agent_plist(command, log_path),
    )
}

pub fn uninstall_launch_agent(home: &Path) -> anyhow::Result<()> {
    uninstall_agent(home, DAEMON_LABEL)
}

pub fn uninstall_intercept_agent(home: &Path) -> anyhow::Result<()> {
    uninstall_agent(home, INTERCEPT_LABEL)
}

pub fn ensure_launch_agent_running(home: &Path) -> anyhow::Result<()> {
    let status = launchd_status(home);
    if status.running {
        return Ok(());
    }
    if status.loaded {
        restart_launch_agent()
    } else {
        bootstrap(&agent_path(home, DAEMON_LABEL), DAEMON_LABEL)
    }
}

pub fn start_daemon_job(
    plist_path: &Path,
    command: &[String],
    log_path: &Path,
) -> anyhow::Result<i32> {
    let started = (|| -> anyhow::Result<i32> {
        write_plist(plist_path, &build_transient_daemon_plist(command, log_path))?;
        bootstrap(plist_path, DAEMON_LABEL)?;
        let output = run_checked(
            "launchctl",
            &["kickstart", "-p", &service_target(DAEMON_LABEL)],
        )?;
        output
            .trim()
            .parse::<i32>()
            .context("unexpected launchctl kickstart output")
    })();
    if started.is_err() {
        stop_daemon_job(plist_path);
    }
    started
}

pub fn stop_daemon_job(plist_path: &Path) {
    bootout(DAEMON_LABEL);
    let _ = std::fs::remove_file(plist_path);
}

fn restart_launch_agent() -> anyhow::Result<()> {
    run_checked(
        "launchctl",
        &["kickstart", "-k", &service_target(DAEMON_LABEL)],
    )?;
    Ok(())
}

fn install_agent(home: &Path, label: &str, plist: &str) -> anyhow::Result<()> {
    let path = agent_path(home, label);
    write_plist(&path, plist)?;
    bootstrap(&path, label)
}

fn uninstall_agent(home: &Path, label: &str) -> anyhow::Result<()> {
    bootout(label);
    let _ = std::fs::remove_file(agent_path(home, label));
    Ok(())
}

fn write_plist(plist_path: &Path, contents: &str) -> anyhow::Result<()> {
    if let Some(parent) = plist_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(plist_path, contents)?;
    Ok(())
}

fn bootstrap(plist_path: &Path, label: &str) -> anyhow::Result<()> {
    bootout(label);
    let path = plist_path.display().to_string();
    run_checked("launchctl", &["bootstrap", &domain(), &path])?;
    Ok(())
}

fn bootout(label: &str) {
    let _ = run("launchctl", &["bootout", &service_target(label)]);
}

fn service_target(label: &str) -> String {
    format!("{}/{}", domain(), label)
}

fn agent_path(home: &Path, label: &str) -> PathBuf {
    home.join("Library")
        .join("LaunchAgents")
        .join(format!("{label}.plist"))
}

fn domain() -> String {
    format!("gui/{}", nix::unistd::getuid().as_raw())
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn parse_pid(output: &str) -> Option<i32> {
    let start = output.find("\"PID\"")? + "\"PID\"".len();
    let rest = output[start..].trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}
