use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

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
    // No KeepAlive: the job runs the one-shot launcher, and rerunning it would
    // start another daemon. The app's heal path restarts a dead daemon.
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
) -> anyhow::Result<()> {
    write_plist(plist_path, &build_transient_daemon_plist(command, log_path))?;
    bootstrap(plist_path, DAEMON_LABEL)
}

pub fn stop_daemon_job(plist_path: &Path) {
    bootout(DAEMON_LABEL);
    let _ = std::fs::remove_file(plist_path);
}

/// How the daemon job's last run failed, if it has exited unsuccessfully and
/// isn't running again. `None` while it's still running or hasn't exited yet.
pub fn daemon_job_failure() -> Option<String> {
    job_failure(&run_checked("launchctl", &["list", DAEMON_LABEL]).ok()?)
}

fn job_failure(list_output: &str) -> Option<String> {
    if parse_pid(list_output).is_some() {
        return None;
    }
    let raw = parse_number(list_output, "LastExitStatus")?;
    let status = ExitStatus::from_raw(raw.try_into().ok()?);
    (!status.success()).then(|| status.to_string())
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
    parse_number(output, "PID")?.try_into().ok()
}

/// Reads `"key" = <integer>;` from `launchctl list <label>` output.
fn parse_number(output: &str, key: &str) -> Option<i64> {
    let quoted = format!("\"{key}\"");
    let start = output.find(&quoted)? + quoted.len();
    let rest = output[start..].trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let end = rest
        .find(|c: char| c != '-' && !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXITED: &str =
        "{\n\t\"Label\" = \"io.github.lqmanh.patchbay.daemon\";\n\t\"LastExitStatus\" = 256;\n};\n";
    const RUNNING: &str = "{\n\t\"LastExitStatus\" = 0;\n\t\"PID\" = 82779;\n};\n";

    #[test]
    fn parses_launchctl_list_numbers() {
        assert_eq!(parse_number(EXITED, "LastExitStatus"), Some(256));
        assert_eq!(parse_pid(EXITED), None);
        assert_eq!(parse_pid(RUNNING), Some(82779));
        assert_eq!(
            parse_number("\"LastExitStatus\" = -1;", "LastExitStatus"),
            Some(-1)
        );
    }

    #[test]
    fn reports_only_a_failed_exit_as_a_job_failure() {
        assert_eq!(job_failure(EXITED).as_deref(), Some("exit status: 1"));
        assert_eq!(job_failure(RUNNING), None);
        assert_eq!(job_failure("{\n\t\"LastExitStatus\" = 0;\n};\n"), None);
    }
}
