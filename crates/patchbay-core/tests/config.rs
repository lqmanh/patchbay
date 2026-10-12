use patchbay_core::config::{AppConfig, Mode, default_config, sanitize_config};
use patchbay_core::domains::RuleKind;
use serde_json::json;

#[test]
fn sanitize_falls_back_to_defaults() {
    assert_eq!(sanitize_config(&json!(null)), default_config());
    assert_eq!(sanitize_config(&json!("nope")), default_config());
}

#[test]
fn sanitize_coerces_and_filters() {
    let sanitized = sanitize_config(&json!({
        "socksPort": 70000,
        "mode": "sideways",
        "activeServer": 5,
        "domains": [
            { "id": "a", "value": "reddit.com", "kind": "domain", "enabled": true },
            { "id": "", "value": "", "kind": "regex", "enabled": false }
        ],
        "theme": ""
    }));

    assert_eq!(
        sanitized,
        AppConfig {
            domains: vec![patchbay_core::domains::DomainRule {
                id: "a".into(),
                value: "reddit.com".into(),
                kind: RuleKind::Domain,
                enabled: true,
            }],
            ..default_config()
        }
    );
}

#[test]
fn sanitize_keeps_valid_values() {
    let sanitized = sanitize_config(&json!({
        "socksPort": 1080,
        "mode": "full",
        "activeServer": "b.conf",
        "domains": [],
        "theme": "dark"
    }));

    assert_eq!(
        sanitized,
        AppConfig {
            socks_port: 1080,
            mode: Mode::Full,
            active_server: Some("b.conf".into()),
            theme: "dark".into(),
            ..default_config()
        }
    );
}
