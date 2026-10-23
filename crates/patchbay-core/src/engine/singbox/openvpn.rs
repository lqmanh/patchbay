use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context as _, bail};
use serde::{Serialize, Serializer};

const DEFAULT_PORT: u16 = 1194;

#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Network {
    #[default]
    Udp,
    Tcp,
}

#[derive(Serialize)]
pub(crate) struct Remote {
    #[serde(rename = "server")]
    pub host: String,
    #[serde(rename = "server_port")]
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
}

#[derive(Default, Serialize)]
pub(crate) struct OpenVpnConfig {
    #[serde(skip)]
    pub dns: Vec<String>,
    pub servers: Vec<Remote>,
    #[serde(skip_serializing_if = "is_false")]
    pub remote_random: bool,
    pub network: Network,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub tls: Tls,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub data_ciphers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_ciphers_fallback: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression_lzo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_compression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mss_fix: Option<u32>,
    #[serde(skip_serializing_if = "is_false")]
    pub mss_fix_disabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fragment: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtu: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "seconds")]
    pub ping_interval: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "seconds")]
    pub ping_restart: Option<u32>,
}

#[derive(Default, Serialize)]
pub(crate) struct Tls {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_name_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_key: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub peer_fingerprint: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_certificate_tls: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_min: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cipher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_wrap: Option<ControlWrap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crl_path: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct ControlWrap {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn seconds<S>(value: &Option<u32>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(seconds) => serializer.collect_str(&format_args!("{seconds}s")),
        None => serializer.serialize_none(),
    }
}

pub(crate) fn parse(text: &str, profile: &Path) -> anyhow::Result<OpenVpnConfig> {
    let dir = profile.parent().unwrap_or_else(|| Path::new("."));
    let mut config = OpenVpnConfig::default();
    let mut inline: HashMap<String, String> = HashMap::new();
    let mut ca: Option<String> = None;
    let mut cert: Option<String> = None;
    let mut key: Option<String> = None;
    let mut key_direction: Option<String> = None;
    let mut auth_user_pass: Option<String> = None;
    let mut wants_credentials = false;

    let lines: Vec<&str> = text.lines().collect();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim();
        index += 1;
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if let Some(tag) = inline_tag(line) {
            if tag.starts_with('/') {
                continue;
            }
            let closing = format!("</{tag}>");
            let mut body = Vec::new();
            let mut closed = false;
            while index < lines.len() {
                let current = lines[index].trim();
                index += 1;
                if current.eq_ignore_ascii_case(&closing) {
                    closed = true;
                    break;
                }
                body.push(current);
            }
            if !closed {
                bail!("invalid OpenVPN config: unterminated <{tag}> block");
            }
            inline.insert(tag, body.join("\n"));
            continue;
        }

        let mut parts = split_args(line);
        if parts.is_empty() {
            continue;
        }
        let name = parts.remove(0).to_lowercase();
        match name.as_str() {
            "remote" => {
                let Some(host) = parts.first() else {
                    bail!("invalid OpenVPN config: remote without a host");
                };
                let port = match parts.get(1) {
                    Some(value) => value
                        .parse()
                        .with_context(|| format!("invalid OpenVPN remote port {value:?}"))?,
                    None => DEFAULT_PORT,
                };
                if port == 0 {
                    bail!("invalid OpenVPN remote port 0");
                }
                let network = parts.get(2).and_then(|value| parse_network(value));
                let host = host.trim_start_matches('[').trim_end_matches(']');
                config.servers.push(Remote {
                    host: host.to_string(),
                    port,
                    network,
                });
            }
            "remote-random" => config.remote_random = true,
            "proto" => {
                if let Some(network) = parts.first().and_then(|value| parse_network(value)) {
                    config.network = network;
                }
            }
            "dev" => {
                let dev = parts.first().map(String::as_str).unwrap_or("tun");
                if dev.starts_with("tap") {
                    bail!("OpenVPN TAP mode is not supported");
                }
                if !dev.starts_with("tun") {
                    bail!("OpenVPN device {dev:?} is not supported");
                }
            }
            "dev-type" => {
                if parts
                    .first()
                    .is_some_and(|kind| kind.eq_ignore_ascii_case("tap"))
                {
                    bail!("OpenVPN TAP mode is not supported");
                }
            }
            "auth-user-pass" => match parts.first().map(String::as_str) {
                Some("[inline]") => auth_user_pass = Some(inline_content(&inline, &name)?),
                Some(file) => auth_user_pass = Some(read_file(dir, file)?),
                None => wants_credentials = true,
            },
            "ca" => ca = parts.first().cloned(),
            "cert" => cert = parts.first().cloned(),
            "key" => key = parts.first().cloned(),
            "crl-verify" => {
                if let Some(path) = parts.first() {
                    config.tls.crl_path = Some(dir.join(path).display().to_string());
                }
            }
            "tls-auth" | "tls-crypt" | "tls-crypt-v2" => {
                let kind = control_wrap_kind(&name).expect("control wrap directive");
                let key = content(&name, parts.first().cloned(), &inline, dir)?
                    .with_context(|| format!("invalid OpenVPN config: missing {name} key"))?;
                let direction = if kind == "tls_auth" {
                    parts.get(1).and_then(|value| parse_direction(value))
                } else {
                    None
                };
                config.tls.control_wrap = Some(ControlWrap {
                    kind,
                    key,
                    direction,
                });
            }
            "key-direction" => {
                key_direction = parts.first().and_then(|value| parse_direction(value))
            }
            "verify-x509-name" => {
                config.tls.server_name = parts.first().cloned();
                config.tls.server_name_type = Some(
                    parts
                        .get(1)
                        .cloned()
                        .unwrap_or_else(|| "subject".to_string()),
                );
            }
            "remote-cert-tls" => {
                if let Some(kind) = parts
                    .first()
                    .filter(|value| matches!(value.as_str(), "server" | "client"))
                {
                    config.tls.remote_certificate_tls = Some(kind.clone());
                }
            }
            "peer-fingerprint" => {
                config.tls.peer_fingerprint.extend(
                    parts
                        .iter()
                        .map(|value| value.replace(':', "").to_ascii_lowercase()),
                );
            }
            "data-ciphers" => config.data_ciphers.extend(
                parts
                    .iter()
                    .flat_map(|value| value.split(':'))
                    .map(|value| value.trim_start_matches('?').to_string())
                    .filter(|value| !value.is_empty()),
            ),
            "data-ciphers-fallback" => config.data_ciphers_fallback = parts.first().cloned(),
            "cipher" => {
                if config.data_ciphers_fallback.is_none() {
                    config.data_ciphers_fallback = parts.first().cloned();
                }
            }
            "auth" => {
                config.auth = parts
                    .first()
                    .filter(|value| !value.eq_ignore_ascii_case("none"))
                    .cloned();
            }
            "comp-lzo" => {
                config.compression_lzo = Some(
                    parts
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "adaptive".to_string()),
                );
            }
            "compress" => {
                let mode = parts.first().cloned().unwrap_or_else(|| "stub".to_string());
                if mode != "migrate" {
                    config.compression = Some(mode);
                }
            }
            "allow-compression" => config.allow_compression = parts.first().cloned(),
            "mssfix" => match parts.first().and_then(|value| value.parse::<u32>().ok()) {
                Some(0) => config.mss_fix_disabled = true,
                Some(value) => config.mss_fix = Some(value),
                None => {}
            },
            "fragment" => {
                config.fragment = parts.first().and_then(|value| strict_positive_int(value))
            }
            "tun-mtu" => config.mtu = parts.first().and_then(|value| strict_positive_int(value)),
            "ping" => {
                config.ping_interval = parts.first().and_then(|value| strict_positive_int(value))
            }
            "ping-restart" | "ping-exit" => {
                config.ping_restart = parts.first().and_then(|value| strict_positive_int(value));
            }
            "keepalive" => {
                config.ping_interval = parts.first().and_then(|value| strict_positive_int(value));
                config.ping_restart = parts.get(1).and_then(|value| strict_positive_int(value));
            }
            "dhcp-option" => {
                let is_dns = parts.first().is_some_and(|kind| {
                    kind.eq_ignore_ascii_case("DNS") || kind.eq_ignore_ascii_case("DNS6")
                });
                if is_dns && let Some(server) = parts.get(1) {
                    config.dns.push(server.clone());
                }
            }
            "tls-version-min" => config.tls.version_min = parts.first().cloned(),
            "tls-version-max" => config.tls.version_max = parts.first().cloned(),
            "tls-cipher" => config.tls.cipher = parts.first().cloned(),
            "pkcs12" => {
                bail!("OpenVPN PKCS#12 profiles are not supported; use PEM ca/cert/key files")
            }
            "askpass" => {
                bail!("OpenVPN profiles that prompt for a key passphrase are not supported")
            }
            "secret" => bail!("OpenVPN static key mode is not supported"),
            "http-proxy" | "socks-proxy" => bail!("OpenVPN proxy directives are not supported"),
            _ => {}
        }
    }

    if inline.contains_key("connection") {
        bail!(
            "OpenVPN <connection> blocks are not supported; use a profile with a top-level remote"
        );
    }
    if inline.contains_key("secret") {
        bail!("OpenVPN static key mode is not supported");
    }
    if config.servers.is_empty() {
        bail!("invalid OpenVPN config: missing remote");
    }

    config.tls.certificate = content("ca", ca, &inline, dir)?;
    config.tls.client_certificate = content("cert", cert, &inline, dir)?;
    config.tls.client_key = content("key", key, &inline, dir)?;
    if config.tls.certificate.is_none() && config.tls.peer_fingerprint.is_empty() {
        bail!("invalid OpenVPN config: missing ca or peer-fingerprint");
    }
    if config.tls.client_certificate.is_some() != config.tls.client_key.is_some() {
        bail!("invalid OpenVPN config: cert and key must both be set or both be empty");
    }
    if config
        .tls
        .client_key
        .as_deref()
        .is_some_and(|key| key.contains("ENCRYPTED"))
    {
        bail!("encrypted OpenVPN client keys are not supported; decrypt the key first");
    }

    if config.tls.control_wrap.is_none() {
        for tag in ["tls-auth", "tls-crypt", "tls-crypt-v2"] {
            if let Some(key) = inline.get(tag) {
                config.tls.control_wrap = Some(ControlWrap {
                    kind: control_wrap_kind(tag).expect("control wrap tag"),
                    key: key.clone(),
                    direction: None,
                });
                break;
            }
        }
    }
    if let Some(wrap) = config.tls.control_wrap.as_mut()
        && wrap.kind == "tls_auth"
        && wrap.direction.is_none()
    {
        wrap.direction = key_direction;
    }

    let credentials = if let Some(text) =
        auth_user_pass.or_else(|| inline.get("auth-user-pass").cloned())
    {
        Some(text)
    } else if wants_credentials {
        let sidecar = profile.with_extension("auth");
        Some(std::fs::read_to_string(&sidecar).with_context(|| {
            format!(
                "OpenVPN credentials: add `auth-user-pass <file>` to {}, or create {} with the username and password on separate lines",
                profile.display(),
                sidecar.display()
            )
        })?)
    } else {
        None
    };
    if let Some(text) = credentials {
        let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
        let username = lines
            .next()
            .context("OpenVPN credentials: missing username")?;
        let password = lines
            .next()
            .context("OpenVPN credentials: missing password")?;
        config.username = Some(username.to_string());
        config.password = Some(password.to_string());
    }

    Ok(config)
}

