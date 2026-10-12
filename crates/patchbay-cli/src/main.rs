mod smoke;

use std::process::ExitCode;
use std::time::Duration;

use patchbay_core::config;
use patchbay_core::daemon;
use patchbay_core::engine::singbox::Singbox;
use patchbay_core::interception;
use patchbay_core::options;
use patchbay_core::paths::Paths;
use patchbay_core::platform::macos::MacPlatform;
use patchbay_core::servers;

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("patchbay: {error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> anyhow::Result<u8> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some(command) if command == interception::INTERCEPT_ARGS[0] => {
            intercept(args.get(1).map(String::as_str))
        }
        Some("smoke") => smoke::run(args.get(1).map(String::as_str)),
        _ => {
            eprintln!("usage: patchbay intercept on|off | patchbay smoke [server]");
            Ok(1)
        }
    }
}

fn intercept(action: Option<&str>) -> anyhow::Result<u8> {
    if action != Some(interception::INTERCEPT_ARGS[1]) && action != Some("off") {
        eprintln!("usage: patchbay intercept on|off");
        return Ok(1);
    }

    let paths = Paths::from_env()?;
    let engine = Singbox;
    let platform = MacPlatform::from_env()?;

    let config = config::load_config(&paths);
    let servers = servers::list_servers(&paths);
    let options = options::require_run_options(&paths, &config, &servers)?;

    if action == Some("off") {
        interception::disable_interception(&paths, &platform)?;
        println!("interception off");
        return Ok(0);
    }

    daemon::wait_for_reachable(config.socks_port, 30, Duration::from_millis(500));
    interception::enable_interception(&paths, &platform, &engine, &options)?;
    println!("interception on");
    Ok(0)
}
