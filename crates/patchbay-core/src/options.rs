use crate::config::AppConfig;
use crate::engine::EngineOptions;
use crate::paths::Paths;
use crate::servers::{Protocol, Server, pick_server};

#[derive(Debug, Clone, PartialEq)]
pub struct RunOptions {
    pub server_file: String,
    pub protocol: Protocol,
    pub engine: EngineOptions,
}

pub fn build_run_options(config: &AppConfig, servers: &[Server]) -> Option<RunOptions> {
    let server = pick_server(servers, config.active_server.as_deref())?;
    Some(RunOptions {
        server_file: server.file.clone(),
        protocol: server.protocol,
        engine: EngineOptions {
            socks_port: config.socks_port,
            mode: config.mode,
            domains: config.domains.clone(),
        },
    })
}

pub fn require_run_options(
    paths: &Paths,
    config: &AppConfig,
    servers: &[Server],
) -> anyhow::Result<RunOptions> {
    build_run_options(config, servers)
        .ok_or_else(|| anyhow::anyhow!("no server config in {}", paths.servers_dir().display()))
}
