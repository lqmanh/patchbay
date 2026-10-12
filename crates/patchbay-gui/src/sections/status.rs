use gpui_kit::component::{
    ActiveTheme, Disableable, Icon, IconName, Sizable, StyledExt as _, button::Button, h_flex,
    input::Input, select::Select, spinner::Spinner, switch::Switch, v_flex,
};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::app::AppShell;
use crate::selects::ServerItem;
use crate::state::StateKind;
use crate::widgets::{field_row, group_box, page_body, separated_rows};

impl AppShell {
    pub(crate) fn status_section(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + 'static {
        let state = self.state.read(cx);
        let kind = state.state_kind();
        let tunnel = state.tunnel_label();
        let pid = state.status.daemon.pid();
        let intercepting = state.intercepting();
        let busy = state.busy;
        let active = state.active_server().map(|server| server.file.clone());

        let items_changed = self.server_items.len() != state.servers.len()
            || self
                .server_items
                .iter()
                .zip(&state.servers)
                .any(|(item, server)| item.file != server.file || item.name != server.name);
        let active_changed = self
            .server_select
            .read(cx)
            .selected_value()
            .map(String::as_str)
            != active.as_deref();
        if items_changed {
            let items: Vec<ServerItem> = state.servers.iter().map(ServerItem::new).collect();
            self.server_items = items.clone();
            self.server_select
                .update(cx, |select, cx| select.set_items(items, window, cx));
        }
        if active_changed {
            self.server_select.update(cx, |select, cx| match &active {
                Some(file) => select.set_selected_value(file, window, cx),
                None => select.set_selected_index(None, window, cx),
            });
        }

        let muted = cx.theme().muted_foreground;
        let state_color = kind.color(cx.theme());
        let state_description = match kind {
            StateKind::On => match pid {
                Some(pid) => format!("{tunnel} · daemon pid {pid}"),
                None => tunnel.to_string(),
            },
            StateKind::Broken => {
                "The system proxy points at the tunnel, but the daemon is not running.".to_string()
            }
            StateKind::Off => {
                "Traffic goes out directly. Turn on interception to route it through the tunnel."
                    .to_string()
            }
        };

        let interception_control = h_flex()
            .items_center()
            .gap_3()
            .when(busy, |this| this.child(Spinner::new().small()))
            .child(
                Switch::new("interception")
                    .checked(intercepting)
                    .disabled(busy)
                    .on_change(cx.listener(|this, _, _, cx| {
                        this.state
                            .update(cx, |state, cx| state.toggle_interception(cx));
                    })),
            );

        let active_control = Select::new(&self.server_select)
            .placeholder("No server")
            .accessibility_label("Active server")
            .disabled(busy || self.server_items.is_empty())
            .w(px(220.));

        let mode_control = Select::new(&self.mode_select)
            .accessibility_label("Mode")
            .disabled(busy)
            .w(px(220.));

        let port_description = if intercepting {
            "Turn off interception to change the port."
        } else {
            "Local port the tunnel listens on."
        };

        let restart_control = Button::new("restart-tunnel")
            .outline()
            .icon(IconName::RefreshCw)
            .label("Restart")
            .loading(busy)
            .disabled(busy)
            .on_click(cx.listener(|this, _, _, cx| {
                this.state.update(cx, |state, cx| state.restart(cx));
            }));

        let rows = separated_rows(
            cx,
            vec![
                field_row(
                    cx,
                    "Interception",
                    "Route matched traffic through the system SOCKS proxy.",
                    interception_control,
                )
                .into_any_element(),
                field_row(
                    cx,
                    "Active server",
                    "The sing-box config the tunnel runs with.",
                    active_control,
                )
                .into_any_element(),
                field_row(
                    cx,
                    "Mode",
                    "Split routes only matched traffic; Full routes all traffic.",
                    mode_control,
                )
                .into_any_element(),
                field_row(
                    cx,
                    "SOCKS port",
                    port_description,
                    Input::new(&self.port_input)
                        .disabled(intercepting)
                        .w(px(120.)),
                )
                .into_any_element(),
                field_row(
                    cx,
                    "Tunnel daemon",
                    "Restart to apply config changes or recover a stuck tunnel.",
                    restart_control,
                )
                .into_any_element(),
            ],
        );

        let header = h_flex()
            .w_full()
            .items_center()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size_12()
                    .flex_shrink_0()
                    .rounded(cx.theme().radius_lg)
                    .bg(cx.theme().muted)
                    .text_color(state_color)
                    .child(Icon::new(kind.icon()).large()),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_xl().font_semibold().child(kind.label()))
                    .child(div().text_sm().text_color(muted).child(state_description)),
            );

        self.page(
            "Status",
            "Current tunnel and interception state",
            div(),
            page_body(
                "status-scroll",
                [
                    header.into_any_element(),
                    group_box(cx, rows).into_any_element(),
                ],
            ),
            cx,
        )
    }
}
