use std::io;
use std::path::PathBuf;

use anyhow::Context;

pub fn home_dir() -> anyhow::Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home))
}

#[derive(Debug, Clone)]
pub struct Paths {
    config_home: PathBuf,
    cache_home: PathBuf,
    state_home: PathBuf,
    runtime_home: Option<PathBuf>,
}

impl Paths {
    pub fn new(home: impl Into<PathBuf>, env: impl Fn(&str) -> Option<String>) -> Self {
        let home = home.into();
        let xdg = |name: &str, fallback: &str| -> PathBuf {
            match env(name) {
                Some(value) if !value.is_empty() => PathBuf::from(value),
                _ => home.join(fallback),
            }
        };
        Self {
            config_home: xdg("XDG_CONFIG_HOME", ".config"),
            cache_home: xdg("XDG_CACHE_HOME", ".cache"),
            state_home: xdg("XDG_STATE_HOME", ".local/state"),
            runtime_home: env("XDG_RUNTIME_DIR")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
        }
    }

    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self::new(home_dir()?, |name| std::env::var(name).ok()))
    }

    pub fn config_dir(&self) -> PathBuf {
        self.config_home.join("patchbay")
    }

    pub fn servers_dir(&self) -> PathBuf {
        self.config_dir().join("servers")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.cache_home.join("patchbay")
    }

    pub fn state_dir(&self) -> PathBuf {
        self.state_home.join("patchbay")
    }

    pub fn runtime_dir(&self) -> PathBuf {
        self.runtime_home
            .as_ref()
            .map(|home| home.join("patchbay"))
            .unwrap_or_else(|| self.state_dir())
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir().join("config.json")
    }

    pub fn daemon_config_file(&self) -> PathBuf {
        self.cache_dir().join("daemon.json")
    }

    pub fn pid_file(&self) -> PathBuf {
        self.runtime_dir().join("daemon.pid")
    }

    pub fn log_file(&self) -> PathBuf {
        self.state_dir().join("daemon.log")
    }

    pub fn intercept_log_file(&self) -> PathBuf {
        self.state_dir().join("intercept.log")
    }

    pub fn proxy_state_file(&self) -> PathBuf {
        self.state_dir().join("proxy-state.json")
    }

    pub fn window_state_file(&self) -> PathBuf {
        self.state_dir().join("window.json")
    }

    pub fn ensure_dirs(&self) -> io::Result<()> {
        for dir in [
            self.config_dir(),
            self.servers_dir(),
            self.cache_dir(),
            self.state_dir(),
            self.runtime_dir(),
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}
