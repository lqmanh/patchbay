pub(crate) mod exec;
pub mod macos;

use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SocksState {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LaunchdStatus {
    pub installed: bool,
    pub loaded: bool,
    pub running: bool,
    pub pid: Option<i32>,
}

pub trait Platform {
    fn active_network_service(&self) -> Option<String>;
    fn socks_proxy(&self, service: &str) -> SocksState;
    fn set_socks_proxy(&self, service: &str, host: &str, port: u16) -> anyhow::Result<()>;
    fn disable_socks_proxy(&self, service: &str) -> anyhow::Result<()>;

    fn is_daemon_agent_installed(&self) -> bool;
    fn is_intercept_agent_installed(&self) -> bool;
    fn daemon_agent_status(&self) -> LaunchdStatus;
    fn install_daemon_agent(&self, command: &[String], log_path: &Path) -> anyhow::Result<()>;
    fn uninstall_daemon_agent(&self) -> anyhow::Result<()>;
    fn ensure_daemon_agent_running(&self) -> anyhow::Result<()>;
    fn install_intercept_agent(&self, command: &[String], log_path: &Path) -> anyhow::Result<()>;
    fn uninstall_intercept_agent(&self) -> anyhow::Result<()>;
}
