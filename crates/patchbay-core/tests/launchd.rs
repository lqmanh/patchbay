use std::path::{Path, PathBuf};

use patchbay_core::platform::macos::{
    build_intercept_agent_plist, build_launch_agent_plist, intercept_agent_path, launch_agent_path,
};

#[test]
fn lives_in_the_users_launch_agents_directory() {
    assert_eq!(
        launch_agent_path(Path::new("/Users/test")),
        PathBuf::from("/Users/test/Library/LaunchAgents/io.github.lqmanh.patchbay.daemon.plist")
    );
}

#[test]
fn builds_a_keep_alive_plist_with_the_sing_box_arguments() {
    let command = [
        "/opt/homebrew/bin/sing-box".to_string(),
        "run".to_string(),
        "-c".to_string(),
        "/Users/me/.cache/patchbay/daemon.json".to_string(),
        "--disable-color".to_string(),
    ];
    let plist = build_launch_agent_plist(
        &command,
        Path::new("/Users/me/.local/state/patchbay/daemon.log"),
    );

    assert!(plist.contains("<string>io.github.lqmanh.patchbay.daemon</string>"));
    assert!(plist.contains("<key>KeepAlive</key>"));
    assert!(plist.contains("<key>RunAtLoad</key>"));
    assert!(plist.contains("<string>/opt/homebrew/bin/sing-box</string>"));
    assert!(plist.contains("<string>run</string>"));
    assert!(plist.contains("<string>-c</string>"));
    assert!(plist.contains("<string>/Users/me/.cache/patchbay/daemon.json</string>"));
    assert!(plist.contains("<string>--disable-color</string>"));
    assert!(plist.contains("<key>StandardOutPath</key>"));
}

#[test]
fn escapes_xml_sensitive_characters() {
    let command = [
        "/Users/me/&co/sing-box".to_string(),
        "run".to_string(),
        "-c".to_string(),
        "/tmp/<config>.json".to_string(),
        "--disable-color".to_string(),
    ];
    let plist = build_launch_agent_plist(&command, Path::new("/tmp/log"));

    assert!(plist.contains("&amp;co"));
    assert!(plist.contains("&lt;config&gt;"));
}

#[test]
fn builds_a_one_shot_intercept_plist() {
    let command = [
        "/usr/local/bin/patchbay".to_string(),
        "intercept".to_string(),
        "on".to_string(),
    ];
    let plist = build_intercept_agent_plist(&command, Path::new("/tmp/intercept.log"));

    assert!(plist.contains("<string>io.github.lqmanh.patchbay.intercept</string>"));
    assert!(plist.contains("<string>intercept</string>"));
    assert!(plist.contains("<string>on</string>"));
    assert!(plist.contains("<key>RunAtLoad</key>"));
    assert!(!plist.contains("KeepAlive"));
    assert!(
        intercept_agent_path(Path::new("/Users/test"))
            .to_string_lossy()
            .contains("io.github.lqmanh.patchbay.intercept.plist")
    );
}
