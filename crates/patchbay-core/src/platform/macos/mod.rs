mod launchd;
mod proxy;

pub use launchd::{
    build_intercept_agent_plist, build_launch_agent_plist, build_transient_daemon_plist,
    intercept_agent_path, launch_agent_path,
};
pub(crate) use launchd::{start_daemon_job, stop_daemon_job};
pub use proxy::{parse_active_interface, parse_service_for_device, parse_socks_state};

use std::path::{Path, PathBuf};

use crate::platform::{LaunchdStatus, Platform, SocksState};

#[derive(Clone)]
pub struct MacPlatform {
    home: PathBuf,
}

impl MacPlatform {
    pub fn new(home: impl Into<PathBuf>) -> Self {
        Self { home: home.into() }
    }

    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self::new(crate::paths::home_dir()?))
    }

    pub fn reveal_in_finder(&self, path: &Path) -> anyhow::Result<()> {
        let path = path.display().to_string();
        crate::platform::exec::run_checked("open", &[&path])?;
        Ok(())
    }
}

impl Platform for MacPlatform {
    fn active_network_service(&self) -> Option<String> {
        proxy::active_network_service()
    }

    fn socks_proxy(&self, service: &str) -> SocksState {
        proxy::get_socks_proxy(service)
    }

    fn set_socks_proxy(&self, service: &str, host: &str, port: u16) -> anyhow::Result<()> {
        proxy::set_socks_proxy(service, host, port)
    }

    fn disable_socks_proxy(&self, service: &str) -> anyhow::Result<()> {
        proxy::disable_socks_proxy(service)
    }

    fn is_daemon_agent_installed(&self) -> bool {
        launch_agent_path(&self.home).exists()
    }

    fn is_intercept_agent_installed(&self) -> bool {
        intercept_agent_path(&self.home).exists()
    }

    fn daemon_agent_status(&self) -> LaunchdStatus {
        launchd::launchd_status(&self.home)
    }

    fn install_daemon_agent(&self, command: &[String], log_path: &Path) -> anyhow::Result<()> {
        launchd::install_launch_agent(&self.home, command, log_path)
    }

    fn uninstall_daemon_agent(&self) -> anyhow::Result<()> {
        launchd::uninstall_launch_agent(&self.home)
    }

    fn ensure_daemon_agent_running(&self) -> anyhow::Result<()> {
        launchd::ensure_launch_agent_running(&self.home)
    }

    fn install_intercept_agent(&self, command: &[String], log_path: &Path) -> anyhow::Result<()> {
        launchd::install_intercept_agent(&self.home, command, log_path)
    }

    fn uninstall_intercept_agent(&self) -> anyhow::Result<()> {
        launchd::uninstall_intercept_agent(&self.home)
    }
}
