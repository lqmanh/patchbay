use std::fs::OpenOptions;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

use nix::errno::Errno;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

use crate::engine::{TunnelEngine, require_binary};
use crate::options::RunOptions;
use crate::paths::Paths;
use crate::servers;

const CONNECT_TIMEOUT: Duration = Duration::from_millis(1500);
const STOP_TIMEOUT: Duration = Duration::from_secs(5);
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonStatus {
    Stopped,
    Running { pid: i32 },
    Stale { pid: i32 },
}

impl DaemonStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }

    pub fn pid(&self) -> Option<i32> {
        match self {
            Self::Running { pid } => Some(*pid),
            _ => None,
        }
    }
}

pub fn status(paths: &Paths) -> DaemonStatus {
    let Ok(text) = std::fs::read_to_string(paths.pid_file()) else {
        return DaemonStatus::Stopped;
    };
    let Ok(pid) = text.trim().parse::<i32>() else {
        return DaemonStatus::Stopped;
    };
    if pid <= 0 {
        return DaemonStatus::Stopped;
    }
    if is_alive(pid) {
        DaemonStatus::Running { pid }
    } else {
        DaemonStatus::Stale { pid }
    }
}

pub fn write_generated_config(
    paths: &Paths,
    engine: &impl TunnelEngine,
    options: &RunOptions,
) -> anyhow::Result<()> {
    let binary = require_binary(engine)?;
    engine.check_binary(&binary)?;
    let server_conf = servers::read_server_config(paths, &options.server_file)?;
    let profile = servers::server_path(paths, &options.server_file);
    let config = engine.render_config(options.protocol, &server_conf, &profile, &options.engine)?;
    paths.ensure_dirs()?;
    write_private(&paths.daemon_config_file(), &config)
}

pub fn command(paths: &Paths, engine: &impl TunnelEngine) -> anyhow::Result<Vec<String>> {
    let binary = require_binary(engine)?;
    let mut command = vec![binary.display().to_string()];
    command.extend(engine.run_args(&paths.daemon_config_file()));
    Ok(command)
}

pub fn start(
    paths: &Paths,
    engine: &impl TunnelEngine,
    options: &RunOptions,
) -> anyhow::Result<i32> {
    let command = command(paths, engine)?;
    if let DaemonStatus::Running { pid } = status(paths) {
        anyhow::bail!("daemon already running (pid {pid})");
    }
    write_generated_config(paths, engine, options)?;

    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.log_file())?;
    let mut process = Command::new(&command[0]);
    process
        .args(&command[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log));
    unsafe {
        process.pre_exec(|| {
            nix::unistd::setsid()
                .map(|_| ())
                .map_err(std::io::Error::from)
        });
    }
    let mut child = process.spawn()?;
    let pid = child.id() as i32;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    std::fs::write(paths.pid_file(), pid.to_string())?;
    Ok(pid)
}

pub fn stop(paths: &Paths) -> anyhow::Result<()> {
    if let DaemonStatus::Running { pid } = status(paths) {
        let _ = kill(Pid::from_raw(pid), Signal::SIGTERM);
        let deadline = Instant::now() + STOP_TIMEOUT;
        while Instant::now() < deadline && is_alive(pid) {
            std::thread::sleep(STOP_POLL_INTERVAL);
        }
        if is_alive(pid) {
            let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
        }
    }
    let _ = std::fs::remove_file(paths.pid_file());
    Ok(())
}

pub fn restart(
    paths: &Paths,
    engine: &impl TunnelEngine,
    options: &RunOptions,
) -> anyhow::Result<i32> {
    stop(paths)?;
    start(paths, engine, options)
}

pub fn is_reachable(port: u16) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&address, CONNECT_TIMEOUT).is_ok()
}

pub fn wait_for_reachable(port: u16, attempts: u32, delay: Duration) -> bool {
    for _ in 0..attempts {
        if is_reachable(port) {
            return true;
        }
        std::thread::sleep(delay);
    }
    false
}

pub fn log_stamp(paths: &Paths) -> Option<(u64, SystemTime)> {
    let metadata = std::fs::metadata(paths.log_file()).ok()?;
    Some((metadata.len(), metadata.modified().ok()?))
}

pub fn read_log(paths: &Paths, window_lines: usize, keep_lines: usize) -> Vec<String> {
    read_log_inner(paths, window_lines, keep_lines).unwrap_or_default()
}

fn read_log_inner(
    paths: &Paths,
    window_lines: usize,
    keep_lines: usize,
) -> anyhow::Result<Vec<String>> {
    let text = std::fs::read_to_string(paths.log_file())?;
    let lines: Vec<&str> = text.split('\n').filter(|line| !line.is_empty()).collect();
    let kept = &lines[lines.len().saturating_sub(keep_lines)..];
    if kept.len() < lines.len() {
        let mut text = kept.join("\n");
        text.push('\n');
        std::fs::write(paths.log_file(), text)?;
    }
    let window = &kept[kept.len().saturating_sub(window_lines)..];
    Ok(window.iter().map(|line| line.to_string()).collect())
}

fn is_alive(pid: i32) -> bool {
    match kill(Pid::from_raw(pid), None) {
        // EPERM means the process exists but belongs to another user.
        Ok(()) | Err(Errno::EPERM) => true,
        Err(_) => false,
    }
}

fn write_private(path: &Path, text: &str) -> anyhow::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(text.as_bytes())?;
    Ok(())
}
