pub mod singbox;

use std::path::{Path, PathBuf};

use crate::config::Mode;
use crate::domains::DomainRule;

#[derive(Debug, Clone, PartialEq)]
pub struct EngineOptions {
    pub socks_port: u16,
    pub mode: Mode,
    pub domains: Vec<DomainRule>,
}

pub trait TunnelEngine {
    fn id(&self) -> &'static str;
    fn resolve_binary(&self) -> Option<PathBuf>;
    fn render_config(&self, server_conf: &str, options: &EngineOptions) -> anyhow::Result<String>;
    fn run_args(&self, config_path: &Path) -> Vec<String>;
}

pub fn require_binary(engine: &impl TunnelEngine) -> anyhow::Result<PathBuf> {
    engine.resolve_binary().ok_or_else(|| {
        anyhow::anyhow!(
            "{} binary not found; install it with `brew install {}`",
            engine.id(),
            engine.id()
        )
    })
}
