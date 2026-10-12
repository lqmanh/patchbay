mod generate;
mod wireguard;

pub use generate::generate_config;

use std::path::{Path, PathBuf};

use super::{EngineOptions, TunnelEngine};

pub struct Singbox;

const FALLBACK_BINARIES: [&str; 2] = ["/opt/homebrew/bin/sing-box", "/usr/local/bin/sing-box"];

impl TunnelEngine for Singbox {
    fn id(&self) -> &'static str {
        "sing-box"
    }

    fn resolve_binary(&self) -> Option<PathBuf> {
        which("sing-box").or_else(|| {
            FALLBACK_BINARIES
                .iter()
                .map(PathBuf::from)
                .find(|path| path.exists())
        })
    }

    fn render_config(&self, server_conf: &str, options: &EngineOptions) -> anyhow::Result<String> {
        generate_config(server_conf, options)
    }

    fn run_args(&self, config_path: &Path) -> Vec<String> {
        vec![
            "run".to_string(),
            "-c".to_string(),
            config_path.display().to_string(),
            "--disable-color".to_string(),
        ]
    }
}

fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(name);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}
