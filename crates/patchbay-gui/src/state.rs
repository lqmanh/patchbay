use std::time::{Duration, SystemTime};

use gpui_kit::component::{IconName, Theme, ThemeMode};
use gpui_kit::*;

use patchbay_core::config::{self, AppConfig, Mode};
use patchbay_core::daemon::{self, DaemonStatus};
use patchbay_core::domains::{self, RuleKind};
use patchbay_core::engine::singbox::Singbox;
use patchbay_core::interception::{self, InterceptionStatus};
use patchbay_core::options;
use patchbay_core::paths::Paths;
use patchbay_core::platform::macos::MacPlatform;
use patchbay_core::platform::{LaunchdStatus, Platform};
use patchbay_core::servers::{self, Server};

const STATUS_POLL_INTERVAL: Duration = Duration::from_secs(60);
const LOG_POLL_INTERVAL: Duration = Duration::from_secs(5);
const LOG_WINDOW_LINES: usize = 1000;
const LOG_KEEP_LINES: usize = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    On,
    Broken,
    Off,
}

impl StateKind {
    pub fn icon(self) -> IconName {
        match self {
            Self::On => IconName::CircleCheck,
            Self::Broken => IconName::TriangleAlert,
            Self::Off => IconName::CircleX,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::On => "Intercepted",
            Self::Broken => "Needs attention",
            Self::Off => "Direct",
        }
    }

