use patchbay_core::config::{AppConfig, default_config};
use patchbay_core::options::build_run_options;
use patchbay_core::servers::{Protocol, Server};

fn servers() -> Vec<Server> {
    vec![
        Server {
            file: "a.conf".into(),
            name: "a".into(),
            protocol: Protocol::Wireguard,
        },
        Server {
            file: "b.ovpn".into(),
            name: "b".into(),
            protocol: Protocol::OpenVpn,
        },
    ]
}

#[test]
fn prefers_the_active_server() {
    let config = AppConfig {
        active_server: Some("b.ovpn".into()),
        ..default_config()
    };
    let options = build_run_options(&config, &servers()).expect("options");
    assert_eq!(options.server_file, "b.ovpn");
    assert_eq!(options.protocol, Protocol::OpenVpn);
}

#[test]
fn falls_back_to_the_first_server() {
    let options = build_run_options(&default_config(), &servers()).expect("options");
    assert_eq!(options.server_file, "a.conf");
    assert_eq!(options.protocol, Protocol::Wireguard);
}

#[test]
fn returns_none_without_any_server() {
    assert_eq!(build_run_options(&default_config(), &[]), None);
}
