use std::fs::{File, OpenOptions};
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

use anyhow::Context;
use nix::errno::Errno;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

use crate::engine::{TunnelEngine, require_binary};
use crate::options::RunOptions;
use crate::paths::Paths;
use crate::platform::macos::{daemon_job_failure, start_daemon_job, stop_daemon_job};
use crate::servers;

const CONNECT_TIMEOUT: Duration = Duration::from_millis(1500);
const START_TIMEOUT: Duration = Duration::from_secs(5);
const STOP_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const LAUNCHER_CHECK_INTERVAL: Duration = Duration::from_millis(250);
const LAUNCHER_ARG: &str = "__patchbay_daemon_launch";

/// Set once this executable has called [`launcher_main`]. [`launch`] re-runs
/// the executable as the launcher, and without that call launchd would run it
/// as itself, with arguments it doesn't understand.
static HANDLES_LAUNCHER: AtomicBool = AtomicBool::new(false);

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
    let Some(pid) = read_pid(&paths.pid_file()) else {
        return DaemonStatus::Stopped;
    };
    if is_alive(pid) {
        DaemonStatus::Running { pid }
    } else {
        DaemonStatus::Stale { pid }
    }
}

fn read_pid(path: &Path) -> Option<i32> {
    let text = std::fs::read_to_string(path).ok()?;
    let pid = text.trim().parse::<i32>().ok()?;
    (pid > 0).then_some(pid)
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
    let _lock = lock_daemon(paths)?;
    if let DaemonStatus::Running { pid } = status(paths) {
        anyhow::bail!("daemon already running (pid {pid})");
    }
    launch(paths, engine, options)
}

/// Starts the daemon unless one is already running, and returns whether this
/// call started it. Checking [`status`] before calling [`start`] would let a
/// concurrent start slip in between the check and the launch.
pub fn ensure_running(
    paths: &Paths,
    engine: &impl TunnelEngine,
    options: &RunOptions,
) -> anyhow::Result<bool> {
    let _lock = lock_daemon(paths)?;
    if status(paths).is_running() {
        return Ok(false);
    }
    launch(paths, engine, options)?;
    Ok(true)
}

/// Serializes starting and stopping the daemon across threads and processes.
/// launchd doesn't track the daemon, so two unserialized starts could each
/// spawn one and lose the PID of the first. Dropping the file releases the
/// lock.
fn lock_daemon(paths: &Paths) -> anyhow::Result<File> {
    let path = paths.daemon_lock_file();
    std::fs::create_dir_all(paths.state_dir())?;
    let file =
        open_private(&path, false).with_context(|| format!("cannot open {}", path.display()))?;
    file.lock()
        .with_context(|| format!("cannot lock {}", path.display()))?;
    Ok(file)
}

fn launch(paths: &Paths, engine: &impl TunnelEngine, options: &RunOptions) -> anyhow::Result<i32> {
    anyhow::ensure!(
        HANDLES_LAUNCHER.load(Ordering::Relaxed),
        "this executable can't launch the daemon: its main() must call \
         patchbay_core::daemon::launcher_main() first"
    );
    let command = command(paths, engine)?;
    write_generated_config(paths, engine, options)?;

    // Quitting the app whose terminal we run in (seen with Zed) can unload
    // every launchd job loaded from that terminal and kill each job's
    // processes, so sing-box can't be the job itself. Start a one-shot
    // launcher job instead: it starts the daemon in a session of its own and
    // exits, and unloading the job then leaves the daemon running.
    let pid_file = paths.pid_file();
    let plist = paths.daemon_plist_file();
    let log = paths.log_file();
    let launcher = launcher_command(&pid_file, &command)?;
    let _ = std::fs::remove_file(&pid_file);
    let started = start_daemon_job(&plist, &launcher, &log).and_then(|()| {
        wait_for_pid(&pid_file)
            .with_context(|| format!("daemon did not start; check {}", log.display()))
    });
    // The job's work is done whether or not the daemon came up.
    stop_daemon_job(&plist);
    started
}

/// Handles the hidden launcher mode the daemon is started through. Every
/// executable that starts the daemon must call this first thing in `main` and
/// exit with the returned code. Returns `None` when the process was not
/// started as a launcher.
pub fn launcher_main() -> Option<u8> {
    HANDLES_LAUNCHER.store(true, Ordering::Relaxed);
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some(LAUNCHER_ARG) {
        return None;
    }
    Some(match launch_detached(args) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("patchbay: {error:#}");
            1
        }
    })
}

fn launcher_command(pid_file: &Path, command: &[String]) -> anyhow::Result<Vec<String>> {
    let executable = std::env::current_exe().context("cannot locate the patchbay executable")?;
    let mut launcher = vec![
        executable.display().to_string(),
        LAUNCHER_ARG.to_string(),
        pid_file.display().to_string(),
    ];
    launcher.extend_from_slice(command);
    Ok(launcher)
}

