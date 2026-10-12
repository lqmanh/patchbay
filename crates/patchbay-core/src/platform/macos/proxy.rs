use crate::platform::SocksState;
use crate::platform::exec::{run, run_checked};

pub fn active_network_service() -> Option<String> {
    let device = parse_active_interface(&run("route", &["-n", "get", "default"]))?;
    parse_service_for_device(&run("networksetup", &["-listnetworkserviceorder"]), &device)
}

pub fn get_socks_proxy(service: &str) -> SocksState {
    parse_socks_state(&run("networksetup", &["-getsocksfirewallproxy", service]))
}

pub fn set_socks_proxy(service: &str, host: &str, port: u16) -> anyhow::Result<()> {
    let port = port.to_string();
    run_checked(
        "networksetup",
        &["-setsocksfirewallproxy", service, host, &port],
    )?;
    run_checked(
        "networksetup",
        &["-setsocksfirewallproxystate", service, "on"],
    )?;
    Ok(())
}

pub fn disable_socks_proxy(service: &str) -> anyhow::Result<()> {
    run_checked(
        "networksetup",
        &["-setsocksfirewallproxystate", service, "off"],
    )?;
    Ok(())
}

pub fn parse_active_interface(output: &str) -> Option<String> {
    for line in output.split('\n') {
        let line = line.trim_start_matches([' ', '\t']);
        let Some(value) = line.strip_prefix("interface:") else {
            continue;
        };
        let value = value.trim_matches([' ', '\t']);
        if !value.is_empty() && !value.chars().any(char::is_whitespace) {
            return Some(value.to_string());
        }
    }
    None
}

pub fn parse_service_for_device(output: &str, device: &str) -> Option<String> {
    let lines: Vec<&str> = output.split('\n').collect();
    for pair in lines.windows(2) {
        let Some(name) = service_name(pair[0]) else {
            continue;
        };
        if hardware_device(pair[1]) == Some(device) {
            return Some(name);
        }
    }
    None
}

pub fn parse_socks_state(output: &str) -> SocksState {
    SocksState {
        enabled: line_value(output, "Enabled:") == Some("Yes"),
        host: line_value(output, "Server:").unwrap_or("").to_string(),
        port: line_value(output, "Port:")
            .and_then(|port| port.parse::<u16>().ok())
            .unwrap_or(0),
    }
}

fn service_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix('(')?;
    let rest = strip_index(rest)?;
    let rest = rest.strip_prefix(')')?;
    if !rest.starts_with([' ', '\t']) || rest.len() < 2 {
        return None;
    }
    Some(rest.trim().to_string())
}

fn strip_index(rest: &str) -> Option<&str> {
    if let Some(rest) = rest.strip_prefix('*') {
        return Some(rest);
    }
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        None
    } else {
        Some(&rest[digits..])
    }
}

fn hardware_device(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("(Hardware Port: ")?;
    let comma = rest.find(',')?;
    if comma == 0 {
        return None;
    }
    let rest = rest[comma + 1..].trim_start_matches([' ', '\t']);
    let rest = rest.strip_prefix("Device:")?;
    let rest = rest.trim_start_matches([' ', '\t']);
    let rest = rest.strip_suffix(')')?;
    if rest.chars().any(char::is_whitespace) {
        return None;
    }
    Some(rest)
}

fn line_value<'a>(output: &'a str, key: &str) -> Option<&'a str> {
    output
        .split('\n')
        .find_map(|line| Some(line.strip_prefix(key)?.trim_matches([' ', '\t'])))
}
