use patchbay_core::paths::Paths;
use patchbay_core::servers::{Protocol, Server, list_servers};

fn paths(dir: &tempfile::TempDir) -> Paths {
    Paths::new(dir.path(), |_| None)
}

#[test]
fn lists_wireguard_and_openvpn_configs_and_skips_other_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = paths(&dir);
    std::fs::create_dir_all(paths.servers_dir()).expect("servers dir");
    for file in ["work.conf", "home.ovpn", "notes.txt"] {
        std::fs::write(paths.servers_dir().join(file), "").expect("server file");
    }

    assert_eq!(
        list_servers(&paths),
        vec![
            Server {
                file: "home.ovpn".into(),
                name: "home".into(),
                protocol: Protocol::OpenVpn,
            },
            Server {
                file: "work.conf".into(),
                name: "work".into(),
                protocol: Protocol::Wireguard,
            },
        ]
    );
}
