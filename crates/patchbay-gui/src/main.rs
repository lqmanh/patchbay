mod app;
mod frame;
mod sections;
mod selects;
mod state;
mod tray;
mod widgets;

use gpui_kit::assets::Assets;
use gpui_kit::component::{
    ActiveTheme, StyledExt as _, TitleBar, WindowExt as _, dialog::AlertDialog, h_flex, link::Link,
    v_flex,
};
use gpui_kit::*;

use app::{AppShell, Section, ShellLayout};
use frame::WindowFrame;
use state::AppState;

/// The main window and its view. Menu commands run while AppKit reports no
/// active window (menu tracking clears `mainWindow`), so handlers cannot rely
/// on `App::active_window`.
struct MainWindow {
    handle: AnyWindowHandle,
    shell: Entity<AppShell>,
}

/// App-wide UI state. The app outlives its window: closing it leaves the
/// menu bar item, which can open a new one.
struct Ui {
    state: Entity<AppState>,
    main: Option<MainWindow>,
    layout: ShellLayout,
}

impl Global for Ui {}

actions!(
    patchbay,
    [
        Quit,
        OpenApp,
        OpenSettings,
        ShowAbout,
        ToggleInterception,
        RestartTunnel,
        HideApp,
        HideOthers,
        MinimizeWindow,
        CloseWindow,
    ]
);

fn main() {
    // Accessibility is off, so VoiceOver can't read the app: with it on,
    // closing a window aborts in AppKit's Touch Bar KVO teardown (gpui bug).
    // Hiding the window instead would avoid that, but in the menu bar the app
    // then holds ~64 MB instead of ~31 MB.
    let app = Application::new_inaccessible(gpui_kit::platform::current_platform(false))
        .with_assets(Assets)
        .with_quit_mode(QuitMode::Explicit);
    // Launching the app again while it runs in the menu bar.
    app.on_reopen(show_main_window);
    app.run(|cx| {
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

        let frame = WindowFrame::load(&state.read(cx).paths);
        cx.set_global(Ui {
            state: state.clone(),
            main: None,
            layout: ShellLayout {
                frame,
                ..ShellLayout::default()
            },
        });
        register_actions(cx, state.clone());
        register_menus(cx);
        cx.on_window_closed(on_window_closed).detach();
        cx.on_app_quit(|cx| {
            if let Some(main) = main_window(cx) {
                save_frame(&main.shell.read(cx).layout(), cx);
            }
            async {}
        })
        .detach();

        if let Err(error) = tray::init(state, cx) {
            eprintln!("patchbay: menu bar item: {error:#}");
        }
        if let Err(error) = open_main_window(cx) {
            eprintln!("patchbay: {error}");
            cx.quit();
        }
    });
}

/// Brings the main window to the front, opening it if it's closed.
fn show_main_window(cx: &mut App) {
    if main_window(cx).is_some() {
        cx.activate(true);
        with_main_window(cx, |window, _| window.activate_window());
    } else if let Err(error) = open_main_window(cx) {
        eprintln!("patchbay: {error}");
    }
}

fn open_main_window(cx: &mut App) -> anyhow::Result<()> {
    let ui = cx.global::<Ui>();
    let state = ui.state.clone();
    let layout = ui.layout.clone();

    // Back in the Dock and app menu while a window is open.
    cx.set_activation_policy(ActivationPolicy::Regular);
    let (bounds, display_id) = match layout.frame.as_ref().and_then(|frame| frame.placement(cx)) {
        Some((bounds, display)) => (bounds, Some(display)),
        None => (WindowBounds::centered(size(px(940.), px(620.)), cx), None),
    };
    let window_options = WindowOptions {
        window_bounds: Some(bounds),
        display_id,
        window_min_size: Some(size(px(760.), px(520.))),
        ..TitleBar::window_options()
    };
    let (handle, shell) = open_window(window_options, cx, {
        let state = state.clone();
        move |window, cx| {
            let theme = state.read(cx).theme();
            theme.apply(Some(window), cx);
            cx.new(|cx| AppShell::new(state.clone(), layout, window, cx))
        }
    })?;
    cx.global_mut::<Ui>().main = Some(MainWindow { handle, shell });
    state.update(cx, |state, cx| state.set_log_polling(true, cx));
    cx.activate(true);
    Ok(())
}

fn on_window_closed(cx: &mut App, id: WindowId) {
    let Some(main) = cx
        .global_mut::<Ui>()
        .main
        .take_if(|main| main.handle.window_id() == id)
    else {
        return;
    };
    let layout = main.shell.read(cx).layout();
    save_frame(&layout, cx);
    let ui = cx.global_mut::<Ui>();
    ui.layout = layout;
    let state = ui.state.clone();
    state.update(cx, |state, cx| state.set_log_polling(false, cx));
    // Only the menu bar item is left; drop the Dock icon and app menu.
    cx.set_activation_policy(ActivationPolicy::Accessory);
}

fn save_frame(layout: &ShellLayout, cx: &App) {
    let Some(frame) = &layout.frame else {
        return;
    };
    let paths = &cx.global::<Ui>().state.read(cx).paths;
    if let Err(error) = frame.save(paths) {
        eprintln!("patchbay: saving the window frame: {error:#}");
    }
}

fn register_actions(cx: &mut App, state: Entity<AppState>) {
    cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
    cx.on_action(|_: &HideApp, cx: &mut App| cx.hide());
    cx.on_action(|_: &HideOthers, cx: &mut App| cx.hide_other_apps());
    cx.on_action(|_: &MinimizeWindow, cx: &mut App| {
        with_main_window(cx, |window, _| window.minimize_window());
    });
    cx.on_action(|_: &CloseWindow, cx: &mut App| {
        with_main_window(cx, |window, _| window.remove_window());
    });
    cx.on_action(|_: &OpenApp, cx: &mut App| show_main_window(cx));
    cx.on_action(|_: &OpenSettings, cx: &mut App| {
        show_main_window(cx);
        let Some(shell) = main_window(cx).map(|main| main.shell.clone()) else {
            return;
        };
        shell.update(cx, |shell, cx| {
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
    let Some(handle) = main_window(cx).map(|main| main.handle) else {
        return;
    };
    cx.defer(move |cx| {
        let _ = handle.update(cx, |_, window, cx| f(window, cx));
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

fn main_window(cx: &App) -> Option<&MainWindow> {
    cx.try_global::<Ui>()?.main.as_ref()
}

fn register_menus(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-h", HideApp, None),
        KeyBinding::new("cmd-alt-h", HideOthers, None),
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
            MenuItem::action("Hide Others", HideOthers),
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
