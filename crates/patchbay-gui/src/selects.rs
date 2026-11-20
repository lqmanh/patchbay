use gpui_kit::component::{ActiveTheme, h_flex, select::SelectItem, v_flex};
use gpui_kit::*;

use patchbay_core::config::{DirectDns, Mode};
use patchbay_core::servers::Server;

use crate::state::ThemePref;

// Keeps both ends visible: server names share a prefix and differ at the end.
fn middle_truncated(text: impl Into<SharedString>) -> Div {
    div()
        .min_w_0()
        .truncate()
        .text_ellipsis_middle()
        .child(text.into())
}

/// A short name for the closed dropdown, with muted details in the open list.
#[derive(Clone, PartialEq)]
pub struct DetailItem<V> {
    pub value: V,
    pub name: &'static str,
    pub detail: SharedString,
}

impl<V> DetailItem<V> {
    fn new(value: V, name: &'static str, detail: impl Into<SharedString>) -> Self {
        Self {
            value,
            name,
            detail: detail.into(),
        }
    }
}

impl<V: Clone + PartialEq> SelectItem for DetailItem<V> {
    type Value = V;

    fn title(&self) -> SharedString {
        format!("{} · {}", self.name, self.detail).into()
    }

    fn display_title(&self) -> Option<AnyElement> {
        Some(self.name.into_any_element())
    }

    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .gap_2()
            .child(self.name)
            .child(middle_truncated(self.detail.clone()).text_color(cx.theme().muted_foreground))
    }

    fn value(&self) -> &V {
        &self.value
    }
}

pub fn mode_items() -> Vec<DetailItem<Mode>> {
    vec![
        DetailItem::new(Mode::Split, "Split", "Matched traffic"),
        DetailItem::new(Mode::Full, "Full", "All traffic"),
    ]
}

pub fn direct_dns_items() -> Vec<DetailItem<DirectDns>> {
    DirectDns::ALL
        .into_iter()
        .map(|dns| {
            let detail = match dns.doh_server() {
                Some(server) => format!("{server} · DoH"),
                None => "Network DNS".to_string(),
            };
            DetailItem::new(dns, dns.label(), detail)
        })
        .collect()
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
        Some(middle_truncated(self.0.name.clone()).into_any_element())
    }

    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex().child(middle_truncated(self.0.name.clone())).child(
            middle_truncated(format!("{} · {}", self.0.protocol.label(), self.0.file))
                .text_xs()
                .text_color(cx.theme().muted_foreground),
        )
    }

    fn value(&self) -> &String {
        &self.0.file
    }
}
