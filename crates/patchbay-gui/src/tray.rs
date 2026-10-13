use std::io::Cursor;

use anyhow::Context as _;
use gpui_kit::base::async_util;
use gpui_kit::*;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use patchbay_core::config::Mode;

use crate::state::{AppState, StateKind};
use crate::{OpenApp, OpenSettings, Quit, RestartTunnel, ToggleInterception};

const INTERCEPT: &str = "intercept";
const RESTART: &str = "restart";
const OPEN: &str = "open";
const SETTINGS: &str = "settings";
const QUIT: &str = "quit";

/// The menu bar item: an icon showing the routing state, and a menu with the
/// everyday controls. It lives as long as the app, with or without a window.
struct Tray {
    icon: TrayIcon,
    status: MenuItem,
    intercept: CheckMenuItem,
    restart: MenuItem,
    shown: Option<Shown>,
}

impl Global for Tray {}

/// What the menu bar item shows, to skip updates that change nothing:
/// `AppState` also notifies for log lines.
#[derive(Clone, PartialEq, Eq)]
struct Shown {
    glyph: Glyph,
    label: String,
    port: Option<u16>,
    intercepting: bool,
    busy: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Glyph {
    SplitOn,
    FullOn,
    Off,
    Broken,
}

enum Event {
    Clicked,
    Menu(MenuId),
}

pub fn init(state: Entity<AppState>, cx: &mut App) -> anyhow::Result<()> {
    let status = MenuItem::new("", false, None);
    let intercept = CheckMenuItem::with_id(INTERCEPT, "Intercept Traffic", true, false, None);
    let restart = MenuItem::with_id(RESTART, "Restart Tunnel", true, None);
    let menu = Menu::with_items(&[
        &status,
        &intercept,
        &restart,
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id(OPEN, "Open App", true, None),
        &MenuItem::with_id(SETTINGS, "Settings…", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id(QUIT, "Quit", true, None),
    ])?;
    let icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Patchbay")
        .build()?;

    cx.set_global(Tray {
        icon,
        status,
        intercept,
        restart,
        shown: None,
    });
    sync(&state, cx);
    cx.observe(&state, |state, cx| sync(&state, cx)).detach();

    // Both handlers run outside gpui, so they hand events to a task that does.
    let (sender, receiver) = async_util::unbounded();
    MenuEvent::set_event_handler(Some({
        let sender = sender.clone();
        move |event: MenuEvent| {
            let _ = sender.try_send(Event::Menu(event.id));
        }
    }));
    TrayIconEvent::set_event_handler(Some(move |event| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left | MouseButton::Right,
            button_state: MouseButtonState::Down,
            ..
        } = event
        {
            let _ = sender.try_send(Event::Clicked);
        }
    }));
    cx.spawn(async move |cx| {
        while let Ok(event) = receiver.recv().await {
            cx.update(|cx| handle(event, &state, cx));
        }
    })
    .detach();

    Ok(())
}

fn handle(event: Event, state: &Entity<AppState>, cx: &mut App) {
    match event {
        // Status is otherwise polled once a minute; refresh it as the menu opens.
        Event::Clicked => state.update(cx, |state, cx| state.refresh_status(cx)),
        Event::Menu(id) => match id.as_ref() {
            INTERCEPT => {
                // muda flips the check mark on click; keep it on the state.
                let intercepting = state.read(cx).intercepting();
                cx.global::<Tray>().intercept.set_checked(intercepting);
                cx.dispatch_action(&ToggleInterception);
            }
            RESTART => cx.dispatch_action(&RestartTunnel),
            OPEN => cx.dispatch_action(&OpenApp),
            SETTINGS => cx.dispatch_action(&OpenSettings),
            QUIT => cx.dispatch_action(&Quit),
            _ => {}
        },
    }
}

fn sync(state: &Entity<AppState>, cx: &mut App) {
    let state = state.read(cx);
    let kind = state.state_kind();
    let glyph = match kind {
        StateKind::On => match state.config.mode {
            Mode::Split => Glyph::SplitOn,
            Mode::Full => Glyph::FullOn,
        },
        StateKind::Off => Glyph::Off,
        StateKind::Broken => Glyph::Broken,
    };
    let label = match (kind, state.active_server()) {
        (StateKind::On, Some(server)) => format!("{} · {}", kind.label(), server.name),
        _ => kind.label().to_string(),
    };
    let port = state
        .status
        .proxy
        .as_ref()
        .and_then(|proxy| proxy.loopback_port());
    let tooltip = match kind {
        StateKind::Broken => format!("Patchbay: {}", state.broken_description()),
        _ => format!("Patchbay: {label}"),
    };
    let next = Shown {
        glyph,
        label,
        port,
        intercepting: state.intercepting(),
        busy: state.busy,
    };

    let tray = cx.global_mut::<Tray>();
    let shown = tray.shown.replace(next.clone());
    if shown.as_ref() == Some(&next) {
        return;
    }
    if shown.is_none_or(|shown| shown.glyph != next.glyph) {
        match glyph_icon(next.glyph) {
            Ok(icon) => {
                let _ = tray.icon.set_icon_templated(Some(icon));
            }
            Err(error) => eprintln!("patchbay: menu bar icon: {error:#}"),
        }
    }
    let _ = tray.icon.set_tooltip(Some(tooltip));
    tray.status.set_text(&next.label);
    tray.intercept.set_checked(next.intercepting);
    tray.intercept.set_enabled(!next.busy);
    tray.restart.set_enabled(!next.busy);
}

/// tray-icon takes an image's pixels as points and shrinks anything taller
/// than 22pt to 22pt. Centering the 44 × 32 @2x art on a 44px-tall canvas
/// makes that 22pt, so the art keeps its drawn 22 × 16pt at 2x sharpness.
fn glyph_icon(glyph: Glyph) -> anyhow::Result<Icon> {
    const CANVAS_HEIGHT: usize = 44;

    let png: &[u8] = match glyph {
        Glyph::SplitOn => include_bytes!("../../../assets/icon/menubar-split-on@2x.png"),
        Glyph::FullOn => include_bytes!("../../../assets/icon/menubar-full-on@2x.png"),
        Glyph::Off => include_bytes!("../../../assets/icon/menubar-off@2x.png"),
        Glyph::Broken => include_bytes!("../../../assets/icon/menubar-broken@2x.png"),
    };
    let mut reader = png::Decoder::new(Cursor::new(png)).read_info()?;
    let mut art = vec![0; reader.output_buffer_size().context("image too large")?];
    let info = reader.next_frame(&mut art)?;
    anyhow::ensure!(
        info.color_type == png::ColorType::Rgba && info.bit_depth == png::BitDepth::Eight,
        "expected 8-bit RGBA, got {:?} {:?}",
        info.color_type,
        info.bit_depth,
    );

    let (width, height) = (info.width as usize, info.height as usize);
    anyhow::ensure!(height <= CANVAS_HEIGHT, "art taller than {CANVAS_HEIGHT}px");
    let row = width * 4;
    let mut rgba = vec![0; (CANVAS_HEIGHT - height) / 2 * row];
    rgba.extend_from_slice(&art[..height * row]);
    rgba.resize(CANVAS_HEIGHT * row, 0);
    Ok(Icon::from_rgba(rgba, width as u32, CANVAS_HEIGHT as u32)?)
}

#[cfg(test)]
mod tests {
    use super::{Glyph, glyph_icon};

    #[test]
    fn glyphs_load() {
        for glyph in [Glyph::SplitOn, Glyph::FullOn, Glyph::Off, Glyph::Broken] {
            glyph_icon(glyph).unwrap();
        }
    }
}