    pub fn color(self, theme: &Theme) -> Hsla {
        match self {
            Self::On => theme.success,
            Self::Broken => theme.danger,
            Self::Off => theme.muted_foreground,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePref {
    System,
    Light,
    Dark,
}

impl ThemePref {
    pub const ALL: [ThemePref; 3] = [Self::System, Self::Light, Self::Dark];

    pub fn from_config(value: &str) -> Self {
        match value {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    pub fn apply(self, window: Option<&mut Window>, cx: &mut App) {
        match self {
            Self::Light => Theme::change(ThemeMode::Light, window, cx),
            Self::Dark => Theme::change(ThemeMode::Dark, window, cx),
            Self::System => Theme::sync_system_appearance(window, cx),
        }
    }
}

struct Snapshot {
    paths: Paths,
    platform: MacPlatform,
    config: AppConfig,
    servers: Vec<Server>,
}

struct StatusSnapshot {
    servers: Vec<Server>,
    status: InterceptionStatus,
    intercept_agent: bool,
    ready: Option<bool>,
}

pub struct AppState {
    pub paths: Paths,
    pub platform: MacPlatform,
    pub config: AppConfig,
    pub servers: Vec<Server>,
    pub status: InterceptionStatus,
    pub intercept_agent: bool,
    pub log_lines: Vec<SharedString>,
    pub ready: Option<bool>,
    pub busy: bool,
    log_stamp: Option<(u64, SystemTime)>,
    log_polling: bool,
    log_poll_task: Option<Task<()>>,
    refresh_in_flight: bool,
    refresh_pending: bool,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let paths = Paths::from_env()?;
        let platform = MacPlatform::from_env()?;
        let config = config::load_config(&paths);
        Ok(Self {
            paths,
            platform,
            config,
            servers: Vec::new(),
            status: InterceptionStatus {
                daemon: DaemonStatus::Stopped,
                service: None,
                proxy: None,
                agent: LaunchdStatus::default(),
            },
            intercept_agent: false,
            log_lines: Vec::new(),
            ready: None,
            busy: false,
            log_stamp: None,
            log_polling: true,
            log_poll_task: None,
            refresh_in_flight: false,
            refresh_pending: false,
        })
    }

    pub fn theme(&self) -> ThemePref {
        ThemePref::from_config(&self.config.theme)
    }

    pub fn intercepting(&self) -> bool {
        self.status.intercepting()
    }

    pub fn running(&self) -> bool {
        self.status.daemon.is_running()
    }

    pub fn state_kind(&self) -> StateKind {
        if is_broken(&self.status) {
            StateKind::Broken
        } else if self.intercepting() {
            StateKind::On
        } else {
            StateKind::Off
        }
    }

    pub fn tunnel_label(&self) -> &'static str {
        if !self.running() {
            return "Daemon stopped";
        }
        match self.ready {
            None => "Checking",
            Some(false) => "Connecting",
            Some(true) => "Ready",
        }
    }

    pub fn active_server(&self) -> Option<&Server> {
        servers::pick_server(&self.servers, self.config.active_server.as_deref())
    }

    pub fn start(&mut self, cx: &mut Context<Self>) {
        self.refresh_log(cx);
        self.refresh_status_inner(cx, true);

        poll_every(cx, STATUS_POLL_INTERVAL, |this, cx| this.refresh_status(cx)).detach();
        self.start_log_poll(cx);
    }

    /// Only a window shows the log, so while none is open, stop polling it
    /// and drop the lines.
    pub fn set_log_polling(&mut self, polling: bool, cx: &mut Context<Self>) {
        if self.log_polling == polling {
            return;
        }
        self.log_polling = polling;
        if polling {
            self.start_log_poll(cx);
            self.refresh_log(cx);
        } else {
            self.log_poll_task = None;
            self.log_lines = Vec::new();
            self.log_stamp = None;
        }
    }

    fn start_log_poll(&mut self, cx: &mut Context<Self>) {
        self.log_poll_task = Some(poll_every(cx, LOG_POLL_INTERVAL, |this, cx| {
            this.refresh_log(cx);
        }));
    }

    pub fn refresh_status(&mut self, cx: &mut Context<Self>) {
        self.refresh_status_inner(cx, false);
    }

    fn refresh_status_inner(&mut self, cx: &mut Context<Self>, heal: bool) {
        if self.refresh_in_flight {
            self.refresh_pending = true;
            return;
        }
        self.refresh_in_flight = true;
        let snapshot = self.snapshot();
        spawn_task(
            cx,
            move || status_snapshot(&snapshot, heal),
            |this, status, cx| {
                let changed = this.status != status.status
                    || this.servers != status.servers
                    || this.intercept_agent != status.intercept_agent
                    || this.ready != status.ready;
                this.status = status.status;
                this.servers = status.servers;
                this.intercept_agent = status.intercept_agent;
                this.ready = status.ready;
                this.refresh_in_flight = false;
                if this.refresh_pending {
                    this.refresh_pending = false;
                    this.refresh_status(cx);
                }
                if changed {
                    cx.notify();
                }
            },
        );
    }

    pub fn refresh_log(&mut self, cx: &mut Context<Self>) {
        let paths = self.paths.clone();
        let stamp = self.log_stamp;
        spawn_task(
            cx,
            move || {
                let current = daemon::log_stamp(&paths);
                if current == stamp {
                    return None;
                }
                Some((
                    current,
                    daemon::read_log(&paths, LOG_WINDOW_LINES, LOG_KEEP_LINES),
                ))
            },
            |this, result, cx| {
                let Some((stamp, lines)) = result.filter(|_| this.log_polling) else {
                    return;
                };
                this.log_stamp = stamp;
                let unchanged = this.log_lines.len() == lines.len()
                    && this
                        .log_lines
                        .iter()
                        .zip(&lines)
                        .all(|(current, next)| current.as_ref() == next.as_str());
                if unchanged {
                    return;
                }
                this.log_lines = lines.into_iter().map(SharedString::from).collect();
                cx.notify();
            },
        );
    }

    pub fn toggle_interception(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }

        if self.intercepting() {
            self.run_busy(cx, |snapshot| {
                let _ = interception::disable_interception(&snapshot.paths, &snapshot.platform);
            });
            return;
        }

        let Some(options) = options::build_run_options(&self.config, &self.servers) else {
            return;
        };
        self.config.active_server = Some(options.server_file.clone());
        self.run_busy(cx, move |snapshot| {
            let _ = config::save_config(&snapshot.paths, &snapshot.config);
            let _ = interception::enable_interception(
                &snapshot.paths,
                &snapshot.platform,
                &Singbox,
                &options,
            );
        });
    }

    pub fn restart(&mut self, cx: &mut Context<Self>) {
        let options = options::build_run_options(&self.config, &self.servers);
        self.run_busy(cx, move |snapshot| {
            if let Some(options) = options {
                let _ = interception::restart_service(
                    &snapshot.paths,
                    &snapshot.platform,
                    &Singbox,
                    &options,
                );
            }
        });
    }

    pub fn add_rule(&mut self, input: &str, kind: RuleKind, cx: &mut Context<Self>) {
        let Some(rule) = domains::create_rule(input, kind) else {
            return;
        };
        self.mutate_and_apply(cx, move |config| config.domains.push(rule));
    }

    pub fn update_rule(&mut self, id: &str, input: &str, kind: RuleKind, cx: &mut Context<Self>) {
        let Some(rule) = domains::create_rule(input, kind) else {
            return;
        };
        let id = id.to_string();
        self.mutate_and_apply(cx, move |config| {
            if let Some(target) = config.domains.iter_mut().find(|rule| rule.id == id) {
                target.value = rule.value;
                target.kind = rule.kind;
            }
        });
    }

    pub fn toggle_rule(&mut self, id: &str, cx: &mut Context<Self>) {
        let id = id.to_string();
        self.mutate_and_apply(cx, move |config| {
            if let Some(rule) = config.domains.iter_mut().find(|rule| rule.id == id) {
                rule.enabled = !rule.enabled;
            }
        });
    }

    pub fn remove_rule(&mut self, id: &str, cx: &mut Context<Self>) {
        let id = id.to_string();
        self.mutate_and_apply(cx, move |config| {
            config.domains.retain(|rule| rule.id != id)
        });
    }

    pub fn select_server(&mut self, file: &str, cx: &mut Context<Self>) {
        let file = file.to_string();
        self.mutate_and_apply(cx, move |config| config.active_server = Some(file));
    }

    pub fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        self.mutate_and_apply(cx, move |config| config.mode = mode);
    }

    pub fn set_socks_port(&mut self, port: u16, cx: &mut Context<Self>) {
        if self.intercepting() || port == self.config.socks_port {
            return;
        }
        self.mutate_and_apply(cx, move |config| config.socks_port = port);
    }

    pub fn toggle_launch_at_login(&mut self, cx: &mut Context<Self>) {
        let installed = self.status.agent.installed;
        let intercept_agent = self.intercept_agent;
        let intercepting = self.intercepting();
        self.run_busy(cx, move |snapshot| {
            if installed {
                let _ = interception::uninstall_launch_at_login(&snapshot.platform);
                if intercept_agent {
                    let _ = interception::uninstall_intercept_at_login(&snapshot.platform);
                }
                if intercepting
                    && let Some(options) =
                        options::build_run_options(&snapshot.config, &snapshot.servers)
                {
                    let _ = interception::ensure_daemon(
                        &snapshot.paths,
                        &snapshot.platform,
                        &Singbox,
                        &options,
                    );
                }
            } else {
                ensure_launch_at_login(&snapshot);
            }
        });
    }

    pub fn toggle_intercept_at_login(&mut self, cx: &mut Context<Self>) {
        let installed = self.status.agent.installed;
        let intercept_agent = self.intercept_agent;
        self.run_busy(cx, move |snapshot| {
            if intercept_agent {
                let _ = interception::uninstall_intercept_at_login(&snapshot.platform);
                return;
            }
            if !installed && !ensure_launch_at_login(&snapshot) {
                return;
            }
            if let Some(cli) = interception::cli_path() {
                let _ = interception::install_intercept_at_login(
                    &snapshot.paths,
                    &snapshot.platform,
                    &cli,
                );
            }
        });
    }

    pub fn set_theme(&mut self, pref: ThemePref, cx: &mut Context<Self>) {
        self.config.theme = pref.as_str().to_string();
        let paths = self.paths.clone();
        let config = self.config.clone();
        spawn_task(
            cx,
            move || {
                let _ = config::save_config(&paths, &config);
            },
            |_, _, _| {},
        );
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            paths: self.paths.clone(),
            platform: self.platform.clone(),
            config: self.config.clone(),
            servers: self.servers.clone(),
        }
    }

