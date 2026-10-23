mod generate;
mod openvpn;
mod wireguard;

pub use generate::generate_config;

use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};

use super::{EngineOptions, TunnelEngine};
use crate::platform::exec::run_checked;
use crate::servers::Protocol;

pub struct Singbox;

const FALLBACK_BINARIES: [&str; 2] = ["/opt/homebrew/bin/sing-box", "/usr/local/bin/sing-box"];
const MIN_SINGBOX_VERSION: (u64, u64, u64) = (1, 14, 0);

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

    fn render_config(
        &self,
        protocol: Protocol,
        server_conf: &str,
        profile: &Path,
        options: &EngineOptions,
    ) -> anyhow::Result<String> {
        generate_config(protocol, server_conf, profile, options)
    }

    fn run_args(&self, config_path: &Path) -> Vec<String> {
        vec![
            "run".to_string(),
            "-c".to_string(),
            config_path.display().to_string(),
            "--disable-color".to_string(),
        ]
    }

    fn check_binary(&self, binary: &Path) -> anyhow::Result<()> {
        let text = run_checked(&binary.display().to_string(), &["version"])?;
        let version = parse_version(&text)
            .with_context(|| format!("reading the version of {}", binary.display()))?;
        if version < MIN_SINGBOX_VERSION {
            bail!(
                "patchbay needs sing-box {} or later ({} reports {})",
                format_version(MIN_SINGBOX_VERSION),
                binary.display(),
                format_version(version)
            );
        }
        Ok(())
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

fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let token = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("sing-box version "))?
        .split_whitespace()
        .next()?;
    let core = token.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

fn format_version(version: (u64, u64, u64)) -> String {
    format!("{}.{}.{}", version.0, version.1, version.2)
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    #[test]
    fn reads_sing_box_versions() {
        let banner = "sing-box version 1.14.3\n\nEnvironment: go1.26.9 darwin/arm64\n";
        assert_eq!(parse_version(banner), Some((1, 14, 3)));
        assert_eq!(
            parse_version("sing-box version 1.15.0-alpha.2"),
            Some((1, 15, 0))
        );
        assert_eq!(parse_version("sing-box version 2.0"), Some((2, 0, 0)));
        assert_eq!(parse_version("nonsense"), None);
    }

    #[test]
    fn requires_sing_box_1_14_0() {
        let dir = tempfile::tempdir().expect("tempdir");
        let binary = dir.path().join("sing-box");
        let script = |version: &str| format!("#!/bin/sh\necho 'sing-box version {version}'\n");

        std::fs::write(&binary, script("1.13.4")).expect("script");
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).expect("mode");
        let error = Singbox.check_binary(&binary).expect_err("old version");
        assert!(
            error
                .to_string()
                .contains(&format_version(MIN_SINGBOX_VERSION))
        );

        std::fs::write(&binary, script("1.14.0")).expect("script");
        assert!(Singbox.check_binary(&binary).is_ok());
    }
}
