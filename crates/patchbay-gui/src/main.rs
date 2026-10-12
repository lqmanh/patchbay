mod app;
mod sections;
mod selects;
mod state;
mod widgets;

use gpui_kit::assets::Assets;
use gpui_kit::component::{
    ActiveTheme, StyledExt as _, TitleBar, WindowExt as _, dialog::AlertDialog, h_flex, link::Link,
    v_flex,
};
use gpui_kit::*;

use app::{AppShell, Section};
use state::AppState;

/// The main window and its view. Menu commands run while AppKit reports no
/// active window (menu tracking clears `mainWindow`), so handlers cannot rely
/// on `App::active_window`.
struct MainWindow {
    handle: AnyWindowHandle,
    shell: Entity<AppShell>,
}

impl Global for MainWindow {}

actions!(
    patchbay,
    [
        Quit,
        OpenSettings,
        ShowAbout,
        ToggleInterception,
        RestartTunnel,
        HideApp,
        MinimizeWindow,
        CloseWindow,
    ]
);

fn main() {
    application()
        .with_assets(Assets)
        .with_quit_mode(QuitMode::Explicit)
        .run(|cx| {
            init(cx);

            let state = match AppState::new() {
                Ok(mut app_state) => cx.new(|cx| {
                    app_state.start(cx);
                    app_state
                }),
                Err(error) => {
                    eprintln!("patchbay: {error}");
                    cx.quit();
                    return;
                }
            };

            let theme = state.read(cx).theme();
            theme.apply(None, cx);

            register_actions(cx, state.clone());
            register_menus(cx);

            let window_options = WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(940.), px(620.)), cx)),
                window_min_size: Some(size(px(760.), px(520.))),
                ..TitleBar::window_options()
            };
            match open_window(window_options, cx, {
                let state = state.clone();
                move |window, cx| {
                    let theme = state.read(cx).theme();
                    theme.apply(Some(window), cx);
                    // Quit without closing the window: closing it aborts in
                    // AppKit's Touch Bar KVO teardown (gpui accesskit bug).
                    window.on_window_should_close(cx, |_, cx| {
                        cx.quit();
                        false
                    });
                    cx.new(|cx| AppShell::new(state.clone(), window, cx))
                }
            }) {
                Ok((handle, shell)) => cx.set_global(MainWindow { handle, shell }),
                Err(error) => {
                    eprintln!("patchbay: {error}");
                    cx.quit();
                }
            }
        });
}

fn register_actions(cx: &mut App, state: Entity<AppState>) {
    cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
    cx.on_action(|_: &HideApp, cx: &mut App| cx.hide());
    cx.on_action(|_: &MinimizeWindow, cx: &mut App| {
        with_main_window(cx, |window, _| window.minimize_window());
    });
    // Quit like the close button; removing the window crashes.
    cx.on_action(|_: &CloseWindow, cx: &mut App| cx.quit());
    cx.on_action(|_: &OpenSettings, cx: &mut App| {
        let Some(main) = main_window(cx) else {
            return;
        };
        main.shell.update(cx, |shell, cx| {
            shell.section = Section::Settings;
            cx.notify();
        });
    });
    cx.on_action(|_: &ShowAbout, cx: &mut App| show_about(cx));
    cx.on_action({
        let state = state.clone();
        move |_: &ToggleInterception, cx: &mut App| {
            state.update(cx, |state, cx| state.toggle_interception(cx));
        }
    });
    cx.on_action(move |_: &RestartTunnel, cx: &mut App| {
        state.update(cx, |state, cx| state.restart(cx));
    });
}

/// Runs `f` against the main window on the next tick. Menu actions are
/// dispatched while the window is mid-update (it is taken out of the window
/// map), so a direct window lookup fails with "window not found".
fn with_main_window(cx: &mut App, f: impl FnOnce(&mut Window, &mut App) + 'static) {
    let Some(main) = main_window(cx) else {
        return;
    };
    cx.defer(move |cx| {
        let _ = main.handle.update(cx, |_, window, cx| f(window, cx));
    });
}

pub(crate) fn show_about(cx: &mut App) {
    with_main_window(cx, |window, cx| {
        window.open_alert_dialog(cx, about_dialog);
    });
}

const REPO_URL: &str = "https://github.com/lqmanh/patchbay";

fn about_dialog(alert: AlertDialog, _: &mut Window, cx: &mut App) -> AlertDialog {
    let muted = cx.theme().muted_foreground;
    alert.ok_text("OK").child(
        v_flex()
            .w_full()
            .items_center()
            .gap_2()
            .py_2()
            .child(widgets::logo().size_16())
            .child(div().text_lg().font_semibold().child("Patchbay"))
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child("Rule-based VPN routing for macOS"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(concat!("Version ", env!("CARGO_PKG_VERSION"))),
            )
            .child(
                h_flex().pt_4().child(
                    Link::new("about-repo")
                        .href(REPO_URL)
                        .text_sm()
                        .child(REPO_URL),
                ),
            ),
    )
}

fn main_window(cx: &App) -> Option<MainWindow> {
    cx.try_global::<MainWindow>().map(|main| MainWindow {
        handle: main.handle,
        shell: main.shell.clone(),
    })
}

fn register_menus(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-h", HideApp, None),
        KeyBinding::new("cmd-m", MinimizeWindow, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-r", RestartTunnel, None),
    ]);

    cx.set_menus(vec![
        Menu::new("Patchbay").items([
            MenuItem::action("About Patchbay", ShowAbout),
            MenuItem::separator(),
            MenuItem::action("Settings…", OpenSettings),
            MenuItem::separator(),
            MenuItem::action("Hide Patchbay", HideApp),
            MenuItem::separator(),
            MenuItem::action("Quit Patchbay", Quit),
        ]),
        Menu::new("Tunnel").items([
            MenuItem::action("Toggle Interception", ToggleInterception),
            MenuItem::action("Restart Tunnel", RestartTunnel),
        ]),
        Menu::new("Window").items([
            MenuItem::action("Minimize", MinimizeWindow),
            MenuItem::action("Close Window", CloseWindow),
        ]),
    ]);
}
