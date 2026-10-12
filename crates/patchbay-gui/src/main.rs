use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, Theme, ThemeMode};
use gpui_kit::*;

struct PatchbayApp;

impl Render for PatchbayApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme_name = Theme::global(cx).theme_name().clone();

        div()
            .flex()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .gap_4()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(format!("Patchbay — theme: {theme_name}"))
            .child(
                Button::new("toggle-theme")
                    .primary()
                    .label("Toggle light/dark")
                    .on_click(|_, window, cx| {
                        let mode = Theme::global(cx).mode;
                        Theme::change(
                            if mode.is_dark() {
                                ThemeMode::Light
                            } else {
                                ThemeMode::Dark
                            },
                            Some(window),
                            cx,
                        );
                    }),
            )
    }
}

fn main() {
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);
        open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("Patchbay".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            cx,
            |window, cx| {
                Theme::sync_system_appearance(Some(window), cx);
                cx.new(|_| PatchbayApp)
            },
        )
        .expect("failed to open window");
    });
}
