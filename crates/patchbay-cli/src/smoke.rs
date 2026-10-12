use std::process::Command;
use std::time::Duration;

use patchbay_core::config::{DEFAULT_SOCKS_PORT, Mode};
use patchbay_core::daemon::{self, DaemonStatus};
use patchbay_core::domains::{RuleKind, create_rule};
use patchbay_core::engine::singbox::Singbox;
use patchbay_core::engine::{EngineOptions, TunnelEngine};
use patchbay_core::options::RunOptions;
use patchbay_core::paths::Paths;
use patchbay_core::platform::Platform;
use patchbay_core::platform::macos::MacPlatform;
use patchbay_core::servers;

pub fn run(requested: Option<&str>) -> anyhow::Result<u8> {
    let paths = Paths::from_env()?;
    let engine = Singbox;
    let platform = MacPlatform::from_env()?;
    let mut failures: Vec<String> = Vec::new();

    let binary = engine.resolve_binary();
    println!(
        "sing-box: {}",
        binary
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "NOT FOUND".to_string())
    );

    let servers = servers::list_servers(&paths);
    let names = servers
        .iter()
        .map(|server| server.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "servers: {}",
        if names.is_empty() {
            "none".to_string()
        } else {
            names
        }
    );

    let server = match requested {
        Some(requested) => servers
            .iter()
            .find(|server| server.file == requested)
            .or_else(|| servers.iter().find(|server| server.name == requested))
            .cloned(),
        None => servers::resolve_active_server(&paths, None),
    };
    let (Some(_), Some(server)) = (binary, server) else {
        eprintln!(
            "missing sing-box binary or server config (requested: {})",
            requested.unwrap_or("first")
        );
        return Ok(1);
    };

    if let Some(service) = platform.active_network_service() {
        let proxy = platform.socks_proxy(&service);
        println!(
            "active service: {service} {}",
            serde_json::to_string(&proxy).unwrap_or_default()
        );
    }

    daemon::stop(&paths)?;
    let options = RunOptions {
        server_file: server.file.clone(),
        engine: EngineOptions {
            socks_port: DEFAULT_SOCKS_PORT,
            mode: Mode::Split,
            domains: vec![
                create_rule("reddit.com", RuleKind::Domain).expect("reddit rule"),
                create_rule("medium.com", RuleKind::Domain).expect("medium rule"),
            ],
        },
    };

    let outcome = run_checks(&paths, &engine, &options, &mut failures);

    if std::env::var_os("SMOKE_KEEP").is_some() {
        println!("SMOKE_KEEP set, leaving daemon running");
    } else {
        daemon::stop(&paths)?;
        println!("stopped: {:?}", daemon::status(&paths));
    }
    outcome?;

    if failures.is_empty() {
        println!("smoke passed");
        Ok(0)
    } else {
        eprintln!("smoke failed: {}", failures.join(", "));
        Ok(1)
    }
}

fn run_checks(
    paths: &Paths,
    engine: &Singbox,
    options: &RunOptions,
    failures: &mut Vec<String>,
) -> anyhow::Result<()> {
    let proxy_url = format!("socks5h://127.0.0.1:{}", options.engine.socks_port);
    let pid = daemon::start(paths, engine, options)?;
    println!("started daemon, pid {pid}");
    check(
        failures,
        "daemon reports running",
        matches!(daemon::status(paths), DaemonStatus::Running { .. }),
    );

    let reachable = daemon::wait_for_reachable(DEFAULT_SOCKS_PORT, 50, Duration::from_millis(200));
    check(failures, "socks port reachable", reachable);

    let (reddit_code, reddit_status) = curl(&[
        "-s",
        "-o",
        "/dev/null",
        "-w",
        "%{http_code}",
        "--proxy",
        &proxy_url,
        "--max-time",
        "20",
        "https://www.reddit.com",
    ]);
    println!(
        "reddit via proxy: {}",
        if reddit_code.is_empty() {
            format!(
                "curl exited {}",
                reddit_status.map_or_else(|| "null".to_string(), |code| code.to_string())
            )
        } else {
            reddit_code.clone()
        }
    );
    check(
        failures,
        "reddit reachable via proxy",
        is_http_success(&reddit_code),
    );

    let (exit_ip, _) = curl(&[
        "-s",
        "--max-time",
        "10",
        "--proxy",
        &proxy_url,
        "https://ipinfo.io/ip",
    ]);
    let (direct_ip, _) = curl(&["-s", "--max-time", "10", "https://ipinfo.io/ip"]);
    let exit_ip = exit_ip.trim().to_string();
    let direct_ip = direct_ip.trim().to_string();
    println!("exit via proxy (unmatched, expect direct): {exit_ip}");
    println!("exit direct: {direct_ip}");
    check(
        failures,
        "unmatched traffic exits direct",
        !exit_ip.is_empty() && exit_ip == direct_ip,
    );

    std::thread::sleep(Duration::from_millis(500));
    let log = daemon::read_log(paths, 400, 800).join("\n");
    check(
        failures,
        "tunneled connections logged",
        log.contains("endpoint/wireguard"),
    );
    check(
        failures,
        "direct connections logged",
        log.contains("outbound/direct"),
    );

    println!("--- full mode ---");
    let full_options = RunOptions {
        server_file: options.server_file.clone(),
        engine: EngineOptions {
            socks_port: DEFAULT_SOCKS_PORT,
            mode: Mode::Full,
            domains: Vec::new(),
        },
    };
    let full_pid = daemon::restart(paths, engine, &full_options)?;
    println!("started full-mode daemon, pid {full_pid}");
    check(
        failures,
        "full-mode socks port reachable",
        daemon::wait_for_reachable(DEFAULT_SOCKS_PORT, 50, Duration::from_millis(200)),
    );

    let (full_exit_ip, _) = curl(&[
        "-s",
        "--max-time",
        "30",
        "--proxy",
        &proxy_url,
        "https://ipinfo.io/ip",
    ]);
    let full_exit_ip = full_exit_ip.trim().to_string();
    println!("exit via proxy (full mode, expect VPN): {full_exit_ip}");
    check(
        failures,
        "full mode tunnels unmatched traffic",
        !full_exit_ip.is_empty() && full_exit_ip != direct_ip,
    );

    println!("status: {:?}", daemon::status(paths));
    Ok(())
}

fn check(failures: &mut Vec<String>, label: &str, ok: bool) {
    if ok {
        println!("ok: {label}");
    } else {
        println!("FAIL: {label}");
        failures.push(label.to_string());
    }
}

fn curl(args: &[&str]) -> (String, Option<i32>) {
    match Command::new("curl").args(args).output() {
        Ok(output) => (
            String::from_utf8_lossy(&output.stdout).into_owned(),
            output.status.code(),
        ),
        Err(_) => (String::new(), None),
    }
}

fn is_http_success(code: &str) -> bool {
    code.parse::<u16>()
        .is_ok_and(|code| (100..=999).contains(&code))
}
