use std::path::PathBuf;

use patchbay_core::paths::Paths;

fn paths(home: &str, vars: &[(&str, &str)]) -> Paths {
    Paths::new(home, |key| {
        vars.iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value.to_string())
    })
}

#[test]
fn uses_xdg_environment_overrides() {
    let paths = paths(
        "/home/test",
        &[
            ("XDG_CONFIG_HOME", "/tmp/xdg-config"),
            ("XDG_CACHE_HOME", "/tmp/xdg-cache"),
            ("XDG_STATE_HOME", "/tmp/xdg-state"),
            ("XDG_RUNTIME_DIR", "/tmp/xdg-runtime"),
        ],
    );

    assert_eq!(
        paths.config_dir(),
        PathBuf::from("/tmp/xdg-config/patchbay")
    );
    assert_eq!(paths.cache_dir(), PathBuf::from("/tmp/xdg-cache/patchbay"));
    assert_eq!(paths.state_dir(), PathBuf::from("/tmp/xdg-state/patchbay"));
    assert_eq!(
        paths.runtime_dir(),
        PathBuf::from("/tmp/xdg-runtime/patchbay")
    );
}

#[test]
fn falls_back_to_conventional_locations() {
    let paths = paths("/home/test", &[]);

    assert_eq!(
        paths.config_dir(),
        PathBuf::from("/home/test/.config/patchbay")
    );
    assert_eq!(
        paths.cache_dir(),
        PathBuf::from("/home/test/.cache/patchbay")
    );
    assert_eq!(
        paths.state_dir(),
        PathBuf::from("/home/test/.local/state/patchbay")
    );
    assert_eq!(
        paths.runtime_dir(),
        PathBuf::from("/home/test/.local/state/patchbay")
    );
}

#[test]
fn ignores_empty_environment_values() {
    let paths = paths(
        "/home/test",
        &[("XDG_CONFIG_HOME", ""), ("XDG_RUNTIME_DIR", "")],
    );

    assert_eq!(
        paths.config_dir(),
        PathBuf::from("/home/test/.config/patchbay")
    );
    assert_eq!(paths.runtime_dir(), paths.state_dir());
}
