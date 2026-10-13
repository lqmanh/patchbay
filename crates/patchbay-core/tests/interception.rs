use std::cell::RefCell;
use std::path::Path;

use patchbay_core::daemon::DaemonStatus;
use patchbay_core::interception::{
    SavedProxyState, capture_state_before_enable, clear_proxy_state, interception_status,
    load_proxy_state, save_proxy_state, sync_proxy_port,
};
use patchbay_core::paths::Paths;
use patchbay_core::platform::{LaunchdStatus, Platform, SocksState};

#[test]
fn does_not_overwrite_a_state_we_already_own() {
    let current = SocksState {
        enabled: true,
        host: "127.0.0.1".into(),
        port: 25344,
    };
    assert_eq!(capture_state_before_enable(&current, 25344), None);
}

#[test]
fn captures_disabled_and_foreign_proxy_states() {
    let disabled = SocksState {
        enabled: false,
        host: String::new(),
        port: 0,
    };
    assert_eq!(
        capture_state_before_enable(&disabled, 25344),
        Some(disabled.clone())
    );

    let foreign = SocksState {
        enabled: true,
        host: "10.0.0.1".into(),
        port: 1080,
    };
    assert_eq!(
        capture_state_before_enable(&foreign, 25344),
        Some(foreign.clone())
    );
}

#[test]
fn sync_repoints_an_enabled_loopback_proxy() {
    let platform = FakePlatform {
        service: Some("Wi-Fi".into()),
        proxy: SocksState {
            enabled: true,
            host: "127.0.0.1".into(),
            port: 25344,
        },
        ..FakePlatform::new()
    };

    sync_proxy_port(&platform, 26000).expect("sync");

    let calls = platform.set_calls.borrow();
    assert_eq!(
        calls.as_slice(),
        [("Wi-Fi".to_string(), "127.0.0.1".to_string(), 26000)]
    );
}

#[test]
fn sync_leaves_foreign_or_current_state_alone() {
    let untouched = [
        (
            None,
            SocksState {
                enabled: true,
                host: "127.0.0.1".into(),
                port: 25344,
            },
        ),
        (
            Some("Wi-Fi".into()),
            SocksState {
                enabled: false,
                host: String::new(),
                port: 0,
            },
        ),
        (
            Some("Wi-Fi".into()),
            SocksState {
                enabled: true,
                host: "10.0.0.1".into(),
                port: 1080,
            },
        ),
        (
            Some("Wi-Fi".into()),
            SocksState {
                enabled: true,
                host: "127.0.0.1".into(),
                port: 25344,
            },
        ),
    ];

    for (service, proxy) in untouched {
        let platform = FakePlatform {
            service,
            proxy,
            ..FakePlatform::new()
        };
        sync_proxy_port(&platform, 25344).expect("sync");
        assert!(platform.set_calls.borrow().is_empty());
    }
}

#[test]
fn round_trips_saved_state() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = test_paths(&dir);

    let state = SavedProxyState {
        service: "Wi-Fi".into(),
        enabled: true,
        host: "10.0.0.1".into(),
        port: 1080,
    };
    save_proxy_state(&paths, &state).expect("save");
    assert_eq!(load_proxy_state(&paths), Some(state));

    clear_proxy_state(&paths);
    assert_eq!(load_proxy_state(&paths), None);
}

#[test]
fn daemon_status_comes_from_the_pid_file_without_a_launch_agent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = test_paths(&dir);
    let pid = std::process::id() as i32;
    std::fs::write(paths.pid_file(), pid.to_string()).expect("pid file");

    let platform = FakePlatform::new();
    let status = interception_status(&paths, &platform);
    assert_eq!(status.daemon, DaemonStatus::Running { pid });
}

#[test]
fn daemon_status_comes_from_the_launch_agent_when_installed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = test_paths(&dir);
    std::fs::write(paths.pid_file(), std::process::id().to_string()).expect("pid file");

    let platform = FakePlatform {
        agent_installed: true,
        ..FakePlatform::new()
    };
    let status = interception_status(&paths, &platform);
    assert_eq!(status.daemon, DaemonStatus::Stopped);

    let platform = FakePlatform {
        agent_installed: true,
        agent: LaunchdStatus {
            installed: true,
            loaded: true,
            running: true,
            pid: Some(4242),
        },
        ..FakePlatform::new()
    };
    let status = interception_status(&paths, &platform);
    assert_eq!(status.daemon, DaemonStatus::Running { pid: 4242 });
}

fn test_paths(dir: &tempfile::TempDir) -> Paths {
    let paths = Paths::new("/home/test", |name| match name {
        "XDG_STATE_HOME" => Some(dir.path().display().to_string()),
        _ => None,
    });
    std::fs::create_dir_all(paths.state_dir()).expect("state dir");
    paths
}

struct FakePlatform {
    agent_installed: bool,
    agent: LaunchdStatus,
    service: Option<String>,
    proxy: SocksState,
    set_calls: RefCell<Vec<(String, String, u16)>>,
}

impl FakePlatform {
    fn new() -> Self {
        Self {
            agent_installed: false,
            agent: LaunchdStatus::default(),
            service: None,
            proxy: SocksState {
                enabled: false,
                host: String::new(),
                port: 0,
            },
            set_calls: RefCell::new(Vec::new()),
        }
    }
}

impl Platform for FakePlatform {
    fn active_network_service(&self) -> Option<String> {
        self.service.clone()
    }

    fn socks_proxy(&self, _service: &str) -> SocksState {
        self.proxy.clone()
    }

    fn set_socks_proxy(&self, service: &str, host: &str, port: u16) -> anyhow::Result<()> {
        self.set_calls
            .borrow_mut()
            .push((service.to_string(), host.to_string(), port));
        Ok(())
    }

    fn disable_socks_proxy(&self, _service: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn is_daemon_agent_installed(&self) -> bool {
        self.agent_installed
    }

    fn is_intercept_agent_installed(&self) -> bool {
        false
    }

    fn daemon_agent_status(&self) -> LaunchdStatus {
        self.agent
    }

    fn install_daemon_agent(&self, _command: &[String], _log_path: &Path) -> anyhow::Result<()> {
        Ok(())
    }

    fn uninstall_daemon_agent(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn ensure_daemon_agent_running(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn install_intercept_agent(&self, _command: &[String], _log_path: &Path) -> anyhow::Result<()> {
        Ok(())
    }

    fn uninstall_intercept_agent(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
