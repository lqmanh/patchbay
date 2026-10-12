use anyhow::bail;

#[derive(Default)]
pub(crate) struct WireguardConfig {
    pub addresses: Vec<String>,
    pub private_key: String,
    pub mtu: Option<u32>,
    pub dns: Vec<String>,
    pub peers: Vec<WireguardPeer>,
}

#[derive(Default)]
pub(crate) struct WireguardPeer {
    pub public_key: String,
    pub pre_shared_key: Option<String>,
    pub host: String,
    pub port: u32,
    pub allowed_ips: Vec<String>,
    pub keepalive: Option<u32>,
}

enum Section {
    Interface,
    Peer,
}

pub(crate) fn parse(text: &str) -> anyhow::Result<WireguardConfig> {
    let mut config = WireguardConfig::default();
    let mut section: Option<Section> = None;
    let mut peer: Option<WireguardPeer> = None;

    for raw_line in text.split('\n') {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(header) = header_name(line) {
            flush_peer(&mut config, &mut peer);
            if header.eq_ignore_ascii_case("peer") {
                section = Some(Section::Peer);
                peer = Some(WireguardPeer::default());
            } else {
                section = Some(Section::Interface);
            }
            continue;
        }

        let Some(separator) = line.find('=') else {
            continue;
        };
        let key = line[..separator].trim().to_lowercase();
        let value = line[separator + 1..].trim();

        match section {
            Some(Section::Interface) => match key.as_str() {
                "address" => config.addresses.extend(split_list(value)),
                "privatekey" => config.private_key = value.to_string(),
                "mtu" => config.mtu = parse_positive_int(value),
                "dns" => config.dns.extend(split_list(value)),
                _ => {}
            },
            Some(Section::Peer) => {
                let Some(peer) = peer.as_mut() else {
                    continue;
                };
                match key.as_str() {
                    "publickey" => peer.public_key = value.to_string(),
                    "presharedkey" => {
                        peer.pre_shared_key = (!value.is_empty()).then(|| value.to_string());
                    }
                    "allowedips" => peer.allowed_ips = split_list(value),
                    "persistentkeepalive" => peer.keepalive = parse_positive_int(value),
                    "endpoint" => {
                        if let Some((host, port)) = parse_endpoint(value) {
                            peer.host = host;
                            peer.port = port;
                        }
                    }
                    _ => {}
                }
            }
            None => {}
        }
    }
    flush_peer(&mut config, &mut peer);

    if config.addresses.is_empty() || config.private_key.is_empty() || config.peers.is_empty() {
        bail!("invalid WireGuard config: missing Address, PrivateKey, or Peer");
    }
    Ok(config)
}

fn flush_peer(config: &mut WireguardConfig, peer: &mut Option<WireguardPeer>) {
    if let Some(peer) = peer.take()
        && !peer.public_key.is_empty()
        && !peer.host.is_empty()
        && peer.port > 0
    {
        config.peers.push(peer);
    }
}

fn header_name(line: &str) -> Option<&str> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    let is_word = !inner.is_empty() && inner.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    is_word.then_some(inner)
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_positive_int(value: &str) -> Option<u32> {
    let parsed = parse_int(value)?;
    u32::try_from(parsed).ok().filter(|parsed| *parsed > 0)
}

// Mirrors Number.parseInt(value, 10): optional sign, then as many digits as possible.
fn parse_int(value: &str) -> Option<i64> {
    let value = value.trim_start();
    let (negative, digits) = match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    let digits: String = digits.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let parsed: i64 = digits.parse().ok()?;
    Some(if negative { -parsed } else { parsed })
}

fn parse_endpoint(value: &str) -> Option<(String, u32)> {
    let colon = value.rfind(':')?;
    let (host_part, port_part) = (&value[..colon], &value[colon + 1..]);
    if host_part.is_empty() || port_part.is_empty() {
        return None;
    }
    if !port_part.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let port: u32 = port_part.parse().ok()?;
    if port == 0 {
        return None;
    }
    let host = host_part
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host_part);
    if host.is_empty() {
        return None;
    }
    Some((host.to_string(), port))
}
