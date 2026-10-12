use gpui_kit::component::{Disableable, select::Select, switch::Switch, v_flex};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::app::AppShell;
use crate::widgets::{field_row, group_box, group_title, page_body, separated_rows};

impl AppShell {
    pub(crate) fn settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + 'static {
        let state = self.state.read(cx);
        let agent = state.status.agent;
        let intercept_agent = state.intercept_agent;
        let busy = state.busy;

        let launch_description = if agent.installed {
            match (agent.running, agent.pid) {
                (true, Some(pid)) => format!("Running (pid {pid})"),
                _ => "Installed, not running".to_string(),
            }
        } else {
            "Start the tunnel when you log in".to_string()
        };

        let launch_control = Switch::new("launch-at-login")
            .checked(agent.installed)
            .disabled(busy)
            .on_change(cx.listener(|this, _, _, cx| {
                this.state
                    .update(cx, |state, cx| state.toggle_launch_at_login(cx));
            }));

        let intercept_control = Switch::new("intercept-at-login")
            .checked(intercept_agent)
            .disabled(busy)
            .on_change(cx.listener(|this, _, _, cx| {
                this.state
                    .update(cx, |state, cx| state.toggle_intercept_at_login(cx));
            }));

        let theme_control = Select::new(&self.theme_select)
            .accessibility_label("Theme")
            .disabled(busy)
            .w(px(140.));

        let startup_rows = separated_rows(
            cx,
            vec![
                field_row(cx, "Launch at login", launch_description, launch_control)
                    .into_any_element(),
                field_row(
                    cx,
                    "Intercept at login",
                    "Enable the system proxy when you log in",
                    intercept_control,
                )
                .into_any_element(),
            ],
        );

        let appearance_rows = separated_rows(
            cx,
            vec![
                field_row(
                    cx,
                    "Theme",
                    "Follows the system appearance when set to System",
                    theme_control,
                )
                .into_any_element(),
            ],
        );

        self.page(
            "Settings",
            "Startup and appearance",
            div(),
            page_body(
                "settings-scroll",
                [
                    v_flex()
                        .w_full()
                        .child(group_title(cx, "Startup"))
                        .child(group_box(cx, startup_rows))
                        .into_any_element(),
                    v_flex()
                        .w_full()
                        .child(group_title(cx, "Appearance"))
                        .child(group_box(cx, appearance_rows))
                        .into_any_element(),
                ],
            ),
            cx,
        )
    }
}
