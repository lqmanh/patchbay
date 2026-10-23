use std::path::PathBuf;

use crate::paths::Paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Wireguard,
    OpenVpn,
}

impl Protocol {
    pub fn label(self) -> &'static str {
        match self {
            Self::Wireguard => "WireGuard",
            Self::OpenVpn => "OpenVPN",
        }
    }

    fn from_file(file: &str) -> Option<Self> {
        if file.ends_with(".conf") {
            Some(Self::Wireguard)
        } else if file.ends_with(".ovpn") {
            Some(Self::OpenVpn)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Server {
    pub file: String,
    pub name: String,
    pub protocol: Protocol,
}

pub fn list_servers(paths: &Paths) -> Vec<Server> {
    let Ok(entries) = std::fs::read_dir(paths.servers_dir()) else {
        return Vec::new();
    };
    let mut servers: Vec<Server> = entries
        .flatten()
        .filter_map(|entry| {
            let file = entry.file_name().to_str()?.to_string();
            let protocol = Protocol::from_file(&file)?;
            let name = file.rsplit_once('.')?.0.to_string();
            entry.path().is_file().then_some(Server {
                file,
                name,
                protocol,
            })
        })
        .collect();
    servers.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.file.cmp(&right.file))
    });
    servers
}

pub fn server_path(paths: &Paths, file: &str) -> PathBuf {
    paths.servers_dir().join(file)
}

pub fn read_server_config(paths: &Paths, file: &str) -> anyhow::Result<String> {
    Ok(std::fs::read_to_string(server_path(paths, file))?)
}

pub fn resolve_active_server(paths: &Paths, active: Option<&str>) -> Option<Server> {
    pick_server(&list_servers(paths), active).cloned()
}

pub fn pick_server<'a>(servers: &'a [Server], active: Option<&str>) -> Option<&'a Server> {
    active
        .and_then(|active| servers.iter().find(|server| server.file == active))
        .or_else(|| servers.first())
}
