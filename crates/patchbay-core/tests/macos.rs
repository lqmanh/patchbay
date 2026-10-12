use patchbay_core::platform::SocksState;
use patchbay_core::platform::macos::{
    parse_active_interface, parse_service_for_device, parse_socks_state,
};

const ROUTE_OUTPUT: &str = r#"   route to: default
destination: default
       mask: default
    gateway: 192.168.1.1
  interface: en0
      flags: <UP,GATEWAY,DONE,STATIC,PRCLONING,GLOBAL>
"#;

const SERVICES_OUTPUT: &str = r#"An asterisk (*) denotes that a network service is disabled.
(1) USB 10/100/1000 LAN
(Hardware Port: USB 10/100/1000 LAN, Device: en5)

(2) Wi-Fi
(Hardware Port: Wi-Fi, Device: en0)

(3) Thunderbolt Bridge
(Hardware Port: Thunderbolt Bridge, Device: bridge0)

(4) MEGA VPN
(Hardware Port: nz.mega.MEGAVPN, Device: )
"#;

const SOCKS_OFF_OUTPUT: &str = r#"Enabled: No
Server: 
Port: 0
Authenticated Proxy Enabled: 0
"#;

#[test]
fn finds_the_default_interface() {
    assert_eq!(
        parse_active_interface(ROUTE_OUTPUT),
        Some("en0".to_string())
    );
    assert_eq!(parse_active_interface("nothing here"), None);
}

#[test]
fn maps_a_device_to_its_network_service_tolerating_empty_devices() {
    assert_eq!(
        parse_service_for_device(SERVICES_OUTPUT, "en0"),
        Some("Wi-Fi".to_string())
    );
    assert_eq!(
        parse_service_for_device(SERVICES_OUTPUT, "en5"),
        Some("USB 10/100/1000 LAN".to_string())
    );
    assert_eq!(
        parse_service_for_device(SERVICES_OUTPUT, "bridge0"),
        Some("Thunderbolt Bridge".to_string())
    );
    assert_eq!(parse_service_for_device(SERVICES_OUTPUT, "en99"), None);
}

#[test]
fn parses_socks_proxy_state() {
    assert_eq!(
        parse_socks_state(SOCKS_OFF_OUTPUT),
        SocksState {
            enabled: false,
            host: String::new(),
            port: 0,
        }
    );
    assert_eq!(
        parse_socks_state("Enabled: Yes\nServer: 127.0.0.1\nPort: 25344\n"),
        SocksState {
            enabled: true,
            host: "127.0.0.1".into(),
            port: 25344,
        }
    );
}
