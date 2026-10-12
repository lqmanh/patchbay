use patchbay_core::interception::{
    SavedProxyState, capture_state_before_enable, clear_proxy_state, load_proxy_state,
    save_proxy_state,
};
use patchbay_core::paths::Paths;
use patchbay_core::platform::SocksState;

#[test]
fn does_not_overwrite_a_state_we_already_own() {
    let current = SocksState {
        enabled: true,
        host: "127.0.0.1".into(),
        port: 25344,
    };
    assert_eq!(capture_state_before_enable(&current, 25344), None);
}

#[test]
fn captures_disabled_and_foreign_proxy_states() {
    let disabled = SocksState {
        enabled: false,
        host: String::new(),
        port: 0,
    };
    assert_eq!(
        capture_state_before_enable(&disabled, 25344),
        Some(disabled.clone())
    );

    let foreign = SocksState {
        enabled: true,
        host: "10.0.0.1".into(),
        port: 1080,
    };
    assert_eq!(
        capture_state_before_enable(&foreign, 25344),
        Some(foreign.clone())
    );
}

#[test]
fn round_trips_saved_state() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = Paths::new("/home/test", |name| match name {
        "XDG_STATE_HOME" => Some(dir.path().display().to_string()),
        _ => None,
    });

    let state = SavedProxyState {
        service: "Wi-Fi".into(),
        enabled: true,
        host: "10.0.0.1".into(),
        port: 1080,
    };
    save_proxy_state(&paths, &state).expect("save");
    assert_eq!(load_proxy_state(&paths), Some(state));

    clear_proxy_state(&paths);
    assert_eq!(load_proxy_state(&paths), None);
}
