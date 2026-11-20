use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domains::DomainRule;
use crate::json::write_json;
use crate::paths::Paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Split,
    Full,
}

/// Resolver for domains that don't go through the VPN.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DirectDns {
    #[default]
    System,
    Cloudflare,
    Google,
    Quad9,
}

impl DirectDns {
    pub const ALL: [Self; 4] = [Self::System, Self::Cloudflare, Self::Google, Self::Quad9];

    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Cloudflare => "Cloudflare",
            Self::Google => "Google",
            Self::Quad9 => "Quad9",
        }
    }

    /// DoH server address; `None` for `System`, which uses the gateway resolver.
    pub fn doh_server(self) -> Option<&'static str> {
        match self {
            Self::System => None,
            Self::Cloudflare => Some("1.1.1.1"),
            Self::Google => Some("8.8.8.8"),
            Self::Quad9 => Some("9.9.9.9"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub socks_port: u16,
    pub mode: Mode,
    pub active_server: Option<String>,
    pub domains: Vec<DomainRule>,
    pub theme: String,
    pub direct_dns: DirectDns,
}

pub const DEFAULT_SOCKS_PORT: u16 = 25344;
pub const MIN_SOCKS_PORT: u16 = 1024;
pub const DEFAULT_THEME: &str = "system";

pub fn default_config() -> AppConfig {
    AppConfig {
        socks_port: DEFAULT_SOCKS_PORT,
        mode: Mode::Split,
        active_server: None,
        domains: Vec::new(),
        theme: DEFAULT_THEME.to_string(),
        direct_dns: DirectDns::System,
    }
}

pub fn parse_port_input(text: &str) -> Option<u16> {
    text.trim().parse::<u16>().ok()
}

pub fn parse_port_text(text: &str) -> Option<u16> {
    parse_port_input(text).filter(|port| is_user_port(*port))
}

fn is_user_port(port: u16) -> bool {
    port >= MIN_SOCKS_PORT
}

pub(crate) fn parse_port(value: &Value) -> Option<u16> {
    let number = value.as_f64()?;
    if number.fract() == 0.0 && number > 0.0 && number < 65536.0 {
        Some(number as u16)
    } else {
        None
    }
}

pub fn sanitize_config(input: &Value) -> AppConfig {
    let Some(raw) = input.as_object() else {
        return default_config();
    };
    let mut config = default_config();
    if let Some(port) = raw
        .get("socksPort")
        .and_then(parse_port)
        .filter(|port| is_user_port(*port))
    {
        config.socks_port = port;
    }
    if raw.get("mode").and_then(Value::as_str) == Some("full") {
        config.mode = Mode::Full;
    }
    if let Some(server) = raw.get("activeServer").and_then(Value::as_str) {
        config.active_server = Some(server.to_string());
    }
    if let Some(domains) = raw.get("domains").and_then(Value::as_array) {
        config.domains = domains.iter().filter_map(parse_rule).collect();
    }
    if let Some(theme) = raw
        .get("theme")
        .and_then(Value::as_str)
        .filter(|theme| !theme.is_empty())
    {
        config.theme = theme.to_string();
    }
    if let Some(direct_dns) = raw
        .get("directDns")
        .and_then(|value| DirectDns::deserialize(value).ok())
    {
        config.direct_dns = direct_dns;
    }
    config
}

fn parse_rule(value: &Value) -> Option<DomainRule> {
    let rule = DomainRule::deserialize(value).ok()?;
    (!rule.value.is_empty()).then_some(rule)
}

pub fn load_config(paths: &Paths) -> AppConfig {
    std::fs::read_to_string(paths.config_file())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .map(|input| sanitize_config(&input))
        .unwrap_or_else(default_config)
}

pub fn save_config(paths: &Paths, config: &AppConfig) -> anyhow::Result<()> {
    write_json(&paths.config_file(), config)
}