fn inline_tag(line: &str) -> Option<String> {
    let inner = line.strip_prefix('<')?.strip_suffix('>')?;
    (!inner.is_empty()).then(|| inner.to_ascii_lowercase())
}

fn inline_content(inline: &HashMap<String, String>, tag: &str) -> anyhow::Result<String> {
    inline
        .get(tag)
        .cloned()
        .with_context(|| format!("invalid OpenVPN config: missing <{tag}> block"))
}

fn content(
    tag: &str,
    arg: Option<String>,
    inline: &HashMap<String, String>,
    dir: &Path,
) -> anyhow::Result<Option<String>> {
    match arg.as_deref() {
        None | Some("[inline]") => Ok(inline.get(tag).cloned()),
        Some(file) => Ok(Some(read_file(dir, file)?)),
    }
}

fn control_wrap_kind(tag: &str) -> Option<&'static str> {
    match tag {
        "tls-auth" => Some("tls_auth"),
        "tls-crypt" => Some("tls_crypt"),
        "tls-crypt-v2" => Some("tls_crypt_v2"),
        _ => None,
    }
}

fn read_file(dir: &Path, file: &str) -> anyhow::Result<String> {
    let path = dir.join(file);
    std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
}

fn parse_network(value: &str) -> Option<Network> {
    let value = value.to_ascii_lowercase();
    let base = value.trim_end_matches(|ch: char| ch.is_ascii_digit());
    match base {
        "udp" => Some(Network::Udp),
        "tcp" | "tcp-client" | "tcp-server" => Some(Network::Tcp),
        _ => None,
    }
}

fn parse_direction(value: &str) -> Option<String> {
    match value.to_ascii_lowercase().as_str() {
        "0" | "server" => Some("server".to_string()),
        "1" | "client" => Some("client".to_string()),
        _ => None,
    }
}

fn strict_positive_int(value: &str) -> Option<u32> {
    value.parse::<u32>().ok().filter(|value| *value > 0)
}

fn split_args(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in line.chars() {
        match quote {
            Some(open) if ch == open => quote = None,
            Some(_) => current.push(ch),
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            None => current.push(ch),
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_quoted_arguments() {
        assert_eq!(
            split_args(r#"auth-user-pass "/path with space/creds.txt""#),
            vec!["auth-user-pass", "/path with space/creds.txt"]
        );
        assert_eq!(
            split_args("remote vpn.example.com 1194"),
            vec!["remote", "vpn.example.com", "1194"]
        );
    }
}