    fn run_busy(&mut self, cx: &mut Context<Self>, work: impl FnOnce(Snapshot) + Send + 'static) {
        if self.busy {
            return;
        }
        self.busy = true;
        cx.notify();
        let snapshot = self.snapshot();
        spawn_task(
            cx,
            move || work(snapshot),
            |this, _, cx| {
                this.busy = false;
                cx.notify();
                this.refresh_status(cx);
            },
        );
    }

    fn mutate_and_apply(&mut self, cx: &mut Context<Self>, mutate: impl FnOnce(&mut AppConfig)) {
        mutate(&mut self.config);
        let restart = self.running();
        let snapshot = self.snapshot();
        spawn_task(
            cx,
            move || {
                let _ = config::save_config(&snapshot.paths, &snapshot.config);
                if restart
                    && let Some(options) =
                        options::build_run_options(&snapshot.config, &snapshot.servers)
                {
                    let _ = interception::restart_service(
                        &snapshot.paths,
                        &snapshot.platform,
                        &Singbox,
                        &options,
                    );
                }
            },
            |this, _, cx| {
                cx.notify();
                this.refresh_status(cx);
            },
        );
    }
}

fn is_broken(status: &InterceptionStatus) -> bool {
    status.intercepting() && !status.daemon.is_running()
}

