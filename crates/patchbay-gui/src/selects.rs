use gpui_kit::component::{ActiveTheme, select::SelectItem, v_flex};
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
pub struct ServerItem(Server);

impl ServerItem {
    pub fn new(server: &Server) -> Self {
        Self(server.clone())
    }
}

impl SelectItem for ServerItem {
    type Value = String;

    fn title(&self) -> SharedString {
        format!("{} · {}", self.0.name, self.0.protocol.label()).into()
    }

    fn display_title(&self) -> Option<AnyElement> {
        Some(self.0.name.clone().into_any_element())
    }

    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .child(div().truncate().child(self.0.name.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .truncate()
                    .child(format!("{} · {}", self.0.protocol.label(), self.0.file)),
            )
    }

    fn value(&self) -> &String {
        &self.0.file
    }
}
