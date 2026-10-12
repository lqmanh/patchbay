use gpui_kit::component::select::SelectItem;
use gpui_kit::*;

use patchbay_core::config::Mode;
use patchbay_core::servers::Server;

use crate::state::ThemePref;

#[derive(Clone, PartialEq)]
pub struct ModeItem {
    pub mode: Mode,
    pub label: &'static str,
}

impl SelectItem for ModeItem {
    type Value = Mode;

    fn title(&self) -> SharedString {
        self.label.into()
    }

    fn value(&self) -> &Mode {
        &self.mode
    }
}

pub fn mode_items() -> Vec<ModeItem> {
    vec![
        ModeItem {
            mode: Mode::Split,
            label: "Split (matched traffic)",
        },
        ModeItem {
            mode: Mode::Full,
            label: "Full (all traffic)",
        },
    ]
}

#[derive(Clone, PartialEq)]
pub struct ThemeItem {
    pub pref: ThemePref,
    pub label: &'static str,
}

impl SelectItem for ThemeItem {
    type Value = ThemePref;

    fn title(&self) -> SharedString {
        self.label.into()
    }

    fn value(&self) -> &ThemePref {
        &self.pref
    }
}

pub fn theme_items() -> Vec<ThemeItem> {
    ThemePref::ALL
        .into_iter()
        .map(|pref| ThemeItem {
            pref,
            label: pref.label(),
        })
        .collect()
}

#[derive(Clone, PartialEq)]
pub struct ServerItem {
    pub file: String,
    pub name: String,
}

impl ServerItem {
    pub fn new(server: &Server) -> Self {
        Self {
            file: server.file.clone(),
            name: server.name.clone(),
        }
    }
}

impl SelectItem for ServerItem {
    type Value = String;

    fn title(&self) -> SharedString {
        self.name.clone().into()
    }

    fn value(&self) -> &String {
        &self.file
    }
}