fn ensure_launch_at_login(snapshot: &Snapshot) -> bool {
    let Some(options) = options::build_run_options(&snapshot.config, &snapshot.servers) else {
        return false;
    };
    interception::install_launch_at_login(&snapshot.paths, &snapshot.platform, &Singbox, &options)
        .is_ok()
}

fn status_snapshot(snapshot: &Snapshot, heal: bool) -> StatusSnapshot {
    let mut status = interception::interception_status(&snapshot.paths, &snapshot.platform);
    if heal && is_broken(&status) {
        let healed = options::build_run_options(&snapshot.config, &snapshot.servers).is_some_and(
            |options| {
                let ok = interception::ensure_daemon(
                    &snapshot.paths,
                    &snapshot.platform,
                    &Singbox,
                    &options,
                )
                .is_ok();
                if ok {
                    daemon::wait_for_reachable(
                        options.engine.socks_port,
                        8,
                        Duration::from_millis(250),
                    );
                }
                ok
            },
        );
        if !healed {
            let _ = interception::disable_interception(&snapshot.paths, &snapshot.platform);
        }
        status = interception::interception_status(&snapshot.paths, &snapshot.platform);
    }

    let ready = status
        .daemon
        .is_running()
        .then(|| daemon::is_reachable(snapshot.config.socks_port));
    StatusSnapshot {
        servers: servers::list_servers(&snapshot.paths),
        intercept_agent: snapshot.platform.is_intercept_agent_installed(),
        status,
        ready,
    }
}

fn poll_every(
    cx: &mut Context<AppState>,
    interval: Duration,
    mut tick: impl FnMut(&mut AppState, &mut Context<AppState>) + 'static,
) -> Task<()> {
    cx.spawn(async move |this, cx| {
        loop {
            cx.background_executor().timer(interval).await;
            if this.update(cx, |this, cx| tick(this, cx)).is_err() {
                break;
            }
        }
    })
}

fn spawn_task<R, W, D>(cx: &mut Context<AppState>, work: W, done: D)
where
    R: Send + 'static,
    W: FnOnce() -> R + Send + 'static,
    D: FnOnce(&mut AppState, R, &mut Context<AppState>) + 'static,
{
    cx.spawn(async move |this, cx| {
        let result = cx.background_executor().spawn(async move { work() }).await;
        let _ = this.update(cx, |this, cx| done(this, result, cx));
    })
    .detach();
}
