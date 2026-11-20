use gpui_kit::base::SelectableText;
use gpui_kit::component::{ActiveTheme, IconName, Theme, scroll::Scrollbar};
use gpui_kit::*;

use crate::app::AppShell;
use crate::widgets::empty_state;

/// Rows kept laid out beyond each edge of the viewport so scrolling stays smooth.
const LOG_OVERDRAW: Pixels = px(400.);

pub(crate) fn new_log_list(line_count: usize) -> ListState {
    let list = ListState::new(line_count, ListAlignment::Top, LOG_OVERDRAW);
    list.set_follow_mode(FollowMode::Tail);
    list
}

impl AppShell {
    /// Runs from the AppState observer. Head drops go first: `splice` shifts
    /// the scroll anchor with the content, so lines stay put while the window
    /// slides.
    pub(crate) fn sync_log_list(&mut self, cx: &App) {
        let (line_count, dropped_total) = {
            let state = self.state.read(cx);
            (state.log_lines.len(), state.log_dropped)
        };
        let dropped = dropped_total.saturating_sub(self.log_dropped_seen);
        if dropped > 0 {
            self.log_dropped_seen = dropped_total;
            self.log_list
                .splice(0..dropped.min(self.log_list.item_count()), 0);
        }
        let item_count = self.log_list.item_count();
        if item_count < line_count {
            self.log_list
                .splice(item_count..item_count, line_count - item_count);
        } else if item_count > line_count {
            // The log window shrank, e.g. the log was replaced or emptied.
            self.log_list.reset(line_count);
        }
    }

    pub(crate) fn log_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement + 'static {
        let body = if self.state.read(cx).log_lines.is_empty() {
            empty_state(
                IconName::SquareTerminal,
                "No log output",
                "The tunnel writes its log while the daemon runs.",
                None,
            )
        } else {
            let state = self.state.clone();
            let log_list = self.log_list.clone();
            div()
                .relative()
                .size_full()
                .child(
                    list(log_list, move |ix, _window, cx| {
                        let state = state.read(cx);
                        let Some(line) = state.log_lines.get(ix) else {
                            return div().into_any_element();
                        };
                        // The id follows the line as the window slides, so a
                        // selection does not jump when head lines drop.
                        let line_id = state.log_dropped + ix;
                        let color = log_color(line, cx.theme());
                        div()
                            .px_6()
                            .py_0p5()
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_xs()
                            .text_color(color)
                            .child(
                                SelectableText::new(line_id, line.clone())
                                    .document_order(line_id as u64),
                            )
                            .into_any_element()
                    })
                    .size_full()
                    .min_h_0(),
                )
                .child(Scrollbar::vertical(&self.log_list))
                .into_any_element()
        };

        self.page("Log", "Output from the tunnel daemon", div(), body, cx)
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
