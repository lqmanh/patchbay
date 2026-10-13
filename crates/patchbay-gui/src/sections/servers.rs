use gpui_kit::component::{
    ActiveTheme, Icon, IconName, Sizable, StyledExt as _,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::app::AppShell;
use crate::widgets::{empty_state, scroll_list, separated_rows};

impl AppShell {
    pub(crate) fn servers_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement + 'static {
        let state = self.state.read(cx);
        let servers = state.servers.clone();
        let active = state.active_server().map(|server| server.file.clone());
        let servers_dir = state.paths.servers_dir();
        let platform = state.platform.clone();

        let muted = cx.theme().muted_foreground;
        let primary = cx.theme().primary;
        let list_active = cx.theme().list_active;
        let list_hover = cx.theme().list_hover;

        let reveal = {
            let dir = servers_dir.clone();
            move |_: &ClickEvent, _: &mut Window, _: &mut App| {
                let _ = platform.reveal_in_finder(&dir);
            }
        };

        let actions = h_flex()
            .gap_2()
            .child(
                Button::new("refresh-servers")
                    .ghost()
                    .small()
                    .icon(IconName::RefreshCw)
                    .tooltip("Refresh")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.update(cx, |state, cx| state.refresh_status(cx));
                    })),
            )
            .child(
                Button::new("reveal-servers")
                    .ghost()
                    .small()
                    .icon(IconName::FolderOpen)
                    .tooltip("Open servers folder")
                    .on_click(reveal.clone()),
            );

        let description = format!("Server configs from {}", servers_dir.display());

        let body = if servers.is_empty() {
            empty_state(
                IconName::FolderOpen,
                "No server configs",
                "Add .conf files to the servers folder to pick one.",
                Some(
                    Button::new("reveal-servers-empty")
                        .outline()
                        .label("Open servers folder")
                        .on_click(reveal)
                        .into_any_element(),
                ),
            )
        } else {
            let rows = separated_rows(
                cx,
                servers.into_iter().enumerate().map(|(ix, server)| {
                    let is_active = active.as_deref() == Some(server.file.as_str());
                    let file = server.file.clone();
                    let name = server.name.clone();
                    let file_label = server.file.clone();
                    div()
                        .id(ElementId::named_usize("server", ix))
                        .w_full()
                        .px_6()
                        .py_3()
                        .flex()
                        .items_center()
                        .gap_3()
                        .when(is_active, |this| this.bg(list_active))
                        .hover(|this| this.bg(list_hover))
                        .cursor(CursorStyle::PointingHand)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.state
                                .update(cx, |state, cx| state.select_server(&file, cx));
                        }))
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .child(div().text_sm().font_medium().child(name))
                                .child(div().text_xs().text_color(muted).child(file_label)),
                        )
                        .when(is_active, |this| {
                            this.child(Icon::new(IconName::CircleCheck).text_color(primary))
                        })
                        .into_any_element()
                }),
            );

            scroll_list("servers-list", &self.servers_scroll, rows)
        };

        self.page("Servers", description, actions, body, cx)
    }
}
