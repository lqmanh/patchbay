use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme, IconName, Theme, scroll::Scrollbar, switch::Switch, v_virtual_list,
};
use gpui_kit::*;

use crate::app::AppShell;
use crate::widgets::empty_state;

const LOG_LINE_HEIGHT: f32 = 18.0;

impl AppShell {
    pub(crate) fn log_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement + 'static {
        let line_count = self.state.read(cx).log_lines.len();
        if self.log_sizes.len() != line_count {
            if self.log_follow {
                self.log_scroll.scroll_to_bottom();
            }
            self.log_sizes = Rc::new(vec![size(px(0.), px(LOG_LINE_HEIGHT)); line_count]);
        }

        let actions = Switch::new("log-follow")
            .label("Follow")
            .checked(self.log_follow)
            .on_change(cx.listener(|this, follow: &bool, _, cx| {
                this.log_follow = *follow;
                if *follow {
                    this.log_scroll.scroll_to_bottom();
                }
                cx.notify();
            }));

        let body = if line_count == 0 {
            empty_state(
                IconName::SquareTerminal,
                "No log output",
                "The tunnel writes its log while the daemon runs.",
                None,
            )
        } else {
            let sizes = self.log_sizes.clone();
            div()
                .relative()
                .size_full()
                .child(
                    v_virtual_list(
                        cx.entity(),
                        "log-list",
                        sizes,
                        move |this, range, _window, cx| {
                            let state = this.state.read(cx);
                            range
                                .filter_map(|ix| {
                                    state.log_lines.get(ix).map(|line| {
                                        div()
                                            .h(px(LOG_LINE_HEIGHT))
                                            .px_6()
                                            .flex()
                                            .items_center()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .font_family(cx.theme().mono_font_family.clone())
                                            .text_xs()
                                            .text_color(log_color(line, cx.theme()))
                                            .child(line.clone())
                                    })
                                })
                                .collect::<Vec<_>>()
                        },
                    )
                    .track_scroll(&self.log_scroll)
                    .size_full(),
                )
                .child(Scrollbar::vertical(&self.log_scroll))
                .into_any_element()
        };

        self.page("Log", "Output from the tunnel daemon", actions, body, cx)
    }
}

fn log_color(line: &str, theme: &Theme) -> Hsla {
    if line.contains("ERROR") || line.contains("WARN") {
        theme.danger
    } else if line.contains("endpoint/") {
        theme.success
    } else if line.contains("outbound/direct") {
        theme.muted_foreground
    } else {
        theme.foreground
    }
}
