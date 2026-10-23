use gpui_kit::component::{
    ActiveTheme, Disableable, Icon, IconName, Sizable, StyledExt as _, button::Button, h_flex,
    select::Select, spinner::Spinner, switch::Switch, v_flex,
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
        let port = state
            .status
            .proxy
            .as_ref()
            .and_then(|proxy| proxy.loopback_port());
        let broken = state.broken_description();
        let intercepting = state.intercepting();
        let busy = state.busy;
        let active = state.active_server().map(|server| server.file.clone());
        let active_changed = self
            .server_select
            .read(cx)
            .selected_value()
            .map(String::as_str)
            != active.as_deref();

        if self.servers != state.servers {
            let items: Vec<ServerItem> = state.servers.iter().map(ServerItem::new).collect();
            self.servers = state.servers.clone();
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
            StateKind::On => {
                let mut description = tunnel.to_string();
                if let Some(pid) = pid {
                    description.push_str(&format!(" · daemon pid {pid}"));
                }
                if let Some(port) = port {
                    description.push_str(&format!(" · port {port}"));
                }
                description
            }
            StateKind::Broken => broken,
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
            .disabled(busy || self.servers.is_empty())
            .w(px(220.));

        let mode_control = Select::new(&self.mode_select)
            .accessibility_label("Mode")
            .disabled(busy)
            .w(px(220.));

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
                    "The server config the tunnel connects with.",
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
