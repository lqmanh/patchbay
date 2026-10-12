use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::config;
use crate::daemon::{self, DaemonStatus};
use crate::engine::TunnelEngine;
use crate::json::write_json;
use crate::options::RunOptions;
use crate::paths::Paths;
use crate::platform::{LaunchdStatus, Platform, SocksState};

pub const INTERCEPT_ARGS: [&str; 2] = ["intercept", "on"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SavedProxyState {
    pub service: String,
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterceptionStatus {
    pub daemon: DaemonStatus,
    pub service: Option<String>,
    pub proxy: Option<SocksState>,
}

pub fn capture_state_before_enable(current: &SocksState, socks_port: u16) -> Option<SocksState> {
    let already_ours = current.enabled && current.host == "127.0.0.1" && current.port == socks_port;
    if already_ours {
        None
    } else {
        Some(current.clone())
    }
}

pub fn save_proxy_state(paths: &Paths, state: &SavedProxyState) -> anyhow::Result<()> {
    write_json(&paths.proxy_state_file(), state)
}

pub fn load_proxy_state(paths: &Paths) -> Option<SavedProxyState> {
    let text = std::fs::read_to_string(paths.proxy_state_file()).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    if value.is_null() {
        return None;
    }
    let raw = value.as_object();
    let get = |key: &str| raw.and_then(|raw| raw.get(key));
    Some(SavedProxyState {
        service: get("service")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        enabled: get("enabled") == Some(&Value::Bool(true)),
        host: get("host")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        port: get("port").and_then(config::parse_port).unwrap_or(0),
    })
}

pub fn clear_proxy_state(paths: &Paths) {
    let _ = std::fs::remove_file(paths.proxy_state_file());
}

pub fn interception_status(
    paths: &Paths,
    platform: &impl Platform,
    agent: Option<LaunchdStatus>,
) -> InterceptionStatus {
    let daemon = daemon_view(paths, platform, agent);
    let service = platform.active_network_service();
    let proxy = service
        .as_deref()
        .map(|service| platform.socks_proxy(service));
    InterceptionStatus {
        daemon,
        service,
        proxy,
    }
}

pub fn ensure_daemon(
    paths: &Paths,
    platform: &impl Platform,
    engine: &impl TunnelEngine,
    options: &RunOptions,
) -> anyhow::Result<bool> {
    if platform.is_daemon_agent_installed() {
        daemon::write_generated_config(paths, engine, options)?;
        platform.ensure_daemon_agent_running()?;
        return Ok(false);
    }
    if matches!(daemon::status(paths), DaemonStatus::Running { .. }) {
        return Ok(false);
    }
    daemon::start(paths, engine, options)?;
    Ok(true)
}

pub fn enable_interception(
    paths: &Paths,
    platform: &impl Platform,
    engine: &impl TunnelEngine,
    options: &RunOptions,
) -> anyhow::Result<()> {
    let started_manually = ensure_daemon(paths, platform, engine, options)?;

    let Some(service) = platform.active_network_service() else {
        if started_manually {
            daemon::stop(paths)?;
        }
        anyhow::bail!(
            "no active network service; disconnect any other VPN app (its tunnel becomes the default route) and retry"
        );
    };

    let current = platform.socks_proxy(&service);
    if let Some(state) = capture_state_before_enable(&current, options.engine.socks_port) {
        save_proxy_state(
            paths,
            &SavedProxyState {
                service: service.clone(),
                enabled: state.enabled,
                host: state.host,
                port: state.port,
            },
        )?;
    }

    platform.set_socks_proxy(&service, "127.0.0.1", options.engine.socks_port)?;
    Ok(())
}

pub fn disable_interception(paths: &Paths, platform: &impl Platform) -> anyhow::Result<()> {
    let saved = load_proxy_state(paths);
    let service = saved
        .as_ref()
        .map(|saved| saved.service.clone())
        .filter(|service| !service.is_empty())
        .or_else(|| platform.active_network_service());

    if let Some(service) = service.as_deref() {
        match saved.as_ref().filter(|saved| saved.enabled) {
            Some(saved) => platform.set_socks_proxy(service, &saved.host, saved.port)?,
            None => platform.disable_socks_proxy(service)?,
        }
    }
    clear_proxy_state(paths);
    if !platform.is_daemon_agent_installed() {
        daemon::stop(paths)?;
    }
    Ok(())
}

pub fn restart_service(
    paths: &Paths,
    platform: &impl Platform,
    engine: &impl TunnelEngine,
    options: &RunOptions,
) -> anyhow::Result<()> {
    if platform.is_daemon_agent_installed() {
        daemon::write_generated_config(paths, engine, options)?;
        let command = daemon::command(paths, engine)?;
        platform.install_daemon_agent(&command, &paths.log_file())?;
    } else {
        daemon::restart(paths, engine, options)?;
    }
    Ok(())
}

pub fn install_launch_at_login(
    paths: &Paths,
    platform: &impl Platform,
    engine: &impl TunnelEngine,
    options: &RunOptions,
) -> anyhow::Result<()> {
    let command = daemon::command(paths, engine)?;
    daemon::write_generated_config(paths, engine, options)?;
    daemon::stop(paths)?;
    platform.install_daemon_agent(&command, &paths.log_file())?;
    Ok(())
}

pub fn uninstall_launch_at_login(platform: &impl Platform) -> anyhow::Result<()> {
    platform.uninstall_daemon_agent()
}

pub fn intercept_command(cli: &Path) -> Vec<String> {
    let mut command = vec![cli.display().to_string()];
    for arg in INTERCEPT_ARGS {
        command.push(arg.to_string());
    }
    command
}

pub fn install_intercept_at_login(
    paths: &Paths,
    platform: &impl Platform,
    cli: &Path,
) -> anyhow::Result<()> {
    platform.install_intercept_agent(&intercept_command(cli), &paths.intercept_log_file())
}

pub fn uninstall_intercept_at_login(platform: &impl Platform) -> anyhow::Result<()> {
    platform.uninstall_intercept_agent()
}

fn daemon_view(
    paths: &Paths,
    platform: &impl Platform,
    agent: Option<LaunchdStatus>,
) -> DaemonStatus {
    let status = match agent {
        Some(agent) => Some(agent),
        None => platform
            .is_daemon_agent_installed()
            .then(|| platform.daemon_agent_status()),
    };
    match status {
        Some(status) => match status.pid {
            Some(pid) if status.running && pid != 0 => DaemonStatus::Running { pid },
            _ => DaemonStatus::Stopped,
        },
        None => daemon::status(paths),
    }
}