fn launch_detached(mut args: impl Iterator<Item = String>) -> anyhow::Result<()> {
    let pid_file = args.next().context("missing launcher pid file")?;
    let program = args.next().context("missing daemon command")?;
    let mut process = Command::new(&program);
    process.args(args).stdin(Stdio::null());
    // A session of its own keeps the daemon out of the job's process group,
    // which launchd kills when the job exits or is unloaded.
    unsafe {
        process.pre_exec(|| {
            nix::unistd::setsid()?;
            Ok(())
        });
    }
    let mut child = process
        .spawn()
        .with_context(|| format!("cannot start the daemon at {program}"))?;
    write_pid_file(Path::new(&pid_file), child.id() as i32).inspect_err(|_| {
        let _ = child.kill();
    })
}

fn wait_for_pid(pid_file: &Path) -> anyhow::Result<i32> {
    let deadline = Instant::now() + START_TIMEOUT;
    let mut next_launcher_check = Instant::now() + LAUNCHER_CHECK_INTERVAL;
    loop {
        // The launcher writes the PID only once the daemon has spawned, so a
        // dead PID means the daemon exited straight away.
        if let Some(pid) = read_pid(pid_file) {
            anyhow::ensure!(is_alive(pid), "the daemon exited right after starting");
            return Ok(pid);
        }
        let now = Instant::now();
        if now >= next_launcher_check {
            if let Some(failure) = daemon_job_failure() {
                anyhow::bail!("the daemon launcher failed ({failure})");
            }
            next_launcher_check = now + LAUNCHER_CHECK_INTERVAL;
        }
        if now >= deadline {
            anyhow::bail!("timed out waiting for {}", pid_file.display());
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

pub fn stop(paths: &Paths) -> anyhow::Result<()> {
    let _lock = lock_daemon(paths)?;
    stop_locked(paths);
    Ok(())
}

fn stop_locked(paths: &Paths) {
    if let DaemonStatus::Running { pid } = status(paths) {
        let _ = kill(Pid::from_raw(pid), Signal::SIGTERM);
        let deadline = Instant::now() + STOP_TIMEOUT;
        while Instant::now() < deadline && is_alive(pid) {
            std::thread::sleep(POLL_INTERVAL);
        }
        if is_alive(pid) {
            let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
        }
    }
    let _ = std::fs::remove_file(paths.pid_file());
}

pub fn restart(
    paths: &Paths,
    engine: &impl TunnelEngine,
    options: &RunOptions,
) -> anyhow::Result<i32> {
    // One lock for both halves, so no other start lands in between. The lock
    // is a flock on a fresh file description, so taking it again here through
    // stop() or start() would deadlock.
    let _lock = lock_daemon(paths)?;
    stop_locked(paths);
    launch(paths, engine, options)
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
    open_private(path, true)?.write_all(text.as_bytes())?;
    Ok(())
}

/// Opens `path` for writing, creating it readable only by the user.
fn open_private(path: &Path, truncate: bool) -> std::io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(truncate)
        .mode(0o600)
        .open(path)
}

fn write_pid_file(path: &Path, pid: i32) -> anyhow::Result<()> {
    let temp = path.with_extension("pid.tmp");
    write_private(&temp, &pid.to_string())?;
    std::fs::rename(&temp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use nix::sys::wait::waitpid;

    use super::*;

    #[test]
    fn launcher_command_prepends_the_pid_handshake() {
        let command = vec!["/opt/homebrew/bin/sing-box".to_string(), "run".to_string()];
        let launcher = launcher_command(Path::new("/tmp/daemon.pid"), &command).expect("launcher");

        assert_eq!(launcher[1], LAUNCHER_ARG);
        assert_eq!(launcher[2], "/tmp/daemon.pid");
        assert_eq!(&launcher[3..], command.as_slice());
    }

    #[test]
    fn daemon_lock_excludes_other_holders_until_dropped() {
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = Paths::new(dir.path(), |_| None);

        let held = lock_daemon(&paths).expect("lock");
        let other = File::open(paths.daemon_lock_file()).expect("open lock file");
        assert!(matches!(
            other.try_lock(),
            Err(std::fs::TryLockError::WouldBlock)
        ));

        drop(held);
        // A test forking in parallel holds a copy of the descriptor, and so
        // the lock, until its child execs.
        let deadline = Instant::now() + Duration::from_secs(2);
        while let Err(error) = other.try_lock() {
            assert!(Instant::now() < deadline, "lock after release: {error}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn launcher_starts_the_daemon_in_its_own_session() {
        let dir = tempfile::tempdir().expect("temp dir");
        let pid_file = dir.path().join("daemon.pid");
        let args = vec![
            pid_file.display().to_string(),
            "/bin/sleep".to_string(),
            "30".to_string(),
        ];

        launch_detached(args.into_iter()).expect("launch");

        let pid = Pid::from_raw(read_pid(&pid_file).expect("pid"));
        assert!(is_alive(pid.as_raw()));
        assert_eq!(nix::unistd::getsid(Some(pid)).expect("session"), pid);

        let _ = kill(pid, Signal::SIGKILL);
        let _ = waitpid(pid, None);
    }
}
